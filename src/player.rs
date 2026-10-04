//! Lecture via mpv, lancé dans un processus séparé et piloté par IPC JSON.
//! Si mpv plante ou fuit, l'interface de Turtlefin n'est pas touchée.
//!
//! Turtlefin observe `time-pos`, `pause` et `duration` via l'IPC et fait le
//! rapport de lecture au serveur (début, progression toutes les 10 s, fin).

use std::{ffi::OsString, path::PathBuf, process::Stdio, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

use crate::api::{Client, Item};

pub struct PlayRequest {
    pub item: Item,
    pub start_secs: f64,
    /// Plein écran propre à mpv (ignoré en mode intégré : c'est la fenêtre de Turtlefin qui décide).
    pub fullscreen: bool,
    /// Identifiant natif de la fenêtre de Turtlefin (HWND sous Windows, XID sous X11).
    /// Si présent, mpv dessine DANS cette fenêtre au lieu d'ouvrir la sienne.
    pub wid: Option<i64>,
}

/// Interface de lecture (barre de contrôle, menus des pistes) dessinée par mpv dans la vidéo.
/// Rien de Slint ne peut s'afficher par-dessus la vidéo intégrée : elle vit donc dans ce script Lua.
const UI_SCRIPT: &str = include_str!("turtlefin_ui.lua");

/// Nom sous lequel mpv connaît le script (nom du fichier sans extension).
const UI_SCRIPT_NAME: &str = "turtlefin_ui";

/// Envoie une commande JSON à mpv.
async fn send<W: AsyncWriteExt + Unpin>(wr: &mut W, v: &Value) {
    let _ = wr.write_all(format!("{v}
").as_bytes()).await;
}

/// Écrit le script d'interface dans le dossier temporaire (mpv ne lit les scripts que depuis un fichier).
fn ui_script_path() -> Option<PathBuf> {
    let p = std::env::temp_dir().join(format!("{UI_SCRIPT_NAME}.lua"));
    std::fs::write(&p, UI_SCRIPT).ok()?;
    Some(p)
}

/// Cherche mpv : variable TURTLEFIN_MPV, puis à côté de l'exécutable, puis dans le PATH.
fn mpv_path() -> OsString {
    if let Ok(p) = std::env::var("TURTLEFIN_MPV") {
        return p.into();
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let name = if cfg!(windows) { "mpv.exe" } else { "mpv" };
            let p = dir.join(name);
            if p.exists() {
                return p.into_os_string();
            }
        }
    }
    "mpv".into()
}

#[cfg(unix)]
mod ipc {
    pub type Stream = tokio::net::UnixStream;

    pub fn path() -> String {
        let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
        format!("{dir}/turtlefin-mpv-{}.sock", std::process::id())
    }

    pub async fn connect(path: &str) -> std::io::Result<Stream> {
        tokio::net::UnixStream::connect(path).await
    }
}

#[cfg(windows)]
mod ipc {
    pub type Stream = tokio::net::windows::named_pipe::NamedPipeClient;

    pub fn path() -> String {
        format!(r"\\.\pipe\turtlefin-mpv-{}", std::process::id())
    }

    pub async fn connect(path: &str) -> std::io::Result<Stream> {
        tokio::net::windows::named_pipe::ClientOptions::new().open(path)
    }
}

/// Décodage matériel : variable TURTLEFIN_HWDEC pour forcer une valeur (ex. « auto-safe », « no »).
/// Sur Raspberry Pi (Linux ARM 64 bits), le décodage matériel V4L2 produit des images au format
/// Broadcom « SAND » que Vulkan ne sait pas importer dans mpv : résultat, un écran vide. On décode
/// donc en logiciel par défaut. Ailleurs (Windows, PC Linux) : « auto-safe ».
fn hwdec_mode() -> String {
    if let Ok(v) = std::env::var("TURTLEFIN_HWDEC") {
        return v;
    }
    if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "no".to_string()
    } else {
        "auto-safe".to_string()
    }
}

fn ticks(secs: f64) -> i64 {
    (secs.max(0.0) * 10_000_000.0) as i64
}

/// Lance mpv et bloque jusqu'à sa fermeture. Rapporte la lecture au serveur.
pub async fn play(
    client: &Client,
    req: PlayRequest,
    mut commands: tokio::sync::mpsc::UnboundedReceiver<String>,
) -> Result<()> {
    let item = &req.item;
    let source = item.media_sources.as_ref().and_then(|v| v.first());
    let ms_id = source.map(|m| m.id.clone()).unwrap_or_else(|| item.id.clone());
    let url = client.stream_url(&item.id, &ms_id);
    let ipc_path = ipc::path();
    let session = uuid::Uuid::new_v4().simple().to_string();

    let (title, subtitle) = item.titles();
    let media_title = if subtitle.is_empty() { title } else { format!("{title} - {subtitle}") };

    let mut cmd = Command::new(mpv_path());
    cmd.arg(format!("--input-ipc-server={ipc_path}"))
        .arg(format!("--force-media-title={media_title}"))
        .arg(format!("--hwdec={}", hwdec_mode()))
        .arg("--keep-open=no");
    // Notre interface remplace la barre de mpv (OSC).
    match ui_script_path() {
        Some(p) => {
            cmd.arg("--osc=no").arg(format!("--script={}", p.display()));
        }
        None => eprintln!("turtlefin : impossible d'écrire le script d'interface, barre de mpv par défaut"),
    }
    // Pi : rendu simplifié (mise à l'échelle bilinéaire...) pour ménager le GPU.
    if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        cmd.arg("--profile=fast");
    }
    if let Some(wid) = req.wid {
        cmd.arg(format!("--wid={wid}"));
    } else if req.fullscreen {
        cmd.arg("--fullscreen");
    }
    if req.start_secs > 1.0 {
        cmd.arg(format!("--start={:.1}", req.start_secs));
    }
    if let Some(src) = source {
        for st in src.media_streams.iter().filter(|s| s.kind == "Subtitle" && s.is_external) {
            if let Some(u) = client.subtitle_url(&item.id, &ms_id, st) {
                cmd.arg(format!("--sub-file={u}"));
            }
        }
    }
    // Linux : sortie audio via PipeWire/PulseAudio (le serveur de son du système), ALSA en dernier
    // recours. Écrase un « ao=alsa » + « audio-device=alsa/plughw:... » de ~/.config/mpv/mpv.conf, qui
    // échoue quand PipeWire occupe déjà la sortie HDMI. TURTLEFIN_AO=alsa (par ex.) pour choisir autre
    // chose, ou TURTLEFIN_AO= (vide) pour ne rien imposer et laisser la config de mpv décider.
    #[cfg(target_os = "linux")]
    match std::env::var("TURTLEFIN_AO") {
        Ok(v) if v.is_empty() => {}
        Ok(v) => {
            cmd.arg(format!("--ao={v}")).arg("--audio-device=auto");
        }
        Err(_) => {
            cmd.arg("--ao=pipewire,pulse,alsa").arg("--audio-device=auto");
        }
    }
    // Plafonne le cache réseau (par défaut mpv peut approcher 200 Mo de plus que nécessaire).
    cmd.arg("--demuxer-max-bytes=100MiB").arg("--demuxer-max-back-bytes=25MiB");
    // Diagnostic : TURTLEFIN_MPV_LOG=/chemin/mpv.log enregistre le journal détaillé de mpv.
    if let Ok(path) = std::env::var("TURTLEFIN_MPV_LOG") {
        cmd.arg(format!("--log-file={path}")).arg("--msg-level=all=v");
    }
    // Essais : TURTLEFIN_MPV_ARGS="--vo=x11 --hwdec=no" ajoute des options à mpv sans recompiler.
    if let Ok(extra) = std::env::var("TURTLEFIN_MPV_ARGS") {
        for a in extra.split_whitespace() {
            cmd.arg(a);
        }
    }
    cmd.arg("--").arg(&url);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| {
        anyhow!("Impossible de lancer mpv ({e}). Installe mpv ou indique son chemin avec TURTLEFIN_MPV.")
    })?;

    // mpv met un instant à créer son socket/pipe IPC.
    let mut stream: Option<ipc::Stream> = None;
    for _ in 0..80 {
        if let Ok(s) = ipc::connect(&ipc_path).await {
            stream = Some(s);
            break;
        }
        if let Ok(Some(_)) = child.try_wait() {
            return Err(anyhow!("mpv s'est fermé au démarrage"));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let Some(stream) = stream else {
        let _ = child.kill().await;
        return Err(anyhow!("mpv n'a pas ouvert son canal IPC"));
    };

    let (rd, mut wr) = tokio::io::split(stream);
    let mut lines = BufReader::new(rd).lines();
    for (id, name) in [(1, "time-pos"), (2, "pause"), (3, "duration")] {
        let msg = format!("{}\n", json!({ "command": ["observe_property", id, name] }));
        wr.write_all(msg.as_bytes()).await?;
    }

    let body = |pos_secs: f64, paused: bool| {
        json!({
            "ItemId": item.id,
            "MediaSourceId": ms_id,
            "PlaySessionId": session,
            "PositionTicks": ticks(pos_secs),
            "IsPaused": paused,
            "CanSeek": true,
            "PlayMethod": "DirectPlay",
        })
    };

    let _ = client.report("/Sessions/Playing", &body(req.start_secs, false)).await;

    let mut pos = req.start_secs;
    let mut paused = false;
    let mut duration = 0.0_f64;
    let mut reached_end = false;

    let mut tick = tokio::time::interval(Duration::from_secs(10));
    tick.tick().await; // le premier tick est immédiat : on le consomme
    let mut commands_open = true;

    loop {
        tokio::select! {
            line = lines.next_line() => {
                let Ok(Some(line)) = line else { break }; // mpv a fermé le canal
                let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };

                match v.get("event").and_then(Value::as_str) {
                    Some("property-change") => match v.get("name").and_then(Value::as_str) {
                        Some("time-pos") => {
                            if let Some(t) = v.get("data").and_then(Value::as_f64) {
                                pos = t;
                            }
                        }
                        Some("duration") => {
                            if let Some(d) = v.get("data").and_then(Value::as_f64) {
                                duration = d;
                            }
                        }
                        Some("pause") => {
                            if let Some(b) = v.get("data").and_then(Value::as_bool) {
                                if b != paused {
                                    paused = b;
                                    let _ = client.report("/Sessions/Playing/Progress", &body(pos, paused)).await;
                                }
                            }
                        }
                        _ => {}
                    },
                    Some("end-file") => {
                        if v.get("reason").and_then(Value::as_str) == Some("eof") {
                            reached_end = true;
                        }
                    }
                    _ => {}
                }
            }
            _ = tick.tick() => {
                let _ = client.report("/Sessions/Playing/Progress", &body(pos, paused)).await;
            }
            cmd = commands.recv(), if commands_open => {
                let Some(c) = cmd else {
                    commands_open = false;
                    continue;
                };

                // Les touches vont à l'interface dessinée par mpv, qui décide selon son état
                // (barre affichée ou non, menu des pistes ouvert...). « q » quitte toujours.
                let msg = if c == "quit" {
                    json!({ "command": ["quit"] })
                } else {
                    json!({ "command": ["script-message-to", UI_SCRIPT_NAME, "key", c] })
                };
                send(&mut wr, &msg).await;
            }
        }
    }

    let status = child.wait().await;

    // Fin de fichier atteinte : on rapporte la durée complète pour que le serveur marque « vu ».
    if reached_end && duration > 0.0 {
        pos = duration;
    }
    let _ = client.report("/Sessions/Playing/Stopped", &body(pos, false)).await;

    // Un code de sortie non nul = mpv a échoué (sortie vidéo/audio impossible, flux illisible...).
    if let Ok(st) = status {
        if !st.success() {
            return Err(anyhow!(
                "mpv s'est terminé avec le code {:?} (relance avec TURTLEFIN_MPV_LOG=/tmp/mpv.log pour le détail)",
                st.code()
            ));
        }
    }
    Ok(())
}
