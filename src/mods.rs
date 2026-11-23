//! Mods : scripts Lua écrits par d'autres que le mainteneur, chargés au démarrage.
//!
//! Dossier « Turtlefin Mods » (à côté de « Turtlefin Themes ») : un mod est un dossier contenant
//! `mod.lua` (et, facultatif, `mod.json` : `name`, `version`, `author`, `description`), ou un simple
//! fichier `nom.lua`. Paramètres → Mods : activer, désactiver, ordre de chargement ; un changement
//! prend effet au redémarrage (proposé après « Sauvegarder »).
//!
//! Un mod est ISOLÉ (bac à sable) : il ne voit que ce que Turtlefin lui donne. Pas de bibliothèques
//! `io`, `os`, `package`, `debug` ; pas de `load`, `loadfile`, `dofile`, ni de bytecode
//! (`string.dump`) ; `require` ne charge que des fichiers `.lua` de son propre dossier ; pas d'accès
//! réseau libre ; `api` seulement en lecture, sur les chemins des médias (pas les comptes, sessions,
//! clés, réglages du serveur) ; fichiers (sons, images du thème) seulement dans son dossier. Mémoire
//! plafonnée (`MEMORY`), et une fonction qui calcule trop longtemps est arrêtée (`MAX_RUN`). Un mod ne
//! peut donc ni lire les fichiers de l'appareil, ni lancer un programme, ni envoyer des données ailleurs.
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
//!   api(chemin) -> table (lecture au serveur Jellyfin, avec le compte ouvert ; chemins de `API_PATHS`)
//!   time() -> secondes depuis 1970 · clock() -> secondes depuis le lancement (os n'est pas fourni)

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};

use mlua::{Function, HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use serde::{Deserialize, Serialize};

use crate::App;

pub const DIR_NAME: &str = "Turtlefin Mods";

/// Mémoire de Lua pour tous les mods réunis.
const MEMORY: usize = 64 << 20;
/// Durée maximale d'un chargement ou d'un appel de mod (une boucle sans fin n'arrête pas les autres).
const MAX_RUN: std::time::Duration = std::time::Duration::from_secs(3);
/// Chemins du serveur qu'un mod peut lire (médias, bibliothèques ; pas les comptes, les sessions, les
/// clés d'API, les réglages ni le système).
const API_PATHS: [&str; 9] = ["/Items", "/UserItems", "/UserViews", "/Shows", "/Movies", "/Genres", "/Studios", "/Persons", "/Search"];
/// Début de l'appel de mod en cours (voir `MAX_RUN`).
static RUN_START: Mutex<Option<std::time::Instant>> = Mutex::new(None);
/// Dossiers des mods activés (leurs fichiers : sons).
struct ModDirs(Vec<PathBuf>);
/// Dossier du mod en cours de chargement (`require`, fichiers).
static LOADING: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Chemin que `turtlefin.api` accepte : un de `API_PATHS` (ou en dessous), fait seulement de lettres,
/// chiffres, `/`, `-`, `_` : `%2e%2e`, ou « . » coupé par une tabulation, deviennent « .. » à l'analyse de
/// l'adresse (`/Items/%2e%2e/Auth/Keys` lisait les clés d'API) ; requête sans caractère de contrôle.
fn mod_api_path_ok(path: &str) -> bool {
    let (base, query) = path.split_once('?').unwrap_or((path, ""));
    API_PATHS.iter().any(|p| base == *p || base.starts_with(&format!("{p}/")))
        && base.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_'))
        && !base.contains("//")
        && !query.chars().any(|c| c.is_control() || c == '#')
}

/// Lua sans rien qui touche à l'appareil (voir l'en-tête du module).
fn sandbox() -> mlua::Result<Lua> {
    let lua = Lua::new_with(StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::MATH | StdLib::COROUTINE, LuaOptions::default())?;
    let _ = lua.set_memory_limit(MEMORY);
    let g = lua.globals();
    for name in ["load", "loadfile", "dofile", "loadstring", "require", "collectgarbage"] {
        g.set(name, Value::Nil)?;
    }
    if let Ok(s) = g.get::<Table>("string") {
        s.set("dump", Value::Nil)?;
    }
    lua.set_hook(HookTriggers::new().every_nth_instruction(20_000), |_, _| {
        let start = *RUN_START.lock().unwrap();
        if start.is_some_and(|t| t.elapsed() > MAX_RUN) {
            return Err(mlua::Error::runtime("trop long : le mod a été interrompu"));
        }
        Ok(VmState::Continue)
    });
    Ok(lua)
}

/// Exécute du code de mod avec la limite de temps.
fn timed<T>(f: impl FnOnce() -> mlua::Result<T>) -> mlua::Result<T> {
    *RUN_START.lock().unwrap() = Some(std::time::Instant::now());
    let r = f();
    *RUN_START.lock().unwrap() = None;
    r
}

/// Fichier du dossier `dir` désigné par un mod : nom simple (lettres, chiffres, `-`, `_`, `.`, `/` pour
/// un sous-dossier), jamais `..`, jamais un chemin absolu.
fn inside(dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    let ok = !name.is_empty()
        && !name.starts_with('/')
        && !name.contains("..")
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'));
    ok.then(|| dir.join(name)).filter(|p| p.is_file())
}

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
    let lua = match sandbox() {
        Ok(l) => l,
        Err(e) => {
            log(&format!("préparation de Lua : {e}"));
            return;
        }
    };
    if let Err(e) = install_api(&lua, &app) {
        log(&format!("préparation de Lua : {e}"));
        return;
    }
    lua.set_app_data(ModDirs(
        mods.iter().filter(|m| m.script.file_name().is_some_and(|n| n == "mod.lua")).filter_map(|m| m.script.parent().map(|p| p.to_path_buf())).collect(),
    ));
    for m in &mods {
        let code = match std::fs::read_to_string(&m.script) {
            Ok(c) => c,
            Err(e) => {
                log(&format!("{} : lecture de {} : {e}", m.id, m.script.display()));
                continue;
            }
        };
        // `require` et les fichiers du mod : dans son dossier (un mod en un seul fichier : aucun).
        let own_dir = m.script.parent().filter(|_| m.script.file_name().is_some_and(|n| n == "mod.lua")).map(|p| p.to_path_buf());
        *LOADING.lock().unwrap() = own_dir;
        // Texte seulement (pas de bytecode Lua précompilé, qui peut contourner les vérifications).
        let res = timed(|| lua.load(code.as_str()).set_name(m.id.as_str()).set_mode(mlua::ChunkMode::Text).exec());
        *LOADING.lock().unwrap() = None;
        match res {
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
        if let Err(e) = timed(|| f.call::<()>(arg)) {
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
    lua.set_named_registry_value("turtlefin_modules", lua.create_table()?)?;

    // `require("nom")` : `nom.lua` du dossier du mod, pendant son chargement seulement ; gardé ensuite.
    lua.globals().set(
        "require",
        lua.create_function(|lua, name: String| {
            let dir = LOADING.lock().unwrap().clone().ok_or_else(|| mlua::Error::runtime("require : seulement au chargement d'un mod en dossier"))?;
            let file = inside(&dir, &format!("{}.lua", name.replace('.', "/")))
                .ok_or_else(|| mlua::Error::runtime(format!("require : « {name} » introuvable dans le dossier du mod")))?;
            let modules: Table = lua.named_registry_value("turtlefin_modules")?;
            let key = file.display().to_string();
            if let Ok(v) = modules.get::<Value>(key.as_str()) {
                if !v.is_nil() {
                    return Ok(v);
                }
            }
            let code = std::fs::read_to_string(&file).map_err(mlua::Error::external)?;
            let v: Value = lua.load(code.as_str()).set_name(name.as_str()).set_mode(mlua::ChunkMode::Text).eval()?;
            let v = if v.is_nil() { Value::Boolean(true) } else { v };
            modules.set(key.as_str(), v.clone())?;
            Ok(v)
        })?,
    )?;
    let t0 = std::time::Instant::now();
    t.set("clock", lua.create_function(move |_, ()| Ok(t0.elapsed().as_secs_f64()))?)?;
    t.set(
        "time",
        lua.create_function(|_, ()| Ok(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)))?,
    )?;

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
    // Un son d'un mod activé (chemin relatif à son dossier ; rien d'autre sur l'appareil).
    t.set(
        "sound",
        lua.create_function(|lua, name: String| {
            let found = lua.app_data_ref::<ModDirs>().and_then(|d| d.0.iter().find_map(|dir| inside(dir, &name)));
            match found {
                Some(f) => crate::music::sound(&f.display().to_string()),
                None => log(&format!("sound : « {name} » n'est pas un fichier du dossier d'un mod")),
            }
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
            // Lecture seulement, chemins des médias seulement (voir API_PATHS) ; pas de « .. » ni d'adresse.
            if !mod_api_path_ok(&path) {
                return Err(mlua::Error::runtime(format!("api : chemin non autorisé aux mods ({path})")));
            }
            match a.rt.block_on(c.get_json(&path)) {
                Ok(v) => json_to_lua(lua, &v),
                Err(e) => Err(mlua::Error::runtime(format!("api {path} : {e}"))),
            }
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
                // Fichiers du thème (fond, logo, musique) : pas pour un mod (ils pourraient désigner
                // n'importe quel fichier de l'appareil).
                if matches!(k.as_str(), "background" | "logo_file" | "music" | "dir") {
                    log(&format!("theme : « {k} » n'est pas modifiable par un mod"));
                    continue;
                }
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
-- Un mod tourne dans un bac à sable : il n'a accès qu'à ce que Turtlefin lui donne (table turtlefin),
-- pas aux fichiers de l'appareil ni au réseau.

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
    fn sandbox_blocks_the_device() {
        let lua = sandbox().unwrap();
        for code in ["return io", "return os", "return debug", "return package", "return load", "return dofile", "return loadfile", "return string.dump"] {
            assert!(lua.load(code).eval::<Value>().unwrap().is_nil(), "{code}");
        }
        // Boucle sans fin : arrêtée.
        let r = timed(|| lua.load("while true do end").exec());
        assert!(r.is_err());
        // Mémoire : plafonnée.
        let r = timed(|| lua.load("local t = {} for i = 1, 1e8 do t[i] = string.rep('x', 100) end").exec());
        assert!(r.is_err());
        assert!(inside(std::path::Path::new("."), "../secret").is_none());
        assert!(inside(std::path::Path::new("."), "/etc/passwd").is_none());
    }

    #[test]
    fn mod_api_paths() {
        assert!(mod_api_path_ok("/Items/abc123/Images"));
        assert!(mod_api_path_ok("/Search/Hints?searchTerm=la mer&Limit=5"));
        for bad in ["/Items/%2e%2e/Auth/Keys", "/Items/.\t./Users", "/Items/../Users", "/Users", "/Items//x", "/Items\\..", "/Itemsx", "/Items?x=\n"] {
            assert!(!mod_api_path_ok(bad), "{bad:?}");
        }
    }

    #[test]
    fn json_becomes_lua() {
        let lua = Lua::new();
        let v = json_to_lua(&lua, &serde_json::json!({"Name": "x", "List": [1, 2]})).unwrap();
        lua.globals().set("v", v).unwrap();
        assert_eq!(lua.load("return v.Name .. #v.List").eval::<String>().unwrap(), "x2");
    }
}
