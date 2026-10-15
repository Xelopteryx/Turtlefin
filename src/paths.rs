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
    // Android : dossier de cache de l'application (voir android/src/lib.rs).
    if let Some(d) = std::env::var_os("TURTLEFIN_CACHE_DIR") {
        return Some(PathBuf::from(d));
    }
    if let Some(p) = portable_root() {
        return Some(p.join("cache"));
    }
    project().map(|d| d.cache_dir().to_path_buf())
}

/// Données (téléchargements).
pub fn data_dir() -> Option<PathBuf> {
    // Essais : téléchargements dans un autre dossier (comme TURTLEFIN_CONFIG_DIR pour la config).
    if let Some(d) = std::env::var_os("TURTLEFIN_DATA_DIR") {
        return Some(PathBuf::from(d));
    }
    if let Some(p) = portable_root() {
        return Some(p);
    }
    project().map(|d| d.data_dir().to_path_buf())
}

/// Commande système lancée sans fenêtre de console (Windows : sinon une fenêtre CMD s'ouvre un
/// instant, Turtlefin n'ayant pas de console à partager).
pub fn quiet_command(prog: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(prog);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Fichier texte ajouté par l'utilisateur (langue, thème, préréglage : il peut venir de n'importe où) :
/// refusé au-delà de `max` octets plutôt que chargé en entier en mémoire.
pub fn read_text_limited(p: impl AsRef<std::path::Path>, max: u64) -> std::io::Result<String> {
    let p = p.as_ref();
    if std::fs::metadata(p)?.len() > max {
        return Err(std::io::Error::other("fichier trop gros"));
    }
    std::fs::read_to_string(p)
}
