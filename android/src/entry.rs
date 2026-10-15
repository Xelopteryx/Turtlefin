//! Turtlefin pour Android : point d'entrée lancé par NativeActivity (moteur Android de Slint).
//! Module de src/main.rs, compilé seulement pour Android (voir android/Cargo.toml).

use std::ffi::{c_char, c_int, CString};
use std::io::BufRead;
use std::os::fd::FromRawFd;

#[link(name = "log")]
extern "C" {
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
}

/// Messages de Turtlefin (`eprintln!`, panics) vers le journal d'Android : `adb logcat -s turtlefin`.
fn stderr_to_logcat() {
    let mut fds = [0 as c_int; 2];
    // SAFETY : tableau de deux descripteurs ; la sortie d'erreur est remplacée par l'entrée du tube.
    if unsafe { pipe(fds.as_mut_ptr()) } != 0 || unsafe { dup2(fds[1], 2) } < 0 {
        return;
    }
    // SAFETY : descripteur de lecture du tube, à nous seuls.
    let read = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    std::thread::spawn(move || {
        let tag = CString::new("turtlefin").unwrap();
        for line in std::io::BufReader::new(read).lines().map_while(Result::ok) {
            let text = CString::new(line.replace('\0', " ")).unwrap_or_default();
            // SAFETY : chaînes terminées par zéro, valides pendant l'appel. 4 = ANDROID_LOG_INFO.
            unsafe { __android_log_write(4, tag.as_ptr(), text.as_ptr()) };
        }
    });
}

extern "C" {
    fn __system_property_get(name: *const c_char, value: *mut c_char) -> c_int;
}

/// Essais : variables d'environnement données par une propriété d'Android, comme sur le bureau
/// (`adb shell setprop debug.turtlefin.env "TURTLEFIN_HWDEC=no;TURTLEFIN_DEBUG_TOUCH=1"`).
fn env_from_property() {
    let mut buf = [0 as c_char; 92]; // PROP_VALUE_MAX
    let name = CString::new("debug.turtlefin.env").unwrap();
    // SAFETY : tampon de PROP_VALUE_MAX octets, comme l'exige l'appel.
    let n = unsafe { __system_property_get(name.as_ptr(), buf.as_mut_ptr()) };
    if n <= 0 {
        return;
    }
    let bytes: Vec<u8> = buf[..n as usize].iter().map(|&c| c as u8).collect();
    for pair in String::from_utf8_lossy(&bytes).split(';') {
        if let Some((k, v)) = pair.split_once('=') {
            if k.starts_with("TURTLEFIN_") || k.starts_with("SLINT_") {
                std::env::set_var(k.trim(), v.trim());
            }
        }
    }
}

#[no_mangle]
fn android_main(a: slint::android::AndroidApp) {
    stderr_to_logcat();
    env_from_property();
    std::panic::set_hook(Box::new(|p| eprintln!("turtlefin : PANIQUE {p}")));
    // Dossiers de l'application : configuration et cache en mémoire interne (privée), téléchargements
    // en mémoire externe propre à l'appli (plus de place, pas d'autorisation à demander).
    if let Some(files) = a.internal_data_path() {
        std::env::set_var("HOME", &files);
        std::env::set_var("TURTLEFIN_CONFIG_DIR", files.join("config"));
        let cache = files.parent().map(|p| p.join("cache")).unwrap_or_else(|| files.join("cache"));
        std::env::set_var("TURTLEFIN_CACHE_DIR", cache);
        let data = a.external_data_path().unwrap_or_else(|| files.clone());
        std::env::set_var("TURTLEFIN_DATA_DIR", data);
    }
    if let Err(e) = slint::android::init(a) {
        eprintln!("turtlefin : moteur Android de Slint : {e}");
        return;
    }
    if let Err(e) = crate::main() {
        eprintln!("turtlefin : {e:#}");
    }
}
