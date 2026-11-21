//! Détail du décor animé (global `Perf.decor` de theme.slint) : 0 complet, 1 allégé, 2 figé.
//!
//! Réglage « Détail du décor » (Paramètres → Affichage → Thème) : automatique, complet, allégé ou figé.
//! En automatique, Turtlefin mesure ce que coûte vraiment une image (carte graphique comprise : on
//! attend qu'elle ait fini, glFinish) sur 40 images d'affilée, quelques secondes après le lancement.
//! Au-delà de `LIMIT_MS`, le décor descend d'un niveau, puis la mesure reprend ; le niveau retenu est
//! gardé pour cette carte graphique et ce pilote (rien n'est remesuré au lancement suivant, sauf si
//! le pilote change). Sans accélération graphique (rendu par le processeur), le décor est figé d'office.
//! Turtlefin vise 60 images/s : 16,7 ms par image, avec de la marge pour les animations des pages.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use slint::ComponentHandle;

use crate::{config, AppWindow, Perf, Theme};

/// Coût d'une image au-delà duquel le décor s'allège (médiane de la mesure).
const LIMIT_MS: f32 = 14.5;
const SAMPLES: usize = 40;
/// Attente avant la première mesure (démarrage, chargement de l'accueil) et après un changement.
const SETTLE: Duration = Duration::from_secs(6);

pub const MODES: [&str; 4] = ["", "full", "light", "still"];

static PROBING: AtomicBool = AtomicBool::new(false);

struct State {
    samples: Vec<f32>,
    resume_at: Instant,
    gpu: String,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn level_of(mode: &str) -> Option<i32> {
    match mode {
        "full" => Some(0),
        "light" => Some(1),
        "still" => Some(2),
        _ => None,
    }
}

/// Rendu par le processeur (pas d'accélération graphique) : llvmpipe, softpipe, SwiftShader…
fn software(gpu: &str) -> bool {
    let g = gpu.to_ascii_lowercase();
    ["llvmpipe", "softpipe", "swiftshader", "software rasterizer", "gdi generic"].iter().any(|s| g.contains(s))
}

/// Carte graphique connue (premier rendu, voir video.rs) : niveau du décor posé selon le réglage, et
/// mesure lancée si besoin.
pub fn gpu_known(u: &AppWindow, gpu: &str) {
    *STATE.lock().unwrap() = Some(State { samples: Vec::new(), resume_at: Instant::now() + SETTLE, gpu: gpu.to_string() });
    apply(u);
}

/// Pose le niveau du décor selon le réglage (au lancement, quand le réglage ou le thème change).
pub fn apply(u: &AppWindow) {
    let p = config::ui_prefs();
    let gpu = STATE.lock().unwrap().as_ref().map(|s| s.gpu.clone()).unwrap_or_default();
    PROBING.store(false, Ordering::SeqCst);
    let level = match level_of(&p.decor_mode) {
        Some(l) => l,
        None if software(&gpu) => {
            remember(u, 2, &gpu);
            2
        }
        None if !gpu.is_empty() && p.decor_gpu == key(u, &gpu) => p.decor_auto.min(2) as i32,
        None => {
            // Carte graphique pas encore mesurée (ou pilote changé) : complet, puis mesure.
            if let Some(s) = STATE.lock().unwrap().as_mut() {
                s.samples.clear();
                s.resume_at = Instant::now() + SETTLE;
            }
            PROBING.store(!gpu.is_empty(), Ordering::SeqCst);
            0
        }
    };
    u.global::<Perf>().set_decor(level);
}

/// Ce que la mesure retenue concerne : la carte graphique (et son pilote) et le décor du thème, qui ne
/// coûtent pas la même chose (le lagon d'Aero bien plus que les autres).
fn key(u: &AppWindow, gpu: &str) -> String {
    format!("{gpu} | décor {}", u.global::<Theme>().get_scenery())
}

fn remember(u: &AppWindow, level: i32, gpu: &str) {
    let key = key(u, gpu);
    let mut p = config::ui_prefs();
    if p.decor_auto as i32 != level || p.decor_gpu != key {
        p.decor_auto = level as u8;
        p.decor_gpu = key;
        config::save_ui_prefs(&p);
    }
}

/// Une mesure est-elle en cours (video.rs attend alors la fin de chaque image pour la chronométrer) ?
pub fn probing() -> bool {
    PROBING.load(Ordering::Relaxed)
}

/// Durée d'une image, carte graphique comprise (images d'affilée seulement, voir video.rs).
pub fn sample(u: &AppWindow, ms: f32) {
    let mut guard = STATE.lock().unwrap();
    let Some(s) = guard.as_mut() else { return };
    // Pendant la lecture, l'image est surtout la vidéo : ce n'est pas le décor qu'on mesure.
    if Instant::now() < s.resume_at || u.get_playing() {
        return;
    }
    s.samples.push(ms);
    if s.samples.len() < SAMPLES {
        return;
    }
    let mut v = std::mem::take(&mut s.samples);
    v.sort_by(|a, b| a.total_cmp(b));
    let med = v[v.len() / 2];
    let perf = u.global::<Perf>();
    let level = perf.get_decor();
    eprintln!("turtlefin : décor, {med:.1} ms par image (niveau {level})");
    let gpu = s.gpu.clone();
    drop(guard);
    if med > LIMIT_MS && level < 2 {
        perf.set_decor(level + 1);
        remember(u, level + 1, &gpu);
        if level + 1 < 2 {
            if let Some(s) = STATE.lock().unwrap().as_mut() {
                s.resume_at = Instant::now() + Duration::from_secs(2);
            }
            return;
        }
    } else {
        remember(u, level, &gpu);
    }
    PROBING.store(false, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_and_software() {
        assert_eq!(level_of("light"), Some(1));
        assert_eq!(level_of(""), None);
        assert!(software("OpenGL 4.5 (Core Profile) Mesa 26.0.8 · llvmpipe (LLVM 21.1.8, 256 bits)"));
        assert!(!software("OpenGL ES 3.1 Mesa 26.2.2 · V3D 7.1.7.0"));
    }
}
