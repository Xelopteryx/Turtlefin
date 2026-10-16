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
//!   - Linux paquet .deb : le paquet, installé avec `apt-get` en administrateur : `sudo` s'il ne
//!     demande rien (Raspberry Pi), sinon `pkexec` sur un bureau complet, sinon mot de passe demandé
//!     par Turtlefin lui-même (session sans agent polkit : Openbox, XeLauncher), voir `deb_auth`.

use crate::i18n::{tr, trf};
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
    /// Flatpak (Flathub) : mis à jour par Flatpak, jamais par l'application.
    Flatpak,
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
    if std::env::var_os("FLATPAK_ID").is_some() {
        return Kind::Flatpak;
    }
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
        Kind::Source => tr("compilé depuis les sources"),
        Kind::WinInstalled => tr("Windows, installé"),
        Kind::WinPortable => tr("Windows, portable"),
        Kind::AppImage => "AppImage",
        Kind::Deb => tr("paquet .deb"),
        Kind::Flatpak => "Flatpak",
        Kind::Unknown => tr("installation inconnue"),
    }
}

fn http() -> Result<reqwest::Client> {
    Ok(crate::api::http_builder()
        .user_agent(concat!("Turtlefin/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(20))
        .redirect(crate::api::redirect_policy())
        .build()?)
}

/// Réponse 404 de GitHub (reconnue sans dépendre de la langue du message).
#[derive(Debug)]
struct NotFound;

impl std::fmt::Display for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", tr("introuvable sur GitHub"))
    }
}

impl std::error::Error for NotFound {}

async fn github(path: &str) -> Result<serde_json::Value> {
    let resp = http()?
        .get(format!("https://api.github.com/repos/{REPO}/{path}"))
        .send()
        .await
        .map_err(|_| anyhow!("{}", tr("GitHub injoignable (connexion Internet ?)")))?;
    match resp.status().as_u16() {
        404 => Err(NotFound.into()),
        403 | 429 => Err(anyhow!("{}", tr("GitHub limite les vérifications : réessaie dans une heure"))),
        s if !(200..300).contains(&s) => Err(anyhow!("{}", trf("GitHub a répondu {}", &[&s]))),
        _ => Ok(serde_json::from_slice(&crate::api::body_limited(resp, 8 << 20).await?)?),
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
        Kind::Flatpak => Err(anyhow!("{}", tr("mise à jour faite par Flatpak (logithèque, ou « flatpak update »)"))),
        Kind::Unknown => Err(anyhow!("{}", tr("mise à jour automatique impossible pour cette installation : télécharge la dernière version sur GitHub"))),
        _ => check_release().await,
    }
}

async fn check_source() -> Result<Option<(String, Vec<String>)>> {
    let local = commit();
    if local.is_empty() {
        return Err(anyhow!("{}", tr("version installée inconnue (compilée sans git)")));
    }
    // Version qui contient des modifications pas encore publiées (commit inconnu de GitHub) :
    // rien de plus récent à installer, ce n'est pas un échec.
    let v = match github(&format!("compare/{local}...main")).await {
        Ok(v) => v,
        Err(e) if e.downcast_ref::<NotFound>().is_some() => return Ok(None),
        Err(e) => return Err(e),
    };
    let ahead = v["ahead_by"].as_u64().unwrap_or(0);
    if ahead == 0 {
        return Ok(None);
    }
    let titles = v["commits"]
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .take(30)
        .filter_map(|c| c["commit"]["message"].as_str().map(|m| m.lines().next().unwrap_or("").to_string()))
        .collect();
    Ok(Some((trf("{} nouveauté(s)", &[&ahead]), titles)))
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
        .filter(|l| !l.is_empty() && !l.contains("://") && !l.starts_with("Full Changelog"))
        .take(30)
        .collect();
    Ok(Some((trf("Version {}", &[&version]), notes)))
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
        _ => return Err(anyhow!("{}", tr("pas de fichier publié pour cette installation"))),
    })
}

/// Fichier téléchargé, prêt à être appliqué au redémarrage (installeur Windows, paquet .deb).
static PENDING: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

fn run(dir: &Path, prog: &str, args: &[&str]) -> Result<()> {
    let out = crate::paths::quiet_command(prog).args(args).current_dir(dir).output().map_err(|e| anyhow!("{prog} introuvable ({e})"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
        return Err(anyhow!("{}", trf("{} {} a échoué : {}", &[&prog, &args.first().unwrap_or(&""), &last])));
    }
    Ok(())
}

/// Récupère la nouvelle version (plusieurs minutes pour une compilation sur une petite machine). `step` reçoit
/// l'étape en cours. À appeler hors du thread de l'interface.
pub fn install(step: impl Fn(&str)) -> Result<()> {
    match kind() {
        Kind::Source => install_source(step),
        Kind::Flatpak => Err(anyhow!("{}", tr("mise à jour faite par Flatpak (logithèque, ou « flatpak update »)"))),
        Kind::Unknown => Err(anyhow!("{}", tr("mise à jour automatique impossible pour cette installation"))),
        k => tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(install_release(k, step)),
    }
}

fn install_source(step: impl Fn(&str)) -> Result<()> {
    let dir = source_dir();
    step(tr("Téléchargement des nouveautés…"));
    // Fichiers modifiés sur place (copiés à la main, Cargo.lock réécrit par une compilation…) ou
    // ajoutés sans être suivis par git (que la nouvelle version veut créer) : git refuse alors de
    // tirer (« abandon »). Ils sont mis de côté (git stash -u, récupérables avec `git stash pop`), puis
    // la version publiée est tirée. Les fichiers ignorés (target/) ne sont pas touchés.
    let dirty = crate::paths::quiet_command("git")
        .args(["status", "--porcelain"])
        .current_dir(&dir)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    if dirty {
        run(&dir, "git", &["-c", "user.name=Turtlefin", "-c", "user.email=turtlefin@localhost", "stash", "push", "--include-untracked", "-m", "Turtlefin : modifications locales avant mise à jour"])?;
    }
    run(&dir, "git", &["pull", "--ff-only"])?;
    // Windows : l'exécutable en cours ne peut pas être remplacé, mais il peut être renommé.
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        let old = exe.with_extension("old.exe");
        let _ = std::fs::remove_file(&old);
        let _ = std::fs::rename(&exe, &old);
    }
    step(tr("Compilation (quelques minutes, l'appli reste utilisable)…"));
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| {
        // Sous Linux, cargo est dans ~/.cargo/bin (pas toujours dans le PATH d'un lancement graphique).
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
        let p = PathBuf::from(home).join(".cargo").join("bin").join(if cfg!(windows) { "cargo.exe" } else { "cargo" });
        if p.exists() { p.to_string_lossy().into_owned() } else { "cargo".into() }
    });
    run(&dir, &cargo, &["build", "--release"])?;
    Ok(())
}

/// Télécharge le fichier publié `name` de la dernière version, avec l'avancement. Son empreinte
/// SHA-256 doit être celle annoncée par GitHub (fichier abîmé ou remplacé en route : refusé).
async fn download(name: &str, dest: &Path, step: &impl Fn(&str)) -> Result<()> {
    let v = github("releases/latest").await?;
    let asset = v["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| a["name"].as_str() == Some(name))
        .ok_or_else(|| anyhow!("{}", trf("fichier {} absent de la publication", &[&name])))?;
    let url = asset["browser_download_url"].as_str().unwrap_or("").to_string();
    // Seulement depuis GitHub, en HTTPS.
    if !url.starts_with(&format!("https://github.com/{REPO}/releases/download/")) {
        return Err(anyhow!("{}", trf("fichier {} absent de la publication", &[&name])));
    }
    let digest = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(str::to_ascii_lowercase);
    let client = crate::api::http_builder()
        .user_agent(concat!("Turtlefin/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(15))
        .redirect(crate::api::redirect_policy())
        .build()?;
    let mut resp = client.get(&url).send().await?.error_for_status()?;
    let total = resp.content_length().unwrap_or(0);
    let tmp = dest.with_extension("part");
    let mut file = tokio::fs::File::create(&tmp).await?;
    use sha2::Digest;
    let mut hash = sha2::Sha256::new();
    let mut done = 0u64;
    let mut last = std::time::Instant::now();
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = tokio::time::timeout(std::time::Duration::from_secs(60), resp.chunk())
        .await
        .map_err(|_| anyhow!("{}", tr("téléchargement bloqué (plus de données depuis 60 s)")))??
    {
        file.write_all(&chunk).await?;
        hash.update(&chunk);
        done += chunk.len() as u64;
        if total > 0 && last.elapsed().as_millis() > 400 {
            last = std::time::Instant::now();
            step(&trf("Téléchargement… {} %", &[&(done * 100 / total)]));
        }
    }
    file.flush().await?;
    drop(file);
    let got: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if digest.as_deref().is_some_and(|d| d != got) {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(anyhow!("{}", tr("fichier téléchargé abîmé ou modifié (empreinte différente) : mise à jour annulée")));
    }
    tokio::fs::rename(&tmp, dest).await?;
    Ok(())
}

/// Dossier des fichiers de mise à jour : dans le cache de l'utilisateur, pas dans le /tmp partagé
/// (un autre compte de la machine pourrait y préparer un dossier piégé ou remplacer le paquet
/// avant son installation avec les droits administrateur).
fn update_dir() -> PathBuf {
    crate::paths::cache_dir().unwrap_or_else(std::env::temp_dir).join("update")
}

async fn install_release(k: Kind, step: impl Fn(&str)) -> Result<()> {
    let v = github("releases/latest").await?;
    let version = v["tag_name"].as_str().unwrap_or("").trim_start_matches('v').to_string();
    let name = asset_name(&version)?;
    // Le numéro vient de GitHub : rien d'autre que des chiffres, lettres, points et tirets.
    if version.is_empty() || !version.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
        return Err(anyhow!("{}", tr("pas de fichier publié pour cette installation")));
    }
    let tmp_dir = update_dir();
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let file = tmp_dir.join(&name);
    step(tr("Téléchargement…"));
    download(&name, &file, &step).await?;
    match k {
        // Installeur / paquet : appliqués au redémarrage (l'appli doit être fermée).
        Kind::WinInstalled | Kind::Deb => {
            *PENDING.lock().unwrap() = Some(file);
        }
        Kind::WinPortable => {
            step(tr("Remplacement des fichiers…"));
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
            step(tr("Remplacement de l'AppImage…"));
            let target = PathBuf::from(std::env::var_os("APPIMAGE").ok_or_else(|| anyhow!("{}", tr("AppImage introuvable")))?);
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

/// Options de lancement : celles de la ligne de commande, ou (relance par l'installeur après une mise
/// à jour, qui ne les connaît pas) celles notées par `restart` dans `restart-args`, lues une fois.
pub fn launch_args() -> Vec<String> {
    // Restes d'une mise à jour (fichiers verrouillés renommés en .old, voir replace_files).
    if let Some(dir) = crate::paths::exe_dir() {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            if e.path().extension().is_some_and(|x| x == "old") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        return args;
    }
    let Some(file) = crate::paths::config_dir().map(|d| d.join("restart-args")) else { return args };
    let saved = std::fs::read_to_string(&file).ok();
    let _ = std::fs::remove_file(&file);
    saved.map(|t| t.lines().filter(|l| !l.is_empty()).map(str::to_string).collect()).unwrap_or_default()
}

/// Paquet .deb : comment obtenir les droits administrateur pour l'installer.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DebAuth {
    /// `sudo` sans mot de passe (Raspberry Pi OS par défaut).
    Sudo,
    /// `pkexec` : le bureau (GNOME, KDE...) demande lui-même le mot de passe.
    Pkexec,
    /// Ni l'un ni l'autre (session Openbox / XeLauncher) : Turtlefin demande le mot de passe et le
    /// donne à `sudo` (jamais enregistré).
    Password,
}

fn quiet_ok(cmd: &mut Command) -> bool {
    use std::process::Stdio;
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

pub fn deb_auth() -> DebAuth {
    if quiet_ok(Command::new("sudo").args(["-n", "true"])) {
        return DebAuth::Sudo;
    }
    let pkexec = quiet_ok(Command::new("sh").args(["-c", "command -v pkexec"]));
    // pkexec sans agent polkit (pas de bureau complet) échoue sans rien demander.
    let desktop = std::env::var_os("XDG_CURRENT_DESKTOP").is_some_and(|d| !d.is_empty());
    let agent = quiet_ok(Command::new("pgrep").args(["-f", "polkit.*agent|lxpolkit|polkit-gnome|polkit-kde|xfce-polkit|mate-polkit|lxqt-policykit"]));
    if pkexec && (desktop || agent) {
        DebAuth::Pkexec
    } else {
        DebAuth::Password
    }
}

/// La mise à jour prête (paquet .deb) demande un mot de passe à Turtlefin.
pub fn needs_password() -> bool {
    kind() == Kind::Deb && PENDING.lock().unwrap().is_some() && deb_auth() == DebAuth::Password
}

/// Vérifie le mot de passe administrateur (sudo) sans rien lancer : sinon Turtlefin se fermerait pour
/// une installation vouée à l'échec.
pub fn check_password(password: &str) -> bool {
    use std::io::Write;
    use std::process::Stdio;
    // -k : sans ticket déjà valable ; -S : mot de passe lu sur l'entrée ; -p "" : pas d'invite ; -v : vérification.
    let Ok(mut child) = Command::new("sudo")
        .args(["-k", "-S", "-p", "", "-v"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(format!("{password}\n").as_bytes());
    }
    child.wait().is_ok_and(|s| s.success())
}

/// Relance l'application (nouvelle version) avec les mêmes options, en appliquant d'abord le
/// fichier en attente (installeur Windows, paquet .deb).
pub fn restart() {
    restart_with(None);
}

/// Comme `restart`, avec le mot de passe administrateur demandé par Turtlefin (paquet .deb, voir
/// `DebAuth::Password`) : donné à `sudo` sur son entrée, jamais écrit ni passé en argument.
pub fn restart_with_password(password: String) {
    restart_with(Some(password));
}

fn restart_with(password: Option<String>) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pending = PENDING.lock().unwrap().take();
    let ok = match (kind(), pending) {
        // L'installeur remplace les fichiers une fois l'appli fermée, puis la relance.
        (Kind::WinInstalled, Some(setup)) => {
            let dir = crate::paths::exe_dir().unwrap_or_default();
            // L'installeur relance Turtlefin sans options : elles l'attendent dans un fichier.
            if let Some(cfg) = crate::paths::config_dir() {
                let _ = std::fs::write(cfg.join("restart-args"), args.join("\n"));
            }
            // Installé pour tous (Program Files) : l'installeur doit l'être aussi (Windows demande
            // alors les droits administrateur), sinon il ne peut pas écrire dans le dossier.
            let all_users = ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"]
                .iter()
                .filter_map(std::env::var_os)
                .any(|p| !p.is_empty() && dir.starts_with(PathBuf::from(p)));
            let mut cmd = Command::new(setup);
            cmd.args(["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/CLOSEAPPLICATIONS"]);
            cmd.arg(if all_users { "/ALLUSERS" } else { "/CURRENTUSER" });
            cmd.arg(format!("/DIR={}", dir.display()));
            cmd.spawn().is_ok()
        }
        // Paquet : installé avec les droits administrateur (voir DebAuth), puis relance.
        (Kind::Deb, Some(deb)) => {
            use std::io::Write;
            // Chemin et options passés en paramètres du script (« $1 », « $@ ») : jamais interprétés.
            // Installation sans question (fichiers de configuration modifiés : gardés).
            let apt = r#"env DEBIAN_FRONTEND=noninteractive apt-get install -y --allow-downgrades -o Dpkg::Options::=--force-confold "$deb""#;
            let script = match (&password, deb_auth()) {
                (Some(_), _) => format!(r#"deb="$1"; shift; sudo -S -p "" {apt} && exec turtlefin "$@""#),
                (None, DebAuth::Sudo) => format!(r#"deb="$1"; shift; sudo -n {apt} && exec turtlefin "$@""#),
                (None, _) => format!(r#"deb="$1"; shift; pkexec {apt} && exec turtlefin "$@""#),
            };
            let mut cmd = Command::new("sh");
            cmd.args(["-c", &script, "sh"]).arg(&deb).args(&args);
            if password.is_some() {
                cmd.stdin(std::process::Stdio::piped());
            }
            match cmd.spawn() {
                Ok(mut child) => {
                    if let (Some(pw), Some(mut stdin)) = (&password, child.stdin.take()) {
                        let _ = stdin.write_all(format!("{pw}\n").as_bytes());
                    }
                    true
                }
                Err(_) => false,
            }
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
