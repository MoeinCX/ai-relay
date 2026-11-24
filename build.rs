// The UI font (Satrap) is not part of this public repository. Release builds get it from
// GitHub secrets (see .github/workflows/release.yml); without it the UI uses the system font.
fn main() {
    println!("cargo:rustc-check-cfg=cfg(has_fonts)");
    println!("cargo:rerun-if-changed=fonts");
    let all = ["Regular", "Bold", "ExtraBold"]
        .iter()
        .all(|w| std::path::Path::new(&format!("fonts/Satrap-{w}.woff2")).exists());
    if all {
        println!("cargo:rustc-cfg=has_fonts");
    }
}
