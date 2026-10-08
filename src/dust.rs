//! Changement de langue : une barre lumineuse parcourt chaque ligne de texte et la recouvre de
//! blanc à l'ouverture de la liste des langues (`dissolve`) ; au choix (ou à l'annulation), les
//! blocs prennent la largeur des nouveaux mots et la barre repasse en dévoilant le texte (`reform`).
//! L'animation est faite par Slint (ui/dust.slint, quelques rectangles) ; ici, on repère seulement
//! les lignes de texte, une fois à l'ouverture et une fois au choix.
//!
//! Les textes sont trouvés sans rien savoir de la page : elle est capturée telle quelle, puis avec
//! une police aux lettres vides (mêmes largeurs) : les pixels qui changent sont ceux des textes.

use crate::{i18n, AppWindow, DustLine, Tr};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::RefCell;
use std::rc::Rc;


/// Durées des phases (voir ui/dust.slint) : passage de la barre sur une ligne, changement de
/// largeur des blocs, et départ de la dernière ligne au plus tard.
const SWEEP_MS: i64 = 420;
const MORPH_MS: i64 = 260;
const STAGGER_MAX_MS: i64 = 420;
/// Attente avant de repérer les textes (fin des animations lancées par l'ouverture de la liste).
const SETTLE_MS: u64 = 280;

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

/// Lignes de texte de la page (sans la liste des langues ni l'effet : dust-snap). La page est
/// capturée telle quelle, puis avec une police aux lettres vides mais aux largeurs identiques
/// (« Turtlefin Blank », tools/make-blank-font.py) : les pixels qui changent sont ceux des textes.
fn find_lines(u: &AppWindow) -> Option<Vec<Line>> {
    let (on, blank) = (u.get_dust_on(), u.get_dust_blank());
    u.set_dust_on(false);
    u.set_dust_snap(true);
    u.set_dust_blank(false);
    let a = snap(u);
    u.set_dust_blank(true);
    let b = snap(u);
    u.set_dust_blank(blank);
    u.set_dust_snap(false);
    u.set_dust_on(on);
    let (a, b) = (a?, b?);
    let (w, h) = (a.w, a.h);
    if b.w != w || b.h != h {
        return None;
    }
    let mask: Vec<bool> = a
        .px
        .iter()
        .zip(&b.px)
        .map(|(p, q)| (p[0] as i32 - q[0] as i32).abs() + (p[1] as i32 - q[1] as i32).abs() + (p[2] as i32 - q[2] as i32).abs() > 6)
        .collect();
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
    let vgap = ((3.0 * s).round() as usize).max(2);
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
            // Rangées voisines jusqu'à `vgap` au-dessus : les accents, points et apostrophes,
            // séparés de leur lettre par un ou deux pixels, rejoignent sa ligne.
            for d in 1..=vgap.min(y) {
                for (k, &(pl, pr)) in runs[y - d].iter().enumerate() {
                    if pl < r && l < pr {
                        let (a, b) = (find(&mut parent, ids[y - d][k]), find(&mut parent, id));
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
        .filter(|&(x0, y0, x1, y1)| (x1 - x0) * (y1 - y0) >= 3 && ((y1 - y0) as f32) < 90.0 * s)
        .map(|(x0, y0, x1, y1)| Line { x: x0 as f32 / s, y: y0 as f32 / s, w: (x1 - x0) as f32 / s, h: (y1 - y0) as f32 / s })
        .collect();
    // Ordre de lecture : de haut en bas, puis de gauche à droite (lignes à peu près alignées).
    lines.sort_by(|a, b| ((a.y / 8.0) as i32, a.x as i32).cmp(&((b.y / 8.0) as i32, b.x as i32)));
    lines
}

/// Ouverture de la liste des langues : la barre recouvre les lignes de texte. `layer` : où les
/// blocs sont posés (1 Paramètres, 2 démarrage), sous la liste.
pub fn dissolve(u: &AppWindow, layer: i32) {
    finish(u);
    // Les textes sont repérés une fois l'écran posé : la liste vient de s'ouvrir, et ce qu'elle
    // change dessous s'anime encore (la ligne choisie perd son léger agrandissement…). Repérés
    // trop tôt, ces textes se décaleraient ensuite hors de leurs blocs.
    let weak = u.as_weak();
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::SingleShot, std::time::Duration::from_millis(SETTLE_MS), move || {
        if let Some(u) = weak.upgrade() {
            // Choix déjà fait ou liste fermée entre-temps (finish a vidé l'état) : rien à faire.
            if STATE.with(|s| s.borrow().is_some()) {
                STATE.with(|s| s.borrow_mut().take());
                cover(&u, layer);
            }
        }
    });
    STATE.with(|s| *s.borrow_mut() = Some(State { lines: Vec::new(), timer: Some(timer), done: None }));
}

/// Repère les lignes de texte et lance la barre qui les recouvre.
fn cover(u: &AppWindow, layer: i32) {
    let Some(lines) = find_lines(u) else { return };
    u.set_dust_lines(pairs_model(&lines.iter().map(|l| (*l, *l)).collect::<Vec<_>>()));
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
                // Tout est couvert : plus aucun texte n'est dessiné (police vide), même celui qui
                // apparaîtrait en agrandissant la fenêtre ; la liste des langues garde le sien.
                u.set_dust_blank(true);
            }
        }
    });
    STATE.with(|s| *s.borrow_mut() = Some(State { lines, timer: None, done: None }));
}

/// Applique une langue (code) : langue, textes venus de Rust, liste des langues.
pub type Apply = Rc<dyn Fn(&AppWindow, &str)>;

/// Blocs pour une liste de paires (départ, arrivée) ; la barre part dans l'ordre de la liste.
fn pairs_model(pairs: &[(Line, Line)]) -> ModelRc<DustLine> {
    let n = pairs.len().max(1) as i64;
    let step = (STAGGER_MAX_MS / n).min(28);
    let rows: Vec<DustLine> = pairs
        .iter()
        .enumerate()
        .map(|(i, (f, t))| DustLine { x: t.x, y: t.y, w: t.w, h: t.h, fx: f.x, fw: f.w, delay: i as i64 * step })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

/// Ligne d'avant qui correspond à `l` : même rangée, la plus proche à gauche.
fn matching<'a>(old: &'a [Line], l: &Line) -> Option<&'a Line> {
    old.iter()
        .filter(|o| o.y < l.y + l.h && l.y < o.y + o.h)
        .min_by(|a, b| (a.x - l.x).abs().total_cmp(&(b.x - l.x).abs()))
}

fn union(a: &Line, b: &Line) -> Line {
    let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
    Line { x: x0, y: y0, w: (a.x + a.w).max(b.x + b.w) - x0, h: (a.y + a.h).max(b.y + b.h) - y0 }
}

/// Choix fait (`change` : code de la langue et comment l'appliquer) ou annulé (None).
/// Choix : les blocs s'élargissent d'abord pour couvrir aussi la place des nouveaux mots (l'ancienne
/// langue reste affichée dessous, rien ne dépasse), la langue change, puis ils se resserrent sur les
/// nouveaux mots et la barre les dévoile. Annulation : la barre dévoile les anciens mots.
/// `done` à la fin (tout de suite s'il n'y a pas d'effet en cours).
pub fn reform(u: &AppWindow, change: Option<(String, Apply)>, done: impl FnOnce(&AppWindow) + 'static) {
    let st = STATE.with(|s| s.borrow_mut().take());
    let Some(st) = st.filter(|_| u.get_dust_cloud()) else {
        finish(u);
        if let Some((code, apply)) = change {
            apply(u, &code);
            bump(u);
        }
        done(u);
        return;
    };
    u.set_dust_cloud(false);
    let Some((code, apply)) = change else {
        // Annulation : les anciens mots sont dévoilés.
        let pairs: Vec<(Line, Line)> = st.lines.iter().map(|l| (*l, *l)).collect();
        u.set_dust_lines(pairs_model(&pairs));
        u.set_dust_blank(false);
        u.invoke_dust_start(3);
        schedule_end(u, st.lines, pairs.len(), Box::new(done));
        return;
    };
    // Lignes de la nouvelle langue (rendue hors écran pour les captures), puis retour à l'ancienne
    // le temps que les blocs s'élargissent.
    let old_code = i18n::current();
    apply(u, &code);
    bump(u);
    let new = find_lines(u);
    apply(u, &old_code);
    bump(u);
    let Some(new) = new else {
        finish(u);
        apply(u, &code);
        bump(u);
        done(u);
        return;
    };
    let mut used = vec![false; st.lines.len()];
    let mut grow: Vec<(Line, Line)> = Vec::new();
    let mut shrink: Vec<(Line, Line)> = Vec::new();
    for l in &new {
        let prev = matching(&st.lines, l);
        if let Some(p) = prev {
            if let Some(i) = st.lines.iter().position(|o| std::ptr::eq(o, p)) {
                used[i] = true;
            }
        }
        let from = prev.copied().unwrap_or(*l);
        let big = union(&from, l);
        grow.push((from, big));
        shrink.push((big, *l));
    }
    // Lignes d'avant sans équivalent : restent couvertes jusqu'au changement de langue.
    for (o, u) in st.lines.iter().zip(&used) {
        if !u {
            grow.push((*o, *o));
        }
    }
    u.set_dust_lines(pairs_model(&grow));
    u.invoke_dust_start(4);
    let (apply_keep, code_keep) = (apply.clone(), code.clone());
    let weak = u.as_weak();
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::SingleShot, std::time::Duration::from_millis((MORPH_MS + 40) as u64), move || {
        let Some(u) = weak.upgrade() else { return };
        // Effet arrêté entre-temps (finish a déjà fait la suite) : rien à faire.
        let Some(d) = STATE.with(|s| s.borrow_mut().as_mut().and_then(|st| st.done.take())) else { return };
        // Tout est couvert : la langue change dessous, les textes réapparaissent sous les blocs,
        // qui se resserrent, puis la barre dévoile.
        apply(&u, &code);
        bump(&u);
        u.set_dust_blank(false);
        u.set_dust_lines(pairs_model(&shrink));
        u.invoke_dust_start(3);
        schedule_end(&u, new.clone(), shrink.len(), d);
    });
    // Arrêt pendant que les blocs s'élargissent : la nouvelle langue est appliquée quand même.
    let (apply2, code2) = (apply_keep, code_keep);
    let done: Box<dyn FnOnce(&AppWindow)> = Box::new(move |u: &AppWindow| {
        if i18n::current() != code2 {
            apply2(u, &code2);
            bump(u);
        }
        done(u);
    });
    STATE.with(|s| *s.borrow_mut() = Some(State { lines: st.lines, timer: Some(timer), done: Some(done) }));
}

/// Fin du dévoilement (phase 3) : `done` après le passage de la barre sur la dernière ligne.
fn schedule_end(u: &AppWindow, lines: Vec<Line>, n: usize, done: Box<dyn FnOnce(&AppWindow)>) {
    let n = n.max(1) as i64;
    let total = MORPH_MS + (STAGGER_MAX_MS / n).min(28) * n + SWEEP_MS + 40;
    let timer = slint::Timer::default();
    let weak = u.as_weak();
    timer.start(slint::TimerMode::SingleShot, std::time::Duration::from_millis(total as u64), move || {
        if let Some(u) = weak.upgrade() {
            finish(&u);
        }
    });
    STATE.with(|s| *s.borrow_mut() = Some(State { lines, timer: Some(timer), done: Some(done) }));
}

/// Arrête l'effet sans attendre (le texte reste tel quel) et fait ce qui était prévu à la fin.
pub fn finish(u: &AppWindow) {
    let st = STATE.with(|s| s.borrow_mut().take());
    u.set_dust_blank(false);
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

/// Fenêtre redimensionnée pendant le choix (textes recouverts) : les lignes sont repérées de
/// nouveau, une fois la taille posée, et les blocs suivent la nouvelle mise en page. En attendant,
/// les textes restent invisibles (police vide).
pub fn resized(u: &AppWindow) {
    if !u.get_dust_cloud() || !u.get_dust_blank() {
        return;
    }
    let weak = u.as_weak();
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::SingleShot, std::time::Duration::from_millis(220), move || {
        let Some(u) = weak.upgrade() else { return };
        if !u.get_dust_cloud() {
            return;
        }
        if let Some(lines) = find_lines(&u) {
            u.set_dust_lines(pairs_model(&lines.iter().map(|l| (*l, *l)).collect::<Vec<_>>()));
            STATE.with(|s| {
                if let Some(st) = s.borrow_mut().as_mut() {
                    st.lines = lines;
                }
            });
        }
    });
    RESIZE.with(|r| *r.borrow_mut() = Some(timer));
}

thread_local! {
    /// Repérage après redimensionnement en attente (remplacé à chaque nouveau changement de taille).
    static RESIZE: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
}
