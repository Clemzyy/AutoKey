fn main() {
    // icône de l'exécutable (resource Windows)
    embed_resource::compile("assets/autokey.rc", embed_resource::NONE).manifest_optional().unwrap();
}
