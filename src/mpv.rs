//! Liaison minimale avec libmpv, chargée à l'exécution (libmpv-2.dll / libmpv.so.2).
//!
//! Chargement dynamique : pas de bibliothèque d'import à installer pour compiler, et si libmpv
//! manque, Turtlefin démarre quand même et l'explique au moment de lire une vidéo.
//!
//! Deux objets :
//! - `Mpv` : le lecteur (commandes, propriétés, événements). Utilisable depuis n'importe quel thread.
//! - `Render` : le rendu OpenGL de la vidéo dans une texture, à créer et utiliser sur le thread
//!   de l'interface (celui qui a le contexte OpenGL de Slint).

use crate::i18n::{tr, trf};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::sync::{Arc, OnceLock};

use anyhow::{anyhow, Result};
use libloading::Library;
use serde_json::Value;

type Handle = *mut c_void;

const FORMAT_STRING: c_int = 1;
const FORMAT_FLAG: c_int = 3;
const FORMAT_INT64: c_int = 4;
const FORMAT_DOUBLE: c_int = 5;
const FORMAT_NODE: c_int = 6;
const FORMAT_NODE_ARRAY: c_int = 7;
const FORMAT_NODE_MAP: c_int = 8;

const EVENT_SHUTDOWN: c_int = 1;
const EVENT_END_FILE: c_int = 7;
const EVENT_FILE_LOADED: c_int = 8;
const EVENT_PLAYBACK_RESTART: c_int = 21;
const EVENT_PROPERTY_CHANGE: c_int = 22;

const RENDER_PARAM_INVALID: c_int = 0;
const RENDER_PARAM_API_TYPE: c_int = 1;
const RENDER_PARAM_OPENGL_INIT_PARAMS: c_int = 2;
const RENDER_PARAM_OPENGL_FBO: c_int = 3;
const RENDER_PARAM_FLIP_Y: c_int = 4;

#[repr(C)]
struct RawEvent {
    event_id: c_int,
    error: c_int,
    reply_userdata: u64,
    data: *mut c_void,
}

#[repr(C)]
struct RawEventProperty {
    name: *const c_char,
    format: c_int,
    data: *mut c_void,
}

#[repr(C)]
struct RawEventEndFile {
    reason: c_int,
    error: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
union NodeU {
    string: *mut c_char,
    flag: c_int,
    int64: i64,
    double: f64,
    list: *mut NodeList,
}

#[repr(C)]
struct Node {
    u: NodeU,
    format: c_int,
}

#[repr(C)]
struct NodeList {
    num: c_int,
    values: *mut Node,
    keys: *mut *mut c_char,
}

#[repr(C)]
struct RenderParam {
    kind: c_int,
    data: *mut c_void,
}

#[repr(C)]
struct OpenGlInitParams {
    get_proc_address: Option<unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void>,
    get_proc_address_ctx: *mut c_void,
}

#[repr(C)]
struct OpenGlFbo {
    fbo: c_int,
    w: c_int,
    h: c_int,
    internal_format: c_int,
}

/// Fonctions de libmpv utilisées par Turtlefin.
struct Api {
    _lib: Library,
    create: unsafe extern "C" fn() -> Handle,
    initialize: unsafe extern "C" fn(Handle) -> c_int,
    terminate_destroy: unsafe extern "C" fn(Handle),
    error_string: unsafe extern "C" fn(c_int) -> *const c_char,
    set_option_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    set_property_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    command: unsafe extern "C" fn(Handle, *mut *const c_char) -> c_int,
    observe_property: unsafe extern "C" fn(Handle, u64, *const c_char, c_int) -> c_int,
    wait_event: unsafe extern "C" fn(Handle, f64) -> *mut RawEvent,
    render_create: unsafe extern "C" fn(*mut *mut c_void, Handle, *mut RenderParam) -> c_int,
    render_set_update_callback:
        unsafe extern "C" fn(*mut c_void, Option<unsafe extern "C" fn(*mut c_void)>, *mut c_void),
    render_update: unsafe extern "C" fn(*mut c_void) -> u64,
    render_render: unsafe extern "C" fn(*mut c_void, *mut RenderParam) -> c_int,
    render_free: unsafe extern "C" fn(*mut c_void),
}

fn candidates() -> Vec<std::path::PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["libmpv-2.dll", "mpv-2.dll", "mpv-1.dll"]
    } else if cfg!(target_os = "macos") {
        &["libmpv.2.dylib", "libmpv.dylib"]
    } else {
        &["libmpv.so.2", "libmpv.so.1", "libmpv.so"]
    };
    let mut out = Vec::new();
    // TURTLEFIN_LIBMPV=chemin/vers/libmpv : emplacement explicite.
    if let Ok(p) = std::env::var("TURTLEFIN_LIBMPV") {
        out.push(p.into());
    }
    // À côté de l'exécutable (Windows), puis recherche standard du système.
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
        out.extend(names.iter().map(|n| dir.join(n)));
    }
    out.extend(names.iter().map(std::path::PathBuf::from));
    out
}

fn load() -> Result<Api, String> {
    let mut last_err = String::new();
    for path in candidates() {
        // SAFETY : libmpv n'exécute rien de particulier au chargement.
        let lib = match unsafe { Library::new(&path) } {
            Ok(l) => l,
            Err(e) => {
                last_err = e.to_string();
                continue;
            }
        };
        macro_rules! sym {
            ($name:literal) => {
                // SAFETY : signatures recopiées de client.h / render.h.
                *unsafe { lib.get(concat!($name, "\0").as_bytes()) }
                    .map_err(|e| format!("{} : fonction {} absente ({e})", path.display(), $name))?
            };
        }
        let api = Api {
            create: sym!("mpv_create"),
            initialize: sym!("mpv_initialize"),
            terminate_destroy: sym!("mpv_terminate_destroy"),
            error_string: sym!("mpv_error_string"),
            set_option_string: sym!("mpv_set_option_string"),
            set_property_string: sym!("mpv_set_property_string"),
            command: sym!("mpv_command"),
            observe_property: sym!("mpv_observe_property"),
            wait_event: sym!("mpv_wait_event"),
            render_create: sym!("mpv_render_context_create"),
            render_set_update_callback: sym!("mpv_render_context_set_update_callback"),
            render_update: sym!("mpv_render_context_update"),
            render_render: sym!("mpv_render_context_render"),
            render_free: sym!("mpv_render_context_free"),
            _lib: lib,
        };
        return Ok(api);
    }
    Err(last_err)
}

/// libmpv peut-elle être chargée ? (vérification du démarrage)
pub fn available() -> Result<(), String> {
    api().map(|_| ()).map_err(|e| e.to_string())
}

fn api() -> Result<&'static Api> {
    static API: OnceLock<Result<Api, String>> = OnceLock::new();
    API.get_or_init(load).as_ref().map_err(|e| {
        let hint = if cfg!(windows) {
            tr("place libmpv-2.dll à côté de turtlefin.exe")
        } else {
            tr("installe-la avec « sudo apt install libmpv2 »")
        };
        anyhow!("{}", trf("libmpv introuvable ({}). Détail : {}", &[&hint, &e]))
    })
}

fn cstr(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

/// Convertit un nœud mpv en JSON (listes de pistes, de chapitres...).
unsafe fn node_to_json(n: &Node) -> Value {
    match n.format {
        FORMAT_STRING => Value::String(CStr::from_ptr(n.u.string).to_string_lossy().into_owned()),
        FORMAT_FLAG => Value::Bool(n.u.flag != 0),
        FORMAT_INT64 => Value::from(n.u.int64),
        FORMAT_DOUBLE => serde_json::Number::from_f64(n.u.double).map(Value::Number).unwrap_or(Value::Null),
        FORMAT_NODE_ARRAY | FORMAT_NODE_MAP => {
            let list = &*n.u.list;
            let len = list.num.max(0) as usize;
            let values = if len == 0 { &[][..] } else { std::slice::from_raw_parts(list.values, len) };
            if n.format == FORMAT_NODE_ARRAY {
                Value::Array(values.iter().map(|v| node_to_json(v)).collect())
            } else {
                let keys = std::slice::from_raw_parts(list.keys, len);
                let mut map = serde_json::Map::new();
                for (k, v) in keys.iter().zip(values) {
                    map.insert(CStr::from_ptr(*k).to_string_lossy().into_owned(), node_to_json(v));
                }
                Value::Object(map)
            }
        }
        _ => Value::Null,
    }
}

/// Événements utiles du lecteur.
pub enum Event {
    /// Propriété observée modifiée (valeur Null si indisponible).
    Property(String, Value),
    FileLoaded,
    /// Fin d'un fichier : `eof` = arrivé au bout, `error` = lecture impossible.
    EndFile { eof: bool, error: Option<String> },
    /// Lecture (re)partie après un chargement ou un saut.
    Restart,
    Shutdown,
}

/// Le lecteur mpv. Détruit (mpv_terminate_destroy) quand la dernière référence disparaît.
pub struct Mpv {
    api: &'static Api,
    h: Handle,
}

// SAFETY : l'API client de libmpv est utilisable depuis plusieurs threads.
unsafe impl Send for Mpv {}
unsafe impl Sync for Mpv {}

impl Mpv {
    pub fn new() -> Result<Self> {
        let api = api()?;
        // SAFETY : appel sans argument, retour vérifié.
        let h = unsafe { (api.create)() };
        if h.is_null() {
            return Err(anyhow!("{}", tr("libmpv n'a pas pu créer de lecteur")));
        }
        Ok(Mpv { api, h })
    }

    fn check(&self, code: c_int, what: &str) -> Result<()> {
        if code >= 0 {
            return Ok(());
        }
        // SAFETY : mpv_error_string renvoie une chaîne statique.
        let msg = unsafe { CStr::from_ptr((self.api.error_string)(code)) }.to_string_lossy();
        Err(anyhow!("mpv : {what} : {msg}"))
    }

    /// Option à poser avant `initialize`. Une option inconnue n'est pas fatale.
    pub fn set_option(&self, name: &str, value: &str) {
        let (n, v) = (cstr(name), cstr(value));
        // SAFETY : chaînes valides le temps de l'appel.
        let code = unsafe { (self.api.set_option_string)(self.h, n.as_ptr(), v.as_ptr()) };
        if let Err(e) = self.check(code, &format!("option {name}={value}")) {
            eprintln!("turtlefin : {e}");
        }
    }

    pub fn initialize(&self) -> Result<()> {
        // SAFETY : handle valide.
        self.check(unsafe { (self.api.initialize)(self.h) }, "initialisation")
    }

    pub fn command(&self, args: &[&str]) -> Result<()> {
        let owned: Vec<CString> = args.iter().map(|a| cstr(a)).collect();
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        // SAFETY : tableau terminé par NULL, chaînes valides le temps de l'appel.
        self.check(unsafe { (self.api.command)(self.h, ptrs.as_mut_ptr()) }, args.first().copied().unwrap_or(""))
    }

    pub fn set_property(&self, name: &str, value: &str) -> Result<()> {
        let (n, v) = (cstr(name), cstr(value));
        // SAFETY : chaînes valides le temps de l'appel.
        self.check(unsafe { (self.api.set_property_string)(self.h, n.as_ptr(), v.as_ptr()) }, name)
    }

    pub fn observe(&self, name: &str) {
        let n = cstr(name);
        // SAFETY : chaîne valide le temps de l'appel (mpv la copie).
        let code = unsafe { (self.api.observe_property)(self.h, 0, n.as_ptr(), FORMAT_NODE) };
        let _ = self.check(code, name);
    }

    /// Attend le prochain événement (bloquant). À n'appeler que depuis UN thread.
    pub fn wait_event(&self, timeout: f64) -> Option<Event> {
        // SAFETY : l'événement reste valide jusqu'au prochain mpv_wait_event sur ce handle.
        unsafe {
            let ev = &*(self.api.wait_event)(self.h, timeout);
            match ev.event_id {
                EVENT_SHUTDOWN => Some(Event::Shutdown),
                EVENT_FILE_LOADED => Some(Event::FileLoaded),
                EVENT_PLAYBACK_RESTART => Some(Event::Restart),
                EVENT_END_FILE => {
                    let ef = &*(ev.data as *const RawEventEndFile);
                    let error = (ef.reason == 4).then(|| {
                        CStr::from_ptr((self.api.error_string)(ef.error)).to_string_lossy().into_owned()
                    });
                    Some(Event::EndFile { eof: ef.reason == 0, error })
                }
                EVENT_PROPERTY_CHANGE => {
                    let p = &*(ev.data as *const RawEventProperty);
                    let name = CStr::from_ptr(p.name).to_string_lossy().into_owned();
                    let value = if p.format == FORMAT_NODE && !p.data.is_null() {
                        node_to_json(&*(p.data as *const Node))
                    } else {
                        Value::Null
                    };
                    Some(Event::Property(name, value))
                }
                _ => None,
            }
        }
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        // SAFETY : dernière référence ; le contexte de rendu (qui tient une référence) est déjà libéré.
        unsafe { (self.api.terminate_destroy)(self.h) }
    }
}

/// Rendu OpenGL de la vidéo. Thread de l'interface uniquement (contexte OpenGL courant).
pub struct Render {
    api: &'static Api,
    ctx: *mut c_void,
    // Fonction appelée par mpv quand une nouvelle image est prête (gardée en vie ici).
    _wake: Box<Box<dyn Fn() + Send + Sync>>,
    // Le lecteur doit survivre au contexte de rendu.
    _mpv: Arc<Mpv>,
}

// ---------------------------------------------------------------------------
// Contournement d'un bug de mpv 0.40 / 0.41 : à chaque image, mpv crée une barrière OpenGL
// (glFenceSync) qu'il ne libère qu'au « swap », que l'API libmpv n'appelle jamais. Elles
// s'accumulent : avec le pilote v3d, chacune occupe un fichier ouvert, et au bout de ~42 s (1024 fichiers)
// le pilote échoue (« MESA: error: Export failed »). Corrigé dans mpv après la 0.41.
//
// En OpenGL ES, on cache à mpv les tampons persistants (glBufferStorageEXT, une option) : ses
// seules barrières sont alors celles des images. Turtlefin note celles qu'il crée pendant le
// dessin d'une image et libère, après coup, celles qu'il n'a pas libérées lui-même. Avec un mpv
// corrigé, il n'y en a simplement aucune.
// ---------------------------------------------------------------------------
type FenceSyncFn = unsafe extern "system" fn(u32, u32) -> *mut c_void;
type DeleteSyncFn = unsafe extern "system" fn(*mut c_void);

struct FenceHooks {
    real_fence: Option<FenceSyncFn>,
    real_delete: Option<DeleteSyncFn>,
    /// Barrières créées par mpv et pas encore libérées.
    live: Vec<usize>,
}

// Le rendu se fait sur le seul thread de l'interface : le verrou n'est jamais disputé.
static HOOKS: std::sync::Mutex<FenceHooks> =
    std::sync::Mutex::new(FenceHooks { real_fence: None, real_delete: None, live: Vec::new() });

unsafe extern "system" fn fence_sync_hook(condition: u32, flags: u32) -> *mut c_void {
    let mut h = HOOKS.lock().unwrap();
    let Some(real) = h.real_fence else { return std::ptr::null_mut() };
    let sync = real(condition, flags);
    if !sync.is_null() {
        h.live.push(sync as usize);
    }
    sync
}

unsafe extern "system" fn delete_sync_hook(sync: *mut c_void) {
    let real = {
        let mut h = HOOKS.lock().unwrap();
        h.live.retain(|s| *s != sync as usize);
        h.real_delete
    };
    if let Some(real) = real {
        real(sync);
    }
}

/// Libère les barrières que mpv a laissées derrière lui pendant le dessin d'une image.
fn release_leaked_fences() {
    let (leaked, real) = {
        let mut h = HOOKS.lock().unwrap();
        (std::mem::take(&mut h.live), h.real_delete)
    };
    if let Some(real) = real {
        for s in leaked {
            // SAFETY : barrière créée par glFenceSync dans ce contexte, jamais libérée.
            unsafe { real(s as *mut c_void) };
        }
    }
}

/// Ce que reçoit get_proc_trampoline : la fonction de Slint et l'état du contournement.
struct ProcCtx<'a> {
    get_proc_address: &'a dyn Fn(&CStr) -> *const c_void,
    fence_workaround: bool,
}

unsafe extern "C" fn get_proc_trampoline(ctx: *mut c_void, name: *const c_char) -> *mut c_void {
    let c = &*(ctx as *const ProcCtx);
    let name = CStr::from_ptr(name);
    if c.fence_workaround {
        match name.to_bytes() {
            b"glBufferStorageEXT" | b"glBufferStorage" => return std::ptr::null_mut(),
            b"glFenceSync" => {
                let real = (c.get_proc_address)(name);
                if real.is_null() {
                    return std::ptr::null_mut();
                }
                HOOKS.lock().unwrap().real_fence = Some(std::mem::transmute::<*const c_void, FenceSyncFn>(real));
                return fence_sync_hook as *mut c_void;
            }
            b"glDeleteSync" => {
                let real = (c.get_proc_address)(name);
                if real.is_null() {
                    return std::ptr::null_mut();
                }
                HOOKS.lock().unwrap().real_delete = Some(std::mem::transmute::<*const c_void, DeleteSyncFn>(real));
                return delete_sync_hook as *mut c_void;
            }
            _ => {}
        }
    }
    (c.get_proc_address)(name) as *mut c_void
}

/// Le contexte OpenGL courant est-il OpenGL ES ?
fn is_gles(get_proc_address: &dyn Fn(&CStr) -> *const c_void) -> bool {
    const GL_VERSION: u32 = 0x1F02;
    let p = get_proc_address(c"glGetString");
    if p.is_null() {
        return false;
    }
    // SAFETY : glGetString(GL_VERSION) avec le contexte courant ; chaîne statique du pilote.
    unsafe {
        let get_string = std::mem::transmute::<*const c_void, unsafe extern "system" fn(u32) -> *const c_char>(p);
        let v = get_string(GL_VERSION);
        !v.is_null() && CStr::from_ptr(v).to_bytes().starts_with(b"OpenGL ES")
    }
}

unsafe extern "C" fn wake_trampoline(ctx: *mut c_void) {
    let f = &*(ctx as *const Box<dyn Fn() + Send + Sync>);
    f();
}

impl Render {
    /// `get_proc_address` : celui que Slint fournit (n'est utilisé que pendant cet appel).
    /// `on_frame` : appelé depuis un thread de mpv quand il faut redessiner.
    pub fn new(
        mpv: Arc<Mpv>,
        get_proc_address: &dyn Fn(&CStr) -> *const c_void,
        on_frame: Box<dyn Fn() + Send + Sync>,
    ) -> Result<Self> {
        let api = mpv.api;
        let api_type = cstr("opengl");
        let fence_workaround = is_gles(get_proc_address);
        {
            let mut h = HOOKS.lock().unwrap();
            h.live.clear();
            h.real_fence = None;
            h.real_delete = None;
        }
        let proc_ctx = ProcCtx { get_proc_address, fence_workaround };
        let mut init = OpenGlInitParams {
            get_proc_address: Some(get_proc_trampoline),
            get_proc_address_ctx: &proc_ctx as *const ProcCtx as *mut c_void,
        };
        let mut params = [
            RenderParam { kind: RENDER_PARAM_API_TYPE, data: api_type.as_ptr() as *mut c_void },
            RenderParam { kind: RENDER_PARAM_OPENGL_INIT_PARAMS, data: &mut init as *mut _ as *mut c_void },
            RenderParam { kind: RENDER_PARAM_INVALID, data: std::ptr::null_mut() },
        ];
        let mut ctx: *mut c_void = std::ptr::null_mut();
        // SAFETY : paramètres valides le temps de l'appel ; le contexte OpenGL est courant.
        let code = unsafe { (api.render_create)(&mut ctx, mpv.h, params.as_mut_ptr()) };
        mpv.check(code, "création du rendu OpenGL")?;
        let wake = Box::new(on_frame);
        // SAFETY : `wake` vit aussi longtemps que le contexte (champ de la structure, retiré avant libération).
        unsafe {
            (api.render_set_update_callback)(ctx, Some(wake_trampoline), &*wake as *const _ as *mut c_void);
        }
        Ok(Render { api, ctx, _wake: wake, _mpv: mpv })
    }

    /// Prend connaissance d'une nouvelle image sans la dessiner (diagnostic).
    pub fn acknowledge(&self) {
        // SAFETY : contexte valide.
        unsafe { (self.api.render_update)(self.ctx) };
    }

    /// Dessine l'image courante dans le framebuffer `fbo` (taille w x h).
    pub fn render(&self, fbo: u32, w: i32, h: i32) {
        let mut target = OpenGlFbo { fbo: fbo as c_int, w, h, internal_format: 0 };
        let mut flip: c_int = 0;
        let mut params = [
            RenderParam { kind: RENDER_PARAM_OPENGL_FBO, data: &mut target as *mut _ as *mut c_void },
            RenderParam { kind: RENDER_PARAM_FLIP_Y, data: &mut flip as *mut _ as *mut c_void },
            RenderParam { kind: RENDER_PARAM_INVALID, data: std::ptr::null_mut() },
        ];
        // SAFETY : contexte valide, paramètres valides le temps de l'appel.
        unsafe {
            (self.api.render_update)(self.ctx);
            (self.api.render_render)(self.ctx, params.as_mut_ptr());
        }
        release_leaked_fences();
    }
}

impl Drop for Render {
    fn drop(&mut self) {
        // SAFETY : on retire le rappel avant de libérer le contexte (contexte OpenGL courant).
        unsafe {
            (self.api.render_set_update_callback)(self.ctx, None, std::ptr::null_mut());
            (self.api.render_free)(self.ctx);
        }
        release_leaked_fences();
    }
}
