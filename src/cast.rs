//! Diffuser vers une télé DLNA (UPnP MediaRenderer) du réseau local : recherche (SSDP), envoi de
//! l'adresse de la vidéo (AVTransport : SetAVTransportURI puis Play), commandes de lecture et volume
//! (RenderingControl). La télé lit elle-même le flux du serveur Jellyfin.
//!
//! Sécurité : seules les réponses venues d'une adresse privée (réseau local) sont prises, et leur
//! description n'est lue qu'à cette même adresse (une réponse ne fait pas contacter un autre appareil).
//! Réponses et descriptions bornées en taille ; textes de la télé nettoyés avant affichage.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};

/// Télé trouvée sur le réseau.
#[derive(Clone, Debug)]
pub struct Renderer {
    pub name: String,
    /// Identifiant stable de la télé (UDN de sa description) : garde son nom choisi dans Turtlefin.
    pub id: String,
    /// Adresses de commande (complètes) des services AVTransport, RenderingControl et ConnectionManager.
    pub av_control: String,
    pub rc_control: Option<String>,
    pub cm_control: Option<String>,
}

fn private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v.is_private() || v.is_link_local(),
        IpAddr::V6(_) => false,
    }
}

/// Valeur d'un en-tête (insensible à la casse) dans une réponse SSDP.
fn header<'a>(resp: &'a str, name: &str) -> Option<&'a str> {
    resp.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.trim().eq_ignore_ascii_case(name).then(|| v.trim())
    })
}

/// Texte d'une balise XML (première occurrence après `from`), sans entités.
fn tag<'a>(xml: &'a str, name: &str, from: usize) -> Option<(&'a str, usize)> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let a = xml[from..].find(&open)? + from + open.len();
    let b = xml[a..].find(&close)? + a;
    Some((&xml[a..b], b))
}

fn clean(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).take(60).collect::<String>().replace("&amp;", "&").trim().to_string()
}

/// Adresses IPv4 de la machine sur ses réseaux locaux (cartes actives, adresse privée, hors VPN en /32) :
/// la recherche part de chacune (sinon Windows choisit une seule carte, parfois celle d'une machine
/// virtuelle, et rien ne répond).
pub fn lan_ipv4s() -> Vec<Ipv4Addr> {
    let mut out = Vec::new();
    for itf in if_addrs::get_if_addrs().unwrap_or_default() {
        let if_addrs::IfAddr::V4(a) = &itf.addr else { continue };
        if a.ip.is_private() && !a.ip.is_loopback() && a.prefixlen < 31 && itf.is_oper_up() && !out.contains(&a.ip) {
            out.push(a.ip);
        }
    }
    if out.is_empty() {
        out.push(Ipv4Addr::UNSPECIFIED);
    }
    out
}

/// Prise UDP liée à l'adresse `ip`, qui envoie ses messages multidiffusés par la carte de cette adresse.
pub fn multicast_socket(ip: Ipv4Addr) -> Option<tokio::net::UdpSocket> {
    let s = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, Some(socket2::Protocol::UDP)).ok()?;
    // Android, VPN actif (Tailscale) : la recherche passe par le Wi-Fi, pas par le VPN.
    #[cfg(target_os = "android")]
    crate::android_entry::bind_to_lan(std::os::fd::AsRawFd::as_raw_fd(&s));
    if !ip.is_unspecified() {
        s.set_multicast_if_v4(&ip).ok()?;
    }
    s.set_multicast_ttl_v4(2).ok()?;
    s.bind(&SocketAddr::from((ip, 0)).into()).ok()?;
    s.set_nonblocking(true).ok()?;
    tokio::net::UdpSocket::from_std(s.into()).ok()
}

/// L'adresse `base` (serveur) se joint-elle par le réseau local de l'écran `screen` lui-même, sans
/// VPN ? Connexion d'essai partie de l'adresse de cette machine sur ce réseau (Android : attachée au
/// Wi-Fi). Tailscale peut faire suivre tout le réseau de la maison (routage de sous-réseau) : chez
/// quelqu'un, l'adresse 192.168.1.x du serveur répondait alors par le VPN, et la télé recevait une
/// adresse qu'elle ne pouvait pas joindre.
pub async fn lan_reachable(base: &str, screen: Ipv4Addr) -> bool {
    let Ok(u) = reqwest::Url::parse(base) else { return false };
    let Some(Ok(host)) = u.host_str().map(|h| h.parse::<Ipv4Addr>()) else { return false };
    let port = u.port_or_known_default().unwrap_or(80);
    let Some(me) = lan_ipv4s().into_iter().find(|ip| ip.octets()[..3] == screen.octets()[..3]) else { return false };
    tokio::task::spawn_blocking(move || -> bool {
        let Ok(s) = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::STREAM, Some(socket2::Protocol::TCP)) else { return false };
        #[cfg(target_os = "android")]
        crate::android_entry::bind_to_lan(std::os::fd::AsRawFd::as_raw_fd(&s));
        s.bind(&SocketAddr::from((me, 0)).into()).is_ok() && s.connect_timeout(&SocketAddr::from((host, port)).into(), Duration::from_millis(1500)).is_ok()
    })
    .await
    .unwrap_or(false)
}

/// Recherche des télés DLNA pendant `wait`.
pub async fn discover(wait: Duration) -> Vec<Renderer> {
    let socks: Vec<_> = lan_ipv4s().into_iter().filter_map(multicast_socket).collect();
    let msg = "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n\r\n";
    let to: SocketAddr = (Ipv4Addr::new(239, 255, 255, 250), 1900).into();
    for _ in 0..2 {
        for sock in &socks {
            let _ = sock.send_to(msg.as_bytes(), to).await;
        }
    }
    let mut locations: Vec<(IpAddr, String)> = Vec::new();
    let end = Instant::now() + wait;
    let mut bufs: Vec<[u8; 2048]> = vec![[0u8; 2048]; socks.len()];
    while let Some(left) = end.checked_duration_since(Instant::now()) {
        if socks.is_empty() {
            break;
        }
        let recv = futures_util::future::select_all(
            socks.iter().zip(bufs.iter_mut()).map(|(s, b)| Box::pin(async move { s.recv_from(b).await.map(|(n, from)| (from, b[..n].to_vec())) })),
        );
        let Ok((Ok((from, data)), _, _)) = tokio::time::timeout(left, recv).await else { break };
        if !private(from.ip()) {
            continue;
        }
        let text = String::from_utf8_lossy(&data);
        let Some(loc) = header(&text, "location") else { continue };
        // La description doit venir de l'appareil qui a répondu (pas d'une autre adresse).
        let Ok(url) = reqwest::Url::parse(loc) else { continue };
        if url.scheme() != "http" || url.host_str().and_then(|h| h.parse::<IpAddr>().ok()) != Some(from.ip()) {
            continue;
        }
        if locations.len() < 16 && !locations.iter().any(|(_, l)| l == loc) {
            locations.push((from.ip(), loc.to_string()));
        }
    }
    let http = match crate::api::http_builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(3)).build() {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for (_, loc) in locations {
        if let Some(r) = describe(&http, &loc).await {
            if !out.iter().any(|o: &Renderer| o.av_control == r.av_control) {
                out.push(r);
            }
        }
    }
    out
}

/// Lit la description d'une télé : nom, adresses de commande.
async fn describe(http: &reqwest::Client, loc: &str) -> Option<Renderer> {
    let resp = http.get(loc).send().await.ok()?;
    let body = crate::api::body_limited(resp, 256 << 10).await.ok()?;
    let xml = String::from_utf8_lossy(&body).to_string();
    let base = reqwest::Url::parse(loc).ok()?;
    let name = tag(&xml, "friendlyName", 0).map(|(n, _)| clean(n)).filter(|n| !n.is_empty()).unwrap_or_else(|| "Télé DLNA".into());
    let control = |service: &str| -> Option<String> {
        let mut at = 0;
        while let Some((ty, end)) = tag(&xml, "serviceType", at) {
            at = end;
            if ty.contains(service) {
                let (c, _) = tag(&xml, "controlURL", end)?;
                let url = base.join(c.trim()).ok()?;
                // Commandes à la même adresse que la description.
                return (url.host_str() == base.host_str() && url.scheme() == "http").then(|| url.to_string());
            }
        }
        None
    };
    let id = tag(&xml, "UDN", 0).map(|(u, _)| clean(u)).filter(|u| !u.is_empty()).unwrap_or_else(|| base.host_str().unwrap_or("").to_string());
    Some(Renderer { name, id, av_control: control(":AVTransport:")?, rc_control: control(":RenderingControl:"), cm_control: control(":ConnectionManager:") })
}

/// La télé lit-elle le HEVC (H.265) ? D'après les formats qu'elle déclare (GetProtocolInfo) ; sans
/// réponse, non (une télé de 2015 comme la Samsung J5200 ne lit que le H.264, et refuse sans rien dire).
pub async fn supports_hevc(r: &Renderer) -> bool {
    let Some(cm) = &r.cm_control else { return false };
    let Ok(t) = soap_in(cm, "ConnectionManager", "GetProtocolInfo", "", 4).await else { return false };
    let sink = tag(&t, "Sink", 0).map(|(s, _)| s.to_ascii_uppercase()).unwrap_or_default();
    ["HEVC", "H265", "H.265"].iter().any(|k| sink.contains(k))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// Action SOAP ; renvoie le corps de la réponse.
async fn soap(url: &str, service: &str, action: &str, args: &str) -> Result<String> {
    // Une télé ne répond à SetAVTransportURI / Play qu'après avoir ouvert le flux : avec une
    // conversion du serveur qui démarre, plus de 10 s (Samsung).
    let wait = if matches!(action, "SetAVTransportURI" | "Play") { 30 } else { 6 };
    soap_in(url, service, action, args, wait).await
}

/// Action SOAP, réponse attendue `wait` secondes au plus.
async fn soap_in(url: &str, service: &str, action: &str, args: &str, wait: u64) -> Result<String> {
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{action} xmlns:u=\"urn:schemas-upnp-org:service:{service}:1\">{args}</u:{action}></s:Body></s:Envelope>"
    );
    let http = crate::api::http_builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(wait)).build()?;
    let resp = http
        .post(url)
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .header("SOAPACTION", format!("\"urn:schemas-upnp-org:service:{service}:1#{action}\""))
        .body(body)
        .send()
        .await
        .map_err(|e| anyhow!("la télé ne répond pas ({action}) : {e}"))?;
    let ok = resp.status().is_success();
    let text = String::from_utf8_lossy(&crate::api::body_limited(resp, 64 << 10).await?).to_string();
    if !ok {
        // Raison donnée par la télé (UPnP : errorCode / errorDescription), si elle en donne une.
        let why = tag(&text, "errorDescription", 0).map(|(d, _)| clean(d)).filter(|d| !d.is_empty())
            .or_else(|| tag(&text, "errorCode", 0).map(|(c, _)| format!("code {}", clean(c))));
        return Err(match why {
            Some(w) => anyhow!("la télé refuse ({action} : {w})"),
            None => anyhow!("la télé refuse ({action})"),
        });
    }
    Ok(text)
}

/// Fait lire cette adresse (flux du serveur) par la télé.
/// `mime` : type exact du flux (les télés Samsung refusent un « video/* » vague et restent arrêtées).
/// `subs` : adresse d'un fichier de sous-titres SRT, donné à la télé à côté de la vidéo
/// (CaptionInfoEx des télés Samsung, compris aussi par d'autres).
pub async fn play_url(r: &Renderer, url: &str, mime: &str, title: &str, subs: Option<&str>) -> Result<()> {
    let caption = subs
        .map(|s| {
            let s = xml_escape(s);
            format!("<sec:CaptionInfoEx sec:type=\"srt\">{s}</sec:CaptionInfoEx><sec:CaptionInfo sec:type=\"srt\">{s}</sec:CaptionInfo>")
        })
        .unwrap_or_default();
    let sub_res = subs.map(|s| format!("<res protocolInfo=\"http-get:*:text/srt:*\">{}</res>", xml_escape(s))).unwrap_or_default();
    let flags = if mime == "video/mpeg" { "DLNA.ORG_OP=00;DLNA.ORG_CI=1;DLNA.ORG_FLAGS=01700000000000000000000000000000" } else { "DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000" };
    let meta = format!(
        "<DIDL-Lite xmlns=\"urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:upnp=\"urn:schemas-upnp-org:metadata-1-0/upnp/\" xmlns:sec=\"http://www.sec.co.kr/\"><item id=\"0\" parentID=\"-1\" restricted=\"1\"><dc:title>{}</dc:title><upnp:class>object.item.videoItem</upnp:class>{caption}<res protocolInfo=\"http-get:*:{}:{}\">{}</res>{sub_res}</item></DIDL-Lite>",
        xml_escape(title),
        mime,
        flags,
        xml_escape(url)
    );
    let _ = soap(&r.av_control, "AVTransport", "Stop", "<InstanceID>0</InstanceID>").await;
    // Une vidéo arrive aussitôt après l'arrêt de la précédente (changement de piste) : une Samsung
    // garde alors l'image figée (le son passe). Elle a besoin d'un instant pour libérer son décodeur.
    tokio::time::sleep(Duration::from_millis(1200)).await;
    soap(
        &r.av_control,
        "AVTransport",
        "SetAVTransportURI",
        &format!("<InstanceID>0</InstanceID><CurrentURI>{}</CurrentURI><CurrentURIMetaData>{}</CurrentURIMetaData>", xml_escape(url), xml_escape(&meta)),
    )
    .await?;
    soap(&r.av_control, "AVTransport", "Play", "<InstanceID>0</InstanceID><Speed>1</Speed>").await?;
    Ok(())
}

fn hms(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

fn secs(hms: &str) -> f64 {
    hms.split(':').fold(0.0, |acc, p| acc * 60.0 + p.trim().parse::<f64>().unwrap_or(0.0))
}

/// Commande de lecture : "play", "pause", "stop", "seek" (secondes), "vol" (0 à 100).
pub async fn control(r: &Renderer, cmd: &str, value: f64) -> Result<()> {
    match cmd {
        "play" => soap(&r.av_control, "AVTransport", "Play", "<InstanceID>0</InstanceID><Speed>1</Speed>").await.map(|_| ()),
        "pause" => soap(&r.av_control, "AVTransport", "Pause", "<InstanceID>0</InstanceID>").await.map(|_| ()),
        "stop" => soap(&r.av_control, "AVTransport", "Stop", "<InstanceID>0</InstanceID>").await.map(|_| ()),
        "seek" => soap(&r.av_control, "AVTransport", "Seek", &format!("<InstanceID>0</InstanceID><Unit>REL_TIME</Unit><Target>{}</Target>", hms(value))).await.map(|_| ()),
        "vol" => match &r.rc_control {
            Some(rc) => soap(rc, "RenderingControl", "SetVolume", &format!("<InstanceID>0</InstanceID><Channel>Master</Channel><DesiredVolume>{}</DesiredVolume>", value.clamp(0.0, 100.0) as u32)).await.map(|_| ()),
            None => Ok(()),
        },
        _ => Ok(()),
    }
}

/// État de la lecture : (en pause, position, durée, volume).
pub async fn status(r: &Renderer) -> Option<(bool, f64, f64, Option<u32>)> {
    let t = soap(&r.av_control, "AVTransport", "GetTransportInfo", "<InstanceID>0</InstanceID>").await.ok()?;
    let state = tag(&t, "CurrentTransportState", 0).map(|(s, _)| s.to_string()).unwrap_or_default();
    let p = soap(&r.av_control, "AVTransport", "GetPositionInfo", "<InstanceID>0</InstanceID>").await.ok()?;
    let pos = tag(&p, "RelTime", 0).map(|(s, _)| secs(s)).unwrap_or(0.0);
    let dur = tag(&p, "TrackDuration", 0).map(|(s, _)| secs(s)).unwrap_or(0.0);
    let vol = match &r.rc_control {
        Some(rc) => soap(rc, "RenderingControl", "GetVolume", "<InstanceID>0</InstanceID><Channel>Master</Channel>").await.ok().and_then(|v| tag(&v, "CurrentVolume", 0).and_then(|(s, _)| s.trim().parse().ok())),
        None => None,
    };
    Some((state != "PLAYING", pos, dur, vol))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Essai réel (réseau de la machine) : LANR_BASE=http://ip:port LANR_SCREEN=ip
    /// cargo test --release cast::tests::lan_reach -- --ignored --nocapture
    #[test]
    #[ignore]
    fn lan_reach() {
        let base = std::env::var("LANR_BASE").unwrap();
        let screen = std::env::var("LANR_SCREEN").unwrap().parse().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        println!("{base} depuis le réseau de {screen} : {}", rt.block_on(lan_reachable(&base, screen)));
    }

    #[test]
    fn parsing() {
        let r = "HTTP/1.1 200 OK\r\nLOCATION: http://192.168.1.20:1234/desc.xml\r\nST: x\r\n\r\n";
        assert_eq!(header(r, "location"), Some("http://192.168.1.20:1234/desc.xml"));
        assert_eq!(tag("<a><b>x</b></a>", "b", 0).map(|t| t.0), Some("x"));
        assert_eq!(secs("01:02:03"), 3723.0);
        assert_eq!(hms(3723.4), "01:02:03");
        assert!(private("192.168.1.2".parse().unwrap()));
        assert!(!private("8.8.8.8".parse().unwrap()));
        assert_eq!(xml_escape("<a&b>"), "&lt;a&amp;b&gt;");
    }
}
