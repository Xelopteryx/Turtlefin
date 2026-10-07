//! Langue de l'interface.
//!
//! Les textes sont écrits en français dans le code (langue source) ; les traductions sont dans
//! `lang/<langue>/LC_MESSAGES/turtlefin.po` (format gettext). Les mêmes fichiers servent :
//! - à l'interface Slint (`@tr("…")`, catalogue intégré à la compilation par build.rs) ;
//! - aux textes produits par Rust (`tr("…")`, `trf("… {} …", &[…])`), catalogue intégré ici.
//!
//! Ajouter une langue : copier `lang/en`, traduire les `msgstr`, l'ajouter à `LANGUAGES` et aux
//! `include_str!` de `catalog`.

use std::collections::HashMap;
use std::sync::RwLock;

/// Langues proposées : (code, nom dans sa langue).
pub const LANGUAGES: [(&str, &str); 2] = [("fr", "Français"), ("en", "English")];

// Traductions de la langue choisie. Les textes sont gardés pour toute la durée du programme
// (`&'static str`) : `tr` s'utilise partout où un texte littéral l'était.
static CURRENT: RwLock<Option<HashMap<String, &'static str>>> = RwLock::new(None);

fn catalog(code: &str) -> Option<&'static str> {
    match code {
        "en" => Some(include_str!("../lang/en/LC_MESSAGES/turtlefin.po")),
        _ => None,
    }
}

/// Lecture d'un fichier .po (msgid / msgstr, chaînes sur plusieurs lignes comprises).
fn parse_po(text: &str) -> HashMap<String, String> {
    fn unquote(s: &str) -> String {
        let s = s.trim();
        let s = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(s);
        let mut out = String::new();
        let mut it = s.chars();
        while let Some(c) = it.next() {
            if c == '\\' {
                match it.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some(o) => out.push(o),
                    None => {}
                }
            } else {
                out.push(c);
            }
        }
        out
    }
    let mut map = HashMap::new();
    let (mut id, mut st, mut field) = (String::new(), String::new(), 0);
    let mut flush = |id: &mut String, st: &mut String| {
        if !id.is_empty() && !st.is_empty() {
            map.insert(std::mem::take(id), std::mem::take(st));
        }
        id.clear();
        st.clear();
    };
    for line in text.lines() {
        let l = line.trim();
        if let Some(r) = l.strip_prefix("msgid ") {
            flush(&mut id, &mut st);
            id = unquote(r);
            field = 1;
        } else if let Some(r) = l.strip_prefix("msgstr ") {
            st = unquote(r);
            field = 2;
        } else if l.starts_with('"') {
            match field {
                1 => id.push_str(&unquote(l)),
                2 => st.push_str(&unquote(l)),
                _ => {}
            }
        } else if l.is_empty() {
            field = 0;
        }
    }
    flush(&mut id, &mut st);
    map
}

/// Vérification du démarrage : la langue est connue et ses traductions se lisent.
/// Renvoie le nombre de textes traduits (0 pour le français, langue du code).
pub fn check(code: &str) -> Result<usize, String> {
    if code == "fr" || code.is_empty() {
        return Ok(0);
    }
    if !LANGUAGES.iter().any(|(id, _)| *id == code) {
        return Err(format!("langue inconnue « {code} »"));
    }
    let n = catalog(code).map(|t| parse_po(t).values().filter(|v| !v.is_empty()).count()).unwrap_or(0);
    if n == 0 {
        Err(format!("traductions « {code} » absentes"))
    } else {
        Ok(n)
    }
}

/// Langue choisie à l'installation (Windows : l'installeur écrit « fr » ou « en » dans un fichier
/// `language` à côté de l'exécutable). `None` si absente ou inconnue.
pub fn installer_language() -> Option<String> {
    let text = std::fs::read_to_string(crate::paths::exe_dir()?.join("language")).ok()?;
    let code = text.trim().trim_start_matches('\u{feff}').to_lowercase();
    LANGUAGES.iter().any(|(id, _)| *id == code).then_some(code)
}

/// Langue du système (« fr », « en »...), pour présélectionner le choix au premier lancement.
pub fn system_language() -> String {
    let raw = std::env::var("LC_ALL").or_else(|_| std::env::var("LANG")).unwrap_or_default();
    let mut code = raw.split(['_', '.', '-']).next().unwrap_or("").to_lowercase();
    #[cfg(windows)]
    if code.is_empty() {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetUserDefaultUILanguage() -> u16;
        }
        // SAFETY : appel Win32 sans argument.
        let lang = unsafe { GetUserDefaultUILanguage() } & 0x3ff;
        code = if lang == 0x0c { "fr".into() } else { "en".into() };
    }
    if LANGUAGES.iter().any(|(c, _)| *c == code) {
        code
    } else {
        "en".into()
    }
}

/// Change la langue (interface Slint et textes Rust). À appeler après la création de la fenêtre.
pub fn set_language(code: &str) {
    *CURRENT.write().unwrap() =
        catalog(code).map(|t| parse_po(t).into_iter().map(|(k, v)| (k, &*Box::leak(v.into_boxed_str()))).collect());
    // Le français est la langue source de l'interface : catalogue « vide ». Slint ne change de
    // langue que sur le fil de l'interface : la demande y est envoyée (d'où qu'elle vienne).
    let slint_code = if code == "fr" { String::new() } else { code.to_string() };
    let code = code.to_string();
    let _ = slint::invoke_from_event_loop(move || {
        if let Err(e) = slint::select_bundled_translation(&slint_code) {
            eprintln!("turtlefin : langue {code} : {e:?}");
        }
    });
}

/// Texte traduit (le texte français lui-même s'il n'a pas de traduction).
pub fn tr(fr: &'static str) -> &'static str {
    CURRENT.read().unwrap().as_ref().and_then(|m| m.get(fr).copied()).unwrap_or(fr)
}

/// Texte traduit avec des valeurs à la place des `{}`, dans l'ordre.
pub fn trf(fr: &'static str, args: &[&dyn std::fmt::Display]) -> String {
    let t = tr(fr);
    let mut out = String::with_capacity(t.len() + 16);
    let mut it = args.iter();
    let mut rest = t;
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        if let Some(a) = it.next() {
            out.push_str(&a.to_string());
        }
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn po() {
        let m = super::parse_po("msgid \"\"\nmsgstr \"entête\"\n\nmsgid \"Bonjour {}\"\nmsgstr \"Hello {}\"\n\nmsgid \"a\"\n\"b\"\nmsgstr \"c\\\"d\"\n");
        assert_eq!(m.get("Bonjour {}").map(String::as_str), Some("Hello {}"));
        assert_eq!(m.get("ab").map(String::as_str), Some("c\"d"));
    }
}
