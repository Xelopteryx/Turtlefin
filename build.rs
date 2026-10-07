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
    // Traductions (lang/<langue>/LC_MESSAGES/turtlefin.po) intégrées à l'exécutable, sans contexte :
    // les mêmes textes servent à Slint (@tr) et à Rust (i18n::tr).
    println!("cargo:rerun-if-changed=lang");
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent-dark".into())
        .with_bundled_translations("lang")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/app.slint", config).expect("échec de la compilation Slint");

    // Windows : icône de l'exécutable (Explorateur, barre des tâches, raccourcis).
    println!("cargo:rerun-if-changed=packaging/icons/turtlefin.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/icons/turtlefin.ico");
        res.compile().expect("échec de l'ajout de l'icône Windows");
    }
}
