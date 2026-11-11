//! Effets sonores de l'interface (Paramètres → Affichage → « Effets sonores », coupés par défaut).
//!
//! Chaque thème a son jeu de sons, fabriqués ici par synthèse (aucun fichier dans le dépôt) : gouttes
//! et sonar pour Turtlefin (« abyss »), verre qui tinte pour Aero (« aero »), bips carrés de vieille
//! machine pour Nerd (« nerd »). Un thème personnel peut remplacer un son par un fichier (`custom`).
//!
//! Lecture : un lecteur mpv à part (audio seul, sans vidéo), créé au premier son et gardé ; les sons
//! sont écrits une fois en WAV dans le cache (`sfx/`). Rien n'est créé tant que les sons sont coupés.
//!
//! Événements : `move` (flèches), `ok` (Entrée), `back` (Échap / Retour), `tap` (clic, toucher),
//! `type` (lettre tapée), `toggle` (réglage basculé), `error`, `start` (arrivée sur l'accueil).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::{Duration, Instant};

static ON: AtomicBool = AtomicBool::new(false);
static VOLUME: AtomicU32 = AtomicU32::new(50);
static SET: Mutex<String> = Mutex::new(String::new());
/// Sons remplacés par le thème en cours : événement -> fichier.
static CUSTOM: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
static QUEUE: OnceLock<Mutex<mpsc::Sender<String>>> = OnceLock::new();
static LAST_MOVE: Mutex<Option<Instant>> = Mutex::new(None);

#[allow(dead_code)]
pub const EVENTS: [&str; 8] = ["move", "ok", "back", "tap", "type", "toggle", "error", "start"];
const RATE: u32 = 44_100;

/// Réglages de l'appareil : sons activés, volume (0 à 100).
pub fn configure(on: bool, volume: u32) {
    ON.store(on, Ordering::Relaxed);
    VOLUME.store(volume.min(100), Ordering::Relaxed);
}

/// Jeu de sons du thème (« abyss », « aero », « nerd ») et fichiers qui en remplacent certains.
pub fn set_theme(set: &str, custom: HashMap<String, String>) {
    *SET.lock().unwrap() = set.to_string();
    *CUSTOM.lock().unwrap() = if custom.is_empty() { None } else { Some(custom) };
}

/// Joue le son d'un événement (sans attendre ; ignoré si les sons sont coupés).
pub fn play(event: &str) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    // Touche maintenue : pas plus d'un son de déplacement toutes les 50 ms.
    if event == "move" || event == "type" {
        let mut last = LAST_MOVE.lock().unwrap();
        if last.is_some_and(|t| t.elapsed() < Duration::from_millis(50)) {
            return;
        }
        *last = Some(Instant::now());
    }
    let tx = QUEUE.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<String>();
        std::thread::spawn(move || worker(rx));
        Mutex::new(tx)
    });
    let _ = tx.lock().unwrap().send(event.to_string());
}

/// Fil des sons : possède le lecteur mpv, joue les événements dans l'ordre (un son chasse le précédent).
fn worker(rx: mpsc::Receiver<String>) {
    let mut player: Option<crate::mpv::Mpv> = None;
    while let Ok(mut event) = rx.recv() {
        // Rafale (touche maintenue) : seul le dernier son demandé est joué.
        while let Ok(next) = rx.try_recv() {
            event = next;
        }
        if !ON.load(Ordering::Relaxed) {
            continue;
        }
        let Some(file) = file_for(&event) else { continue };
        if player.is_none() {
            player = new_player();
        }
        let Some(m) = &player else { continue };
        let _ = m.set_property("volume", &VOLUME.load(Ordering::Relaxed).to_string());
        if let Err(e) = m.command(&["loadfile", &file, "replace"]) {
            eprintln!("turtlefin : son « {event} » : {e}");
        }
    }
}

fn new_player() -> Option<crate::mpv::Mpv> {
    let m = crate::mpv::Mpv::new().ok()?;
    for (k, v) in [
        ("config", "no"),
        ("terminal", "no"),
        ("vid", "no"),
        ("vo", "null"),
        ("idle", "yes"),
        ("input-default-bindings", "no"),
        ("osc", "no"),
        ("ytdl", "no"),
        ("load-scripts", "no"),
        ("load-stats-overlay", "no"),
        ("load-console", "no"),
        ("load-auto-profiles", "no"),
        ("cache", "no"),
        ("audio-buffer", "0.05"),
        ("audio-client-name", "Turtlefin"),
        // Fichiers locaux seulement (WAV du cache, ou son d'un thème personnel).
        ("network-timeout", "1"),
    ] {
        m.set_option(k, v);
    }
    #[cfg(target_os = "linux")]
    match std::env::var("TURTLEFIN_AO") {
        Ok(v) if v.is_empty() => {}
        Ok(v) => m.set_option("ao", &v),
        Err(_) => m.set_option("ao", "pipewire,pulse,alsa"),
    }
    #[cfg(target_os = "android")]
    m.set_option("ao", "audiotrack,opensles");
    match m.initialize() {
        Ok(()) => Some(m),
        Err(e) => {
            eprintln!("turtlefin : lecteur des effets sonores : {e}");
            None
        }
    }
}

/// Fichier du son : celui du thème personnel s'il existe, sinon le WAV synthétisé (écrit au besoin).
fn file_for(event: &str) -> Option<String> {
    if let Some(f) = CUSTOM.lock().unwrap().as_ref().and_then(|c| c.get(event)).filter(|f| std::path::Path::new(f).is_file()) {
        return Some(f.clone());
    }
    let set = {
        let s = SET.lock().unwrap();
        if s.is_empty() { "abyss".to_string() } else { s.clone() }
    };
    let dir = crate::paths::cache_dir()?.join("sfx");
    // « v1 » : à changer quand la synthèse change (les anciens fichiers sont alors refaits).
    let path = dir.join(format!("{set}-{event}-v1.wav"));
    if !path.is_file() {
        let samples = synth(&set, event)?;
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::write(&path, wav(&samples)).ok()?;
    }
    Some(path.to_string_lossy().into_owned())
}

/// WAV mono 16 bits à partir d'échantillons entre -1 et 1.
pub fn wav(samples: &[f32]) -> Vec<u8> {
    let data = samples.len() as u32 * 2;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    out
}

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    /// Carré adouci (les angles vifs d'un vrai carré sifflent).
    Square,
    /// Cloche de verre : fondamentale et deux partiels inharmoniques qui s'éteignent plus vite.
    Bell,
    Noise,
}

/// Une note : départ (s), durée (s), fréquence de début et de fin (glissando), forme, volume.
struct Note(f32, f32, f32, f32, Wave, f32);

/// Mélange des notes, avec une attaque de 3 ms et une extinction exponentielle (pas de clic).
fn render(notes: &[Note]) -> Vec<f32> {
    let total = notes.iter().map(|n| n.0 + n.1).fold(0.0f32, f32::max) + 0.02;
    let mut out = vec![0.0f32; (total * RATE as f32) as usize];
    let mut seed = 0x2545_f491u32;
    for Note(start, dur, f0, f1, wave, gain) in notes {
        let n = (dur * RATE as f32) as usize;
        let at = (start * RATE as f32) as usize;
        let mut phase = 0.0f32;
        for i in 0..n {
            let t = i as f32 / n as f32;
            let f = f0 + (f1 - f0) * t;
            phase += f / RATE as f32;
            let p = phase * std::f32::consts::TAU;
            let secs = i as f32 / RATE as f32;
            let s = match wave {
                Wave::Sine => p.sin(),
                Wave::Square => (p.sin() * 3.0).tanh() * 0.6,
                Wave::Bell => {
                    p.sin() * 0.7 + (p * 2.76).sin() * 0.25 * (-secs * 30.0).exp() + (p * 5.4).sin() * 0.12 * (-secs * 55.0).exp()
                }
                Wave::Noise => {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
                }
            };
            let attack = (secs / 0.003).min(1.0);
            let env = attack * (-4.5 * t).exp() * (1.0 - t).min(1.0).powf(0.5);
            if let Some(o) = out.get_mut(at + i) {
                *o += s * env * gain;
            }
        }
    }
    out
}

/// Synthèse du son d'un événement pour un jeu ; `None` : événement inconnu.
pub fn synth(set: &str, event: &str) -> Option<Vec<f32>> {
    use Wave::*;
    let notes: Vec<Note> = match (set, event) {
        // Aero : verre qui tinte.
        ("aero", "move") => vec![Note(0.0, 0.07, 1480.0, 1480.0, Bell, 0.22)],
        ("aero", "ok") => vec![Note(0.0, 0.26, 1046.0, 1046.0, Bell, 0.3), Note(0.05, 0.3, 1568.0, 1568.0, Bell, 0.2)],
        ("aero", "back") => vec![Note(0.0, 0.2, 988.0, 988.0, Bell, 0.26), Note(0.05, 0.24, 740.0, 740.0, Bell, 0.22)],
        ("aero", "tap") => vec![Note(0.0, 0.05, 1760.0, 1760.0, Bell, 0.2)],
        ("aero", "type") => vec![Note(0.0, 0.035, 2090.0, 2090.0, Bell, 0.13)],
        ("aero", "toggle") => vec![Note(0.0, 0.09, 1175.0, 1175.0, Bell, 0.24), Note(0.06, 0.14, 1760.0, 1760.0, Bell, 0.2)],
        ("aero", "error") => vec![Note(0.0, 0.3, 415.0, 415.0, Bell, 0.3), Note(0.0, 0.3, 587.0, 587.0, Bell, 0.22)],
        ("aero", "start") => vec![
            Note(0.0, 0.5, 659.0, 659.0, Bell, 0.24),
            Note(0.16, 0.5, 988.0, 988.0, Bell, 0.22),
            Note(0.32, 0.55, 880.0, 880.0, Bell, 0.2),
            Note(0.5, 0.9, 1319.0, 1319.0, Bell, 0.24),
        ],
        // Nerd : bips carrés et relais.
        ("nerd", "move") => vec![Note(0.0, 0.028, 1200.0, 1200.0, Square, 0.14)],
        ("nerd", "ok") => vec![Note(0.0, 0.045, 880.0, 880.0, Square, 0.16), Note(0.05, 0.07, 1320.0, 1320.0, Square, 0.16)],
        ("nerd", "back") => vec![Note(0.0, 0.045, 660.0, 660.0, Square, 0.16), Note(0.05, 0.07, 440.0, 440.0, Square, 0.16)],
        ("nerd", "tap") => vec![Note(0.0, 0.014, 0.0, 0.0, Noise, 0.22), Note(0.0, 0.02, 1500.0, 900.0, Square, 0.08)],
        ("nerd", "type") => vec![Note(0.0, 0.012, 0.0, 0.0, Noise, 0.16)],
        ("nerd", "toggle") => vec![Note(0.0, 0.03, 990.0, 990.0, Square, 0.15), Note(0.035, 0.015, 0.0, 0.0, Noise, 0.16)],
        ("nerd", "error") => vec![Note(0.0, 0.11, 185.0, 185.0, Square, 0.2), Note(0.14, 0.16, 185.0, 185.0, Square, 0.2)],
        ("nerd", "start") => vec![
            Note(0.0, 0.07, 523.0, 523.0, Square, 0.15),
            Note(0.08, 0.07, 659.0, 659.0, Square, 0.15),
            Note(0.16, 0.07, 784.0, 784.0, Square, 0.15),
            Note(0.24, 0.2, 1047.0, 1047.0, Square, 0.16),
        ],
        // Turtlefin (et thèmes sans jeu propre) : gouttes d'eau, sonar doux.
        (_, "move") => vec![Note(0.0, 0.05, 620.0, 880.0, Sine, 0.26)],
        (_, "ok") => vec![Note(0.0, 0.11, 520.0, 780.0, Sine, 0.3), Note(0.04, 0.16, 1040.0, 1040.0, Sine, 0.12)],
        (_, "back") => vec![Note(0.0, 0.13, 560.0, 330.0, Sine, 0.3)],
        (_, "tap") => vec![Note(0.0, 0.035, 760.0, 980.0, Sine, 0.22)],
        (_, "type") => vec![Note(0.0, 0.022, 1100.0, 1250.0, Sine, 0.13)],
        (_, "toggle") => vec![Note(0.0, 0.07, 700.0, 1050.0, Sine, 0.26)],
        (_, "error") => vec![Note(0.0, 0.24, 220.0, 196.0, Sine, 0.34), Note(0.0, 0.24, 233.0, 207.0, Sine, 0.24)],
        (_, "start") => vec![
            Note(0.0, 0.5, 392.0, 392.0, Sine, 0.22),
            Note(0.14, 0.5, 523.0, 523.0, Sine, 0.22),
            Note(0.28, 0.9, 659.0, 659.0, Sine, 0.24),
            Note(0.28, 0.9, 1318.0, 1318.0, Sine, 0.07),
        ],
        _ => return None,
    };
    Some(render(&notes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_set_has_every_event() {
        for set in ["abyss", "aero", "nerd"] {
            for ev in EVENTS {
                let s = synth(set, ev).unwrap_or_else(|| panic!("{set} / {ev}"));
                assert!(s.len() > 400, "{set} / {ev} trop court");
                assert!(s.iter().all(|v| v.abs() <= 1.0), "{set} / {ev} sature");
                // Pas de clic : le son commence et finit près de zéro.
                assert!(s[0].abs() < 0.05 && s[s.len() - 1].abs() < 0.05, "{set} / {ev} claque");
            }
        }
    }

    #[test]
    fn wav_header() {
        let w = wav(&[0.0, 0.5, -0.5]);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(w.len(), 44 + 6);
    }
}
