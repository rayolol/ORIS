fn main() {
    let manifest_path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let manifest_path = manifest_path.join("Cargo.toml");
    let target_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let ws = sema::analysis(sema::Config {
        manifest_path,
        target_dir,
    })
    .expect("sema analysis failed");

    ws.emit_rerun_directives(); // tells cargo to re-run this build.rs when source files change

    // ... generate code from ws, see below
}
