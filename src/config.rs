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
    pub user_name: String,
    pub user_id: String,
    pub token: String,
    pub device_id: String,
}

fn path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "turtlefin").map(|d| d.config_dir().join("session.json"))
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
    directories::ProjectDirs::from("", "", "turtlefin").map(|d| d.config_dir().join("tracks.json"))
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
