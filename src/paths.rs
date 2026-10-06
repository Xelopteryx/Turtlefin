//! Dossiers de Turtlefin (configuration, cache, données).
//!
//! Installation normale : dossiers de l'utilisateur fournis par le système (%APPDATA%,
//! ~/.config, ~/.cache, ~/.local/share...). Version portable (fichier `portable` à côté de
//! l'exécutable, posé par l'installeur ou présent dans l'archive portable) : tout reste dans le
//! dossier `data` à côté de l'exécutable, rien n'est écrit ailleurs sur la machine.

use std::path::PathBuf;

/// Dossier de l'exécutable.
pub fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(|p| p.to_path_buf())
}

/// Version portable : `<dossier de l'exécutable>/data`.
pub fn portable_root() -> Option<PathBuf> {
    let dir = exe_dir()?;
    dir.join("portable").exists().then(|| dir.join("data"))
}

fn project() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("", "", "turtlefin")
}

/// Configuration (session, comptes, préférences). `TURTLEFIN_CONFIG_DIR` la remplace (essais).
pub fn config_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("TURTLEFIN_CONFIG_DIR") {
        return Some(PathBuf::from(d));
    }
    if let Some(p) = portable_root() {
        return Some(p.join("config"));
    }
    project().map(|d| d.config_dir().to_path_buf())
}

/// Cache (images) : peut être vidé sans rien perdre.
pub fn cache_dir() -> Option<PathBuf> {
    if let Some(p) = portable_root() {
        return Some(p.join("cache"));
    }
    project().map(|d| d.cache_dir().to_path_buf())
}

/// Données (téléchargements).
pub fn data_dir() -> Option<PathBuf> {
    if let Some(p) = portable_root() {
        return Some(p);
    }
    project().map(|d| d.data_dir().to_path_buf())
}
