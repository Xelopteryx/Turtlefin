//! Décor jour et nuit : où en est la journée, pour les décors des thèmes (global `Sky` de sky.slint).
//!
//! Heure réelle de l'appareil, ou jour / nuit forcés (Paramètres > Affichage > Thème). Le passage au
//! jour et à la nuit se fait autour de deux heures réglables, sur une durée réglable : la nuit monte
//! pendant le crépuscule et redescend pendant l'aube. Le soleil traverse le décor de l'aube au
//! crépuscule, la lune du crépuscule à l'aube (de droite à gauche). Les valeurs affichées rejoignent
//! la cible en douceur : un changement de réglage se voit comme une courte transition, pas un saut.

use crate::{config, AppWindow, Sky};
use slint::ComponentHandle;
use std::cell::Cell;
use std::f32::consts::PI;

/// État du ciel pour les décors.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    /// 0 le jour, 1 la nuit.
    pub night: f32,
    /// Soleil : avancement de sa course (0 entre à droite, 1 sort à gauche) et hauteur (0 à 1).
    pub sun: Option<(f32, f32)>,
    /// Lune : de même.
    pub moon: Option<(f32, f32)>,
    /// Aurore boréale (décor Aero) : 0 à 1.
    pub aurora: f32,
    /// Matin (aube) plutôt que soir (crépuscule) : le décor Nerd redémarre au lieu de se mettre en veille.
    pub dawn: bool,
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Course d'un astre qui se lève à `rise` et se couche à `set` (minutes de la journée).
fn arc(m: f32, rise: f32, set: f32) -> Option<(f32, f32)> {
    let span = (set - rise).rem_euclid(1440.0);
    if span <= 0.0 {
        return None;
    }
    let u = (m - rise).rem_euclid(1440.0) / span;
    (u <= 1.0).then(|| (u, (PI * u).sin()))
}

/// État à la minute `m` de la journée, avec le passage au jour à `day`, à la nuit à `night` (milieux de
/// l'aube et du crépuscule) et des transitions de `span` minutes.
pub fn at(m: f32, day: f32, night: f32, span: f32) -> State {
    let span = span.max(5.0);
    let half = span / 2.0;
    let dusk_in = (m - (night - half)).rem_euclid(1440.0);
    let dawn_in = (m - (day - half)).rem_euclid(1440.0);
    let day_len = ((night - half) - (day + half)).rem_euclid(1440.0);
    let n = if dusk_in < span {
        smooth(dusk_in / span)
    } else if dawn_in < span {
        1.0 - smooth(dawn_in / span)
    } else if (m - (day + half)).rem_euclid(1440.0) < day_len {
        0.0
    } else {
        1.0
    };
    // Aurore boréale : s'allume dans les 40 minutes qui suivent la fin du crépuscule, s'éteint dans les
    // 40 minutes qui précèdent l'aube.
    let after_dusk = (m - (night + half)).rem_euclid(1440.0);
    let before_dawn = ((day - half) - m).rem_euclid(1440.0);
    let night_len = ((day - half) - (night + half)).rem_euclid(1440.0);
    let aurora = if after_dusk < night_len { smooth(after_dusk / 40.0).min(smooth(before_dawn / 40.0)) } else { 0.0 };
    State {
        night: n,
        sun: arc(m, day - half * 0.6, night + half * 0.2),
        moon: arc(m, night + half * 0.4, day - half * 0.8),
        aurora,
        dawn: dawn_in < span,
    }
}

/// Minute de la journée que montrent les décors : l'heure réelle, ou un milieu de journée (jour forcé)
/// ou de nuit (nuit forcée). Sert aux décors qui ont des teintes par heure (le lagon).
pub fn scene_minute() -> f32 {
    let p = config::ui_prefs();
    if let Some((h, m)) = std::env::var("TURTLEFIN_SKY_TIME").ok().and_then(|v| v.split_once(':').map(|(h, m)| (h.parse::<f32>(), m.parse::<f32>()))).and_then(|(h, m)| Some((h.ok()?, m.ok()?))) {
        return h * 60.0 + m;
    }
    let (d, n) = (p.day_at as f32, p.night_at as f32);
    match p.daynight_mode.as_str() {
        "day" => d + (n - d).rem_euclid(1440.0) / 2.0,
        "night" => (n + (d - n).rem_euclid(1440.0) / 2.0).rem_euclid(1440.0),
        _ => {
            use chrono::Timelike;
            let now = chrono::Local::now();
            now.hour() as f32 * 60.0 + now.minute() as f32 + now.second() as f32 / 60.0
        }
    }
}

/// État voulu maintenant, selon les réglages.
pub fn target() -> State {
    let p = config::ui_prefs();
    // Essais : TURTLEFIN_SKY_TIME=HH:MM fige l'heure du décor.
    if let Some((h, m)) = std::env::var("TURTLEFIN_SKY_TIME").ok().and_then(|v| v.split_once(':').map(|(h, m)| (h.parse::<f32>(), m.parse::<f32>()))).and_then(|(h, m)| Some((h.ok()?, m.ok()?))) {
        return at(h * 60.0 + m, p.day_at as f32, p.night_at as f32, p.daynight_span as f32);
    }
    match p.daynight_mode.as_str() {
        "day" => State { night: 0.0, sun: Some((0.5, 1.0)), moon: None, aurora: 0.0, dawn: false },
        "night" => State { night: 1.0, sun: None, moon: Some((0.5, 1.0)), aurora: 1.0, dawn: false },
        _ => {
            use chrono::Timelike;
            let now = chrono::Local::now();
            let m = now.hour() as f32 * 60.0 + now.minute() as f32 + now.second() as f32 / 60.0;
            at(m, p.day_at as f32, p.night_at as f32, p.daynight_span as f32)
        }
    }
}

thread_local! {
    static SHOWN: Cell<Option<State>> = const { Cell::new(None) };
}

/// Rapproche l'état affiché de la cible (une demi-seconde à chaque appel) et le pousse dans `Sky`.
fn step(u: &AppWindow, dt: f32) {
    let goal = target();
    let shown = SHOWN.with(|s| s.get()).unwrap_or(goal);
    let k = (dt / 2.0).min(1.0);
    let ease = |a: f32, b: f32| a + (b - a) * k;
    let body = |a: Option<(f32, f32)>, b: Option<(f32, f32)>| match (a, b) {
        (Some(a), Some(b)) => Some((ease(a.0, b.0), ease(a.1, b.1))),
        // Un astre qui apparaît ou disparaît le fait par le bord (sa course l'y amène), pas en fondu.
        (_, b) => b,
    };
    let s = State { night: ease(shown.night, goal.night), sun: body(shown.sun, goal.sun), moon: body(shown.moon, goal.moon), aurora: ease(shown.aurora, goal.aurora), dawn: goal.dawn };
    SHOWN.with(|c| c.set(Some(s)));
    let g = u.global::<Sky>();
    g.set_night(s.night);
    g.set_sun_on(s.sun.is_some());
    g.set_sun_u(s.sun.map(|x| x.0).unwrap_or(0.0));
    g.set_sun_h(s.sun.map(|x| x.1).unwrap_or(0.0));
    g.set_moon_on(s.moon.is_some());
    g.set_moon_u(s.moon.map(|x| x.0).unwrap_or(0.0));
    g.set_moon_h(s.moon.map(|x| x.1).unwrap_or(0.0));
    g.set_aurora(s.aurora);
    g.set_dawn(s.dawn);
}

/// Démarre la mise à jour du ciel (garder la minuterie rendue). Premier état posé tout de suite.
pub fn start(u: &AppWindow) -> slint::Timer {
    u.global::<Sky>().set_hex(hex_path().into());
    SHOWN.with(|c| c.set(Some(target())));
    step(u, 0.0);
    let t = slint::Timer::default();
    let w = u.as_weak();
    t.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(250), move || {
        if let Some(u) = w.upgrade() {
            step(&u, 0.25);
        }
    });
    t
}

/// Trame hexagonale du décor Turtlefin (repère 1600 x 900, hexagones de 46 de rayon, pointe en haut).
fn hex_path() -> String {
    let r = 46.0_f32;
    let w = 3.0_f32.sqrt() * r;
    let mut s = String::new();
    let rows = (900.0 / (r * 1.5)) as i32 + 2;
    let cols = (1600.0 / w) as i32 + 2;
    for row in -1..rows {
        for col in -1..cols {
            let cx = col as f32 * w + if row % 2 != 0 { w / 2.0 } else { 0.0 };
            let cy = row as f32 * r * 1.5;
            for k in 0..6 {
                let a = std::f32::consts::PI / 6.0 + k as f32 * std::f32::consts::PI / 3.0;
                s.push_str(&format!("{} {:.1} {:.1} ", if k == 0 { "M" } else { "L" }, cx + a.cos() * r, cy + a.sin() * r));
            }
            s.push_str("Z ");
        }
    }
    s
}

/// Libellé d'une minute de la journée (« 7:00 »).
pub fn hm(m: u32) -> String {
    format!("{}:{:02}", m / 60 % 24, m % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_and_night() {
        // Jour à 7:00, nuit à 19:00, transitions d'une heure.
        assert_eq!(at(720.0, 420.0, 1140.0, 60.0).night, 0.0);
        assert_eq!(at(0.0, 420.0, 1140.0, 60.0).night, 1.0);
        let dusk = at(1140.0, 420.0, 1140.0, 60.0);
        assert!((dusk.night - 0.5).abs() < 1e-3);
        assert!(!dusk.dawn);
        let dawn = at(420.0, 420.0, 1140.0, 60.0);
        assert!((dawn.night - 0.5).abs() < 1e-3);
        assert!(dawn.dawn);
        assert!(at(720.0, 420.0, 1140.0, 60.0).sun.is_some());
        assert!(at(720.0, 420.0, 1140.0, 60.0).moon.is_none());
        assert!(at(0.0, 420.0, 1140.0, 60.0).moon.is_some());
        assert!(at(0.0, 420.0, 1140.0, 60.0).aurora > 0.99);
        // Nuit qui passe minuit (23:30) : toujours cohérent.
        assert!(at(1430.0, 420.0, 1410.0, 60.0).night > 0.4);
        assert_eq!(at(120.0, 420.0, 1410.0, 60.0).night, 1.0);
    }
}
