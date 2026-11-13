fn main() {
    // Commit compilé (Paramètres > À propos, mise à jour) : vide hors d'un dépôt git.
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    println!("cargo:rustc-env=TURTLEFIN_COMMIT={commit}");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    // Style imposé : "fluent-dark" (rendu 100 % Slint). Sans ça, Slint peut
    // choisir le style "native" qui dépend de Qt si Qt est installé.
    // Traductions : faites à l'exécution par src/i18n.rs (global Tr de ui/theme.slint), pas par Slint.
    let config = slint_build::CompilerConfiguration::new().with_style("fluent-dark".into());
    slint_build::compile_with_config("ui/app.slint", config).expect("échec de la compilation Slint");

    // Windows : icône de l'exécutable (Explorateur, barre des tâches, raccourcis).
    println!("cargo:rerun-if-changed=packaging/icons/turtlefin.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/icons/turtlefin.ico");
        res.compile().expect("échec de l'ajout de l'icône Windows");
        libmpv_next_to_exe();
    }
}

/// libmpv de Windows, fournie par le dépôt (vendor/libmpv, voir son README) : décompressée, vérifiée
/// (SHA-256) et posée à côté de l'exécutable, où Turtlefin la cherche. Rien à télécharger : après un
/// `cargo clean`, `cargo build` la remet en place. Linux : libmpv vient du système.
fn libmpv_next_to_exe() {
    use std::io::Read;
    let dir = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86_64") => "windows-x86_64",
        Ok("x86") => "windows-i686",
        _ => return,
    };
    let src = format!("vendor/libmpv/{dir}/libmpv-2.dll.gz");
    // Empreinte SHA-256 de la DLL décompressée, écrite à côté par tools/update-libmpv.ps1.
    let sum = format!("vendor/libmpv/{dir}/libmpv-2.dll.sha256");
    println!("cargo:rerun-if-changed={src}");
    println!("cargo:rerun-if-changed={sum}");
    let want = std::fs::read_to_string(&sum).unwrap_or_default().split_whitespace().next().unwrap_or("").to_ascii_lowercase();
    if want.len() != 64 {
        println!("cargo:warning=libmpv : {sum} absent ou illisible ; DLL non installée.");
        return;
    }
    let want = want.as_str();
    // OUT_DIR = <target>/[<cible>/]<profil>/build/turtlefin-<hash>/out : l'exécutable est dans <profil>.
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let Some(exe_dir) = out.ancestors().nth(3) else { return };
    let dst = exe_dir.join("libmpv-2.dll");
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let sha = |b: &[u8]| {
        use sha2::Digest;
        hex(&sha2::Sha256::digest(b))
    };
    // Déjà la bonne : rien à faire.
    if std::fs::read(&dst).is_ok_and(|b| sha(&b) == want) {
        return;
    }
    let mut raw = Vec::new();
    let read = std::fs::File::open(&src).and_then(|f| flate2::read::GzDecoder::new(f).read_to_end(&mut raw));
    if let Err(e) = read {
        println!("cargo:warning=libmpv : {src} illisible ({e}) ; la lecture vidéo ne marchera pas.");
        return;
    }
    if sha(&raw) != want {
        println!("cargo:warning=libmpv : {src} ne correspond pas à l'empreinte attendue ; DLL non installée.");
        return;
    }
    if let Err(e) = std::fs::write(&dst, &raw) {
        // Turtlefin lancé (DLL en cours d'utilisation) : l'ancienne reste en place.
        println!("cargo:warning=libmpv : impossible d'écrire {} ({e}).", dst.display());
    }
}
