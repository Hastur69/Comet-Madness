// Embeds the application icon into the Windows executable.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon/icon.rc");
    println!("cargo:rerun-if-changed=assets/icon/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let _ = embed_resource::compile("assets/icon/icon.rc", embed_resource::NONE);
    }
}
