//! Mods : scripts Lua écrits par d'autres que le mainteneur, chargés au démarrage.
//!
//! Dossier « Turtlefin Mods » (à côté de « Turtlefin Themes ») : un mod est un dossier contenant
//! `mod.lua` (et, facultatif, `mod.json` : `name`, `version`, `author`, `description`), ou un simple
//! fichier `nom.lua`. Paramètres → Mods : activer, désactiver, ordre de chargement ; un changement
//! prend effet au redémarrage (proposé après « Sauvegarder »).
//!
//! Un mod n'est PAS isolé : Lua complet (`io`, `os`), il peut lire et écrire des fichiers, lancer des
//! programmes et parler au serveur avec le compte ouvert. Turtlefin le dit avant d'en activer un.
//!
//! Journal : `mods.log` dans le dossier de configuration, créé seulement si un mod est activé.
//!
//! Ce qu'un mod reçoit (table globale `turtlefin`) :
//!   version, platform · log(texte) · toast(texte) · sound(fichier audio)
//!   on(événement, fonction) : « start », « screen » (nom de la page), « play » / « stop » (identifiant
//!     du média), « tick » (chaque seconde)
//!   theme{ accent1 = "#ff0000", radius = 4, … } : change des réglages du thème en cours (mêmes noms
//!     que dans un fichier .tftheme)
//!   open(id) · search(texte) · screen() · server() -> { url, user }
//!   api(chemin) -> table (requête GET au serveur Jellyfin, avec le compte ouvert)
//!   http(adresse) -> texte

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};

use mlua::{Function, Lua, Table, Value};
use serde::{Deserialize, Serialize};

use crate::App;

pub const DIR_NAME: &str = "Turtlefin Mods";

/// Mod connu des réglages : identifiant (nom du dossier ou du fichier) et activation, dans l'ordre de
/// chargement.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct ModPref {
    pub id: String,
    pub on: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ModInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub script: PathBuf,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Manifest {
    name: String,
    version: String,
    author: String,
    description: String,
}

/// À côté des dossiers « Turtlefin Themes » et « Turtlefin Languages » (Documents, ou le dossier de
/// l'appli portable).
pub fn dir() -> Option<PathBuf> {
    Some(crate::i18n::lang_dir()?.parent()?.join(DIR_NAME))
}

fn log_path() -> Option<PathBuf> {
    crate::paths::config_dir().map(|d| d.join("mods.log"))
}

/// Mods présents dans le dossier (ordre alphabétique).
pub fn installed() -> Vec<ModInfo> {
    let Some(dir) = dir() else { return Vec::new() };
    let mut out: Vec<ModInfo> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let id = path.file_stem()?.to_string_lossy().into_owned();
            if path.is_dir() {
                let script = path.join("mod.lua");
                if !script.is_file() {
                    return None;
                }
                let m: Manifest = std::fs::read_to_string(path.join("mod.json"))
                    .ok()
                    .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
                    .unwrap_or_default();
                Some(ModInfo {
                    name: if m.name.is_empty() { id.clone() } else { m.name },
                    id,
                    version: m.version,
                    author: m.author,
                    description: m.description,
                    script,
                })
            } else if path.extension().is_some_and(|x| x == "lua") {
                Some(ModInfo { name: id.clone(), id, script: path, ..Default::default() })
            } else {
                None
            }
        })
        .collect();
    out.sort_by_key(|m| m.id.to_lowercase());
    out
}

/// Liste des réglages mise à jour : les mods disparus sont retirés, les nouveaux ajoutés à la fin,
/// désactivés. L'ordre choisi est gardé.
pub fn merged(saved: &[ModPref]) -> Vec<ModPref> {
    let found = installed();
    let mut out: Vec<ModPref> = saved.iter().filter(|p| found.iter().any(|m| m.id == p.id)).cloned().collect();
    for m in &found {
        if !out.iter().any(|p| p.id == m.id) {
            out.push(ModPref { id: m.id.clone(), on: false });
        }
    }
    out
}

enum Msg {
    Event(String, String),
}

static QUEUE: OnceLock<Mutex<mpsc::Sender<Msg>>> = OnceLock::new();

/// Mods en cours d'exécution (au moins un a été chargé au démarrage).
pub fn running() -> bool {
    QUEUE.get().is_some()
}

/// Événement donné aux mods (sans effet si aucun n'est chargé).
pub fn emit(event: &str, arg: &str) {
    if let Some(q) = QUEUE.get() {
        let _ = q.lock().unwrap().send(Msg::Event(event.to_string(), arg.to_string()));
    }
}

fn log(text: &str) {
    use std::io::Write;
    let Some(path) = log_path() else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(f, "[{now}] {text}");
    }
}

/// Charge les mods activés, dans l'ordre des réglages. Aucun activé : rien n'est créé (ni fil, ni
/// journal).
pub fn start(app: &Arc<App>) {
    let prefs = crate::config::ui_prefs().mods;
    let found = installed();
    let enabled: Vec<ModInfo> = prefs.iter().filter(|p| p.on).filter_map(|p| found.iter().find(|m| m.id == p.id).cloned()).collect();
    if enabled.is_empty() {
        return;
    }
    // Journal repris à zéro à chaque lancement (il ne grossit pas sans fin).
    if let Some(p) = log_path() {
        let _ = std::fs::write(p, "");
    }
    log(&format!("Turtlefin {} : {} mod(s) activé(s)", env!("CARGO_PKG_VERSION"), enabled.len()));
    let (tx, rx) = mpsc::channel::<Msg>();
    let _ = QUEUE.set(Mutex::new(tx));
    let app = app.clone();
    let spawned = std::thread::Builder::new().name("mods".into()).spawn(move || run(app, enabled, rx));
    if let Err(e) = spawned {
        log(&format!("fil des mods : {e}"));
    }
}

fn run(app: Arc<App>, mods: Vec<ModInfo>, rx: mpsc::Receiver<Msg>) {
    let lua = Lua::new();
    if let Err(e) = install_api(&lua, &app) {
        log(&format!("préparation de Lua : {e}"));
        return;
    }
    for m in &mods {
        let code = match std::fs::read_to_string(&m.script) {
            Ok(c) => c,
            Err(e) => {
                log(&format!("{} : lecture de {} : {e}", m.id, m.script.display()));
                continue;
            }
        };
        // Les `require` d'un mod cherchent d'abord dans son dossier.
        if let Some(d) = m.script.parent() {
            let set_path = format!("package.path = {:?} .. '/?.lua;' .. package.path", d.display().to_string().replace('\\', "/"));
            let _ = lua.load(set_path.as_str()).exec();
        }
        match lua.load(code.as_str()).set_name(m.id.as_str()).exec() {
            Ok(()) => log(&format!("{} {} chargé", m.name, m.version)),
            Err(e) => log(&format!("{} : erreur au chargement : {e}", m.id)),
        }
    }
    call(&lua, "start", "");
    loop {
        match rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Msg::Event(name, arg)) => call(&lua, &name, &arg),
            Err(mpsc::RecvTimeoutError::Timeout) => call(&lua, "tick", ""),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

/// Appelle les fonctions inscrites pour un événement ; une erreur va au journal, les autres continuent.
fn call(lua: &Lua, event: &str, arg: &str) {
    let Ok(handlers) = lua.named_registry_value::<Table>("turtlefin_handlers") else { return };
    let Ok(list) = handlers.get::<Table>(event) else { return };
    for f in list.sequence_values::<Function>().flatten() {
        if let Err(e) = f.call::<()>(arg) {
            log(&format!("événement « {event} » : {e}"));
        }
    }
}

fn json_to_lua(lua: &Lua, v: &serde_json::Value) -> mlua::Result<Value> {
    Ok(match v {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => Value::String(lua.create_string(s)?),
        serde_json::Value::Array(a) => {
            let t = lua.create_table()?;
            for (i, x) in a.iter().enumerate() {
                t.set(i + 1, json_to_lua(lua, x)?)?;
            }
            Value::Table(t)
        }
        serde_json::Value::Object(o) => {
            let t = lua.create_table()?;
            for (k, x) in o {
                t.set(k.as_str(), json_to_lua(lua, x)?)?;
            }
            Value::Table(t)
        }
    })
}

fn install_api(lua: &Lua, app: &Arc<App>) -> mlua::Result<()> {
    let t = lua.create_table()?;
    t.set("version", env!("CARGO_PKG_VERSION"))?;
    t.set("platform", std::env::consts::OS)?;
    lua.set_named_registry_value("turtlefin_handlers", lua.create_table()?)?;

    t.set(
        "on",
        lua.create_function(|lua, (event, f): (String, Function)| {
            let handlers: Table = lua.named_registry_value("turtlefin_handlers")?;
            let list: Table = match handlers.get::<Option<Table>>(event.as_str())? {
                Some(l) => l,
                None => {
                    let l = lua.create_table()?;
                    handlers.set(event.as_str(), l.clone())?;
                    l
                }
            };
            list.push(f)?;
            Ok(())
        })?,
    )?;
    let log_fn = lua.create_function(|_, text: String| {
        log(&text);
        Ok(())
    })?;
    t.set("log", log_fn.clone())?;
    // `print` écrit dans le journal des mods (pas de console).
    lua.globals().set("print", log_fn)?;

    let a = app.clone();
    t.set(
        "toast",
        lua.create_function(move |_, text: String| {
            let _ = a.ui().upgrade_in_event_loop(move |u| u.set_toast(text.into()));
            Ok(())
        })?,
    )?;
    t.set(
        "sound",
        lua.create_function(|_, name: String| {
            crate::music::sound(&name);
            Ok(())
        })?,
    )?;
    let a = app.clone();
    t.set(
        "open",
        lua.create_function(move |_, id: String| {
            let a2 = a.clone();
            let _ = a.ui().upgrade_in_event_loop(move |_| crate::push_detail(&a2, id));
            Ok(())
        })?,
    )?;
    let a = app.clone();
    t.set(
        "search",
        lua.create_function(move |_, text: String| {
            let a2 = a.clone();
            let _ = a.ui().upgrade_in_event_loop(move |_| crate::search_person(&a2, &text));
            Ok(())
        })?,
    )?;
    let a = app.clone();
    t.set(
        "screen",
        lua.create_function(move |_, ()| {
            let (tx, rx) = mpsc::channel::<String>();
            let _ = a.ui().upgrade_in_event_loop(move |u| {
                let _ = tx.send(u.get_screen().to_string());
            });
            Ok(rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap_or_default())
        })?,
    )?;
    let a = app.clone();
    t.set(
        "server",
        lua.create_function(move |lua, ()| {
            let t = lua.create_table()?;
            if let Some(c) = a.client() {
                t.set("url", c.server)?;
                t.set("user", c.user_name)?;
            }
            Ok(t)
        })?,
    )?;
    let a = app.clone();
    t.set(
        "api",
        lua.create_function(move |lua, path: String| {
            let Some(c) = a.client() else { return Ok(Value::Nil) };
            if !path.starts_with('/') {
                return Err(mlua::Error::runtime("api : le chemin commence par « / »"));
            }
            match a.rt.block_on(c.get_json(&path)) {
                Ok(v) => json_to_lua(lua, &v),
                Err(e) => Err(mlua::Error::runtime(format!("api {path} : {e}"))),
            }
        })?,
    )?;
    let a = app.clone();
    t.set(
        "http",
        lua.create_function(move |_, url: String| {
            let r = a.rt.block_on(async {
                let resp = reqwest::Client::new().get(&url).timeout(std::time::Duration::from_secs(20)).send().await?;
                resp.text().await
            });
            r.map_err(|e| mlua::Error::runtime(format!("http {url} : {e}")))
        })?,
    )?;
    let a = app.clone();
    t.set(
        "theme",
        lua.create_function(move |_, changes: Table| {
            // Réglages du thème en cours, remplacés un à un par ceux du mod (mêmes noms qu'un .tftheme).
            let p = crate::config::ui_prefs();
            let mut v = serde_json::to_value(crate::theme::find(&p.theme, &p.themes)).map_err(mlua::Error::external)?;
            for pair in changes.pairs::<String, Value>() {
                let (k, x) = pair?;
                v[k.as_str()] = match x {
                    Value::Boolean(b) => serde_json::Value::Bool(b),
                    Value::Integer(i) => serde_json::json!(i),
                    Value::Number(n) => serde_json::json!(n),
                    Value::String(s) => serde_json::Value::String(s.to_string_lossy()),
                    _ => continue,
                };
            }
            let theme: crate::theme::ThemeDef = serde_json::from_value(v).map_err(mlua::Error::external)?;
            let _ = a.ui().upgrade_in_event_loop(move |u| crate::theme::apply(&u, &theme));
            Ok(())
        })?,
    )?;
    lua.globals().set("turtlefin", t)?;
    Ok(())
}

/// Mod d'exemple écrit dans le dossier à sa création : montre les fonctions offertes.
pub const EXAMPLE: &str = r##"-- Exemple de mod Turtlefin. Active-le dans Paramètres -> Mods, puis redémarre.
-- Un mod n'est pas isolé : n'installe que des mods dont tu connais l'auteur.

turtlefin.on("start", function()
  turtlefin.log("Bonjour depuis l'exemple (Turtlefin " .. turtlefin.version .. ")")
  turtlefin.toast("Mod d'exemple chargé")
end)

-- Change un réglage du thème en cours (mêmes noms que dans un fichier .tftheme).
-- turtlefin.theme{ accent1 = "#ff7a00", accent2 = "#ffd000" }

-- Appelée à chaque changement de page : "home", "detail", "search", "library", "settings"...
turtlefin.on("screen", function(name)
  turtlefin.log("page : " .. name)
end)

-- Début et fin de lecture (identifiant du média).
turtlefin.on("play", function(id)
  local item = turtlefin.api("/Items/" .. id)
  if item then turtlefin.log("lecture : " .. (item.Name or id)) end
end)
"##;

/// Crée le dossier des mods (avec l'exemple, la première fois) et rend son chemin.
pub fn ensure_dir() -> Option<PathBuf> {
    let d = dir()?;
    if !d.exists() {
        std::fs::create_dir_all(d.join("exemple")).ok()?;
        let _ = std::fs::write(d.join("exemple").join("mod.lua"), EXAMPLE);
        let _ = std::fs::write(
            d.join("exemple").join("mod.json"),
            "{\n  \"name\": \"Exemple\",\n  \"version\": \"1.0\",\n  \"author\": \"Turtlefin\",\n  \"description\": \"Montre ce qu'un mod peut faire : message, journal, page ouverte, lecture.\"\n}\n",
        );
    }
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_runs_and_handlers_fire() {
        let lua = Lua::new();
        lua.set_named_registry_value("turtlefin_handlers", lua.create_table().unwrap()).unwrap();
        let t = lua.create_table().unwrap();
        t.set(
            "on",
            lua.create_function(|lua, (event, f): (String, Function)| {
                let handlers: Table = lua.named_registry_value("turtlefin_handlers")?;
                let list = lua.create_table()?;
                list.push(f)?;
                handlers.set(event, list)?;
                Ok(())
            })
            .unwrap(),
        )
        .unwrap();
        lua.globals().set("turtlefin", t).unwrap();
        lua.load("hits = 0; turtlefin.on('screen', function(name) hits = hits + #name end)").exec().unwrap();
        call(&lua, "screen", "home");
        call(&lua, "absent", "");
        assert_eq!(lua.globals().get::<i64>("hits").unwrap(), 4);
    }

    #[test]
    fn json_becomes_lua() {
        let lua = Lua::new();
        let v = json_to_lua(&lua, &serde_json::json!({"Name": "x", "List": [1, 2]})).unwrap();
        lua.globals().set("v", v).unwrap();
        assert_eq!(lua.load("return v.Name .. #v.List").eval::<String>().unwrap(), "x2");
    }
}
