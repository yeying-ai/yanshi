//! Source ingestion, format detection, extraction (HTML, PDF, OCR, DOCX,
//! PPTX, XLSX, CSV/TSV, JSON, SRT/VTT, GitHub snapshots), ASR transcription,
//! and paragraph chunking.

use std::path::Path as FsPath;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Uploaded / pasted sources larger than this are refused at submit.
pub const MAX_SOURCE_BYTES: usize = 10 * 1024 * 1024;
/// Uncompressed XML read ceiling when unpacking `.docx` / `.xlsx` / `.pptx` archives (zip-bomb guard).
pub const MAX_OOXML_XML_BYTES: usize = 16 * 1024 * 1024;
/// Paragraph chunking target size, in characters.
pub const CHUNK_SIZE: usize = 1200;
/// Paragraph chunking overlap, in characters.
pub const CHUNK_OVERLAP: usize = 150;

/// Request payload for `POST /v1/notebooks/{id}/sources`.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddSourceRequest {
  Url {
    url: String,
    #[serde(default)]
    title: Option<String>,
  },
  Text {
    text: String,
    #[serde(default)]
    title: Option<String>,
  },
  /// An uploaded document (`.txt`, `.md`, `.html`, `.pdf`, `.docx`, `.xlsx`,
  /// `.pptx`, `.csv`, `.tsv`, `.json`, `.srt`, `.vtt`, or image), base64-encoded.
  File {
    filename: String,
    content_base64: String,
    #[serde(default)]
    title: Option<String>,
  },
  /// An uploaded audio recording (`.mp3`, `.wav`, `.m4a`, `.ogg`, `.flac`,
  /// `.webm`, `.aac`), base64-encoded and transcribed via ASR.
  Recording {
    #[serde(default)]
    filename: String,
    #[serde(default)]
    content_base64: String,
    #[serde(default)]
    title: Option<String>,
    /// Registry name of the ASR model; falls back to `YANSHI_ASR_MODEL`.
    #[serde(default)]
    model: Option<String>,
    /// Optional BCP-47 / ISO-639 language hint passed to the ASR provider.
    #[serde(default)]
    language: Option<String>,
  },
}

/// Detected source payload format used to dispatch extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BytesFormat {
  PlainText,
  Html,
  Pdf,
  Image,
  Docx,
  Xlsx,
  Pptx,
  Csv,
  Tsv,
  Json,
  Subtitle,
}

/// Smart URL routing classification for paper and code-hosting URLs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartUrlTarget {
  Direct,
  RewriteUrl(String),
  GitHubRepo {
    owner: String,
    repo: String,
    git_ref: Option<String>,
    subpath: Option<String>,
  },
}

/// Extract text from raw source bytes according to `format`.
pub async fn extract_bytes_text(bytes: Vec<u8>, format: BytesFormat) -> Result<String, String> {
  match format {
    BytesFormat::Html => html_text(&bytes).await,
    BytesFormat::Pdf => pdf_text(&bytes).await,
    BytesFormat::Image => image_ocr_text(&bytes).await,
    BytesFormat::Docx => docx_text(&bytes),
    BytesFormat::Xlsx => xlsx_text(&bytes),
    BytesFormat::Pptx => pptx_text(&bytes),
    BytesFormat::Csv => delimited_text(&bytes, b','),
    BytesFormat::Tsv => delimited_text(&bytes, b'\t'),
    BytesFormat::Json => json_text(&bytes),
    BytesFormat::Subtitle => subtitle_text(&bytes),
    BytesFormat::PlainText => {
      String::from_utf8(bytes).map_err(|_| "the source is not UTF-8 text".to_string())
    }
  }
}

/// Parse SubRip (`.srt`) or WebVTT (`.vtt`) subtitles into timestamped cue paragraphs.
pub fn subtitle_text(bytes: &[u8]) -> Result<String, String> {
  let raw = std::str::from_utf8(bytes)
    .map_err(|_| "subtitle file is not UTF-8 text".to_string())?
    .trim_start_matches('\u{feff}');
  let mut cues: Vec<String> = Vec::new();
  let mut cur_ts: Option<String> = None;
  let mut cur_lines: Vec<String> = Vec::new();

  let flush_cue = |ts: &mut Option<String>, lines: &mut Vec<String>, out: &mut Vec<String>| {
    if !lines.is_empty() {
      let body = lines.join(" ");
      if let Some(stamp) = ts.take() {
        out.push(format!("[{stamp}] {body}"));
      } else {
        out.push(body);
      }
      lines.clear();
    } else {
      *ts = None;
    }
  };

  for line in raw.lines() {
    let trimmed = line.trim();
    if trimmed.is_empty() {
      flush_cue(&mut cur_ts, &mut cur_lines, &mut cues);
      continue;
    }
    if trimmed.eq_ignore_ascii_case("WEBVTT")
      || trimmed.starts_with("NOTE")
      || trimmed.starts_with("STYLE")
    {
      continue;
    }
    if cur_ts.is_none() && trimmed.chars().all(|c| c.is_ascii_digit()) {
      continue;
    }
    if let Some((start, rest)) = trimmed.split_once("-->") {
      flush_cue(&mut cur_ts, &mut cur_lines, &mut cues);
      let end = rest.split_whitespace().next().unwrap_or(rest);
      cur_ts = Some(format!(
        "{} -> {}",
        start.trim().replace(',', "."),
        end.trim().replace(',', ".")
      ));
      continue;
    }
    let cleaned = trimmed
      .replace("<i>", "")
      .replace("</i>", "")
      .replace("<b>", "")
      .replace("</b>", "");
    if !cleaned.trim().is_empty() {
      cur_lines.push(cleaned.trim().to_string());
    }
  }
  flush_cue(&mut cur_ts, &mut cur_lines, &mut cues);

  if cues.is_empty() {
    return Ok(raw.trim().to_string());
  }
  Ok(cues.join("\n\n"))
}

/// Detect OOXML ZIP formats (`.docx`, `.xlsx`, `.pptx`) by header + archive layout.
pub fn sniff_ooxml_zip_format(bytes: &[u8], ext: Option<&str>) -> Option<BytesFormat> {
  if !bytes.starts_with(b"PK\x03\x04") {
    return None;
  }
  match ext {
    Some("docx") => return Some(BytesFormat::Docx),
    Some("xlsx") => return Some(BytesFormat::Xlsx),
    Some("pptx") => return Some(BytesFormat::Pptx),
    _ => {}
  }
  let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
  for name in archive.file_names() {
    let Ok(name) = name else {
      continue;
    };
    let name: &str = name.as_ref();
    if name == "word/document.xml" {
      return Some(BytesFormat::Docx);
    }
    if name == "xl/workbook.xml" || name.starts_with("xl/worksheets/sheet") {
      return Some(BytesFormat::Xlsx);
    }
    if name == "ppt/presentation.xml" || name.starts_with("ppt/slides/slide") {
      return Some(BytesFormat::Pptx);
    }
  }
  None
}

/// Extract a lowercase file extension from a URL path, if present.
pub fn url_path_extension(raw_url: &str) -> Option<String> {
  let path = url::Url::parse(raw_url)
    .ok()
    .map(|u| u.path().to_string())
    .unwrap_or_else(|| {
      raw_url
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_string()
    });
  FsPath::new(&path)
    .extension()
    .and_then(|e| e.to_str())
    .map(str::to_ascii_lowercase)
}

/// Percent-encode a single URI path/query component.
pub fn percent_encode_component(raw: &str) -> String {
  let mut out = String::with_capacity(raw.len());
  for &b in raw.as_bytes() {
    if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
      out.push(b as char);
    } else {
      out.push_str(&format!("%{b:02X}"));
    }
  }
  out
}

/// Percent-encode slash-delimited GitHub path segments while preserving `/`.
pub fn encode_github_path(raw: &str) -> String {
  raw
    .split('/')
    .filter(|seg| !seg.is_empty())
    .map(percent_encode_component)
    .collect::<Vec<_>>()
    .join("/")
}

/// Classify well-known academic/code URLs (`arxiv.org`, `openreview.net`, `github.com`).
pub fn classify_smart_source_url(raw_url: &str) -> SmartUrlTarget {
  let Ok(parsed) = url::Url::parse(raw_url.trim()) else {
    return SmartUrlTarget::Direct;
  };
  let Some(host) = parsed.host_str().map(str::to_ascii_lowercase) else {
    return SmartUrlTarget::Direct;
  };
  let segments: Vec<&str> = parsed
    .path_segments()
    .map(|it| it.filter(|s| !s.is_empty()).collect())
    .unwrap_or_default();

  if matches!(
    host.as_str(),
    "arxiv.org" | "www.arxiv.org" | "export.arxiv.org"
  ) && segments.len() >= 2
    && matches!(segments[0], "abs" | "e-print" | "html")
  {
    let paper_id = segments[1..].join("/");
    let paper_id = paper_id.strip_suffix(".pdf").unwrap_or(&paper_id);
    if !paper_id.is_empty() {
      return SmartUrlTarget::RewriteUrl(format!("https://arxiv.org/pdf/{paper_id}.pdf"));
    }
  }

  if matches!(host.as_str(), "openreview.net" | "www.openreview.net")
    && segments.first().copied() == Some("forum")
    && let Some((_, forum_id)) = parsed
      .query_pairs()
      .find(|(k, v)| k == "id" && !v.is_empty())
  {
    return SmartUrlTarget::RewriteUrl(format!(
      "https://openreview.net/pdf?id={}",
      percent_encode_component(&forum_id)
    ));
  }

  if matches!(host.as_str(), "github.com" | "www.github.com") && segments.len() >= 2 {
    let owner = segments[0];
    let repo = segments[1].strip_suffix(".git").unwrap_or(segments[1]);
    if !owner.is_empty() && !repo.is_empty() {
      if segments.len() == 2 {
        return SmartUrlTarget::GitHubRepo {
          owner: owner.to_string(),
          repo: repo.to_string(),
          git_ref: None,
          subpath: None,
        };
      }
      if segments[2] == "tree" && segments.len() >= 4 {
        let git_ref = segments[3].to_string();
        let subpath = (segments.len() > 4).then(|| segments[4..].join("/"));
        return SmartUrlTarget::GitHubRepo {
          owner: owner.to_string(),
          repo: repo.to_string(),
          git_ref: Some(git_ref),
          subpath,
        };
      }
      if segments[2] == "blob" && segments.len() >= 5 {
        let enc_owner = percent_encode_component(owner);
        let enc_repo = percent_encode_component(repo);
        let enc_tail = encode_github_path(&segments[3..].join("/"));
        return SmartUrlTarget::RewriteUrl(format!(
          "https://raw.githubusercontent.com/{enc_owner}/{enc_repo}/{enc_tail}"
        ));
      }
    }
  }

  SmartUrlTarget::Direct
}

/// Rank and select up to `max_files` high-signal source paths from a GitHub repo tree.
pub fn select_github_snapshot_paths(
  tree_paths: &[String],
  subpath: Option<&str>,
  max_files: usize,
) -> Vec<String> {
  const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "vendor",
    "__pycache__",
    ".next",
    "coverage",
  ];
  const SKIP_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "pdf", "zip", "tar", "gz", "xz", "woff", "woff2",
    "ttf", "otf", "mp3", "mp4", "mov", "wav", "wasm", "pyc", "so", "dylib", "dll", "exe", "bin",
    "lock",
  ];
  const SKIP_NAMES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "poetry.lock",
    "uv.lock",
    "go.sum",
  ];

  let prefix = subpath
    .map(|s| s.trim_matches('/'))
    .filter(|s| !s.is_empty());

  let mut scored: Vec<(u8, usize, String)> = Vec::new();
  for path in tree_paths {
    if let Some(pfx) = prefix
      && path != pfx
      && !path.starts_with(&format!("{pfx}/"))
    {
      continue;
    }
    if path
      .split('/')
      .any(|seg| SKIP_DIRS.contains(&seg) || seg.starts_with('.'))
    {
      continue;
    }
    let file_name = path.rsplit('/').next().unwrap_or(path.as_str());
    if SKIP_NAMES.contains(&file_name) {
      continue;
    }
    let ext = FsPath::new(file_name)
      .extension()
      .and_then(|e| e.to_str())
      .map(str::to_ascii_lowercase);
    if let Some(ref e) = ext
      && SKIP_EXTS.contains(&e.as_str())
    {
      continue;
    }
    let lower_name = file_name.to_ascii_lowercase();
    let priority = if lower_name.starts_with("readme") {
      0
    } else if matches!(
      file_name,
      "Cargo.toml" | "package.json" | "pyproject.toml" | "go.mod" | "pom.xml"
    ) {
      1
    } else if matches!(
      lower_name.as_str(),
      "lib.rs"
        | "main.rs"
        | "mod.rs"
        | "index.ts"
        | "index.tsx"
        | "app.tsx"
        | "main.py"
        | "main.go"
    ) {
      2
    } else if matches!(ext.as_deref(), Some("md" | "toml" | "yaml" | "yml")) {
      3
    } else {
      4
    };
    let depth = path.matches('/').count();
    scored.push((priority, depth, path.clone()));
  }
  scored.sort_by(|a, b| {
    a.0
      .cmp(&b.0)
      .then_with(|| a.1.cmp(&b.1))
      .then_with(|| a.2.cmp(&b.2))
  });
  scored
    .into_iter()
    .take(max_files)
    .map(|(_, _, p)| p)
    .collect()
}

/// Fetch repository tree metadata and top priority files from GitHub into a Markdown snapshot.
pub async fn ingest_github_repo_snapshot(
  owner: &str,
  repo: &str,
  git_ref: Option<&str>,
  subpath: Option<&str>,
) -> Result<(String, String), String> {
  const MAX_SNAPSHOT_FILES: usize = 16;
  const MAX_SNAPSHOT_TOTAL_CHARS: usize = 96_000;
  const MAX_SINGLE_FILE_CHARS: usize = 10_000;

  let tool = yanshi_tools::builtin::HttpTool::new(Arc::new(yanshi_tools::SandboxPolicy::default()))
    .map_err(|e| format!("http client: {e}"))?;
  let gh_headers = json!({
    "Accept": "application/vnd.github+json",
    "User-Agent": "yanshi-notebook/0.2",
  });

  let enc_owner = percent_encode_component(owner);
  let enc_repo = percent_encode_component(repo);
  let repo_api = format!("https://api.github.com/repos/{enc_owner}/{enc_repo}");
  let meta_res = tool
    .send_bytes(
      &json!({ "url": repo_api, "method": "GET", "headers": gh_headers }),
      Some(512 * 1024),
    )
    .await
    .map_err(|e| format!("fetching GitHub repo metadata failed: {e}"))?;
  if !meta_res.status.is_success() {
    return Err(format!(
      "GitHub repo metadata returned HTTP {}",
      meta_res.status
    ));
  }
  let meta_json: Value =
    serde_json::from_slice(&meta_res.body).map_err(|e| format!("invalid GitHub repo JSON: {e}"))?;
  let mut resolved_ref = git_ref
    .filter(|r| !r.is_empty())
    .or_else(|| meta_json.get("default_branch").and_then(Value::as_str))
    .unwrap_or("main")
    .to_string();
  let mut resolved_subpath = subpath.map(str::to_string);
  let description = meta_json
    .get("description")
    .and_then(Value::as_str)
    .unwrap_or_default()
    .trim();

  let fetch_tree = |ref_name: &str| {
    let enc_ref = percent_encode_component(ref_name);
    let url = format!(
      "https://api.github.com/repos/{enc_owner}/{enc_repo}/git/trees/{enc_ref}?recursive=1"
    );
    let tool = &tool;
    let gh_headers = &gh_headers;
    async move {
      tool
        .send_bytes(
          &json!({ "url": url, "method": "GET", "headers": gh_headers }),
          Some(4 * 1024 * 1024),
        )
        .await
    }
  };

  let mut tree_res = fetch_tree(&resolved_ref)
    .await
    .map_err(|e| format!("fetching GitHub tree failed: {e}"))?;

  // Resolve slash-bearing branch/tag names (e.g. `/tree/feat/studio/crates`) when
  // the single-segment ref lookup returns 404.
  if tree_res.status.as_u16() == 404
    && let (Some(first_seg), Some(rest_sub)) = (git_ref, subpath)
  {
    let full_tail = format!("{first_seg}/{rest_sub}");
    let enc_prefix = percent_encode_component(first_seg);
    let mut candidate_refs: Vec<String> = Vec::new();
    for ns in ["heads", "tags"] {
      let refs_url = format!(
        "https://api.github.com/repos/{enc_owner}/{enc_repo}/git/matching-refs/{ns}/{enc_prefix}"
      );
      if let Ok(res) = tool
        .send_bytes(
          &json!({ "url": refs_url, "method": "GET", "headers": gh_headers }),
          Some(512 * 1024),
        )
        .await
        && res.status.is_success()
        && let Ok(Value::Array(items)) = serde_json::from_slice::<Value>(&res.body)
      {
        let strip_pfx = format!("refs/{ns}/");
        for item in items {
          if let Some(r) = item
            .get("ref")
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix(&strip_pfx))
          {
            candidate_refs.push(r.to_string());
          }
        }
      }
    }
    candidate_refs.sort_by_key(|r| std::cmp::Reverse(r.len()));
    for cand in candidate_refs {
      if full_tail == cand {
        resolved_ref = cand;
        resolved_subpath = None;
        tree_res = fetch_tree(&resolved_ref)
          .await
          .map_err(|e| format!("fetching GitHub tree failed: {e}"))?;
        break;
      } else if let Some(rem) = full_tail.strip_prefix(&format!("{cand}/")) {
        resolved_ref = cand;
        resolved_subpath = (!rem.is_empty()).then(|| rem.to_string());
        tree_res = fetch_tree(&resolved_ref)
          .await
          .map_err(|e| format!("fetching GitHub tree failed: {e}"))?;
        break;
      }
    }
  }

  if !tree_res.status.is_success() {
    return Err(format!("GitHub tree returned HTTP {}", tree_res.status));
  }
  let tree_json: Value =
    serde_json::from_slice(&tree_res.body).map_err(|e| format!("invalid GitHub tree JSON: {e}"))?;
  let blob_paths: Vec<String> = tree_json
    .get("tree")
    .and_then(Value::as_array)
    .map(|items| {
      items
        .iter()
        .filter(|entry| entry.get("type").and_then(Value::as_str) == Some("blob"))
        .filter_map(|entry| {
          entry
            .get("path")
            .and_then(Value::as_str)
            .map(str::to_string)
        })
        .collect()
    })
    .unwrap_or_default();

  let chosen =
    select_github_snapshot_paths(&blob_paths, resolved_subpath.as_deref(), MAX_SNAPSHOT_FILES);
  let mut md = format!("# GitHub Repository: {owner}/{repo} (`{resolved_ref}`)\n\n");
  if !description.is_empty() {
    md.push_str(&format!("{description}\n\n"));
  }
  if let Some(sub) = resolved_subpath.as_deref().filter(|s| !s.is_empty()) {
    md.push_str(&format!("Subpath filter: `{sub}`\n\n"));
  }
  md.push_str("## Repository File Tree\n\n```text\n");
  for path in blob_paths.iter().take(250) {
    md.push_str(path);
    md.push('\n');
  }
  if blob_paths.len() > 250 {
    md.push_str(&format!("... ({} more files)\n", blob_paths.len() - 250));
  }
  md.push_str("```\n\n");

  let enc_resolved_ref = encode_github_path(&resolved_ref);
  let mut used_chars = 0usize;
  for rel_path in chosen {
    if used_chars >= MAX_SNAPSHOT_TOTAL_CHARS {
      break;
    }
    let enc_rel = encode_github_path(&rel_path);
    let raw_url = format!(
      "https://raw.githubusercontent.com/{enc_owner}/{enc_repo}/{enc_resolved_ref}/{enc_rel}"
    );
    let Ok(raw_res) = tool
      .send_bytes(
        &json!({ "url": raw_url, "method": "GET" }),
        Some(256 * 1024),
      )
      .await
    else {
      continue;
    };
    if !raw_res.status.is_success() {
      continue;
    }
    let Ok(content) = String::from_utf8(raw_res.body) else {
      continue;
    };
    let trimmed: String = content.chars().take(MAX_SINGLE_FILE_CHARS).collect();
    used_chars += trimmed.len();
    md.push_str(&format!("## File: `{rel_path}`\n\n```\n{trimmed}\n```\n\n"));
  }

  let digest = format!("{:x}", Sha256::digest(md.as_bytes()));
  Ok((digest, md))
}

/// Ingest a URL using smart-URL rewrites, SSRF-checked fetch, extraction, and
/// optional DeepSeek fetch fallback. Returns `(sha256_hex, extracted_text)`.
pub async fn ingest_url(url: &str) -> Result<(String, String), String> {
  match classify_smart_source_url(url) {
    SmartUrlTarget::GitHubRepo {
      owner,
      repo,
      git_ref,
      subpath,
    } => {
      match ingest_github_repo_snapshot(&owner, &repo, git_ref.as_deref(), subpath.as_deref()).await
      {
        Ok(snapshot) => return Ok(snapshot),
        // Explicit `/tree/<ref>` URLs must surface the 404/API error rather than
        // silently scraping GitHub's HTML navigation chrome.
        Err(err) if git_ref.is_some() => return Err(err),
        Err(_) => {}
      }
    }
    SmartUrlTarget::RewriteUrl(rewritten) => {
      let is_github_raw = rewritten.starts_with("https://raw.githubusercontent.com/");
      match fetch_source_url(&rewritten).await {
        Ok((bytes, format)) => {
          let text = extract_bytes_text(bytes.clone(), format).await?;
          if !text.trim().is_empty() {
            let digest = format!("{:x}", Sha256::digest(&bytes));
            return Ok((digest, text));
          }
        }
        Err(err) if is_github_raw => return Err(err),
        Err(_) => {}
      }
    }
    SmartUrlTarget::Direct => {}
  }

  let (bytes, format) = match fetch_source_url(url).await {
    Ok(pair) => pair,
    Err(err) => {
      if err.contains("returned HTTP ")
        && let Some(fetched) = deepseek_fetch_fallback(url).await
      {
        return Ok(fetched);
      }
      return Err(err);
    }
  };
  let digest = format!("{:x}", Sha256::digest(&bytes));
  let text = extract_bytes_text(bytes, format).await?;
  if format == BytesFormat::Html
    && text.trim().is_empty()
    && let Some(fetched) = deepseek_fetch_fallback(url).await
  {
    return Ok(fetched);
  }
  Ok((digest, text))
}

/// Sniff PNG, JPEG, GIF, or WebP magic bytes and return a canonical filename.
pub fn sniff_image_canonical_name(bytes: &[u8]) -> Option<&'static str> {
  if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
    Some("image.png")
  } else if bytes.starts_with(b"\xff\xd8\xff") {
    Some("image.jpg")
  } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
    Some("image.gif")
  } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
    Some("image.webp")
  } else {
    None
  }
}

/// Fallback webpage fetch via DeepSeek `/v1/tools/fetch` when `DEEPSEEK_API_KEY` is configured.
pub async fn deepseek_fetch_fallback(url: &str) -> Option<(String, String)> {
  let client = yanshi_tools::builtin::DeepSeekToolsClient::from_env()?;
  let fetched = client.fetch_markdown(url).await.ok()?;
  let content = fetched.content.trim();
  if content.is_empty() {
    return None;
  }
  let combined = if fetched.title.trim().is_empty() {
    content.to_string()
  } else {
    format!("# {}\n\n{content}", fetched.title.trim())
  };
  let digest = format!("{:x}", Sha256::digest(combined.as_bytes()));
  Some((digest, combined))
}

/// Extract layout-aware text from PNG/JPEG/GIF/WebP bytes via DeepSeek OCR.
pub async fn image_ocr_text(bytes: &[u8]) -> Result<String, String> {
  let filename = sniff_image_canonical_name(bytes)
    .ok_or_else(|| "unsupported image format (expected PNG, JPEG, GIF or WebP)".to_string())?;
  let client = yanshi_tools::builtin::DeepSeekToolsClient::from_env().ok_or_else(|| {
    "image OCR requires DEEPSEEK_API_KEY to be configured (/v1/tools/ocr)".to_string()
  })?;
  let resp = client
    .ocr_bytes(
      bytes,
      filename,
      yanshi_tools::builtin::DeepSeekOcrPurpose::RichLayoutRecognition,
    )
    .await
    .map_err(|e| format!("extracting image text via DeepSeek OCR: {e}"))?;
  Ok(resp.formatted_text(None))
}

/// GET `url` through the HTTP tool's default SSRF policy, which refuses
/// loopback, private, and cloud-metadata addresses.
pub async fn fetch_source_url(url: &str) -> Result<(Vec<u8>, BytesFormat), String> {
  let tool = yanshi_tools::builtin::HttpTool::new(Arc::new(yanshi_tools::SandboxPolicy::default()))
    .map_err(|e| format!("http client: {e}"))?;
  let response = tool
    .send_bytes(
      &json!({ "url": url, "method": "GET" }),
      Some(MAX_SOURCE_BYTES),
    )
    .await
    .map_err(|e| format!("fetching {url} failed: {e}"))?;
  if !response.status.is_success() {
    return Err(format!("fetching {url} returned HTTP {}", response.status));
  }
  let content_type = response
    .content_type
    .as_deref()
    .and_then(|ct| ct.split(';').next())
    .map(|ct| ct.trim().to_ascii_lowercase());
  if let Some(ref ct) = content_type
    && (ct.starts_with("audio/") || ct.starts_with("video/"))
  {
    return Err(format!(
      "URL returned binary media Content-Type '{ct}'; upload audio via `kind: recording` or run `video_summarizer` in Studio"
    ));
  }
  let bytes = response.body;
  let ext = url_path_extension(url);
  if sniff_image_canonical_name(&bytes).is_some() {
    return Ok((bytes, BytesFormat::Image));
  }
  if let Some(ref ct) = content_type
    && ct.starts_with("image/")
    && ct != "image/svg+xml"
  {
    return Err(format!(
      "URL returned unsupported image Content-Type '{ct}' (expected PNG, JPEG, GIF or WebP)"
    ));
  }
  let format = if bytes.starts_with(b"%PDF-") || content_type.as_deref() == Some("application/pdf")
  {
    BytesFormat::Pdf
  } else if let Some(ooxml) = sniff_ooxml_zip_format(&bytes, ext.as_deref()) {
    ooxml
  } else {
    let sample = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
    let lowered = sample.trim_start().to_ascii_lowercase();
    if lowered.starts_with("<!doctype html")
      || lowered.contains("<html")
      || matches!(
        content_type.as_deref(),
        Some("text/html" | "application/xhtml+xml")
      )
    {
      BytesFormat::Html
    } else if ext.as_deref() == Some("csv") || content_type.as_deref() == Some("text/csv") {
      BytesFormat::Csv
    } else if ext.as_deref() == Some("tsv")
      || content_type.as_deref() == Some("text/tab-separated-values")
    {
      BytesFormat::Tsv
    } else if matches!(ext.as_deref(), Some("srt" | "vtt"))
      || lowered.starts_with("webvtt")
      || matches!(
        content_type.as_deref(),
        Some("text/vtt" | "application/x-subrip")
      )
    {
      BytesFormat::Subtitle
    } else if ext.as_deref() == Some("json")
      || content_type
        .as_deref()
        .is_some_and(|ct| ct == "application/json" || ct.ends_with("+json"))
    {
      BytesFormat::Json
    } else {
      BytesFormat::PlainText
    }
  };
  Ok((bytes, format))
}

/// Visible text of an HTML document (scripts and styles dropped).
pub async fn html_text(bytes: &[u8]) -> Result<String, String> {
  use yanshi_rag::sources::{DocumentLoader, html::HtmlLoader};
  let path = std::env::temp_dir().join(format!("yanshi-ingest-{}.html", Uuid::new_v4()));
  tokio::fs::write(&path, bytes)
    .await
    .map_err(|e| format!("staging the page: {e}"))?;
  let loaded = HtmlLoader::new().load(&path).await;
  let _ = tokio::fs::remove_file(&path).await;
  loaded
    .map(|document| document.content)
    .map_err(|e| format!("extracting text: {e}"))
}

/// Text of a PDF document: pure-Rust `PdfLoader` first, falling back to
/// `pdftotext` when installed, and finally DeepSeek OCR when configured.
pub async fn pdf_text(bytes: &[u8]) -> Result<String, String> {
  use yanshi_rag::sources::{DocumentLoader, pdf::PdfLoader};
  let id = Uuid::new_v4();
  let path = std::env::temp_dir().join(format!("yanshi-ingest-{id}.pdf"));
  tokio::fs::write(&path, bytes)
    .await
    .map_err(|e| format!("staging the PDF: {e}"))?;
  let pure_rust = PdfLoader::new()
    .with_max_bytes(Some(MAX_SOURCE_BYTES as u64))
    .load(&path)
    .await;
  if let Ok(doc) = &pure_rust
    && !doc.content.trim().is_empty()
  {
    let _ = tokio::fs::remove_file(&path).await;
    return Ok(doc.content.clone());
  }
  let fallback = pdftotext_fallback(&path, id).await;
  let _ = tokio::fs::remove_file(&path).await;
  if let Ok(text) = fallback
    && !text.trim().is_empty()
  {
    return Ok(text);
  }
  if let Some(ocr) = yanshi_tools::builtin::DeepSeekToolsClient::from_env()
    && let Ok(resp) = ocr
      .ocr_bytes(
        bytes,
        "document.pdf",
        yanshi_tools::builtin::DeepSeekOcrPurpose::TextRecognition,
      )
      .await
  {
    let ocr_text = resp.formatted_text(None);
    if !ocr_text.trim().is_empty() {
      return Ok(ocr_text);
    }
  }
  pure_rust
    .map(|doc| doc.content)
    .map_err(|e| format!("extracting PDF text: {e}"))
}

async fn pdftotext_fallback(pdf_path: &FsPath, id: Uuid) -> Result<String, String> {
  let out_path = std::env::temp_dir().join(format!("yanshi-ingest-{id}.txt"));
  let run = tokio::time::timeout(
    std::time::Duration::from_secs(60),
    tokio::process::Command::new("pdftotext")
      .args(["-layout", "-enc", "UTF-8"])
      .arg(pdf_path)
      .arg(&out_path)
      .output(),
  )
  .await;
  let result = match run {
    Ok(Ok(output)) if output.status.success() => tokio::fs::read_to_string(&out_path)
      .await
      .map_err(|e| format!("reading pdftotext output: {e}")),
    Ok(Ok(output)) => Err(format!(
      "pdftotext exited with {}: {}",
      output.status,
      String::from_utf8_lossy(&output.stderr)
    )),
    Ok(Err(e)) => Err(format!("spawning pdftotext: {e}")),
    Err(_) => Err("pdftotext timed out after 60s".to_string()),
  };
  let _ = tokio::fs::remove_file(&out_path).await;
  result
}

/// Read an XML entry from a ZIP archive while enforcing an aggregate uncompressed byte budget.
pub fn read_zip_xml_bounded(
  archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
  entry_name: &str,
  budget_remaining: &mut usize,
  label: &str,
) -> Result<Option<String>, String> {
  use std::io::Read as _;
  let mut entry = match archive.by_name(entry_name) {
    Ok(file) => file,
    Err(zip::result::ZipError::FileNotFound) => return Ok(None),
    Err(e) => return Err(format!("reading {entry_name} failed: {e}")),
  };
  let mut xml_bytes = Vec::new();
  entry
    .by_ref()
    .take((*budget_remaining as u64).saturating_add(1))
    .read_to_end(&mut xml_bytes)
    .map_err(|e| format!("decompressing {entry_name} failed: {e}"))?;
  if xml_bytes.len() > *budget_remaining {
    return Err(format!(
      "{label} uncompressed XML exceeds {MAX_OOXML_XML_BYTES} bytes safety limit"
    ));
  }
  *budget_remaining -= xml_bytes.len();
  String::from_utf8(xml_bytes)
    .map(Some)
    .map_err(|_| format!("{entry_name} is not valid UTF-8 XML"))
}

/// Pure-Rust OOXML `.docx` text extraction (`word/document.xml`, plus optional
/// `word/footnotes.xml` and `word/endnotes.xml`), preserving paragraph breaks
/// and table rows (`| cell1 | cell2 |`) while bounding uncompressed XML bytes
/// against zip-bomb payloads.
pub fn docx_text(bytes: &[u8]) -> Result<String, String> {
  let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
    .map_err(|e| format!("invalid DOCX archive: {e}"))?;

  let mut budget_remaining = MAX_OOXML_XML_BYTES;
  let mut sections: Vec<String> = Vec::new();
  for entry_name in [
    "word/document.xml",
    "word/footnotes.xml",
    "word/endnotes.xml",
  ] {
    let Some(xml) = read_zip_xml_bounded(&mut archive, entry_name, &mut budget_remaining, "DOCX")?
    else {
      if entry_name == "word/document.xml" {
        return Err("invalid DOCX archive: missing word/document.xml".to_string());
      }
      continue;
    };
    let extracted = extract_ooxml_rich_text(&xml);
    if !extracted.trim().is_empty() {
      sections.push(extracted);
    }
  }
  Ok(sections.join("\n\n"))
}

/// Pure-Rust OOXML `.pptx` slide & speaker-notes extraction (`ppt/slides/slide*.xml`
/// and optional `ppt/notesSlides/notesSlide*.xml`), ordered numerically by slide
/// index and preserving paragraph breaks and `<a:tbl>` rows.
pub fn pptx_text(bytes: &[u8]) -> Result<String, String> {
  let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
    .map_err(|e| format!("invalid PPTX archive: {e}"))?;

  let mut slide_entries: Vec<(usize, String)> = archive
    .file_names()
    .filter_map(|name| {
      let name = name.ok()?;
      let stem = name
        .as_ref()
        .strip_prefix("ppt/slides/slide")?
        .strip_suffix(".xml")?;
      let num = stem.parse::<usize>().ok()?;
      Some((num, name.into_owned()))
    })
    .collect();
  if slide_entries.is_empty() {
    return Err("invalid PPTX archive: missing ppt/slides/slide*.xml".to_string());
  }
  slide_entries.sort_by_key(|(num, _)| *num);

  let mut budget_remaining = MAX_OOXML_XML_BYTES;
  let mut slides_out: Vec<String> = Vec::new();
  for (slide_num, entry_name) in slide_entries {
    let Some(slide_xml) =
      read_zip_xml_bounded(&mut archive, &entry_name, &mut budget_remaining, "PPTX")?
    else {
      continue;
    };
    let body = extract_ooxml_rich_text(&slide_xml);
    let notes_entry = format!("ppt/notesSlides/notesSlide{slide_num}.xml");
    let notes = read_zip_xml_bounded(&mut archive, &notes_entry, &mut budget_remaining, "PPTX")?
      .map(|xml| extract_ooxml_rich_text(&xml))
      .unwrap_or_default();
    let cleaned_notes: Vec<&str> = notes
      .lines()
      .map(str::trim)
      .filter(|line| !line.is_empty() && line.parse::<usize>().ok() != Some(slide_num))
      .collect();

    if body.trim().is_empty() && cleaned_notes.is_empty() {
      continue;
    }
    let mut section = format!("Slide {slide_num}:");
    if !body.trim().is_empty() {
      section.push('\n');
      section.push_str(body.trim());
    }
    if !cleaned_notes.is_empty() {
      section.push_str("\nSpeaker notes: ");
      section.push_str(&cleaned_notes.join(" "));
    }
    slides_out.push(section);
  }
  Ok(slides_out.join("\n\n"))
}

/// Pure-Rust OOXML `.xlsx` workbook extraction (`xl/sharedStrings.xml`,
/// `xl/workbook.xml`, and `xl/worksheets/sheet*.xml`), formatting rows with
/// sheet and column header labels (`Header: Value | ...`) for BM25 recall.
pub fn xlsx_text(bytes: &[u8]) -> Result<String, String> {
  let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
    .map_err(|e| format!("invalid XLSX archive: {e}"))?;

  let mut budget_remaining = MAX_OOXML_XML_BYTES;
  let shared_strings = match read_zip_xml_bounded(
    &mut archive,
    "xl/sharedStrings.xml",
    &mut budget_remaining,
    "XLSX",
  )? {
    Some(xml) => parse_xlsx_shared_strings(&xml),
    None => Vec::new(),
  };

  let sheet_titles = match read_zip_xml_bounded(
    &mut archive,
    "xl/workbook.xml",
    &mut budget_remaining,
    "XLSX",
  )? {
    Some(xml) => parse_xlsx_sheet_titles(&xml),
    None => Vec::new(),
  };

  let mut sheet_entries: Vec<(usize, String)> = archive
    .file_names()
    .filter_map(|name| {
      let name = name.ok()?;
      let stem = name
        .as_ref()
        .strip_prefix("xl/worksheets/sheet")?
        .strip_suffix(".xml")?;
      let num = stem.parse::<usize>().ok()?;
      Some((num, name.into_owned()))
    })
    .collect();
  if sheet_entries.is_empty() {
    return Err("invalid XLSX archive: missing xl/worksheets/sheet*.xml".to_string());
  }
  sheet_entries.sort_by_key(|(num, _)| *num);

  let mut sections: Vec<String> = Vec::new();
  for (pos, (sheet_num, entry_name)) in sheet_entries.into_iter().enumerate() {
    let Some(sheet_xml) =
      read_zip_xml_bounded(&mut archive, &entry_name, &mut budget_remaining, "XLSX")?
    else {
      continue;
    };
    let rows = parse_xlsx_sheet_rows(&sheet_xml, &shared_strings);
    if rows.is_empty() {
      continue;
    }
    let sheet_title = sheet_titles
      .get(pos)
      .cloned()
      .unwrap_or_else(|| format!("Sheet{sheet_num}"));
    sections.push(format_xlsx_sheet_rows(&sheet_title, &rows));
  }
  Ok(sections.join("\n\n"))
}

fn xml_attr_value(tag_body: &str, attr_name: &str) -> Option<String> {
  let mut rest = tag_body;
  while let Some(eq_idx) = rest.find('=') {
    let before = rest[..eq_idx].trim_end();
    let key = before
      .rsplit_once(char::is_whitespace)
      .map_or(before, |(_, k)| k);
    let after = rest[eq_idx + 1..].trim_start();
    let Some(quote) = after.chars().next().filter(|&c| c == '"' || c == '\'') else {
      rest = after;
      continue;
    };
    let val_rest = &after[quote.len_utf8()..];
    let Some(end_idx) = val_rest.find(quote) else {
      break;
    };
    if key == attr_name {
      return Some(decode_xml_entities(&val_rest[..end_idx]));
    }
    rest = &val_rest[end_idx + quote.len_utf8()..];
  }
  None
}

fn parse_xlsx_shared_strings(xml: &str) -> Vec<String> {
  let mut items = Vec::new();
  let mut current = String::new();
  let mut in_si = false;
  let mut in_t = false;
  let mut phonetic_depth: usize = 0;

  let mut rest = xml;
  while let Some(lt) = rest.find('<') {
    if in_si && in_t && phonetic_depth == 0 && lt > 0 {
      current.push_str(&decode_xml_entities(&rest[..lt]));
    }
    rest = &rest[lt + 1..];
    let Some(gt) = rest.find('>') else {
      break;
    };
    let raw_tag = rest[..gt].trim();
    rest = &rest[gt + 1..];
    if raw_tag.starts_with('?') || raw_tag.starts_with('!') {
      continue;
    }
    let is_closing = raw_tag.starts_with('/');
    let is_self_closing = raw_tag.ends_with('/');
    let tag_body = raw_tag.trim_start_matches('/').trim_end_matches('/').trim();
    let local = tag_body
      .split_whitespace()
      .next()
      .unwrap_or_default()
      .rsplit_once(':')
      .map_or_else(
        || tag_body.split_whitespace().next().unwrap_or_default(),
        |(_, l)| l,
      );
    match (local, is_closing, is_self_closing) {
      ("si", false, false) => {
        in_si = true;
        current.clear();
      }
      ("si", false, true) => {
        items.push(String::new());
      }
      ("si", true, _) => {
        if in_si {
          items.push(current.clone());
        }
        in_si = false;
        in_t = false;
        current.clear();
      }
      ("rPh", false, false) => {
        phonetic_depth += 1;
      }
      ("rPh", true, _) => {
        phonetic_depth = phonetic_depth.saturating_sub(1);
      }
      ("t", false, false) if in_si => {
        in_t = true;
      }
      ("t", true, _) => {
        in_t = false;
      }
      _ => {}
    }
  }
  items
}

fn parse_xlsx_sheet_titles(xml: &str) -> Vec<String> {
  let mut titles = Vec::new();
  let mut rest = xml;
  while let Some(lt) = rest.find('<') {
    rest = &rest[lt + 1..];
    let Some(gt) = rest.find('>') else {
      break;
    };
    let raw_tag = rest[..gt].trim();
    rest = &rest[gt + 1..];
    if raw_tag.starts_with('/') || raw_tag.starts_with('?') || raw_tag.starts_with('!') {
      continue;
    }
    let tag_body = raw_tag.trim_end_matches('/').trim();
    let tag_name = tag_body.split_whitespace().next().unwrap_or_default();
    let local = tag_name.rsplit_once(':').map_or(tag_name, |(_, l)| l);
    if local == "sheet"
      && let Some(name) = xml_attr_value(tag_body, "name")
      && !name.trim().is_empty()
    {
      titles.push(name.trim().to_string());
    }
  }
  titles
}

fn xlsx_col_index_from_ref(cell_ref: &str) -> Option<usize> {
  let letters: String = cell_ref
    .chars()
    .take_while(char::is_ascii_alphabetic)
    .collect();
  if letters.is_empty() {
    return None;
  }
  let mut idx: usize = 0;
  for ch in letters.chars() {
    let digit = (ch.to_ascii_uppercase() as u8).saturating_sub(b'A') as usize + 1;
    idx = idx.saturating_mul(26).saturating_add(digit);
  }
  idx.checked_sub(1)
}

fn xlsx_col_letters(mut col_idx: usize) -> String {
  let mut chars = Vec::new();
  loop {
    chars.push((b'A' + (col_idx % 26) as u8) as char);
    if col_idx < 26 {
      break;
    }
    col_idx = col_idx / 26 - 1;
  }
  chars.into_iter().rev().collect()
}

fn parse_xlsx_sheet_rows(xml: &str, shared_strings: &[String]) -> Vec<Vec<(usize, String)>> {
  let mut rows: Vec<Vec<(usize, String)>> = Vec::new();
  let mut current_row: Vec<(usize, String)> = Vec::new();
  let mut in_row = false;
  let mut in_cell = false;
  let mut next_col_idx: usize = 0;
  let mut cell_col_idx: usize = 0;
  let mut cell_type = String::new();
  let mut in_v = false;
  let mut in_is_t = false;
  let mut cell_v = String::new();
  let mut cell_inline = String::new();

  let mut rest = xml;
  while let Some(lt) = rest.find('<') {
    if in_cell && lt > 0 {
      if in_v {
        cell_v.push_str(&decode_xml_entities(&rest[..lt]));
      } else if in_is_t {
        cell_inline.push_str(&decode_xml_entities(&rest[..lt]));
      }
    }
    rest = &rest[lt + 1..];
    let Some(gt) = rest.find('>') else {
      break;
    };
    let raw_tag = rest[..gt].trim();
    rest = &rest[gt + 1..];
    if raw_tag.starts_with('?') || raw_tag.starts_with('!') {
      continue;
    }
    let is_closing = raw_tag.starts_with('/');
    let is_self_closing = raw_tag.ends_with('/');
    let tag_body = raw_tag.trim_start_matches('/').trim_end_matches('/').trim();
    let tag_name = tag_body.split_whitespace().next().unwrap_or_default();
    let local = tag_name.rsplit_once(':').map_or(tag_name, |(_, l)| l);

    match (local, is_closing, is_self_closing) {
      ("row", false, false) => {
        in_row = true;
        current_row.clear();
        next_col_idx = 0;
      }
      ("row", true, _) => {
        if in_row && !current_row.is_empty() {
          rows.push(std::mem::take(&mut current_row));
        }
        in_row = false;
      }
      ("c", false, false) if in_row => {
        in_cell = true;
        cell_col_idx = xml_attr_value(tag_body, "r")
          .and_then(|r| xlsx_col_index_from_ref(&r))
          .unwrap_or(next_col_idx);
        next_col_idx = cell_col_idx + 1;
        cell_type = xml_attr_value(tag_body, "t").unwrap_or_default();
        cell_v.clear();
        cell_inline.clear();
      }
      ("c", false, true) if in_row => {
        let col = xml_attr_value(tag_body, "r")
          .and_then(|r| xlsx_col_index_from_ref(&r))
          .unwrap_or(next_col_idx);
        next_col_idx = col + 1;
      }
      ("c", true, _) if in_cell => {
        let resolved = match cell_type.as_str() {
          "s" => cell_v
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|idx| shared_strings.get(idx).cloned())
            .unwrap_or_default(),
          "inlineStr" => cell_inline.clone(),
          "b" => match cell_v.trim() {
            "1" => "true".to_string(),
            "0" => "false".to_string(),
            other => other.to_string(),
          },
          _ => cell_v.clone(),
        };
        let trimmed = resolved.trim();
        if !trimmed.is_empty() {
          current_row.push((cell_col_idx, trimmed.to_string()));
        }
        in_cell = false;
        in_v = false;
        in_is_t = false;
      }
      ("v", false, false) if in_cell => {
        in_v = true;
      }
      ("v", true, _) => {
        in_v = false;
      }
      ("t", false, false) if in_cell => {
        in_is_t = true;
      }
      ("t", true, _) => {
        in_is_t = false;
      }
      _ => {}
    }
  }
  rows
}

fn format_xlsx_sheet_rows(sheet_title: &str, rows: &[Vec<(usize, String)>]) -> String {
  let max_col = rows
    .iter()
    .flat_map(|r| r.iter().map(|(col, _)| *col))
    .max()
    .unwrap_or(0);
  if rows.len() == 1 {
    let pairs: Vec<String> = rows[0]
      .iter()
      .map(|(col, val)| format!("{}: {val}", xlsx_col_letters(*col)))
      .collect();
    return format!("Sheet: {sheet_title}\nRow 1: {}", pairs.join(" | "));
  }

  let mut headers = vec![String::new(); max_col + 1];
  for (col, val) in &rows[0] {
    if let Some(slot) = headers.get_mut(*col) {
      *slot = val.clone();
    }
  }
  for (col_idx, slot) in headers.iter_mut().enumerate() {
    if slot.trim().is_empty() {
      *slot = format!("col_{}", xlsx_col_letters(col_idx));
    }
  }

  let mut blocks = vec![format!(
    "Sheet: {sheet_title}\nColumns: {}",
    headers.join(" | ")
  )];
  let mut current_group: Vec<String> = Vec::new();
  for (row_idx, row) in rows[1..].iter().enumerate() {
    let pairs: Vec<String> = row
      .iter()
      .map(|(col, val)| {
        let label = headers
          .get(*col)
          .cloned()
          .unwrap_or_else(|| format!("col_{}", xlsx_col_letters(*col)));
        format!("{label}: {val}")
      })
      .collect();
    if !pairs.is_empty() {
      current_group.push(format!("Row {}: {}", row_idx + 1, pairs.join(" | ")));
      if current_group.len() >= 10 {
        blocks.push(current_group.join("\n"));
        current_group.clear();
      }
    }
  }
  if !current_group.is_empty() {
    blocks.push(current_group.join("\n"));
  }
  blocks.join("\n\n")
}

/// Shared paragraph + table extractor for WordprocessingML (`w:*`, `.docx`) and
/// DrawingML / PresentationML (`a:*`, `.pptx`).
pub fn extract_ooxml_rich_text(xml: &str) -> String {
  let mut blocks: Vec<String> = Vec::new();
  let mut current_para = String::new();
  let mut in_text_run = false;
  let mut table_depth: usize = 0;
  let mut current_row: Vec<String> = Vec::new();
  let mut current_cell_paras: Vec<String> = Vec::new();

  let mut rest = xml;
  while let Some(lt) = rest.find('<') {
    if in_text_run && lt > 0 {
      current_para.push_str(&decode_xml_entities(&rest[..lt]));
    }
    rest = &rest[lt + 1..];
    let Some(gt) = rest.find('>') else {
      break;
    };
    let raw_tag = rest[..gt].trim();
    rest = &rest[gt + 1..];
    if raw_tag.starts_with('?') || raw_tag.starts_with('!') {
      continue;
    }
    let is_closing = raw_tag.starts_with('/');
    let is_self_closing = raw_tag.ends_with('/');
    let tag_body = raw_tag.trim_start_matches('/').trim_end_matches('/').trim();
    let tag_name = tag_body.split_whitespace().next().unwrap_or_default();

    match (tag_name, is_closing, is_self_closing) {
      ("w:t" | "a:t" | "t", false, false) => {
        in_text_run = true;
      }
      ("w:t" | "a:t" | "t", true, _) => {
        in_text_run = false;
      }
      ("w:tab" | "a:tab", false, _) => {
        current_para.push('\t');
      }
      ("w:br" | "w:cr" | "a:br", false, _) => {
        current_para.push('\n');
      }
      ("w:tbl" | "a:tbl", false, false) => {
        let trimmed = current_para.trim();
        if !trimmed.is_empty() {
          blocks.push(trimmed.to_string());
        }
        current_para.clear();
        table_depth += 1;
      }
      ("w:tbl" | "a:tbl", true, _) => {
        table_depth = table_depth.saturating_sub(1);
      }
      ("w:tr" | "a:tr", false, false) if table_depth > 0 => {
        current_row.clear();
      }
      ("w:tc" | "a:tc", false, false) if table_depth > 0 => {
        current_cell_paras.clear();
        current_para.clear();
      }
      ("w:p" | "a:p", true, _) => {
        let trimmed = current_para.trim();
        if !trimmed.is_empty() {
          if table_depth > 0 {
            current_cell_paras.push(trimmed.to_string());
          } else {
            blocks.push(trimmed.to_string());
          }
        }
        current_para.clear();
      }
      ("w:tc" | "a:tc", true, _) if table_depth > 0 => {
        current_row.push(current_cell_paras.join(" "));
        current_cell_paras.clear();
      }
      ("w:tr" | "a:tr", true, _) if table_depth > 0 => {
        if current_row.iter().any(|c| !c.trim().is_empty()) {
          blocks.push(format!("| {} |", current_row.join(" | ")));
        }
        current_row.clear();
      }
      _ => {}
    }
  }
  blocks.join("\n\n")
}

/// Decode standard named and numeric XML character entities.
pub fn decode_xml_entities(input: &str) -> String {
  if !input.contains('&') {
    return input.to_string();
  }
  let mut out = String::with_capacity(input.len());
  let mut rest = input;
  while let Some(amp) = rest.find('&') {
    out.push_str(&rest[..amp]);
    let after = &rest[amp + 1..];
    if let Some(semi) = after.find(';') {
      let entity = &after[..semi];
      let decoded = match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ if entity.starts_with("#x") || entity.starts_with("#X") => {
          u32::from_str_radix(&entity[2..], 16)
            .ok()
            .and_then(char::from_u32)
        }
        _ if entity.starts_with('#') => entity[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
      };
      if let Some(ch) = decoded {
        out.push(ch);
        rest = &after[semi + 1..];
        continue;
      }
    }
    out.push('&');
    rest = after;
  }
  out.push_str(rest);
  out
}

/// Format CSV / TSV tabular bytes into self-describing paragraph blocks
/// (`Header: Value | ...`) so every chunk retains column labels during BM25
/// search and downstream Studio generation.
pub fn delimited_text(bytes: &[u8], delimiter: u8) -> Result<String, String> {
  let raw = std::str::from_utf8(bytes)
    .map_err(|_| "tabular file is not UTF-8 text".to_string())?
    .trim_start_matches('\u{feff}');
  let mut reader = csv::ReaderBuilder::new()
    .delimiter(delimiter)
    .flexible(true)
    .from_reader(raw.as_bytes());
  let headers: Vec<String> = reader
    .headers()
    .map_err(|e| format!("parsing table header failed: {e}"))?
    .iter()
    .map(|h| h.trim().to_string())
    .collect();
  if headers.is_empty() {
    return Err("tabular file has no columns".to_string());
  }

  let mut lines: Vec<String> = Vec::new();
  lines.push(format!("Columns: {}", headers.join(" | ")));

  let mut current_group: Vec<String> = Vec::new();
  for (row_idx, record) in reader.records().enumerate() {
    let record = record.map_err(|e| format!("parsing row {} failed: {e}", row_idx + 1))?;
    let pairs: Vec<String> = record
      .iter()
      .enumerate()
      .filter_map(|(col_idx, cell)| {
        let val = cell.trim();
        if val.is_empty() {
          return None;
        }
        let col_label = headers
          .get(col_idx)
          .filter(|h| !h.is_empty())
          .cloned()
          .unwrap_or_else(|| format!("col_{}", col_idx + 1));
        Some(format!("{col_label}: {val}"))
      })
      .collect();
    if !pairs.is_empty() {
      current_group.push(format!("Row {}: {}", row_idx + 1, pairs.join(" | ")));
      if current_group.len() >= 10 {
        lines.push(current_group.join("\n"));
        current_group.clear();
      }
    }
  }
  if !current_group.is_empty() {
    lines.push(current_group.join("\n"));
  }
  Ok(lines.join("\n\n"))
}

/// Format structured JSON into paragraph-separated records for chunking & BM25.
pub fn json_text(bytes: &[u8]) -> Result<String, String> {
  let raw = std::str::from_utf8(bytes)
    .map_err(|_| "JSON file is not UTF-8 text".to_string())?
    .trim_start_matches('\u{feff}');
  let parsed: Value = serde_json::from_str(raw).map_err(|e| format!("invalid JSON: {e}"))?;
  match parsed {
    Value::Array(items) => {
      let mut blocks = Vec::with_capacity(items.len());
      for (idx, item) in items.into_iter().enumerate() {
        match item {
          Value::Object(map) => {
            let pairs: Vec<String> = map
              .into_iter()
              .filter(|(_, v)| !v.is_null())
              .map(|(k, v)| match v {
                Value::String(s) => format!("{k}: {s}"),
                other => format!("{k}: {other}"),
              })
              .collect();
            if !pairs.is_empty() {
              blocks.push(format!("Record {}: {}", idx + 1, pairs.join(" | ")));
            }
          }
          other => {
            blocks.push(format!(
              "Record {}: {}",
              idx + 1,
              serde_json::to_string_pretty(&other).unwrap_or_default()
            ));
          }
        }
      }
      Ok(blocks.join("\n\n"))
    }
    other => serde_json::to_string_pretty(&other).map_err(|e| format!("formatting JSON: {e}")),
  }
}

/// Transcribe an uploaded audio recording via a registered ASR model.
pub async fn transcribe_recording(
  bytes: Vec<u8>,
  filename: String,
  model: String,
  language: Option<String>,
) -> Result<String, String> {
  yanshi_llm::Yanshi::ensure_init()
    .await
    .map_err(|e| format!("initializing model registry: {e}"))?;
  let provider = yanshi_llm::Yanshi::asr(&model)
    .await
    .map_err(|e| format!("resolving ASR model '{model}': {e}"))?;
  let response = provider
    .transcribe(yanshi_llm::AsrRequest {
      model,
      response_format: "text".to_string(),
      audio_data: bytes,
      filename,
      language: language
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty()),
      temperature: None,
      prompt: None,
    })
    .await
    .map_err(|e| format!("transcribing recording failed: {e}"))?;
  Ok(response.text)
}

/// Split source text into paragraph chunks with line-range metadata.
pub fn chunk_paragraphs(text: &str) -> Result<Vec<(String, Value)>, String> {
  use yanshi_rag::chunking::{ChunkingStrategy, ParagraphChunker};
  let chunks = ParagraphChunker::new(CHUNK_SIZE, CHUNK_OVERLAP)
    .chunk(text)
    .map_err(|e| format!("chunking failed: {e}"))?;
  Ok(
    chunks
      .into_iter()
      .filter(|c| !c.content.trim().is_empty())
      .map(|c| {
        (
          c.content,
          json!({ "start_line": c.start_idx, "end_line": c.end_idx }),
        )
      })
      .collect(),
  )
}

/// Parse an optional comma-separated `source_ids` query string into deduplicated UUIDs.
pub fn parse_source_ids_csv(raw: Option<&str>) -> Result<Option<Vec<Uuid>>, String> {
  let Some(raw) = raw else {
    return Ok(None);
  };
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return Err("source_ids must select at least one source".to_string());
  }
  let mut ids = Vec::new();
  for token in trimmed.split(',') {
    let token = token.trim();
    if token.is_empty() {
      continue;
    }
    let id = Uuid::parse_str(token).map_err(|_| format!("invalid source_id UUID '{token}'"))?;
    if !ids.contains(&id) {
      ids.push(id);
    }
  }
  if ids.is_empty() {
    return Err("source_ids must select at least one source".to_string());
  }
  Ok(Some(ids))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smart_url_rewrites_arxiv_openreview_and_github() {
    assert_eq!(
      classify_smart_source_url("https://arxiv.org/abs/2401.12345"),
      SmartUrlTarget::RewriteUrl("https://arxiv.org/pdf/2401.12345.pdf".to_string())
    );
    assert_eq!(
      classify_smart_source_url("https://openreview.net/forum?id=abc123"),
      SmartUrlTarget::RewriteUrl("https://openreview.net/pdf?id=abc123".to_string())
    );
    assert_eq!(
      classify_smart_source_url("https://github.com/org/repo/blob/main/src/lib.rs"),
      SmartUrlTarget::RewriteUrl(
        "https://raw.githubusercontent.com/org/repo/main/src/lib.rs".to_string()
      )
    );
    assert_eq!(
      classify_smart_source_url("https://github.com/org/repo/tree/main/crates"),
      SmartUrlTarget::GitHubRepo {
        owner: "org".to_string(),
        repo: "repo".to_string(),
        git_ref: Some("main".to_string()),
        subpath: Some("crates".to_string()),
      }
    );
  }

  #[test]
  fn subtitle_and_tabular_extractors_preserve_structure() {
    let srt = b"1\n00:00:01,000 --> 00:00:03,500\n<i>Hello</i> world\n\n2\n00:00:04,000 --> 00:00:06,000\nSecond cue\n";
    let parsed = subtitle_text(srt).expect("srt");
    assert!(parsed.contains("[00:00:01.000 -> 00:00:03.500] Hello world"));
    assert!(parsed.contains("[00:00:04.000 -> 00:00:06.000] Second cue"));

    let csv_out = delimited_text(b"name,score\nAlice,98\nBob,91\n", b',').expect("csv");
    assert!(csv_out.contains("Columns: name | score"));
    assert!(csv_out.contains("Row 1: name: Alice | score: 98"));
  }

  #[test]
  fn parse_source_ids_csv_deduplicates_and_rejects_empty() {
    assert_eq!(parse_source_ids_csv(None).expect("none"), None);
    assert!(parse_source_ids_csv(Some("  ")).is_err());
    let u = Uuid::new_v4();
    let list = parse_source_ids_csv(Some(&format!("{u}, {u}")))
      .expect("valid")
      .expect("some");
    assert_eq!(list, vec![u]);
  }
}
