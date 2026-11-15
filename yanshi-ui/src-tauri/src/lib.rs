//! Tauri v2 entry point for Yanshi Studio (Desktop + iOS + Android).

use tauri::{WebviewUrl, WebviewWindowBuilder};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod local_engine;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod local_harness;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod local_notebooks;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let builder = tauri::Builder::default().setup(|app| {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
      let api_base = tauri::async_runtime::block_on(local_engine::start_loopback_engine(None))
        .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;

      let serialized_base = serde_json::to_string(&api_base)?;
      let init_script = format!("window.__YANSHI_API_BASE__ = {serialized_base};");

      WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Yanshi Studio · 可视化工作流调试与运行台")
        .inner_size(1440.0, 900.0)
        .min_inner_size(960.0, 640.0)
        .initialization_script(&init_script)
        .build()?;
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
      WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into())).build()?;
    }

    Ok(())
  });

  if let Err(err) = builder.run(tauri::generate_context!()) {
    std::process::exit(if err.to_string().is_empty() { 1 } else { 2 });
  }
}
