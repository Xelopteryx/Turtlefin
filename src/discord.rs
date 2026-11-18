//! Statut Discord (« Rich Presence », bureau seulement, coupé par défaut) : la page ouverte et le média
//! regardé s'affichent sur le profil Discord. Paramètres → Affichage → Partage.
//!
//! Discord (l'application de bureau) écoute un tube local : `\\.\pipe\discord-ipc-N` sous Windows,
//! `$XDG_RUNTIME_DIR/discord-ipc-N` sous Linux (aussi dans les dossiers de Flatpak et de Snap). Chaque
//! message : code (u32), longueur (u32), JSON. Poignée de main (code 0) avec l'identifiant de
//! l'application Discord, puis SET_ACTIVITY (code 1). Rien ne passe par Internet depuis Turtlefin, et
//! l'adresse du serveur n'est jamais envoyée.
//!
//! Un fil relit l'état de l'interface toutes les 5 s et n'envoie qu'un statut qui a changé (Discord
//! limite à 5 mises à jour par 20 s). Discord fermé : nouvel essai à la vérification suivante.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::i18n::{tr, trf};
use crate::AppWindow;

/// Identifiant de l'application Discord « Turtlefin » (portail des développeurs de Discord). Vide : le
/// statut reste coupé. `TURTLEFIN_DISCORD_ID` le remplace (essais).
const APP_ID: &str = "";

static ON: AtomicBool = AtomicBool::new(false);
static STARTED: AtomicBool = AtomicBool::new(false);

fn app_id() -> String {
    std::env::var("TURTLEFIN_DISCORD_ID").ok().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| APP_ID.to_string())
}

/// Le statut peut-il marcher ici (identifiant connu) ?
pub fn available() -> bool {
    !app_id().is_empty()
}

/// Statut activé ou coupé (réglage). Le premier appel lance le fil de mise à jour.
pub fn set_enabled(ui: &slint::Weak<AppWindow>, on: bool) {
    ON.store(on && available(), Ordering::SeqCst);
    if ON.load(Ordering::SeqCst) && !STARTED.swap(true, Ordering::SeqCst) {
        let ui = ui.clone();
        std::thread::Builder::new().name("discord".into()).spawn(move || run(ui)).ok();
    }
}

/// Ce que montre le statut : (première ligne, seconde ligne, début de la lecture en secondes Unix).
#[derive(Clone, PartialEq, Default)]
struct Status {
    details: String,
    state: String,
    start: Option<u64>,
    end: Option<u64>,
}

/// État de l'interface traduit en statut (fil de l'interface).
fn read_status(u: &AppWindow) -> Status {
    if u.get_playing() {
        let title = u.get_p_title().to_string();
        let sub = u.get_p_subtitle().to_string();
        let (pos, dur) = (u.get_p_pos_secs() as u64, u.get_p_dur_secs() as u64);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        if u.get_p_paused() {
            return Status { details: trf("Regarde {}", &[&title]), state: if sub.is_empty() { tr("En pause").into() } else { trf("{} · en pause", &[&sub]) }, ..Default::default() };
        }
        let start = now.saturating_sub(pos);
        let end = (dur > pos).then(|| now + (dur - pos));
        return Status { details: trf("Regarde {}", &[&title]), state: sub, start: Some(start), end };
    }
    let screen = u.get_screen().to_string();
    let (details, state) = match screen.as_str() {
        "home" => (tr("Sur l'accueil").to_string(), String::new()),
        "library" => (tr("Parcourt une bibliothèque").to_string(), u.get_lib_title().to_string()),
        "detail" => (tr("Regarde une fiche").to_string(), u.get_detail().title.to_string()),
        "search" => (tr("Cherche quelque chose à voir").to_string(), String::new()),
        "settings" => (tr("Dans les paramètres").to_string(), String::new()),
        "downloads" => (tr("Dans ses téléchargements").to_string(), String::new()),
        _ => (tr("Dans Turtlefin").to_string(), String::new()),
    };
    Status { details, state, ..Default::default() }
}

/// Fil de mise à jour : relit l'interface, envoie le statut quand il change.
fn run(ui: slint::Weak<AppWindow>) {
    let mut conn: Option<Pipe> = None;
    let mut last: Option<Status> = None;
    loop {
        std::thread::sleep(Duration::from_secs(5));
        if !ON.load(Ordering::SeqCst) {
            // Coupé : le statut est effacé (puis plus rien jusqu'à la réactivation).
            if let Some(mut c) = conn.take() {
                let _ = c.send(1, &activity(None));
            }
            last = None;
            continue;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = ui.upgrade_in_event_loop(move |u| {
            let _ = tx.send(read_status(&u));
        });
        let Ok(status) = rx.recv_timeout(Duration::from_secs(3)) else { continue };
        if last.as_ref() == Some(&status) && conn.is_some() {
            continue;
        }
        if conn.is_none() {
            conn = Pipe::connect(&app_id());
            if conn.is_none() {
                continue;
            }
        }
        let ok = conn.as_mut().is_some_and(|c| c.send(1, &activity(Some(&status))).is_ok());
        if ok {
            last = Some(status);
        } else {
            conn = None;
            last = None;
        }
    }
}

/// Message SET_ACTIVITY (None : statut effacé).
fn activity(s: Option<&Status>) -> Value {
    let act = s.map(|s| {
        let mut a = json!({
            "details": clip(&s.details),
            "assets": { "large_image": "turtlefin", "large_text": "Turtlefin" },
        });
        if !s.state.is_empty() {
            a["state"] = json!(clip(&s.state));
        }
        if let Some(t) = s.start {
            a["timestamps"] = json!({ "start": t });
            if let Some(e) = s.end {
                a["timestamps"]["end"] = json!(e);
            }
        }
        a
    });
    json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": std::process::id(), "activity": act },
        "nonce": format!("{}", SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)),
    })
}

/// Discord refuse les textes de plus de 128 caractères.
fn clip(t: &str) -> String {
    if t.chars().count() <= 120 {
        return t.to_string();
    }
    t.chars().take(119).collect::<String>() + "…"
}

/// Tube vers l'application Discord.
struct Pipe {
    f: std::fs::File,
}

impl Pipe {
    fn connect(id: &str) -> Option<Pipe> {
        for path in candidates() {
            let Ok(f) = open(&path) else { continue };
            let mut p = Pipe { f };
            if p.send(0, &json!({ "v": 1, "client_id": id })).is_ok() && p.recv().is_some() {
                return Some(p);
            }
        }
        None
    }

    fn send(&mut self, op: u32, v: &Value) -> std::io::Result<()> {
        let body = serde_json::to_vec(v)?;
        let mut msg = Vec::with_capacity(8 + body.len());
        msg.extend_from_slice(&op.to_le_bytes());
        msg.extend_from_slice(&(body.len() as u32).to_le_bytes());
        msg.extend_from_slice(&body);
        self.f.write_all(&msg)?;
        self.f.flush()?;
        // Réponse de Discord lue (sinon le tube se remplit).
        if op == 1 {
            let _ = self.recv();
        }
        Ok(())
    }

    fn recv(&mut self) -> Option<Value> {
        let mut head = [0u8; 8];
        self.f.read_exact(&mut head).ok()?;
        let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
        if len > 1 << 20 {
            return None;
        }
        let mut body = vec![0u8; len];
        self.f.read_exact(&mut body).ok()?;
        serde_json::from_slice(&body).ok()
    }
}

#[cfg(windows)]
fn candidates() -> Vec<String> {
    (0..10).map(|i| format!(r"\\.\pipe\discord-ipc-{i}")).collect()
}

#[cfg(windows)]
fn open(path: &str) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new().read(true).write(true).open(path)
}

#[cfg(unix)]
fn candidates() -> Vec<String> {
    let base = std::env::var("XDG_RUNTIME_DIR").ok().or_else(|| std::env::var("TMPDIR").ok()).unwrap_or_else(|| "/tmp".into());
    let mut v = Vec::new();
    for dir in ["", "app/com.discordapp.Discord/", "snap.discord/"] {
        for i in 0..10 {
            v.push(format!("{base}/{dir}discord-ipc-{i}"));
        }
    }
    v
}

#[cfg(unix)]
fn open(path: &str) -> std::io::Result<std::fs::File> {
    use std::os::unix::net::UnixStream;
    let s = UnixStream::connect(path)?;
    s.set_read_timeout(Some(Duration::from_secs(3)))?;
    // Socket vu comme un fichier (lecture, écriture) : même code que le tube de Windows.
    use std::os::fd::{FromRawFd, IntoRawFd};
    // SAFETY : descripteur du socket, qui nous appartient (rendu par into_raw_fd).
    Ok(unsafe { std::fs::File::from_raw_fd(s.into_raw_fd()) })
}
