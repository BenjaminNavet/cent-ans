// Embeds icon.ico in the Windows executable (ADR 0153). Does nothing for other targets, so the
// host build and the unit tests on macOS/Linux stay dependency-free in practice.
fn main() {
    println!("cargo:rerun-if-changed=icon.rc");
    println!("cargo:rerun-if-changed=icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("icon.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
