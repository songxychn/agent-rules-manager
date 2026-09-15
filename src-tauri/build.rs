fn main() {
    // Tauri embeds the development Dock icon at compile time.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
