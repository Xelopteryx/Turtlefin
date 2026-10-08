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
const MOVE_TIMER: usize = 0x7475;

type WndProc = unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize;

#[link(name = "user32")]
extern "system" {
    fn SetWindowLongPtrW(h: Hwnd, index: i32, value: isize) -> isize;
    fn CallWindowProcW(prev: WndProc, h: Hwnd, msg: u32, wp: usize, lp: isize) -> isize;
    fn SetTimer(h: Hwnd, id: usize, ms: u32, f: *const c_void) -> usize;
    fn KillTimer(h: Hwnd, id: usize) -> i32;
}

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
