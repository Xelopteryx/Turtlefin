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
    /// Conversion reprise en cours de vidéo : ses horodatages commencent à la position de reprise,
    /// et une télé Samsung attend alors l'image indéfiniment (le son passe). Ils sont ramenés à 0
    /// au passage (voir TsRebase). false : flux transmis tel quel.
    rebase: bool,
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
            while let Ok((sock, peer)) = listener.accept().await {
                tokio::spawn(async move {
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
    let ips: Vec<Ipv4Addr> = crate::cast::lan_ipv4s().into_iter().filter(|ip| !ip.is_unspecified()).collect();
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
    let token = match routes.iter().find(|(_, v)| v.screen == IpAddr::V4(screen) && v.origin == origin && !v.rebase) {
        Some((t, _)) => t.clone(),
        None => {
            let t = uuid::Uuid::new_v4().simple().to_string();
            routes.insert(t.clone(), Route { origin, screen: IpAddr::V4(screen), rebase: false });
            t
        }
    };
    Ok(format!("http://{me}:{}/r/{token}{rest}", r.port))
}

/// Comme url_for, pour une conversion MPEG-TS reprise en cours de vidéo : ses horodatages sont
/// ramenés à 0 au passage (voir Route::rebase). Une adresse neuve à chaque reprise.
pub async fn url_rebased(upstream: &str, screen: Ipv4Addr) -> Result<String> {
    let u = reqwest::Url::parse(upstream)?;
    let origin = u.origin().ascii_serialization();
    let rest = &upstream[origin.len()..];
    let r = relay().await?;
    let me = local_ip_for(screen).ok_or_else(|| anyhow!("aucune adresse sur le réseau local"))?;
    let mut routes = r.routes.lock().unwrap();
    routes.retain(|_, v| !(v.screen == IpAddr::V4(screen) && v.rebase));
    let t = uuid::Uuid::new_v4().simple().to_string();
    routes.insert(t.clone(), Route { origin, screen: IpAddr::V4(screen), rebase: true });
    Ok(format!("http://{me}:{}/r/{t}{rest}", r.port))
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
        routes.get(token).filter(|v| v.screen == peer && rest.starts_with('/')).map(|v| (format!("{}{rest}", v.origin), v.rebase))
    };
    let Some((upstream, rebase)) = upstream else { return reply(&mut sock, "404 Not Found").await };
    let http = &r.http;
    let m = reqwest::Method::from_bytes(method.as_bytes())?;
    let mut req = http.request(m, &upstream);
    // Ce que l'écran demande et dont le serveur a besoin pour bien répondre (plage d'octets, DLNA,
    // CORS du lecteur web d'un Chromecast).
    for (k, v) in &headers {
        if matches!(
            k.as_str(),
            "range" | "getcontentfeatures.dlna.org" | "transfermode.dlna.org" | "user-agent" | "origin" | "access-control-request-method" | "access-control-request-headers"
        ) {
            req = req.header(k.as_str(), v.as_str());
        }
    }
    let mut resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            let _ = reply(&mut sock, "502 Bad Gateway").await;
            return Err(anyhow!("serveur injoignable : {e}"));
        }
    };
    let status = resp.status();
    if std::env::var_os("TURTLEFIN_RELAY").is_some() {
        let shown = rest.split('?').next().unwrap_or("");
        eprintln!("turtlefin : relais {peer} {method} {shown} -> {}", status.as_u16());
    }
    let mut head = format!("HTTP/1.1 {} {}\r\n", status.as_u16(), status.canonical_reason().unwrap_or(""));
    // En-têtes du serveur recopiés (type, taille, plage, DLNA, CORS…), sauf ceux de la connexion
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
    // Le flux, morceau par morceau, au rythme où l'écran le lit (horodatages ramenés à 0 au besoin).
    let mut ts = rebase.then(TsRebase::new);
    while let Some(chunk) = resp.chunk().await? {
        let out = match ts.as_mut() {
            Some(t) => t.push(&chunk),
            None => chunk.to_vec(),
        };
        if sock.write_all(&out).await.is_err() {
            break;
        }
    }
    let _ = sock.shutdown().await;
    Ok(())
}

/// Horloge MPEG (33 bits, 90 kHz) : les valeurs reviennent à 0 après 2^33.
const TS_WRAP: u64 = 1 << 33;

/// Réécrit un flux MPEG-TS (paquets de 188 octets) pour qu'il commence à 0, comme une conversion
/// partie du début : chaque piste (PID) est recalée sur son premier horodatage (PTS / DTS des
/// en-têtes PES) puis décalée de 1,4 s, et l'horloge (PCR) sur la sienne, 0,7 s avant. Piste par
/// piste : avec des sous-titres incrustés, Jellyfin ramène déjà l'audio à 0 mais pas l'image ; un
/// même décalage pour les deux rendait l'audio négatif (aucun son). Les paquets coupés entre deux
/// morceaux attendent le suivant.
struct TsRebase {
    anchors: HashMap<u16, u64>,
    pcr_anchor: Option<u64>,
    pending: Vec<u8>,
}

/// Départ des horodatages réécrits (90 kHz) : 1,4 s, et l'horloge 0,7 s (ce que fait ffmpeg).
const PES_LEAD: u64 = 126_000;
const PCR_LEAD: u64 = 63_000;

impl TsRebase {
    fn new() -> Self {
        TsRebase { anchors: HashMap::new(), pcr_anchor: None, pending: Vec::with_capacity(188 * 64) }
    }

    /// `v` recalé : `lead` + (v - anchor), modulo l'horloge.
    fn shift(v: u64, anchor: u64, lead: u64) -> u64 {
        (v + TS_WRAP - anchor % TS_WRAP + lead) % TS_WRAP
    }

    /// Paquets complets réécrits ; le reste est gardé pour le prochain morceau.
    fn push(&mut self, data: &[u8]) -> Vec<u8> {
        self.pending.extend_from_slice(data);
        // Resynchronisation : un paquet commence par 0x47.
        if let Some(i) = self.pending.iter().position(|b| *b == 0x47) {
            self.pending.drain(..i);
        } else {
            self.pending.clear();
        }
        let n = self.pending.len() / 188 * 188;
        let mut out: Vec<u8> = self.pending.drain(..n).collect();
        for p in out.chunks_exact_mut(188) {
            if p[0] == 0x47 {
                self.packet(p);
            }
        }
        out
    }

    fn packet(&mut self, p: &mut [u8]) {
        let pid = u16::from(p[1] & 0x1f) << 8 | u16::from(p[2]);
        let afc = (p[3] >> 4) & 3;
        let mut at = 4;
        if afc & 2 != 0 {
            let len = p[4] as usize;
            // PCR : 33 bits de base, puis 6 réservés et 9 d'extension.
            if len >= 7 && p[5] & 0x10 != 0 {
                let b = &mut p[6..12];
                let pcr = (b[0] as u64) << 25 | (b[1] as u64) << 17 | (b[2] as u64) << 9 | (b[3] as u64) << 1 | (b[4] as u64) >> 7;
                let anchor = *self.pcr_anchor.get_or_insert(pcr);
                let v = Self::shift(pcr, anchor, PCR_LEAD);
                b[0] = (v >> 25) as u8;
                b[1] = (v >> 17) as u8;
                b[2] = (v >> 9) as u8;
                b[3] = (v >> 1) as u8;
                b[4] = (b[4] & 0x7f) | ((v & 1) as u8) << 7;
            }
            at = 5 + len;
        }
        // Début d'un en-tête PES (unité qui commence ici, avec données) : PTS, puis DTS.
        if afc & 1 == 0 || p[1] & 0x40 == 0 || at + 14 > 188 || p[at..at + 3] != [0, 0, 1] {
            return;
        }
        let sid = p[at + 3];
        if matches!(sid, 0xBC | 0xBE | 0xBF | 0xF0 | 0xF1 | 0xF2 | 0xF8 | 0xFF) {
            return;
        }
        let flags = p[at + 7] >> 6;
        let has_dts = flags == 3 && at + 19 <= 188;
        if flags & 2 == 0 {
            return;
        }
        // Repère de la piste : son premier horodatage (le DTS s'il précède le PTS).
        let first = if has_dts { read_stamp(&p[at + 14..at + 19]).min(read_stamp(&p[at + 9..at + 14])) } else { read_stamp(&p[at + 9..at + 14]) };
        let anchor = *self.anchors.entry(pid).or_insert(first);
        let pts = Self::shift(read_stamp(&p[at + 9..at + 14]), anchor, PES_LEAD);
        write_stamp(&mut p[at + 9..at + 14], pts);
        if has_dts {
            let dts = Self::shift(read_stamp(&p[at + 14..at + 19]), anchor, PES_LEAD);
            write_stamp(&mut p[at + 14..at + 19], dts);
        }
    }
}

/// PTS ou DTS sur 5 octets.
fn read_stamp(b: &[u8]) -> u64 {
    ((b[0] as u64 >> 1) & 7) << 30 | (b[1] as u64) << 22 | ((b[2] as u64) >> 1) << 15 | (b[3] as u64) << 7 | (b[4] as u64) >> 1
}

/// Réécrit un PTS ou DTS (marqueurs gardés).
fn write_stamp(b: &mut [u8], v: u64) {
    b[0] = (b[0] & 0xf1) | (((v >> 30) & 7) as u8) << 1;
    b[1] = (v >> 22) as u8;
    b[2] = ((((v >> 15) & 0x7f) as u8) << 1) | 1;
    b[3] = (v >> 7) as u8;
    b[4] = (((v & 0x7f) as u8) << 1) | 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paquet vidéo qui commence un PES avec PTS et DTS, et dont l'en-tête porte un PCR.
    fn packet(pid: u8, pcr: u64, pts: u64, dts: u64) -> Vec<u8> {
        let mut p = vec![0xffu8; 188];
        p[..4].copy_from_slice(&[0x47, 0x40, pid, 0x30]);
        p[4] = 7;
        p[5] = 0x10;
        p[6] = (pcr >> 25) as u8;
        p[7] = (pcr >> 17) as u8;
        p[8] = (pcr >> 9) as u8;
        p[9] = (pcr >> 1) as u8;
        p[10] = ((pcr & 1) as u8) << 7 | 0x7e;
        p[11] = 0;
        let at = 12;
        p[at..at + 9].copy_from_slice(&[0, 0, 1, 0xE0, 0, 0, 0x80, 0xC0, 10]);
        let enc = |marker: u8, v: u64| -> [u8; 5] {
            [marker << 4 | (((v >> 30) & 7) as u8) << 1 | 1, (v >> 22) as u8, (((v >> 15) & 0x7f) as u8) << 1 | 1, (v >> 7) as u8, ((v & 0x7f) as u8) << 1 | 1]
        };
        p[at + 9..at + 14].copy_from_slice(&enc(3, pts));
        p[at + 14..at + 19].copy_from_slice(&enc(1, dts));
        p
    }

    fn read(p: &[u8]) -> (u64, u64, u64) {
        let pcr = (p[6] as u64) << 25 | (p[7] as u64) << 17 | (p[8] as u64) << 9 | (p[9] as u64) << 1 | (p[10] as u64) >> 7;
        let st = |b: &[u8]| ((b[0] as u64 >> 1) & 7) << 30 | (b[1] as u64) << 22 | ((b[2] as u64) >> 1) << 15 | (b[3] as u64) << 7 | (b[4] as u64) >> 1;
        (pcr, st(&p[21..26]), st(&p[26..31]))
    }

    #[test]
    fn rebase_timestamps() {
        // Conversion reprise à 300 s, sous-titres incrustés : l'image vers 301 s, l'audio déjà vers
        // 1 s (Jellyfin). Les deux repartent ensemble à 1,4 s, l'horloge à 0,7 s.
        let video = packet(0x11, 301 * 90_000, 302 * 90_000 + 5, 301 * 90_000 + 7);
        let audio = packet(0x12, 301 * 90_000 + 100, 90_000, 90_000);
        let mut t = TsRebase::new();
        // Coupé en deux morceaux : rien avant le paquet complet.
        assert!(t.push(&video[..100]).is_empty());
        let out = t.push(&video[100..]);
        assert_eq!(out.len(), 188);
        assert_eq!(read(&out), (PCR_LEAD, PES_LEAD + 90_000 - 2, PES_LEAD));
        let out = t.push(&audio);
        assert_eq!(read(&out), (PCR_LEAD + 100, PES_LEAD, PES_LEAD));
        // Marqueurs et indicateurs gardés.
        assert_eq!(out[21] >> 4, 3);
        assert_eq!(out[26] >> 4, 1);
        assert_eq!(out[10] & 0x7f, 0x7e);
        // La suite de chaque piste garde ses écarts.
        let out = t.push(&packet(0x11, 302 * 90_000, 303 * 90_000 + 5, 302 * 90_000 + 7));
        assert_eq!(read(&out), (PCR_LEAD + 90_000, PES_LEAD + 2 * 90_000 - 2, PES_LEAD + 90_000));
    }

    /// Réécrit un vrai fichier MPEG-TS : TS_IN=… TS_OUT=…
    /// cargo test --release relay::tests::rebase_file -- --ignored
    #[test]
    #[ignore]
    fn rebase_file() {
        let data = std::fs::read(std::env::var("TS_IN").unwrap()).unwrap();
        let mut t = TsRebase::new();
        let mut out = Vec::new();
        for c in data.chunks(65_536) {
            out.extend(t.push(c));
        }
        std::fs::write(std::env::var("TS_OUT").unwrap(), out).unwrap();
    }

    #[test]
    fn rebase_resync() {
        // Octets parasites avant le premier paquet : ignorés.
        let mut t = TsRebase::new();
        let mut src = vec![1, 2, 3];
        src.extend(packet(0x11, 10, 20, 30));
        let out = t.push(&src);
        assert_eq!(read(&out), (PCR_LEAD, PES_LEAD, PES_LEAD + 10));
    }
}
