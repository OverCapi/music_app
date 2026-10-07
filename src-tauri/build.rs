fn main() {
    // libopus is linked dynamically; Homebrew on Apple Silicon installs it outside the default linker path.
    if std::path::Path::new("/opt/homebrew/lib").exists() {
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
    }
    tauri_build::build()
}
