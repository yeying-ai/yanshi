#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(any(target_os = "android", target_os = "ios")))]
const DEFAULT_LOCAL_WEB_BIND: &str = "127.0.0.1:18080";

/// On macOS, a shell-exported `DYLD_LIBRARY_PATH=/opt/homebrew/lib` overrides
/// Apple `ImageIO.framework`'s private `libPng.dylib` on case-insensitive APFS,
/// causing `AppKit` / `WebKit` icon loading to fault with `SIGBUS` (`0xbad4007`).
/// Re-exec once with `DYLD_LIBRARY_PATH` stripped before `dyld` initializes GUI
/// frameworks in the clean image.
#[cfg(target_os = "macos")]
fn sanitize_macos_dyld_library_path() {
  const GUARD_ENV: &str = "YANSHI_MACOS_DYLD_SANITIZED";
  let has_dyld_override = std::env::var_os("DYLD_LIBRARY_PATH").is_some_and(|val| !val.is_empty());
  if !has_dyld_override || std::env::var_os(GUARD_ENV).is_some() {
    return;
  }
  if let Ok(exe) = std::env::current_exe() {
    use std::os::unix::process::CommandExt;
    let err = std::process::Command::new(exe)
      .args(std::env::args_os().skip(1))
      .env_remove("DYLD_LIBRARY_PATH")
      .env(GUARD_ENV, "1")
      .exec();
    eprintln!("warning: failed to re-exec without DYLD_LIBRARY_PATH: {err}");
  }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
enum LaunchMode {
  Desktop,
  Web(std::net::SocketAddr),
  Help,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn parse_launch_mode() -> Result<LaunchMode, String> {
  let args: Vec<String> = std::env::args().skip(1).collect();
  if args.is_empty() {
    return Ok(LaunchMode::Desktop);
  }
  let mut web_mode = false;
  let mut bind_raw: Option<String> = None;
  let mut idx = 0;
  while idx < args.len() {
    match args[idx].as_str() {
      "-h" | "--help" => return Ok(LaunchMode::Help),
      "--web" => {
        web_mode = true;
        if let Some(next) = args.get(idx + 1)
          && !next.starts_with('-')
        {
          bind_raw = Some(next.clone());
          idx += 1;
        }
      }
      "--bind" => {
        let next = args
          .get(idx + 1)
          .ok_or_else(|| "--bind requires a <host:port> value".to_string())?;
        bind_raw = Some(next.clone());
        web_mode = true;
        idx += 1;
      }
      other => {
        if let Some(val) = other.strip_prefix("--bind=") {
          bind_raw = Some(val.to_string());
          web_mode = true;
        } else if let Some(val) = other.strip_prefix("--web=") {
          bind_raw = Some(val.to_string());
          web_mode = true;
        } else {
          return Err(format!(
            "unknown argument '{other}' (run with --help for usage)"
          ));
        }
      }
    }
    idx += 1;
  }

  if !web_mode {
    return Ok(LaunchMode::Desktop);
  }

  let raw = bind_raw.unwrap_or_else(|| DEFAULT_LOCAL_WEB_BIND.to_string());
  let addr: std::net::SocketAddr = raw
    .parse()
    .map_err(|err| format!("invalid loopback bind address '{raw}': {err}"))?;
  Ok(LaunchMode::Web(addr))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn run_local_web_mode(bind: std::net::SocketAddr) -> Result<(), String> {
  let base_dir = yanshi_studio_lib::local_engine::default_studio_runs_dir();
  let _instance_lock =
    yanshi_studio_lib::runtime_guard::InstanceLockGuard::acquire(&base_dir, "web")?;

  let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .map_err(|err| format!("failed to initialize Tokio runtime: {err}"))?;

  rt.block_on(async move {
    let (listener, app, _token, engine_state) =
      yanshi_studio_lib::local_engine::bind_authenticated_local_web_listener_with_state(
        bind,
        Some(base_dir),
      )
      .await
      .map_err(|err| err.to_string())?;
    let actual = listener
      .local_addr()
      .map_err(|err| format!("failed to inspect bound address: {err}"))?;
    let base = format!("http://{actual}");
    println!("🚀 Yanshi Studio (Zero-Database Local Web Mode)");
    println!("   Studio DAG : {base}/ui/studio");
    println!("   Notebooks  : {base}/ui/notebooks");
    println!("   Harness    : {base}/ui/harness/sessions");
    println!(
      "   Auth       : per-session loopback bearer token enabled (auto-injected in same-origin UI)"
    );
    println!("   Press Ctrl+C to stop.");

    let shutdown_state = engine_state.clone();
    axum::serve(listener, app)
      .with_graceful_shutdown(async move {
        let _ = tokio::signal::ctrl_c().await;
        let (runs, sessions) = shutdown_state.cancel_all_active();
        if runs > 0 || sessions > 0 {
          tracing::info!(
            cancelled_runs = runs,
            cancelled_sessions = sessions,
            "cancelled active local web work on Ctrl+C"
          );
        }
      })
      .await
      .map_err(|err| format!("server error: {err}"))
  })
}

fn main() {
  #[cfg(target_os = "macos")]
  sanitize_macos_dyld_library_path();

  #[cfg(not(any(target_os = "android", target_os = "ios")))]
  yanshi_studio_lib::local_engine::bootstrap_process_env();

  #[cfg(not(any(target_os = "android", target_os = "ios")))]
  match parse_launch_mode() {
    Ok(LaunchMode::Desktop) => {
      let log_dir = yanshi_studio_lib::runtime_guard::default_log_dir();
      let _ =
        yanshi_studio_lib::runtime_guard::init_desktop_logging(&log_dir, cfg!(debug_assertions));
    }
    Ok(LaunchMode::Help) => {
      println!("Usage: yanshi-studio [--web [127.0.0.1:18080]] [--bind <127.0.0.1:port>]");
      println!("  (no args)              Launch native desktop window (zero-Postgres)");
      println!(
        "  --web [host:port]      Serve Studio UI in browser on loopback (default 127.0.0.1:18080)"
      );
      return;
    }
    Ok(LaunchMode::Web(bind)) => {
      let log_dir = yanshi_studio_lib::runtime_guard::default_log_dir();
      let _ = yanshi_studio_lib::runtime_guard::init_desktop_logging(&log_dir, true);
      if let Err(err) = run_local_web_mode(bind) {
        tracing::error!(error = %err, "local web mode exited with error");
        eprintln!("Error: {err}");
        std::process::exit(1);
      }
      return;
    }
    Err(err) => {
      eprintln!("Error: {err}");
      std::process::exit(2);
    }
  }

  yanshi_studio_lib::run();
}
