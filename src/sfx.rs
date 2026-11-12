//! Effets sonores de l'interface (Paramètres → Affichage → « Effets sonores », coupés par défaut).
//!
//! Chaque thème a son jeu de sons : gouttes et sonar pour Turtlefin (« abyss »), verre qui tinte pour
//! Aero (« aero »), bips de vieille machine pour Nerd (« nerd »). Ordre de recherche d'un son : fichier
//! du thème personnel (`custom`), fichier livré avec l'appli (`sounds/<jeu>/<événement>.ogg`, intégré au
//! programme par build.rs), sinon synthèse ici même.
//!
//! `TURTLEFIN_DEBUG_SFX=1` : journal du lecteur des sons (`sfx-mpv.log` dans le cache) et une ligne par
//! son joué sur la sortie d'erreur.
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
static VOLUME: AtomicU32 = AtomicU32::new(70);

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/sounds.rs"));
}
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

/// Touche maintenue : un son toutes les 220 ms au plus (pas une rafale à chaque répétition du clavier).
pub fn play_repeat(event: &str) {
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    {
        let mut last = LAST.lock().unwrap();
        if last.is_some_and(|t| t.elapsed() < Duration::from_millis(220)) {
            return;
        }
        *last = Some(Instant::now());
    }
    play(event);
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
    loop {
        // Le lecteur range ses messages (journal, fin de son) dans une file : on la vide pendant
        // l'attente du son suivant, sinon elle se remplit pour rien (et le journal reste vide).
        let mut event = match rx.recv_timeout(std::time::Duration::from_millis(250)) {
            Ok(e) => e,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(m) = &player {
                    // `None` vaut aussi pour un message du journal : on lit un nombre borné d'événements
                    // (sans attente, une file vide rend aussitôt la main).
                    for _ in 0..256 {
                        if let Some(crate::mpv::Event::EndFile { error: Some(e), .. }) = m.wait_event(0.0) {
                            eprintln!("turtlefin : effet sonore : {e}");
                        }
                    }
                }
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
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
        // Le volume de mpv suit une courbe cubique (50 % y vaut un huitième de l'amplitude, presque
        // rien) : le réglage des effets sonores est ramené à une échelle linéaire (50 % = moitié).
        let v = VOLUME.load(Ordering::Relaxed) as f64 / 100.0;
        let _ = m.set_property("volume", &format!("{:.1}", 100.0 * v.cbrt()));
        if std::env::var_os("TURTLEFIN_DEBUG_SFX").is_some() {
            eprintln!("turtlefin : son « {event} » : {file}");
        }
        if let Err(e) = m.command(&["loadfile", &file, "replace"]) {
            eprintln!("turtlefin : son « {event} » : {e}");
        }
    }
}

fn new_player() -> Option<crate::mpv::Mpv> {
    let m = crate::mpv::Mpv::new().ok()?;
    if std::env::var_os("TURTLEFIN_DEBUG_SFX").is_some() {
        if let Some(d) = crate::paths::cache_dir() {
            let _ = std::fs::create_dir_all(&d);
            m.log_to(&d.join("sfx-mpv.log").display().to_string());
        }
    }
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
    // Fichier livré avec l'appli pour ce jeu (sinon celui du jeu Turtlefin), écrit une fois dans le cache.
    let found = embedded::EMBEDDED
        .iter()
        .find(|(s, e, _, _)| *s == set && *e == event)
        .or_else(|| embedded::EMBEDDED.iter().find(|(s, e, _, _)| *s == "abyss" && *e == event && set != "aero" && set != "nerd"));
    if let Some((s, e, ext, bytes)) = found {
        let path = dir.join(format!("{s}-{e}-{}.{ext}", bytes.len()));
        if !path.is_file() {
            std::fs::create_dir_all(&dir).ok()?;
            std::fs::write(&path, bytes).ok()?;
        }
        return Some(path.to_string_lossy().into_owned());
    }
    // « v3 » : à changer quand la synthèse change (les anciens fichiers sont alors refaits).
    let path = dir.join(format!("{set}-{event}-v3.wav"));
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

/// Une note : départ (s), durée (s), fréquence de début et de fin (glissement), timbre, volume.
#[derive(Clone, Copy)]
struct Note(f32, f32, f32, f32, Timbre, f32);

#[derive(Clone, Copy)]
enum Timbre {
    /// Goutte : sinus dont la hauteur monte vite au début (le « plop » d'une goutte d'eau).
    Drop,
    /// Verre frappé : les harmoniques d'un verre (1 ; 2,32 ; 4,25 ; 6,63 fois la fondamentale, chacune doublée
    /// d'une jumelle à peine désaccordée qui la fait frémir), les aiguës s'éteignant plus vite ; pas de
    /// glissement de hauteur. Une seule frappe par son (deux notes à la suite sonnaient « pièce de jeu vidéo »).
    Glass,
    /// Bip de vieille machine : onde carrée adoucie.
    Bleep,
    /// Sinus pur, doux.
    Soft,
    /// Bruit bref (déclic, relais).
    Tick,
}

fn tone(n: &Note, seed: &mut u32) -> Vec<f32> {
    let Note(_, dur, f0, f1, timbre, gain) = *n;
    let len = (dur * RATE as f32) as usize;
    let mut out = Vec::with_capacity(len);
    let (mut ph, mut ph_m) = (0.0f32, 0.0f32);
    let tau = std::f32::consts::TAU;
    for i in 0..len {
        let t = i as f32 / RATE as f32;
        let q = i as f32 / len.max(1) as f32;
        let f = match timbre {
            // La goutte monte d'abord vite (20 ms), puis tient sa note.
            Timbre::Drop => f1 + (f0 - f1) * (-t / 0.012).exp(),
            _ => f0 + (f1 - f0) * q,
        };
        ph += f / RATE as f32;
        let v = match timbre {
            Timbre::Drop | Timbre::Soft => (ph * tau).sin(),
            Timbre::Glass => {
                let _ = &mut ph_m;
                const MODES: [(f32, f32, f32); 4] = [(1.0, 1.0, 1.0), (2.32, 0.55, 0.55), (4.25, 0.32, 0.3), (6.63, 0.18, 0.16)];
                let mut v = 0.0;
                for (r, a, d) in MODES {
                    let tau_k = dur * 0.45 * d;
                    let e = (-t / tau_k).exp();
                    v += a * e * ((f0 * r * t * tau).sin() * 0.6 + (f0 * r * 1.0035 * t * tau).sin() * 0.4);
                }
                v * 0.55
            }
            Timbre::Bleep => ((ph * tau).sin() * 2.5).tanh() * 0.7,
            Timbre::Tick => {
                *seed ^= *seed << 13;
                *seed ^= *seed >> 17;
                *seed ^= *seed << 5;
                (*seed as f32 / u32::MAX as f32) * 2.0 - 1.0
            }
        };
        // Enveloppe : attaque courte (pas de clic), extinction exponentielle, fin à zéro.
        let attack = match timbre {
            Timbre::Soft => (t / 0.012).min(1.0),
            Timbre::Tick => (t / 0.0005).min(1.0),
            _ => (t / 0.002).min(1.0),
        };
        let decay = match timbre {
            Timbre::Tick => (-t / 0.004).exp(),
            Timbre::Bleep => (-t / (dur * 0.6)).exp(),
            // Chaque harmonique a déjà son extinction (voir plus haut).
            Timbre::Glass => 1.0,
            _ => (-t / (dur * 0.3)).exp(),
        };
        let tail = ((1.0 - q) / 0.08).min(1.0);
        out.push(v * attack * decay * tail * gain);
    }
    out
}

/// Petite réverbération (Schroeder : quatre peignes, deux passe-tout) : un peu d'espace autour du son.
fn reverb(dry: &[f32], wet: f32, size: f32) -> Vec<f32> {
    let extra = (0.25 * size * RATE as f32) as usize;
    let mut out: Vec<f32> = dry.iter().copied().chain(std::iter::repeat(0.0).take(extra)).collect();
    if wet <= 0.0 {
        return out;
    }
    let combs = [(1557, 0.80), (1617, 0.79), (1491, 0.81), (1422, 0.78)];
    let mut acc = vec![0.0f32; out.len()];
    for (d, g) in combs {
        let d = ((d as f32) * size) as usize;
        let mut buf = vec![0.0f32; out.len()];
        for i in 0..out.len() {
            let fb = if i >= d { buf[i - d] } else { 0.0 };
            buf[i] = out[i] + fb * g;
            acc[i] += buf[i] * 0.25;
        }
    }
    for (d, g) in [(225usize, 0.5f32), (556, 0.5)] {
        let mut y = vec![0.0f32; acc.len()];
        for i in 0..acc.len() {
            let xd = if i >= d { acc[i - d] } else { 0.0 };
            let yd = if i >= d { y[i - d] } else { 0.0 };
            y[i] = -g * acc[i] + xd + g * yd;
        }
        acc = y;
    }
    for i in 0..out.len() {
        out[i] = out[i] * (1.0 - wet * 0.5) + acc[i] * wet;
    }
    out
}

/// Mélange, filtre adoucissant (passe-bas à un pôle), réverbération, puis normalisation : chaque son a la
/// même force (crête à 0,8), réglée ensuite par le volume des effets sonores.
fn render(notes: &[Note], cutoff: f32, wet: f32, room: f32, level: f32) -> Vec<f32> {
    let total = notes.iter().map(|n| n.0 + n.1).fold(0.0f32, f32::max);
    let mut mix = vec![0.0f32; (total * RATE as f32) as usize + 1];
    let mut seed = 0x2545_f491u32;
    for n in notes {
        let at = (n.0 * RATE as f32) as usize;
        for (i, v) in tone(n, &mut seed).into_iter().enumerate() {
            if let Some(o) = mix.get_mut(at + i) {
                *o += v;
            }
        }
    }
    let a = 1.0 - (-std::f32::consts::TAU * cutoff / RATE as f32).exp();
    let mut y = 0.0f32;
    for v in mix.iter_mut() {
        y += a * (*v - y);
        *v = y;
    }
    let mut out = reverb(&mix, wet, room);
    let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
    let k = 0.8 * level / peak;
    // Fondu de 8 ms à la fin (la réverbération s'arrête sans claquer).
    let fade = (0.008 * RATE as f32) as usize;
    let n = out.len();
    for (i, v) in out.iter_mut().enumerate() {
        *v *= k * ((n - i) as f32 / fade as f32).min(1.0);
    }
    out
}

/// Synthèse du son d'un événement pour un jeu ; `None` : événement inconnu.
pub fn synth(set: &str, event: &str) -> Option<Vec<f32>> {
    use Timbre::*;
    // (notes, coupure du filtre en Hz, part de réverbération, taille de la pièce, niveau relatif)
    let (notes, cutoff, wet, room, level): (Vec<Note>, f32, f32, f32, f32) = match (set, event) {
        // Aero : verre frappé (une frappe, un petit « tic » au contact), de l'espace autour.
        ("aero", "move") => (vec![Note(0.0, 0.005, 0.0, 0.0, Tick, 0.25), Note(0.0, 0.35, 1480.0, 1480.0, Glass, 1.0)], 7500.0, 0.3, 0.9, 0.4),
        ("aero", "ok") => (vec![Note(0.0, 0.006, 0.0, 0.0, Tick, 0.3), Note(0.0, 0.8, 1047.0, 1047.0, Glass, 1.0), Note(0.0, 0.8, 523.0, 523.0, Glass, 0.35)], 7500.0, 0.35, 1.0, 0.75),
        ("aero", "back") => (vec![Note(0.0, 0.006, 0.0, 0.0, Tick, 0.3), Note(0.0, 0.7, 784.0, 784.0, Glass, 1.0)], 7000.0, 0.35, 1.0, 0.7),
        ("aero", "tap") => (vec![Note(0.0, 0.004, 0.0, 0.0, Tick, 0.25), Note(0.0, 0.25, 1760.0, 1760.0, Glass, 1.0)], 7500.0, 0.25, 0.8, 0.35),
        ("aero", "type") => (vec![Note(0.0, 0.003, 0.0, 0.0, Tick, 0.2), Note(0.0, 0.12, 2217.0, 2217.0, Glass, 1.0)], 7500.0, 0.15, 0.6, 0.22),
        ("aero", "toggle") => (vec![Note(0.0, 0.005, 0.0, 0.0, Tick, 0.3), Note(0.0, 0.45, 1319.0, 1319.0, Glass, 1.0)], 7500.0, 0.3, 0.9, 0.55),
        ("aero", "error") => (vec![Note(0.0, 0.006, 0.0, 0.0, Tick, 0.3), Note(0.0, 0.9, 466.0, 466.0, Glass, 1.0), Note(0.0, 0.9, 494.0, 494.0, Glass, 0.8)], 5500.0, 0.35, 1.0, 0.8),
        // Démarrage : un accord de verre qui sonne longtemps (frappé une fois, pas d'arpège).
        ("aero", "start") => (
            vec![
                Note(0.0, 0.008, 0.0, 0.0, Tick, 0.3),
                Note(0.0, 2.2, 523.0, 523.0, Glass, 0.7),
                Note(0.0, 2.2, 784.0, 784.0, Glass, 0.6),
                Note(0.0, 2.2, 1047.0, 1047.0, Glass, 0.5),
            ],
            7500.0,
            0.45,
            1.3,
            0.85,
        ),
        // Nerd : bips d'une vieille machine, déclics de relais, sans écho.
        ("nerd", "move") => (vec![Note(0.0, 0.03, 1320.0, 1320.0, Bleep, 1.0)], 3500.0, 0.0, 0.5, 0.35),
        ("nerd", "ok") => (vec![Note(0.0, 0.05, 880.0, 880.0, Bleep, 1.0), Note(0.06, 0.08, 1320.0, 1320.0, Bleep, 1.0)], 3500.0, 0.05, 0.5, 0.6),
        ("nerd", "back") => (vec![Note(0.0, 0.05, 1320.0, 1320.0, Bleep, 1.0), Note(0.06, 0.08, 880.0, 880.0, Bleep, 1.0)], 3500.0, 0.05, 0.5, 0.6),
        ("nerd", "tap") => (vec![Note(0.0, 0.02, 0.0, 0.0, Tick, 1.0), Note(0.0, 0.025, 1800.0, 1200.0, Bleep, 0.3)], 5000.0, 0.0, 0.5, 0.4),
        ("nerd", "type") => (vec![Note(0.0, 0.015, 0.0, 0.0, Tick, 1.0)], 4500.0, 0.0, 0.5, 0.25),
        ("nerd", "toggle") => (vec![Note(0.0, 0.02, 0.0, 0.0, Tick, 1.0), Note(0.03, 0.05, 990.0, 990.0, Bleep, 0.8)], 4000.0, 0.0, 0.5, 0.5),
        ("nerd", "error") => (vec![Note(0.0, 0.12, 196.0, 196.0, Bleep, 1.0), Note(0.16, 0.18, 185.0, 185.0, Bleep, 1.0)], 2500.0, 0.05, 0.5, 0.85),
        ("nerd", "start") => (
            vec![
                Note(0.0, 0.08, 523.0, 523.0, Bleep, 1.0),
                Note(0.09, 0.08, 659.0, 659.0, Bleep, 1.0),
                Note(0.18, 0.08, 784.0, 784.0, Bleep, 1.0),
                Note(0.27, 0.3, 1047.0, 1047.0, Bleep, 1.0),
            ],
            3500.0,
            0.08,
            0.5,
            0.8,
        ),
        // Turtlefin : gouttes d'eau et sonar, dans une pièce feutrée.
        (_, "move") => (vec![Note(0.0, 0.07, 500.0, 1100.0, Drop, 1.0)], 4000.0, 0.15, 0.7, 0.4),
        (_, "ok") => (vec![Note(0.0, 0.1, 450.0, 900.0, Drop, 1.0), Note(0.07, 0.14, 650.0, 1350.0, Drop, 0.9)], 4500.0, 0.22, 0.9, 0.75),
        (_, "back") => (vec![Note(0.0, 0.1, 650.0, 1350.0, Drop, 0.9), Note(0.07, 0.14, 450.0, 800.0, Drop, 1.0)], 4500.0, 0.22, 0.9, 0.7),
        (_, "tap") => (vec![Note(0.0, 0.05, 600.0, 1300.0, Drop, 1.0)], 4000.0, 0.12, 0.6, 0.35),
        (_, "type") => (vec![Note(0.0, 0.03, 900.0, 1600.0, Drop, 1.0)], 4000.0, 0.05, 0.5, 0.2),
        (_, "toggle") => (vec![Note(0.0, 0.06, 500.0, 1000.0, Drop, 1.0), Note(0.05, 0.08, 700.0, 1500.0, Drop, 0.8)], 4500.0, 0.18, 0.8, 0.55),
        (_, "error") => (vec![Note(0.0, 0.35, 220.0, 196.0, Soft, 1.0), Note(0.0, 0.35, 262.0, 233.0, Soft, 0.6)], 1800.0, 0.25, 1.0, 0.85),
        (_, "start") => (
            vec![
                Note(0.0, 1.0, 523.0, 523.0, Soft, 0.7),
                Note(0.15, 1.0, 659.0, 659.0, Soft, 0.6),
                Note(0.3, 1.4, 784.0, 784.0, Soft, 0.7),
                Note(0.3, 1.4, 1568.0, 1568.0, Soft, 0.15),
            ],
            5000.0,
            0.45,
            1.3,
            0.85,
        ),
        _ => return None,
    };
    Some(render(&notes, cutoff, wet, room, level))
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
                assert!(s.iter().all(|v| v.abs() <= 0.81), "{set} / {ev} sature");
                assert!(s.iter().any(|v| v.abs() > 0.1), "{set} / {ev} trop faible");
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
