//! Décor du thème Nerd : le schéma d'une carte mère, en vert phosphore sur noir (vue d'un logiciel de
//! conception électronique). Le jour, le système tourne : des signaux filent sur les pistes et
//! l'oscilloscope affiche un signal rapide. La nuit, il passe en veille (ambre) : les signaux se
//! raréfient et ralentissent, l'oscilloscope n'affiche plus qu'une lente sinusoïde. À l'aube, il
//! redémarre (vague verte + journal de démarrage dans la fenêtre tty0). La fenêtre tty0 affiche les
//! vraies infos : serveur Jellyfin (nom, version, connexion, temps de réponse) et Turtlefin (version,
//! mémoire, cache, temps depuis le lancement).
//!
//! Le schéma (repère 1600 x 900) est calculé une fois et passé à Slint en tracés (global `Pcb`,
//! sky.slint) : le processeur au centre, des bus de pistes parallèles qui ne se croisent pas, une
//! broche sans voisin descend vers un via. Ce qui bouge (signaux, oscilloscope) est recalculé 30 fois
//! par seconde, seulement quand le décor Nerd est affiché et animé.

use crate::{config, App, AppWindow, Pcb, PcbBox, PcbDot, PcbLabel, Sky, Theme, TtyRow};
use crate::i18n::tr;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
struct Pin {
    x: f32,
    y: f32,
    nx: f32,
    ny: f32,
}

struct Chip {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &'static str,
    kind: Kind,
    /// Broches par côté (haut, bas, gauche, droite) et si elles sont reliées.
    pins: [Vec<(Pin, bool)>; 4],
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Cpu,
    Ic,
    Slot,
    Conn,
    Ind,
    Fet,
}

const T: usize = 0;
const B: usize = 1;
const L: usize = 2;
const R: usize = 3;

struct Trace {
    pts: Vec<(f32, f32)>,
    len: f32,
    thick: bool,
}

/// Schéma calculé une fois.
struct Layout {
    traces: Vec<Trace>,
    thin: String,
    thick: String,
    vias: String,
    outline: String,
    fill: String,
    grid: String,
    labels: Vec<PcbLabel>,
}

fn chip(label: &'static str, x: f32, y: f32, w: f32, h: f32, n: [usize; 4], kind: Kind) -> Chip {
    let mut pins: [Vec<(Pin, bool)>; 4] = Default::default();
    for (side, &count) in n.iter().enumerate() {
        for i in 0..count {
            let f = (i + 1) as f32 / (count + 1) as f32;
            let p = match side {
                T => Pin { x: x + w * f, y: y - 5.0, nx: 0.0, ny: -1.0 },
                B => Pin { x: x + w * f, y: y + h + 5.0, nx: 0.0, ny: 1.0 },
                L => Pin { x: x - 5.0, y: y + h * f, nx: -1.0, ny: 0.0 },
                _ => Pin { x: x + w + 5.0, y: y + h * f, nx: 1.0, ny: 0.0 },
            };
            pins[side].push((p, false));
        }
    }
    Chip { x, y, w, h, label, kind, pins }
}

/// Coins à 45° (chanfrein de 10 au plus).
fn chamfer(pts: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut out = vec![pts[0]];
    for i in 1..pts.len() - 1 {
        let (a, b, c) = (pts[i - 1], pts[i], pts[i + 1]);
        let l1 = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let l2 = ((c.0 - b.0).powi(2) + (c.1 - b.1).powi(2)).sqrt();
        let k = 10f32.min(l1 / 2.0).min(l2 / 2.0);
        if k < 1.0 {
            out.push(b);
            continue;
        }
        out.push((b.0 + (a.0 - b.0) / l1 * k, b.1 + (a.1 - b.1) / l1 * k));
        out.push((b.0 + (c.0 - b.0) / l2 * k, b.1 + (c.1 - b.1) / l2 * k));
    }
    out.push(pts[pts.len() - 1]);
    out
}

struct Router {
    traces: Vec<Trace>,
    vias: Vec<(f32, f32)>,
}

impl Router {
    fn add(&mut self, pts: &[(f32, f32)], thick: bool) {
        let pts = chamfer(pts);
        let len = pts.windows(2).map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt()).sum();
        self.traces.push(Trace { pts, len, thick });
    }

    /// Bus : on sort de la broche de départ, on tourne sur une « voie » (les pistes qui vont le plus loin
    /// tournent les premières : elles ne se croisent jamais), on rejoint l'axe de la broche d'arrivée.
    fn bus(&mut self, pairs: &[(Pin, Pin)], thick: bool, gap: f32) {
        if pairs.is_empty() {
            return;
        }
        let vert = pairs[0].0.ny != 0.0;
        let delta = |p: &(Pin, Pin)| if vert { p.1.x - p.0.x } else { p.1.y - p.0.y };
        let mut groups: [Vec<(Pin, Pin)>; 3] = Default::default();
        for p in pairs {
            let d = delta(p);
            groups[if d > 2.0 { 2 } else if d < -2.0 { 0 } else { 1 }].push(*p);
        }
        for g in groups.iter_mut() {
            g.sort_by(|a, b| delta(b).abs().partial_cmp(&delta(a).abs()).unwrap_or(std::cmp::Ordering::Equal));
            for (i, (p, q)) in g.iter().enumerate() {
                let off = 16.0 + i as f32 * gap;
                if vert {
                    let lane = p.y + p.ny * off;
                    self.add(&[(p.x, p.y), (p.x, lane), (q.x, lane), (q.x, q.y)], thick);
                } else {
                    let lane = p.x + p.nx * off;
                    self.add(&[(p.x, p.y), (lane, p.y), (lane, q.y), (q.x, q.y)], thick);
                }
            }
        }
    }
}

fn layout() -> Layout {
    let mut chips: Vec<Chip> = Vec::new();
    let mut add = |c: Chip| {
        chips.push(c);
        chips.len() - 1
    };
    let cpu = add(chip("U1  TF-1900", 680.0, 330.0, 240.0, 240.0, [14, 14, 14, 14], Kind::Cpu));
    let ddr: Vec<usize> = (0..4).map(|i| add(chip("DDR5", 585.0 + i as f32 * 110.0, 120.0, 90.0, 44.0, [0, 3, 0, 0], Kind::Ic))).collect();
    let pch = add(chip("U6  PCH", 290.0, 380.0, 150.0, 150.0, [6, 8, 8, 10], Kind::Ic));
    let bios = add(chip("BIOS", 150.0, 250.0, 64.0, 40.0, [0, 0, 4, 4], Kind::Ic));
    let phy = add(chip("LAN", 320.0, 170.0, 90.0, 66.0, [0, 6, 4, 0], Kind::Ic));
    let codec = add(chip("AUDIO", 320.0, 600.0, 90.0, 66.0, [6, 0, 3, 0], Kind::Ic));
    let sata = add(chip("SATA", 110.0, 470.0, 54.0, 76.0, [0, 0, 0, 4], Kind::Conn));
    let usb = add(chip("USB", 1130.0, 150.0, 80.0, 60.0, [0, 0, 2, 4], Kind::Ic));
    let pcie = add(chip("PCIE x16", 560.0, 660.0, 480.0, 34.0, [14, 0, 0, 0], Kind::Slot));
    let m2 = add(chip("M.2", 1180.0, 625.0, 250.0, 44.0, [4, 0, 0, 0], Kind::Slot));
    let ind: Vec<usize> = (0..5).map(|i| add(chip("R47", 1010.0, 340.0 + i as f32 * 46.0, 36.0, 36.0, [0, 0, 2, 1], Kind::Ind))).collect();
    let fet: Vec<usize> = (0..5).map(|i| add(chip("", 1090.0, 346.0 + i as f32 * 46.0, 24.0, 24.0, [0, 0, 1, 1], Kind::Fet))).collect();
    let atx = add(chip("ATX 24", 1470.0, 300.0, 64.0, 240.0, [0, 0, 10, 0], Kind::Conn));
    let fan = add(chip("FAN", 1460.0, 590.0, 60.0, 22.0, [0, 0, 0, 0], Kind::Conn));
    let _ = fan;

    // Broches libres (hors puces) : prise RJ45, prises jack, ports USB, quartz.
    let rj_pins: Vec<Pin> = (0..4).map(|i| Pin { x: 135.0, y: 170.0 + i as f32 * 20.0, nx: 1.0, ny: 0.0 }).collect();
    let jack_pins: Vec<Pin> = (0..3).map(|i| Pin { x: 95.0, y: 595.0 + i as f32 * 42.0, nx: 1.0, ny: 0.0 }).collect();
    let usb_pins: Vec<Pin> = (0..4).map(|i| Pin { x: 1285.0, y: 145.0 + (i / 2) as f32 * 70.0 + (i % 2) as f32 * 16.0, nx: -1.0, ny: 0.0 }).collect();
    let xtal_pins = [Pin { x: 484.0, y: 586.0, nx: 0.0, ny: -1.0 }, Pin { x: 512.0, y: 586.0, nx: 0.0, ny: -1.0 }];

    let mut r = Router { traces: Vec::new(), vias: Vec::new() };
    // Prend les broches (marquées reliées).
    fn take(chips: &mut [Chip], c: usize, side: usize, from: usize, to: usize) -> Vec<Pin> {
        let to = to.min(chips[c].pins[side].len());
        (from..to)
            .map(|i| {
                chips[c].pins[side][i].1 = true;
                chips[c].pins[side][i].0
            })
            .collect()
    }
    fn zip(a: Vec<Pin>, b: Vec<Pin>) -> Vec<(Pin, Pin)> {
        a.into_iter().zip(b).collect()
    }
    let mut ddr_pins = Vec::new();
    for &d in &ddr {
        ddr_pins.extend(take(&mut chips, d, B, 0, 3));
    }
    let p = take(&mut chips, cpu, T, 0, 12);
    r.bus(&zip(p, ddr_pins), false, 9.0);
    let (a, b) = (take(&mut chips, cpu, L, 2, 12), take(&mut chips, pch, R, 0, 10));
    r.bus(&zip(a, b), false, 9.0);
    let (a, b) = (take(&mut chips, cpu, B, 0, 10), take(&mut chips, pcie, T, 0, 10));
    r.bus(&zip(a, b), false, 6.0);
    let (a, b) = (take(&mut chips, cpu, B, 10, 14), take(&mut chips, m2, T, 0, 4));
    r.bus(&zip(a, b), false, 6.0);
    let cpu_r = take(&mut chips, cpu, R, 0, 10);
    let mut vrm = Vec::new();
    for (i, p) in cpu_r.into_iter().enumerate() {
        vrm.push((p, take(&mut chips, ind[i / 2], L, i % 2, i % 2 + 1)[0]));
    }
    r.bus(&vrm, true, 7.0);
    for i in 0..5 {
        let (a, b) = (take(&mut chips, ind[i], R, 0, 1), take(&mut chips, fet[i], L, 0, 1));
        r.bus(&zip(a, b), true, 9.0);
    }
    let mut feed = Vec::new();
    for (i, &f) in fet.iter().enumerate() {
        feed.push((take(&mut chips, f, R, 0, 1)[0], take(&mut chips, atx, L, i * 2, i * 2 + 1)[0]));
    }
    r.bus(&feed, true, 10.0);
    let (a, b) = (take(&mut chips, pch, L, 0, 4), take(&mut chips, bios, R, 0, 4));
    r.bus(&zip(a, b), false, 9.0);
    let (a, b) = (take(&mut chips, pch, L, 4, 8), take(&mut chips, sata, R, 0, 4));
    r.bus(&zip(a, b), false, 9.0);
    let (a, b) = (take(&mut chips, pch, T, 0, 6), take(&mut chips, phy, B, 0, 6));
    r.bus(&zip(a, b), false, 9.0);
    let a = take(&mut chips, phy, L, 0, 4);
    r.bus(&zip(a, rj_pins), false, 9.0);
    let (a, b) = (take(&mut chips, pch, B, 0, 6), take(&mut chips, codec, T, 0, 6));
    r.bus(&zip(a, b), false, 9.0);
    let a = take(&mut chips, pch, B, 6, 8);
    r.bus(&zip(a, xtal_pins.to_vec()), false, 9.0);
    let a = take(&mut chips, codec, L, 0, 3);
    r.bus(&zip(a, jack_pins), false, 9.0);
    let a = take(&mut chips, usb, R, 0, 4);
    r.bus(&zip(a, usb_pins), false, 9.0);
    // Broches restantes : un court départ vers un via.
    for c in &chips {
        for side in &c.pins {
            for (i, (p, used)) in side.iter().enumerate() {
                if *used {
                    continue;
                }
                let l = 14.0 + (i % 2) as f32 * 8.0;
                let v = (p.x + p.nx * l, p.y + p.ny * l);
                r.add(&[(p.x, p.y), v], false);
                r.vias.push(v);
            }
        }
    }
    // Condensateurs de découplage (dans les espaces libres), pile, LED, ventilateur : reliés à des vias.
    let caps = [(560.0, 210.0), (1050.0, 230.0), (450.0, 330.0), (180.0, 380.0), (180.0, 650.0), (480.0, 690.0), (1100.0, 650.0), (1080.0, 300.0), (1300.0, 296.0), (560.0, 570.0)];
    for &(x, y) in &caps {
        r.add(&[(x - 2.0, y + 4.0), (x - 12.0, y + 4.0)], false);
        r.add(&[(x + 16.0, y + 4.0), (x + 26.0, y + 4.0)], false);
        r.vias.push((x - 12.0, y + 4.0));
        r.vias.push((x + 26.0, y + 4.0));
    }
    r.add(&[(520.0, 306.0), (520.0, 326.0)], false);
    r.vias.push((520.0, 326.0));
    for i in 0..3 {
        let y = 135.0 + i as f32 * 30.0;
        r.add(&[(1464.0, y), (1440.0, y)], false);
        r.vias.push((1440.0, y));
        let x = 1470.0 + i as f32 * 20.0;
        r.add(&[(x, 590.0), (x, 572.0)], false);
        r.vias.push((x, 572.0));
    }

    // ---------------- Tracés.
    let mut thin = String::new();
    let mut thick = String::new();
    for t in &r.traces {
        let s = if t.thick { &mut thick } else { &mut thin };
        for (i, p) in t.pts.iter().enumerate() {
            let _ = write!(s, "{} {:.1} {:.1} ", if i == 0 { "M" } else { "L" }, p.0, p.1);
        }
    }
    let mut vias = String::new();
    for v in &r.vias {
        circle(&mut vias, v.0, v.1, 4.0);
    }
    let mut outline = String::new();
    let mut fill = String::new();
    let mut labels: Vec<PcbLabel> = Vec::new();
    let mut label = |x: f32, y: f32, t: &str, size: f32| labels.push(PcbLabel { x, y, text: t.into(), size });
    for c in &chips {
        for (s, side) in c.pins.iter().enumerate() {
            for (p, _) in side {
                if s == T || s == B {
                    rect(&mut outline, p.x - 2.5, p.y - 3.5, 5.0, 7.0);
                } else {
                    rect(&mut outline, p.x - 3.5, p.y - 2.5, 7.0, 5.0);
                }
            }
        }
        match c.kind {
            Kind::Cpu => {
                rect(&mut outline, c.x, c.y, c.w, c.h);
                rect(&mut outline, c.x + 28.0, c.y + 28.0, c.w - 56.0, c.h - 56.0);
                for f in 0..9 {
                    let y = c.y + 44.0 + f as f32 * 18.0;
                    let _ = write!(outline, "M {:.1} {:.1} L {:.1} {:.1} ", c.x + 36.0, y, c.x + c.w - 36.0, y);
                }
                circle(&mut fill, c.x + 12.0, c.y + 12.0, 4.0);
                dashed(&mut outline, c.x - 14.0, c.y - 14.0, c.w + 28.0, c.h + 28.0);
                label(c.x + 34.0, c.y + c.h - 24.0, c.label, 16.0);
            }
            Kind::Ic => {
                rect(&mut outline, c.x, c.y, c.w, c.h);
                circle(&mut fill, c.x + 7.0, c.y + 7.0, 2.5);
                label(c.x + 6.0, c.y + c.h / 2.0 - 7.0, c.label, 11.0);
            }
            Kind::Slot => {
                rect(&mut outline, c.x, c.y, c.w, c.h);
                rect(&mut outline, c.x + 6.0, c.y + c.h / 2.0 - 3.0, c.w - 12.0, 6.0);
                let mut t = c.x + 10.0;
                while t < c.x + c.w - 8.0 {
                    let _ = write!(outline, "M {:.1} {:.1} L {:.1} {:.1} ", t, c.y + 4.0, t, c.y + c.h / 2.0 - 5.0);
                    t += 7.0;
                }
                label(c.x + 8.0, c.y + c.h + 4.0, c.label, 11.0);
            }
            Kind::Conn => {
                rect(&mut outline, c.x, c.y, c.w, c.h);
                label(c.x, c.y - 18.0, c.label, 11.0);
                if c.label == "FAN" {
                    for k in 0..3 {
                        circle(&mut outline, c.x + 10.0 + k as f32 * 20.0, c.y + 11.0, 4.0);
                    }
                } else {
                    let cols = if c.w > 50.0 { 2 } else { 1 };
                    let rows = (c.h / 20.0).floor().max(1.0) as usize;
                    for rr in 0..rows {
                        for q in 0..cols {
                            circle(&mut outline, c.x + (q as f32 + 0.5) * c.w / cols as f32, c.y + (rr as f32 + 0.5) * c.h / rows as f32, 4.0);
                        }
                    }
                }
            }
            Kind::Ind => {
                rect(&mut outline, c.x, c.y, c.w, c.h);
                circle(&mut outline, c.x + c.w / 2.0, c.y + c.h / 2.0, c.w * 0.32);
                label(c.x + 7.0, c.y - 14.0, c.label, 9.0);
            }
            Kind::Fet => {
                rect(&mut outline, c.x, c.y, c.w, c.h);
                rect(&mut fill, c.x + c.w - 6.0, c.y + 3.0, 4.0, c.h - 6.0);
            }
        }
    }
    // RJ45, prises jack, ports USB, quartz, pile, condensateurs, LED.
    rect(&mut outline, 40.0, 150.0, 90.0, 100.0);
    let _ = write!(outline, "M 60 170 L 110 170 L 110 210 L 95 210 L 95 224 L 75 224 L 75 210 L 60 210 Z ");
    label(40.0, 132.0, "RJ45", 11.0);
    for i in 0..3 {
        let y = 595.0 + i as f32 * 42.0;
        circle(&mut outline, 75.0, y, 14.0);
        circle(&mut outline, 75.0, y, 5.0);
    }
    for i in 0..2 {
        let y = 130.0 + i as f32 * 70.0;
        rect(&mut outline, 1290.0, y, 70.0, 46.0);
        rect(&mut outline, 1300.0, y + 14.0, 50.0, 18.0);
        label(1294.0, y - 16.0, "USB", 10.0);
    }
    ellipse(&mut outline, 498.0, 601.0, 28.0, 11.0);
    label(470.0, 574.0, "Y1 25MHz", 10.0);
    circle(&mut outline, 520.0, 280.0, 26.0);
    circle(&mut outline, 520.0, 280.0, 20.0);
    label(498.0, 273.0, "CR2032", 10.0);
    for &(x, y) in &caps {
        rect(&mut outline, x, y, 14.0, 8.0);
    }
    for i in 0..3 {
        let y = 130.0 + i as f32 * 30.0;
        rect(&mut outline, 1480.0, y, 16.0, 10.0);
        let _ = write!(outline, "M 1484 {:.1} L 1492 {:.1} L 1484 {:.1} Z ", y + 2.0, y + 5.0, y + 8.0);
    }
    label(66.0, 72.0, "layer: top · scale 1:1 · drc ok", 12.0);
    // Grille de points (papier du logiciel de conception).
    let mut grid = String::new();
    let mut gx = 0.0;
    while gx < 1600.0 {
        let mut gy = 0.0;
        while gy < 900.0 {
            let _ = write!(grid, "M {gx} {gy} L {} {gy} ", gx + 1.0);
            gy += 20.0;
        }
        gx += 20.0;
    }
    Layout { traces: r.traces, thin, thick, vias, outline, fill, grid, labels }
}

fn rect(s: &mut String, x: f32, y: f32, w: f32, h: f32) {
    let _ = write!(s, "M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} Z ", x, y, x + w, y, x + w, y + h, x, y + h);
}
fn circle(s: &mut String, x: f32, y: f32, r: f32) {
    ellipse(s, x, y, r, r);
}
fn ellipse(s: &mut String, x: f32, y: f32, rx: f32, ry: f32) {
    let _ = write!(s, "M {:.1} {:.1} A {rx} {ry} 0 1 0 {:.1} {:.1} A {rx} {ry} 0 1 0 {:.1} {:.1} Z ", x - rx, y, x + rx, y, x - rx, y);
}
/// Contour en pointillés (zone réservée autour du processeur).
fn dashed(s: &mut String, x: f32, y: f32, w: f32, h: f32) {
    let edges = [((x, y), (x + w, y)), ((x + w, y), (x + w, y + h)), ((x + w, y + h), (x, y + h)), ((x, y + h), (x, y))];
    for ((x0, y0), (x1, y1)) in edges {
        let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let mut t = 0.0;
        while t < len {
            let t1 = (t + 4.0).min(len);
            let _ = write!(s, "M {:.1} {:.1} L {:.1} {:.1} ", x0 + (x1 - x0) * t / len, y0 + (y1 - y0) * t / len, x0 + (x1 - x0) * t1 / len, y0 + (y1 - y0) * t1 / len);
            t += 8.0;
        }
    }
}

fn point_on(t: &Trace, f: f32) -> (f32, f32) {
    let target = f.clamp(0.0, 1.0) * t.len;
    let mut acc = 0.0;
    for w in t.pts.windows(2) {
        let l = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
        if acc + l >= target {
            let u = if l > 0.0 { (target - acc) / l } else { 0.0 };
            return (w[0].0 + (w[1].0 - w[0].0) * u, w[0].1 + (w[1].1 - w[0].1) * u);
        }
        acc += l;
    }
    *t.pts.last().unwrap_or(&(0.0, 0.0))
}

struct Pulse {
    trace: usize,
    f: f32,
    speed: f32,
    alive: bool,
}

struct Live {
    layout: Layout,
    pulses: Vec<Pulse>,
    dots: Rc<VecModel<PcbDot>>,
    seed: u32,
    phase: f32,
    last: Instant,
}

impl Live {
    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed % 100_000) as f32 / 100_000.0
    }
}

const TRAIL: usize = 6;

thread_local! {
    static LIVE: RefCell<Option<Live>> = const { RefCell::new(None) };
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Une image : signaux et oscilloscope.
fn frame(u: &AppWindow) {
    LIVE.with(|l| {
        let mut l = l.borrow_mut();
        let Some(live) = l.as_mut() else { return };
        let now = Instant::now();
        let dt = now.duration_since(live.last).as_secs_f32().min(0.1);
        live.last = now;
        let sleep = smooth((u.global::<Sky>().get_night() - 0.2) / 0.6);
        let active = 1.0 - sleep;
        let n = live.pulses.len();
        let count = live.layout.traces.len();
        for i in 0..n {
            let len = live.layout.traces[live.pulses[i].trace].len.max(1.0);
            let p = &mut live.pulses[i];
            p.f += dt * p.speed * (0.35 + 0.65 * active) / len;
            if p.f >= 1.0 {
                p.f -= 1.0;
                // Un signal finit son trajet ; le soir, il n'en repart plus (pas de disparition en route).
                let alive = (i as f32) / (n as f32) < active;
                let next = (live.seed as usize).wrapping_mul(2654435761).wrapping_add(i * 97) % count;
                let p = &mut live.pulses[i];
                p.alive = alive;
                p.trace = next;
                live.rand();
            }
        }
        for (i, p) in live.pulses.iter().enumerate() {
            let t = &live.layout.traces[p.trace];
            for k in 0..TRAIL {
                let (x, y) = point_on(t, p.f - k as f32 * 8.0 / t.len.max(1.0));
                let a = if p.alive { (1.0 - k as f32 / TRAIL as f32) * 0.9 * (p.f * 8.0).clamp(0.0, 1.0) } else { 0.0 };
                live.dots.set_row_data(i * TRAIL + k, PcbDot { x, y, a, r: 7.0 - k as f32 });
            }
        }
        // Oscilloscope : carré rapide (en marche) qui s'arrondit en sinusoïde lente (veille).
        live.phase += dt * (2.2 + (0.25 - 2.2) * sleep);
        let cyc = 8.0 + (1.5 - 8.0) * sleep;
        let kk = 10.0 + (0.7 - 10.0) * sleep;
        let norm = kk.tanh();
        let mut s = String::with_capacity(4096);
        let mut x = 0.0;
        while x <= 360.0 {
            let y = (kk * (2.0 * std::f32::consts::PI * (x / 360.0 * cyc - live.phase)).sin()).tanh() / norm;
            let _ = write!(s, "{} {:.1} {:.1} ", if x == 0.0 { "M" } else { "L" }, x, 46.0 - y * 33.0);
            x += 2.0;
        }
        let b = u.global::<Pcb>();
        b.set_scope(s.into());
        let hz = 48.0 + (0.2 - 48.0) * sleep;
        b.set_scope_label(if sleep > 0.5 { format!("CH1 · {hz:.1} Hz · {}", tr("veille")) } else { format!("CH1 · {hz:.1} MHz · {}", tr("actif")) }.into());
    });
}

/// Le décor Nerd est-il à l'écran et animé ?
fn visible(u: &AppWindow) -> bool {
    u.global::<Theme>().get_scenery() == 3 && config::ui_prefs().anim.scenery && !u.get_playing()
}

static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
/// Infos du serveur : (nom, version, temps de réponse en ms), None si injoignable.
static SERVER: Mutex<Option<(String, String, u32)>> = Mutex::new(None);

/// Mémoire utilisée par Turtlefin (octets).
fn memory_used() -> Option<u64> {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct Counters {
            cb: u32,
            page_fault_count: u32,
            peak_working_set: usize,
            working_set: usize,
            quota_peak_paged: usize,
            quota_paged: usize,
            quota_peak_non_paged: usize,
            quota_non_paged: usize,
            pagefile: usize,
            peak_pagefile: usize,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentProcess() -> isize;
            fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, cb: u32) -> i32;
        }
        let mut c: Counters = unsafe { std::mem::zeroed() };
        c.cb = std::mem::size_of::<Counters>() as u32;
        // Lecture des compteurs du processus courant (pas d'autre effet).
        let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
        (ok != 0).then_some(c.working_set as u64)
    }
    #[cfg(not(windows))]
    {
        let s = std::fs::read_to_string("/proc/self/status").ok()?;
        let kb: u64 = s.lines().find(|l| l.starts_with("VmRSS:"))?.split_whitespace().nth(1)?.parse().ok()?;
        Some(kb * 1024)
    }
}

fn size_label(b: u64) -> String {
    let mo = b as f64 / 1_048_576.0;
    if mo >= 1024.0 {
        format!("{:.1} Go", mo / 1024.0).replace('.', ",")
    } else {
        format!("{} Mo", mo.round() as u64)
    }
}

/// Lignes de la fenêtre tty0 (vraies infos).
fn tty_rows(cache: u64) -> Vec<TtyRow> {
    let row = |k: &str, v: String| TtyRow { k: k.into(), v: v.into() };
    let mut rows = Vec::new();
    match SERVER.lock().unwrap().clone() {
        Some((name, version, ms)) => {
            rows.push(row(&tr("serveur"), format!("{name} · Jellyfin {version}")));
            rows.push(row(&tr("connexion"), format!("{} · {ms} ms", tr("en ligne"))));
        }
        None => {
            rows.push(row(&tr("serveur"), "—".into()));
            rows.push(row(&tr("connexion"), tr("hors ligne").to_string()));
        }
    }
    rows.push(row("", String::new()));
    rows.push(row("turtlefin", env!("CARGO_PKG_VERSION").into()));
    rows.push(row(&tr("mémoire"), memory_used().map(size_label).unwrap_or_else(|| "—".into())));
    rows.push(row(&tr("cache"), size_label(cache)));
    let up = START.get().map(|s| s.elapsed().as_secs()).unwrap_or(0);
    rows.push(row(&tr("lancé depuis"), format!("{} h {:02} min", up / 3600, up / 60 % 60)));
    rows
}

/// Demande au serveur son nom et sa version, et mesure le temps de réponse.
fn probe(app: &Arc<App>) {
    let Some(client) = app.client() else {
        *SERVER.lock().unwrap() = None;
        return;
    };
    let base = client.server.clone();
    app.rt.spawn(async move {
        let http = reqwest::Client::new();
        let t0 = Instant::now();
        let got = crate::discovery::probe(&http, &base).await;
        *SERVER.lock().unwrap() = got.map(|(_, name, version)| (name, version, t0.elapsed().as_millis() as u32));
    });
}

/// Démarre le décor Nerd : schéma posé une fois ; signaux et oscilloscope quand il est affiché ;
/// tty0 toutes les 2 s, serveur interrogé toutes les 30 s. Garder les minuteries rendues.
pub fn start(app: &Arc<App>, u: &AppWindow) -> Vec<slint::Timer> {
    let _ = START.set(Instant::now());
    let layout = layout();
    let b = u.global::<Pcb>();
    b.set_thin(layout.thin.clone().into());
    b.set_thick(layout.thick.clone().into());
    b.set_vias(layout.vias.clone().into());
    b.set_outline(layout.outline.clone().into());
    b.set_fill(layout.fill.clone().into());
    b.set_grid(layout.grid.clone().into());
    b.set_labels(ModelRc::new(VecModel::from(layout.labels.clone())));
    // Démarrage : langue (BIOS), affichage (PCIe), lecteur vidéo (audio), stockage (M.2), configuration
    // (SATA), réseau (LAN), serveur (processeur).
    let boxes = [(150.0, 250.0, 64.0, 40.0), (560.0, 660.0, 480.0, 34.0), (320.0, 600.0, 90.0, 66.0), (1180.0, 625.0, 250.0, 44.0), (110.0, 470.0, 54.0, 76.0), (320.0, 170.0, 90.0, 66.0), (680.0, 330.0, 240.0, 240.0)];
    b.set_checks(ModelRc::new(VecModel::from(boxes.iter().map(|&(x, y, w, h)| PcbBox { x, y, w, h }).collect::<Vec<_>>())));
    b.set_log(ModelRc::new(VecModel::from(
        ["[ 0.000] reset vector", "[ 0.012] cpu0: TF-1900 @ 3.6 GHz", "[ 0.031] dram: 4 x 16 GiB ok", "[ 0.058] net: link up 1 Gb/s", "[ 0.077] audio: codec ready", "[ 0.102] usb: 4 ports", "[ 0.140] jellyfin: server reachable", "[ 0.166] turtlefin: ready."]
            .iter()
            .map(|s| SharedString::from(*s))
            .collect::<Vec<_>>(),
    )));
    let n_pulses = 50;
    let count = layout.traces.len().max(1);
    let mut live = Live { layout, pulses: Vec::new(), dots: Rc::new(VecModel::from(vec![PcbDot { x: 0.0, y: 0.0, a: 0.0, r: 1.0 }; n_pulses * TRAIL])), seed: 0x9e37_79b9, phase: 0.0, last: Instant::now() };
    for i in 0..n_pulses {
        let f = live.rand();
        let sp = 120.0 + live.rand() * 160.0;
        live.pulses.push(Pulse { trace: (i * 37) % count, f, speed: sp, alive: true });
    }
    b.set_dots(ModelRc::from(live.dots.clone()));
    LIVE.with(|l| *l.borrow_mut() = Some(live));
    b.set_tty(ModelRc::new(VecModel::from(tty_rows(0))));

    // Signaux et oscilloscope : 30 images/s, seulement quand le décor est à l'écran et animé.
    let fast = Rc::new(slint::Timer::default());
    let check = slint::Timer::default();
    {
        let (w, fast) = (u.as_weak(), fast.clone());
        check.start(slint::TimerMode::Repeated, Duration::from_millis(500), move || {
            let Some(u) = w.upgrade() else { return };
            let on = visible(&u);
            if on && !fast.running() {
                LIVE.with(|l| {
                    if let Some(l) = l.borrow_mut().as_mut() {
                        l.last = Instant::now();
                    }
                });
                let w2 = u.as_weak();
                fast.start(slint::TimerMode::Repeated, Duration::from_millis(33), move || {
                    if let Some(u) = w2.upgrade() {
                        frame(&u);
                    }
                });
            } else if !on && fast.running() {
                fast.stop();
            }
        });
    }
    // tty0 : toutes les 2 s (taille du cache lue à part, dans un fil, toutes les 30 s).
    let info = slint::Timer::default();
    {
        let (w, app) = (u.as_weak(), app.clone());
        let cache = Arc::new(Mutex::new(0u64));
        let mut tick = 0u32;
        probe(&app);
        info.start(slint::TimerMode::Repeated, Duration::from_secs(2), move || {
            let Some(u) = w.upgrade() else { return };
            if u.global::<Theme>().get_scenery() != 3 {
                return;
            }
            if tick % 15 == 0 {
                probe(&app);
                let c = cache.clone();
                std::thread::spawn(move || {
                    *c.lock().unwrap() = crate::cache_size() + crate::api::Client::resp_cache_size();
                });
            }
            tick += 1;
            let rows = tty_rows(*cache.lock().unwrap());
            u.global::<Pcb>().set_tty(ModelRc::new(VecModel::from(rows)));
        });
    }
    vec![check, info]
}
