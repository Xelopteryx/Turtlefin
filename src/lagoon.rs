//! Décor du thème Aero : le lagon. Vu toujours du même endroit, seule la lumière change avec l'heure :
//! le soleil puis la lune traversent la surface (de droite à gauche), l'eau passe du bleu à l'or, au
//! rose, au violet, au bleu nuit, puis au rose de l'aube ; les rayons s'inclinent et raccourcissent
//! quand l'astre baisse. La nuit, le plancton s'allume grain par grain, les coraux deviennent
//! fluorescents, des méduses arrivent à la nage par les côtés et l'aurore boréale s'allume dans la
//! surface. Les parties fixes sont des images (ui/lagoon, produites par tools/lagoon) ; ce fichier
//! calcule ce qui bouge (méduses, anémones, poissons) et la lumière, et les passe à Slint (global
//! `Lagoon`, sky.slint), 30 fois par seconde quand le décor est affiché et animé.

use crate::{config, AppWindow, Lagoon, LagoonAnemone, LagoonBubble, LagoonDot, LagoonFish, LagoonJelly, LagoonSpeck, Sky, Theme};
use slint::{Color, ComponentHandle, Model, ModelRc, VecModel};
use serde::Deserialize;
use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;
use std::time::{Duration, Instant};

const W: f32 = 1600.0;

#[derive(Deserialize, Clone)]
struct JellyDef {
    x: f32,
    y: f32,
    s: f32,
    th: f32,
    ph: f32,
    side: f32,
}
#[derive(Deserialize, Clone)]
struct AnemoneDef {
    x: f32,
    y: f32,
    s: f32,
    ph: f32,
    fish: bool,
}
#[derive(Deserialize, Clone)]
struct ChromisDef {
    dx: f32,
    dy: f32,
    ph: f32,
    s: f32,
}
#[derive(Deserialize)]
struct FanDef {
    x: f32,
    y: f32,
    w: f32,
    ph: f32,
}
#[derive(Deserialize)]
struct BubDef {
    col: usize,
    off: f32,
    r: f32,
    sp: f32,
    ph: f32,
}
#[derive(Deserialize)]
struct SchoolDef {
    dx: f32,
    dy: f32,
    s: f32,
    ph: f32,
}
#[derive(Deserialize)]
struct SpeckDef {
    x: f32,
    y: f32,
    r: f32,
    a: f32,
    sp: f32,
    ph: f32,
    lum: bool,
    th: f32,
}
#[derive(Deserialize)]
struct Data {
    jellies: Vec<JellyDef>,
    anemones: Vec<AnemoneDef>,
    chromis: Vec<ChromisDef>,
    fans: Vec<FanDef>,
    cols: Vec<[f32; 2]>,
    bub: Vec<BubDef>,
    fish: Vec<SchoolDef>,
    plankton: Vec<SpeckDef>,
}

/// Images du lagon (ui/lagoon), décodées au premier affichage du décor, dans un fil à part.
fn load_images(w: slint::Weak<AppWindow>) {
    std::thread::spawn(move || {
        let files: [(&str, &[u8]); 21] = [
            ("reef_back", include_bytes!("../ui/lagoon/reef_back.webp")),
            ("reef_day", include_bytes!("../ui/lagoon/reef_day.webp")),
            ("reef_night", include_bytes!("../ui/lagoon/reef_night.webp")),
            ("reef_glow", include_bytes!("../ui/lagoon/reef_glow.webp")),
            ("sand", include_bytes!("../ui/lagoon/sand.webp")),
            ("rays", include_bytes!("../ui/lagoon/rays.webp")),
            ("c1", include_bytes!("../ui/lagoon/c1.webp")),
            ("c2", include_bytes!("../ui/lagoon/c2.webp")),
            ("win", include_bytes!("../ui/lagoon/win.webp")),
            ("aur1", include_bytes!("../ui/lagoon/aur1.webp")),
            ("aur2", include_bytes!("../ui/lagoon/aur2.webp")),
            ("fan_day", include_bytes!("../ui/lagoon/fan_day.webp")),
            ("fan_night", include_bytes!("../ui/lagoon/fan_night.webp")),
            ("bubble", include_bytes!("../ui/lagoon/bubble.png")),
            ("dot", include_bytes!("../ui/lagoon/dot.png")),
            ("jglow", include_bytes!("../ui/lagoon/jglow.png")),
            ("clown_r", include_bytes!("../ui/lagoon/clown_r.png")),
            ("clown_l", include_bytes!("../ui/lagoon/clown_l.png")),
            ("chromis_r", include_bytes!("../ui/lagoon/chromis_r.png")),
            ("chromis_l", include_bytes!("../ui/lagoon/chromis_l.png")),
            ("fish", include_bytes!("../ui/lagoon/fish.png")),
        ];
        let mut out: Vec<(&str, slint::SharedPixelBuffer<slint::Rgba8Pixel>)> = Vec::new();
        for (name, bytes) in files {
            let Ok(img) = image::load_from_memory(bytes) else { continue };
            let rgba = img.to_rgba8();
            out.push((name, slint::SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height())));
        }
        let _ = w.upgrade_in_event_loop(move |u| {
            let g = u.global::<Lagoon>();
            for (name, buf) in out {
                let img = slint::Image::from_rgba8(buf);
                match name {
                    "reef_back" => g.set_reef_back(img),
                    "reef_day" => g.set_reef_day(img),
                    "reef_night" => g.set_reef_night(img),
                    "reef_glow" => g.set_reef_glow(img),
                    "sand" => g.set_sand(img),
                    "rays" => g.set_rays(img),
                    "c1" => g.set_c1(img),
                    "c2" => g.set_c2(img),
                    "win" => g.set_win(img),
                    "aur1" => g.set_aur1(img),
                    "aur2" => g.set_aur2(img),
                    "fan_day" => g.set_fan_day(img),
                    "fan_night" => g.set_fan_night(img),
                    "bubble" => g.set_bubble(img),
                    "dot" => g.set_dot(img),
                    "jglow" => g.set_jglow(img),
                    "clown_r" => g.set_clown_r(img),
                    "clown_l" => g.set_clown_l(img),
                    "chromis_r" => g.set_chromis_r(img),
                    "chromis_l" => g.set_chromis_l(img),
                    _ => g.set_fish_img(img),
                }
            }
            g.set_ready(true);
        });
    });
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Out,
    Arriving,
    In,
    Leaving,
}

struct Jelly {
    d: JellyDef,
    state: State,
    px: f32,
    py: f32,
    vx: f32,
    vy: f32,
    ang: f32,
    c: f32,
    cp: f32,
    hist: Vec<(f32, f32)>,
}

struct Live {
    jellies: Vec<Jelly>,
    anemones: Vec<AnemoneDef>,
    chromis: Vec<ChromisDef>,
    jelly_model: Rc<VecModel<LagoonJelly>>,
    anem_model: Rc<VecModel<LagoonAnemone>>,
    fish_model: Rc<VecModel<LagoonFish>>,
    tips: Rc<VecModel<LagoonDot>>,
    t0: Instant,
    last: Instant,
    fish_lap: i64,
    fish_on: bool,
}

thread_local! {
    static LIVE: RefCell<Option<Live>> = const { RefCell::new(None) };
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn hex(s: &str) -> [f32; 3] {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
    [((v >> 16) & 255) as f32, ((v >> 8) & 255) as f32, (v & 255) as f32]
}
fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}
fn color(c: [f32; 3], a: f32) -> Color {
    Color::from_argb_u8((a.clamp(0.0, 1.0) * 255.0) as u8, c[0] as u8, c[1] as u8, c[2] as u8)
}

/// Couleur de l'eau (cinq teintes, de la surface au fond) à la minute `m`, avec le passage au jour à
/// `d`, à la nuit à `n` et des transitions de `s` minutes (repères de la démo, étirés selon `s`).
fn water(m: f32, d: f32, n: f32, s: f32) -> [[f32; 3]; 5] {
    let k = s / 60.0;
    let c = |a: [&str; 5]| a.map(hex);
    let night = c(["#0c2f5c", "#08214a", "#051634", "#030c22", "#010512"]);
    let day = c(["#38b6e6", "#1683c6", "#0a4f98", "#062c66", "#02132e"]);
    let keys: [(f32, [[f32; 3]; 5]); 10] = [
        (d - 40.0 * k, night),
        (d, c(["#3a3a7a", "#2a3f80", "#16306a", "#0a1c46", "#03102a"])),
        (d + 30.0 * k, c(["#f2a0a8", "#9a86c0", "#3a68a8", "#123a78", "#041a40"])),
        (d + 65.0 * k, c(["#7fd0ea", "#2f97cf", "#0e5aa0", "#062e68", "#02132e"])),
        (d + 150.0 * k, day),
        (n - 95.0 * k, day),
        (n - 50.0 * k, c(["#f0c070", "#5aa8c8", "#0e5a98", "#062c66", "#02132e"])),
        (n - 20.0 * k, c(["#ff8a5a", "#c0607a", "#3a4a8a", "#14275e", "#040f2a"])),
        (n + 10.0 * k, c(["#4a3a7a", "#2a2e6a", "#141f50", "#0a1434", "#020818"])),
        (n + 45.0 * k, night),
    ];
    // Position de m dans la journée, en partant du premier repère (gère le passage de minuit).
    let base = keys[0].0;
    let rel = |x: f32| (x - base).rem_euclid(1440.0);
    let mm = rel(m);
    for i in 1..keys.len() {
        let (a, b) = (rel(keys[i - 1].0), rel(keys[i].0));
        if mm <= b {
            let u = if b > a { smooth((mm - a) / (b - a)) } else { 1.0 };
            let mut out = keys[i - 1].1;
            for j in 0..5 {
                out[j] = mix(keys[i - 1].1[j], keys[i].1[j], u);
            }
            return out;
        }
    }
    night
}

/// Nage d'une méduse (voir la démo) : poussées à chaque contraction, glissade entre deux.
fn swim(j: &mut Jelly, dark: f32, dt: f32, t: f32, snap: bool) {
    let want = if dark >= j.d.th { true } else if dark < j.d.th - 0.05 { false } else { matches!(j.state, State::In | State::Arriving) };
    let side = if j.d.side < 0.0 { -300.0 } else { W + 300.0 };
    if snap {
        j.state = if want { State::In } else { State::Out };
        j.px = j.d.x;
        j.py = j.d.y;
        j.vx = 0.0;
        j.vy = 0.0;
        j.ang = 0.0;
        j.c = 0.0;
        j.cp = (j.d.ph * 0.37).rem_euclid(1.0);
        j.hist.clear();
        return;
    }
    if want && j.state == State::Out {
        j.state = State::Arriving;
        j.px = side;
        j.py = j.d.y + 60.0 * (j.d.ph * 3.0).sin();
        j.vx = 0.0;
        j.vy = 0.0;
        j.ang = if j.d.side < 0.0 { 1.25 } else { -1.25 };
        j.hist.clear();
    }
    if want && j.state == State::Leaving {
        j.state = State::Arriving;
    }
    if !want && matches!(j.state, State::In | State::Arriving) {
        j.state = State::Leaving;
    }
    if j.state == State::Out {
        return;
    }
    let (tx, ty) = if j.state == State::Leaving { (side, j.py - 40.0) } else { (j.d.x + (t * 0.07 + j.d.ph).sin() * 40.0, j.d.y + (t * 0.05 + j.d.ph).sin() * 25.0) };
    let (dx, dy) = (tx - j.px, ty - j.py);
    let dist = (dx * dx + dy * dy).sqrt();
    let travel = j.state == State::Leaving || dist > 70.0 * j.d.s;
    let desired = if travel { dx.atan2(-dy).clamp(-1.45, 1.45) } else { (t * 0.4 + j.d.ph).sin() * 0.08 };
    j.ang += (desired - j.ang) * (dt * 0.9).min(1.0);
    let period = (2.0 + j.d.s * 1.2) * match j.state {
        State::Leaving => 0.6,
        State::Arriving => 0.8,
        _ => 1.0,
    };
    const TH: f32 = 0.28;
    j.cp = (j.cp + dt / period).rem_euclid(1.0);
    j.c = if j.cp < TH { (j.cp / TH * std::f32::consts::PI).sin() } else { 0.0 };
    let push = if travel { 1.0 } else { ((j.py - ty) / (40.0 * j.d.s)).clamp(0.0, 1.2) };
    if j.cp < TH {
        let boost = match j.state {
            State::Leaving => 1.8,
            State::Arriving => 1.3,
            _ => 1.0,
        };
        let a = 270.0 * j.d.s * push * j.c * boost;
        j.vx += j.ang.sin() * a * dt;
        j.vy -= j.ang.cos() * a * dt;
    }
    j.vy += 9.0 * dt;
    let drag = 0.35f32.powf(dt);
    j.vx *= drag;
    j.vy *= drag;
    if !travel {
        j.vx += dx * 0.25 * dt;
    }
    j.px += j.vx * dt;
    j.py += j.vy * dt;
    if j.state == State::Arriving && dist < 45.0 * j.d.s {
        j.state = State::In;
    }
    if j.state == State::Leaving && (j.px < -320.0 || j.px > W + 320.0) {
        j.state = State::Out;
        j.hist.clear();
    }
}

/// Courbe lissée (milieux des segments) d'une suite de points.
fn curve(s: &mut String, pts: &[(f32, f32)]) {
    let Some(first) = pts.first() else { return };
    let _ = write!(s, "M {:.1} {:.1} ", first.0, first.1);
    for i in 1..pts.len().saturating_sub(1) {
        let (p, q) = (pts[i], pts[i + 1]);
        let _ = write!(s, "Q {:.1} {:.1} {:.1} {:.1} ", p.0, p.1, (p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0);
    }
    if let Some(l) = pts.last() {
        let _ = write!(s, "L {:.1} {:.1} ", l.0, l.1);
    }
}

fn ellipse(s: &mut String, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, ox: f32, oy: f32, ang: f32) {
    // Ellipse tournée de `rot`, dans le repère de la cloche (origine ox, oy, inclinaison ang).
    let n = 16;
    for i in 0..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let (ex, ey) = (a.cos() * rx, a.sin() * ry);
        let (lx, ly) = (cx + ex * rot.cos() - ey * rot.sin(), cy + ex * rot.sin() + ey * rot.cos());
        let (x, y) = (ox + lx * ang.cos() - ly * ang.sin(), oy + lx * ang.sin() + ly * ang.cos());
        let _ = write!(s, "{} {:.1} {:.1} ", if i == 0 { "M" } else { "L" }, x, y);
    }
    s.push_str("Z ");
}

/// Tracés d'une méduse (repère 1600 x 900).
fn jelly_paths(j: &mut Jelly, t: f32, anim: bool) -> LagoonJelly {
    let (x, y, ang, c, s) = (j.px, j.py, j.ang, j.c, j.d.s);
    let r = 62.0 * s;
    let rr = r * (1.0 - 0.24 * c);
    let bh = r * (0.82 + 0.26 * c);
    let (ux, uy, rx, ry) = (ang.sin(), -ang.cos(), ang.cos(), ang.sin());
    if anim {
        j.hist.push((x, y));
        if j.hist.len() > 60 {
            j.hist.remove(0);
        }
    } else {
        j.hist = vec![(x, y)];
    }
    let hist = &j.hist;
    let past = |n: usize| -> (f32, f32) {
        let q = hist[hist.len().saturating_sub(1 + n)];
        let (mut ox, mut oy) = (q.0 - x, q.1 - y);
        let d = (ox * ox + oy * oy).sqrt();
        let mx = r * 1.2;
        if d > mx {
            ox *= mx / d;
            oy *= mx / d;
        }
        (ox, oy)
    };
    let wob = |i: f32, sp: f32, ph: f32| if anim { (t * sp - i * 0.42 + ph).sin() } else { (-i * 0.42 + ph).sin() };
    let mut arms = String::new();
    for a in 0..4 {
        let ox = (a as f32 - 1.5) * rr * 0.16;
        let n = 14;
        let len = r * 1.5;
        let pts: Vec<(f32, f32)> = (0..=n)
            .map(|i| {
                let f = i as f32 / n as f32;
                let lag = past((i as f32 * 1.6).round() as usize);
                let w1 = wob(i as f32, 1.1, a as f32 * 1.7 + j.d.ph) * r * 0.16 * f;
                let w2 = wob(i as f32 * 2.3, 1.7, a as f32) * r * 0.05 * f;
                let k = (f * 1.4).min(1.0);
                (x + rx * (ox + w1 + w2) - ux * (len * f + r * 0.05) + lag.0 * k, y + ry * (ox + w1 + w2) - uy * (len * f + r * 0.05) + lag.1 * k)
            })
            .collect();
        curve(&mut arms, &pts);
    }
    let mut tent = String::new();
    let nt = 14;
    for k in 0..nt {
        let fx = (k as f32 + 0.5) / nt as f32 * 2.0 - 1.0;
        let sq = (1.0 - fx * fx).max(0.0).sqrt();
        let ax = x + rx * fx * rr * 0.96 - ux * sq * r * 0.04;
        let ay = y + ry * fx * rr * 0.96 - uy * sq * r * 0.04;
        let n2 = 22;
        let len2 = r * (2.4 + ((k * 7) % 5) as f32 * 0.25) * (1.0 - 0.12 * c);
        let mut pts = vec![(ax, ay)];
        for i in 1..=n2 {
            let f = i as f32 / n2 as f32;
            let lag = past((i as f32 * 1.4).round() as usize);
            let spread = fx * r * (0.15 + 0.35 * (1.0 - c)) * f;
            let wave = wob(i as f32, 2.1 + (k % 3) as f32 * 0.3, k as f32 * 1.3 + j.d.ph) * r * 0.2 * f * f;
            let kk = (f * 1.3).min(1.0);
            pts.push((ax + rx * (spread + wave) - ux * len2 * f + lag.0 * kk, ay + ry * (spread + wave) - uy * len2 * f + lag.1 * kk));
        }
        curve(&mut tent, &pts);
    }
    // Cloche, dans son repère (origine au bord, tournée de ang), ramenée dans celui de l'écran.
    let tr = |lx: f32, ly: f32| (x + lx * ang.cos() - ly * ang.sin(), y + lx * ang.sin() + ly * ang.cos());
    let mut bell = String::new();
    let p = tr(-rr, 0.0);
    let _ = write!(bell, "M {:.1} {:.1} ", p.0, p.1);
    let cubic = |s: &mut String, a: (f32, f32), b: (f32, f32), e: (f32, f32)| {
        let (a, b, e) = (tr(a.0, a.1), tr(b.0, b.1), tr(e.0, e.1));
        let _ = write!(s, "C {:.1} {:.1} {:.1} {:.1} {:.1} {:.1} ", a.0, a.1, b.0, b.1, e.0, e.1);
    };
    cubic(&mut bell, (-rr * 1.02, -bh * 0.85), (-rr * 0.55, -bh), (0.0, -bh));
    cubic(&mut bell, (rr * 0.55, -bh), (rr * 1.02, -bh * 0.85), (rr, 0.0));
    let nl = 10;
    for l in (0..nl).rev() {
        let lx0 = -rr + 2.0 * rr * (l + 1) as f32 / nl as f32;
        let lx1 = -rr + 2.0 * rr * l as f32 / nl as f32;
        let dip = r * 0.07 + if anim { (t * 3.2 + l as f32 * 0.9 + j.d.ph).sin() * r * 0.03 } else { 0.0 };
        let (cq, e) = (tr((lx0 + lx1) / 2.0, dip), tr(lx1, 0.0));
        let _ = write!(bell, "Q {:.1} {:.1} {:.1} {:.1} ", cq.0, cq.1, e.0, e.1);
    }
    bell.push_str("Z ");
    // Lueurs intérieures, sous-ombrelle et reflet.
    let mut organs = String::new();
    for q in 0..4 {
        let qa = q as f32 * std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4;
        ellipse(&mut organs, qa.cos() * rr * 0.3, -bh * 0.45 + qa.sin() * bh * 0.1, r * 0.14, r * 0.1, qa, x, y, ang);
    }
    let mut under = String::new();
    ellipse(&mut under, 0.0, -bh * 0.12, rr * 0.78, bh * 0.22, 0.0, x, y, ang);
    let mut shine = String::new();
    ellipse(&mut shine, -rr * 0.25, -bh * 0.72, rr * 0.32, bh * 0.12, -0.4, x, y, ang);
    let (gx, gy) = (x + ux * bh * 0.5, y + uy * bh * 0.5);
    LagoonJelly {
        on: true,
        bell: bell.into(),
        arms: arms.into(),
        tentacles: tent.into(),
        organs: organs.into(),
        under: under.into(),
        shine: shine.into(),
        gx,
        gy,
        gr: r * 2.2,
        c,
        s,
    }
}

/// Tracés d'une anémone : colonne, tentacules (deux couronnes), bouts des tentacules.
fn anemone_paths(a: &AnemoneDef, t: f32, anim: bool, tips: &mut Vec<LagoonDot>) -> LagoonAnemone {
    let s = a.s;
    let mut col = String::new();
    let _ = write!(col, "M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1} L {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1} Z ", a.x - 18.0 * s, a.y + 30.0 * s, a.x - 14.0 * s, a.y, a.x - 22.0 * s, a.y - 4.0 * s, a.x + 22.0 * s, a.y - 4.0 * s, a.x + 14.0 * s, a.y, a.x + 18.0 * s, a.y + 30.0 * s);
    let mut outer = String::new();
    let mut inner = String::new();
    let mut ends = String::new();
    for ring in 0..2 {
        for k in 0..18 {
            let f = k as f32 / 17.0;
            let ang = (-1.3 + f * 2.6) * if ring == 1 { 0.75 } else { 1.0 };
            let len = if ring == 1 { 30.0 } else { 42.0 } * s * (1.0 + (k as f32 * 2.1).sin() * 0.15);
            let sw = if anim { (t * 1.4 - f * 2.4 + a.ph + ring as f32).sin() * 10.0 * s } else { 0.0 };
            let bx = a.x + ang.sin() * 15.0 * s;
            let by = a.y - 5.0 * s - ring as f32 * 3.0 * s;
            let ex = bx + ang.sin() * len + sw;
            let ey = by - ang.cos() * len * 0.9 + sw.abs() * 0.25;
            let (cx, cy) = (bx + ang.sin() * len * 0.5 + sw * 0.3, by - ang.cos() * len * 0.6);
            let out = if ring == 0 { &mut outer } else { &mut inner };
            let _ = write!(out, "M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1} ", bx, by, cx, cy, ex, ey);
            // Bout clair : le dernier quart du tentacule.
            let (mx, my) = ((bx + 2.0 * cx + ex) / 4.0 * 0.5 + ex * 0.5, (by + 2.0 * cy + ey) / 4.0 * 0.5 + ey * 0.5);
            let _ = write!(ends, "M {:.1} {:.1} L {:.1} {:.1} ", mx, my, ex, ey);
            tips.push(LagoonDot { x: ex, y: ey, a: 1.0, r: 7.0 * s });
        }
    }
    LagoonAnemone { column: col.into(), outer: outer.into(), inner: inner.into(), tips: ends.into(), s }
}

/// Le décor Aero (lagon) est-il à l'écran ?
fn shown(u: &AppWindow) -> bool {
    u.global::<Theme>().get_scenery() == 5 && !u.get_playing()
}

/// Une image : lumière selon l'heure, méduses, anémones, poissons.
fn frame(u: &AppWindow, anim: bool, snap: bool) {
    LIVE.with(|l| {
        let mut l = l.borrow_mut();
        let Some(live) = l.as_mut() else { return };
        let now = Instant::now();
        let dt = now.duration_since(live.last).as_secs_f32().min(0.1);
        live.last = now;
        let t = if anim { now.duration_since(live.t0).as_secs_f32() } else { 0.0 };
        let sky = u.global::<Sky>();
        let dark = sky.get_night();
        let g = u.global::<Lagoon>();
        // ---------------- Lumière.
        let p = config::ui_prefs();
        let m = crate::daynight::scene_minute();
        let w = water(m, p.day_at as f32, p.night_at as f32, p.daynight_span as f32);
        g.set_w0(color(w[0], 1.0));
        g.set_w1(color(w[1], 1.0));
        g.set_w2(color(w[2], 1.0));
        g.set_w3(color(w[3], 1.0));
        g.set_w4(color(w[4], 1.0));
        let dawnish = sky.get_dawn() || m < p.day_at as f32 + 300.0 && m > p.day_at as f32 - 200.0;
        let mut light = 0.0;
        if sky.get_sun_on() {
            let (su, sh) = (sky.get_sun_u(), sky.get_sun_h());
            let low = 1.0 - sh;
            let sc = mix([235.0, 252.0, 255.0], if dawnish { [255.0, 160.0, 175.0] } else { [255.0, 130.0, 60.0] }, smooth((low - 0.35) / 0.55));
            let sx = lerp(W + 260.0, -260.0, su);
            g.set_sun_x(sx);
            g.set_sun_scale(lerp(0.55, 1.0, sh));
            g.set_sun_color(color(sc, 1.0));
            g.set_sun_a(0.85);
            g.set_ray_x(sx + if anim { (t * 0.2).sin() * 12.0 } else { 0.0 });
            g.set_ray_angle(-(((W / 2.0 - sx) / W * 1.1).atan()).to_degrees());
            g.set_ray_len(lerp(140.0, 900.0 + 260.0, sh.powf(0.6)));
            g.set_ray_color(color(mix([215.0, 245.0, 255.0], sc, 0.7), 1.0));
            g.set_ray_a((if anim { 0.82 + 0.18 * (t * 0.35).sin() } else { 0.9 }) * lerp(0.55, 1.0, sh));
            light += sh.sqrt();
        } else {
            g.set_sun_a(0.0);
            g.set_ray_a(0.0);
        }
        if sky.get_moon_on() {
            let (mu, mh) = (sky.get_moon_u(), sky.get_moon_h());
            let mx = lerp(W + 260.0, -260.0, mu);
            g.set_moon_x(mx);
            g.set_moon_a(0.6);
            g.set_moon_ray_angle(-(((W / 2.0 - mx) / W * 1.1).atan()).to_degrees());
            g.set_moon_ray_len(lerp(80.0, 700.0, mh.powf(0.7)));
            light += mh * 0.2;
        } else {
            g.set_moon_a(0.0);
        }
        g.set_caustic_a(0.55 * light.clamp(0.0, 1.0));
        g.set_aurora(sky.get_aurora());
        // ---------------- Banc de poissons (le soir, il n'en part plus ; le dernier finit sa traversée).
        let lap = (t / 40.0).floor() as i64;
        if !anim {
            live.fish_on = dark < 0.5;
        } else if lap != live.fish_lap {
            live.fish_lap = lap;
            live.fish_on = dark < 0.5;
        }
        g.set_school(if live.fish_on { if anim { (t % 40.0) / 40.0 } else { 0.55 } } else { -1.0 });
        // ---------------- Méduses.
        for (i, j) in live.jellies.iter_mut().enumerate() {
            swim(j, dark, dt, t, snap || !anim);
            let row = if j.state == State::Out { LagoonJelly::default() } else { jelly_paths(j, t, anim) };
            live.jelly_model.set_row_data(i, row);
        }
        // ---------------- Anémones, poissons-clowns, demoiselles.
        let mut tips = Vec::new();
        for (i, a) in live.anemones.iter().enumerate() {
            live.anem_model.set_row_data(i, anemone_paths(a, t, anim, &mut tips));
        }
        let mut fish = Vec::new();
        for a in live.anemones.iter().filter(|a| a.fish) {
            for f in 0..2 {
                let ph = a.ph + f as f32 * 2.4;
                let rad = (36.0 + f as f32 * 10.0) * a.s * (1.0 - dark * 0.85);
                let fx = a.x + if anim { (t * 0.8 + ph).sin() } else { 0.5 } * rad;
                let fy = a.y - (40.0 + f as f32 * 8.0) * a.s * (1.0 - dark * 0.6) + if anim { (t * 1.3 + ph).sin() * 8.0 } else { 0.0 };
                let right = !anim || (t * 0.8 + ph).cos() > 0.0;
                fish.push(LagoonFish { x: fx, y: fy, s: 0.9 * a.s, right, clown: true });
            }
        }
        let (scx, scy) = (1420.0 + if anim { (t * 0.3).sin() * 50.0 } else { 0.0 }, 690.0 + if anim { (t * 0.21).sin() * 18.0 } else { 0.0 } + dark * 170.0);
        let right = anim && (t * 0.3).cos() > 0.0;
        for c in &live.chromis {
            let fx = scx + c.dx + if anim { (t * 1.3 + c.ph).sin() * 6.0 } else { 0.0 };
            let fy = scy + c.dy + if anim { (t * 1.7 + c.ph).sin() * 4.0 } else { 0.0 };
            fish.push(LagoonFish { x: fx, y: fy, s: c.s, right, clown: false });
        }
        for (i, f) in fish.into_iter().enumerate() {
            if i < live.fish_model.row_count() {
                live.fish_model.set_row_data(i, f);
            }
        }
        for (i, d) in tips.into_iter().enumerate() {
            if i < live.tips.row_count() {
                live.tips.set_row_data(i, d);
            }
        }
    });
}

/// Démarre le décor Aero : données posées une fois ; 30 images/s quand il est affiché et animé, une
/// toutes les 2 s sinon (la lumière suit quand même l'heure). Garder les minuteries rendues.
pub fn start(u: &AppWindow) -> Vec<slint::Timer> {
    let Ok(data) = serde_json::from_str::<Data>(include_str!("../ui/lagoon/lagoon.json")) else { return Vec::new() };
    let jellies: Vec<Jelly> = data.jellies.iter().map(|d| Jelly { d: d.clone(), state: State::Out, px: d.x, py: d.y, vx: 0.0, vy: 0.0, ang: 0.0, c: 0.0, cp: 0.0, hist: Vec::new() }).collect();
    let jelly_model = Rc::new(VecModel::from(vec![LagoonJelly::default(); jellies.len()]));
    let anem_model = Rc::new(VecModel::from(vec![LagoonAnemone::default(); data.anemones.len()]));
    let n_fish = data.anemones.iter().filter(|a| a.fish).count() * 2 + data.chromis.len();
    let fish_model = Rc::new(VecModel::from(vec![LagoonFish::default(); n_fish]));
    let tips = Rc::new(VecModel::from(vec![LagoonDot::default(); data.anemones.len() * 36]));
    let g = u.global::<Lagoon>();
    g.set_plankton(ModelRc::new(VecModel::from(data.plankton.iter().map(|p| LagoonSpeck { x: p.x, y: p.y, r: p.r, a: p.a, sp: p.sp, ph: p.ph, lum: p.lum, th: p.th }).collect::<Vec<_>>())));
    g.set_bubbles(ModelRc::new(VecModel::from(data.bub.iter().map(|b| LagoonBubble { x: data.cols.get(b.col).map(|c| c[0]).unwrap_or(0.0), off: b.off, r: b.r, sp: b.sp, ph: b.ph }).collect::<Vec<_>>())));
    g.set_school_fish(ModelRc::new(VecModel::from(data.fish.iter().map(|f| LagoonDot { x: f.dx, y: f.dy, a: f.ph, r: f.s }).collect::<Vec<_>>())));
    g.set_fans(ModelRc::new(VecModel::from(data.fans.iter().map(|f| LagoonDot { x: f.x, y: f.y, a: f.ph, r: f.w }).collect::<Vec<_>>())));
    g.set_jellies(ModelRc::from(jelly_model.clone()));
    g.set_anemones(ModelRc::from(anem_model.clone()));
    g.set_fish(ModelRc::from(fish_model.clone()));
    g.set_tips(ModelRc::from(tips.clone()));
    LIVE.with(|l| {
        *l.borrow_mut() = Some(Live { jellies, anemones: data.anemones, chromis: data.chromis, jelly_model, anem_model, fish_model, tips, t0: Instant::now(), last: Instant::now(), fish_lap: -1, fish_on: true })
    });
    frame(u, false, true);

    let fast = Rc::new(slint::Timer::default());
    let check = slint::Timer::default();
    let w = u.as_weak();
    let fast2 = fast.clone();
    let mut was_anim = false;
    let mut slow = 0u32;
    let mut loaded = false;
    check.start(slint::TimerMode::Repeated, Duration::from_millis(500), move || {
        let Some(u) = w.upgrade() else { return };
        let on = shown(&u);
        if on && !loaded {
            loaded = true;
            load_images(u.as_weak());
        }
        let anim = on && config::ui_prefs().anim.scenery;
        if anim && !fast2.running() {
            // Reprise : les méduses sont posées à leur place (pas de traversée de rattrapage).
            frame(&u, true, !was_anim);
            let w2 = u.as_weak();
            fast2.start(slint::TimerMode::Repeated, Duration::from_millis(33), move || {
                if let Some(u) = w2.upgrade() {
                    frame(&u, true, false);
                }
            });
        } else if !anim && fast2.running() {
            fast2.stop();
        }
        if on && !anim {
            slow += 1;
            if slow % 4 == 0 {
                frame(&u, false, true);
            }
        }
        was_anim = anim;
    });
    let _ = fast;
    vec![check]
}
