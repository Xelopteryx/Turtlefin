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

/// Activité Android (référence globale de NativeActivity) : `setRequestedOrientation` est une méthode
/// de l'activité, pas du contexte d'application que donne ndk-context.
static ACTIVITY: std::sync::atomic::AtomicPtr<std::ffi::c_void> = std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

/// Lecture : écran tenu en paysage (dans un sens ou l'autre, selon le capteur) ; hors lecture,
/// l'orientation revient au manifeste (portrait ou paysage, selon le verrouillage de la rotation).
pub fn lock_landscape(on: bool) {
    // ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE / SCREEN_ORIENTATION_UNSPECIFIED.
    let value = if on { 6 } else { -1 };
    let ctx = ndk_context::android_context();
    let act = ACTIVITY.load(std::sync::atomic::Ordering::Relaxed);
    if act.is_null() {
        return;
    }
    // SAFETY : machine virtuelle et activité fournies par android-activity, valables toute la vie
    // du processus (référence globale : jamais libérée ici).
    let result = (|| -> jni::errors::Result<()> {
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { jni::objects::JObject::from_raw(act.cast()) };
        env.call_method(&activity, "setRequestedOrientation", "(I)V", &[jni::objects::JValue::Int(value)])?;
        Ok(())
    })();
    if let Err(e) = result {
        eprintln!("turtlefin : orientation de l'écran : {e}");
    }
}

/// Volume « média » d'Android (STREAM_MUSIC) : celui des touches du téléphone. Le lecteur de Turtlefin
/// règle celui-ci (mpv reste à 100 %), pour qu'un seul volume existe sur le téléphone.
fn with_audio_manager<T>(f: impl FnOnce(&mut jni::JNIEnv, &jni::objects::JObject) -> jni::errors::Result<T>) -> Option<T> {
    let ctx = ndk_context::android_context();
    let act = ACTIVITY.load(std::sync::atomic::Ordering::Relaxed);
    if act.is_null() {
        return None;
    }
    // SAFETY : comme lock_landscape (machine virtuelle et activité valables toute la vie du processus).
    let result = (|| -> jni::errors::Result<T> {
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { jni::objects::JObject::from_raw(act.cast()) };
        env.with_local_frame(8, |env| {
            let name = env.new_string("audio")?;
            let am = env
                .call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&name).into()])?
                .l()?;
            f(env, &am)
        })
    })();
    match result {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("turtlefin : volume d'Android : {e}");
            None
        }
    }
}

const STREAM_MUSIC: i32 = 3;

/// Volume média actuel, de 0 à 100.
pub fn system_volume() -> Option<i32> {
    with_audio_manager(|env, am| {
        let max = env.call_method(am, "getStreamMaxVolume", "(I)I", &[STREAM_MUSIC.into()])?.i()?;
        let cur = env.call_method(am, "getStreamVolume", "(I)I", &[STREAM_MUSIC.into()])?.i()?;
        Ok(if max > 0 { (cur * 100 + max / 2) / max } else { 100 })
    })
}

/// Règle le volume média (0 à 100, arrondi au cran d'Android le plus proche), sans la fenêtre de
/// volume d'Android (la barre de Turtlefin est à l'écran).
pub fn set_system_volume(percent: i32) {
    with_audio_manager(|env, am| {
        let max = env.call_method(am, "getStreamMaxVolume", "(I)I", &[STREAM_MUSIC.into()])?.i()?;
        let step = (percent.clamp(0, 100) * max + 50) / 100;
        env.call_method(am, "setStreamVolume", "(III)V", &[STREAM_MUSIC.into(), step.into(), 0.into()])?;
        Ok(())
    });
}

#[no_mangle]
fn android_main(a: slint::android::AndroidApp) {
    stderr_to_logcat();
    env_from_property();
    ACTIVITY.store(a.activity_as_ptr(), std::sync::atomic::Ordering::Relaxed);
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
