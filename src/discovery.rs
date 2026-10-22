//! Recherche des serveurs Jellyfin sur les réseaux de la machine (pas Internet).
//!
//! Toutes les interfaces réseau sont prises en compte, VPN compris : un VPN sert justement à
//! mettre des appareils distants sur un même réseau. Sources, en parallèle :
//! - la découverte UDP de Jellyfin (« who is JellyfinServer? » sur le port 7359), quand le serveur
//!   l'expose (souvent pas dans un conteneur) ;
//! - un sondage HTTP du sous-réseau de chaque interface (limité à 1022 hôtes : au-delà, le /24
//!   autour de l'adresse de la machine) ;
//! - les voisins connus de la machine (table ARP) et les pairs des VPN maillés qui ne donnent pas
//!   de sous-réseau (adresse en /32), quand leur outil en ligne de commande les liste.
//!
//! Un même serveur (même identifiant) trouvé à plusieurs adresses donne une seule entrée : une
//! adresse principale et une adresse de secours.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

/// Ports Jellyfin courants (8096 par défaut, 8097 souvent utilisé en conteneur, 8920 en HTTPS).
const PORTS: [u16; 3] = [8096, 8097, 8920];

#[derive(Clone, Debug, Default)]
pub struct Found {
    pub id: String,
    pub name: String,
    pub version: String,
    /// Adresse principale (de préférence celle du réseau de la sortie par défaut).
    pub main: Option<String>,
    /// Adresse de secours : le même serveur par un autre réseau (VPN...).
    pub backup: Option<String>,
}

fn http() -> reqwest::Client {
    http_with(Duration::from_millis(1500))
}

/// Client des sondages, connexion limitée à `connect` (le total l'est par probe).
fn http_with(connect: Duration) -> reqwest::Client {
    let mut b = crate::api::http_builder().timeout(Duration::from_millis(2500)).connect_timeout(connect).redirect(crate::api::redirect_policy());
    if std::env::var("TURTLEFIN_INSECURE").is_ok() {
        b = b.danger_accept_invalid_certs(true);
    }
    b.build().unwrap_or_default()
}

/// Interroge `/System/Info/Public` : (identifiant, nom, version) si c'est un serveur Jellyfin.
pub async fn probe(client: &reqwest::Client, base: &str) -> Option<(String, String, String)> {
    // Délai court : un serveur éteint ne doit pas bloquer l'écran (sans limite, plusieurs minutes).
    // N'importe quel appareil du réseau peut répondre : réponse bornée (1 Mio).
    let resp = client.get(format!("{base}/System/Info/Public")).timeout(Duration::from_secs(4)).send().await.ok()?;
    let v: Value = serde_json::from_slice(&crate::api::body_limited(resp, 1 << 20).await.ok()?).ok()?;
    let id = v["Id"].as_str()?.to_string();
    let name = v["ServerName"].as_str().unwrap_or("Jellyfin").to_string();
    let version = v["Version"].as_str().unwrap_or("").to_string();
    Some((id, name, version))
}

/// Adresse IPv4 de la sortie par défaut (rien n'est envoyé : seule la table de routage est lue).
/// Un réseau est-il disponible (interface active autre que la boucle locale) ?
pub fn has_network() -> bool {
    default_ipv4().is_some()
        || if_addrs::get_if_addrs().unwrap_or_default().iter().any(|i| !i.is_loopback() && i.is_oper_up())
}

fn default_ipv4() -> Option<Ipv4Addr> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("192.0.2.1:9").ok()?; // adresse de documentation
    match s.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_loopback() => Some(ip),
        _ => None,
    }
}

/// Plus grand réseau sondé en entier (/22). Au-delà, seul le /24 de la machine l'est.
const MAX_SCAN_PREFIX: u8 = 22;

/// Hôtes à sonder sur chaque interface IPv4 active (hors boucle locale), avec l'adresse de la
/// machine sur cette interface.
fn interface_hosts() -> Vec<(Ipv4Addr, Vec<Ipv4Addr>)> {
    let mut out = Vec::new();
    for itf in if_addrs::get_if_addrs().unwrap_or_default() {
        let if_addrs::IfAddr::V4(a) = &itf.addr else { continue };
        if a.ip.is_loopback() || a.ip.is_link_local() || a.ip.is_unspecified() || !itf.is_oper_up() {
            continue;
        }
        // Pas de sous-réseau utile (/31, /32 : VPN maillé) : rien à balayer ici. Seuls les réseaux
        // privés (192.168…, 10…, 172.16…) sont balayés : les données mobiles d'un téléphone ont souvent
        // une adresse d'opérateur en 100.64/10 avec un /24, et on sondait alors les appareils d'autres
        // abonnés (lent, et indiscret).
        if a.prefixlen >= 31 || !a.ip.is_private() {
            out.push((a.ip, Vec::new()));
            continue;
        }
        let prefix = if a.prefixlen < MAX_SCAN_PREFIX { 24 } else { a.prefixlen };
        let mask = u32::MAX << (32 - prefix as u32);
        let net = u32::from(a.ip) & mask;
        let hosts = (1..(!mask)).map(|i| Ipv4Addr::from(net | i)).filter(|h| *h != a.ip).collect();
        out.push((a.ip, hosts));
    }
    out
}

/// Voisins connus de la machine (table ARP / NDP du système).
fn neighbours() -> Vec<Ipv4Addr> {
    let text = if cfg!(target_os = "linux") {
        std::fs::read_to_string("/proc/net/arp").unwrap_or_default()
    } else {
        crate::paths::quiet_command("arp").arg("-a").output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
    };
    text.split(|c: char| c.is_whitespace() || c == '(' || c == ')')
        .filter_map(|w| w.parse::<Ipv4Addr>().ok())
        .filter(|ip| !ip.is_broadcast() && !ip.is_multicast() && !ip.is_unspecified() && ip.octets()[3] != 255)
        .collect()
}

/// Pairs des VPN maillés : leurs adresses sont en /32, sans sous-réseau à balayer. Seuls ceux dont
/// l'outil en ligne de commande donne la liste des pairs en JSON sont pris en charge.
fn mesh_peers() -> Vec<Ipv4Addr> {
    let candidates: &[&str] = if cfg!(windows) {
        &["tailscale", r"C:\Program Files\Tailscale\tailscale.exe"]
    } else {
        &["tailscale"]
    };
    for exe in candidates {
        let Ok(out) = crate::paths::quiet_command(exe).args(["status", "--json"]).output() else { continue };
        let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) else { continue };
        let mut ips: Vec<Ipv4Addr> = Vec::new();
        let mut add = |p: &Value| {
            if p["Online"].as_bool() == Some(false) {
                return;
            }
            for ip in p["TailscaleIPs"].as_array().into_iter().flatten() {
                if let Some(Ok(v4)) = ip.as_str().map(|s| s.parse::<Ipv4Addr>()) {
                    ips.push(v4);
                }
            }
        };
        add(&v["Self"]);
        for p in v["Peer"].as_object().into_iter().flatten().map(|(_, p)| p) {
            add(p);
        }
        return ips;
    }
    Vec::new()
}

/// Adresse d'un réseau local ou d'un VPN (privée, lien local, CGNAT 100.64/10 de Tailscale…).
pub fn local_ip(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_private() || ip.is_link_local() || ip.is_loopback() || (o[0] == 100 && (64..128).contains(&o[1]))
}

/// Découverte UDP de Jellyfin : réponses au format {"Address", "Id", "Name"}. Seules les réponses
/// venues d'un réseau local ou d'un VPN sont gardées (une machine d'Internet pourrait sinon se faire
/// passer pour le serveur et recevoir le mot de passe saisi) ; l'adresse annoncée est ensuite vérifiée.
fn udp_discovery(targets: &[Ipv4Addr]) -> Vec<(String, String, String)> {
    let Ok(s) = UdpSocket::bind("0.0.0.0:0") else { return Vec::new() };
    let _ = s.set_broadcast(true);
    let _ = s.set_read_timeout(Some(Duration::from_millis(300)));
    let msg = b"who is JellyfinServer?";
    let _ = s.send_to(msg, "255.255.255.255:7359");
    for ip in targets {
        let _ = s.send_to(msg, SocketAddr::new(IpAddr::V4(*ip), 7359));
    }
    let mut out = Vec::new();
    let end = std::time::Instant::now() + Duration::from_millis(1500);
    let mut buf = [0u8; 2048];
    while std::time::Instant::now() < end {
        // Sous Windows, un « port injoignable » renvoyé par un appareil fait échouer recv : on continue.
        let Ok((n, from)) = s.recv_from(&mut buf) else { continue };
        if !matches!(from.ip(), IpAddr::V4(ip) if local_ip(ip)) {
            continue;
        }
        if let Ok(v) = serde_json::from_slice::<Value>(&buf[..n]) {
            if let (Some(addr), Some(id)) = (v["Address"].as_str(), v["Id"].as_str()) {
                out.push((addr.trim_end_matches('/').to_string(), id.to_string(), v["Name"].as_str().unwrap_or("").to_string()));
            }
        }
    }
    out
}

/// Recherche complète (quelques secondes). Résultats regroupés par serveur.
pub async fn discover() -> Vec<Found> {
    let client = http();
    let (itfs, near, mesh) = tokio::task::spawn_blocking(|| (interface_hosts(), neighbours(), mesh_peers())).await.unwrap_or_default();
    let mine: Vec<Ipv4Addr> = itfs.iter().map(|(ip, _)| *ip).collect();
    let default = default_ipv4();

    // Hôtes à sonder : sous-réseaux des interfaces, voisins connus, pairs des VPN maillés.
    let mut hosts: Vec<Ipv4Addr> = Vec::new();
    for (_, h) in &itfs {
        hosts.extend(h.iter().copied());
    }
    hosts.extend(near.iter().copied());
    hosts.extend(mesh.iter().copied().filter(|ip| !mine.contains(ip)));
    hosts.sort();
    hosts.dedup();

    let udp = {
        let direct: Vec<Ipv4Addr> = near.iter().chain(mesh.iter()).copied().collect();
        tokio::task::spawn_blocking(move || udp_discovery(&direct))
    };

    // Adresses déjà connues (session, adresses locales notées) : sondées telles quelles, avec un
    // délai généreux. Sur un téléphone, un serveur joint par Tailscale n'est trouvé que par elles
    // (pas de liste des pairs Tailscale sur Android, interface en /32, table des voisins interdite).
    // Serveurs déjà trouvés : leurs adresses vues, et leur nom comme nom Tailscale (MagicDNS : un
    // serveur « ABYDOS » sur la machine « abydos » se joint par http://abydos:8096 où qu'on soit ; le
    // téléphone, lui, ne connaissait que son adresse locale). Un nom deviné n'est gardé que s'il mène
    // au même serveur (même identifiant).
    let known_tasks: Vec<_> = {
        let s = crate::config::load();
        let prefs = crate::config::ui_prefs();
        let mut known: Vec<(String, Option<String>)> = [s.server, s.server_main, s.server_backup].into_iter().map(|b| (b, None)).collect();
        known.extend(prefs.lan_servers.into_values().map(|b| (b, None)));
        for (id, k) in &prefs.known_servers {
            known.extend(k.addrs.iter().map(|b| (b.clone(), None)));
            if let Some(host) = magic_name(&k.name) {
                let mut ports: Vec<String> = k.addrs.iter().filter_map(|a| a.rsplit(':').next().filter(|p| p.parse::<u16>().is_ok()).map(str::to_string)).collect();
                ports.sort();
                ports.dedup();
                if ports.is_empty() {
                    ports.push("8096".into());
                }
                known.extend(ports.into_iter().map(|p| (format!("http://{host}:{p}"), Some(id.clone()))));
            }
        }
        known.retain(|(b, _)| b.starts_with("http://") || b.starts_with("https://"));
        known.iter_mut().for_each(|(b, _)| *b = b.trim_end_matches('/').to_string());
        known.sort();
        known.dedup_by(|a, b| a.0 == b.0);
        let slow = http_with(Duration::from_secs(3));
        known
            .into_iter()
            .map(|(base, want)| {
                let slow = slow.clone();
                tokio::spawn(async move {
                    let (id, name, version) = probe(&slow, &base).await?;
                    if want.is_some_and(|w| w != id) {
                        return None;
                    }
                    Some((base, id, name, version))
                })
            })
            .collect()
    };

    // Sondage : connexion TCP rapide, puis requête HTTP(S) seulement si le port répond.
    // 512 essais à la fois : avec 1 s par hôte muet, plusieurs /24 (cartes virtuelles…) restent
    // sondés en quelques secondes.
    let permits = Arc::new(tokio::sync::Semaphore::new(512));
    let mut tasks = Vec::new();
    for ip in hosts {
        for port in PORTS {
            let (client, permits) = (client.clone(), permits.clone());
            tasks.push(tokio::spawn(async move {
                let _p = permits.acquire().await.ok()?;
                let addr = SocketAddr::new(IpAddr::V4(ip), port);
                // Un téléphone en Wi-Fi (économie d'énergie) ou un VPN répondent parfois en plus de
                // 400 ms : le serveur n'était alors trouvé qu'une fois sur deux.
                tokio::time::timeout(Duration::from_millis(1000), tokio::net::TcpStream::connect(addr)).await.ok()?.ok()?;
                let scheme = if port == 8920 { "https" } else { "http" };
                let base = format!("{scheme}://{ip}:{port}");
                let (id, name, version) = probe(&client, &base).await?;
                Some((base, id, name, version))
            }));
        }
    }

    let mut hits: Vec<(String, String, String, String)> = Vec::new();
    for t in tasks.into_iter().chain(known_tasks) {
        if let Ok(Some(h)) = t.await {
            if !hits.iter().any(|x| x.0 == h.0) {
                hits.push(h);
            }
        }
    }
    for (addr, id, name) in udp.await.unwrap_or_default() {
        if hits.iter().any(|h| h.0 == addr) {
            continue;
        }
        // L'adresse annoncée doit répondre, en tant que ce même serveur (sinon : ignorée).
        if !(addr.starts_with("http://") || addr.starts_with("https://")) {
            continue;
        }
        match probe(&client, &addr).await {
            Some((real_id, real_name, version)) if real_id == id => hits.push((addr, id, if name.is_empty() { real_name } else { name }, version)),
            _ => {}
        }
    }

    // Adresse principale : celle du réseau de la sortie par défaut (souvent le réseau local),
    // puis les adresses privées ; les autres (VPN...) viennent en secours.
    let rank = |url: &str| -> u8 {
        let ip = host_of(url).parse::<Ipv4Addr>().ok();
        match (ip, default) {
            (Some(ip), Some(d)) if ip.octets()[..3] == d.octets()[..3] => 0,
            (Some(ip), _) if ip.is_private() => 1,
            _ => 2,
        }
    };
    hits.sort_by_key(|h| rank(&h.0));

    // Regroupement par identifiant de serveur.
    let mut found: Vec<Found> = Vec::new();
    for (base, id, name, version) in hits {
        let i = match found.iter().position(|f| f.id == id) {
            Some(i) => i,
            None => {
                found.push(Found { id: id.clone(), name: name.clone(), version: version.clone(), ..Default::default() });
                found.len() - 1
            }
        };
        let f = &mut found[i];
        if f.version.is_empty() {
            f.version = version;
        }
        if f.main.is_none() {
            f.main = Some(base);
        } else if f.backup.is_none() && host_of(f.main.as_deref().unwrap_or("")) != host_of(&base) {
            f.backup = Some(base);
        }
    }
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    remember(&found);
    found
}

/// Nom de machine (MagicDNS) tiré du nom d'un serveur : « ABYDOS » -> « abydos ». Rien si le nom
/// n'en est pas un (espaces, accents…) ou si c'est une adresse.
fn magic_name(name: &str) -> Option<String> {
    let h = name.trim().to_ascii_lowercase();
    let ok = !h.is_empty() && h.len() <= 63 && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !h.starts_with('-') && h.parse::<Ipv4Addr>().is_err();
    ok.then_some(h)
}

/// Note les serveurs trouvés et leurs adresses (les plus récentes d'abord, 6 au plus par serveur).
fn remember(found: &[Found]) {
    if found.is_empty() {
        return;
    }
    let mut p = crate::config::ui_prefs();
    let before = p.known_servers.clone();
    for f in found {
        let k = p.known_servers.entry(f.id.clone()).or_default();
        k.name = f.name.clone();
        for a in [&f.main, &f.backup].into_iter().flatten().rev() {
            k.addrs.retain(|x| x != a);
            k.addrs.insert(0, a.clone());
        }
        k.addrs.truncate(6);
    }
    if p.known_servers != before {
        crate::config::save_ui_prefs(&p);
    }
}

/// Hôte d'une adresse (« http://192.168.1.32:8097 » -> « 192.168.1.32 »).
fn host_of(url: &str) -> &str {
    url.split("://").nth(1).unwrap_or(url).split(['/', ':']).next().unwrap_or("")
}

/// Saisie manuelle : avec « http(s):// », l'adresse telle quelle ; sans, les deux (et le port 8096
/// par défaut si rien ne répond). Renvoie les adresses qui répondent, avec le serveur trouvé.
pub async fn resolve(input: &str) -> Vec<(String, String, String)> {
    let client = http();
    let input = input.trim().trim_end_matches('/');
    if input.is_empty() {
        return Vec::new();
    }
    let mut candidates: Vec<String> = if input.starts_with("http://") || input.starts_with("https://") {
        vec![input.to_string()]
    } else {
        vec![format!("http://{input}"), format!("https://{input}")]
    };
    let mut out = Vec::new();
    for round in 0..2 {
        for base in &candidates {
            if let Some((id, name, _)) = probe(&client, base).await {
                out.push((base.clone(), id, name));
            }
        }
        // Rien trouvé et aucun port précisé : on retente sur le port Jellyfin par défaut.
        let host_part = input.split("://").last().unwrap_or(input);
        if round == 0 && out.is_empty() && !host_part.contains(':') {
            candidates = candidates.iter().map(|c| format!("{c}:8096")).collect();
        } else {
            break;
        }
    }
    out
}

/// Adresse en `http://` qui passe par Internet (ni réseau local, ni VPN) : le mot de passe et le
/// jeton y circuleraient en clair. Un nom est résolu : il compte comme local si toutes ses adresses le
/// sont (MagicDNS de Tailscale, `.local`…).
pub async fn plain_over_internet(server: &str) -> bool {
    let Some(rest) = server.strip_prefix("http://") else { return false };
    let host_port = rest.split('/').next().unwrap_or("");
    let with_port = if host_port.rsplit_once(':').is_some_and(|(_, p)| p.parse::<u16>().is_ok()) && !host_port.starts_with('[') {
        host_port.to_string()
    } else {
        format!("{host_port}:80")
    };
    let Ok(Ok(addrs)) = tokio::time::timeout(Duration::from_secs(3), tokio::net::lookup_host(with_port)).await else { return false };
    let addrs: Vec<SocketAddr> = addrs.collect();
    !addrs.is_empty()
        && !addrs.iter().all(|a| match a.ip() {
            IpAddr::V4(ip) => local_ip(ip),
            IpAddr::V6(ip) => ip.is_loopback() || (ip.segments()[0] & 0xfe00) == 0xfc00 || (ip.segments()[0] & 0xffc0) == 0xfe80,
        })
}

/// Identifiant du serveur Jellyfin qui répond à cette adresse (quelques secondes au plus).
pub async fn server_id(base: &str) -> Option<String> {
    probe(&http(), base).await.map(|(id, _, _)| id)
}

/// Le serveur répond-il (quelques secondes au plus) ?
pub async fn reachable(base: &str) -> bool {
    probe(&http(), base).await.is_some()
}

/// Choisit l'adresse à utiliser : la principale si elle répond, sinon celle de secours (et la
/// principale si aucune ne répond).
pub async fn pick(main: &str, backup: &str) -> String {
    let candidates: Vec<&str> = [main, backup].into_iter().filter(|s| !s.is_empty()).collect();
    if candidates.len() <= 1 {
        return candidates.first().map(|s| s.to_string()).unwrap_or_default();
    }
    let client = http();
    for c in &candidates {
        if probe(&client, c).await.is_some() {
            return c.to_string();
        }
    }
    candidates[0].to_string()
}
#[cfg(test)]
mod tests {
    #[test]
    fn magic_names() {
        assert_eq!(super::magic_name("ABYDOS").as_deref(), Some("abydos"));
        assert_eq!(super::magic_name("pc-salon").as_deref(), Some("pc-salon"));
        // Pas un nom de machine : rien n'est deviné.
        assert_eq!(super::magic_name("Mon serveur"), None);
        assert_eq!(super::magic_name("Médiathèque"), None);
        assert_eq!(super::magic_name("192.168.1.32"), None);
        assert_eq!(super::magic_name(""), None);
    }
}
