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

/// Android TV (ou boîtier sans écran tactile) : interface TV, pilotée à la télécommande. Un téléphone
/// ou une tablette restent en interface tactile (le réglage « Interface TV » n'y est pas proposé).
pub fn is_tv() -> bool {
    let ctx = ndk_context::android_context();
    let act = ACTIVITY.load(std::sync::atomic::Ordering::Relaxed);
    if act.is_null() {
        return false;
    }
    // SAFETY : comme lock_landscape.
    let result = (|| -> jni::errors::Result<bool> {
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { jni::objects::JObject::from_raw(act.cast()) };
        env.with_local_frame(8, |env| {
            let pm = env.call_method(&activity, "getPackageManager", "()Landroid/content/pm/PackageManager;", &[])?.l()?;
            let mut has = |name: &str| -> jni::errors::Result<bool> {
                let s = env.new_string(name)?;
                env.call_method(&pm, "hasSystemFeature", "(Ljava/lang/String;)Z", &[(&s).into()])?.z()
            };
            Ok(has("android.software.leanback")? || !has("android.hardware.touchscreen")?)
        })
    })();
    result.unwrap_or(false)
}

/// Recherche d'écrans (DLNA, Google Cast) : Android ignore les réponses multidiffusées tant qu'aucune
/// appli ne tient un « verrou multicast » du Wi-Fi. Il est pris pendant `secs` secondes seulement (il
/// coûte de la batterie), dans un fil à part.
pub fn multicast_for(secs: u64) {
    let act = ACTIVITY.load(std::sync::atomic::Ordering::Relaxed) as usize;
    if act == 0 {
        return;
    }
    std::thread::spawn(move || {
        let ctx = ndk_context::android_context();
        // SAFETY : comme lock_landscape (machine virtuelle et activité valables toute la vie du processus).
        let result = (|| -> jni::errors::Result<()> {
            let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
            let mut env = vm.attach_current_thread()?;
            let activity = unsafe { jni::objects::JObject::from_raw(act as *mut _) };
            env.with_local_frame(16, |env| {
                let name = env.new_string("wifi")?;
                let wifi = env
                    .call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&name).into()])?
                    .l()?;
                let tag = env.new_string("turtlefin")?;
                let lock = env
                    .call_method(&wifi, "createMulticastLock", "(Ljava/lang/String;)Landroid/net/wifi/WifiManager$MulticastLock;", &[(&tag).into()])?
                    .l()?;
                env.call_method(&lock, "setReferenceCounted", "(Z)V", &[false.into()])?;
                env.call_method(&lock, "acquire", "()V", &[])?;
                std::thread::sleep(std::time::Duration::from_secs(secs));
                env.call_method(&lock, "release", "()V", &[])?;
                Ok(())
            })
        })();
        if let Err(e) = result {
            eprintln!("turtlefin : verrou multicast : {e}");
        }
    });
}

#[link(name = "android")]
extern "C" {
    fn android_setsocknetwork(network: u64, fd: std::os::raw::c_int) -> std::os::raw::c_int;
}

/// Réseau local (Wi-Fi, sinon Ethernet) : son identifiant système (Network.getNetworkHandle).
fn lan_network() -> Option<u64> {
    let act = ACTIVITY.load(std::sync::atomic::Ordering::Relaxed) as usize;
    if act == 0 {
        return None;
    }
    let ctx = ndk_context::android_context();
    // SAFETY : comme lock_landscape (machine virtuelle et activité valables toute la vie du processus).
    let result = (|| -> jni::errors::Result<Option<u64>> {
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { jni::objects::JObject::from_raw(act as *mut _) };
        env.with_local_frame(32, |env| {
            let name = env.new_string("connectivity")?;
            let cm = env.call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&name).into()])?.l()?;
            let nets = jni::objects::JObjectArray::from(env.call_method(&cm, "getAllNetworks", "()[Landroid/net/Network;", &[])?.l()?);
            let mut found = None;
            for i in 0..env.get_array_length(&nets)? {
                let net = env.get_object_array_element(&nets, i)?;
                let caps = env.call_method(&cm, "getNetworkCapabilities", "(Landroid/net/Network;)Landroid/net/NetworkCapabilities;", &[(&net).into()])?.l()?;
                if caps.is_null() {
                    continue;
                }
                // 1 : Wi-Fi, 3 : Ethernet, 4 : VPN (exclu).
                let has = |env: &mut jni::JNIEnv, t: i32| env.call_method(&caps, "hasTransport", "(I)Z", &[t.into()]).and_then(|v| v.z());
                if has(env, 4)? {
                    continue;
                }
                let wifi = has(env, 1)?;
                if wifi || has(env, 3)? {
                    let h = env.call_method(&net, "getNetworkHandle", "()J", &[])?.j()? as u64;
                    found = Some(h);
                    if wifi {
                        break;
                    }
                }
            }
            Ok(found)
        })
    })();
    result.unwrap_or_else(|e| {
        eprintln!("turtlefin : réseau local : {e}");
        None
    })
}

/// Attache la prise `fd` au réseau local (Wi-Fi) : avec un VPN actif (Tailscale), Android pouvait
/// envoyer les recherches d'écrans et leurs réponses par le VPN, et les écrans n'apparaissaient
/// qu'au hasard. Sans réseau local trouvé, rien n'est changé.
pub fn bind_to_lan(fd: std::os::raw::c_int) {
    if let Some(net) = lan_network() {
        // SAFETY : `fd` est une prise ouverte qui appartient à l'appelant ; seul son réseau change.
        if unsafe { android_setsocknetwork(net, fd) } != 0 {
            eprintln!("turtlefin : prise non attachée au réseau local");
        }
    }
}

/// Icône du lanceur aux couleurs du thème `key` : active son entrée (activity-alias « .Icon_<key> »
/// du manifeste) et désactive les autres, sans fermer l'appli. Rien n'est fait si c'est déjà la bonne.
pub fn set_launcher_icon(key: &str) {
    const KEYS: [&str; 3] = ["turtlefin", "aero", "nerd"];
    let key = if KEYS.contains(&key) { key.to_string() } else { "turtlefin".to_string() };
    let act = ACTIVITY.load(std::sync::atomic::Ordering::Relaxed) as usize;
    if act == 0 {
        return;
    }
    // Hors du fil de l'interface : PackageManager peut prendre un moment.
    std::thread::spawn(move || {
        let ctx = ndk_context::android_context();
        // SAFETY : comme lock_landscape (machine virtuelle et activité valables toute la vie du processus).
        let result = (|| -> jni::errors::Result<()> {
            let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
            let mut env = vm.attach_current_thread()?;
            let activity = unsafe { jni::objects::JObject::from_raw(act as *mut _) };
            env.with_local_frame(32, |env| {
                let pm = env.call_method(&activity, "getPackageManager", "()Landroid/content/pm/PackageManager;", &[])?.l()?;
                let pkg = env.call_method(&activity, "getPackageName", "()Ljava/lang/String;", &[])?.l()?;
                let pkg_s: String = env.get_string(&jni::objects::JString::from(env.new_local_ref(&pkg)?))?.into();
                // La nouvelle d'abord, puis les autres (jamais aucune entrée active).
                let mut order: Vec<&str> = vec![key.as_str()];
                order.extend(KEYS.iter().filter(|k| **k != key));
                for k in order {
                    let cls = env.new_string(format!("{pkg_s}.Icon_{k}"))?;
                    let comp = env.new_object("android/content/ComponentName", "(Ljava/lang/String;Ljava/lang/String;)V", &[(&pkg).into(), (&cls).into()])?;
                    // 1 : activée, 2 : désactivée ; 0 : celle du manifeste (seule « turtlefin » y est activée).
                    let want = if k == key { 1 } else { 2 };
                    let now = env.call_method(&pm, "getComponentEnabledSetting", "(Landroid/content/ComponentName;)I", &[(&comp).into()])?.i()?;
                    let effective = if now == 0 { if k == "turtlefin" { 1 } else { 2 } } else { now };
                    if effective != want {
                        // DONT_KILL_APP (1) : l'appli continue.
                        env.call_method(&pm, "setComponentEnabledSetting", "(Landroid/content/ComponentName;II)V", &[(&comp).into(), want.into(), 1.into()])?;
                    }
                }
                Ok(())
            })
        })();
        if let Err(e) = result {
            eprintln!("turtlefin : icône du thème : {e}");
        }
    });
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

/// Nom du téléphone : celui choisi dans les réglages (« Galaxy A12 », Settings.Global.device_name),
/// sinon le modèle (android.os.Build.MODEL, « SM-A127F »).
fn build_model() -> Option<String> {
    let ctx = ndk_context::android_context();
    let act = ACTIVITY.load(std::sync::atomic::Ordering::Relaxed);
    // SAFETY : machine virtuelle fournie par android-activity, valable toute la vie du processus.
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    if !act.is_null() {
        // SAFETY : comme lock_landscape.
        let activity = unsafe { jni::objects::JObject::from_raw(act.cast()) };
        let named = env.with_local_frame(8, |env| -> jni::errors::Result<Option<String>> {
            let cr = env.call_method(&activity, "getContentResolver", "()Landroid/content/ContentResolver;", &[])?.l()?;
            let key = env.new_string("device_name")?;
            let v = env
                .call_static_method(
                    "android/provider/Settings$Global",
                    "getString",
                    "(Landroid/content/ContentResolver;Ljava/lang/String;)Ljava/lang/String;",
                    &[(&cr).into(), (&key).into()],
                )?
                .l()?;
            if v.is_null() {
                return Ok(None);
            }
            Ok(Some(env.get_string(&jni::objects::JString::from(v))?.into()))
        });
        match named {
            Ok(Some(n)) if !n.trim().is_empty() => return Some(n),
            Err(_) => {
                let _ = env.exception_clear();
            }
            _ => {}
        }
    }
    let v = env.get_static_field("android/os/Build", "MODEL", "Ljava/lang/String;").ok()?.l().ok()?;
    let s: String = env.get_string(&jni::objects::JString::from(v)).ok()?.into();
    Some(s)
}

#[no_mangle]
fn android_main(a: slint::android::AndroidApp) {
    stderr_to_logcat();
    env_from_property();
    ACTIVITY.store(a.activity_as_ptr(), std::sync::atomic::Ordering::Relaxed);
    // Nom de l'appareil (Diffuser : « Redmi Note 12 Pro »…), lu dans android.os.Build.MODEL.
    if let Some(model) = build_model() {
        std::env::set_var("TURTLEFIN_DEVICE_NAME", model);
    }
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
