//! Changement de langue « en poussière », comme le slogan de loadix.fun (« Rien ne se perd, tout
//! se transforme »).
//!
//! 1. `dissolve`, à l'ouverture de la liste des langues : tous les textes de la page partent en
//!    grains, qui flottent en nuage (courant tourbillonnant) pendant le choix.
//! 2. `reform`, au choix (ou à l'annulation) : les mêmes grains vont réécrire les mots dans la
//!    nouvelle langue (ou les anciens), par ordre de lecture.
//!
//! Les textes sont trouvés sans rien savoir de la page : elle est capturée telle quelle, puis avec
//! chaque texte inversé, puis décalé d'une lettre (`i18n::set_pseudo`) — mêmes lettres, même
//! largeur, mais chaque lettre change de place : les pixels qui changent sont ceux des textes, tous.
//! Une image calculée ici (dust-img dans app.slint) est posée sous la liste des langues : le fond
//! sous les textes, et les grains par-dessus. À la fin elle est identique à la page, puis retirée.

use crate::{i18n, AppWindow, Tr};
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

/// Recalcule les textes venus de Rust (rangées des Paramètres, étiquettes du démarrage).
pub type Refresh = Rc<dyn Fn(&AppWindow)>;

/// Départ en vague de gauche à droite (secondes), et durée maximale d'un retour.
const SWEEP: f32 = 0.30;
const MAX_REFORM: f32 = 2.6;

struct Snap {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>,
}

/// Une case de texte : coin, couleur moyenne, part de la case couverte.
#[derive(Clone, Copy)]
struct Cell {
    x: f32,
    y: f32,
    rgb: [f32; 3],
    cov: f32,
}

struct Grain {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    seed: f32,
    /// Point autour duquel le grain flotte dans le nuage.
    ax: f32,
    ay: f32,
    rgb: [f32; 3],
    cov: f32,
    /// Départ (nuage) ou départ vers la case à écrire (retour), en secondes depuis le début de la phase.
    delay: f32,
    /// Encore à sa place d'origine (nuage, avant son départ).
    home: Option<(f32, f32)>,
    /// Case à réécrire (retour) ; None : grain en trop, il s'efface.
    target: Option<usize>,
    arrived: bool,
}

/// Textes d'une capture : pixels de texte, fond sous les textes, cases de texte.
struct Scene {
    w: usize,
    h: usize,
    img: Vec<[u8; 4]>,
    txt: Vec<bool>,
    wide: Vec<bool>,
    bg: Vec<[u8; 4]>,
    cells: Vec<Cell>,
}

enum Phase {
    Cloud,
    Reform { cells: Vec<Cell>, img: Vec<[u8; 4]>, txt: Vec<bool> },
}

struct Run {
    w: usize,
    h: usize,
    s: f32,
    cell: usize,
    a: Scene,
    base: Vec<[u8; 4]>,
    trail: Vec<[u8; 4]>,
    grains: Vec<Grain>,
    phase: Phase,
    start: Instant,
    frames: u32,
    noise_t: f32,
    rng: u64,
    refresh: Refresh,
    done: Option<Box<dyn FnOnce(&AppWindow)>>,
}

thread_local! {
    static RUN: RefCell<Option<(Run, slint::Timer)>> = const { RefCell::new(None) };
}

/// Capture de la fenêtre.
fn snap(u: &AppWindow) -> Option<Snap> {
    let b = u.window().take_snapshot().map_err(|e| eprintln!("turtlefin : effet de langue impossible (capture : {e})")).ok()?;
    let (w, h) = (b.width() as usize, b.height() as usize);
    if w == 0 || h == 0 {
        return None;
    }
    Some(Snap { w, h, px: b.as_slice().iter().map(|p| [p.r, p.g, p.b, 255]).collect() })
}

fn bump(u: &AppWindow) {
    let t = u.global::<Tr>();
    t.set_l(t.get_l() + 1);
}

/// La page (sans le panneau de choix ni l'effet : dust-snap) telle quelle, puis ses textes inversés
/// et décalés : (capture, pixels de texte).
fn capture(u: &AppWindow, refresh: &Refresh) -> Option<(Snap, Vec<bool>)> {
    let on = u.get_dust_on();
    u.set_dust_on(false);
    u.set_dust_snap(true);
    let a = snap(u);
    let mut others = Vec::new();
    for mode in [1u8, 2] {
        i18n::set_pseudo(mode);
        refresh(u);
        bump(u);
        others.push(snap(u));
    }
    i18n::set_pseudo(0);
    refresh(u);
    bump(u);
    u.set_dust_snap(false);
    u.set_dust_on(on);
    let a = a?;
    let mut mask = vec![false; a.w * a.h];
    for o in others.into_iter().flatten() {
        if o.w != a.w || o.h != a.h {
            return None;
        }
        for (i, m) in mask.iter_mut().enumerate() {
            if dist(a.px[i], o.px[i]) > 24 {
                *m = true;
            }
        }
    }
    if !mask.iter().any(|&m| m) {
        eprintln!("turtlefin : effet de langue : aucun texte trouvé à l'écran");
        return None;
    }
    Some((a, mask))
}

/// Ouverture de la liste des langues : les textes partent en nuage. `layer` : où l'image est posée
/// (1 Paramètres, 2 démarrage), sous la liste ; `refresh` : textes venus de Rust.
pub fn dissolve(u: &AppWindow, layer: i32, refresh: Refresh) {
    finish(u);
    let Some((a, mask)) = capture(u, &refresh) else { return };
    let s = u.window().scale_factor();
    let cell = ((2.0 * s).round() as usize).clamp(2, 4);
    let scene = Scene::new(a, &mask, cell);
    let (w, h) = (scene.w, scene.h);
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15 ^ (w as u64 * 31 + h as u64);
    let (minx, maxx) = scene.cells.iter().fold((f32::MAX, 0f32), |(a, b), c| (a.min(c.x), b.max(c.x)));
    let grains = scene
        .cells
        .iter()
        .map(|c| {
            let seed = rand(&mut rng);
            let ang = rand(&mut rng) * std::f32::consts::TAU;
            let r = (12.0 + rand(&mut rng) * 34.0) * s;
            Grain {
                x: c.x,
                y: c.y,
                vx: 0.0,
                vy: 0.0,
                seed,
                ax: c.x + ang.cos() * r,
                ay: c.y + ang.sin() * r * 0.7,
                rgb: c.rgb,
                cov: c.cov,
                delay: ((c.x - minx) / (maxx - minx).max(1.0)) * SWEEP + seed * 0.08,
                home: Some((c.x, c.y)),
                target: None,
                arrived: false,
            }
        })
        .collect();
    let base = (0..w * h).map(|i| if scene.wide[i] { if scene.txt[i] { scene.bg[i] } else { scene.img[i] } } else { [0; 4] }).collect();
    let mut run = Run {
        w,
        h,
        s,
        cell,
        a: scene,
        base,
        trail: vec![[0u8; 4]; w * h],
        grains,
        phase: Phase::Cloud,
        start: Instant::now(),
        frames: 0,
        noise_t: 0.0,
        rng,
        refresh,
        done: None,
    };
    u.set_dust_img(run.frame());
    u.set_dust_w(w as f32 / s);
    u.set_dust_h(h as f32 / s);
    u.set_dust_layer(layer);
    u.set_dust_on(true);
    u.set_dust_cloud(true);
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

/// Choix fait (`change` : nouvelle langue et textes) ou annulé (None) : les grains réécrivent les
/// mots ; `done` à la fin (tout de suite s'il n'y a pas de nuage).
pub fn reform(u: &AppWindow, change: Option<&dyn Fn(&AppWindow)>, done: impl FnOnce(&AppWindow) + 'static) {
    let taken = RUN.with(|r| r.borrow_mut().take());
    let (mut run, timer) = match taken {
        Some((run, timer)) if matches!(run.phase, Phase::Cloud) => (run, timer),
        other => {
            // Pas de nuage (effet impossible, ou déjà fini) : le changement seul.
            if let Some((mut run, timer)) = other {
                timer.stop();
                if let Some(f) = run.done.take() {
                    f(u);
                }
            }
            u.set_dust_on(false);
            u.set_dust_cloud(false);
            if let Some(c) = change {
                c(u);
                bump(u);
            }
            done(u);
            return;
        }
    };
    u.set_dust_cloud(false);
    let (cells, img, txt) = match change {
        Some(c) => {
            c(u);
            bump(u);
            match capture(u, &run.refresh) {
                Some((b, mask)) if b.w == run.w && b.h == run.h => {
                    let sc = Scene::new(b, &mask, run.cell);
                    // Fond : le nouveau rendu, sans ses textes.
                    for i in 0..run.w * run.h {
                        if sc.wide[i] || run.a.wide[i] {
                            run.base[i] = if sc.txt[i] { sc.bg[i] } else { sc.img[i] };
                        }
                    }
                    (sc.cells, sc.img, sc.txt)
                }
                // Capture impossible : plus d'effet, la page s'affiche telle quelle.
                _ => {
                    timer.stop();
                    u.set_dust_on(false);
                    u.set_dust_img(Image::default());
                    done(u);
                    return;
                }
            }
        }
        None => (run.a.cells.clone(), run.a.img.clone(), run.a.txt.clone()),
    };
    run.assign(&cells);
    run.phase = Phase::Reform { cells, img, txt };
    run.start = Instant::now();
    run.frames = 0;
    run.done = Some(Box::new(done));
    RUN.with(|r| *r.borrow_mut() = Some((run, timer)));
}

/// Arrête l'effet sans attendre (nuage compris) et appelle son rappel de fin.
pub fn finish(u: &AppWindow) {
    let run = RUN.with(|r| r.borrow_mut().take());
    if let Some((mut run, timer)) = run {
        timer.stop();
        u.set_dust_on(false);
        u.set_dust_cloud(false);
        u.set_dust_img(Image::default());
        if let Some(f) = run.done.take() {
            f(u);
        }
    }
}

fn dist(p: [u8; 4], q: [u8; 4]) -> i32 {
    (p[0] as i32 - q[0] as i32).abs() + (p[1] as i32 - q[1] as i32).abs() + (p[2] as i32 - q[2] as i32).abs()
}

impl Scene {
    fn new(a: Snap, mask: &[bool], cell: usize) -> Scene {
        let (w, h) = (a.w, a.h);
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
        let (bg_h, len_h) = fill_runs(&a.px, &wide, w, h, false);
        let (bg_v, len_v) = fill_runs(&a.px, &wide, w, h, true);
        let bg: Vec<[u8; 4]> = (0..w * h).map(|i| if len_v[i] <= len_h[i] { bg_v[i] } else { bg_h[i] }).collect();
        let txt: Vec<bool> = (0..w * h).map(|i| wide[i] && dist(a.px[i], bg[i]) > 30).collect();
        // Cases de texte (cell x cell pixels), dans l'ordre de lecture.
        let mut cells = Vec::new();
        for cy in (0..h).step_by(cell) {
            for cx in (0..w).step_by(cell) {
                let (mut n, mut sum) = (0u32, [0u32; 3]);
                for y in cy..(cy + cell).min(h) {
                    for x in cx..(cx + cell).min(w) {
                        if txt[y * w + x] {
                            n += 1;
                            for k in 0..3 {
                                sum[k] += a.px[y * w + x][k] as u32;
                            }
                        }
                    }
                }
                if n > 0 {
                    cells.push(Cell {
                        x: cx as f32,
                        y: cy as f32,
                        rgb: [sum[0] as f32 / n as f32, sum[1] as f32 / n as f32, sum[2] as f32 / n as f32],
                        cov: (n as f32 / (cell * cell) as f32 * 1.6).min(1.0),
                    });
                }
            }
        }
        Scene { w, h, img: a.px, txt, wide, bg, cells }
    }
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
    /// Grains du nuage -> cases à écrire, dans l'ordre de lecture : la n-ième case reçoit le grain
    /// au même rang (proportionnellement) ; s'il en manque, un grain se dédouble ; ceux en trop
    /// s'effacent.
    fn assign(&mut self, cells: &[Cell]) {
        let old = std::mem::take(&mut self.grains);
        let (ng, nt) = (old.len(), cells.len());
        let mut used = vec![false; ng];
        let (minx, maxx) = cells.iter().fold((f32::MAX, 0f32), |(a, b), c| (a.min(c.x), b.max(c.x)));
        let mut out = Vec::with_capacity(nt + ng);
        if ng > 0 {
            for (j, c) in cells.iter().enumerate() {
                let i = j * ng / nt;
                used[i] = true;
                let src = &old[i];
                let seed = rand(&mut self.rng);
                let (x, y) = src.home.unwrap_or((src.x, src.y));
                out.push(Grain {
                    x,
                    y,
                    vx: src.vx,
                    vy: src.vy,
                    seed,
                    ax: src.ax,
                    ay: src.ay,
                    rgb: src.rgb,
                    cov: src.cov,
                    delay: ((c.x - minx) / (maxx - minx).max(1.0)) * SWEEP + seed * 0.06,
                    home: None,
                    target: Some(j),
                    arrived: false,
                });
            }
        }
        for (i, mut g) in old.into_iter().enumerate() {
            if !used[i] {
                g.target = None;
                if let Some((hx, hy)) = g.home.take() {
                    g.x = hx;
                    g.y = hy;
                }
                out.push(g);
            }
        }
        self.grains = out;
    }

    /// Avance jusqu'à l'instant présent (60 pas par seconde) ; faux quand c'est fini.
    fn step(&mut self) -> bool {
        let want = (self.start.elapsed().as_secs_f32() * 60.0) as u32;
        let mut n = 0;
        while self.frames < want && n < 4 {
            self.tick();
            self.frames += 1;
            n += 1;
        }
        match self.phase {
            Phase::Cloud => true,
            Phase::Reform { .. } => {
                let t = self.frames as f32 / 60.0;
                let all = self.grains.iter().all(|g| g.target.is_none() || g.arrived);
                !(all && t > 0.5) && t < MAX_REFORM
            }
        }
    }

    fn tick(&mut self) {
        let t = self.frames as f32 / 60.0;
        let s = self.s;
        self.noise_t += 0.012;
        let reform = matches!(self.phase, Phase::Reform { .. });
        let cells: &[Cell] = match &self.phase {
            Phase::Reform { cells, .. } => cells,
            Phase::Cloud => &[],
        };
        for g in &mut self.grains {
            if g.arrived {
                continue;
            }
            if let Some((hx, hy)) = g.home {
                if t < g.delay {
                    continue;
                }
                // Envol : vitesse au hasard, comme au survol du slogan de loadix.
                g.home = None;
                let ang = rand(&mut self.rng) * std::f32::consts::TAU;
                let sp = (0.8 + rand(&mut self.rng) * 2.2) * s;
                g.x = hx;
                g.y = hy;
                g.vx = ang.cos() * sp;
                g.vy = ang.sin() * sp - 0.5 * s;
            }
            match g.target {
                Some(j) if reform && t >= g.delay => {
                    // Retour : ressort amorti vers la case à écrire ; la couleur passe à la sienne.
                    let c = cells[j];
                    g.vx = g.vx * 0.62 + (c.x - g.x) * 0.08;
                    g.vy = g.vy * 0.62 + (c.y - g.y) * 0.08;
                    g.x += g.vx;
                    g.y += g.vy;
                    for k in 0..3 {
                        g.rgb[k] += (c.rgb[k] - g.rgb[k]) * 0.15;
                    }
                    g.cov += (c.cov - g.cov) * 0.15;
                    if (c.x - g.x).abs() < 0.4 && (c.y - g.y).abs() < 0.4 && g.vx.abs() < 0.25 && g.vy.abs() < 0.25 {
                        g.x = c.x;
                        g.y = c.y;
                        g.arrived = true;
                    }
                }
                _ => {
                    // Nuage : courant tourbillonnant, rappel léger vers son point d'attache.
                    let (fx, fy) = curl(g.x / s, g.y / s, self.noise_t);
                    g.vx = (g.vx + fx * 2.4 * s + (g.ax - g.x) * 0.006) * 0.95;
                    g.vy = (g.vy + fy * 2.4 * s + (g.ay - g.y) * 0.006) * 0.95;
                    g.x += g.vx;
                    g.y += g.vy;
                }
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
        let reform = matches!(self.phase, Phase::Reform { .. });
        let (dst_img, dst_txt, cells): (&[[u8; 4]], &[bool], &[Cell]) = match &self.phase {
            Phase::Reform { img, txt, cells } => (img, txt, cells),
            Phase::Cloud => (&self.a.img, &self.a.txt, &[]),
        };
        for g in &self.grains {
            // À sa place (avant l'envol, ou arrivé) : les pixels exacts du texte.
            let exact = match (g.home, g.arrived, g.target) {
                (Some((hx, hy)), _, _) => Some((hx as usize, hy as usize, &self.a.img[..], &self.a.txt[..])),
                (None, true, Some(j)) => Some((cells[j].x as usize, cells[j].y as usize, dst_img, dst_txt)),
                _ => None,
            };
            if let Some((cx, cy, img, txt)) = exact {
                for y in cy..(cy + cell).min(h) {
                    for x in cx..(cx + cell).min(w) {
                        if txt[y * w + x] {
                            self.trail[y * w + x] = img[y * w + x];
                        }
                    }
                }
                continue;
            }
            let mut alpha = g.cov * 0.85;
            if reform && g.target.is_none() {
                alpha *= (1.0 - t / 0.45).max(0.0);
            }
            if alpha <= 0.02 {
                continue;
            }
            let size = if g.seed > 0.6 { cell + 1 } else { cell };
            let (px, py) = (g.x.round() as i64, g.y.round() as i64);
            let rgb = [g.rgb[0] as u8, g.rgb[1] as u8, g.rgb[2] as u8];
            for y in py..py + size as i64 {
                for x in px..px + size as i64 {
                    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                        over(&mut self.trail[y as usize * w + x as usize], rgb, alpha);
                    }
                }
            }
        }
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
