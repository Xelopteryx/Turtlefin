//! Session sauvegardée (serveur + jeton) dans le dossier de config de l'OS.
//! Le mot de passe n'est JAMAIS écrit sur disque, seulement le jeton d'accès.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Saved {
    /// Adresse utilisée pour la session en cours.
    pub server: String,
    /// Adresse du serveur sur le réseau local (vide si inconnue).
    pub server_local: String,
    /// Adresse distante du même serveur : Tailscale ou autre (vide si inconnue).
    pub server_remote: String,
    /// Paramètres réseau : passer par l'adresse distante même si la locale répond.
    pub prefer_remote: bool,
    /// Affichage : avatars GIF figés sur leur première image (moins de calcul).
    pub still_gifs: bool,
    /// Affichage : pas de fond d'écran tiré du média sélectionné.
    pub no_backdrop: bool,
    /// Identifiant du serveur (pour retrouver les comptes enregistrés).
    pub server_id: String,
    pub user_name: String,
    pub user_id: String,
    pub token: String,
    pub device_id: String,
}

/// Dossier de configuration ; `TURTLEFIN_CONFIG_DIR` le remplace (essais avec une autre session).
fn config_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("TURTLEFIN_CONFIG_DIR") {
        return Some(PathBuf::from(d));
    }
    directories::ProjectDirs::from("", "", "turtlefin").map(|d| d.config_dir().to_path_buf())
}

fn path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("session.json"))
}

/// Charge la session ; crée et enregistre un identifiant d'appareil au premier lancement.
pub fn load() -> Saved {
    let mut s: Saved = path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    if s.device_id.is_empty() {
        s.device_id = uuid::Uuid::new_v4().to_string();
        save(&s);
    }
    s
}

pub fn save(s: &Saved) {
    let Some(p) = path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(&p, text);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
        }
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
        .and_then(|t| serde_json::from_str(&t).ok())
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
        let _ = std::fs::write(p, t);
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
    pub token: String,
}

fn accounts_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("accounts.json"))
}

pub fn accounts() -> Vec<Account> {
    accounts_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_accounts(list: &[Account]) {
    let Some(p) = accounts_path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(t) = serde_json::to_string_pretty(list) {
        let _ = std::fs::write(&p, t);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
        }
    }
}

/// Comptes enregistrés au plus sur un appareil : au-delà, le moins récemment utilisé est retiré
/// (un jeton reste valable sur le serveur tant qu'il n'est pas révoqué : on n'en garde pas trop).
pub const MAX_ACCOUNTS: usize = 12;

/// Ajoute ou met à jour un compte (le plus récent en premier).
pub fn save_account(a: Account) {
    let mut list = accounts();
    list.retain(|x| x.user_id != a.user_id);
    list.insert(0, a);
    list.truncate(MAX_ACCOUNTS);
    save_accounts(&list);
}

pub fn forget_account(user_id: &str) {
    let mut list = accounts();
    list.retain(|x| x.user_id != user_id);
    save_accounts(&list);
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
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_flags(all: &std::collections::HashMap<String, Flags>) {
    let Some(p) = flags_path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(t) = serde_json::to_string_pretty(all) {
        let _ = std::fs::write(p, t);
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
    pub show_ratings: bool,
    pub marquee: bool,
    pub show_clock: bool,
}

impl Default for UiPrefs {
    fn default() -> Self {
        UiPrefs { auto_skip_intro: false, sub_scale: 1.0, tv: false, show_ratings: true, marquee: true, show_clock: true }
    }
}

fn ui_prefs_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("prefs.json"))
}

pub fn ui_prefs() -> UiPrefs {
    ui_prefs_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_ui_prefs(p: &UiPrefs) {
    let Some(path) = ui_prefs_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(t) = serde_json::to_string_pretty(p) {
        let _ = std::fs::write(path, t);
    }
}
