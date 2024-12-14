fn main() {
    // Link proxy export defs
    let absolute_path = std::fs::canonicalize("src/proxy/exports.def").unwrap();
    println!("cargo:rustc-cdylib-link-arg=/DEF:{}", absolute_path.display());

    // Generate and link version information
    let res = tauri_winres::WindowsResource::new();
    res.compile().unwrap();
}