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
    /// Contour des cartes, lueur de l'élément choisi, reflet en diagonale des affiches (0 à 1).
    pub card_border: String,
    pub glow: String,
    pub sheen: f32,
    /// Lecteur : barre de commandes, encre, bouton lecture en orbe.
    pub player_bar: String,
    pub player_ink: String,
    pub player_orb: bool,
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
    /// Décor de bulles et de ciel (ancien nom du décor « sky »).
    pub bubbles: bool,
    /// Décor : « » aucun, « sky » ciel, herbe et bulles, « harmony » gerbe de lumière (Windows 7).
    pub scenery: String,
    /// Barre de verre derrière l'en-tête.
    pub header_glass: bool,
    /// Liseré intérieur des éléments brillants (« #rrggbbaa », transparent : aucun).
    pub edge: String,
    /// Reflets en biais sur les grands panneaux.
    pub streaks: bool,
    /// Animation de démarrage : « » logo qui se relie, « aero » billes de verre qui tourbillonnent.
    pub boot: String,
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
        card_border: s("#00000000"),
        glow: s("#00a4db40"),
        sheen: 0.0,
        player_bar: s("#181820d9"),
        player_ink: s("#ffffff"),
        player_orb: false,
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
        scenery: String::new(),
        header_glass: false,
        edge: s("#00000000"),
        streaks: false,
        boot: String::new(),
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

/// Frutiger Aero, façon Windows 7 : fond « Harmony » (gerbe de lumière sur bleu profond), verre
/// bleuté à liseré clair et reflets en biais, sélection bleu-cyan brillante, barre de verre en haut,
/// lecteur à orbe, démarrage en billes de verre qui tourbillonnent.
fn aero() -> ThemeDef {
    ThemeDef {
        name: s("Frutiger Aero"),
        dark: true,
        bg: s("#062a63"),
        bg_mid: s("#0b4fa8"),
        bg_end: s("#05214f"),
        fg: s("#ffffff"),
        text: s("#ffffffeb"),
        muted: s("#d8e8ffb3"),
        accent1: s("#2a7fd8"),
        accent2: s("#6cc6ff"),
        on_accent: s("#ffffff"),
        glass: s("#7fb2e84d"),
        glass_border: s("#ffffff73"),
        panel: s("#0d2c58eb"),
        card: s("#ffffff1a"),
        card_focus: s("#ffffff38"),
        card_border: s("#ffffff4d"),
        glow: s("#7fd0ffb3"),
        sheen: 0.6,
        player_bar: s("#08121fd9"),
        player_ink: s("#ffffff"),
        player_orb: true,
        placeholder1: s("#1f5aa8"),
        placeholder2: s("#0d2f66"),
        scrim: s("#000814"),
        danger: s("#ff6b6b"),
        danger_text: s("#ffb0b0"),
        ok: s("#7ee08a"),
        warn: s("#ffc04d"),
        veil: 1.5,
        radius: 8.0,
        gloss: 1.0,
        scenery: s("harmony"),
        header_glass: true,
        edge: s("#ffffff73"),
        streaks: true,
        boot: s("aero"),
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
    th.set_card_border(c(&t.card_border, &d.card_border));
    th.set_glow(c(&t.glow, &d.glow));
    th.set_sheen(t.sheen.clamp(0.0, 1.0));
    th.set_player_bar(c(&t.player_bar, &d.player_bar));
    th.set_player_ink(c(&t.player_ink, &d.player_ink));
    th.set_player_orb(t.player_orb);
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
    let scenery = match t.scenery.as_str() {
        "sky" => 1,
        "harmony" => 2,
        _ if t.bubbles => 1,
        _ => 0,
    };
    th.set_bubbles(scenery > 0);
    th.set_scenery(scenery);
    th.set_header_glass(t.header_glass);
    th.set_edge(c(&t.edge, &d.edge));
    th.set_streaks(t.streaks);
    th.set_boot_style(if t.boot == "aero" { 1 } else { 0 });
    window_icon(u, t);
}

/// Turtlefin Theme Creator (tools/theme-creator.html), intégré au programme : déposé dans le
/// dossier des thèmes et ouvert dans le navigateur (aperçu en direct, export en .tftheme).
pub const CREATOR: &str = include_str!("../tools/theme-creator.html");
pub const CREATOR_FILE: &str = "Turtlefin Theme Creator.html";

pub fn write_creator() -> std::io::Result<std::path::PathBuf> {
    let d = dir().ok_or_else(|| std::io::Error::other("dossier des thèmes introuvable"))?;
    let path = d.join(CREATOR_FILE);
    std::fs::write(&path, CREATOR)?;
    Ok(path)
}

/// Logo de Turtlefin aux couleurs d'un thème (même dessin que packaging/turtlefin.svg) : carré
/// arrondi du fond du thème, carapace en dégradé d'accent, reflet brillant pour les thèmes à reflet.
/// Sert d'icône à la fenêtre (barre des tâches).
pub fn icon(t: &ThemeDef, n: u32) -> slint::SharedPixelBuffer<slint::Rgba8Pixel> {
    let d = turtlefin();
    let col = |x: &str, f: &str| parse(x).or_else(|| parse(f)).unwrap_or_default();
    let (bg1, bg2) = (col(&t.bg, &d.bg), parse(&t.bg_end).unwrap_or_else(|| col(&t.bg, &d.bg)));
    let (a1, a2) = (col(&t.accent1, &d.accent1), col(&t.accent2, &d.accent2));
    let lerp = |a: Color, b: Color, f: f32| {
        let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * f).round().clamp(0.0, 255.0);
        [m(a.red(), b.red()), m(a.green(), b.green()), m(a.blue(), b.blue())]
    };
    // Géométrie du logo (repère 256), agrandie x1,4 autour du centre.
    let outer = [(128.0, 60.0), (187.0, 94.0), (187.0, 162.0), (128.0, 196.0), (69.0, 162.0), (69.0, 94.0)];
    let inner = [(128.0, 98.0), (154.0, 113.0), (154.0, 143.0), (128.0, 158.0), (102.0, 143.0), (102.0, 113.0)];
    fn seg(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let h = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        ((p.0 - a.0 - dx * h).powi(2) + (p.1 - a.1 - dy * h).powi(2)).sqrt()
    }
    fn inside(p: (f32, f32), poly: &[(f32, f32)]) -> bool {
        let mut c = false;
        let mut j = poly.len() - 1;
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[j]);
            if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
                c = !c;
            }
            j = i;
        }
        c
    }
    let px = 256.0 / n as f32; // taille d'un pixel dans le repère 256
    let cov = |dist: f32| (0.5 - dist / px).clamp(0.0, 1.0);
    let gloss = t.gloss.clamp(0.0, 1.5);
    let mut buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(n, n);
    for (i, p) in buf.make_mut_slice().iter_mut().enumerate() {
        let (x, y) = ((i as u32 % n) as f32 + 0.5, (i as u32 / n) as f32 + 0.5);
        let (sx, sy) = (x * px, y * px);
        // Carré arrondi (rayon 56).
        let (qx, qy) = ((sx - 128.0).abs() - 72.0, (sy - 128.0).abs() - 72.0);
        let rd = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - 56.0;
        let bga = cov(rd);
        let mut c = lerp(bg1, bg2, sy / 256.0);
        // Logo
        let l = (128.0 + (sx - 128.0) / 1.4, 128.0 + (sy - 128.0) / 1.4);
        let mut dist = f32::MAX;
        for k in 0..6 {
            dist = dist.min(seg(l, outer[k], outer[(k + 1) % 6]) - 7.0);
            dist = dist.min(seg(l, outer[k], inner[k]) - 4.0);
        }
        if inside(l, &inner) {
            dist = dist.min(-1.0);
        } else {
            for k in 0..6 {
                dist = dist.min(seg(l, inner[k], inner[(k + 1) % 6]));
            }
        }
        let la = cov(dist * 1.4);
        let g = lerp(a1, a2, (((l.0 - 53.0) + (l.1 - 53.0)) / 300.0).clamp(0.0, 1.0));
        for ch in 0..3 {
            c[ch] = c[ch] * (1.0 - la) + g[ch] * la;
        }
        // Reflet : moitié haute plus claire.
        if gloss > 0.0 && sy < 128.0 {
            let w = 0.28 * gloss * (1.0 - sy / 128.0 * 0.6);
            for ch in 0..3 {
                c[ch] = c[ch] * (1.0 - w) + 255.0 * w;
            }
        }
        *p = slint::Rgba8Pixel { r: c[0] as u8, g: c[1] as u8, b: c[2] as u8, a: (bga * 255.0) as u8 };
    }
    buf
}

/// Icône de la fenêtre (et de la barre des tâches sous Windows) aux couleurs du thème. Posée
/// directement sur la fenêtre : Slint ne transmet pas une icône dessinée en mémoire (elle n'a pas de
/// clé de cache). Sans fenêtre encore créée, rien ne se passe : main la repose une fois la fenêtre là.
pub fn window_icon(u: &AppWindow, t: &ThemeDef) {
    use slint::winit_030::{winit, WinitWindowAccessor};
    let make = |n: u32| {
        let b = icon(t, n);
        winit::window::Icon::from_rgba(b.as_bytes().to_vec(), n, n).ok()
    };
    u.window().with_winit_window(|w| {
        w.set_window_icon(make(64));
        #[cfg(windows)]
        {
            use winit::platform::windows::WindowExtWindows;
            w.set_taskbar_icon(make(256));
        }
    });
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
    /// `TURTLEFIN_ICON_DUMP=<dossier>` : écrit le logo de chaque thème intégré en PNG (vérification).
    #[test]
    fn icon_dump() {
        let Some(dir) = std::env::var_os("TURTLEFIN_ICON_DUMP") else { return };
        for (k, t) in super::builtin() {
            let b = super::icon(&t, 128);
            let img = image::RgbaImage::from_raw(b.width(), b.height(), b.as_bytes().to_vec()).unwrap();
            img.save(std::path::Path::new(&dir).join(format!("icon_{k}.png"))).unwrap();
        }
    }

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
