//! Relais de diffusion : une télé qui ne peut pas joindre le serveur (serveur sur un autre réseau,
//! joint par Tailscale ou Internet) lit la vidéo par cet appareil. Il ouvre un petit serveur HTTP sur
//! le réseau local, récupère le flux auprès du serveur Jellyfin et le transmet tel quel (aucune
//! conversion ici : le serveur fait le travail lourd).
//!
//! Sécurité : chaque adresse relayée porte un jeton aléatoire et n'est servie qu'à l'écran à qui elle
//! a été donnée (son adresse IP) ; rien d'autre n'est servi. Les jetons ne vivent que le temps de
//! l'appli.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use anyhow::{anyhow, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Serveur relayé (« http://hôte:port », sans chemin) et l'écran autorisé à le lire. Le chemin et
/// la requête de l'écran sont repris tels quels derrière le jeton : les listes HLS (adresses
/// relatives de leurs morceaux) passent donc aussi par le relais.
struct Route {
    origin: String,
    screen: IpAddr,
}

struct Relay {
    port: u16,
    routes: Mutex<HashMap<String, Route>>,
    /// Client HTTP partagé par toutes les requêtes (une vidéo HLS en fait des centaines : un client
    /// neuf à chaque fois refaisait sa configuration TLS et perdait ses connexions ouvertes).
    http: reqwest::Client,
}

static RELAY: OnceLock<Relay> = OnceLock::new();

/// Démarre le relais au premier besoin (port choisi par le système, toutes les cartes).
async fn relay() -> Result<&'static Relay> {
    if let Some(r) = RELAY.get() {
        return Ok(r);
    }
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))).await?;
    let port = listener.local_addr()?.port();
    // Pas de délai sur l'ensemble (un épisode dure longtemps), seulement sur la connexion.
    let http = crate::api::http_builder().connect_timeout(Duration::from_secs(15)).build()?;
    let r = RELAY.get_or_init(|| Relay { port, routes: Mutex::new(HashMap::new()), http });
    if r.port == port {
        tokio::spawn(async move {
            // 32 requêtes à la fois au plus : un appareil du réseau qui ouvre des milliers de
            // connexions (chacune attend jusqu'à 10 s ses en-têtes) n'épuise pas l'appli. Une
            // télé en lit 2 ou 3 à la fois (vidéo, sous-titres, morceaux HLS).
            let slots = std::sync::Arc::new(tokio::sync::Semaphore::new(32));
            while let Ok((sock, peer)) = listener.accept().await {
                let Ok(slot) = slots.clone().try_acquire_owned() else {
                    drop(sock);
                    continue;
                };
                tokio::spawn(async move {
                    let _slot = slot;
                    if let Err(e) = serve(sock, peer.ip()).await {
                        eprintln!("turtlefin : relais : {e}");
                    }
                });
            }
        });
        eprintln!("turtlefin : relais de diffusion sur le port {port}");
    }
    Ok(r)
}

/// Adresse de cet appareil sur le réseau de l'écran (même /24), sinon la première adresse locale.
fn local_ip_for(screen: Ipv4Addr) -> Option<Ipv4Addr> {
    let ips: Vec<Ipv4Addr> = crate::lan::lan_ipv4s().into_iter().filter(|ip| !ip.is_unspecified()).collect();
    ips.iter().find(|ip| ip.octets()[..3] == screen.octets()[..3]).or(ips.first()).copied()
}

/// Adresse, sur le réseau local, par laquelle l'écran `screen` lira `upstream` (adresse complète du
/// serveur, jeton d'accès compris).
pub async fn url_for(upstream: &str, screen: Ipv4Addr) -> Result<String> {
    let u = reqwest::Url::parse(upstream)?;
    let origin = u.origin().ascii_serialization();
    let rest = &upstream[origin.len()..];
    let r = relay().await?;
    let me = local_ip_for(screen).ok_or_else(|| anyhow!("aucune adresse sur le réseau local"))?;
    let mut routes = r.routes.lock().unwrap();
    // Un jeton par écran et par serveur, gardé tant que l'appli tourne (vidéo, sous-titres, suite).
    let token = match routes.iter().find(|(_, v)| v.screen == IpAddr::V4(screen) && v.origin == origin) {
        Some((t, _)) => t.clone(),
        None => {
            let t = uuid::Uuid::new_v4().simple().to_string();
            routes.insert(t.clone(), Route { origin, screen: IpAddr::V4(screen) });
            t
        }
    };
    Ok(format!("http://{me}:{}/r/{token}{rest}", r.port))
}

/// En-têtes de la requête de l'écran (nom en minuscules) et sa première ligne.
async fn read_head(sock: &mut TcpStream) -> Result<(String, Vec<(String, String)>)> {
    let mut buf = Vec::with_capacity(2048);
    let mut b = [0u8; 1024];
    loop {
        let n = tokio::time::timeout(Duration::from_secs(10), sock.read(&mut b)).await??;
        if n == 0 {
            return Err(anyhow!("connexion fermée"));
        }
        buf.extend_from_slice(&b[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > 16 << 10 {
            return Err(anyhow!("requête trop longue"));
        }
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    let mut lines = text.split("\r\n");
    let first = lines.next().unwrap_or("").to_string();
    let headers = lines
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_once(':').map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())))
        .collect();
    Ok((first, headers))
}

async fn reply(sock: &mut TcpStream, status: &str) -> Result<()> {
    sock.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await?;
    Ok(())
}

/// Une requête de l'écran : la même au serveur (avec sa plage d'octets), et la réponse recopiée.
async fn serve(mut sock: TcpStream, peer: IpAddr) -> Result<()> {
    let (first, headers) = read_head(&mut sock).await?;
    let mut parts = first.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    // OPTIONS : vérification préalable du lecteur web d'un Chromecast (CORS), relayée aussi.
    if !matches!(method, "GET" | "HEAD" | "OPTIONS") {
        return reply(&mut sock, "405 Method Not Allowed").await;
    }
    let after = path.strip_prefix("/r/").unwrap_or("");
    let cut = after.find(['?', '/']).unwrap_or(after.len());
    let (token, rest) = after.split_at(cut);
    let r = RELAY.get().ok_or_else(|| anyhow!("relais absent"))?;
    let upstream = {
        let routes = r.routes.lock().unwrap();
        // Seul l'écran à qui l'adresse a été donnée la lit, et seulement sur ce serveur.
        routes.get(token).filter(|v| v.screen == peer && rest.starts_with('/')).map(|v| format!("{}{rest}", v.origin))
    };
    let Some(upstream) = upstream else { return reply(&mut sock, "404 Not Found").await };
    let http = &r.http;
    let m = reqwest::Method::from_bytes(method.as_bytes())?;
    let mut req = http.request(m, &upstream);
    // Ce que l'écran demande et dont le serveur a besoin pour bien répondre (plage d'octets, CORS du
    // lecteur web d'un Chromecast).
    for (k, v) in &headers {
        if matches!(
            k.as_str(),
            "range" | "user-agent" | "origin" | "access-control-request-method" | "access-control-request-headers"
        ) {
            req = req.header(k.as_str(), v.as_str());
        }
    }
    let mut resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            let _ = reply(&mut sock, "502 Bad Gateway").await;
            // Sans l'adresse : celle d'une diffusion porte le jeton (api_key), et ce message finit
            // dans les journaux (sur le Pi, recopiés dans un fichier par Xelauncher).
            return Err(anyhow!("serveur injoignable : {}", e.without_url()));
        }
    };
    let status = resp.status();
    if std::env::var_os("TURTLEFIN_RELAY").is_some() {
        let shown = rest.split('?').next().unwrap_or("");
        eprintln!("turtlefin : relais {peer} {method} {shown} -> {}", status.as_u16());
    }
    let mut head = format!("HTTP/1.1 {} {}\r\n", status.as_u16(), status.canonical_reason().unwrap_or(""));
    // En-têtes du serveur recopiés (type, taille, plage, CORS…), sauf ceux de la connexion
    // elle-même : le relais ferme chaque connexion après sa réponse.
    for (name, v) in resp.headers() {
        if matches!(name.as_str(), "connection" | "keep-alive" | "transfer-encoding" | "set-cookie") {
            continue;
        }
        if let Ok(v) = v.to_str() {
            head.push_str(&format!("{}: {v}\r\n", name.as_str()));
        }
    }
    head.push_str("Connection: close\r\n\r\n");
    sock.write_all(head.as_bytes()).await?;
    if method != "GET" {
        return Ok(());
    }
    // Le flux, morceau par morceau, au rythme où l'écran le lit.
    while let Some(chunk) = resp.chunk().await? {
        if sock.write_all(&chunk).await.is_err() {
            break;
        }
    }
    let _ = sock.shutdown().await;
    Ok(())
}
