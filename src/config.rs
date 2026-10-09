//! Session sauvegardée (serveur + jeton) dans le dossier de config de l'OS.
//! Le mot de passe n'est JAMAIS écrit sur disque, seulement le jeton d'accès.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Saved {
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
    pub token: String,
    pub device_id: String,
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
        .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
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
    /// Plein écran hors interface TV (Paramètres → Affichage, F11).
    pub fullscreen: bool,
    pub show_ratings: bool,
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
    /// Visite guidée déjà proposée (premier lancement).
    pub tutorial_offered: bool,
    /// Visite guidée à lancer à la prochaine arrivée sur l'accueil (acceptée, `--tutorial`).
    pub tutorial_pending: bool,
    /// Animations activées, par partie de l'interface (Paramètres → Animations).
    pub anim: AnimFlags,
    /// Presets d'animations créés par l'utilisateur, et preset en cours (nom ; vide : réglages à la main).
    pub anim_presets: Vec<AnimPreset>,
    pub anim_preset: String,
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
        }
    }
    /// Clés (Paramètres, fichiers de preset), dans l'ordre d'affichage.
    pub const KEYS: [&'static str; 12] = ["boot", "pages", "menu", "select", "scroll", "panels", "detail", "player", "search", "login", "language", "tour"];
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
            _ => {}
        }
    }
    pub fn count(&self) -> usize {
        Self::KEYS.iter().filter(|k| self.get(k)).count()
    }
}

impl Default for AnimFlags {
    fn default() -> Self {
        AnimFlags::all(true)
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
            marquee: true,
            show_clock: true,
            language: String::new(),
            autostart_user: String::new(),
            autostart_server: String::new(),
            volume: 100,
            tutorial_offered: false,
            tutorial_pending: false,
            anim: AnimFlags::default(),
            anim_presets: Vec::new(),
            anim_preset: String::new(),
        }
    }
}

fn ui_prefs_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("prefs.json"))
}

pub fn ui_prefs() -> UiPrefs {
    ui_prefs_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
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
