use std::path::PathBuf;

fn main() {
  // `tauri_build::build()` and `tauri::generate_context!()` require `build.frontendDist`
  // (`../yanshi-ui-dx/dist/public`) to exist on disk at compile time. Because WASM build
  // outputs are intentionally gitignored, create the empty directory on fresh checkouts
  // (e.g. `cargo clippy` / `cargo test` in CI) without writing a fake `index.html`, so
  // runtime `resolve_dx_ui_dir()` still distinguishes a real built bundle from an empty dir.
  if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
    let dist_dir = PathBuf::from(manifest_dir).join("../yanshi-ui-dx/dist/public");
    if !dist_dir.exists() {
      let _ = std::fs::create_dir_all(&dist_dir);
    }
  }
  tauri_build::build();
}
