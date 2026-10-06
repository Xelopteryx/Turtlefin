//! Mise à jour depuis GitHub (Paramètres > À propos).
//!
//! Turtlefin est compilé sur l'appareil à partir de son dépôt git (`~/turtlefin` sur le Pi) :
//! la version installée est le commit compilé (`TURTLEFIN_COMMIT`, posé par build.rs). La mise à
//! jour compare ce commit à la branche `main` sur GitHub, puis fait `git pull --ff-only` et
//! `cargo build --release` dans ce même dépôt, et relance l'application.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Result};

const REPO: &str = "Xelopteryx/Turtlefin";

/// Commit compilé (court), vide si inconnu (archive sans git).
pub fn commit() -> &'static str {
    env!("TURTLEFIN_COMMIT")
}

/// Dépôt d'où l'application a été compilée.
fn source_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Nouveautés disponibles sur GitHub : (nombre de commits, titres des plus récents).
pub async fn check() -> Result<(u32, Vec<String>)> {
    let local = commit();
    if local.is_empty() {
        return Err(anyhow!("version installée inconnue (compilée sans git)"));
    }
    let http = reqwest::Client::builder()
        .user_agent(concat!("Turtlefin/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let url = format!("https://api.github.com/repos/{REPO}/compare/{local}...main");
    let resp = http.get(url).send().await.map_err(|_| anyhow!("GitHub injoignable (connexion Internet ?)"))?;
    match resp.status().as_u16() {
        // Commit inconnu de GitHub : cette version contient des modifications pas encore publiées.
        404 => return Err(anyhow!("cette version contient des modifications pas encore publiées sur GitHub ; rien à installer")),
        403 | 429 => return Err(anyhow!("GitHub limite les vérifications : réessaie dans une heure")),
        s if !(200..300).contains(&s) => return Err(anyhow!("GitHub a répondu {s}")),
        _ => {}
    }
    let v: serde_json::Value = resp.json().await?;
    let ahead = v["ahead_by"].as_u64().unwrap_or(0) as u32;
    let titles = v["commits"]
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .take(5)
        .filter_map(|c| c["commit"]["message"].as_str().map(|m| m.lines().next().unwrap_or("").to_string()))
        .collect();
    Ok((ahead, titles))
}

fn run(dir: &PathBuf, prog: &str, args: &[&str]) -> Result<()> {
    let out = Command::new(prog).args(args).current_dir(dir).output().map_err(|e| anyhow!("{prog} introuvable ({e})"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
        return Err(anyhow!("{prog} {} a échoué : {last}", args.first().unwrap_or(&"")));
    }
    Ok(())
}

/// Récupère et compile la nouvelle version (long : plusieurs minutes sur un Pi). `step` reçoit
/// l'étape en cours.
pub fn install(step: impl Fn(&str)) -> Result<()> {
    let dir = source_dir();
    if !dir.join(".git").exists() {
        return Err(anyhow!("dépôt introuvable ({})", dir.display()));
    }
    step("Téléchargement des nouveautés…");
    run(&dir, "git", &["pull", "--ff-only"])?;
    // Windows : l'exécutable en cours ne peut pas être remplacé, mais il peut être renommé.
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        let old = exe.with_extension("old.exe");
        let _ = std::fs::remove_file(&old);
        let _ = std::fs::rename(&exe, &old);
    }
    step("Compilation (quelques minutes, l'appli reste utilisable)…");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| {
        // Sur le Pi, cargo est dans ~/.cargo/bin (pas toujours dans le PATH d'un lancement graphique).
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
        let p = PathBuf::from(home).join(".cargo").join("bin").join(if cfg!(windows) { "cargo.exe" } else { "cargo" });
        if p.exists() { p.to_string_lossy().into_owned() } else { "cargo".into() }
    });
    run(&dir, &cargo, &["build", "--release"])?;
    Ok(())
}

/// Relance l'application (nouvelle version) avec les mêmes options.
pub fn restart() {
    let exe = source_dir().join("target").join("release").join(if cfg!(windows) { "turtlefin.exe" } else { "turtlefin" });
    let args: Vec<String> = std::env::args().skip(1).collect();
    if Command::new(exe).args(args).spawn().is_ok() {
        // Fermeture normale (la watch party est quittée à la sortie de main).
        let _ = slint::quit_event_loop();
    }
}
