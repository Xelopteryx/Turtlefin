//! Thèmes : couleurs et effets de l'interface (global `Theme` de ui/theme.slint).
//!
//! Cinq thèmes intégrés (Turtlefin, Sombre, Clair, Frutiger Aero, Turtlefin vert) et des thèmes
//! personnels : fichiers `.tftheme` (JSON) importés depuis le dossier « Turtlefin Themes »
//! (à côté de « Turtlefin Languages »), gardés dans prefs.json. Un fichier est reconnu par sa clé
//! `turtlefin_theme` (version du format, 1). Couleurs : « #rrggbb » ou « #rrggbbaa » ; une couleur
//! absente ou illisible prend celle du thème Turtlefin (`text` et `muted` vides : tirées de `fg`).

use serde::{Deserialize, Serialize};
use slint::{Color, ComponentHandle};

use crate::AppWindow;

pub const THEMES_DIR_NAME: &str = "Turtlefin Themes";
pub const EXT: &str = "tftheme";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ThemeDef {
    /// Version du format (0 : pas un thème de Turtlefin ; absente d'un fichier : 0).
    #[serde(default)]
    pub turtlefin_theme: u32,
    pub name: String,
    /// Thème sombre (texte clair) ou clair (texte foncé).
    pub dark: bool,
    /// Fond : dégradé vertical, haut / milieu / bas.
    pub bg: String,
    pub bg_mid: String,
    pub bg_end: String,
    /// Encre : textes, icônes, surfaces translucides.
    pub fg: String,
    pub text: String,
    pub muted: String,
    pub accent1: String,
    pub accent2: String,
    /// Texte posé sur le dégradé d'accent.
    pub on_accent: String,
    pub glass: String,
    pub glass_border: String,
    pub panel: String,
    pub card: String,
    pub card_focus: String,
    pub placeholder1: String,
    pub placeholder2: String,
    pub scrim: String,
    pub danger: String,
    pub danger_text: String,
    pub ok: String,
    pub warn: String,
    /// Voile sur l'image de fond du média (1 : normal).
    pub veil: f32,
    /// Arrondi des éléments, en pixels.
    pub radius: f32,
    /// Reflet brillant (0 à 1,5).
    pub gloss: f32,
    /// Décor de bulles et de ciel.
    pub bubbles: bool,
}

impl Default for ThemeDef {
    fn default() -> Self {
        turtlefin()
    }
}

fn s(x: &str) -> String {
    x.to_string()
}

/// Thème par défaut : nuit bleutée, accents violet et bleu (JellySkin).
pub fn turtlefin() -> ThemeDef {
    ThemeDef {
        turtlefin_theme: 1,
        name: s("Turtlefin"),
        dark: true,
        bg: s("#010e18"),
        bg_mid: s("#010e18"),
        bg_end: s("#010e18"),
        fg: s("#ffffff"),
        text: s("#ffffffe6"),
        muted: s("#ffffff99"),
        accent1: s("#a95bc2"),
        accent2: s("#00a4db"),
        on_accent: s("#ffffff"),
        glass: s("#181820d9"),
        glass_border: s("#ffffff24"),
        panel: s("#0b1520f5"),
        card: s("#00000033"),
        card_focus: s("#00000080"),
        placeholder1: s("#2b2142"),
        placeholder2: s("#10283a"),
        scrim: s("#000000"),
        danger: s("#ff6b6b"),
        danger_text: s("#ff8a8a"),
        ok: s("#4ade80"),
        warn: s("#f59e0b"),
        veil: 1.0,
        radius: 10.0,
        gloss: 0.0,
        bubbles: false,
    }
}

/// Noir profond, neutre : pour les écrans OLED et le noir complet.
fn sombre() -> ThemeDef {
    ThemeDef {
        name: s("Sombre"),
        bg: s("#000000"),
        bg_mid: s("#050507"),
        bg_end: s("#0a0b0e"),
        text: s("#ffffffeb"),
        muted: s("#ffffff8c"),
        accent1: s("#6366f1"),
        accent2: s("#38bdf8"),
        glass: s("#121216e6"),
        glass_border: s("#ffffff1f"),
        panel: s("#0e0f12f7"),
        card: s("#ffffff0a"),
        card_focus: s("#ffffff1c"),
        placeholder1: s("#1d1e2a"),
        placeholder2: s("#11161d"),
        veil: 1.15,
        ..turtlefin()
    }
}

/// Clair : fond blanc bleuté, encre marine.
fn clair() -> ThemeDef {
    ThemeDef {
        name: s("Clair"),
        dark: false,
        bg: s("#f5f8fc"),
        bg_mid: s("#eef3f9"),
        bg_end: s("#e7eef6"),
        fg: s("#0f1a2a"),
        text: s("#0f1a2ae8"),
        muted: s("#0f1a2a99"),
        accent1: s("#8b3fd1"),
        accent2: s("#0284c7"),
        glass: s("#ffffffe0"),
        glass_border: s("#0f1a2a1f"),
        panel: s("#fbfcfef7"),
        card: s("#ffffffa6"),
        card_focus: s("#ffffff"),
        placeholder1: s("#ddd3ee"),
        placeholder2: s("#cde1ee"),
        scrim: s("#0f1a2a"),
        danger: s("#dc2626"),
        danger_text: s("#b91c1c"),
        ok: s("#16a34a"),
        warn: s("#d97706"),
        veil: 1.6,
        ..turtlefin()
    }
}

/// Frutiger Aero : ciel lumineux qui descend vers l'herbe, verre clair et brillant, boutons en
/// gel bleu, bulles. Encre bleu nuit sur le verre.
fn aero() -> ThemeDef {
    ThemeDef {
        name: s("Frutiger Aero"),
        dark: false,
        bg: s("#4fb8ee"),
        bg_mid: s("#c9efff"),
        bg_end: s("#a6e58a"),
        fg: s("#0b2f52"),
        text: s("#0b2f52f2"),
        muted: s("#0b2f52b0"),
        accent1: s("#0a5fc0"),
        accent2: s("#0a8bd3"),
        on_accent: s("#ffffff"),
        glass: s("#ffffff94"),
        glass_border: s("#ffffffe6"),
        panel: s("#effaffee"),
        card: s("#ffffff80"),
        card_focus: s("#ffffffd9"),
        placeholder1: s("#9fd8f5"),
        placeholder2: s("#bfeeb1"),
        scrim: s("#06345a"),
        danger: s("#e0313f"),
        danger_text: s("#b4162a"),
        ok: s("#23a047"),
        warn: s("#de8600"),
        veil: 2.0,
        radius: 14.0,
        gloss: 1.0,
        bubbles: true,
        ..turtlefin()
    }
}

/// Turtlefin vert : nuit des fonds marins, accents vert tortue et vert tendre.
fn vert() -> ThemeDef {
    ThemeDef {
        name: s("Turtlefin vert"),
        bg: s("#02140f"),
        bg_mid: s("#031b14"),
        bg_end: s("#04211a"),
        accent1: s("#1f9d5c"),
        accent2: s("#52c96b"),
        glass: s("#0d1f19e0"),
        glass_border: s("#ffffff21"),
        panel: s("#072019f5"),
        placeholder1: s("#173a2a"),
        placeholder2: s("#0f2c33"),
        ok: s("#86efac"),
        ..turtlefin()
    }
}

/// Thèmes intégrés : clé, thème.
pub fn builtin() -> Vec<(&'static str, ThemeDef)> {
    vec![("turtlefin", turtlefin()), ("sombre", sombre()), ("clair", clair()), ("aero", aero()), ("vert", vert())]
}

/// Thème choisi dans les réglages : clé intégrée, ou « custom:<nom> » (thème importé).
pub fn find(key: &str, custom: &[ThemeDef]) -> ThemeDef {
    if let Some(name) = key.strip_prefix("custom:") {
        if let Some(t) = custom.iter().find(|t| t.name == name) {
            return t.clone();
        }
    }
    builtin().into_iter().find(|(k, _)| *k == key).map(|(_, t)| t).unwrap_or_else(turtlefin)
}

/// « #rgb », « #rrggbb » ou « #rrggbbaa ».
pub fn parse(c: &str) -> Option<Color> {
    let h = c.trim().strip_prefix('#')?;
    let v = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok();
    match h.len() {
        3 => Some(Color::from_rgb_u8(v(0, 1)? * 17, v(1, 1)? * 17, v(2, 1)? * 17)),
        6 => Some(Color::from_rgb_u8(v(0, 2)?, v(2, 2)?, v(4, 2)?)),
        8 => Some(Color::from_argb_u8(v(6, 2)?, v(0, 2)?, v(2, 2)?, v(4, 2)?)),
        _ => None,
    }
}

/// Lit un fichier de thème (`None` : pas un thème de Turtlefin).
pub fn load(path: &std::path::Path) -> Option<ThemeDef> {
    let text = std::fs::read_to_string(path).ok()?;
    let t: ThemeDef = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    (t.turtlefin_theme >= 1 && !t.name.trim().is_empty()).then_some(t)
}

/// Applique un thème à l'interface.
pub fn apply(u: &AppWindow, t: &ThemeDef) {
    let d = turtlefin();
    let c = |x: &str, fallback: &str| parse(x).or_else(|| parse(fallback)).unwrap_or_default();
    let th = u.global::<crate::Theme>();
    let fg = c(&t.fg, if t.dark { "#ffffff" } else { "#0f1a2a" });
    let alpha = |a: f32| Color::from_argb_u8((a * 255.0) as u8, fg.red(), fg.green(), fg.blue());
    th.set_dark(t.dark);
    th.set_bg(c(&t.bg, &d.bg));
    th.set_bg_mid(parse(&t.bg_mid).unwrap_or_else(|| c(&t.bg, &d.bg)));
    th.set_bg_end(parse(&t.bg_end).unwrap_or_else(|| c(&t.bg, &d.bg)));
    th.set_fg(fg);
    th.set_text(parse(&t.text).unwrap_or_else(|| alpha(0.9)));
    th.set_muted(parse(&t.muted).unwrap_or_else(|| alpha(0.6)));
    th.set_accent1(c(&t.accent1, &d.accent1));
    th.set_accent2(c(&t.accent2, &d.accent2));
    th.set_on_accent(c(&t.on_accent, &d.on_accent));
    th.set_glass(c(&t.glass, &d.glass));
    th.set_glass_border(c(&t.glass_border, &d.glass_border));
    th.set_panel(c(&t.panel, &d.panel));
    th.set_card(c(&t.card, &d.card));
    th.set_card_focus(c(&t.card_focus, &d.card_focus));
    th.set_ph1(c(&t.placeholder1, &d.placeholder1));
    th.set_ph2(c(&t.placeholder2, &d.placeholder2));
    th.set_scrim(c(&t.scrim, &d.scrim));
    th.set_danger(c(&t.danger, &d.danger));
    th.set_danger_text(c(&t.danger_text, &d.danger_text));
    th.set_ok(c(&t.ok, &d.ok));
    th.set_warn(c(&t.warn, &d.warn));
    th.set_veil(if t.veil > 0.0 { t.veil.clamp(0.2, 3.0) } else { 1.0 });
    th.set_radius(if t.radius > 0.0 { t.radius.clamp(0.0, 30.0) } else { 10.0 });
    th.set_gloss(t.gloss.clamp(0.0, 1.5));
    th.set_bubbles(t.bubbles);
}

/// Dossier des thèmes personnels (à côté de celui des langues), créé au besoin.
pub fn dir() -> Option<std::path::PathBuf> {
    let d = crate::i18n::lang_dir()?.parent()?.join(THEMES_DIR_NAME);
    let _ = std::fs::create_dir_all(&d);
    Some(d)
}

/// Écrit un thème dans le dossier des thèmes (`<nom>.tftheme`), pour le partager ou s'en servir
/// de modèle.
pub fn export(t: &ThemeDef) -> std::io::Result<std::path::PathBuf> {
    let d = dir().ok_or_else(|| std::io::Error::other("dossier des thèmes introuvable"))?;
    let safe: String = t.name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' }).collect();
    let path = d.join(format!("{}.{EXT}", safe.trim()));
    let mut t = t.clone();
    t.turtlefin_theme = 1;
    std::fs::write(&path, serde_json::to_string_pretty(&t).unwrap_or_default())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn colors() {
        let c = super::parse("#0a6fd1").unwrap();
        assert_eq!((c.red(), c.green(), c.blue(), c.alpha()), (0x0a, 0x6f, 0xd1, 255));
        let c = super::parse("#ffffff80").unwrap();
        assert_eq!(c.alpha(), 0x80);
        assert!(super::parse("bleu").is_none());
    }

    #[test]
    fn file_round_trip() {
        let t = super::builtin().into_iter().find(|(k, _)| *k == "aero").unwrap().1;
        let json = serde_json::to_string(&t).unwrap();
        let back: super::ThemeDef = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
        // Fichier minimal : le reste vient du thème Turtlefin.
        let min: super::ThemeDef = serde_json::from_str(r##"{"turtlefin_theme": 1, "name": "X", "accent1": "#ff0000"}"##).unwrap();
        assert_eq!(min.accent2, "#00a4db");
    }
}
