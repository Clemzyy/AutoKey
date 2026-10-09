fn main() {
    // icône de l'exécutable (ressource Windows) : sans objet sur les autres systèmes
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("assets/autokey.rc", embed_resource::NONE).manifest_optional().unwrap();
    }
}
