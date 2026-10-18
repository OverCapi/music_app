fn main() {
    // Windows test executables (not the app, which tauri-build handles) need the
    // common-controls v6 manifest, else they fail to start with STATUS_ENTRYPOINT_NOT_FOUND.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows-test.manifest");
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifest.display()
        );
        println!("cargo:rerun-if-changed=windows-test.manifest");
    }
    tauri_build::build()
}
