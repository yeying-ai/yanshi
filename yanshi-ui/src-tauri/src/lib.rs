//! Tauri v2 entry point for Yanshi Studio (Desktop + iOS + Android).

use tauri::{WebviewUrl, WebviewWindowBuilder};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod local_engine;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod local_harness;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod local_notebooks;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod local_questions;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn resolve_desktop_webview_url(api_base: &str) -> Result<WebviewUrl, Box<dyn std::error::Error>> {
  if cfg!(feature = "custom-protocol") {
    return Ok(WebviewUrl::App("index.html".into()));
  }
  let vite_dev_addr = std::net::SocketAddr::from(([127, 0, 0, 1], 5173));
  if std::net::TcpStream::connect_timeout(&vite_dev_addr, std::time::Duration::from_millis(60))
    .is_ok()
  {
    return Ok(WebviewUrl::App("index.html".into()));
  }
  let studio_url: tauri::Url = format!("{api_base}/ui/studio").parse()?;
  Ok(WebviewUrl::External(studio_url))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let builder = tauri::Builder::default().setup(|app| {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
      let (api_base, api_token) =
        tauri::async_runtime::block_on(local_engine::start_authenticated_loopback_engine(None))
          .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;

      let serialized_base = serde_json::to_string(&api_base)?;
      let serialized_token = serde_json::to_string(&api_token)?;
      let init_script = format!(
        "window.__YANSHI_API_BASE__ = {serialized_base}; window.__YANSHI_API_TOKEN__ = {serialized_token};"
      );
      let webview_url = resolve_desktop_webview_url(&api_base)?;

      WebviewWindowBuilder::new(app, "main", webview_url)
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
