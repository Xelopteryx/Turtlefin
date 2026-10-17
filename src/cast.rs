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
    /// Adresses de commande (complètes) des services AVTransport et RenderingControl.
    pub av_control: String,
    pub rc_control: Option<String>,
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

/// Recherche des télés DLNA pendant `wait`.
pub async fn discover(wait: Duration) -> Vec<Renderer> {
    let Ok(sock) = tokio::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await else { return Vec::new() };
    let msg = "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n\r\n";
    let to: SocketAddr = (Ipv4Addr::new(239, 255, 255, 250), 1900).into();
    for _ in 0..2 {
        let _ = sock.send_to(msg.as_bytes(), to).await;
    }
    let mut locations: Vec<(IpAddr, String)> = Vec::new();
    let end = Instant::now() + wait;
    let mut buf = [0u8; 2048];
    while let Some(left) = end.checked_duration_since(Instant::now()) {
        let Ok(Ok((n, from))) = tokio::time::timeout(left, sock.recv_from(&mut buf)).await else { break };
        if !private(from.ip()) {
            continue;
        }
        let text = String::from_utf8_lossy(&buf[..n]);
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
    Some(Renderer { name, av_control: control(":AVTransport:")?, rc_control: control(":RenderingControl:") })
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// Action SOAP ; renvoie le corps de la réponse.
async fn soap(url: &str, service: &str, action: &str, args: &str) -> Result<String> {
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{action} xmlns:u=\"urn:schemas-upnp-org:service:{service}:1\">{args}</u:{action}></s:Body></s:Envelope>"
    );
    let http = crate::api::http_builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(6)).build()?;
    let resp = http
        .post(url)
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .header("SOAPACTION", format!("\"urn:schemas-upnp-org:service:{service}:1#{action}\""))
        .body(body)
        .send()
        .await?;
    let ok = resp.status().is_success();
    let text = String::from_utf8_lossy(&crate::api::body_limited(resp, 64 << 10).await?).to_string();
    if !ok {
        return Err(anyhow!("la télé refuse ({action})"));
    }
    Ok(text)
}

/// Fait lire cette adresse (flux du serveur) par la télé.
pub async fn play_url(r: &Renderer, url: &str, title: &str) -> Result<()> {
    let meta = format!(
        "<DIDL-Lite xmlns=\"urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:upnp=\"urn:schemas-upnp-org:metadata-1-0/upnp/\"><item id=\"0\" parentID=\"-1\" restricted=\"1\"><dc:title>{}</dc:title><upnp:class>object.item.videoItem</upnp:class><res protocolInfo=\"http-get:*:video/*:*\">{}</res></item></DIDL-Lite>",
        xml_escape(title),
        xml_escape(url)
    );
    let _ = soap(&r.av_control, "AVTransport", "Stop", "<InstanceID>0</InstanceID>").await;
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
