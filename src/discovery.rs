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
    let mut b = reqwest::Client::builder().timeout(Duration::from_millis(2500)).connect_timeout(Duration::from_millis(700));
    if std::env::var("TURTLEFIN_INSECURE").is_ok() {
        b = b.danger_accept_invalid_certs(true);
    }
    b.build().unwrap_or_default()
}

/// Interroge `/System/Info/Public` : (identifiant, nom, version) si c'est un serveur Jellyfin.
pub async fn probe(client: &reqwest::Client, base: &str) -> Option<(String, String, String)> {
    // Délai court : un serveur éteint ne doit pas bloquer l'écran (sans limite, plusieurs minutes).
    let v: Value = client.get(format!("{base}/System/Info/Public")).timeout(Duration::from_secs(4)).send().await.ok()?.json().await.ok()?;
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
        // Pas de sous-réseau utile (/31, /32 : VPN maillé) : rien à balayer ici.
        if a.prefixlen >= 31 {
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

/// Découverte UDP de Jellyfin : réponses au format {"Address", "Id", "Name"}.
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
        let Ok((n, _)) = s.recv_from(&mut buf) else { continue };
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

    // Sondage : connexion TCP rapide, puis requête HTTP(S) seulement si le port répond.
    let permits = Arc::new(tokio::sync::Semaphore::new(192));
    let mut tasks = Vec::new();
    for ip in hosts {
        for port in PORTS {
            let (client, permits) = (client.clone(), permits.clone());
            tasks.push(tokio::spawn(async move {
                let _p = permits.acquire().await.ok()?;
                let addr = SocketAddr::new(IpAddr::V4(ip), port);
                tokio::time::timeout(Duration::from_millis(400), tokio::net::TcpStream::connect(addr)).await.ok()?.ok()?;
                let scheme = if port == 8920 { "https" } else { "http" };
                let base = format!("{scheme}://{ip}:{port}");
                let (id, name, version) = probe(&client, &base).await?;
                Some((base, id, name, version))
            }));
        }
    }

    let mut hits: Vec<(String, String, String, String)> = Vec::new();
    for t in tasks {
        if let Ok(Some(h)) = t.await {
            hits.push(h);
        }
    }
    for (addr, id, name) in udp.await.unwrap_or_default() {
        if !hits.iter().any(|h| h.0 == addr) {
            hits.push((addr, id, name, String::new()));
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
    found
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