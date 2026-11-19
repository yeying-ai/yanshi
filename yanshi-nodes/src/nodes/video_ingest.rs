use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{Value, json};
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  run_context,
  value::FlowValue,
};
use yanshi_tools::SandboxPolicy;
use yanshi_tools::builtin::HttpTool;

use crate::common::{artifacts, sandboxed};

const MAX_VIDEO_BYTES: usize = 150 * 1024 * 1024;
const HTTP_TIMEOUT_SECS: u64 = 120;
const FFMPEG_TIMEOUT_SECS: u64 = 300;
const YTDLP_TIMEOUT_SECS: u64 = 150;
const DEFAULT_SUBTITLE_LANGS: &str = "zh.*,en.*";

/// Ingest a video or audio source (`FlowValue::File`, public video URL,
/// `data:` URI, or sandbox-allowed local path):
/// 1. Prefer native/embedded `.srt` subtitles (`yt-dlp` on URLs or embedded
///    subtitle streams in container files) when `prefer_subtitles` is true.
/// 2. Otherwise extract 16 kHz mono 48 kbps `.mp3` audio via `ffmpeg` inside
///    the platform OS sandbox.
/// 3. Emit `media_or_subtitles` (`FlowValue::File`) so a downstream `asr` node
///    either passes native `.srt` straight through or transcribes the `.mp3`.
#[derive(Clone)]
pub struct VideoIngestNode {
  id: String,
  policy: Arc<SandboxPolicy>,
  no_proxy: bool,
}

impl std::fmt::Debug for VideoIngestNode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("VideoIngestNode")
      .field("id", &self.id)
      .finish_non_exhaustive()
  }
}

impl VideoIngestNode {
  pub fn new(id: impl Into<String>, policy: Arc<SandboxPolicy>) -> Self {
    Self {
      id: id.into(),
      policy,
      no_proxy: false,
    }
  }

  pub fn with_no_proxy(mut self) -> Self {
    self.no_proxy = true;
    self
  }
}

struct IngestOutcome {
  title: String,
  duration_seconds: Option<f64>,
  subtitles_srt: Option<String>,
  audio_mp3: Option<Vec<u8>>,
}

#[async_trait]
impl AsyncNode for VideoIngestNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let prefer_subtitles = optional_bool(inputs, "prefer_subtitles").unwrap_or(true);
    let subtitle_langs = optional_str(inputs, "subtitle_langs")
      .unwrap_or(DEFAULT_SUBTITLE_LANGS)
      .to_string();
    let max_duration_seconds = optional_u64(inputs, "max_duration_seconds").filter(|s| *s > 0);

    let base_dir = artifacts::artifact_dir(&self.id);
    let workdir = base_dir.join(format!(".video-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&workdir)
      .await
      .map_err(|err| YanshiError::NodeExecutionFailed {
        message: format!(
          "{}: cannot create scratch dir '{}': {err}",
          self.id,
          workdir.display()
        ),
      })?;

    let outcome_res = self
      .ingest_into_workdir(
        inputs,
        &workdir,
        prefer_subtitles,
        &subtitle_langs,
        max_duration_seconds,
      )
      .await;
    let _ = tokio::fs::remove_dir_all(&workdir).await;
    let outcome = outcome_res?;

    let mut outputs = HashMap::new();
    outputs.insert("title".to_string(), FlowValue::Json(json!(outcome.title)));
    outputs.insert(
      "duration_seconds".to_string(),
      FlowValue::Json(json!(outcome.duration_seconds)),
    );

    if let Some(srt_text) = outcome.subtitles_srt {
      let srt_file =
        artifacts::write_artifact(&self.id, srt_text.as_bytes(), "application/x-subrip").await?;
      outputs.insert("has_subtitles".to_string(), FlowValue::Json(json!(true)));
      outputs.insert(
        "subtitles_srt".to_string(),
        FlowValue::Json(json!(srt_text)),
      );
      outputs.insert("subtitles_file".to_string(), srt_file.clone());
      outputs.insert("audio_file".to_string(), FlowValue::Json(Value::Null));
      outputs.insert("media_or_subtitles".to_string(), srt_file.clone());
      outputs.insert("output".to_string(), srt_file);
      return Ok(outputs);
    }

    let mp3_bytes = outcome
      .audio_mp3
      .ok_or_else(|| YanshiError::NodeExecutionFailed {
        message: format!(
          "{}: neither subtitles nor extractable audio track found in video source",
          self.id
        ),
      })?;
    let audio_file = artifacts::write_artifact(&self.id, &mp3_bytes, "audio/mpeg").await?;
    outputs.insert("has_subtitles".to_string(), FlowValue::Json(json!(false)));
    outputs.insert("subtitles_srt".to_string(), FlowValue::Json(json!("")));
    outputs.insert("subtitles_file".to_string(), FlowValue::Json(Value::Null));
    outputs.insert("audio_file".to_string(), audio_file.clone());
    outputs.insert("media_or_subtitles".to_string(), audio_file.clone());
    outputs.insert("output".to_string(), audio_file);
    Ok(outputs)
  }
}

impl VideoIngestNode {
  async fn ingest_into_workdir(
    &self,
    inputs: &AsyncNodeInputs,
    workdir: &Path,
    prefer_subtitles: bool,
    subtitle_langs: &str,
    max_duration_seconds: Option<u64>,
  ) -> Result<IngestOutcome, YanshiError> {
    let source = inputs
      .get("source")
      .ok_or_else(|| input_err(&self.id, "missing required input 'source'"))?;

    match source {
      FlowValue::File { path, .. } => {
        let canonical = resolve_readable_path(&self.id, path, &self.policy)?;
        self
          .process_local_media(&canonical, workdir, prefer_subtitles, max_duration_seconds)
          .await
      }
      FlowValue::Url { url, .. } => {
        self
          .process_url_media(
            url,
            workdir,
            prefer_subtitles,
            subtitle_langs,
            max_duration_seconds,
          )
          .await
      }
      FlowValue::Json(Value::Object(obj)) => {
        let path_str = obj
          .get("path")
          .and_then(Value::as_str)
          .ok_or_else(|| input_err(&self.id, "'source' object must contain a string 'path'"))?;
        let canonical = resolve_readable_path(&self.id, Path::new(path_str), &self.policy)?;
        self
          .process_local_media(&canonical, workdir, prefer_subtitles, max_duration_seconds)
          .await
      }
      FlowValue::Json(Value::String(raw)) => {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
          return Err(input_err(&self.id, "'source' must not be empty"));
        }
        if trimmed.starts_with("data:") {
          let staged = stage_data_uri(&self.id, trimmed, workdir).await?;
          self
            .process_local_media(&staged, workdir, prefer_subtitles, max_duration_seconds)
            .await
        } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
          self
            .process_url_media(
              trimmed,
              workdir,
              prefer_subtitles,
              subtitle_langs,
              max_duration_seconds,
            )
            .await
        } else {
          let canonical = resolve_readable_path(&self.id, Path::new(trimmed), &self.policy)?;
          self
            .process_local_media(&canonical, workdir, prefer_subtitles, max_duration_seconds)
            .await
        }
      }
      other => Err(input_err(
        &self.id,
        format!("unsupported 'source' value: {other:?}"),
      )),
    }
  }

  async fn process_url_media(
    &self,
    raw_url: &str,
    workdir: &Path,
    prefer_subtitles: bool,
    subtitle_langs: &str,
    max_duration_seconds: Option<u64>,
  ) -> Result<IngestOutcome, YanshiError> {
    let mut http = HttpTool::new(Arc::clone(&self.policy))
      .map_err(|err| YanshiError::AsyncExecutionError {
        message: format!("{}: failed to build HTTP client: {err}", self.id),
      })?
      .with_timeout(Duration::from_secs(HTTP_TIMEOUT_SECS));
    if self.no_proxy {
      http = http.with_no_proxy();
    }

    // Always run SSRF / domain / private-IP validation BEFORE invoking yt-dlp or HTTP GET.
    http
      .validate_url_str(raw_url)
      .await
      .map_err(|err| YanshiError::NodeInputError {
        message: format!("{}: video URL rejected by sandbox policy: {err}", self.id),
      })?;

    if !looks_like_direct_media_url(raw_url)
      && allows_ytdlp_for_url(raw_url)
      && let Some(ytdlp) = sandboxed::find_tool("yt-dlp")
      && let Ok(outcome) = self
        .try_ytdlp(
          &ytdlp,
          raw_url,
          workdir,
          prefer_subtitles,
          subtitle_langs,
          max_duration_seconds,
        )
        .await
    {
      return Ok(outcome);
    }

    // Fallback (or direct media file URL such as `.mp4` / `.mp3` / `.srt`):
    // download with HttpTool::send_bytes and process locally via sandboxed ffmpeg.
    let resp = http
      .send_bytes(
        &json!({ "url": raw_url, "method": "GET" }),
        Some(MAX_VIDEO_BYTES),
      )
      .await
      .map_err(|err| YanshiError::AsyncExecutionError {
        message: format!("{}: failed to fetch media URL '{raw_url}': {err}", self.id),
      })?;
    if !resp.status.is_success() {
      let preview: String = String::from_utf8_lossy(&resp.body)
        .chars()
        .take(200)
        .collect();
      return Err(YanshiError::NodeExecutionFailed {
        message: format!(
          "{}: HTTP {} fetching media URL '{raw_url}': {preview}",
          self.id, resp.status
        ),
      });
    }

    let ext = infer_media_extension(raw_url, resp.content_type.as_deref());
    let staged_path = workdir.join(format!("downloaded.{ext}"));
    tokio::fs::write(&staged_path, &resp.body)
      .await
      .map_err(|err| YanshiError::NodeExecutionFailed {
        message: format!("{}: cannot write staged media: {err}", self.id),
      })?;

    let mut outcome = self
      .process_local_media(
        &staged_path,
        workdir,
        prefer_subtitles,
        max_duration_seconds,
      )
      .await?;
    if outcome.title == "downloaded" {
      outcome.title = title_from_url(raw_url);
    }
    Ok(outcome)
  }

  async fn try_ytdlp(
    &self,
    ytdlp: &Path,
    url: &str,
    workdir: &Path,
    prefer_subtitles: bool,
    subtitle_langs: &str,
    max_duration_seconds: Option<u64>,
  ) -> Result<IngestOutcome, YanshiError> {
    if prefer_subtitles {
      let sub_args = vec![
        "--no-playlist".to_string(),
        "--no-exec".to_string(),
        "--restrict-filenames".to_string(),
        "--skip-download".to_string(),
        "--write-subs".to_string(),
        "--write-auto-subs".to_string(),
        "--sub-langs".to_string(),
        subtitle_langs.to_string(),
        "--convert-subs".to_string(),
        "srt".to_string(),
        "-o".to_string(),
        "yt_sub.%(ext)s".to_string(),
        "--".to_string(),
        url.to_string(),
      ];
      let _ = run_ytdlp_command(&self.id, ytdlp, workdir, &sub_args, self.no_proxy).await;
      if let Some(srt_text) = find_first_subtitle_in_dir(workdir).await {
        return Ok(IngestOutcome {
          title: title_from_url(url),
          duration_seconds: None,
          subtitles_srt: Some(srt_text),
          audio_mp3: None,
        });
      }
    }

    let dl_args = vec![
      "--no-playlist".to_string(),
      "--no-exec".to_string(),
      "--restrict-filenames".to_string(),
      "--max-filesize".to_string(),
      "150M".to_string(),
      "-f".to_string(),
      "ba/bestaudio/best".to_string(),
      "-o".to_string(),
      "yt_media.%(ext)s".to_string(),
      "--".to_string(),
      url.to_string(),
    ];
    let dl_out = run_ytdlp_command(&self.id, ytdlp, workdir, &dl_args, self.no_proxy).await?;
    if !dl_out.status.success() {
      return Err(YanshiError::NodeExecutionFailed {
        message: format!(
          "{}: yt-dlp exited with {}: {}",
          self.id,
          dl_out.status,
          sandboxed::output_tail(&dl_out, 10)
        ),
      });
    }

    let media_file = find_downloaded_media_in_dir(workdir).await.ok_or_else(|| {
      YanshiError::NodeExecutionFailed {
        message: format!(
          "{}: yt-dlp finished without producing a media file",
          self.id
        ),
      }
    })?;
    let mut outcome = self
      .process_local_media(&media_file, workdir, false, max_duration_seconds)
      .await?;
    outcome.title = title_from_url(url);
    Ok(outcome)
  }

  async fn process_local_media(
    &self,
    input_path: &Path,
    workdir: &Path,
    prefer_subtitles: bool,
    max_duration_seconds: Option<u64>,
  ) -> Result<IngestOutcome, YanshiError> {
    let title = input_path
      .file_stem()
      .map(|s| s.to_string_lossy().into_owned())
      .unwrap_or_else(|| "video".to_string());

    let ext = input_path
      .extension()
      .and_then(|e| e.to_str())
      .unwrap_or_default()
      .to_ascii_lowercase();
    if matches!(ext.as_str(), "srt" | "vtt") {
      let bytes = tokio::fs::read(input_path).await.map_err(|err| {
        input_err(
          &self.id,
          format!(
            "cannot read subtitle file '{}': {err}",
            input_path.display()
          ),
        )
      })?;
      let raw = String::from_utf8_lossy(&bytes).into_owned();
      let srt = normalize_subtitle_content(&raw);
      return Ok(IngestOutcome {
        title,
        duration_seconds: None,
        subtitles_srt: Some(srt),
        audio_mp3: None,
      });
    }

    let ffmpeg =
      sandboxed::find_tool("ffmpeg").ok_or_else(|| YanshiError::NodeExecutionFailed {
        message: format!("{}: ffmpeg is not installed (`yanshi doctor`)", self.id),
      })?;

    let mut read_paths = sandboxed::font_dirs();
    if let Some(parent) = input_path.parent() {
      read_paths.push(parent.to_path_buf());
    }

    let duration_seconds = probe_duration_seconds(&self.id, input_path, workdir, &read_paths).await;

    if prefer_subtitles {
      let embedded_srt = workdir.join("embedded.srt");
      let sub_args: Vec<OsString> = vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-y".into(),
        "-i".into(),
        input_path.as_os_str().to_os_string(),
        "-map".into(),
        "0:s:0".into(),
        embedded_srt.as_os_str().to_os_string(),
      ];
      if let Ok(sub_out) = sandboxed::run(sandboxed::SandboxedRun {
        node: &self.id,
        program: ffmpeg.clone(),
        args: sub_args,
        workdir,
        read_paths: read_paths.clone(),
        write_paths: Vec::new(),
        env: Vec::new(),
        timeout: Duration::from_secs(60),
      })
      .await
        && sub_out.status.success()
        && let Ok(bytes) = tokio::fs::read(&embedded_srt).await
      {
        let raw = String::from_utf8_lossy(&bytes);
        let normalized = normalize_subtitle_content(&raw);
        if normalized.contains("-->") {
          return Ok(IngestOutcome {
            title,
            duration_seconds,
            subtitles_srt: Some(normalized),
            audio_mp3: None,
          });
        }
      }
    }

    let output_mp3 = workdir.join("audio.mp3");
    let mut args: Vec<OsString> = vec![
      "-hide_banner".into(),
      "-nostdin".into(),
      "-y".into(),
      "-i".into(),
      input_path.as_os_str().to_os_string(),
    ];
    if let Some(secs) = max_duration_seconds {
      args.push("-t".into());
      args.push(secs.to_string().into());
    }
    args.extend([
      OsString::from("-vn"),
      OsString::from("-ac"),
      OsString::from("1"),
      OsString::from("-ar"),
      OsString::from("16000"),
      OsString::from("-b:a"),
      OsString::from("48k"),
      output_mp3.as_os_str().to_os_string(),
    ]);

    let out = sandboxed::run(sandboxed::SandboxedRun {
      node: &self.id,
      program: ffmpeg,
      args,
      workdir,
      read_paths,
      write_paths: Vec::new(),
      env: Vec::new(),
      timeout: Duration::from_secs(FFMPEG_TIMEOUT_SECS),
    })
    .await?;

    if !out.status.success() {
      return Err(YanshiError::NodeExecutionFailed {
        message: format!(
          "{}: ffmpeg audio extraction exited with {}: {}",
          self.id,
          out.status,
          sandboxed::output_tail(&out, 12)
        ),
      });
    }

    let mp3_bytes =
      tokio::fs::read(&output_mp3)
        .await
        .map_err(|err| YanshiError::NodeExecutionFailed {
          message: format!("{}: cannot read extracted audio.mp3: {err}", self.id),
        })?;
    if mp3_bytes.is_empty() {
      return Err(YanshiError::NodeExecutionFailed {
        message: format!("{}: extracted audio.mp3 is empty", self.id),
      });
    }

    Ok(IngestOutcome {
      title,
      duration_seconds,
      subtitles_srt: None,
      audio_mp3: Some(mp3_bytes),
    })
  }
}

async fn probe_duration_seconds(
  node_id: &str,
  input_path: &Path,
  workdir: &Path,
  read_paths: &[PathBuf],
) -> Option<f64> {
  let ffprobe = sandboxed::find_tool("ffprobe")?;
  let args: Vec<OsString> = vec![
    "-v".into(),
    "error".into(),
    "-show_entries".into(),
    "format=duration".into(),
    "-of".into(),
    "default=noprint_wrappers=1:nokey=1".into(),
    input_path.as_os_str().to_os_string(),
  ];
  let out = sandboxed::run(sandboxed::SandboxedRun {
    node: node_id,
    program: ffprobe,
    args,
    workdir,
    read_paths: read_paths.to_vec(),
    write_paths: Vec::new(),
    env: Vec::new(),
    timeout: Duration::from_secs(20),
  })
  .await
  .ok()?;
  if !out.status.success() {
    return None;
  }
  String::from_utf8_lossy(&out.stdout)
    .trim()
    .parse::<f64>()
    .ok()
    .filter(|d| d.is_finite() && *d > 0.0)
}

async fn run_ytdlp_command(
  node_id: &str,
  ytdlp: &Path,
  workdir: &Path,
  args: &[String],
  no_proxy: bool,
) -> Result<std::process::Output, YanshiError> {
  let mut cmd = tokio::process::Command::new(ytdlp);
  cmd
    .arg("--ignore-config")
    .arg("--compat-options")
    .arg("no-external-downloader")
    .args(args)
    .current_dir(workdir)
    .kill_on_drop(true);
  cmd.env_clear();
  for key in [
    "PATH",
    "HOME",
    "LANG",
    "LC_ALL",
    "TMPDIR",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
  ] {
    if let Some(val) = std::env::var_os(key) {
      cmd.env(key, val);
    }
  }
  if no_proxy {
    cmd.arg("--proxy").arg("");
  } else {
    for proxy_key in [
      "HTTPS_PROXY",
      "HTTP_PROXY",
      "https_proxy",
      "http_proxy",
      "ALL_PROXY",
    ] {
      if let Some(val) = std::env::var_os(proxy_key) {
        cmd.env(proxy_key, val);
      }
    }
  }

  tokio::time::timeout(Duration::from_secs(YTDLP_TIMEOUT_SECS), cmd.output())
    .await
    .map_err(|_| YanshiError::NodeExecutionFailed {
      message: format!("{node_id}: yt-dlp timed out after {YTDLP_TIMEOUT_SECS}s"),
    })?
    .map_err(|err| YanshiError::NodeExecutionFailed {
      message: format!("{node_id}: failed to spawn yt-dlp: {err}"),
    })
}

async fn find_first_subtitle_in_dir(workdir: &Path) -> Option<String> {
  let mut rd = tokio::fs::read_dir(workdir).await.ok()?;
  let mut candidates = Vec::new();
  while let Ok(Some(entry)) = rd.next_entry().await {
    let path = entry.path();
    let ext = path
      .extension()
      .and_then(|e| e.to_str())
      .unwrap_or_default()
      .to_ascii_lowercase();
    if matches!(ext.as_str(), "srt" | "vtt") {
      candidates.push(path);
    }
  }
  candidates.sort();
  for path in candidates {
    if let Ok(bytes) = tokio::fs::read(&path).await {
      let raw = String::from_utf8_lossy(&bytes);
      let normalized = normalize_subtitle_content(&raw);
      if normalized.contains("-->") {
        return Some(normalized);
      }
    }
  }
  None
}

async fn find_downloaded_media_in_dir(workdir: &Path) -> Option<PathBuf> {
  let mut rd = tokio::fs::read_dir(workdir).await.ok()?;
  while let Ok(Some(entry)) = rd.next_entry().await {
    let path = entry.path();
    let name = path
      .file_name()
      .and_then(|n| n.to_str())
      .unwrap_or_default();
    let ext = path
      .extension()
      .and_then(|e| e.to_str())
      .unwrap_or_default()
      .to_ascii_lowercase();
    if name.starts_with("yt_media.") && !matches!(ext.as_str(), "srt" | "vtt" | "part" | "ytdl") {
      return Some(path);
    }
  }
  None
}

fn normalize_subtitle_content(raw: &str) -> String {
  let clean = raw.replace("\r\n", "\n");
  let trimmed = clean.trim();
  if !trimmed.starts_with("WEBVTT") {
    return format!("{trimmed}\n");
  }
  let mut out = String::new();
  let mut idx = 1usize;
  let mut cue_ts: Option<String> = None;
  let mut cue_lines = Vec::new();
  for line in trimmed.lines() {
    let t = line.trim();
    if t.starts_with("WEBVTT")
      || t.starts_with("Kind:")
      || t.starts_with("Language:")
      || t.starts_with("NOTE")
    {
      continue;
    }
    if t.is_empty() {
      if let Some(ts) = cue_ts.take()
        && !cue_lines.is_empty()
      {
        out.push_str(&format!("{idx}\n{ts}\n{}\n\n", cue_lines.join("\n")));
        idx += 1;
        cue_lines.clear();
      }
      continue;
    }
    if let Some((start, rest)) = t.split_once("-->") {
      let end = rest.split_whitespace().next().unwrap_or(rest);
      let fmt_ts = |s: &str| {
        let rep = s.trim().replace('.', ",");
        if rep.chars().filter(|c| *c == ':').count() == 1 {
          format!("00:{rep}")
        } else {
          rep
        }
      };
      cue_ts = Some(format!("{} --> {}", fmt_ts(start), fmt_ts(end)));
    } else if cue_ts.is_some() {
      cue_lines.push(t.to_string());
    }
  }
  if let Some(ts) = cue_ts
    && !cue_lines.is_empty()
  {
    out.push_str(&format!("{idx}\n{ts}\n{}\n", cue_lines.join("\n")));
  }
  out.trim_end().to_string() + "\n"
}

fn allows_ytdlp_for_url(raw_url: &str) -> bool {
  // On local CLI / SDK runs (`current_allowed_path_roots() == None`), allow yt-dlp
  // on any SSRF-validated URL. On the multi-tenant gateway, restrict yt-dlp
  // (which follows HTTP redirects and HLS/HTML manifests outside HttpTool's
  // per-hop DNS pinning) to known public video platforms; all other URLs go
  // through SSRF-pinned `HttpTool::send_bytes` + sandboxed `ffmpeg`.
  if run_context::current_allowed_path_roots().is_none() {
    return true;
  }
  let Ok(parsed) = reqwest::Url::parse(raw_url) else {
    return false;
  };
  let Some(host) = parsed
    .host_str()
    .map(|h| h.trim_end_matches('.').to_ascii_lowercase())
  else {
    return false;
  };
  [
    "youtube.com",
    "youtu.be",
    "youtube-nocookie.com",
    "bilibili.com",
    "b23.tv",
    "vimeo.com",
    "ted.com",
    "twitch.tv",
  ]
  .iter()
  .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}

fn looks_like_direct_media_url(raw_url: &str) -> bool {
  let path_part = raw_url.split(['?', '#']).next().unwrap_or(raw_url);
  let lower = path_part.to_ascii_lowercase();
  [
    ".mp4", ".mov", ".webm", ".mkv", ".mp3", ".wav", ".m4a", ".ogg", ".flac", ".srt", ".vtt",
  ]
  .iter()
  .any(|ext| lower.ends_with(ext))
}

fn infer_media_extension(raw_url: &str, content_type: Option<&str>) -> &'static str {
  let path_part = raw_url
    .split(['?', '#'])
    .next()
    .unwrap_or(raw_url)
    .to_ascii_lowercase();
  for ext in [
    "mp4", "mov", "webm", "mkv", "mp3", "wav", "m4a", "ogg", "flac", "srt", "vtt",
  ] {
    if path_part.ends_with(&format!(".{ext}")) {
      return ext;
    }
  }
  match content_type
    .and_then(|ct| ct.split(';').next())
    .unwrap_or_default()
    .trim()
    .to_ascii_lowercase()
    .as_str()
  {
    "video/webm" => "webm",
    "video/quicktime" => "mov",
    "audio/mpeg" | "audio/mp3" => "mp3",
    "audio/wav" | "audio/x-wav" => "wav",
    "application/x-subrip" => "srt",
    "text/vtt" => "vtt",
    _ => "mp4",
  }
}

fn title_from_url(raw_url: &str) -> String {
  reqwest::Url::parse(raw_url)
    .ok()
    .and_then(|u| {
      u.path_segments()
        .and_then(|mut segs| segs.rfind(|s| !s.is_empty()))
        .map(|s| s.to_string())
    })
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| "video".to_string())
}

async fn stage_data_uri(node: &str, uri: &str, workdir: &Path) -> Result<PathBuf, YanshiError> {
  let (header, payload) = uri
    .split_once(',')
    .ok_or_else(|| input_err(node, "malformed data URI (missing ',')"))?;
  let clean: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
  let bytes = base64::engine::general_purpose::STANDARD
    .decode(clean)
    .map_err(|err| input_err(node, format!("invalid base64 in data URI: {err}")))?;
  if bytes.len() > MAX_VIDEO_BYTES {
    return Err(input_err(node, "uploaded video exceeds 150 MB limit"));
  }
  let mime = header
    .strip_prefix("data:")
    .and_then(|h| h.split(';').next())
    .unwrap_or("video/mp4");
  let ext = infer_media_extension("", Some(mime));
  let path = workdir.join(format!("upload.{ext}"));
  tokio::fs::write(&path, &bytes)
    .await
    .map_err(|err| input_err(node, format!("cannot stage data URI media: {err}")))?;
  Ok(path)
}

fn resolve_readable_path(
  node: &str,
  path: &Path,
  policy: &SandboxPolicy,
) -> Result<PathBuf, YanshiError> {
  if path.components().any(|c| matches!(c, Component::ParentDir)) {
    return Err(input_err(
      node,
      format!(
        "refusing path '{}': parent-directory components ('..') are not allowed",
        path.display()
      ),
    ));
  }
  let canonical = run_context::resolve_confined_path(path)
    .map_err(|msg| input_err(node, msg))?
    .canonicalize()
    .map_err(|err| input_err(node, format!("cannot resolve '{}': {err}", path.display())))?;

  if let Some(root) = run_context::current_artifact_root() {
    let root_canon = root.canonicalize().unwrap_or(root);
    if canonical.starts_with(&root_canon) {
      return Ok(canonical);
    }
  }

  if let Some(reason) = policy.path_denial_reason(&canonical) {
    return Err(input_err(
      node,
      format!(
        "local path '{}' denied by sandbox policy ({reason})",
        path.display()
      ),
    ));
  }
  Ok(canonical)
}

fn input_err(node: &str, message: impl std::fmt::Display) -> YanshiError {
  YanshiError::NodeInputError {
    message: format!("{node}: {message}"),
  }
}

fn optional_str<'a>(inputs: &'a AsyncNodeInputs, key: &str) -> Option<&'a str> {
  match inputs.get(key) {
    Some(FlowValue::Json(Value::String(s))) if !s.trim().is_empty() => Some(s.trim()),
    _ => None,
  }
}

fn optional_bool(inputs: &AsyncNodeInputs, key: &str) -> Option<bool> {
  match inputs.get(key) {
    Some(FlowValue::Json(Value::Bool(b))) => Some(*b),
    Some(FlowValue::Json(Value::String(s))) => match s.trim().to_ascii_lowercase().as_str() {
      "true" | "1" | "yes" => Some(true),
      "false" | "0" | "no" => Some(false),
      _ => None,
    },
    _ => None,
  }
}

fn optional_u64(inputs: &AsyncNodeInputs, key: &str) -> Option<u64> {
  match inputs.get(key) {
    Some(FlowValue::Json(Value::Number(n))) => n.as_u64(),
    Some(FlowValue::Json(Value::String(s))) => s.trim().parse::<u64>().ok(),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
  };

  #[tokio::test]
  async fn extracts_16khz_mp3_and_embedded_subtitles_from_video() {
    let Some(ffmpeg) = sandboxed::find_tool("ffmpeg") else {
      return;
    };

    let dir = tempfile::tempdir().unwrap();
    let plain_mp4 = dir.path().join("lecture.mp4");
    let gen_status = std::process::Command::new(&ffmpeg)
      .args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "color=c=navy:s=128x72:d=1.2:r=10",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=1.2",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        plain_mp4.to_str().unwrap(),
      ])
      .status()
      .unwrap();
    assert!(
      gen_status.success(),
      "synthetic mp4 generation must succeed"
    );

    // 1) Serve `lecture.mp4` over HTTP and verify `video_ingest` extracts MP3 audio.
    let video_bytes = std::fs::read(&plain_mp4).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
      let (mut stream, _) = listener.accept().await.unwrap();
      let mut req = [0_u8; 2048];
      let _ = stream.read(&mut req).await.unwrap();
      let hdr = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        video_bytes.len()
      );
      stream.write_all(hdr.as_bytes()).await.unwrap();
      stream.write_all(&video_bytes).await.unwrap();
    });

    let policy = Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      allowed_paths: vec![dir.path().to_path_buf()],
      ..SandboxPolicy::default()
    });
    let node = VideoIngestNode::new("ingest", policy).with_no_proxy();
    let mut inputs = HashMap::new();
    inputs.insert(
      "source".to_string(),
      FlowValue::Json(json!(format!("http://{addr}/lecture.mp4"))),
    );

    let out = node.execute(&inputs).await.unwrap();
    server.await.unwrap();

    assert_eq!(out["has_subtitles"], FlowValue::Json(json!(false)));
    let FlowValue::File {
      ref path,
      ref mime_type,
    } = out["media_or_subtitles"]
    else {
      panic!("expected media_or_subtitles File");
    };
    assert_eq!(mime_type.as_deref(), Some("audio/mpeg"));
    assert!(std::fs::metadata(path).unwrap().len() > 200);

    // 2) Mux an embedded subtitle track into `subtitled.mkv` and verify native subtitle extraction.
    let srt_src = dir.path().join("track.srt");
    std::fs::write(
      &srt_src,
      "1\n00:00:00,000 --> 00:00:01,000\nKey architecture insight\n",
    )
    .unwrap();
    let subtitled_mkv = dir.path().join("subtitled.mkv");
    let mux_status = std::process::Command::new(&ffmpeg)
      .args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-i",
        plain_mp4.to_str().unwrap(),
        "-i",
        srt_src.to_str().unwrap(),
        "-c",
        "copy",
        subtitled_mkv.to_str().unwrap(),
      ])
      .status()
      .unwrap();
    assert!(mux_status.success(), "mkv subtitle mux must succeed");

    let mut sub_inputs = HashMap::new();
    sub_inputs.insert(
      "source".to_string(),
      FlowValue::Json(json!(subtitled_mkv.to_string_lossy())),
    );
    let sub_out = node.execute(&sub_inputs).await.unwrap();
    assert_eq!(sub_out["has_subtitles"], FlowValue::Json(json!(true)));
    let FlowValue::Json(Value::String(ref srt_val)) = sub_out["subtitles_srt"] else {
      panic!("expected subtitles_srt string");
    };
    assert!(srt_val.contains("Key architecture insight"));
  }
}
