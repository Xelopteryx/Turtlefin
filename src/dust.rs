//! Changement de langue : une barre lumineuse parcourt chaque ligne de texte et la recouvre de
//! blanc à l'ouverture de la liste des langues (`dissolve`) ; au choix (ou à l'annulation), les
//! blocs prennent la largeur des nouveaux mots et la barre repasse en dévoilant le texte (`reform`).
//! L'animation est faite par Slint (ui/dust.slint, quelques rectangles) ; ici, on repère seulement
//! les lignes de texte, une fois à l'ouverture et une fois au choix.
//!
//! Les textes sont trouvés sans rien savoir de la page : elle est capturée telle quelle, puis avec
//! chaque texte inversé, puis décalé d'une lettre (`i18n::set_pseudo`) — mêmes lettres, même
//! largeur, mais chaque lettre change de place : les pixels qui changent sont ceux des textes, tous.

use crate::{i18n, AppWindow, DustLine, Tr};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

/// Recalcule les textes venus de Rust (rangées des Paramètres, étiquettes du démarrage).
pub type Refresh = Rc<dyn Fn(&AppWindow)>;

/// Durées des phases (voir ui/dust.slint) : passage de la barre sur une ligne, changement de
/// largeur des blocs, et départ de la dernière ligne au plus tard.
const SWEEP_MS: i64 = 420;
const MORPH_MS: i64 = 260;
const STAGGER_MAX_MS: i64 = 420;

/// Ligne de texte, en pixels logiques.
#[derive(Clone, Copy)]
struct Line {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

struct State {
    lines: Vec<Line>,
    refresh: Refresh,
    /// Fin du dévoilement (phase 3), et ce qui est à faire alors.
    timer: Option<slint::Timer>,
    done: Option<Box<dyn FnOnce(&AppWindow)>>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn bump(u: &AppWindow) {
    let t = u.global::<Tr>();
    t.set_l(t.get_l() + 1);
}

struct Snap {
    w: usize,
    h: usize,
    px: Vec<[u8; 3]>,
}

fn snap(u: &AppWindow) -> Option<Snap> {
    let b = u.window().take_snapshot().map_err(|e| eprintln!("turtlefin : effet de langue impossible (capture : {e})")).ok()?;
    let (w, h) = (b.width() as usize, b.height() as usize);
    if w == 0 || h == 0 {
        return None;
    }
    Some(Snap { w, h, px: b.as_slice().iter().map(|p| [p.r, p.g, p.b]).collect() })
}

/// Lignes de texte de la page (sans la liste des langues ni l'effet : dust-snap).
fn find_lines(u: &AppWindow, refresh: &Refresh) -> Option<Vec<Line>> {
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
    let (w, h) = (a.w, a.h);
    let mut mask = vec![false; w * h];
    for o in others.into_iter().flatten() {
        if o.w != w || o.h != h {
            return None;
        }
        for (i, m) in mask.iter_mut().enumerate() {
            let (p, q) = (a.px[i], o.px[i]);
            let d = (p[0] as i32 - q[0] as i32).abs() + (p[1] as i32 - q[1] as i32).abs() + (p[2] as i32 - q[2] as i32).abs();
            if d > 24 {
                *m = true;
            }
        }
    }
    let s = u.window().scale_factor();
    let lines = group_lines(&mask, w, h, s);
    if lines.is_empty() {
        eprintln!("turtlefin : effet de langue : aucun texte trouvé à l'écran");
        return None;
    }
    Some(lines)
}

/// Regroupe les pixels de texte en lignes : les lettres d'un même mot et les mots d'une même ligne
/// (écart de moins de 12 px) forment un bloc ; les lignes d'un paragraphe restent séparées.
fn group_lines(mask: &[bool], w: usize, h: usize, s: f32) -> Vec<Line> {
    let gap = (12.0 * s) as usize;
    // Suites de pixels de texte par rangée, rapprochées si l'écart est petit.
    let mut runs: Vec<Vec<(usize, usize)>> = vec![Vec::new(); h];
    for (y, row_runs) in runs.iter_mut().enumerate() {
        let row = &mask[y * w..(y + 1) * w];
        let mut x = 0;
        while x < w {
            if !row[x] {
                x += 1;
                continue;
            }
            let l = x;
            while x < w && row[x] {
                x += 1;
            }
            match row_runs.last_mut() {
                Some(last) if l - last.1 <= gap => last.1 = x,
                _ => row_runs.push((l, x)),
            }
        }
    }
    // Composantes connexes des suites (rangées voisines qui se chevauchent), par union-find.
    let mut ids: Vec<Vec<usize>> = Vec::with_capacity(h);
    let mut parent: Vec<usize> = Vec::new();
    let mut boxes: Vec<(usize, usize, usize, usize)> = Vec::new();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for y in 0..h {
        let mut row_ids = Vec::with_capacity(runs[y].len());
        for &(l, r) in &runs[y] {
            let id = parent.len();
            parent.push(id);
            boxes.push((l, y, r, y + 1));
            if y > 0 {
                for (k, &(pl, pr)) in runs[y - 1].iter().enumerate() {
                    if pl < r && l < pr {
                        let (a, b) = (find(&mut parent, ids[y - 1][k]), find(&mut parent, id));
                        if a != b {
                            parent[b] = a;
                        }
                    }
                }
            }
            row_ids.push(id);
        }
        ids.push(row_ids);
    }
    let mut merged: std::collections::HashMap<usize, (usize, usize, usize, usize)> = std::collections::HashMap::new();
    for i in 0..parent.len() {
        let r = find(&mut parent, i);
        let b = boxes[i];
        let e = merged.entry(r).or_insert(b);
        *e = (e.0.min(b.0), e.1.min(b.1), e.2.max(b.2), e.3.max(b.3));
    }
    let mut lines: Vec<Line> = merged
        .into_values()
        .filter(|&(x0, y0, x1, y1)| (x1 - x0) as f32 >= 4.0 * s && (y1 - y0) as f32 >= 4.0 * s && ((y1 - y0) as f32) < 90.0 * s)
        .map(|(x0, y0, x1, y1)| Line { x: x0 as f32 / s, y: y0 as f32 / s, w: (x1 - x0) as f32 / s, h: (y1 - y0) as f32 / s })
        .collect();
    // Ordre de lecture : de haut en bas, puis de gauche à droite (lignes à peu près alignées).
    lines.sort_by(|a, b| ((a.y / 8.0) as i32, a.x as i32).cmp(&((b.y / 8.0) as i32, b.x as i32)));
    lines
}

fn model(lines: &[Line], from: Option<&[Line]>) -> ModelRc<DustLine> {
    let n = lines.len().max(1) as i64;
    let step = (STAGGER_MAX_MS / n).min(28);
    let rows: Vec<DustLine> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            // Largeur d'avant : la ligne d'avant qui chevauche le plus celle-ci (même rangée).
            let prev = from.and_then(|f| {
                f.iter()
                    .filter(|o| o.y < l.y + l.h && l.y < o.y + o.h)
                    .min_by(|a, b| (a.x - l.x).abs().total_cmp(&(b.x - l.x).abs()))
            });
            // Au départ, le bloc couvre l'ancien texte ET le nouveau (déjà affiché dessous, peut-être
            // plus long) ; il se resserre ensuite sur le nouveau.
            let (fx, fw) = match (from, prev) {
                (Some(_), Some(o)) => {
                    let x0 = o.x.min(l.x);
                    (x0, (o.x + o.w).max(l.x + l.w) - x0)
                }
                _ => (l.x, l.w),
            };
            DustLine { x: l.x, y: l.y, w: l.w, h: l.h, fx, fw, delay: i as i64 * step }
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

/// Ouverture de la liste des langues : la barre recouvre les lignes de texte. `layer` : où les
/// blocs sont posés (1 Paramètres, 2 démarrage), sous la liste ; `refresh` : textes venus de Rust.
pub fn dissolve(u: &AppWindow, layer: i32, refresh: Refresh) {
    finish(u);
    let Some(lines) = find_lines(u, &refresh) else { return };
    u.set_dust_lines(model(&lines, None));
    u.set_dust_layer(layer);
    u.set_dust_on(true);
    u.set_dust_cloud(true);
    u.invoke_dust_start(1);
    // Une fois recouvert : plus d'animation (rien ne bouge pendant le choix).
    let weak = u.as_weak();
    slint::Timer::single_shot(std::time::Duration::from_millis((STAGGER_MAX_MS + SWEEP_MS + 60) as u64), move || {
        if let Some(u) = weak.upgrade() {
            if u.get_dust_cloud() {
                u.invoke_dust_start(2);
            }
        }
    });
    STATE.with(|s| *s.borrow_mut() = Some(State { lines, refresh, timer: None, done: None }));
}

/// Choix fait (`change` : nouvelle langue et textes) ou annulé (None) : les blocs prennent la
/// largeur des nouveaux mots, la barre les dévoile ; `done` à la fin (tout de suite sans effet).
pub fn reform(u: &AppWindow, change: Option<&dyn Fn(&AppWindow)>, done: impl FnOnce(&AppWindow) + 'static) {
    let st = STATE.with(|s| s.borrow_mut().take());
    let Some(st) = st.filter(|_| u.get_dust_cloud()) else {
        finish(u);
        if let Some(c) = change {
            c(u);
            bump(u);
        }
        done(u);
        return;
    };
    u.set_dust_cloud(false);
    let lines = match change {
        Some(c) => {
            c(u);
            bump(u);
            match find_lines(u, &st.refresh) {
                Some(new) => {
                    u.set_dust_lines(model(&new, Some(&st.lines)));
                    new
                }
                None => {
                    finish(u);
                    done(u);
                    return;
                }
            }
        }
        None => {
            u.set_dust_lines(model(&st.lines, None));
            st.lines.clone()
        }
    };
    u.invoke_dust_start(3);
    let n = lines.len().max(1) as i64;
    let total = MORPH_MS + (STAGGER_MAX_MS / n).min(28) * n + SWEEP_MS + 40;
    let timer = slint::Timer::default();
    let weak = u.as_weak();
    timer.start(slint::TimerMode::SingleShot, std::time::Duration::from_millis(total as u64), move || {
        if let Some(u) = weak.upgrade() {
            finish(&u);
        }
    });
    STATE.with(|s| *s.borrow_mut() = Some(State { lines, refresh: st.refresh, timer: Some(timer), done: Some(Box::new(done)) }));
}

/// Arrête l'effet sans attendre (le texte reste tel quel) et fait ce qui était prévu à la fin.
pub fn finish(u: &AppWindow) {
    let st = STATE.with(|s| s.borrow_mut().take());
    u.set_dust_on(false);
    u.set_dust_cloud(false);
    u.set_dust_lines(ModelRc::default());
    if let Some(mut st) = st {
        if let Some(t) = st.timer.take() {
            t.stop();
        }
        if let Some(f) = st.done.take() {
            f(u);
        }
    }
}
