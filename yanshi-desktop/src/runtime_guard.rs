//! Desktop lifecycle guards: structured file logging and single-instance lock.

use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::MakeWriter;

const MAX_LOG_FILE_BYTES: u64 = 5 * 1024 * 1024;
const INSTANCE_LOCK_FILENAME: &str = "_instance.lock";
const WINDOW_STATE_FILENAME: &str = "_window_state.json";

/// Persisted desktop window geometry stored in `<base_dir>/_window_state.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedWindowState {
  pub width: f64,
  pub height: f64,
  #[serde(default)]
  pub x: Option<i32>,
  #[serde(default)]
  pub y: Option<i32>,
  #[serde(default)]
  pub maximized: bool,
}

impl SavedWindowState {
  pub fn sanitized(mut self) -> Self {
    if !self.width.is_finite() {
      self.width = 1440.0;
    }
    if !self.height.is_finite() {
      self.height = 900.0;
    }
    self.width = self.width.clamp(960.0, 3840.0);
    self.height = self.height.clamp(640.0, 2400.0);
    self
  }
}

pub fn load_window_state(base_dir: &Path) -> Option<SavedWindowState> {
  let path = base_dir.join(WINDOW_STATE_FILENAME);
  let bytes = std::fs::read(path).ok()?;
  serde_json::from_slice::<SavedWindowState>(&bytes)
    .ok()
    .map(SavedWindowState::sanitized)
}

pub fn save_window_state(base_dir: &Path, state: &SavedWindowState) -> io::Result<()> {
  std::fs::create_dir_all(base_dir)?;
  let sanitized = state.clone().sanitized();
  let bytes = serde_json::to_vec_pretty(&sanitized).map_err(io::Error::other)?;
  let tmp = base_dir.join(format!(
    "{WINDOW_STATE_FILENAME}.tmp.{}",
    uuid::Uuid::new_v4()
  ));
  let dst = base_dir.join(WINDOW_STATE_FILENAME);
  std::fs::write(&tmp, bytes)?;
  std::fs::rename(tmp, dst)
}

/// Open the operating system's native folder picker dialog. Returns `Ok(None)`
/// if the user cancels the dialog.
pub fn pick_directory_native(initial_dir: Option<&Path>) -> Result<Option<PathBuf>, String> {
  #[cfg(target_os = "macos")]
  {
    let mut script = String::from("POSIX path of (choose folder with prompt \"选择工作区目录\"");
    if let Some(dir) = initial_dir.filter(|p| p.is_dir()) {
      let escaped = dir
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
      script.push_str(&format!(" default location POSIX file \"{escaped}\""));
    }
    script.push(')');
    let output = std::process::Command::new("osascript")
      .arg("-e")
      .arg(&script)
      .output()
      .map_err(|err| format!("failed to invoke macOS folder picker: {err}"))?;
    if !output.status.success() {
      let stderr = String::from_utf8_lossy(&output.stderr);
      if stderr.contains("-128") || stderr.to_ascii_lowercase().contains("cancel") {
        return Ok(None);
      }
      return Err(format!("folder picker failed: {}", stderr.trim()));
    }
    let chosen = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if chosen.is_empty() {
      return Ok(None);
    }
    let trimmed = chosen.trim_end_matches('/');
    let path = if trimmed.is_empty() { "/" } else { trimmed };
    Ok(Some(PathBuf::from(path)))
  }

  #[cfg(target_os = "linux")]
  {
    let mut cmd = std::process::Command::new("zenity");
    cmd.args([
      "--file-selection",
      "--directory",
      "--title=Select Workspace",
    ]);
    if let Some(dir) = initial_dir.filter(|p| p.is_dir()) {
      cmd.arg(format!("--filename={}/", dir.display()));
    }
    match cmd.output() {
      Ok(output) if output.status.success() => {
        let chosen = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((!chosen.is_empty()).then(|| PathBuf::from(chosen)))
      }
      Ok(output) if output.status.code() == Some(1) => Ok(None),
      Ok(output) => Err(format!(
        "zenity folder picker failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
      )),
      Err(err) => Err(format!(
        "failed to launch folder picker (install `zenity` on Linux): {err}"
      )),
    }
  }

  #[cfg(target_os = "windows")]
  {
    let initial = initial_dir
      .filter(|p| p.is_dir())
      .map(|p| p.to_string_lossy().replace('\'', "''"))
      .unwrap_or_default();
    let ps = format!(
      "Add-Type -AssemblyName System.Windows.Forms; \
       $d = New-Object System.Windows.Forms.FolderBrowserDialog; \
       if ('{initial}' -ne '') {{ $d.SelectedPath = '{initial}' }}; \
       if ($d.ShowDialog() -eq 'OK') {{ Write-Output $d.SelectedPath }}"
    );
    let output = std::process::Command::new("powershell")
      .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
      .output()
      .map_err(|err| format!("failed to invoke Windows folder picker: {err}"))?;
    if !output.status.success() {
      return Err(format!(
        "Windows folder picker failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
      ));
    }
    let chosen = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!chosen.is_empty()).then(|| PathBuf::from(chosen)))
  }

  #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
  {
    let _ = initial_dir;
    Err("native directory picker is not supported on this platform".to_string())
  }
}

/// Open a local artifact with the system default application, or reveal it in
/// the OS file manager (Finder / Explorer / xdg-open parent directory).
pub fn open_or_reveal_path_native(path: &Path, reveal: bool) -> Result<(), String> {
  if !path.exists() {
    return Err(format!("file does not exist: {}", path.display()));
  }

  #[cfg(target_os = "macos")]
  {
    let mut cmd = std::process::Command::new("open");
    if reveal {
      cmd.arg("-R");
    }
    cmd.arg(path);
    let status = cmd
      .status()
      .map_err(|err| format!("failed to launch `open`: {err}"))?;
    if !status.success() {
      return Err(format!("`open` exited with status {status}"));
    }
    Ok(())
  }

  #[cfg(target_os = "linux")]
  {
    let target = if reveal {
      path.parent().unwrap_or(path)
    } else {
      path
    };
    let status = std::process::Command::new("xdg-open")
      .arg(target)
      .status()
      .map_err(|err| format!("failed to launch `xdg-open`: {err}"))?;
    if !status.success() {
      return Err(format!("`xdg-open` exited with status {status}"));
    }
    Ok(())
  }

  #[cfg(target_os = "windows")]
  {
    let status = if reveal {
      std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .status()
        .map_err(|err| format!("failed to launch explorer: {err}"))?
    } else {
      std::process::Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(path)
        .status()
        .map_err(|err| format!("failed to open file: {err}"))?
    };
    if !status.success() {
      return Err(format!("OS opener exited with status {status}"));
    }
    Ok(())
  }

  #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
  {
    let _ = (path, reveal);
    Err("native file opener is not supported on this platform".to_string())
  }
}

/// Default log directory under the user's home (`~/.yanshi/logs`).
pub fn default_log_dir() -> PathBuf {
  dirs::home_dir()
    .unwrap_or_else(std::env::temp_dir)
    .join(".yanshi")
    .join("logs")
}

#[derive(Clone)]
struct TeeLogWriter {
  file: Arc<Mutex<File>>,
  also_stderr: bool,
}

struct TeeLogGuard {
  file: Arc<Mutex<File>>,
  also_stderr: bool,
}

impl Write for TeeLogGuard {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    if let Ok(mut guard) = self.file.lock() {
      let _ = guard.write_all(buf);
    }
    if self.also_stderr {
      let _ = io::stderr().write_all(buf);
    }
    Ok(buf.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    if let Ok(mut guard) = self.file.lock() {
      let _ = guard.flush();
    }
    if self.also_stderr {
      let _ = io::stderr().flush();
    }
    Ok(())
  }
}

impl<'a> MakeWriter<'a> for TeeLogWriter {
  type Writer = TeeLogGuard;

  fn make_writer(&'a self) -> Self::Writer {
    TeeLogGuard {
      file: Arc::clone(&self.file),
      also_stderr: self.also_stderr,
    }
  }
}

fn rotate_log_if_oversized(log_path: &Path) {
  let Ok(meta) = std::fs::metadata(log_path) else {
    return;
  };
  if meta.len() < MAX_LOG_FILE_BYTES {
    return;
  }
  let rotated = log_path.with_extension("log.1");
  let _ = std::fs::rename(log_path, rotated);
}

/// Initialize process-wide `tracing` logging to `<log_dir>/desktop.log`
/// (and optionally stderr for `--web` / debug builds). Idempotent.
pub fn init_desktop_logging(log_dir: &Path, also_stderr: bool) -> io::Result<PathBuf> {
  static INIT: Once = Once::new();
  let log_path = log_dir.join("desktop.log");
  let mut init_err: Option<io::Error> = None;

  INIT.call_once(|| {
    if let Err(err) = std::fs::create_dir_all(log_dir) {
      init_err = Some(err);
      return;
    }
    rotate_log_if_oversized(&log_path);
    let file = match OpenOptions::new().create(true).append(true).open(&log_path) {
      Ok(f) => f,
      Err(err) => {
        init_err = Some(err);
        return;
      }
    };
    let writer = TeeLogWriter {
      file: Arc::new(Mutex::new(file)),
      also_stderr,
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
      EnvFilter::new(
        "info,yanshi_studio_lib=info,yanshi_core=info,yanshi_llm=info,yanshi_harness=info,hyper=warn,reqwest=warn",
      )
    });
    let _ = tracing_subscriber::fmt()
      .with_env_filter(filter)
      .with_ansi(false)
      .with_target(true)
      .with_writer(writer)
      .try_init();
  });

  if let Some(err) = init_err {
    return Err(err);
  }
  Ok(log_path)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstanceLockMetadata {
  pid: u32,
  mode: String,
  started_at: String,
}

/// Advisory OS file lock guarding `<base_dir>/_instance.lock` so concurrent
/// GUI / `--web` launches cannot corrupt local run and notebook JSON files.
/// Automatically released by the OS kernel when dropped or when the process exits.
pub struct InstanceLockGuard {
  _file: File,
  lock_path: PathBuf,
}

impl InstanceLockGuard {
  /// Acquire an exclusive non-blocking OS lock in `base_dir`.
  pub fn acquire(base_dir: &Path, mode: &str) -> Result<Self, String> {
    std::fs::create_dir_all(base_dir)
      .map_err(|err| format!("failed to create data dir '{}': {err}", base_dir.display()))?;
    let lock_path = base_dir.join(INSTANCE_LOCK_FILENAME);
    let mut file = OpenOptions::new()
      .create(true)
      .read(true)
      .write(true)
      .truncate(false)
      .open(&lock_path)
      .map_err(|err| format!("failed to open lock file '{}': {err}", lock_path.display()))?;

    match file.try_lock() {
      Ok(()) => {
        let meta = InstanceLockMetadata {
          pid: std::process::id(),
          mode: mode.to_string(),
          started_at: Utc::now().to_rfc3339(),
        };
        let _ = file.set_len(0);
        let _ = file.seek(SeekFrom::Start(0));
        if let Ok(json) = serde_json::to_vec_pretty(&meta) {
          let _ = file.write_all(&json);
          let _ = file.flush();
        }
        Ok(Self {
          _file: file,
          lock_path,
        })
      }
      Err(_) => {
        let holder = std::fs::read_to_string(&lock_path)
          .ok()
          .and_then(|raw| serde_json::from_str::<InstanceLockMetadata>(&raw).ok());
        match holder {
          Some(info) => Err(format!(
            "Another Yanshi Studio instance is already running (pid {}, mode '{}', started {}) sharing '{}'. Close the existing instance first.",
            info.pid,
            info.mode,
            info.started_at,
            base_dir.display()
          )),
          None => Err(format!(
            "Another Yanshi Studio instance already holds the lock at '{}'. Close the existing instance first.",
            lock_path.display()
          )),
        }
      }
    }
  }

  pub fn lock_path(&self) -> &Path {
    &self.lock_path
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn instance_lock_blocks_second_holder_and_releases_on_drop() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let first = InstanceLockGuard::acquire(tmp.path(), "desktop").expect("first lock succeeds");
    assert!(first.lock_path().is_file());

    let second = InstanceLockGuard::acquire(tmp.path(), "web");
    match second {
      Ok(_) => panic!("second concurrent lock must fail"),
      Err(err) => {
        assert!(err.contains("already running"), "unexpected error: {err}");
        assert!(err.contains("desktop"), "missing holder mode: {err}");
      }
    }

    drop(first);

    let third = InstanceLockGuard::acquire(tmp.path(), "web").expect("lock re-acquired after drop");
    assert!(third.lock_path().is_file());
  }

  #[test]
  fn rotate_log_moves_oversized_file_to_backup() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let log_path = tmp.path().join("desktop.log");
    let oversized = vec![b'x'; (MAX_LOG_FILE_BYTES as usize) + 16];
    std::fs::write(&log_path, oversized).expect("write oversized log");

    rotate_log_if_oversized(&log_path);
    assert!(!log_path.exists());
    assert!(tmp.path().join("desktop.log.1").is_file());
  }

  #[test]
  fn window_state_round_trips_and_clamps_bounds() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let raw = SavedWindowState {
      width: 200.0,
      height: 9999.0,
      x: Some(120),
      y: Some(80),
      maximized: true,
    };
    save_window_state(tmp.path(), &raw).expect("save window state");
    let loaded = load_window_state(tmp.path()).expect("load window state");
    assert_eq!(loaded.width, 960.0);
    assert_eq!(loaded.height, 2400.0);
    assert_eq!(loaded.x, Some(120));
    assert_eq!(loaded.y, Some(80));
    assert!(loaded.maximized);
  }
}
