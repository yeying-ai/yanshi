//! Tauri v2 entry point for Yanshi Studio (Desktop + iOS + Android).

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use std::sync::Arc;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri::{Manager, RunEvent};
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
pub mod runtime_guard;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
struct DesktopLifecycleState {
  engine: local_engine::LocalEngineState,
  _instance_lock: runtime_guard::InstanceLockGuard,
  shutdown_started: AtomicBool,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl DesktopLifecycleState {
  fn shutdown_once(&self) {
    if self.shutdown_started.swap(true, Ordering::AcqRel) {
      return;
    }
    let (cancelled_runs, cancelled_sessions) = self.engine.cancel_all_active();
    if cancelled_runs > 0 || cancelled_sessions > 0 {
      tracing::info!(
        cancelled_runs,
        cancelled_sessions,
        "cancelled active desktop work on exit"
      );
      let deadline = std::time::Instant::now() + std::time::Duration::from_millis(350);
      while self.engine.has_active_work() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(25));
      }
    }
  }
}

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
      let base_dir = local_engine::default_studio_runs_dir();
      let instance_lock = runtime_guard::InstanceLockGuard::acquire(&base_dir, "desktop")
        .map_err(|err| -> Box<dyn std::error::Error> {
          tracing::error!(error = %err, "desktop single-instance lock denied");
          err.into()
        })?;

      let (api_base, api_token, engine_state) = tauri::async_runtime::block_on(
        local_engine::start_authenticated_loopback_engine_with_state(Some(base_dir)),
      )
      .map_err(|err| -> Box<dyn std::error::Error> {
        tracing::error!(error = %err, "failed to start embedded loopback engine");
        err.into()
      })?;

      app.manage(Arc::new(DesktopLifecycleState {
        engine: engine_state,
        _instance_lock: instance_lock,
        shutdown_started: AtomicBool::new(false),
      }));

      let serialized_base = serde_json::to_string(&api_base)?;
      let serialized_token = serde_json::to_string(&api_token)?;
      let init_script = format!(
        "window.__YANSHI_API_BASE__ = {serialized_base}; window.__YANSHI_API_TOKEN__ = {serialized_token};"
      );
      let webview_url = resolve_desktop_webview_url(&api_base)?;
      tracing::info!(api_base = %api_base, "Yanshi Studio desktop window initialized");

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

  match builder.build(tauri::generate_context!()) {
    Ok(app) => {
      app.run(|_app_handle, _event| {
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        if matches!(_event, RunEvent::ExitRequested { .. } | RunEvent::Exit)
          && let Some(lifecycle) = _app_handle.try_state::<Arc<DesktopLifecycleState>>()
        {
          lifecycle.shutdown_once();
        }
      });
    }
    Err(err) => {
      #[cfg(not(any(target_os = "android", target_os = "ios")))]
      tracing::error!(error = %err, "failed to initialize Yanshi Studio desktop app");
      eprintln!("Error starting Yanshi Studio: {err}");
      std::process::exit(if err.to_string().is_empty() { 1 } else { 2 });
    }
  }
}
