//! Diffuser vers un appareil Google Cast du réseau local (Chromecast, Google TV, télé ou décodeur
//! « Chromecast intégré ») : recherche par mDNS (`_googlecast._tcp`), puis protocole Cast v2 (messages
//! protobuf dans une connexion TLS, port 8009). Turtlefin y lance le lecteur multimédia par défaut de
//! Google (application CC1AD845) et lui donne l'adresse de la vidéo : l'appareil lit lui-même le flux
//! du serveur Jellyfin ; Turtlefin le pilote ensuite (lecture, pause, avance, arrêt, volume).
//!
//! Sécurité : seules les réponses venues d'une adresse privée sont prises, et l'appareil est joint à
//! cette adresse. Le certificat des appareils Cast est auto-signé : il n'est pas vérifié (comme le font
//! les autres applications), la connexion reste limitée au réseau local. Messages bornés en taille.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Appareil Google Cast trouvé sur le réseau.
#[derive(Clone, Debug)]
pub struct Device {
    pub name: String,
    /// Identifiant stable de l'appareil (« id » de son annonce mDNS).
    pub id: String,
    pub addr: SocketAddr,
}

/// Lecteur multimédia par défaut de Google (lit MP4, WebM, HLS…).
const DEFAULT_RECEIVER: &str = "CC1AD845";
const NS_CONN: &str = "urn:x-cast:com.google.cast.tp.connection";
const NS_HEARTBEAT: &str = "urn:x-cast:com.google.cast.tp.heartbeat";
const NS_RECEIVER: &str = "urn:x-cast:com.google.cast.receiver";
const NS_MEDIA: &str = "urn:x-cast:com.google.cast.media";
const SENDER: &str = "sender-turtlefin";
const RECEIVER: &str = "receiver-0";
/// Plus gros message accepté (les états de lecture font quelques kilo-octets).
const MAX_MSG: usize = 256 << 10;

// ---------------------------------------------------------------------------------------------
// Recherche (mDNS)

/// Recherche les appareils Cast pendant `wait` sur chaque réseau local de la machine.
pub async fn discover(wait: Duration) -> Vec<Device> {
    let mut socks = Vec::new();
    for ip in crate::cast::lan_ipv4s() {
        if let Some(s) = crate::cast::multicast_socket(ip) {
            socks.push(s);
        }
    }
    // Question PTR « _googlecast._tcp.local », réponse demandée en direct (bit QU) : pas besoin
    // d'écouter le port 5353 (souvent pris par le système).
    let mut q = vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for part in ["_googlecast", "_tcp", "local"] {
        q.push(part.len() as u8);
        q.extend_from_slice(part.as_bytes());
    }
    q.extend_from_slice(&[0, 0, 12, 0x80, 1]);
    let to: SocketAddr = (Ipv4Addr::new(224, 0, 0, 251), 5353).into();
    for s in &socks {
        let _ = s.send_to(&q, to).await;
    }
    let mut out: Vec<Device> = Vec::new();
    let end = Instant::now() + wait;
    let mut bufs: Vec<[u8; 4096]> = vec![[0u8; 4096]; socks.len()];
    while let Some(left) = end.checked_duration_since(Instant::now()) {
        let recv = futures_util::future::select_all(
            socks.iter().zip(bufs.iter_mut()).map(|(s, b)| Box::pin(async move { s.recv_from(b).await.map(|(n, from)| (n, from, b[..n].to_vec())) })),
        );
        if socks.is_empty() {
            break;
        }
        let Ok((Ok((_, from, data)), _, _)) = tokio::time::timeout(left, recv).await else { break };
        let IpAddr::V4(ip) = from.ip() else { continue };
        if !(ip.is_private() || ip.is_link_local()) {
            continue;
        }
        let Some((name, port, id)) = parse_answer(&data) else { continue };
        let addr = SocketAddr::new(from.ip(), port);
        if out.len() < 16 && !out.iter().any(|d| d.addr == addr) {
            let id = if id.is_empty() { addr.ip().to_string() } else { id };
            out.push(Device { name, id, addr });
        }
    }
    out
}

/// Nom lu dans un message DNS (étiquettes, pointeurs de compression) ; position après le nom.
fn dns_name(msg: &[u8], mut pos: usize) -> Option<(String, usize)> {
    let mut parts = Vec::new();
    let mut end = None;
    for _ in 0..32 {
        let len = *msg.get(pos)? as usize;
        if len == 0 {
            return Some((parts.join("."), end.unwrap_or(pos + 1)));
        }
        if len & 0xc0 == 0xc0 {
            let ptr = ((len & 0x3f) << 8) | *msg.get(pos + 1)? as usize;
            end.get_or_insert(pos + 2);
            pos = ptr;
            continue;
        }
        parts.push(String::from_utf8_lossy(msg.get(pos + 1..pos + 1 + len)?).into_owned());
        pos += 1 + len;
    }
    None
}

/// Réponse mDNS d'un appareil Cast : son nom (« fn » du TXT), son port (SRV, 8009 sinon) et son
/// identifiant (« id » du TXT, vide s'il manque).
fn parse_answer(msg: &[u8]) -> Option<(String, u16, String)> {
    let count = |i: usize| -> Option<usize> { Some(u16::from_be_bytes([*msg.get(i)?, *msg.get(i + 1)?]) as usize) };
    let (qd, records) = (count(4)?, count(6)? + count(8)? + count(10)?);
    let mut pos = 12;
    for _ in 0..qd {
        pos = dns_name(msg, pos)?.1 + 4;
    }
    let (mut name, mut port, mut cast, mut id) = (None, None, false, String::new());
    for _ in 0..records.min(64) {
        let (owner, p) = dns_name(msg, pos)?;
        let kind = count(p)?;
        let len = count(p + 8)?;
        let data = msg.get(p + 10..p + 10 + len)?;
        if owner.to_ascii_lowercase().contains("_googlecast._tcp") {
            cast = true;
            match kind {
                // TXT : suites « longueur, clé=valeur ».
                16 => {
                    let mut i = 0;
                    while i < data.len() {
                        let l = data[i] as usize;
                        let entry = String::from_utf8_lossy(data.get(i + 1..i + 1 + l).unwrap_or_default());
                        if let Some(v) = entry.strip_prefix("fn=") {
                            name = Some(v.chars().filter(|c| !c.is_control()).take(60).collect::<String>());
                        } else if let Some(v) = entry.strip_prefix("id=") {
                            id = v.chars().filter(|c| c.is_ascii_alphanumeric()).take(64).collect();
                        }
                        i += 1 + l;
                    }
                }
                // SRV : priorité, poids, port, cible.
                33 if data.len() >= 6 => port = Some(u16::from_be_bytes([data[4], data[5]])),
                _ => {}
            }
        }
        pos = p + 10 + len;
    }
    cast.then(|| (name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "Chromecast".into()), port.unwrap_or(8009), id))
}

// ---------------------------------------------------------------------------------------------
// Connexion Cast v2

/// Certificats non vérifiés : ceux des appareils Cast sont auto-signés (voir l'en-tête du module).
#[derive(Debug)]
struct AnyCert(Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for AnyCert {
    fn verify_server_cert(
        &self,
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>,
        _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// Message Cast (protobuf CastMessage) : version 0, source, destination, espace, texte JSON.
fn encode(src: &str, dst: &str, ns: &str, payload: &str) -> Vec<u8> {
    fn varint(out: &mut Vec<u8>, mut v: u64) {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                break;
            }
            out.push(b | 0x80);
        }
    }
    fn field(out: &mut Vec<u8>, tag: u8, s: &str) {
        out.push(tag);
        varint(out, s.len() as u64);
        out.extend_from_slice(s.as_bytes());
    }
    let mut m = vec![0x08, 0x00];
    field(&mut m, 0x12, src);
    field(&mut m, 0x1a, dst);
    field(&mut m, 0x22, ns);
    m.extend_from_slice(&[0x28, 0x00]);
    field(&mut m, 0x32, payload);
    let mut out = (m.len() as u32).to_be_bytes().to_vec();
    out.extend(m);
    out
}

/// Champs utiles d'un message reçu : source, espace, texte JSON.
fn decode(m: &[u8]) -> Option<(String, String, String)> {
    let (mut src, mut ns, mut payload) = (String::new(), String::new(), String::new());
    let mut i = 0;
    let varint = |i: &mut usize| -> Option<u64> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let b = *m.get(*i)?;
            *i += 1;
            v |= ((b & 0x7f) as u64) << shift;
            if b & 0x80 == 0 {
                return Some(v);
            }
        }
        None
    };
    while i < m.len() {
        let key = varint(&mut i)?;
        match key & 7 {
            0 => {
                varint(&mut i)?;
            }
            2 => {
                let len = varint(&mut i)? as usize;
                let bytes = m.get(i..i + len)?;
                i += len;
                let s = String::from_utf8_lossy(bytes).into_owned();
                match key >> 3 {
                    2 => src = s,
                    4 => ns = s,
                    6 => payload = s,
                    _ => {}
                }
            }
            _ => return None,
        }
    }
    Some((src, ns, payload))
}

struct Conn {
    tls: tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
    next_id: i64,
}

impl Conn {
    async fn open(dev: &Device) -> Result<Conn> {
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let cfg = rustls::ClientConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AnyCert(provider)))
            .with_no_client_auth();
        let tcp = tokio::time::timeout(Duration::from_secs(4), tokio::net::TcpStream::connect(dev.addr)).await??;
        let name = rustls::pki_types::ServerName::IpAddress(dev.addr.ip().into());
        let tls = tokio::time::timeout(Duration::from_secs(4), tokio_rustls::TlsConnector::from(Arc::new(cfg)).connect(name, tcp)).await??;
        let mut c = Conn { tls, next_id: 1 };
        c.send(RECEIVER, NS_CONN, json!({ "type": "CONNECT" })).await?;
        Ok(c)
    }

    async fn send(&mut self, dst: &str, ns: &str, v: Value) -> Result<()> {
        self.tls.write_all(&encode(SENDER, dst, ns, &v.to_string())).await?;
        Ok(())
    }

    /// Prochain message (les « PING » de l'appareil reçoivent leur « PONG » au passage).
    async fn recv(&mut self) -> Result<(String, String, Value)> {
        loop {
            let mut len = [0u8; 4];
            self.tls.read_exact(&mut len).await?;
            let n = u32::from_be_bytes(len) as usize;
            if n > MAX_MSG {
                return Err(anyhow!("message Cast trop grand"));
            }
            let mut buf = vec![0u8; n];
            self.tls.read_exact(&mut buf).await?;
            let Some((src, ns, payload)) = decode(&buf) else { continue };
            let v: Value = serde_json::from_str(&payload).unwrap_or(Value::Null);
            if ns == NS_HEARTBEAT && v["type"] == "PING" {
                self.send(&src, NS_HEARTBEAT, json!({ "type": "PONG" })).await?;
                continue;
            }
            return Ok((src, ns, v));
        }
    }

    /// Requête avec numéro, et sa réponse (5 s au plus).
    async fn request(&mut self, dst: &str, ns: &str, mut v: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        v["requestId"] = json!(id);
        self.send(dst, ns, v).await?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let left = deadline.checked_duration_since(Instant::now()).ok_or_else(|| anyhow!("l'appareil ne répond pas"))?;
            let (_, rns, r) = tokio::time::timeout(left, self.recv()).await.map_err(|_| anyhow!("l'appareil ne répond pas"))??;
            if rns == ns && r["requestId"].as_i64() == Some(id) {
                return Ok(r);
            }
        }
    }

    /// État de l'appareil : application en cours (le lecteur par défaut ?) et volume.
    async fn receiver_status(&mut self) -> Result<Value> {
        Ok(self.request(RECEIVER, NS_RECEIVER, json!({ "type": "GET_STATUS" })).await?["status"].clone())
    }

    /// Lecteur par défaut ouvert sur l'appareil (lancé si besoin) : son identifiant de transport.
    async fn media_app(&mut self, launch: bool) -> Result<Option<String>> {
        let find = |st: &Value| -> Option<String> {
            st["applications"].as_array()?.iter().find(|a| a["appId"] == DEFAULT_RECEIVER)?["transportId"].as_str().map(str::to_string)
        };
        let mut tid = find(&self.receiver_status().await?);
        if tid.is_none() && launch {
            let r = self.request(RECEIVER, NS_RECEIVER, json!({ "type": "LAUNCH", "appId": DEFAULT_RECEIVER })).await?;
            tid = find(&r["status"]);
            // Lancement lent : l'état suivant porte l'application.
            for _ in 0..6 {
                if tid.is_some() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
                tid = find(&self.receiver_status().await?);
            }
            if tid.is_none() {
                return Err(anyhow!("le lecteur de l'appareil ne s'est pas ouvert"));
            }
        }
        if let Some(t) = &tid {
            self.send(t, NS_CONN, json!({ "type": "CONNECT" })).await?;
        }
        Ok(tid)
    }

    /// Lecture en cours : (numéro de session, état « PLAYING / PAUSED / BUFFERING / IDLE », position, durée).
    async fn media_status(&mut self, tid: &str) -> Result<Option<(i64, String, f64, f64)>> {
        let r = self.request(tid, NS_MEDIA, json!({ "type": "GET_STATUS" })).await?;
        let Some(st) = r["status"].as_array().and_then(|a| a.first()) else { return Ok(None) };
        let Some(id) = st["mediaSessionId"].as_i64() else { return Ok(None) };
        Ok(Some((
            id,
            st["playerState"].as_str().unwrap_or("").to_string(),
            st["currentTime"].as_f64().unwrap_or(0.0),
            st["media"]["duration"].as_f64().unwrap_or(0.0),
        )))
    }
}

/// Piste de sous-titres WebVTT donnée à l'appareil : numéro (> 0), adresse, nom, langue.
pub struct TextTrack {
    pub id: i32,
    pub url: String,
    pub name: String,
    pub lang: Option<String>,
}

/// Fait lire `url` par l'appareil (type MIME `mime`), à partir de `start` secondes.
/// `subs` : sous-titres WebVTT de la vidéo, tous donnés à l'appareil (on passe de l'un à l'autre
/// ensuite sans recharger, voir set_text_track) ; `active` : celui affiché.
pub async fn play_url(dev: &Device, url: &str, mime: &str, title: &str, start: Option<f64>, subs: &[TextTrack], active: Option<i32>) -> Result<()> {
    let mut c = Conn::open(dev).await?;
    let tid = c.media_app(true).await?.ok_or_else(|| anyhow!("lecteur indisponible"))?;
    let mut load = json!({
        "type": "LOAD",
        "autoplay": true,
        "media": {
            "contentId": url,
            "contentUrl": url,
            "streamType": "BUFFERED",
            "contentType": mime,
            "metadata": { "type": 0, "metadataType": 0, "title": title },
        },
    });
    if let Some(s) = start {
        load["currentTime"] = json!(s);
    }
    if !subs.is_empty() {
        load["media"]["tracks"] = subs
            .iter()
            .map(|t| {
                let mut j = json!({ "trackId": t.id, "type": "TEXT", "trackContentId": t.url, "trackContentType": "text/vtt", "subtype": "SUBTITLES", "name": t.name });
                if let Some(l) = &t.lang {
                    j["language"] = json!(l);
                }
                j
            })
            .collect();
        load["activeTrackIds"] = json!(active.into_iter().collect::<Vec<_>>());
    }
    let r = c.request(&tid, NS_MEDIA, load).await?;
    match r["type"].as_str() {
        Some("LOAD_FAILED" | "LOAD_CANCELLED" | "INVALID_REQUEST") => Err(anyhow!("la vidéo a été refusée par l'appareil")),
        _ => Ok(()),
    }
}

/// Sous-titres affichés : une des pistes données à la lecture (play_url), ou aucune ; sans recharger.
pub async fn set_text_track(dev: &Device, track: Option<i32>) -> Result<()> {
    let mut c = Conn::open(dev).await?;
    let tid = c.media_app(false).await?.ok_or_else(|| anyhow!("rien en lecture"))?;
    let (id, ..) = c.media_status(&tid).await?.ok_or_else(|| anyhow!("rien en lecture"))?;
    let ids: Vec<i32> = track.into_iter().collect();
    let r = c.request(&tid, NS_MEDIA, json!({ "type": "EDIT_TRACKS_INFO", "mediaSessionId": id, "activeTrackIds": ids })).await?;
    match r["type"].as_str() {
        Some("INVALID_REQUEST" | "LOAD_FAILED") => Err(anyhow!("piste refusée par l'appareil")),
        _ => Ok(()),
    }
}

/// Commande : « play », « pause », « stop », « seek » (secondes), « vol » (0 à 100), « mute ».
pub async fn control(dev: &Device, cmd: &str, value: f64) -> Result<()> {
    let mut c = Conn::open(dev).await?;
    match cmd {
        "vol" => {
            c.send(RECEIVER, NS_RECEIVER, json!({ "type": "SET_VOLUME", "requestId": 900, "volume": { "level": (value / 100.0).clamp(0.0, 1.0) } })).await?;
        }
        "mute" => {
            let muted = c.receiver_status().await?["volume"]["muted"].as_bool().unwrap_or(false);
            c.send(RECEIVER, NS_RECEIVER, json!({ "type": "SET_VOLUME", "requestId": 901, "volume": { "muted": !muted } })).await?;
        }
        _ => {
            let Some(tid) = c.media_app(false).await? else { return Ok(()) };
            let Some((id, ..)) = c.media_status(&tid).await? else { return Ok(()) };
            let msg = match cmd {
                "play" => json!({ "type": "PLAY", "mediaSessionId": id }),
                "pause" => json!({ "type": "PAUSE", "mediaSessionId": id }),
                "stop" => json!({ "type": "STOP", "mediaSessionId": id }),
                "seek" => json!({ "type": "SEEK", "mediaSessionId": id, "currentTime": value.max(0.0) }),
                _ => return Ok(()),
            };
            c.request(&tid, NS_MEDIA, msg).await?;
        }
    }
    Ok(())
}

/// État : (en pause, position, durée, volume 0 à 100).
pub async fn status(dev: &Device) -> Option<(bool, f64, f64, Option<u32>)> {
    let mut c = Conn::open(dev).await.ok()?;
    let st = c.receiver_status().await.ok()?;
    let vol = st["volume"]["level"].as_f64().map(|l| (l * 100.0).round().clamp(0.0, 100.0) as u32);
    let playing = st["applications"].as_array().is_some_and(|a| a.iter().any(|x| x["appId"] == DEFAULT_RECEIVER));
    if !playing {
        return Some((true, 0.0, 0.0, vol));
    }
    let tid = c.media_app(false).await.ok()??;
    match c.media_status(&tid).await.ok()? {
        Some((_, state, pos, dur)) => Some((state == "PAUSED", pos, dur, vol)),
        None => Some((true, 0.0, 0.0, vol)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Essai sur un vrai appareil : CAST_ADDR=ip:8009 CAST_URL=… [CAST_MIME=…] [CAST_VTT=…]
    /// cargo test --release chromecast::tests::real_device -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_device() {
        let addr = std::env::var("CAST_ADDR").unwrap().parse().unwrap();
        let url = std::env::var("CAST_URL").unwrap();
        let mime = std::env::var("CAST_MIME").unwrap_or_else(|_| "application/x-mpegURL".into());
        let vtt = std::env::var("CAST_VTT").ok();
        if std::env::var("CAST_STOP").is_ok() {
            let dev = Device { name: "essai".into(), id: String::new(), addr };
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async { control(&dev, "stop", 0.0).await.unwrap() });
            return;
        }
        let dev = Device { name: "essai".into(), id: String::new(), addr };
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let subs: Vec<TextTrack> = vtt.into_iter().map(|url| TextTrack { id: 1, url, name: "Sous-titres".into(), lang: None }).collect();
            play_url(&dev, &url, &mime, "Essai Turtlefin", Some(60.0), &subs, subs.first().map(|t| t.id)).await.unwrap();
            for _ in 0..6 {
                tokio::time::sleep(Duration::from_secs(4)).await;
                println!("état : {:?}", status(&dev).await);
            }
            control(&dev, "stop", 0.0).await.unwrap();
            println!("arrêt : {:?}", status(&dev).await);
        });
    }

    #[test]
    fn message_round_trip() {
        let m = encode("a", "b", NS_MEDIA, r#"{"type":"PING"}"#);
        assert_eq!(u32::from_be_bytes([m[0], m[1], m[2], m[3]]) as usize, m.len() - 4);
        let (src, ns, p) = decode(&m[4..]).unwrap();
        assert_eq!((src.as_str(), ns.as_str(), p.as_str()), ("a", NS_MEDIA, r#"{"type":"PING"}"#));
    }

    #[test]
    fn mdns_answer() {
        // Réponse : PTR vers « Salon._googlecast._tcp.local », TXT fn=Salon, SRV port 8009.
        let mut m = vec![0, 0, 0x84, 0, 0, 0, 0, 3, 0, 0, 0, 0];
        let name = |m: &mut Vec<u8>, parts: &[&str]| {
            for p in parts {
                m.push(p.len() as u8);
                m.extend_from_slice(p.as_bytes());
            }
            m.push(0);
        };
        name(&mut m, &["_googlecast", "_tcp", "local"]);
        let mut inst = Vec::new();
        name(&mut inst, &["Salon", "_googlecast", "_tcp", "local"]);
        m.extend_from_slice(&[0, 12, 0, 1, 0, 0, 0, 120, 0, inst.len() as u8]);
        m.extend_from_slice(&inst);
        m.extend_from_slice(&inst);
        let txt = b"\x08fn=Salon\x05id=ab";
        m.extend_from_slice(&[0, 16, 0x80, 1, 0, 0, 0, 120, 0, txt.len() as u8]);
        m.extend_from_slice(txt);
        m.extend_from_slice(&inst);
        m.extend_from_slice(&[0, 33, 0x80, 1, 0, 0, 0, 120, 0, 8, 0, 0, 0, 0, 0x1f, 0x49, 0xc0, 12]);
        assert_eq!(parse_answer(&m), Some(("Salon".to_string(), 8009, "ab".to_string())));
        assert_eq!(parse_answer(&[0; 12]), None);
    }
}
