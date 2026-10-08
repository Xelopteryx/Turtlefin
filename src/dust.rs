//! Changement de langue « en poussière », comme le slogan de loadix.fun (« Rien ne se perd, tout
//! se transforme ») : les textes partent en grains emportés par un courant tourbillonnant, et les
//! grains des nouveaux textes reviennent se poser à leur place.
//!
//! Fonctionne pour n'importe quel écran sans rien savoir de ses textes : la fenêtre est capturée
//! juste avant et juste après le changement (même instant d'animation), les pixels qui diffèrent
//! sont ceux des textes. Une image par-dessus l'interface (dust-img dans app.slint) montre le fond
//! sous ces textes et les grains ; à la fin elle est identique à l'interface, puis retirée.

use crate::{AppWindow, Tr};
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};
use std::cell::RefCell;
use std::time::Instant;

/// Durées (secondes) : départ en vague de gauche à droite, envol, retour des nouveaux grains.
const SWEEP: f32 = 0.30;
const FLY: f32 = 0.75;
const BACK_AT: f32 = 0.38;
const FADE_IN: f32 = 0.25;
const MAX_TIME: f32 = 2.4;

struct Grain {
    /// Coin de la case d'origine (pixels de la zone), position et vitesse (pixels par image).
    hx: f32,
    hy: f32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    seed: f32,
    delay: f32,
    rgb: [u8; 3],
    cov: f32,
    gone: bool,
    home: bool,
    /// Grain qui s'envole (ou revient en volant) ; les autres s'effacent (ou apparaissent) sur place.
    fly: bool,
}

struct Run {
    /// Zone animée (pixels physiques de la fenêtre) et facteur d'échelle.
    x0: usize,
    y0: usize,
    w: usize,
    h: usize,
    s: f32,
    cell: usize,
    a: Vec<[u8; 4]>,
    b: Vec<[u8; 4]>,
    old_txt: Vec<bool>,
    new_txt: Vec<bool>,
    /// Fond sous les textes (nouveau rendu sans ses textes) ; transparent hors des textes.
    base: Vec<[u8; 4]>,
    /// Calque des grains, qui s'efface un peu à chaque image (traînées de fumée).
    trail: Vec<[u8; 4]>,
    old: Vec<Grain>,
    new: Vec<Grain>,
    start: Instant,
    frames: u32,
    noise_t: f32,
    rng: u64,
    done: Option<Box<dyn FnOnce(&AppWindow)>>,
}

thread_local! {
    static RUN: RefCell<Option<(Run, slint::Timer)>> = const { RefCell::new(None) };
}

/// Applique `change` (qui change la langue et les textes) avec l'effet ; `done` est appelé à la fin
/// (tout de suite si l'effet est impossible : rendu logiciel, rien n'a changé à l'écran).
pub fn switch(u: &AppWindow, change: impl FnOnce(&AppWindow), done: impl FnOnce(&AppWindow) + 'static) {
    finish(u);
    let before = u.window().take_snapshot().map_err(|e| eprintln!("turtlefin : effet de langue impossible (capture : {e})")).ok();
    change(u);
    // Les textes de l'interface se recalculent sur Tr.l (voir i18n::on_change, qui passe plus tard).
    let t = u.global::<Tr>();
    t.set_l(t.get_l() + 1);
    let run = before.and_then(|a| {
        let b = u.window().take_snapshot().map_err(|e| eprintln!("turtlefin : effet de langue impossible (capture : {e})")).ok()?;
        Run::new(&a, &b, u.window().scale_factor())
    });
    let Some(mut run) = run else {
        done(u);
        return;
    };
    run.done = Some(Box::new(done));
    let img = run.frame();
    u.set_dust_img(img);
    u.set_dust_x(run.x0 as f32 / run.s);
    u.set_dust_y(run.y0 as f32 / run.s);
    u.set_dust_w(run.w as f32 / run.s);
    u.set_dust_h(run.h as f32 / run.s);
    u.set_dust_on(true);
    let timer = slint::Timer::default();
    let weak = u.as_weak();
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(16), move || {
        let Some(u) = weak.upgrade() else { return };
        let more = RUN.with(|r| {
            let mut r = r.borrow_mut();
            let Some((run, _)) = r.as_mut() else { return false };
            let more = run.step();
            if more {
                u.set_dust_img(run.frame());
            }
            more
        });
        if !more {
            finish(&u);
        }
    });
    RUN.with(|r| *r.borrow_mut() = Some((run, timer)));
}

/// Arrête l'effet en cours (s'il y en a un) et appelle son rappel de fin.
pub fn finish(u: &AppWindow) {
    let run = RUN.with(|r| r.borrow_mut().take());
    if let Some((mut run, timer)) = run {
        timer.stop();
        u.set_dust_on(false);
        u.set_dust_img(Image::default());
        if let Some(f) = run.done.take() {
            f(u);
        }
    }
}

fn dist(p: [u8; 4], q: [u8; 4]) -> i32 {
    (p[0] as i32 - q[0] as i32).abs() + (p[1] as i32 - q[1] as i32).abs() + (p[2] as i32 - q[2] as i32).abs()
}

/// Bruit de valeur lissé et son « curl » (champ de courant sans divergence), comme loadix.fun.
fn hash(x: f32, y: f32) -> f32 {
    let n = (x * 127.1 + y * 311.7).sin() * 43758.547;
    n - n.floor()
}

fn noise(x: f32, y: f32) -> f32 {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (x - ix, y - iy);
    let (u, v) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (a, b, c, d) = (hash(ix, iy), hash(ix + 1.0, iy), hash(ix, iy + 1.0), hash(ix + 1.0, iy + 1.0));
    (a * (1.0 - u) + b * u) * (1.0 - v) + (c * (1.0 - u) + d * u) * v
}

fn curl(x: f32, y: f32, t: f32) -> (f32, f32) {
    const Q: f32 = 0.014;
    const R: f32 = 1.5;
    let n1 = noise(x * Q, (y + R) * Q + t);
    let n2 = noise(x * Q, (y - R) * Q + t);
    let n3 = noise((x + R) * Q, y * Q + t);
    let n4 = noise((x - R) * Q, y * Q + t);
    ((n1 - n2) / (2.0 * R), -(n3 - n4) / (2.0 * R))
}

impl Run {
    fn new(a: &SharedPixelBuffer<Rgba8Pixel>, b: &SharedPixelBuffer<Rgba8Pixel>, s: f32) -> Option<Run> {
        let (fw, fh) = (a.width() as usize, a.height() as usize);
        if fw == 0 || fh == 0 || b.width() as usize != fw || b.height() as usize != fh {
            eprintln!("turtlefin : effet de langue : captures de tailles différentes");
            return None;
        }
        let px = |buf: &SharedPixelBuffer<Rgba8Pixel>, i: usize| {
            let p = buf.as_slice()[i];
            [p.r, p.g, p.b, 255]
        };
        // Pixels changés, et leur cadre.
        let mut diff = vec![false; fw * fh];
        let (mut minx, mut miny, mut maxx, mut maxy) = (usize::MAX, usize::MAX, 0, 0);
        for y in 0..fh {
            for x in 0..fw {
                let i = y * fw + x;
                if dist(px(a, i), px(b, i)) > 24 {
                    diff[i] = true;
                    minx = minx.min(x);
                    maxx = maxx.max(x);
                    miny = miny.min(y);
                    maxy = maxy.max(y);
                }
            }
        }
        if minx == usize::MAX {
            eprintln!("turtlefin : effet de langue : aucun texte n'a changé à l'écran");
            return None;
        }
        // Zone animée : le cadre et une marge pour l'envol des grains.
        let m = (110.0 * s) as usize;
        let (x0, y0) = (minx.saturating_sub(m), miny.saturating_sub(m));
        let (x1, y1) = ((maxx + m + 1).min(fw), (maxy + m + 1).min(fh));
        let (w, h) = (x1 - x0, y1 - y0);
        let mut ra = Vec::with_capacity(w * h);
        let mut rb = Vec::with_capacity(w * h);
        let mut mask = vec![false; w * h];
        for y in 0..h {
            for x in 0..w {
                let i = (y + y0) * fw + x + x0;
                ra.push(px(a, i));
                rb.push(px(b, i));
                mask[y * w + x] = diff[i];
            }
        }
        // Masque élargi de 2 pixels (bords adoucis des lettres).
        let r = 2usize;
        let mut wide = vec![false; w * h];
        for y in 0..h {
            for x in 0..w {
                if mask[y * w + x] {
                    for yy in y.saturating_sub(r)..(y + r + 1).min(h) {
                        for xx in x.saturating_sub(r)..(x + r + 1).min(w) {
                            wide[yy * w + xx] = true;
                        }
                    }
                }
            }
        }
        // Fond sous les textes : dégradé entre les pixels intacts de part et d'autre, en ligne et en
        // colonne ; pour chaque pixel, la direction où l'écart est le plus court (une ligne de texte
        // est basse : en colonne, le fond d'un bouton en dégradé est repris tel quel).
        let (bg_h, len_h) = fill_runs(&rb, &wide, w, h, false);
        let (bg_v, len_v) = fill_runs(&rb, &wide, w, h, true);
        let bg: Vec<[u8; 4]> = (0..w * h).map(|i| if len_v[i] <= len_h[i] { bg_v[i] } else { bg_h[i] }).collect();
        let mut old_txt = vec![false; w * h];
        let mut new_txt = vec![false; w * h];
        let mut base = vec![[0u8; 4]; w * h];
        for i in 0..w * h {
            if wide[i] {
                old_txt[i] = dist(ra[i], bg[i]) > 30;
                new_txt[i] = dist(rb[i], bg[i]) > 30;
                base[i] = if new_txt[i] { bg[i] } else { rb[i] };
            }
        }
        // Grains : une case de 2 x 2 pixels (à l'échelle) par morceau de texte.
        let cell = ((2.0 * s).round() as usize).clamp(2, 4);
        let mut rng: u64 = 0x9E37_79B9_7F4A_7C15 ^ (w as u64 * 31 + h as u64);
        let mut old = Vec::new();
        let mut new = Vec::new();
        let (mx0, mx1) = ((minx - x0) as f32, (maxx - x0).max(minx - x0 + 1) as f32);
        for cy in (0..h).step_by(cell) {
            for cx in (0..w).step_by(cell) {
                for (txt, src, out) in [(&old_txt, &ra, &mut old), (&new_txt, &rb, &mut new)] {
                    let (mut n, mut sum) = (0u32, [0u32; 3]);
                    for y in cy..(cy + cell).min(h) {
                        for x in cx..(cx + cell).min(w) {
                            if txt[y * w + x] {
                                n += 1;
                                for k in 0..3 {
                                    sum[k] += src[y * w + x][k] as u32;
                                }
                            }
                        }
                    }
                    if n == 0 {
                        continue;
                    }
                    let seed = rand(&mut rng);
                    let wave = ((cx as f32 - mx0) / (mx1 - mx0)).clamp(0.0, 1.0);
                    out.push(Grain {
                        hx: cx as f32,
                        hy: cy as f32,
                        x: cx as f32,
                        y: cy as f32,
                        vx: 0.0,
                        vy: 0.0,
                        seed,
                        delay: wave * SWEEP + seed * 0.08,
                        rgb: [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8],
                        cov: (n as f32 / (cell * cell) as f32 * 1.6).min(1.0),
                        gone: false,
                        home: true,
                        fly: rand(&mut rng) < 0.45,
                    });
                }
            }
        }
        // Nouveaux grains : partent d'un peu plus loin, déjà dispersés.
        for g in &mut new {
            g.home = false;
            if g.fly {
                let ang = rand(&mut rng) * std::f32::consts::TAU;
                let d = (25.0 + rand(&mut rng) * 75.0) * s;
                g.x = g.hx + ang.cos() * d;
                g.y = g.hy + ang.sin() * d;
            }
        }
        Some(Run {
            x0,
            y0,
            w,
            h,
            s,
            cell,
            a: ra,
            b: rb,
            old_txt,
            new_txt,
            base,
            trail: vec![[0u8; 4]; w * h],
            old,
            new,
            start: Instant::now(),
            frames: 0,
            noise_t: 0.0,
            rng,
            done: None,
        })
    }

    /// Avance l'animation jusqu'à l'instant présent (60 pas par seconde) ; faux quand c'est fini.
    fn step(&mut self) -> bool {
        let want = (self.start.elapsed().as_secs_f32() * 60.0) as u32;
        let mut n = 0;
        while self.frames < want && n < 4 {
            self.tick();
            self.frames += 1;
            n += 1;
        }
        let t = self.frames as f32 / 60.0;
        let settled = self.old.iter().all(|g| g.gone) && self.new.iter().all(|g| g.home);
        !(settled && t > BACK_AT + SWEEP + 0.6) && t < MAX_TIME
    }

    fn tick(&mut self) {
        let t = self.frames as f32 / 60.0;
        let s = self.s;
        self.noise_t += 0.012;
        for g in &mut self.old {
            if g.gone || t < g.delay {
                continue;
            }
            if !g.fly {
                if t - g.delay > 0.22 {
                    g.gone = true;
                }
                continue;
            }
            if g.home {
                // Envol : vitesse au hasard, comme au survol du slogan de loadix.
                g.home = false;
                let ang = rand(&mut self.rng) * std::f32::consts::TAU;
                let sp = (0.8 + rand(&mut self.rng) * 2.2) * s;
                g.vx = ang.cos() * sp;
                g.vy = ang.sin() * sp - 0.6 * s;
            }
            let (fx, fy) = curl(g.x / s, g.y / s, self.noise_t);
            g.vx = (g.vx + fx * 2.2 * s) * 0.965;
            g.vy = (g.vy + fy * 2.2 * s) * 0.965;
            g.x += g.vx;
            g.y += g.vy;
            if t - g.delay > FLY {
                g.gone = true;
            }
        }
        for g in &mut self.new {
            if g.home || t < BACK_AT + g.delay {
                continue;
            }
            if !g.fly {
                // Apparaît sur place (fondu, voir frame).
                if t - BACK_AT - g.delay > 0.35 {
                    g.home = true;
                }
                continue;
            }
            // Retour : ressort amorti vers la case d'origine.
            g.vx = g.vx * 0.6 + (g.hx - g.x) * 0.085;
            g.vy = g.vy * 0.6 + (g.hy - g.y) * 0.085;
            g.x += g.vx;
            g.y += g.vy;
            if (g.hx - g.x).abs() < 0.35 && (g.hy - g.y).abs() < 0.35 && g.vx.abs() < 0.2 && g.vy.abs() < 0.2 {
                g.x = g.hx;
                g.y = g.hy;
                g.home = true;
            }
        }
    }

    /// Image de l'instant : fond sous les textes, puis le calque des grains par-dessus.
    fn frame(&mut self) -> Image {
        let (w, h, cell) = (self.w, self.h, self.cell);
        let t = self.frames as f32 / 60.0;
        // Traînées : le calque pâlit à chaque image.
        for p in &mut self.trail {
            p[3] = ((p[3] as u32 * 150) >> 8) as u8;
        }
        let s = self.s;
        let (old, new) = (std::mem::take(&mut self.old), std::mem::take(&mut self.new));
        for (grains, is_new) in [(&old, false), (&new, true)] {
            for g in grains {
                if g.gone {
                    continue;
                }
                if !g.fly && !g.home && is_new || !g.fly && !is_new && t >= g.delay {
                    // Sur place : fondu du texte exact (disparition ou apparition).
                    let f = if is_new { (t - BACK_AT - g.delay) / 0.35 } else { 1.0 - (t - g.delay) / 0.22 };
                    let f = f.clamp(0.0, 1.0);
                    if f <= 0.0 {
                        continue;
                    }
                    let (txt, src) = if is_new { (&self.new_txt, &self.b) } else { (&self.old_txt, &self.a) };
                    let (cx, cy) = (g.hx as usize, g.hy as usize);
                    for y in cy..(cy + cell).min(h) {
                        for x in cx..(cx + cell).min(w) {
                            if txt[y * w + x] {
                                let p = src[y * w + x];
                                over(&mut self.trail[y * w + x], [p[0], p[1], p[2]], f);
                            }
                        }
                    }
                    continue;
                }
                if g.home && (!is_new || t >= BACK_AT) {
                    // À sa place : les pixels exacts du texte (ancien ou nouveau).
                    let (txt, src) = if is_new { (&self.new_txt, &self.b) } else { (&self.old_txt, &self.a) };
                    let (cx, cy) = (g.hx as usize, g.hy as usize);
                    for y in cy..(cy + cell).min(h) {
                        for x in cx..(cx + cell).min(w) {
                            if txt[y * w + x] {
                                self.trail[y * w + x] = src[y * w + x];
                            }
                        }
                    }
                    continue;
                }
                if g.home {
                    continue;
                }
                let d = ((g.x - g.hx).powi(2) + (g.y - g.hy).powi(2)).sqrt() / s;
                let mut alpha = (1.0 - d / 140.0).max(0.0) * g.cov;
                if is_new {
                    let age = t - BACK_AT - g.delay;
                    if age < 0.0 {
                        continue;
                    }
                    alpha *= (age / FADE_IN).min(1.0);
                } else {
                    alpha *= (1.0 - (t - g.delay - 0.25) / (FLY - 0.25)).clamp(0.0, 1.0);
                }
                if alpha <= 0.02 {
                    continue;
                }
                let size = if g.seed > 0.6 { cell + 1 } else { cell };
                let (px, py) = (g.x.round() as i64, g.y.round() as i64);
                for y in py..py + size as i64 {
                    for x in px..px + size as i64 {
                        if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
                            continue;
                        }
                        over(&mut self.trail[y as usize * w + x as usize], g.rgb, alpha);
                    }
                }
            }
        }
        self.old = old;
        self.new = new;
        let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(w as u32, h as u32);
        for (o, (b, l)) in buf.make_mut_slice().iter_mut().zip(self.base.iter().zip(self.trail.iter())) {
            let mut c = *b;
            if l[3] > 0 {
                over(&mut c, [l[0], l[1], l[2]], l[3] as f32 / 255.0);
            }
            *o = Rgba8Pixel { r: c[0], g: c[1], b: c[2], a: c[3] };
        }
        Image::from_rgba8(buf)
    }
}

/// Remplit les suites de pixels masqués (en ligne, ou en colonne si `vertical`) par un dégradé
/// entre leurs deux voisins intacts ; rend aussi la longueur de la suite de chaque pixel.
fn fill_runs(src: &[[u8; 4]], mask: &[bool], w: usize, h: usize, vertical: bool) -> (Vec<[u8; 4]>, Vec<u32>) {
    let mut out = src.to_vec();
    let mut len = vec![u32::MAX; w * h];
    let (lines, n) = if vertical { (w, h) } else { (h, w) };
    let at = |line: usize, k: usize| if vertical { k * w + line } else { line * w + k };
    for line in 0..lines {
        let mut k = 0;
        while k < n {
            if !mask[at(line, k)] {
                k += 1;
                continue;
            }
            let l = k;
            while k < n && mask[at(line, k)] {
                k += 1;
            }
            let c0 = if l > 0 { Some(src[at(line, l - 1)]) } else { None };
            let c1 = if k < n { Some(src[at(line, k)]) } else { None };
            let (c0, c1) = match (c0, c1) {
                (Some(p), Some(q)) => (p, q),
                (Some(p), None) => (p, p),
                (None, Some(q)) => (q, q),
                (None, None) => continue,
            };
            // Suite touchant un bord : moins sûre, comptée plus longue.
            let run = (k - l) as u32 * if l == 0 || k == n { 3 } else { 1 };
            let span = (k - l + 1) as f32;
            for j in l..k {
                let t = (j - l + 1) as f32 / span;
                let i = at(line, j);
                for c in 0..3 {
                    out[i][c] = (c0[c] as f32 * (1.0 - t) + c1[c] as f32 * t) as u8;
                }
                out[i][3] = 255;
                len[i] = run;
            }
        }
    }
    (out, len)
}

/// Pose une couleur (opacité `a`) sur un pixel non prémultiplié.
fn over(dst: &mut [u8; 4], c: [u8; 3], a: f32) {
    let da = dst[3] as f32 / 255.0;
    let oa = a + da * (1.0 - a);
    if oa <= 0.0 {
        return;
    }
    for k in 0..3 {
        dst[k] = ((c[k] as f32 * a + dst[k] as f32 * da * (1.0 - a)) / oa).round() as u8;
    }
    dst[3] = (oa * 255.0).round() as u8;
}

fn rand(state: &mut u64) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 40) as f32 / (1u64 << 24) as f32
}
