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
}
