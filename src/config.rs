//! Session sauvegardée (serveur + jeton) dans le dossier de config de l'OS.
//! Le mot de passe n'est JAMAIS écrit sur disque, seulement le jeton d'accès.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Diffusion en cours : l'écran et la vidéo.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct CastLast {
    pub name: String,
    /// Chromecast : identifiant et adresse (ip:port) ; appli Jellyfin : `session`.
    pub id: String,
    pub addr: String,
    pub session: String,
    pub item: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Saved {
    /// Vidéo en cours de diffusion (pour s'y rebrancher si l'appli a été tuée), sinon rien.
    pub cast_last: Option<CastLast>,
    /// Adresse utilisée pour la session en cours.
    pub server: String,
    /// Adresse principale du serveur (vide si inconnue).
    #[serde(alias = "server_local")]
    pub server_main: String,
    /// Adresse de secours du même serveur, essayée quand la principale ne répond pas (facultative).
    #[serde(alias = "server_remote")]
    pub server_backup: String,
    /// L'adresse de secours a été retirée dans les paramètres : elle n'est plus complétée toute seule.
    pub backup_cleared: bool,
    /// Ancien réglage (adresse distante d'abord) : lu une fois pour ranger les adresses, puis retiré.
    #[serde(skip_serializing)]
    pub prefer_remote: bool,
    /// Affichage : avatars GIF figés sur leur première image (moins de calcul).
    pub still_gifs: bool,
    /// Affichage : pas de fond d'écran tiré du média sélectionné.
    pub no_backdrop: bool,
    /// Identifiant du serveur (pour retrouver les comptes enregistrés).
    pub server_id: String,
    pub user_name: String,
    pub user_id: String,
    #[serde(with = "secret")]
    pub token: String,
    pub device_id: String,
}

/// Jetons d'accès sur le disque. Windows : chiffrés par DPAPI (liés au compte Windows : un
/// `session.json` copié ailleurs ou lu par un autre compte ne donne pas accès au serveur). Ailleurs :
/// en clair, dans des fichiers lisibles du seul utilisateur (0600). Un jeton illisible (fichier venu
/// d'un autre PC) est vide : le compte redemande son mot de passe.
mod secret {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(token: &str, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&protect(token))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
        let v = String::deserialize(d)?;
        // Trousseau illisible pour l'instant : la référence est gardée (elle sera réécrite telle
        // quelle) ; le serveur la refusera et la connexion sera redemandée.
        #[cfg(all(unix, not(target_os = "android")))]
        if let Some(k) = v.strip_prefix(keyring::PREFIX) {
            return Ok(keyring::lookup(k).unwrap_or(v));
        }
        Ok(match v.strip_prefix(PREFIX) {
            Some(b) => unprotect(b).unwrap_or_default(),
            None => v,
        })
    }

    const PREFIX: &str = "dpapi:";

    #[cfg(not(all(unix, not(target_os = "android"))))]
    #[cfg(not(windows))]
    fn protect(t: &str) -> String {
        t.to_string()
    }

    #[cfg(not(windows))]
    fn unprotect(_: &str) -> Option<String> {
        None
    }

    /// Linux (et autres Unix de bureau) : le jeton va dans le trousseau de la session (Secret Service :
    /// GNOME Keyring, KWallet…, chiffré par le mot de passe de session) ; le fichier ne garde qu'une
    /// référence (empreinte du jeton). Sans trousseau (Raspberry Pi sans bureau complet, Flatpak,
    /// version portable) : fichier 0600 comme avant.
    #[cfg(all(unix, not(target_os = "android")))]
    pub(super) mod keyring {
        use std::collections::HashMap;
        use std::io::Write;
        use std::process::Stdio;
        use std::sync::Mutex;
        use std::time::{Duration, Instant};

        pub const PREFIX: &str = "secret-service:";
        const ATTR: &str = "turtlefin-token";

        /// Jetons déjà rangés ou relus pendant cette exécution (empreinte -> jeton) : le trousseau n'est
        /// interrogé qu'une fois par jeton.
        static CACHE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

        fn usable() -> bool {
            crate::paths::portable_root().is_none() && std::env::var_os("TURTLEFIN_NO_KEYRING").is_none()
        }

        pub fn key(token: &str) -> String {
            use sha2::Digest;
            sha2::Sha256::digest(token.as_bytes()).iter().take(16).map(|b| format!("{b:02x}")).collect()
        }

        /// `secret-tool` lancé avec `stdin` en entrée, arrêté au bout de `wait` (fenêtre de
        /// déverrouillage laissée sans réponse) : sa sortie si tout s'est bien passé.
        fn run(args: &[&str], stdin: Option<&str>, wait: Duration) -> Option<String> {
            let mut child = crate::paths::quiet_command("secret-tool")
                .args(args)
                .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .ok()?;
            if let (Some(text), Some(mut input)) = (stdin, child.stdin.take()) {
                let _ = input.write_all(text.as_bytes());
            }
            let end = Instant::now() + wait;
            loop {
                match child.try_wait() {
                    Ok(Some(st)) if st.success() => {
                        let mut out = String::new();
                        std::io::Read::read_to_string(&mut child.stdout.take()?, &mut out).ok()?;
                        return Some(out);
                    }
                    Ok(Some(_)) | Err(_) => return None,
                    Ok(None) if Instant::now() >= end => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return None;
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(15)),
                }
            }
        }

        /// Range le jeton dans le trousseau : la référence à écrire dans le fichier, sinon `None`.
        pub fn store(token: &str) -> Option<String> {
            if !usable() {
                return None;
            }
            let k = key(token);
            let mut cache = CACHE.lock().unwrap();
            let map = cache.get_or_insert_with(HashMap::new);
            if map.get(&k).map(String::as_str) != Some(token) {
                run(&["store", "--label=Turtlefin", ATTR, &k], Some(token), Duration::from_secs(3))?;
                map.insert(k.clone(), token.to_string());
            }
            Some(format!("{PREFIX}{k}"))
        }

        /// Jeton rangé sous cette référence (`None` : trousseau absent, verrouillé, ou jeton retiré).
        pub fn lookup(k: &str) -> Option<String> {
            if let Some(t) = CACHE.lock().unwrap().as_ref().and_then(|m| m.get(k)) {
                return Some(t.clone());
            }
            // Fenêtre de déverrouillage possible : on laisse le temps de taper le mot de passe.
            let t = run(&["lookup", ATTR, k], None, Duration::from_secs(20))?.trim_end_matches('\n').to_string();
            (!t.is_empty() && key(&t) == k).then(|| {
                CACHE.lock().unwrap().get_or_insert_with(HashMap::new).insert(k.to_string(), t.clone());
                t
            })
        }

        /// Jeton retiré du trousseau (compte retiré de l'appareil).
        pub fn forget(token: &str) {
            if token.is_empty() || !usable() {
                return;
            }
            let k = key(token);
            if let Some(m) = CACHE.lock().unwrap().as_mut() {
                m.remove(&k);
            }
            let _ = run(&["clear", ATTR, &k], None, Duration::from_secs(3));
        }
    }

    #[cfg(all(unix, not(target_os = "android")))]
    fn protect(t: &str) -> String {
        // Référence dont le jeton n'a pas pu être relu (trousseau verrouillé) : gardée telle quelle,
        // le jeton sera relu au prochain lancement.
        if t.is_empty() || t.starts_with(keyring::PREFIX) {
            return t.to_string();
        }
        keyring::store(t).unwrap_or_else(|| t.to_string())
    }

    #[cfg(windows)]
    #[repr(C)]
    struct Blob {
        len: u32,
        data: *mut u8,
    }

    #[cfg(windows)]
    #[link(name = "crypt32")]
    extern "system" {
        fn CryptProtectData(input: *const Blob, desc: *const u16, entropy: *const Blob, reserved: *const u8, prompt: *const u8, flags: u32, out: *mut Blob) -> i32;
        fn CryptUnprotectData(input: *const Blob, desc: *mut *mut u16, entropy: *const Blob, reserved: *const u8, prompt: *const u8, flags: u32, out: *mut Blob) -> i32;
    }

    #[cfg(windows)]
    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(mem: *mut u8) -> *mut u8;
    }

    /// Pas de fenêtre (CRYPTPROTECT_UI_FORBIDDEN).
    #[cfg(windows)]
    const UI_FORBIDDEN: u32 = 1;

    #[cfg(windows)]
    fn call(input: &[u8], encrypt: bool) -> Option<Vec<u8>> {
        let blob = Blob { len: input.len() as u32, data: input.as_ptr() as *mut u8 };
        let mut out = Blob { len: 0, data: std::ptr::null_mut() };
        let (nb, nu): (*const Blob, *const u8) = (std::ptr::null(), std::ptr::null());
        // SAFETY : tampons valides pendant l'appel ; la sortie, allouée par Windows, est copiée puis libérée.
        unsafe {
            let ok = if encrypt {
                CryptProtectData(&blob, std::ptr::null(), nb, nu, nu, UI_FORBIDDEN, &mut out)
            } else {
                CryptUnprotectData(&blob, std::ptr::null_mut(), nb, nu, nu, UI_FORBIDDEN, &mut out)
            };
            if ok == 0 || out.data.is_null() {
                return None;
            }
            let v = std::slice::from_raw_parts(out.data, out.len as usize).to_vec();
            LocalFree(out.data);
            Some(v)
        }
    }

    #[cfg(windows)]
    fn protect(t: &str) -> String {
        use base64::Engine;
        if t.is_empty() {
            return String::new();
        }
        match call(t.as_bytes(), true) {
            Some(v) => format!("{PREFIX}{}", base64::engine::general_purpose::STANDARD.encode(v)),
            None => t.to_string(),
        }
    }

    #[cfg(windows)]
    fn unprotect(b: &str) -> Option<String> {
        use base64::Engine;
        let raw = base64::engine::general_purpose::STANDARD.decode(b).ok()?;
        String::from_utf8(call(&raw, false)?).ok()
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn round_trip() {
            let p = super::protect("abc123");
            match p.strip_prefix(super::PREFIX) {
                Some(b) => assert_eq!(super::unprotect(b).as_deref(), Some("abc123")),
                None => assert_eq!(p, "abc123"),
            }
            assert_eq!(super::protect(""), "");
        }

        /// Trousseau imité par un faux `secret-tool` (un fichier par jeton) : rangement, relecture,
        /// retrait ; trousseau absent : fichier comme avant, référence illisible gardée telle quelle.
        #[cfg(all(unix, not(target_os = "android")))]
        #[test]
        fn keyring_round_trip() {
            use std::os::unix::fs::PermissionsExt;
            let dir = std::env::temp_dir().join(format!("tf-keyring-{}", std::process::id()));
            let store = dir.join("store");
            std::fs::create_dir_all(&store).unwrap();
            let fake = dir.join("secret-tool");
            std::fs::write(
                &fake,
                format!(
                    "#!/bin/sh\nS={}\ncase \"$1\" in\n store) cat > \"$S/$4\" ;;\n lookup) cat \"$S/$3\" 2>/dev/null || exit 1 ;;\n clear) rm -f \"$S/$3\" ;;\nesac\n",
                    store.display()
                ),
            )
            .unwrap();
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
            let old_path = std::env::var_os("PATH").unwrap_or_default();
            std::env::set_var("PATH", format!("{}:{}", dir.display(), old_path.to_string_lossy()));

            let token = "0123456789abcdef0123456789abcdef";
            let r = super::protect(token);
            let k = r.strip_prefix(super::keyring::PREFIX).expect("rangé dans le trousseau").to_string();
            assert!(!r.contains(token));
            assert_eq!(std::fs::read_to_string(store.join(&k)).unwrap(), token);
            // Relecture sans le cache de cette exécution.
            super::keyring::forget(token);
            assert_eq!(super::keyring::lookup(&k), None);
            std::fs::write(store.join(&k), token).unwrap();
            assert_eq!(super::keyring::lookup(&k).as_deref(), Some(token));
            let back: String = super::deserialize(serde_json::Value::String(r.clone())).unwrap();
            assert_eq!(back, token);
            // Retrait.
            super::keyring::forget(token);
            assert!(!store.join(&k).exists());
            // Référence illisible : gardée telle quelle, réécrite telle quelle.
            let lost: String = super::deserialize(serde_json::Value::String(r.clone())).unwrap();
            assert_eq!(lost, r);
            assert_eq!(super::protect(&lost), r);
            // Pas de trousseau : le jeton reste dans le fichier.
            std::env::set_var("PATH", "/nonexistent");
            assert_eq!(super::protect("autre-jeton"), "autre-jeton");
            std::env::set_var("PATH", old_path);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

/// Écrit un fichier de configuration d'un coup (fichier provisoire puis renommage : une coupure de
/// courant ne laisse pas un fichier à moitié écrit). Sous Unix, `private` : lisible du seul
/// utilisateur dès sa création (pas d'instant où un autre compte pourrait lire le jeton).
pub fn write_atomic(p: &std::path::Path, text: &str, private: bool) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
        // Dossier de Turtlefin (jamais un dossier choisi par l'utilisateur) : fermé aux autres comptes.
        #[cfg(unix)]
        if private {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
    }
    let tmp = p.with_extension("json.tmp");
    let _ = std::fs::remove_file(&tmp);
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(if private { 0o600 } else { 0o644 });
    }
    let _ = private;
    let mut f = o.open(&tmp)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, p)
}

/// Dossier de configuration (voir `paths`).
fn config_dir() -> Option<PathBuf> {
    crate::paths::config_dir()
}

fn path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("session.json"))
}

/// Vérification du démarrage, à faire avant toute lecture (un fichier abîmé serait remplacé par les
/// valeurs par défaut au premier enregistrement) : noms des fichiers de configuration présents mais
/// illisibles (JSON invalide ou lecture impossible).
pub fn unreadable_files() -> Vec<&'static str> {
    let Some(dir) = config_dir() else { return Vec::new() };
    ["session.json", "accounts.json", "prefs.json", "tracks.json", "userdata.json"]
        .into_iter()
        .filter(|name| {
            let p = dir.join(name);
            p.exists()
                && !std::fs::read_to_string(&p)
                    .ok()
                    .is_some_and(|t| serde_json::from_str::<serde_json::Value>(t.trim_start_matches('\u{feff}')).is_ok())
        })
        .collect()
}

/// Charge la session ; crée et enregistre un identifiant d'appareil au premier lancement.
pub fn load() -> Saved {
    let mut s: Saved = path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default();
    let mut dirty = false;
    if s.prefer_remote {
        // L'adresse « distante » était préférée : elle devient la principale.
        std::mem::swap(&mut s.server_main, &mut s.server_backup);
        if s.server_main.is_empty() {
            std::mem::swap(&mut s.server_main, &mut s.server_backup);
        }
        s.prefer_remote = false;
        dirty = true;
    }
    if s.device_id.is_empty() {
        s.device_id = uuid::Uuid::new_v4().to_string();
        dirty = true;
    }
    if dirty {
        save(&s);
    }
    s
}

pub fn save(s: &Saved) {
    let Some(p) = path() else { return };
    if let Ok(text) = serde_json::to_string_pretty(s) {
        let _ = write_atomic(&p, &text, true);
    }
}

pub fn clear_token() {
    let mut s = load();
    s.token.clear();
    save(&s);
}

// ---------------------------------------------------------------------------
// Pistes préférées : par série (ou par film), langue audio et langue de sous-titres.
// Sous-titres : "" = choix du fichier, "off" = désactivés, sinon un code de langue.
// ---------------------------------------------------------------------------
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(default)]
pub struct TrackPref {
    pub audio: String,
    pub sub: String,
}

fn prefs_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("tracks.json"))
}

fn load_prefs() -> std::collections::HashMap<String, TrackPref> {
    prefs_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
}

pub fn track_pref(key: &str) -> TrackPref {
    load_prefs().get(key).cloned().unwrap_or_default()
}

/// Modifie la préférence d'une série / d'un film (audio et/ou sous-titres).
pub fn set_track_pref(key: &str, audio: Option<&str>, sub: Option<&str>) {
    if key.is_empty() {
        return;
    }
    let mut all = load_prefs();
    let e = all.entry(key.to_string()).or_default();
    if let Some(a) = audio {
        e.audio = a.to_string();
    }
    if let Some(s) = sub {
        e.sub = s.to_string();
    }
    let Some(p) = prefs_path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(t) = serde_json::to_string_pretty(&all) {
        let _ = write_atomic(&p, &t, false);
    }
}

/// Préférences de lecture du compte Jellyfin (audio, sous-titres), utilisées quand rien n'est
/// choisi pour la série : (langue audio, langue des sous-titres, mode des sous-titres).
static USER_DEFAULTS: std::sync::Mutex<(String, String, String)> = std::sync::Mutex::new((String::new(), String::new(), String::new()));

pub fn set_user_defaults(audio: &str, sub: &str, mode: &str) {
    *USER_DEFAULTS.lock().unwrap() = (audio.to_string(), sub.to_string(), mode.to_string());
}

pub fn user_defaults() -> (String, String, String) {
    USER_DEFAULTS.lock().unwrap().clone()
}

// ---------------------------------------------------------------------------
// Comptes enregistrés sur cet appareil (jeton d'accès, jamais le mot de passe) : on change de
// compte sans le ressaisir.
// ---------------------------------------------------------------------------
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(default)]
pub struct Account {
    pub server_id: String,
    pub user_id: String,
    pub user_name: String,
    #[serde(with = "secret")]
    pub token: String,
    /// Mot de passe jugé trop simple à la dernière connexion (voir password.rs) : l'interface demande
    /// de le changer. Seul ce verdict est gardé, jamais le mot de passe.
    pub weak_pw: bool,
    /// Code demandé pour ouvrir ce compte depuis « Qui regarde ? » (facultatif, voir lock.rs) :
    /// vide (aucun), « pin:… » (4 chiffres) ou « keys:… » (6 flèches), haché, jamais en clair.
    pub lock: String,
    /// Essais ratés de suite, et attente imposée jusqu'à (secondes depuis 1970).
    pub lock_fails: u32,
    pub lock_until: u64,
}

fn accounts_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("accounts.json"))
}

pub fn accounts() -> Vec<Account> {
    accounts_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
}

fn save_accounts(list: &[Account]) {
    let Some(p) = accounts_path() else { return };
    if let Ok(t) = serde_json::to_string_pretty(list) {
        let _ = write_atomic(&p, &t, true);
    }
}

/// Comptes enregistrés au plus sur un appareil : au-delà, le moins récemment utilisé est retiré
/// (un jeton reste valable sur le serveur tant qu'il n'est pas révoqué : on n'en garde pas trop).
pub const MAX_ACCOUNTS: usize = 64;

/// Ajoute ou met à jour un compte (le plus récent en premier).
pub fn save_account(a: Account) {
    let mut list = accounts();
    // Ancien jeton du même compte (nouvelle connexion) : retiré du trousseau.
    #[cfg(all(unix, not(target_os = "android")))]
    for old in list.iter().filter(|x| x.user_id == a.user_id && x.token != a.token) {
        secret::keyring::forget(&old.token);
    }
    // Le code du compte (et ses essais ratés) survit à une reconnexion : il ne se change que par
    // `set_lock` (Paramètres, ou « Code oublié » suivi du mot de passe).
    let mut a = a;
    if let Some(old) = list.iter().find(|x| x.user_id == a.user_id) {
        a.lock = old.lock.clone();
        a.lock_fails = old.lock_fails;
        a.lock_until = old.lock_until;
    }
    list.retain(|x| x.user_id != a.user_id);
    list.insert(0, a);
    list.truncate(MAX_ACCOUNTS);
    save_accounts(&list);
}

/// Code d'un compte enregistré (`lock` haché, vide : aucun) et essais ratés remis à zéro.
pub fn set_lock(user_id: &str, lock: &str) {
    update_account(user_id, |a| {
        a.lock = lock.to_string();
        a.lock_fails = 0;
        a.lock_until = 0;
    });
}

/// Modifie un compte enregistré (rien s'il n'existe pas).
pub fn update_account(user_id: &str, f: impl FnOnce(&mut Account)) {
    let mut list = accounts();
    if let Some(a) = list.iter_mut().find(|a| a.user_id == user_id) {
        f(a);
        save_accounts(&list);
    }
}

/// Verdict « mot de passe trop simple » d'un compte enregistré.
pub fn weak_pw(user_id: &str) -> bool {
    accounts().iter().any(|a| a.user_id == user_id && a.weak_pw)
}

pub fn set_weak_pw(user_id: &str, weak: bool) {
    let mut list = accounts();
    let mut changed = false;
    for a in list.iter_mut().filter(|a| a.user_id == user_id && a.weak_pw != weak) {
        a.weak_pw = weak;
        changed = true;
    }
    if changed {
        save_accounts(&list);
    }
}

pub fn forget_account(user_id: &str) {
    let mut list = accounts();
    #[cfg(all(unix, not(target_os = "android")))]
    for a in list.iter().filter(|x| x.user_id == user_id) {
        secret::keyring::forget(&a.token);
    }
    list.retain(|x| x.user_id != user_id);
    save_accounts(&list);
    // Compte de démarrage retiré : « Qui regarde ? » au prochain lancement.
    let mut p = ui_prefs();
    if p.autostart_user == user_id {
        p.autostart_user.clear();
        p.autostart_server.clear();
        save_ui_prefs(&p);
    }
}

// ---------------------------------------------------------------------------
// « Vu » et favoris décidés sur l'appareil (hors ligne, ou sur les fiches des téléchargements) :
// gardés ici, puis renvoyés au compte à la reconnexion (l'appareil a le dernier mot).
// ---------------------------------------------------------------------------
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(default)]
pub struct Flags {
    pub played: Option<bool>,
    pub favorite: Option<bool>,
    /// À renvoyer au serveur.
    pub dirty_played: bool,
    pub dirty_favorite: bool,
}

fn flags_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("userdata.json"))
}

pub fn all_flags() -> std::collections::HashMap<String, Flags> {
    flags_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
}

fn save_flags(all: &std::collections::HashMap<String, Flags>) {
    let Some(p) = flags_path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(t) = serde_json::to_string_pretty(all) {
        let _ = write_atomic(&p, &t, false);
    }
}

pub fn flags(id: &str) -> Flags {
    all_flags().get(id).cloned().unwrap_or_default()
}

/// Change « vu » (`favorite` = false) ou « favori » d'un élément ; `dirty` : à renvoyer au serveur.
pub fn set_flag(id: &str, favorite: bool, value: bool, dirty: bool) {
    if id.is_empty() {
        return;
    }
    let mut all = all_flags();
    let f = all.entry(id.to_string()).or_default();
    if favorite {
        f.favorite = Some(value);
        f.dirty_favorite |= dirty;
    } else {
        f.played = Some(value);
        f.dirty_played |= dirty;
    }
    save_flags(&all);
}

/// Changement renvoyé : plus à envoyer.
pub fn clear_dirty(id: &str, favorite: bool) {
    let mut all = all_flags();
    if let Some(f) = all.get_mut(id) {
        if favorite {
            f.dirty_favorite = false;
        } else {
            f.dirty_played = false;
        }
        save_flags(&all);
    }
}

// ---------------------------------------------------------------------------
// Préférences de l'appareil (Paramètres > Lecture / Sous-titres / Affichage), dans `prefs.json`.
// ---------------------------------------------------------------------------
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct UiPrefs {
    /// Intro (segment Jellyfin) passée sans demander.
    pub auto_skip_intro: bool,
    /// Taille des sous-titres (mpv `sub-scale`).
    pub sub_scale: f64,
    /// Interface TV (grands éléments, plein écran) ; les options --tv / --desktop priment.
    pub tv: bool,
    /// Plein écran hors interface TV (Paramètres → Affichage, F11).
    pub fullscreen: bool,
    pub show_ratings: bool,
    /// Résumé sur l'affiche sélectionnée.
    pub card_info: bool,
    /// La page d'un média prend la couleur de son affiche.
    pub media_color: bool,
    pub marquee: bool,
    pub show_clock: bool,
    /// Langue de l'interface (« fr », « en ») ; vide : pas encore choisie (demandée au démarrage).
    pub language: String,
    /// Compte ouvert au démarrage (comme sur une console de jeu) : identifiant de l'utilisateur et
    /// du serveur ; vide : écran « Qui regarde ? ».
    pub autostart_user: String,
    pub autostart_server: String,
    /// Volume de Turtlefin (0 à 100, celui de mpv : ni Windows ni le système de son).
    pub volume: u32,
    /// Débit maximal de lecture en kbit/s (0 : fichier original). Au-delà, le serveur convertit la
    /// vidéo (Paramètres → Lecture, bouton Qualité du lecteur).
    pub max_bitrate: u32,
    /// Visite guidée déjà proposée (premier lancement).
    pub tutorial_offered: bool,
    /// Visite guidée à lancer à la prochaine arrivée sur l'accueil (acceptée, `--tutorial`).
    pub tutorial_pending: bool,
    /// Animations activées, par partie de l'interface (Paramètres → Animations).
    pub anim: AnimFlags,
    /// Presets d'animations créés par l'utilisateur, et preset en cours (nom ; vide : réglages à la main).
    pub anim_presets: Vec<AnimPreset>,
    pub anim_preset: String,
    /// Thème : clé intégrée (« turtlefin », « aero »…) ou « custom:<nom> » ; thèmes importés.
    pub theme: String,
    pub themes: Vec<crate::theme::ThemeDef>,
    /// Écrans de diffusion déjà vus (Paramètres → Diffusion) : nom d'origine et nom choisi.
    pub cast_screens: Vec<CastScreen>,
    /// Adresse du serveur sur le réseau local (identifiant du serveur -> adresse), notée à chaque
    /// connexion par une adresse locale : donnée aux écrans de diffusion même quand l'appareil passe
    /// par Tailscale ou Internet (l'écran, lui, ne joint que le réseau local).
    pub lan_servers: std::collections::BTreeMap<String, String>,
    /// Serveurs déjà trouvés (identifiant -> nom et adresses vues) : réessayés à chaque recherche,
    /// même depuis un autre réseau (voir discovery::discover).
    pub known_servers: std::collections::BTreeMap<String, KnownServer>,
    /// Relais de la diffusion par cet appareil (Paramètres → Diffusion) : « » automatique (quand
    /// l'écran ne joint pas le serveur), « always » toujours, « never » jamais.
    pub relay_mode: String,
    /// Bandes-annonces YouTube (par yt-dlp) refusées : seulement celles du serveur.
    pub no_yt_trailers: bool,
    /// Décodage matériel coupé (Paramètres → Lecture) : image décodée par le processeur, pour les
    /// cartes graphiques qui affichent des parasites.
    pub no_hwdec: bool,
    /// Dossier des téléchargements choisi (Paramètres → Stockage) ; vide : dossier de Turtlefin.
    pub download_dir: String,
    /// Place maximale des téléchargements, en Go (0 : sans limite, autant que les fichiers le
    /// demandent tant que le disque a de la place).
    pub download_limit_gb: u32,
}

/// Serveur déjà trouvé par la recherche : son nom et les adresses où il a répondu (6 au plus).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct KnownServer {
    pub name: String,
    pub addrs: Vec<String>,
    /// Nom choisi dans Turtlefin (Paramètres -> Réseau) ; vide : celui du serveur.
    pub custom: String,
}

/// Nom à afficher pour un serveur : celui choisi dans Turtlefin, sinon `original`.
pub fn server_label(id: &str, original: &str) -> String {
    ui_prefs().known_servers.get(id).map(|k| k.custom.clone()).filter(|c| !c.is_empty()).unwrap_or_else(|| original.to_string())
}

/// Écran de diffusion connu : appareil Google Cast.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct CastScreen {
    /// « gcast:<id> » (les anciennes télés DLNA « dlna:<UDN> » sont oubliées au chargement).
    pub key: String,
    /// Nom donné par l'appareil.
    pub original: String,
    /// Nom choisi dans Turtlefin (vide : le nom d'origine).
    pub name: String,
}

impl CastScreen {
    pub fn shown(&self) -> &str {
        if self.name.is_empty() { &self.original } else { &self.name }
    }
}

/// Animations activées (voir `Motion` dans theme.slint).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct AnimFlags {
    pub boot: bool,
    pub pages: bool,
    pub menu: bool,
    pub select: bool,
    pub scroll: bool,
    pub panels: bool,
    pub detail: bool,
    pub player: bool,
    pub search: bool,
    pub login: bool,
    pub language: bool,
    pub tour: bool,
    /// Bibliothèque ouverte : ses affiches distribuées comme des cartes (la première fois de la session).
    pub deal: bool,
    /// Décor du thème qui bouge doucement (fond redessiné en continu : plus de calcul).
    pub scenery: bool,
}

impl AnimFlags {
    pub const fn all(on: bool) -> AnimFlags {
        AnimFlags {
            boot: on,
            pages: on,
            menu: on,
            select: on,
            scroll: on,
            panels: on,
            detail: on,
            player: on,
            search: on,
            login: on,
            language: on,
            tour: on,
            deal: on,
            scenery: on,
        }
    }
    /// Clés (Paramètres, fichiers de preset), dans l'ordre d'affichage.
    pub const KEYS: [&'static str; 14] = ["boot", "pages", "deal", "menu", "select", "scroll", "panels", "detail", "player", "search", "login", "language", "tour", "scenery"];
    pub fn get(&self, key: &str) -> bool {
        match key {
            "boot" => self.boot,
            "pages" => self.pages,
            "menu" => self.menu,
            "select" => self.select,
            "scroll" => self.scroll,
            "panels" => self.panels,
            "detail" => self.detail,
            "player" => self.player,
            "search" => self.search,
            "login" => self.login,
            "language" => self.language,
            "tour" => self.tour,
            "deal" => self.deal,
            "scenery" => self.scenery,
            _ => false,
        }
    }
    pub fn set(&mut self, key: &str, on: bool) {
        match key {
            "boot" => self.boot = on,
            "pages" => self.pages = on,
            "menu" => self.menu = on,
            "select" => self.select = on,
            "scroll" => self.scroll = on,
            "panels" => self.panels = on,
            "detail" => self.detail = on,
            "player" => self.player = on,
            "search" => self.search = on,
            "login" => self.login = on,
            "language" => self.language = on,
            "tour" => self.tour = on,
            "deal" => self.deal = on,
            "scenery" => self.scenery = on,
            _ => {}
        }
    }
    pub fn count(&self) -> usize {
        Self::KEYS.iter().filter(|k| self.get(k)).count()
    }
}

impl Default for AnimFlags {
    fn default() -> Self {
        // Raspberry Pi (Linux ARM 64 bits) : décor fixe par défaut (fond redessiné en continu sinon).
        AnimFlags { scenery: !cfg!(all(target_os = "linux", target_arch = "aarch64")), ..AnimFlags::all(true) }
    }
}

/// Preset d'animations (aussi le contenu d'un fichier exporté, `<nom>.json`).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnimPreset {
    pub name: String,
    pub anim: AnimFlags,
}

impl Default for UiPrefs {
    fn default() -> Self {
        UiPrefs {
            auto_skip_intro: false,
            sub_scale: 1.0,
            tv: false,
            fullscreen: false,
            show_ratings: true,
            card_info: true,
            media_color: true,
            marquee: true,
            show_clock: true,
            language: String::new(),
            autostart_user: String::new(),
            autostart_server: String::new(),
            volume: 100,
            max_bitrate: 0,
            tutorial_offered: false,
            tutorial_pending: false,
            anim: AnimFlags::default(),
            anim_presets: Vec::new(),
            anim_preset: String::new(),
            theme: "turtlefin".into(),
            themes: Vec::new(),
            cast_screens: Vec::new(),
            lan_servers: std::collections::BTreeMap::new(),
            known_servers: std::collections::BTreeMap::new(),
            relay_mode: String::new(),
            no_yt_trailers: false,
            no_hwdec: false,
            download_dir: String::new(),
            download_limit_gb: 0,
        }
    }
}

fn ui_prefs_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("prefs.json"))
}

pub fn ui_prefs() -> UiPrefs {
    let mut p: UiPrefs = ui_prefs_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default();
    // Diffusion DLNA retirée (1.9.6) : ses télés ne sont plus proposées au renommage.
    p.cast_screens.retain(|s| !s.key.starts_with("dlna:"));
    p
}

pub fn save_ui_prefs(p: &UiPrefs) {
    let Some(path) = ui_prefs_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(t) = serde_json::to_string_pretty(p) {
        let _ = write_atomic(&path, &t, false);
    }
}
