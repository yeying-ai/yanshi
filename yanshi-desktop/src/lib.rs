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
  base_dir: std::path::PathBuf,
  _instance_lock: runtime_guard::InstanceLockGuard,
  shutdown_started: AtomicBool,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl DesktopLifecycleState {
  fn persist_main_window_geometry(&self, app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
      return;
    };
    let scale = win.scale_factor().unwrap_or(1.0).max(0.5);
    let Ok(size) = win.inner_size() else {
      return;
    };
    let pos = win.outer_position().ok();
    let maximized = win.is_maximized().unwrap_or(false);
    let state = runtime_guard::SavedWindowState {
      width: f64::from(size.width) / scale,
      height: f64::from(size.height) / scale,
      x: pos.map(|p| (f64::from(p.x) / scale).round() as i32),
      y: pos.map(|p| (f64::from(p.y) / scale).round() as i32),
      maximized,
    };
    if let Err(err) = runtime_guard::save_window_state(&self.base_dir, &state) {
      tracing::warn!(error = %err, "failed to persist desktop window state");
    }
  }

  fn shutdown_once(&self, app: &tauri::AppHandle) {
    self.persist_main_window_geometry(app);
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
fn position_is_on_any_monitor(app: &tauri::App, x: i32, y: i32) -> bool {
  let Ok(monitors) = app.available_monitors() else {
    return false;
  };
  monitors.into_iter().any(|monitor| {
    let scale = monitor.scale_factor().max(0.5);
    let pos = monitor.position();
    let size = monitor.size();
    let min_x = (f64::from(pos.x) / scale).floor() as i32;
    let min_y = (f64::from(pos.y) / scale).floor() as i32;
    let max_x = min_x + (f64::from(size.width) / scale).ceil() as i32;
    let max_y = min_y + (f64::from(size.height) / scale).ceil() as i32;
    x >= min_x - 64 && x < max_x - 64 && y >= min_y && y < max_y - 64
  })
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn resolve_desktop_webview_url(api_base: &str) -> Result<WebviewUrl, Box<dyn std::error::Error>> {
  if cfg!(feature = "custom-protocol") {
    return Ok(WebviewUrl::App("index.html".into()));
  }
  // Default desktop route: loopback `/ui/studio`, which serves the built Dioxus
  // WASM bundle (`yanshi-ui-dx/dist/public`) under strict CSP.
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
        local_engine::start_authenticated_loopback_engine_with_state(Some(base_dir.clone())),
      )
      .map_err(|err| -> Box<dyn std::error::Error> {
        tracing::error!(error = %err, "failed to start embedded loopback engine");
        err.into()
      })?;

      let saved_window = runtime_guard::load_window_state(&base_dir);
      let lifecycle = Arc::new(DesktopLifecycleState {
        engine: engine_state,
        base_dir,
        _instance_lock: instance_lock,
        shutdown_started: AtomicBool::new(false),
      });
      app.manage(Arc::clone(&lifecycle));

      let serialized_base = serde_json::to_string(&api_base)?;
      let serialized_token = serde_json::to_string(&api_token)?;
      let init_script = format!(
        "window.__YANSHI_API_BASE__ = {serialized_base}; window.__YANSHI_API_TOKEN__ = {serialized_token};"
      );
      let webview_url = resolve_desktop_webview_url(&api_base)?;
      tracing::info!(api_base = %api_base, "Yanshi Studio desktop window initialized");

      let (width, height) = saved_window
        .as_ref()
        .map_or((1440.0, 900.0), |w| (w.width, w.height));
      let mut win_builder = WebviewWindowBuilder::new(app, "main", webview_url)
        .title("Yanshi Studio · 可视化工作流调试与运行台")
        .inner_size(width, height)
        .min_inner_size(960.0, 640.0)
        .initialization_script(&init_script);
      if let Some(saved) = saved_window.as_ref() {
        if let (Some(x), Some(y)) = (saved.x, saved.y)
          && position_is_on_any_monitor(app, x, y)
        {
          win_builder = win_builder.position(f64::from(x), f64::from(y));
        }
        if saved.maximized {
          win_builder = win_builder.maximized(true);
        }
      }
      let main_window = win_builder.build()?;
      let app_handle_for_close = app.handle().clone();
      let lifecycle_for_close = Arc::clone(&lifecycle);
      main_window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
          lifecycle_for_close.persist_main_window_geometry(&app_handle_for_close);
        }
      });
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
          lifecycle.shutdown_once(_app_handle);
        }
      });
    }
    Err(err) => {
      #[cfg(not(any(target_os = "android", target_os = "ios")))]
      tracing::error!(error = %err, "failed to initialize Yanshi Studio desktop app");
      std::process::exit(if err.to_string().is_empty() { 1 } else { 2 });
    }
  }
}
