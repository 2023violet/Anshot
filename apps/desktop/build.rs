fn main() {
    // English is a bundled gettext translation of the Chinese source text
    // (lang/en_US/LC_MESSAGES/anshot-desktop.po), selected at run time.
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("lang")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/app.slint", config).expect("compile Slint UI");
    println!("cargo:rerun-if-changed=lang");
}
