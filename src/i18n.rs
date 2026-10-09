//! Langue de l'interface.
//!
//! Les textes sont écrits en français dans le code (langue source). Leurs traductions sont des
//! fichiers gettext `.po` :
//! - langues intégrées : `lang/<code>.po`, compilées dans le programme (`BUILTIN`) ;
//! - langues ajoutées par l'utilisateur, sans rien compiler : tout fichier `<code>.po` posé dans le
//!   dossier `languages` de la configuration (ou à côté de l'exécutable). Paramètres → Affichage →
//!   « Ajouter une langue » y écrit un modèle (`modele.po`) et ouvre le dossier.
//!
//! Le même catalogue sert à Rust (`tr`, `trf`) et à l'interface Slint (global `Tr`, branché dans
//! main.rs) : changer de langue se fait à chaud, sans redémarrer.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

/// Langues intégrées : (code, nom dans sa langue, fichier .po). Le français est la langue du code.
const BUILTIN: [(&str, &str, &str); 8] = [
    ("fr", "Français", ""),
    ("en", "English", include_str!("../lang/en.po")),
    ("es", "Español", include_str!("../lang/es.po")),
    ("de", "Deutsch", include_str!("../lang/de.po")),
    ("it", "Italiano", include_str!("../lang/it.po")),
    ("pt", "Português", include_str!("../lang/pt.po")),
    ("pl", "Polski", include_str!("../lang/pl.po")),
    ("nl", "Nederlands", include_str!("../lang/nl.po")),
];

/// Une langue chargée : textes traduits (formes plurielles comprises) et règle du pluriel.
struct Catalog {
    strs: HashMap<String, Vec<&'static str>>,
    plural: Plural,
}

static CURRENT: RwLock<Option<Catalog>> = RwLock::new(None);
static CODE: RwLock<String> = RwLock::new(String::new());
/// Prévenu à chaque changement de langue (main.rs : l'interface recalcule ses textes).
static ON_CHANGE: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

pub fn on_change(f: impl Fn() + Send + Sync + 'static) {
    let _ = ON_CHANGE.set(Box::new(f));
}

/// Nom du dossier des langues ajoutées (le même dans toutes les langues).
pub const LANG_DIR_NAME: &str = "Turtlefin Languages";

/// Dossier des langues ajoutées, le seul où elles sont cherchées : dans Documents (version
/// installée), à côté de l'exécutable (version portable), dans la configuration d'essai
/// (`TURTLEFIN_CONFIG_DIR`). Créé au besoin ; les langues d'avant (dossier `languages` de la
/// configuration) y sont déplacées.
pub fn lang_dir() -> Option<PathBuf> {
    let dir = if std::env::var_os("TURTLEFIN_CONFIG_DIR").is_some() {
        crate::paths::config_dir()?.join(LANG_DIR_NAME)
    } else if crate::paths::portable_root().is_some() {
        crate::paths::exe_dir()?.join(LANG_DIR_NAME)
    } else {
        directories::UserDirs::new()
            .and_then(|u| u.document_dir().map(|d| d.join(LANG_DIR_NAME)))
            .or_else(|| crate::paths::config_dir().map(|d| d.join(LANG_DIR_NAME)))?
    };
    static MIGRATED: OnceLock<()> = OnceLock::new();
    MIGRATED.get_or_init(|| {
        let _ = std::fs::create_dir_all(&dir);
        let old = [crate::paths::config_dir(), crate::paths::exe_dir()].into_iter().flatten().map(|d| d.join("languages"));
        for o in old {
            for e in std::fs::read_dir(&o).into_iter().flatten().flatten() {
                let p = e.path();
                let name = p.file_name().map(|n| n.to_os_string()).unwrap_or_default();
                if p.extension().and_then(|s| s.to_str()) == Some("po") && !dir.join(&name).exists() {
                    let _ = std::fs::rename(&p, dir.join(&name)).or_else(|_| std::fs::copy(&p, dir.join(&name)).map(|_| ()));
                }
            }
        }
    });
    Some(dir)
}

/// Fichiers .po d'un dossier (sous-dossiers compris, sur `depth` niveaux).
fn po_files(dir: &std::path::Path, depth: u32, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            if depth > 0 {
                po_files(&p, depth - 1, out);
            }
        } else if p.extension().and_then(|s| s.to_str()) == Some("po") {
            out.push(p);
        }
    }
}

/// Le fichier est-il le modèle de traduction (pas une langue) ?
fn is_template(p: &std::path::Path) -> bool {
    matches!(p.file_stem().and_then(|s| s.to_str()).map(str::to_lowercase).as_deref(), Some("modele" | "template"))
}

/// Langues ajoutées : (code, chemin). Un fichier peut remplacer une langue intégrée (même code).
fn custom_files() -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    let Some(dir) = lang_dir() else { return out };
    let mut files = Vec::new();
    po_files(&dir, 3, &mut files);
    for p in files {
        if is_template(&p) {
            continue;
        }
        let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else { continue };
        let code = stem.to_lowercase();
        if !out.iter().any(|(c, _)| *c == code) && std::fs::read_to_string(&p).is_ok_and(|t| is_turtlefin_po(&t)) {
            out.push((code, p));
        }
    }
    out
}

/// Élément de l'explorateur des langues.
pub struct Entry {
    pub path: PathBuf,
    /// "dir" · "lang" (traduction de Turtlefin) · "template" (modèle) · "other" (autre .po)
    pub kind: &'static str,
    pub name: String,
    /// Code de la langue (fichier), ou nombre de fichiers .po (dossier).
    pub code: String,
    /// Part traduite (0 à 1), pour une langue.
    pub done: f32,
}

/// Contenu d'un dossier de l'explorateur : sous-dossiers puis fichiers .po, par nom.
pub fn browse(dir: &std::path::Path) -> Vec<Entry> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    let total = po_text("en").map(|t| parse_po(&t).len().saturating_sub(1)).unwrap_or(1).max(1);
    for p in entries {
        let fname = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if fname.starts_with('.') {
            continue;
        }
        if p.is_dir() {
            let mut inner = Vec::new();
            po_files(&p, 2, &mut inner);
            dirs.push(Entry { path: p, kind: "dir", name: fname, code: inner.len().to_string(), done: 0.0 });
        } else if p.extension().and_then(|s| s.to_str()) == Some("po") {
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            let code = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            if is_template(&p) {
                files.push(Entry { path: p, kind: "template", name: fname, code, done: 0.0 });
            } else if is_turtlefin_po(&text) {
                let filled = parse_po(&text).iter().filter(|(k, v)| !k.is_empty() && v.first().is_some_and(|s| !s.is_empty())).count();
                let name = header(&text, "X-Language-Name").filter(|n| !n.trim().is_empty()).unwrap_or_else(|| code.clone());
                files.push(Entry { path: p, kind: "lang", name, code, done: (filled as f32 / total as f32).min(1.0) });
            } else {
                files.push(Entry { path: p, kind: "other", name: fname, code, done: 0.0 });
            }
        }
    }
    dirs.extend(files);
    dirs
}

/// Texte d'un fichier .po (langue ajoutée en priorité, sinon intégrée).
fn po_text(code: &str) -> Option<String> {
    if let Some((_, p)) = custom_files().into_iter().find(|(c, _)| c == code) {
        if let Ok(t) = std::fs::read_to_string(p) {
            return Some(t);
        }
    }
    BUILTIN.iter().find(|(c, _, t)| *c == code && !t.is_empty()).map(|(_, _, t)| t.to_string())
}

/// Langues proposées : (code, nom). Intégrées, puis ajoutées (nom lu dans l'en-tête du fichier :
/// `X-Language-Name`, sinon le code).
pub fn languages() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = BUILTIN.iter().map(|(c, n, _)| (c.to_string(), n.to_string())).collect();
    for (code, path) in custom_files() {
        let name = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| header(&t, "X-Language-Name"))
            .unwrap_or_else(|| code.clone());
        match v.iter_mut().find(|(c, _)| *c == code) {
            Some(e) => e.1 = name,
            None => v.push((code, name)),
        }
    }
    v
}

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

/// Lecture d'un fichier .po : msgid → [msgstr] ou [msgstr[0], msgstr[1]…]. L'en-tête (msgid "")
/// est gardé sous la clé "". Les traductions vides sont ignorées (le texte français reste).
fn parse_po(text: &str) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    let mut id = String::new();
    let mut strs: Vec<String> = Vec::new();
    // 1 msgid · 2 msgid_plural · 3 msgstr (dernier de `strs`)
    let mut field = 0;
    let mut flush = |id: &mut String, strs: &mut Vec<String>| {
        if strs.iter().any(|s| !s.is_empty()) {
            map.insert(std::mem::take(id), std::mem::take(strs));
        }
        id.clear();
        strs.clear();
    };
    for line in text.trim_start_matches('\u{feff}').lines() {
        let l = line.trim();
        if let Some(r) = l.strip_prefix("msgid_plural ") {
            let _ = r;
            field = 2;
        } else if let Some(r) = l.strip_prefix("msgid ") {
            flush(&mut id, &mut strs);
            id = unquote(r);
            field = 1;
        } else if let Some(r) = l.strip_prefix("msgstr") {
            // « msgstr "…" » ou « msgstr[n] "…" »
            let r = r.trim_start();
            let r = if r.starts_with('[') { r.split_once(']').map(|(_, x)| x).unwrap_or(r) } else { r };
            strs.push(unquote(r));
            field = 3;
        } else if l.starts_with('"') {
            match field {
                1 => id.push_str(&unquote(l)),
                3 => {
                    if let Some(s) = strs.last_mut() {
                        s.push_str(&unquote(l));
                    }
                }
                _ => {}
            }
        } else if l.is_empty() {
            field = 0;
        }
    }
    flush(&mut id, &mut strs);
    map
}

/// Valeur d'un champ de l'en-tête du .po (« Plural-Forms », « X-Language-Name »…).
fn header(text: &str, key: &str) -> Option<String> {
    let h = parse_po(text).remove("")?.into_iter().next()?;
    h.lines()
        .find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix(':')))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Règle du pluriel de gettext (`plural=` de « Plural-Forms »), évaluée pour n. Accepte les
/// expressions habituelles : n, nombres, + - * / %, comparaisons, && || !, ? :, parenthèses.
#[derive(Clone)]
struct Plural(String);

impl Plural {
    fn from_header(h: Option<String>) -> Plural {
        let e = h
            .as_deref()
            .and_then(|h| h.split(';').find_map(|p| p.trim().strip_prefix("plural=")))
            .unwrap_or("n != 1");
        Plural(e.trim().to_string())
    }

    fn index(&self, n: i64) -> usize {
        let toks: Vec<char> = self.0.chars().filter(|c| !c.is_whitespace()).collect();
        let mut p = Expr { t: &toks, i: 0, n };
        let v = p.ternary();
        v.max(0) as usize
    }
}

struct Expr<'a> {
    t: &'a [char],
    i: usize,
    n: i64,
}

impl Expr<'_> {
    fn peek(&self, s: &str) -> bool {
        s.chars().enumerate().all(|(k, c)| self.t.get(self.i + k) == Some(&c))
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.peek(s) {
            self.i += s.len();
            true
        } else {
            false
        }
    }
    fn ternary(&mut self) -> i64 {
        let c = self.or();
        if self.eat("?") {
            let a = self.ternary();
            self.eat(":");
            let b = self.ternary();
            if c != 0 { a } else { b }
        } else {
            c
        }
    }
    fn or(&mut self) -> i64 {
        let mut v = self.and();
        while self.eat("||") {
            let r = self.and();
            v = (v != 0 || r != 0) as i64;
        }
        v
    }
    fn and(&mut self) -> i64 {
        let mut v = self.cmp();
        while self.eat("&&") {
            let r = self.cmp();
            v = (v != 0 && r != 0) as i64;
        }
        v
    }
    fn cmp(&mut self) -> i64 {
        let mut v = self.add();
        loop {
            let op = ["==", "!=", "<=", ">=", "<", ">"].into_iter().find(|o| self.peek(o));
            let Some(op) = op else { return v };
            self.eat(op);
            let r = self.add();
            v = match op {
                "==" => v == r,
                "!=" => v != r,
                "<=" => v <= r,
                ">=" => v >= r,
                "<" => v < r,
                _ => v > r,
            } as i64;
        }
    }
    fn add(&mut self) -> i64 {
        let mut v = self.mul();
        loop {
            if self.eat("+") {
                v += self.mul();
            } else if self.peek("-") {
                self.i += 1;
                v -= self.mul();
            } else {
                return v;
            }
        }
    }
    fn mul(&mut self) -> i64 {
        let mut v = self.unary();
        loop {
            if self.eat("*") {
                v *= self.unary();
            } else if self.eat("/") {
                let r = self.unary();
                v = if r == 0 { 0 } else { v / r };
            } else if self.eat("%") {
                let r = self.unary();
                v = if r == 0 { 0 } else { v % r };
            } else {
                return v;
            }
        }
    }
    fn unary(&mut self) -> i64 {
        if self.peek("!") && !self.peek("!=") {
            self.i += 1;
            return (self.unary() == 0) as i64;
        }
        if self.eat("(") {
            let v = self.ternary();
            self.eat(")");
            return v;
        }
        if self.eat("n") {
            return self.n;
        }
        let start = self.i;
        while self.t.get(self.i).is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        if start == self.i {
            // Caractère inattendu : on l'ignore plutôt que de boucler.
            self.i += 1;
            return 0;
        }
        self.t[start..self.i].iter().collect::<String>().parse().unwrap_or(0)
    }
}

/// Textes d'un fichier .po indexés par le texte français du code. Les fichiers faits depuis le
/// modèle (`X-Source-Language: en`) ont l'anglais pour msgid : il est ramené au français par la
/// traduction anglaise intégrée (un même texte anglais peut valoir pour plusieurs textes français).
fn keyed(text: &str) -> HashMap<String, Vec<String>> {
    let map = parse_po(text);
    if header(text, "X-Source-Language").as_deref() != Some("en") {
        return map;
    }
    let mut from_en: HashMap<String, Vec<String>> = HashMap::new();
    for (fr, en) in parse_po(BUILTIN[1].2) {
        if let Some(first) = en.into_iter().next().filter(|_| !fr.is_empty()) {
            from_en.entry(first).or_default().push(fr);
        }
    }
    let mut out = HashMap::new();
    for (en, tr) in map {
        if en.is_empty() {
            out.insert(en, tr);
            continue;
        }
        for fr in from_en.get(&en).into_iter().flatten() {
            out.insert(fr.clone(), tr.clone());
        }
    }
    out
}

/// Entrées de la traduction anglaise intégrée, dans l'ordre : (français, pluriel français, anglais).
fn en_entries() -> Vec<(String, Option<String>, Vec<String>)> {
    let mut out = Vec::new();
    for block in BUILTIN[1].2.split("\n\n") {
        let mut fr = None;
        let mut fr_pl = None;
        let mut en = Vec::new();
        for l in block.lines() {
            let l = l.trim();
            if let Some(r) = l.strip_prefix("msgid_plural ") {
                fr_pl = Some(unquote(r));
            } else if let Some(r) = l.strip_prefix("msgid ") {
                fr = Some(unquote(r));
            } else if let Some(r) = l.strip_prefix("msgstr") {
                let r = r.trim_start();
                let r = if r.starts_with('[') { r.split_once(']').map(|(_, x)| x).unwrap_or(r) } else { r };
                en.push(unquote(r));
            }
        }
        if let Some(fr) = fr.filter(|f| !f.is_empty()) {
            out.push((fr, fr_pl, en));
        }
    }
    out
}

fn load(code: &str) -> Option<Catalog> {
    let text = po_text(code)?;
    let plural = Plural::from_header(header(&text, "Plural-Forms"));
    // Textes non traduits (langue ajoutée incomplète) : l'anglais plutôt que le français.
    let mut all = if code == "en" { HashMap::new() } else { po_text("en").map(|t| parse_po(&t)).unwrap_or_default() };
    // Formes plurielles anglaises : seulement si la langue en a autant (sinon l'indice ne correspond pas).
    let n = header(&text, "Plural-Forms")
        .and_then(|h| h.split(';').find_map(|p| p.trim().strip_prefix("nplurals=").and_then(|v| v.trim().parse::<usize>().ok())))
        .unwrap_or(2);
    all.retain(|_, v| v.len() == 1 || v.len() == n);
    all.extend(keyed(&text));
    let strs = all
        .into_iter()
        .filter(|(k, _)| !k.is_empty())
        .map(|(k, v)| (k, v.into_iter().map(|s| &*Box::leak(s.into_boxed_str())).collect()))
        .collect();
    Some(Catalog { strs, plural })
}

/// Vérification du démarrage : la langue est connue et ses traductions se lisent.
/// Renvoie le nombre de textes traduits (0 pour le français, langue du code).
pub fn check(code: &str) -> Result<usize, String> {
    if code == "fr" || code.is_empty() {
        return Ok(0);
    }
    if !languages().iter().any(|(id, _)| id == code) {
        return Err(format!("langue inconnue « {code} »"));
    }
    let n = po_text(code).map(|t| parse_po(&t).len().saturating_sub(1)).unwrap_or(0);
    if n == 0 {
        Err(format!("traductions « {code} » absentes"))
    } else {
        Ok(n)
    }
}

/// Langue choisie à l'installation (Windows : l'installeur écrit son code dans un fichier
/// `language` à côté de l'exécutable). `None` si absente ou inconnue.
pub fn installer_language() -> Option<String> {
    let text = std::fs::read_to_string(crate::paths::exe_dir()?.join("language")).ok()?;
    let code = text.trim().trim_start_matches('\u{feff}').to_lowercase();
    languages().iter().any(|(id, _)| *id == code).then_some(code)
}

/// Langue du système (« fr », « en »...), pour présélectionner le choix au premier lancement.
pub fn system_language() -> String {
    let raw = std::env::var("LC_ALL").or_else(|_| std::env::var("LANG")).unwrap_or_default();
    // Modifiée seulement sous Windows (langue de l'interface du système).
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut code = raw.split(['_', '.', '-']).next().unwrap_or("").to_lowercase();
    #[cfg(windows)]
    if code.is_empty() {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetUserDefaultUILanguage() -> u16;
        }
        // SAFETY : appel Win32 sans argument.
        let lang = unsafe { GetUserDefaultUILanguage() } & 0x3ff;
        // Identifiants de langue principaux de Windows.
        code = match lang {
            0x0c => "fr",
            0x0a => "es",
            0x07 => "de",
            0x10 => "it",
            0x16 => "pt",
            0x15 => "pl",
            0x13 => "nl",
            _ => "en",
        }
        .into();
    }
    if languages().iter().any(|(c, _)| *c == code) {
        code
    } else {
        "en".into()
    }
}

/// Langue en cours (« fr » par défaut).
pub fn current() -> String {
    let c = CODE.read().unwrap().clone();
    if c.is_empty() { "fr".into() } else { c }
}

/// Change la langue (textes Rust et interface, à chaud).
pub fn set_language(code: &str) {
    *CURRENT.write().unwrap() = if code == "fr" { None } else { load(code) };
    *CODE.write().unwrap() = code.to_string();
    if let Some(f) = ON_CHANGE.get() {
        f();
    }
}

/// Texte traduit (le texte français lui-même s'il n'a pas de traduction).
pub fn tr(fr: &'static str) -> &'static str {
    CURRENT.read().unwrap().as_ref().and_then(|m| m.strs.get(fr).and_then(|v| v.first().copied())).unwrap_or(fr)
}

/// Comme `tr`, pour un texte qui n'est pas littéral (interface Slint).
pub fn tr_str(fr: &str) -> String {
    CURRENT.read().unwrap().as_ref().and_then(|m| m.strs.get(fr).and_then(|v| v.first().map(|s| s.to_string()))).unwrap_or_else(|| fr.to_string())
}

/// Remplace les `{}` de `t` par les valeurs, dans l'ordre.
fn fill(t: &str, args: &[&dyn std::fmt::Display]) -> String {
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

/// Texte traduit avec des valeurs à la place des `{}`, dans l'ordre.
pub fn trf(fr: &'static str, args: &[&dyn std::fmt::Display]) -> String {
    fill(tr(fr), args)
}

/// Interface Slint : texte traduit avec jusqu'à deux valeurs.
pub fn trf_str(fr: &str, args: &[&str]) -> String {
    let t = tr_str(fr);
    let a: Vec<&dyn std::fmt::Display> = args.iter().map(|s| s as &dyn std::fmt::Display).collect();
    fill(&t, &a)
}

/// Pluriel : `one` / `other` (français) selon n, traduit avec la règle de la langue ; `{n}` est
/// remplacé par n.
pub fn trn(one: &str, other: &str, n: i64) -> String {
    let guard = CURRENT.read().unwrap();
    let t = match guard.as_ref().and_then(|m| m.strs.get(one).map(|v| (v, &m.plural))) {
        Some((forms, rule)) => forms.get(rule.index(n)).or_else(|| forms.last()).map(|s| s.to_string()),
        None => None,
    };
    let t = t.unwrap_or_else(|| if n > 1 { other.to_string() } else { one.to_string() });
    t.replace("{n}", &n.to_string())
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

/// Modèle d'une nouvelle langue, écrit dans le dossier des langues ajoutées (`modele.po`) : textes
/// en anglais à traduire, avec en note le français (texte d'origine) et la langue en cours.
pub fn write_template() -> std::io::Result<PathBuf> {
    let dir = lang_dir().ok_or_else(|| std::io::Error::other("dossier des langues introuvable"))?;
    std::fs::create_dir_all(&dir)?;
    let cur = current();
    let cur_name = languages().into_iter().find(|(c, _)| *c == cur).map(|(_, n)| n).unwrap_or_default();
    let mut out = String::from(
        "# Turtlefin translation template · Modèle de traduction de Turtlefin\n\
         #\n\
         # 1. Copy this file as <code>.po (es.po, ja.po, sv.po…) and translate every msgstr from the English\n\
         #    msgid above it. Keep the {} and {n} placeholders. Notes (#.) show the French original.\n\
         # 2. Fill in X-Language-Name (shown in the list) and Plural-Forms (gettext rule of the language).\n\
         # 3. Keep it in this folder (Turtlefin Languages), then in Turtlefin: Settings → Display →\n\
         #    Add a language, and select it. Empty msgstr: the English text is shown.\n\
         msgid \"\"\n\
         msgstr \"\"\n\
         \"Content-Type: text/plain; charset=UTF-8\\n\"\n\
         \"X-Source-Language: en\\n\"\n\
         \"X-Language-Name: \\n\"\n\
         \"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
    );
    // Un texte anglais une seule fois (un .po n'accepte pas deux fois le même msgid).
    let mut seen = std::collections::HashSet::new();
    for (fr, fr_pl, en) in en_entries() {
        let Some(en0) = en.first().filter(|e| !e.is_empty()) else { continue };
        if !seen.insert(en0.clone()) {
            continue;
        }
        out.push_str(&format!("#. fr: {}\n", esc(&fr)));
        if cur != "fr" && cur != "en" {
            let t = if fr_pl.is_some() { trn(&fr, fr_pl.as_deref().unwrap_or(""), 2) } else { tr_str(&fr) };
            if t != fr {
                out.push_str(&format!("#. {cur_name}: {}\n", esc(&t)));
            }
        }
        out.push_str(&format!("msgid \"{}\"\n", esc(en0)));
        match en.get(1) {
            Some(pl) => out.push_str(&format!("msgid_plural \"{}\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n\n", esc(pl))),
            None => out.push_str("msgstr \"\"\n\n"),
        }
    }
    std::fs::write(dir.join("modele.po"), out)?;
    Ok(dir)
}

/// Un fichier .po est-il une traduction de Turtlefin (et pas un autre .po de passage) ?
fn is_turtlefin_po(text: &str) -> bool {
    text.contains("msgid \"Reprendre\"") || text.contains("msgid \"Continue watching\"") || text.contains("X-Source-Language: en")
}

#[cfg(test)]
mod tests {
    #[test]
    fn po() {
        let m = super::parse_po("msgid \"\"\nmsgstr \"entête\"\n\nmsgid \"Bonjour {}\"\nmsgstr \"Hello {}\"\n\nmsgid \"a\"\n\"b\"\nmsgstr \"c\\\"d\"\n\nmsgid \"{n} x\"\nmsgid_plural \"{n} xs\"\nmsgstr[0] \"un\"\nmsgstr[1] \"deux\"\n");
        assert_eq!(m.get("Bonjour {}").map(|v| v[0].as_str()), Some("Hello {}"));
        assert_eq!(m.get("ab").map(|v| v[0].as_str()), Some("c\"d"));
        assert_eq!(m.get("{n} x").map(|v| v.len()), Some(2));
    }

    #[test]
    fn plural_rules() {
        let pl = super::Plural("(n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2)".into());
        assert_eq!([1, 2, 5, 12, 22, 25].map(|n| pl.index(n)), [0, 1, 2, 2, 1, 2]);
        let fr = super::Plural("(n > 1)".into());
        assert_eq!([0, 1, 2].map(|n| fr.index(n)), [0, 0, 1]);
        let en = super::Plural::from_header(None);
        assert_eq!([1, 2].map(|n| en.index(n)), [0, 1]);
    }

    #[test]
    fn english_source_file() {
        // Fichier fait depuis le modèle : msgid anglais, ramené au texte français du code.
        let po = "msgid \"\"\nmsgstr \"\"\n\"X-Source-Language: en\\n\"\n\nmsgid \"Continue watching\"\nmsgstr \"Fortsätt titta\"\n\nmsgid \"Sign in\"\nmsgstr \"Logga in\"\n";
        let m = super::keyed(po);
        assert_eq!(m.get("Reprendre").map(|v| v[0].as_str()), Some("Fortsätt titta"));
        // « Sign in » traduit deux textes français (Se connecter, Connexion).
        assert_eq!(m.get("Se connecter").map(|v| v[0].as_str()), Some("Logga in"));
        assert_eq!(m.get("Connexion").map(|v| v[0].as_str()), Some("Logga in"));
    }

    #[test]
    fn builtin_catalogs() {
        for (code, _, text) in super::BUILTIN.iter().skip(1) {
            let n = super::parse_po(text).len();
            assert!(n > 300, "{code} : {n} textes seulement");
        }
    }
}
