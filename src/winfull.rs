//! Windows : « plein écran » de l'interface TV sans le vrai plein écran.
//!
//! Une fenêtre OpenGL en plein écran est prise pour un jeu par certains pilotes : AMD Software affiche
//! alors « Appuyez sur ALT + R pour ouvrir » chaque fois que Turtlefin reprend le premier plan. On pose
//! donc une fenêtre sans bordure qui couvre l'écran et le dépasse d'un pixel en bas : visuellement
//! identique, mais plus reconnue comme un jeu en plein écran. La fenêtre elle-même est réglée par Slint
//! (`no-frame`, position, taille, voir `set_tv_window` dans main.rs) ; ce module donne l'écran.

use std::ffi::c_void;

type Hwnd = *mut c_void;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct MonitorInfo {
    size: u32,
    monitor: Rect,
    work: Rect,
    flags: u32,
}

const MONITOR_DEFAULTTONEAREST: u32 = 2;

#[link(name = "user32")]
extern "system" {
    fn EnumWindows(cb: extern "system" fn(Hwnd, isize) -> i32, param: isize) -> i32;
    fn GetWindowThreadProcessId(h: Hwnd, pid: *mut u32) -> u32;
    fn IsWindowVisible(h: Hwnd) -> i32;
    fn GetWindowRect(h: Hwnd, r: *mut Rect) -> i32;
    fn MonitorFromWindow(h: Hwnd, flags: u32) -> *mut c_void;
    fn GetMonitorInfoW(m: *mut c_void, info: *mut MonitorInfo) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcessId() -> u32;
}

/// Plus grande fenêtre visible de ce processus (la fenêtre principale de Slint).
fn main_window() -> Option<Hwnd> {
    struct Search {
        pid: u32,
        best: Hwnd,
        area: i64,
    }
    extern "system" fn each(h: Hwnd, param: isize) -> i32 {
        // SAFETY : `param` pointe sur la `Search` de l'appelant pendant tout EnumWindows.
        let s = unsafe { &mut *(param as *mut Search) };
        let mut pid = 0;
        let mut r = Rect::default();
        // SAFETY : appels Win32 sur une fenêtre fournie par EnumWindows.
        unsafe {
            GetWindowThreadProcessId(h, &mut pid);
            if pid == s.pid && IsWindowVisible(h) != 0 && GetWindowRect(h, &mut r) != 0 {
                let area = (r.right - r.left) as i64 * (r.bottom - r.top) as i64;
                if area > s.area {
                    s.area = area;
                    s.best = h;
                }
            }
        }
        1
    }
    // SAFETY : sans argument.
    let mut s = Search { pid: unsafe { GetCurrentProcessId() }, best: std::ptr::null_mut(), area: 0 };
    // SAFETY : rappel et paramètre valides pendant l'appel.
    unsafe { EnumWindows(each, &mut s as *mut Search as isize) };
    (!s.best.is_null()).then_some(s.best)
}

/// Rectangle de l'écran (pixels physiques) où se trouve Turtlefin, et sa zone de travail (sans la
/// barre des tâches) : (x, y, largeur, hauteur) chacun.
pub fn monitor() -> Option<((i32, i32, i32, i32), (i32, i32, i32, i32))> {
    let h = main_window()?;
    // SAFETY : fenêtre de ce processus, structure initialisée.
    unsafe {
        let mut info = MonitorInfo { size: std::mem::size_of::<MonitorInfo>() as u32, monitor: Rect::default(), work: Rect::default(), flags: 0 };
        if GetMonitorInfoW(MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST), &mut info) == 0 {
            return None;
        }
        let r = |r: Rect| (r.left, r.top, r.right - r.left, r.bottom - r.top);
        Some((r(info.monitor), r(info.work)))
    }
}

// ---------------------------------------------------------------------------------------------
// Fenêtre déplacée ou redimensionnée à la souris : Windows fait alors tourner sa propre boucle
// (« modale ») jusqu'au relâchement, et celle de Turtlefin ne reçoit plus rien : animations,
// minuteries et rendu se figent. Pendant ce temps, une minuterie Windows (WM_TIMER, que la boucle
// modale distribue) fait avancer Slint et redessiner la fenêtre.

const GWLP_WNDPROC: i32 = -4;
const WM_TIMER: u32 = 0x0113;
const WM_ENTERSIZEMOVE: u32 = 0x0231;
const WM_EXITSIZEMOVE: u32 = 0x0232;
const WM_SIZE: u32 = 0x0005;
const SIZE_MAXIMIZED: usize = 2;
const MOVE_TIMER: usize = 0x7475;

type WndProc = unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize;

#[link(name = "user32")]
extern "system" {
    fn SetWindowLongPtrW(h: Hwnd, index: i32, value: isize) -> isize;
    fn CallWindowProcW(prev: WndProc, h: Hwnd, msg: u32, wp: usize, lp: isize) -> isize;
    fn SetTimer(h: Hwnd, id: usize, ms: u32, f: *const c_void) -> usize;
    fn KillTimer(h: Hwnd, id: usize) -> i32;
    fn RedrawWindow(h: Hwnd, rect: *const Rect, rgn: *mut c_void, flags: u32) -> i32;
}

const RDW_INVALIDATE: u32 = 0x0001;
const RDW_UPDATENOW: u32 = 0x0100;

static PREV_PROC: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

thread_local! {
    static ON_TICK: std::cell::RefCell<Option<Box<dyn Fn()>>> = const { std::cell::RefCell::new(None) };
}

unsafe extern "system" fn move_proc(h: Hwnd, msg: u32, wp: usize, lp: isize) -> isize {
    match msg {
        WM_ENTERSIZEMOVE => {
            // SAFETY : fenêtre valide (message reçu pour elle).
            unsafe { SetTimer(h, MOVE_TIMER, 16, std::ptr::null()) };
        }
        WM_EXITSIZEMOVE => {
            // SAFETY : idem.
            unsafe { KillTimer(h, MOVE_TIMER) };
        }
        WM_SIZE => {
            // Nouvelle taille : traitée d'abord (Slint la prend), puis l'image est refaite tout de suite,
            // avant que Windows n'affiche la fenêtre ; sinon il étire l'ancienne image à la nouvelle
            // taille le temps d'un rendu (bords du cadre dédoublés, barres claires pendant un
            // redimensionnement lent).
            let prev = PREV_PROC.load(std::sync::atomic::Ordering::Relaxed);
            // SAFETY : procédure d'origine de la fenêtre (posée par `keep_alive_while_moving`).
            let r = unsafe { CallWindowProcW(std::mem::transmute::<isize, WndProc>(prev), h, msg, wp, lp) };
            ON_TICK.with(|t| {
                if let Some(f) = t.borrow().as_ref() {
                    f();
                }
            });
            // SAFETY : fenêtre valide (message reçu pour elle).
            unsafe { RedrawWindow(h, std::ptr::null(), std::ptr::null_mut(), RDW_INVALIDATE | RDW_UPDATENOW) };
            if ROUND.load(std::sync::atomic::Ordering::Relaxed) {
                let (w, h2) = ((lp & 0xffff) as i32, ((lp >> 16) & 0xffff) as i32);
                round_corners(h, if wp == SIZE_MAXIMIZED { None } else { Some((w, h2)) });
            }
            return r;
        }
        WM_TIMER if wp == MOVE_TIMER => {
            ON_TICK.with(|t| {
                if let Some(f) = t.borrow().as_ref() {
                    f();
                }
            });
            return 0;
        }
        _ => {}
    }
    let prev = PREV_PROC.load(std::sync::atomic::Ordering::Relaxed);
    // SAFETY : `prev` est la procédure d'origine de la fenêtre (posée par `keep_alive_while_moving`).
    unsafe { CallWindowProcW(std::mem::transmute::<isize, WndProc>(prev), h, msg, wp, lp) }
}

/// Garde l'interface vivante pendant un déplacement ou un redimensionnement : `tick` est appelé
/// toutes les 16 ms pendant ce temps (il fait avancer Slint et demande un rendu).
pub fn keep_alive_while_moving(tick: impl Fn() + 'static) {
    if PREV_PROC.load(std::sync::atomic::Ordering::Relaxed) != 0 {
        return;
    }
    let Some(h) = main_window() else { return };
    ON_TICK.with(|t| *t.borrow_mut() = Some(Box::new(tick)));
    // SAFETY : fenêtre de ce processus ; la procédure d'origine est gardée et toujours appelée.
    let prev = unsafe { SetWindowLongPtrW(h, GWLP_WNDPROC, move_proc as *const () as usize as isize) };
    PREV_PROC.store(prev, std::sync::atomic::Ordering::Relaxed);
}

/// Verre flou de Windows 7 sous Windows 10 / 11 : ce qui est derrière la fenêtre est flouté par le
/// gestionnaire de fenêtres là où Turtlefin peint en semi-transparent (cadre de la fenêtre façon
/// Windows 7, voir WinCaption dans app.slint). API non documentée mais stable depuis Windows 10
/// (SetWindowCompositionAttribute, WCA_ACCENT_POLICY) ; sans effet si elle manque.
pub fn set_blur(hwnd: *mut c_void, on: bool) {
    #[repr(C)]
    struct AccentPolicy {
        state: u32,
        flags: u32,
        gradient: u32,
        animation: u32,
    }
    #[repr(C)]
    struct Data {
        attrib: u32,
        data: *mut c_void,
        size: usize,
    }
    type SetWca = unsafe extern "system" fn(Hwnd, *mut Data) -> i32;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> *mut c_void;
        fn GetProcAddress(m: *mut c_void, name: *const u8) -> *mut c_void;
    }
    const ACCENT_DISABLED: u32 = 0;
    const ACCENT_ENABLE_BLURBEHIND: u32 = 3;
    const WCA_ACCENT_POLICY: u32 = 19;
    let user32: Vec<u16> = "user32.dll\0".encode_utf16().collect();
    // SAFETY : fonction cherchée dans user32 (déjà chargée) ; structure à la disposition attendue,
    // valable pendant l'appel ; fenêtre de ce processus.
    unsafe {
        let m = GetModuleHandleW(user32.as_ptr());
        if m.is_null() {
            return;
        }
        let f = GetProcAddress(m, b"SetWindowCompositionAttribute\0".as_ptr());
        if f.is_null() {
            return;
        }
        let set: SetWca = std::mem::transmute(f);
        let mut accent = AccentPolicy { state: if on { ACCENT_ENABLE_BLURBEHIND } else { ACCENT_DISABLED }, flags: 0, gradient: 0, animation: 0 };
        let mut data = Data { attrib: WCA_ACCENT_POLICY, data: (&mut accent as *mut AccentPolicy).cast(), size: std::mem::size_of::<AccentPolicy>() };
        set(hwnd, &mut data);
    }
}

// ---------------------------------------------------------------------------------------------
// Coins arrondis du cadre de Windows 7 : la fenêtre est découpée en rectangle aux coins arrondis
// (région de Windows) ; hors de la région, Windows ne montre rien de la fenêtre (vrais coins
// transparents, même là où l'OpenGL n'a pas d'alpha). Refait à chaque changement de taille.

static ROUND: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Rayon des coins, en pixels (celui du contour dessiné, voir WinCaption).
const CORNER: i32 = 6;

#[link(name = "gdi32")]
extern "system" {
    fn CreateRoundRectRgn(x1: i32, y1: i32, x2: i32, y2: i32, w: i32, h: i32) -> *mut c_void;
}

#[link(name = "user32")]
extern "system" {
    fn SetWindowRgn(h: Hwnd, rgn: *mut c_void, redraw: i32) -> i32;
    fn GetClientRect(h: Hwnd, r: *mut Rect) -> i32;
    fn IsZoomed(h: Hwnd) -> i32;
    fn GetDpiForWindow(h: Hwnd) -> u32;
}

fn round_corners(h: Hwnd, size: Option<(i32, i32)>) {
    // SAFETY : fenêtre de ce processus ; la région passe à Windows (qui la libère), ou aucune.
    unsafe {
        match size {
            Some((w, ht)) if w > 0 && ht > 0 => {
                let dpi = GetDpiForWindow(h).max(96) as i32;
                let d = 2 * CORNER * dpi / 96;
                let rgn = CreateRoundRectRgn(0, 0, w + 1, ht + 1, d, d);
                if !rgn.is_null() {
                    SetWindowRgn(h, rgn, 1);
                }
            }
            _ => {
                SetWindowRgn(h, std::ptr::null_mut(), 1);
            }
        }
    }
}

/// Coins arrondis (cadre de Windows 7, fenêtre ni agrandie ni en plein écran) ou fenêtre rectangulaire.
pub fn set_round(hwnd: *mut c_void, on: bool) {
    ROUND.store(on, std::sync::atomic::Ordering::Relaxed);
    let mut r = Rect::default();
    // SAFETY : fenêtre de ce processus, structure initialisée.
    let maxed = unsafe { GetClientRect(hwnd, &mut r) == 0 || IsZoomed(hwnd) != 0 };
    round_corners(hwnd, if on && !maxed { Some((r.right - r.left, r.bottom - r.top)) } else { None });
}
