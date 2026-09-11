fn main() {
    println!("cargo:rerun-if-changed=src/vision_ocr.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    cc::Build::new()
        .file("src/vision_ocr.m")
        .flag("-fobjc-arc")
        .flag("-fblocks")
        .flag("-fmodules")
        .compile("luma_vision_ocr");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=Vision");
}
