//! Turtlefin Theme Creator : fenêtre à part, lancée par `turtlefin --theme-creator` (raccourci
//! « Turtlefin Theme Creator », ou Paramètres → Affichage → « Créer un thème »).
//!
//! Tout ce qu'un thème peut régler : couleurs, formes, décor ou fond personnel (image, GIF, vidéo),
//! logo et son animation, animation de démarrage, musique de fond. L'aperçu est fait des briques de Turtlefin
//! (global `Theme`, décors, logo). « Enregistrer » écrit un dossier dans « Turtlefin Themes »
//! (`<nom>/<nom>.tftheme` et les fichiers du thème à côté) ; « Installer » l'ajoute aussi aux thèmes
//! de Turtlefin et le choisit.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::i18n::{tr, trf};
use crate::theme::{self, ThemeDef};
use crate::{CField, Theme, ThemeCreator};

#[derive(Clone, Copy)]
enum Kind {
    Head,
    Info,
    Text,
    Color,
    Bool,
    Num(f32, f32),
    /// (valeur dans le fichier, nom affiché)
    Choice(&'static [(&'static str, &'static str)]),
    /// Extensions acceptées.
    File(&'static [&'static str]),
    /// Fichier audio (musique de fond), avec un bouton pour l'écouter.
    Music,
}

struct Def {
    key: &'static str,
    label: &'static str,
    hint: &'static str,
    kind: Kind,
}

const fn d(key: &'static str, label: &'static str, hint: &'static str, kind: Kind) -> Def {
    Def { key, label, hint, kind }
}

const IMAGES: &[&str] = &["png", "jpg", "jpeg", "webp"];
const BACKGROUNDS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "mp4", "mkv", "webm", "mov", "m4v"];
const MUSIC: &[&str] = &["mp3", "ogg", "opus", "flac", "wav", "m4a"];

const TABS: [&str; 6] = ["Général", "Couleurs", "Formes", "Fond", "Logo", "Musique"];

fn defs(tab: usize) -> Vec<Def> {
    use Kind::*;
    match tab {
        0 => vec![
            d("name", "Nom du thème", "", Text),
            d("dark", "Thème sombre", "Texte clair sur fond sombre ; sinon texte foncé sur fond clair.", Bool),
            d("", "Les onglets règlent chaque partie du thème ; l'aperçu de droite suit en direct. « Enregistrer » écrit le thème dans le dossier « Turtlefin Themes » ; « Installer dans Turtlefin » l'ajoute en plus à la liste des thèmes et le choisit.", "", Info),
        ],
        1 => vec![
            d("", "Fond de la fenêtre", "", Head),
            d("bg", "Haut", "", Color),
            d("bg_mid", "Milieu", "", Color),
            d("bg_end", "Bas", "", Color),
            d("", "Texte", "", Head),
            d("fg", "Encre", "Icônes, traits, et base des surfaces translucides.", Color),
            d("text", "Texte", "", Color),
            d("muted", "Texte discret", "", Color),
            d("", "Accents", "", Head),
            d("accent1", "Accent 1", "Début du dégradé des éléments choisis.", Color),
            d("accent2", "Accent 2", "Fin du dégradé, contours, liens.", Color),
            d("on_accent", "Texte sur l'accent", "", Color),
            d("", "Surfaces", "", Head),
            d("glass", "Verre", "Panneaux et bulles.", Color),
            d("glass_border", "Contour du verre", "", Color),
            d("panel", "Panneau plein", "Menu, fenêtres.", Color),
            d("scrim", "Voile des fenêtres", "", Color),
            d("edge", "Liseré brillant", "Transparent : aucun.", Color),
            d("", "Affiches", "", Head),
            d("card", "Carte", "", Color),
            d("card_focus", "Carte choisie", "", Color),
            d("card_border", "Contour", "", Color),
            d("glow", "Lueur", "Autour de l'élément choisi.", Color),
            d("placeholder1", "Sans image 1", "", Color),
            d("placeholder2", "Sans image 2", "", Color),
            d("", "Lecteur", "", Head),
            d("player_bar", "Barre", "", Color),
            d("player_ink", "Encre", "", Color),
            d("", "États", "", Head),
            d("ok", "Réussite", "", Color),
            d("warn", "Attention", "", Color),
            d("danger", "Danger", "", Color),
            d("danger_text", "Texte d'erreur", "", Color),
        ],
        2 => vec![
            d("skin", "Style des boutons", "", Choice(&[("", "Plats"), ("win7", "Verre de Windows 7"), ("term", "Composants électroniques")])),
            d("font", "Police", "", Choice(&[("", "Montserrat"), ("selawik", "Selawik (Windows 7)"), ("mono", "JetBrains Mono")])),
            d("header", "Barre du haut", "", Choice(&[("", "Aucune"), ("glass", "Verre"), ("abyss", "Trait d'accent"), ("term", "Terminal")])),
            d("radius", "Arrondi", "", Num(0.0, 24.0)),
            d("gloss", "Reflet brillant", "", Num(0.0, 1.5)),
            d("sheen", "Reflet des affiches", "", Num(0.0, 1.0)),
            d("streaks", "Reflets en biais", "Sur les grands panneaux de verre.", Bool),
            d("player_orb", "Bouton lecture en orbe", "", Bool),
        ],
        3 => vec![
            d("scenery", "Décor", "", Choice(&[("", "Aucun"), ("abyss", "Abysses"), ("sky", "Ciel et bulles"), ("harmony", "Harmony (Windows 7)"), ("aqua", "Lagon (sous l'eau)"), ("circuit", "Circuit imprimé")])),
            d("background", "Fond personnel", "Image, GIF ou vidéo ; il remplace le décor.", File(BACKGROUNDS)),
            d("background_dim", "Voile sur le fond", "Plus haut : fond plus effacé, textes plus lisibles.", Num(0.0, 1.0)),
            d("veil", "Voile sur l'image du média", "", Num(0.5, 2.0)),
            d("", "Un fond animé (GIF, vidéo) demande plus de ressources : l'interface est redessinée à chaque image. Sur une petite machine, préfère une image. Dans Turtlefin, il ne bouge que si « Décor du thème qui bouge » est activé (Paramètres → Animations).", "", Info),
        ],
        4 => vec![
            d("logo", "Logo dessiné", "", Choice(&[("", "Carapace"), ("orb", "Orbe de verre"), ("chip", "Circuit")])),
            d("logo_file", "Logo personnel", "Image (PNG transparent conseillé) à la place du logo dessiné.", File(IMAGES)),
            d("logo_anim", "Animation du logo", "Jouée quand le logo personnel apparaît.", Choice(&[("", "Aucune"), ("pulse", "Battement"), ("spin", "Tour complet"), ("bounce", "Rebond"), ("fade", "Fondu")])),
            d("boot", "Animation de démarrage", "", Choice(&[("", "Logo qui se relie"), ("aero", "Billes de verre"), ("nerd", "Machine qui s'allume")])),
        ],
        _ => vec![
            d("music", "Musique de fond", "Jouée en boucle dans Turtlefin, coupée dès qu'un média est ouvert (même en pause).", Music),
            d("", "Chaque appareil peut la couper ou baisser son volume (Paramètres → Affichage). Choisis une musique que tu as le droit de partager si tu partages le thème.", "", Info),
        ],
    }
}

fn hex(c: slint::Color) -> String {
    if c.alpha() == 255 {
        format!("#{:02x}{:02x}{:02x}", c.red(), c.green(), c.blue())
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", c.red(), c.green(), c.blue(), c.alpha())
    }
}

/// Valeur d'un réglage du thème, en texte.
fn get(t: &ThemeDef, key: &str) -> String {
    if key == "skin" {
        return if t.win7 { "win7" } else if t.term { "term" } else { "" }.to_string();
    }
    match serde_json::to_value(t).ok().map(|v| v[key].clone()) {
        Some(serde_json::Value::String(s)) => s,
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Change un réglage du thème (texte, oui / non ou nombre selon le réglage).
fn set(t: &mut ThemeDef, key: &str, value: serde_json::Value) {
    if key == "skin" {
        let v = value.as_str().unwrap_or("");
        t.win7 = v == "win7";
        t.term = v == "term";
        return;
    }
    let Ok(mut v) = serde_json::to_value(&*t) else { return };
    v[key] = value;
    if let Ok(new) = serde_json::from_value::<ThemeDef>(v) {
        // `dir` n'est pas écrit dans le fichier (ni donc dans cette copie) : il est gardé.
        *t = ThemeDef { dir: t.dir.clone(), ..new };
    }
}

fn fields(t: &ThemeDef, tab: usize) -> Vec<CField> {
    defs(tab)
        .into_iter()
        .map(|def| {
            let value = get(t, def.key);
            let mut f = CField {
                key: def.key.into(),
                label: tr(def.label).into(),
                hint: if def.hint.is_empty() { SharedString::new() } else { tr(def.hint).into() },
                ..Default::default()
            };
            match def.kind {
                Kind::Head => f.kind = "head".into(),
                Kind::Info => f.kind = "info".into(),
                Kind::Text => {
                    f.kind = "text".into();
                    f.text = value.into();
                }
                Kind::Color => {
                    f.kind = "color".into();
                    f.col = theme::parse(&value).unwrap_or_default();
                    f.text = value.into();
                }
                Kind::Bool => {
                    f.kind = "bool".into();
                    f.on = value == "true";
                }
                Kind::Num(min, max) => {
                    f.kind = "num".into();
                    f.num = value.parse().unwrap_or(min);
                    f.min = min;
                    f.max = max;
                    f.text = format!("{:.2}", f.num).trim_end_matches('0').trim_end_matches('.').to_string().into();
                }
                Kind::Choice(opts) => {
                    f.kind = "choice".into();
                    f.text = tr(opts.iter().find(|(v, _)| *v == value).map(|(_, l)| *l).unwrap_or(opts[0].1)).into();
                }
                Kind::File(_) | Kind::Music => {
                    f.kind = if matches!(def.kind, Kind::Music) { "sound" } else { "file" }.into();
                    f.text = std::path::Path::new(&value).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().into();
                }
            }
            f
        })
        .collect()
}

struct State {
    theme: ThemeDef,
    tab: usize,
    /// Fond et logo déjà chargés (les images ne sont relues que s'ils changent).
    assets: (String, String),
    /// Musique en cours d'écoute (bouton ▶).
    listening: bool,
    /// Essai dans Turtlefin lancé : son fichier est réécrit à chaque changement (l'essai le suit).
    trial: Option<std::path::PathBuf>,
}

/// Fichier du thème pour l'essai dans Turtlefin : ses fichiers (fond, logo, musique) en chemins complets.
fn write_trial(t: &ThemeDef, path: &std::path::Path) -> std::io::Result<()> {
    let mut out = t.clone();
    let full = |f: &str| theme::asset(t, f).map(|p| p.display().to_string()).unwrap_or_default();
    out.background = full(&t.background);
    out.logo_file = full(&t.logo_file);
    out.music = full(&t.music);
    out.turtlefin_theme = 1;
    if out.name.trim().is_empty() {
        out.name = tr("Mon thème").to_string();
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    // Écrit d'un coup (fichier provisoire puis renommage) : l'essai ne lit jamais un fichier à moitié écrit.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&out).unwrap_or_default())?;
    std::fs::rename(&tmp, path)
}

/// Pose le thème sur la fenêtre (aperçu), ses sons, et met les lignes à jour sur place (un champ en
/// cours de saisie garde le clavier).
fn refresh(ui: &ThemeCreator, st: &mut State, rebuild: bool) {
    theme::apply_tokens(&ui.global::<Theme>(), &st.theme);
    crate::music::set_theme(if st.listening { theme::music_path(&st.theme) } else { None });
    if let Some(p) = &st.trial {
        let _ = write_trial(&st.theme, p);
    }
    let assets = (st.theme.background.clone(), st.theme.logo_file.clone());
    if assets != st.assets {
        st.assets = assets;
        theme::load_assets(ui.as_weak(), &st.theme, true);
    }
    let rows = fields(&st.theme, st.tab);
    let model = ui.get_fields();
    if rebuild || model.row_count() != rows.len() {
        ui.set_fields(ModelRc::new(VecModel::from(rows)));
    } else {
        for (i, r) in rows.into_iter().enumerate() {
            if model.row_data(i).as_ref() != Some(&r) {
                model.set_row_data(i, r);
            }
        }
    }
}

fn pick(title: &str, exts: &[&str]) -> Option<String> {
    rfd::FileDialog::new().add_filter(title, exts).pick_file().map(|p| p.display().to_string())
}

pub fn run() -> anyhow::Result<()> {
    let prefs = crate::config::ui_prefs();
    if !prefs.language.is_empty() {
        crate::i18n::set_language(&prefs.language);
    }
    let ui = ThemeCreator::new()?;
    // En haut à gauche de l'écran : sur un écran de 768 px de haut, la place choisie par le système
    // envoyait les boutons du bas sous la barre des tâches.
    ui.window().set_position(slint::PhysicalPosition::new(24, 16));
    // Part du thème en cours dans Turtlefin ; un thème intégré devient « Mon thème ».
    let mut start = theme::find(&prefs.theme, &prefs.themes);
    if !prefs.theme.starts_with("custom:") {
        start.name = tr("Mon thème").to_string();
    }
    crate::music::configure(true, 60);
    ui.set_tabs(ModelRc::new(VecModel::from(TABS.iter().map(|t| SharedString::from(tr(t))).collect::<Vec<_>>())));
    let texts = [
        "Partir de :", "Turtlefin", "Frutiger Aero", "Nerd", "Ouvrir…", "Enregistrer", "Installer dans Turtlefin", "Parcourir…", "Retirer",
        "Aucun fichier", "Aperçu", "Accueil", "Favoris", "Collections", "Reprendre", "Lecture", "Favori", "Vu", "Réglages", "Sous-titres",
        "Essayer dans Turtlefin",
    ];
    ui.set_ui(ModelRc::new(VecModel::from(texts.iter().map(|t| SharedString::from(tr(t))).collect::<Vec<_>>())));
    let st = Rc::new(RefCell::new(State { theme: start, tab: 0, assets: (String::new(), String::new()), listening: false, trial: None }));
    refresh(&ui, &mut st.borrow_mut(), true);

    let weak = ui.as_weak();
    // Petit raccourci : la fenêtre et l'état, dans chaque rappel.
    macro_rules! with {
        (|$u:ident, $s:ident $(, $arg:ident)*| $body:block) => {{
            let (weak, st) = (weak.clone(), st.clone());
            move |$($arg),*| {
                let Some($u) = weak.upgrade() else { return };
                let mut guard = st.borrow_mut();
                let $s = &mut *guard;
                $body
            }
        }};
    }
    ui.on_tab_changed(with!(|u, s, i| {
        s.tab = i.max(0) as usize;
        refresh(&u, s, true);
    }));
    ui.on_edit_text(with!(|u, s, key, text| {
        let is_color = defs(s.tab).iter().any(|d| d.key == key.as_str() && matches!(d.kind, Kind::Color));
        if is_color && theme::parse(text.as_str()).is_none() {
            u.set_status(trf("« {} » n'est pas une couleur : écris #rrggbb ou #rrggbbaa.", &[&text]).into());
            return;
        }
        set(&mut s.theme, key.as_str(), serde_json::Value::String(text.to_string()));
        u.set_status(SharedString::new());
        refresh(&u, s, false);
    }));
    ui.on_edit_color(with!(|u, s, key, col| {
        set(&mut s.theme, key.as_str(), serde_json::Value::String(hex(col)));
        refresh(&u, s, false);
    }));
    ui.on_edit_bool(with!(|u, s, key, on| {
        set(&mut s.theme, key.as_str(), serde_json::Value::Bool(on));
        refresh(&u, s, false);
    }));
    ui.on_edit_num(with!(|u, s, key, v| {
        set(&mut s.theme, key.as_str(), serde_json::json!((v * 100.0).round() / 100.0));
        refresh(&u, s, false);
    }));
    ui.on_cycle(with!(|u, s, key, dir| {
        let Some(Kind::Choice(opts)) = defs(s.tab).iter().find(|d| d.key == key.as_str()).map(|d| d.kind) else { return };
        let cur = get(&s.theme, key.as_str());
        let i = opts.iter().position(|(v, _)| *v == cur).unwrap_or(0) as i32;
        let next = opts[(i + dir).rem_euclid(opts.len() as i32) as usize].0;
        set(&mut s.theme, key.as_str(), serde_json::Value::String(next.to_string()));
        refresh(&u, s, false);
    }));
    ui.on_browse(with!(|u, s, key| {
        let exts = match defs(s.tab).iter().find(|d| d.key == key.as_str()).map(|d| d.kind) {
            Some(Kind::File(e)) => e,
            Some(Kind::Music) => MUSIC,
            _ => return,
        };
        if let Some(path) = pick(tr("Fichiers acceptés"), exts) {
            set(&mut s.theme, key.as_str(), serde_json::Value::String(path));
            if key.as_str() == "background" && theme::background_kind(&s.theme) == 2 {
                u.set_status(tr("Vidéo choisie : elle demande plus de ressources qu'une image. « Essayer dans Turtlefin » la montre en mouvement.").into());
            }
            refresh(&u, s, false);
        }
    }));
    ui.on_clear_file(with!(|u, s, key| {
        set(&mut s.theme, key.as_str(), serde_json::Value::String(String::new()));
        refresh(&u, s, false);
    }));
    // ▶ : écoute la musique du thème (en boucle), un second appui l'arrête.
    ui.on_play_sound(with!(|u, s, _key| {
        s.listening = !s.listening && theme::music_path(&s.theme).is_some();
        refresh(&u, s, false);
    }));
    ui.on_action(with!(|u, s, what| {
        match what.as_str() {
            "open" => {
                let Some(path) = pick(tr("Thème de Turtlefin"), &[theme::EXT]) else { return };
                match theme::load(std::path::Path::new(&path)) {
                    Some(t) => {
                        s.theme = t;
                        u.set_status(SharedString::new());
                    }
                    None => u.set_status(tr("Ce fichier n'est pas un thème de Turtlefin.").into()),
                }
            }
            "save" | "install" => {
                if s.theme.name.trim().is_empty() {
                    u.set_status(tr("Donne d'abord un nom au thème (onglet Général).").into());
                    return;
                }
                match theme::export_package(&s.theme) {
                    Ok((path, saved)) => {
                        s.theme = saved.clone();
                        if what.as_str() == "install" {
                            let mut p = crate::config::ui_prefs();
                            p.themes.retain(|t| t.name != saved.name);
                            p.theme = format!("custom:{}", saved.name);
                            p.themes.push(saved);
                            crate::config::save_ui_prefs(&p);
                            u.set_status(tr("Thème installé et choisi : il s'applique au prochain lancement de Turtlefin.").into());
                        } else {
                            u.set_status(trf("Thème enregistré : {}", &[&path.display()]).into());
                        }
                    }
                    Err(e) => u.set_status(trf("Enregistrement impossible : {}", &[&e]).into()),
                }
            }
            "trial" => {
                // Le vrai Turtlefin, sur un faux serveur (voir demo.rs), avec ce thème ; il suit ensuite
                // chaque changement fait ici.
                let Some(path) = crate::paths::cache_dir().map(|d| d.join("theme-demo").join("essai.tftheme")) else { return };
                if let Err(e) = write_trial(&s.theme, &path) {
                    u.set_status(trf("Essai impossible : {}", &[&e]).into());
                    return;
                }
                let arg = format!("--theme-demo={}", path.display());
                match std::env::current_exe().and_then(|exe| crate::paths::quiet_command(exe).arg(arg).spawn()) {
                    Ok(_) => {
                        s.trial = Some(path);
                        u.set_status(tr("Essai lancé : Turtlefin s'ouvre sur un serveur de démonstration. Tes changements s'y appliquent en direct.").into());
                    }
                    Err(e) => u.set_status(trf("Essai impossible : {}", &[&e]).into()),
                }
                return;
            }
            new => {
                let key = new.strip_prefix("new:").unwrap_or("turtlefin");
                s.theme = theme::find(key, &[]);
                s.theme.name = tr("Mon thème").to_string();
                u.set_status(SharedString::new());
            }
        }
        s.assets = (String::from("?"), String::from("?"));
        refresh(&u, s, true);
    }));
    ui.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_reads_and_writes() {
        let mut t = theme::turtlefin();
        for tab in 0..TABS.len() {
            for def in defs(tab) {
                match def.kind {
                    Kind::Color => {
                        set(&mut t, def.key, serde_json::Value::String("#102030".into()));
                        assert_eq!(get(&t, def.key), "#102030", "{}", def.key);
                    }
                    Kind::Bool => {
                        set(&mut t, def.key, serde_json::Value::Bool(true));
                        assert_eq!(get(&t, def.key), "true", "{}", def.key);
                    }
                    Kind::Num(min, _) => {
                        set(&mut t, def.key, serde_json::json!(min));
                        assert!(get(&t, def.key).parse::<f32>().is_ok(), "{}", def.key);
                    }
                    Kind::Choice(opts) => {
                        for (v, _) in opts {
                            set(&mut t, def.key, serde_json::Value::String(v.to_string()));
                            assert_eq!(get(&t, def.key), *v, "{}", def.key);
                        }
                    }
                    Kind::Text | Kind::File(_) | Kind::Music => {
                        set(&mut t, def.key, serde_json::Value::String("x".into()));
                        assert_eq!(get(&t, def.key), "x", "{}", def.key);
                    }
                    Kind::Head | Kind::Info => {}
                }
            }
        }
        assert_eq!(hex(slint::Color::from_argb_u8(255, 1, 2, 3)), "#010203");
        assert_eq!(hex(slint::Color::from_argb_u8(128, 1, 2, 3)), "#01020380");
    }
}
