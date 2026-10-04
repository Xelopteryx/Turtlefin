fn main() {
    // Style imposé : "fluent-dark" (rendu 100 % Slint). Sans ça, Slint peut
    // choisir le style "native" qui dépend de Qt si Qt est installé.
    let config = slint_build::CompilerConfiguration::new().with_style("fluent-dark".into());
    slint_build::compile_with_config("ui/app.slint", config).expect("échec de la compilation Slint");
}
