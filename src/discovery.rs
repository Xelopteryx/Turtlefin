//! Recherche des serveurs Jellyfin : réseau local et réseau Tailscale (pas Internet).
//!
//! Trois sources, en parallèle :
//! - la découverte UDP de Jellyfin (« who is JellyfinServer? » sur le port 7359), quand le serveur
//!   l'expose (souvent pas dans un conteneur) ;
//! - un sondage HTTP du sous-réseau local (/24) sur les ports habituels ;
//! - un sondage des appareils Tailscale en ligne (liste fournie par `tailscale status --json`).
//!
//! Un même serveur (même identifiant) vu en local et via Tailscale donne une seule entrée avec ses
//! deux adresses.

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
    /// Adresse sur le réseau local (« http://192.168.1.32:8097 »).
    pub local: Option<String>,
    /// Adresse distante (Tailscale ou autre).
    pub remote: Option<String>,
}

/// Adresse Tailscale (100.64.0.0/10) ?
pub fn is_tailscale(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 100 && (64..128).contains(&o[1])
}

/// L'adresse (URL) désigne-t-elle une machine du réseau local ?
pub fn is_local_url(url: &str) -> bool {
    let host = url.split("://").nth(1).unwrap_or(url).split(['/', ':']).next().unwrap_or("");
    match host.parse::<Ipv4Addr>() {
        Ok(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        Err(_) => host.ends_with(".local") || host == "localhost",
    }
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

/// Adresse IPv4 de la machine sur le réseau local (interface de sortie par défaut ; rien n'est envoyé).
fn local_ipv4() -> Option<Ipv4Addr> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("192.0.2.1:9").ok()?; // adresse de documentation : seule la table de routage est consultée
    match s.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_loopback() && !is_tailscale(ip) => Some(ip),
        _ => None,
    }
}

/// Appareils Tailscale en ligne (adresses IPv4), via la commande `tailscale`.
fn tailscale_peers() -> Vec<Ipv4Addr> {
    let candidates: &[&str] = if cfg!(windows) {
        &["tailscale", r"C:\Program Files\Tailscale\tailscale.exe"]
    } else {
        &["tailscale"]
    };
    for exe in candidates {
        let Ok(out) = std::process::Command::new(exe).args(["status", "--json"]).output() else { continue };
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
    let tail = tokio::task::spawn_blocking(tailscale_peers).await.unwrap_or_default();
    let local = local_ipv4();

    // Hôtes à sonder : le /24 local et les appareils Tailscale en ligne.
    let mut hosts: Vec<Ipv4Addr> = Vec::new();
    if let Some(ip) = local {
        let o = ip.octets();
        hosts.extend((1..=254).map(|i| Ipv4Addr::new(o[0], o[1], o[2], i)));
    }
    hosts.extend(tail.iter().copied());

    let udp = {
        let tail = tail.clone();
        tokio::task::spawn_blocking(move || udp_discovery(&tail))
    };

    // Sondage : connexion TCP rapide, puis requête HTTP(S) seulement si le port répond.
    let permits = Arc::new(tokio::sync::Semaphore::new(96));
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
        let slot = if is_local_url(&base) { &mut f.local } else { &mut f.remote };
        if slot.is_none() {
            *slot = Some(base);
        }
    }
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found
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

/// Le serveur répond-il (quelques secondes au plus) ?
pub async fn reachable(base: &str) -> bool {
    probe(&http(), base).await.is_some()
}

/// Choisit l'adresse à utiliser : la préférée si elle répond, sinon l'autre (ou la préférée par défaut).
pub async fn pick(local: &str, remote: &str, prefer_remote: bool) -> String {
    let (first, second) = if prefer_remote { (remote, local) } else { (local, remote) };
    let candidates: Vec<&str> = [first, second].into_iter().filter(|s| !s.is_empty()).collect();
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
