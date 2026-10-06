//! Mise à jour (Paramètres > À propos).
//!
//! Turtlefin peut être installé de plusieurs façons ; chacune a sa mise à jour :
//! - compilé depuis le dépôt git (`cargo build --release`) : comparaison du commit avec `main`
//!   sur GitHub, puis `git pull` + compilation dans ce même dossier ;
//! - versions publiées (GitHub Releases, compilées par l'intégration continue) : comparaison du
//!   numéro de version avec la dernière publication, puis téléchargement du bon fichier :
//!   - Windows installé : l'installeur, relancé en silence dans le même dossier ;
//!   - Windows portable : l'archive, dont les fichiers remplacent ceux du dossier ;
//!   - Linux AppImage : le nouveau fichier AppImage remplace l'ancien ;
//!   - Linux paquet .deb : le paquet, installé avec `pkexec apt-get` (mot de passe demandé).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Result};

const REPO: &str = "Xelopteryx/Turtlefin";

/// Façon dont Turtlefin est installé.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Source,
    WinInstalled,
    WinPortable,
    AppImage,
    Deb,
    Unknown,
}

/// Commit compilé (vide si inconnu : archive sans git).
pub fn commit() -> &'static str {
    env!("TURTLEFIN_COMMIT")
}

/// Dépôt d'où l'application a été compilée (versions compilées sur l'appareil).
fn source_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Version publiée (compilée par l'intégration continue) : `TURTLEFIN_DIST` est posé à la compilation.
fn is_release_build() -> bool {
    option_env!("TURTLEFIN_DIST").is_some()
}

pub fn kind() -> Kind {
    if !is_release_build() && source_dir().join(".git").exists() {
        return Kind::Source;
    }
    let dir = crate::paths::exe_dir().unwrap_or_default();
    if cfg!(windows) {
        if dir.join("portable").exists() {
            return Kind::WinPortable;
        }
        if dir.join("unins000.exe").exists() {
            return Kind::WinInstalled;
        }
        return Kind::Unknown;
    }
    if std::env::var_os("APPIMAGE").is_some() {
        return Kind::AppImage;
    }
    if dir.starts_with("/usr") {
        return Kind::Deb;
    }
    Kind::Unknown
}

/// Libellé pour la page À propos.
pub fn kind_label() -> &'static str {
    match kind() {
        Kind::Source => "compilé depuis les sources",
        Kind::WinInstalled => "Windows, installé",
        Kind::WinPortable => "Windows, portable",
        Kind::AppImage => "AppImage",
        Kind::Deb => "paquet .deb",
        Kind::Unknown => "installation inconnue",
    }
}

fn http() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!("Turtlefin/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(20))
        .build()?)
}

async fn github(path: &str) -> Result<serde_json::Value> {
    let resp = http()?
        .get(format!("https://api.github.com/repos/{REPO}/{path}"))
        .send()
        .await
        .map_err(|_| anyhow!("GitHub injoignable (connexion Internet ?)"))?;
    match resp.status().as_u16() {
        404 => Err(anyhow!("introuvable sur GitHub")),
        403 | 429 => Err(anyhow!("GitHub limite les vérifications : réessaie dans une heure")),
        s if !(200..300).contains(&s) => Err(anyhow!("GitHub a répondu {s}")),
        _ => Ok(resp.json().await?),
    }
}

/// « 0.9.1 » > « 0.9.0 » ? (numéros séparés par des points, suffixe ignoré).
fn newer(remote: &str, local: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v').split(['.', '-', '+']).take(3).map(|p| p.parse().unwrap_or(0)).collect()
    };
    parse(remote) > parse(local)
}

/// Mise à jour disponible : (libellé, notes). `None` : déjà à jour.
pub async fn check() -> Result<Option<(String, Vec<String>)>> {
    match kind() {
        Kind::Source => check_source().await,
        Kind::Unknown => Err(anyhow!("mise à jour automatique impossible pour cette installation : télécharge la dernière version sur GitHub")),
        _ => check_release().await,
    }
}

async fn check_source() -> Result<Option<(String, Vec<String>)>> {
    let local = commit();
    if local.is_empty() {
        return Err(anyhow!("version installée inconnue (compilée sans git)"));
    }
    let v = github(&format!("compare/{local}...main")).await.map_err(|e| {
        if e.to_string().starts_with("introuvable") {
            anyhow!("cette version contient des modifications pas encore publiées sur GitHub ; rien à installer")
        } else {
            e
        }
    })?;
    let ahead = v["ahead_by"].as_u64().unwrap_or(0);
    if ahead == 0 {
        return Ok(None);
    }
    let titles = v["commits"]
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .take(5)
        .filter_map(|c| c["commit"]["message"].as_str().map(|m| m.lines().next().unwrap_or("").to_string()))
        .collect();
    Ok(Some((format!("{ahead} nouveauté(s)"), titles)))
}

async fn check_release() -> Result<Option<(String, Vec<String>)>> {
    let v = github("releases/latest").await?;
    let tag = v["tag_name"].as_str().unwrap_or("").to_string();
    if !newer(&tag, env!("CARGO_PKG_VERSION")) {
        return Ok(None);
    }
    let version = tag.trim_start_matches('v').to_string();
    // Notes : premières lignes non vides de la publication.
    let notes = v["body"]
        .as_str()
        .unwrap_or("")
        .lines()
        .map(|l| l.trim().trim_start_matches(['-', '*', '#', ' ']).trim().to_string())
        .filter(|l| !l.is_empty())
        .take(4)
        .collect();
    Ok(Some((format!("Version {version}"), notes)))
}

/// Nom du fichier publié qui convient à cette installation.
fn asset_name(version: &str) -> Result<String> {
    let win = if cfg!(target_arch = "x86") { "x86" } else { "x64" };
    let linux = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" };
    let deb = if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64" };
    Ok(match kind() {
        Kind::WinInstalled => format!("Turtlefin-{version}-windows-{win}-setup.exe"),
        Kind::WinPortable => format!("Turtlefin-{version}-windows-{win}-portable.zip"),
        Kind::AppImage => format!("Turtlefin-{version}-linux-{linux}.AppImage"),
        Kind::Deb => format!("turtlefin_{version}_{deb}.deb"),
        _ => return Err(anyhow!("pas de fichier publié pour cette installation")),
    })
}

/// Fichier téléchargé, prêt à être appliqué au redémarrage (installeur Windows, paquet .deb).
static PENDING: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

fn run(dir: &Path, prog: &str, args: &[&str]) -> Result<()> {
    let out = Command::new(prog).args(args).current_dir(dir).output().map_err(|e| anyhow!("{prog} introuvable ({e})"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
        return Err(anyhow!("{prog} {} a échoué : {last}", args.first().unwrap_or(&"")));
    }
    Ok(())
}

/// Récupère la nouvelle version (plusieurs minutes pour une compilation sur un Pi). `step` reçoit
/// l'étape en cours. À appeler hors du thread de l'interface.
pub fn install(step: impl Fn(&str)) -> Result<()> {
    match kind() {
        Kind::Source => install_source(step),
        Kind::Unknown => Err(anyhow!("mise à jour automatique impossible pour cette installation")),
        k => tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(install_release(k, step)),
    }
}

fn install_source(step: impl Fn(&str)) -> Result<()> {
    let dir = source_dir();
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

/// Télécharge le fichier publié `name` de la dernière version, avec l'avancement.
async fn download(name: &str, dest: &Path, step: &impl Fn(&str)) -> Result<()> {
    let v = github("releases/latest").await?;
    let url = v["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| a["name"].as_str() == Some(name))
        .and_then(|a| a["browser_download_url"].as_str())
        .ok_or_else(|| anyhow!("fichier {name} absent de la publication"))?
        .to_string();
    let client = reqwest::Client::builder()
        .user_agent(concat!("Turtlefin/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()?;
    let mut resp = client.get(&url).send().await?.error_for_status()?;
    let total = resp.content_length().unwrap_or(0);
    let tmp = dest.with_extension("part");
    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut done = 0u64;
    let mut last = std::time::Instant::now();
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = tokio::time::timeout(std::time::Duration::from_secs(60), resp.chunk())
        .await
        .map_err(|_| anyhow!("téléchargement bloqué (plus de données depuis 60 s)"))??
    {
        file.write_all(&chunk).await?;
        done += chunk.len() as u64;
        if total > 0 && last.elapsed().as_millis() > 400 {
            last = std::time::Instant::now();
            step(&format!("Téléchargement… {} %", done * 100 / total));
        }
    }
    file.flush().await?;
    drop(file);
    tokio::fs::rename(&tmp, dest).await?;
    Ok(())
}

async fn install_release(k: Kind, step: impl Fn(&str)) -> Result<()> {
    let v = github("releases/latest").await?;
    let version = v["tag_name"].as_str().unwrap_or("").trim_start_matches('v').to_string();
    let name = asset_name(&version)?;
    let tmp_dir = std::env::temp_dir().join("turtlefin-update");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir)?;
    let file = tmp_dir.join(&name);
    step("Téléchargement…");
    download(&name, &file, &step).await?;
    match k {
        // Installeur / paquet : appliqués au redémarrage (l'appli doit être fermée).
        Kind::WinInstalled | Kind::Deb => {
            *PENDING.lock().unwrap() = Some(file);
        }
        Kind::WinPortable => {
            step("Remplacement des fichiers…");
            let out = tmp_dir.join("extrait");
            std::fs::create_dir_all(&out)?;
            // tar (fourni avec Windows 10 et suivants) sait lire les archives zip.
            run(&tmp_dir, "tar", &["-xf", &file.to_string_lossy(), "-C", &out.to_string_lossy()])?;
            let dest = crate::paths::exe_dir().ok_or_else(|| anyhow!("dossier de l'application introuvable"))?;
            // L'archive contient un dossier « Turtlefin » (ou directement les fichiers).
            let src = if out.join("Turtlefin").is_dir() { out.join("Turtlefin") } else { out.clone() };
            replace_files(&src, &dest)?;
        }
        Kind::AppImage => {
            step("Remplacement de l'AppImage…");
            let target = PathBuf::from(std::env::var_os("APPIMAGE").ok_or_else(|| anyhow!("AppImage introuvable"))?);
            let new = target.with_extension("AppImage.new");
            std::fs::copy(&file, &new)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&new, std::fs::Permissions::from_mode(0o755))?;
            }
            // Sous Linux, un fichier en cours d'exécution peut être remplacé (renommage atomique).
            std::fs::rename(&new, &target)?;
        }
        _ => {}
    }
    Ok(())
}

/// Copie récursive de `src` dans `dest` ; un fichier verrouillé (exécutable ou DLL en cours
/// d'utilisation) est d'abord renommé en `.old`.
fn replace_files(src: &Path, dest: &Path) -> Result<()> {
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let to = dest.join(e.file_name());
        if e.file_type()?.is_dir() {
            std::fs::create_dir_all(&to)?;
            replace_files(&e.path(), &to)?;
            continue;
        }
        if std::fs::copy(e.path(), &to).is_err() {
            let old = to.with_extension(format!("{}.old", to.extension().and_then(|x| x.to_str()).unwrap_or("")));
            let _ = std::fs::remove_file(&old);
            std::fs::rename(&to, &old)?;
            std::fs::copy(e.path(), &to)?;
        }
    }
    Ok(())
}

/// Relance l'application (nouvelle version) avec les mêmes options, en appliquant d'abord le
/// fichier en attente (installeur Windows, paquet .deb).
pub fn restart() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pending = PENDING.lock().unwrap().take();
    let ok = match (kind(), pending) {
        // L'installeur remplace les fichiers une fois l'appli fermée, puis la relance.
        (Kind::WinInstalled, Some(setup)) => {
            let dir = crate::paths::exe_dir().unwrap_or_default();
            Command::new(setup)
                .args(["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/CLOSEAPPLICATIONS"])
                .arg(format!("/DIR={}", dir.display()))
                .spawn()
                .is_ok()
        }
        // Paquet : installé avec les droits administrateur (pkexec demande le mot de passe), puis relance.
        (Kind::Deb, Some(deb)) => {
            let script = format!(
                "pkexec apt-get install -y --allow-downgrades '{}' && exec turtlefin {}",
                deb.display(),
                args.iter().map(|a| format!("'{}'", a.replace('\'', ""))).collect::<Vec<_>>().join(" ")
            );
            Command::new("sh").args(["-c", &script]).spawn().is_ok()
        }
        (Kind::AppImage, _) => match std::env::var_os("APPIMAGE") {
            Some(p) => Command::new(p).args(&args).spawn().is_ok(),
            None => false,
        },
        (Kind::Source, _) => {
            let exe = source_dir().join("target").join("release").join(if cfg!(windows) { "turtlefin.exe" } else { "turtlefin" });
            Command::new(exe).args(&args).spawn().is_ok()
        }
        _ => std::env::current_exe().ok().is_some_and(|exe| Command::new(exe).args(&args).spawn().is_ok()),
    };
    if ok {
        // Fermeture normale (la watch party est quittée à la sortie de main).
        let _ = slint::quit_event_loop();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions() {
        assert!(super::newer("v0.9.1", "0.9.0"));
        assert!(super::newer("1.0.0", "0.9.9"));
        assert!(!super::newer("v0.9.0", "0.9.0"));
        assert!(!super::newer("0.8.5", "0.9.0"));
    }
}
