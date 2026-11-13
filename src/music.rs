//! Musique de fond d'un thème (Paramètres → Affichage → « Musique de fond du thème »).
//!
//! Un thème peut désigner un fichier audio (`music`), joué en boucle tant qu'aucun média n'est ouvert
//! dans le lecteur. Dès qu'une lecture commence (`video::attach`), la musique s'arrête, et elle ne
//! reprend qu'à la sortie du lecteur : un média en pause la garde coupée. Elle reprend là où elle
//! s'était arrêtée.
//!
//! Lecture : un lecteur mpv à part (audio seul), créé à la première musique et gardé. Rien n'est créé
//! tant qu'aucun thème n'a de musique ou que le réglage est coupé.
//!
//! `TURTLEFIN_DEBUG_MUSIC=1` : une ligne à chaque départ et arrêt de la musique.
//!
//! `sound(fichier)` : un son joué une fois (fonction `sound` offerte aux mods), par un second lecteur.

use std::sync::{mpsc, Mutex, OnceLock};

#[derive(Clone, Default, PartialEq)]
struct Want {
    /// Réglage de l'appareil.
    on: bool,
    /// Volume, 0 à 100.
    volume: u32,
    /// Fichier du thème en cours (vide : pas de musique).
    file: String,
    /// Un média est ouvert dans le lecteur (en lecture ou en pause).
    blocked: bool,
}

enum Msg {
    Update,
    Sound(String),
}

static WANT: Mutex<Option<Want>> = Mutex::new(None);
static QUEUE: OnceLock<Mutex<mpsc::Sender<Msg>>> = OnceLock::new();

fn change(f: impl FnOnce(&mut Want)) {
    let (changed, needed) = {
        let mut g = WANT.lock().unwrap();
        let w = g.get_or_insert_with(|| Want { volume: 40, ..Default::default() });
        let before = w.clone();
        f(w);
        (*w != before, w.on && !w.file.is_empty())
    };
    // Le fil de la musique n'est lancé que s'il y a quelque chose à jouer.
    if changed && (needed || QUEUE.get().is_some()) {
        send(Msg::Update);
    }
}

fn send(m: Msg) {
    let tx = QUEUE.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Msg>();
        std::thread::spawn(move || worker(rx));
        Mutex::new(tx)
    });
    let _ = tx.lock().unwrap().send(m);
}

/// Réglages de l'appareil : musique activée, volume (0 à 100).
pub fn configure(on: bool, volume: u32) {
    change(|w| {
        w.on = on;
        w.volume = volume.min(100);
    });
}

/// Musique du thème en cours (chemin complet du fichier ; `None` : ce thème n'en a pas).
pub fn set_theme(file: Option<String>) {
    change(|w| w.file = file.unwrap_or_default());
}

/// Un média occupe le lecteur (vrai dès son ouverture, même en pause ; faux à la sortie du lecteur).
pub fn block(blocked: bool) {
    change(|w| w.blocked = blocked);
}

/// Joue un fichier audio une fois (mods).
pub fn sound(file: &str) {
    if std::path::Path::new(file).is_file() {
        send(Msg::Sound(file.to_string()));
    }
}

fn worker(rx: mpsc::Receiver<Msg>) {
    let mut music: Option<crate::mpv::Mpv> = None;
    let mut effects: Option<crate::mpv::Mpv> = None;
    // Ce qui est chargé dans le lecteur, et l'endroit où reprendre chaque fichier interrompu.
    let mut playing = String::new();
    let mut resume: Option<(String, f64)> = None;
    // Position de la musique (propriété observée « time-pos »).
    let mut pos = 0.0f64;
    loop {
        let msg = match rx.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok(m) => m,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                drain(&music, &effects, &mut pos);
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        drain(&music, &effects, &mut pos);
        match msg {
            Msg::Sound(file) => {
                if effects.is_none() {
                    effects = new_player(false);
                }
                if let Some(m) = &effects {
                    let _ = m.command(&["loadfile", &file, "replace"]);
                }
            }
            Msg::Update => {
                let want = WANT.lock().unwrap().clone().unwrap_or_default();
                let target = if want.on && !want.blocked && std::path::Path::new(&want.file).is_file() { want.file.clone() } else { String::new() };
                if let Some(m) = &music {
                    // Le volume de mpv suit une courbe cubique : le réglage est ramené à une échelle linéaire.
                    let v = want.volume as f64 / 100.0;
                    let _ = m.set_property("volume", &format!("{:.1}", 100.0 * v.cbrt()));
                }
                if target == playing {
                    continue;
                }
                if !playing.is_empty() {
                    if std::env::var_os("TURTLEFIN_DEBUG_MUSIC").is_some() {
                        eprintln!("turtlefin : musique de fond arrêtée à {pos:.1} s");
                    }
                    if let Some(m) = &music {
                        // Interrompue par une lecture : elle reprendra au même endroit.
                        resume = Some((playing.clone(), pos));
                        let _ = m.command(&["stop"]);
                    }
                    playing.clear();
                }
                if target.is_empty() {
                    continue;
                }
                if music.is_none() {
                    music = new_player(true);
                    if let Some(m) = &music {
                        m.observe("time-pos");
                        let v = want.volume as f64 / 100.0;
                        let _ = m.set_property("volume", &format!("{:.1}", 100.0 * v.cbrt()));
                    }
                }
                let Some(m) = &music else { continue };
                let start = resume.take().filter(|(f, _)| *f == target).map(|(_, p)| p).unwrap_or(0.0);
                let _ = m.set_property("start", &format!("{start:.2}"));
                if std::env::var_os("TURTLEFIN_DEBUG_MUSIC").is_some() {
                    eprintln!("turtlefin : musique de fond : {target} (à {start:.1} s)");
                }
                match m.command(&["loadfile", &target, "replace"]) {
                    Ok(()) => {
                        pos = start;
                        playing = target;
                    }
                    Err(e) => eprintln!("turtlefin : musique de fond : {e}"),
                }
            }
        }
    }
}

/// Vide la file des messages des lecteurs (nombre borné : `None` vaut aussi pour un message ignoré) et
/// relève la position de la musique.
fn drain(music: &Option<crate::mpv::Mpv>, effects: &Option<crate::mpv::Mpv>, pos: &mut f64) {
    if let Some(m) = music {
        for _ in 0..64 {
            if let Some(crate::mpv::Event::Property(name, v)) = m.wait_event(0.0) {
                if name == "time-pos" {
                    if let Some(p) = v.as_f64() {
                        *pos = p;
                    }
                }
            }
        }
    }
    if let Some(m) = effects {
        for _ in 0..16 {
            let _ = m.wait_event(0.0);
        }
    }
}

fn new_player(looped: bool) -> Option<crate::mpv::Mpv> {
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
        ("audio-display", "no"),
        ("audio-client-name", "Turtlefin"),
        // Fichiers locaux seulement (dossier du thème, ou son d'un mod).
        ("network-timeout", "1"),
    ] {
        m.set_option(k, v);
    }
    if looped {
        m.set_option("loop-file", "inf");
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
            eprintln!("turtlefin : lecteur de la musique de fond : {e}");
            None
        }
    }
}
