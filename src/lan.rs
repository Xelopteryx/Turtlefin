//! Réseau local : adresses de la machine, prises multidiffusées par carte (recherche des écrans
//! Google Cast, relais de diffusion), et test d'un serveur joint par le réseau d'un écran.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

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
