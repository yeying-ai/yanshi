use std::collections::{BTreeSet, HashMap};
use std::path::{Component, Path};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  value::FlowValue,
};
use yanshi_tools::SandboxPolicy;
use yanshi_tools::builtin::HttpTool;

const DEFAULT_GITHUB_API_BASE: &str = "https://api.github.com";
const DEFAULT_GITHUB_RAW_BASE: &str = "https://raw.githubusercontent.com";
const DEFAULT_MAX_BYTES: usize = 120_000;
const DEFAULT_MAX_FILES: usize = 28;
const MAX_SINGLE_FILE_CHARS: usize = 10_000;
const MAX_TREE_LINES: usize = 400;

const IGNORED_DIRS: &[&str] = &[
  ".git",
  "node_modules",
  "target",
  "dist",
  "build",
  ".next",
  ".turbo",
  "vendor",
  "__pycache__",
  ".venv",
  "venv",
  "coverage",
];

const IGNORED_EXTENSIONS: &[&str] = &[
  "png", "jpg", "jpeg", "gif", "webp", "ico", "pdf", "zip", "tar", "gz", "xz", "woff", "woff2",
  "ttf", "otf", "mp3", "mp4", "mov", "wav", "wasm", "pyc", "so", "dylib", "dll", "exe", "bin",
  "lock",
];

const IGNORED_FILENAMES: &[&str] = &[
  "Cargo.lock",
  "package-lock.json",
  "pnpm-lock.yaml",
  "yarn.lock",
  "poetry.lock",
  "uv.lock",
  "go.sum",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SampledFile {
  pub path: String,
  pub kind: String,
  pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyEdge {
  pub from: String,
  pub to: String,
  pub kind: String,
}

#[derive(Clone)]
pub struct RepoSnapshotNode {
  policy: Arc<SandboxPolicy>,
  github_api_base: String,
  github_raw_base: String,
  github_token: Option<String>,
  no_proxy: bool,
}

impl std::fmt::Debug for RepoSnapshotNode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RepoSnapshotNode").finish_non_exhaustive()
  }
}

impl RepoSnapshotNode {
  pub fn new(policy: Arc<SandboxPolicy>) -> Self {
    Self {
      policy,
      github_api_base: DEFAULT_GITHUB_API_BASE.to_string(),
      github_raw_base: DEFAULT_GITHUB_RAW_BASE.to_string(),
      github_token: None,
      no_proxy: false,
    }
  }

  pub fn with_github_endpoints(
    mut self,
    api_base: impl Into<String>,
    raw_base: impl Into<String>,
  ) -> Self {
    self.github_api_base = api_base.into();
    self.github_raw_base = raw_base.into();
    self
  }

  pub fn with_github_token(mut self, token: impl Into<String>) -> Self {
    self.github_token = Some(token.into());
    self
  }

  pub fn with_no_proxy(mut self) -> Self {
    self.no_proxy = true;
    self
  }
}

impl Default for RepoSnapshotNode {
  fn default() -> Self {
    Self::new(Arc::new(SandboxPolicy::default()))
  }
}

struct SnapshotData {
  repo_name: String,
  description: String,
  default_branch: String,
  tree_paths: Vec<String>,
  sampled_files: Vec<SampledFile>,
}

#[async_trait]
impl AsyncNode for RepoSnapshotNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let source = read_optional_string(inputs, "source")?
      .or(read_optional_string(inputs, "url")?)
      .or(read_optional_string(inputs, "path")?)
      .ok_or_else(|| YanshiError::NodeInputError {
        message:
          "repo_snapshot requires a non-empty 'source' (GitHub URL or sandboxed local directory)"
            .to_string(),
      })?;

    let git_ref = read_optional_string(inputs, "git_ref")?;
    let subpath = read_optional_string(inputs, "subpath")?;
    let max_bytes = read_optional_usize(inputs, "max_bytes")?
      .unwrap_or(DEFAULT_MAX_BYTES)
      .clamp(4_096, 1_000_000);
    let max_files = read_optional_usize(inputs, "max_files")?
      .unwrap_or(DEFAULT_MAX_FILES)
      .clamp(1, 100);

    let snapshot = if is_remote_repo_source(source) {
      self
        .snapshot_github(source, git_ref, subpath, max_bytes, max_files)
        .await?
    } else {
      self
        .snapshot_local(Path::new(source), subpath, max_bytes, max_files)
        .await?
    };

    let edges = extract_dependency_hints(&snapshot.sampled_files);
    let manifests: Vec<&SampledFile> = snapshot
      .sampled_files
      .iter()
      .filter(|f| f.kind == "manifest")
      .collect();
    let tree_text = format_tree(&snapshot.tree_paths);
    let markdown = format_snapshot_markdown(&snapshot, &tree_text, &edges);

    let manifests_json =
      serde_json::to_value(&manifests).map_err(|err| YanshiError::NodeExecutionFailed {
        message: format!("Failed to serialize manifests: {err}"),
      })?;
    let files_json = serde_json::to_value(&snapshot.sampled_files).map_err(|err| {
      YanshiError::NodeExecutionFailed {
        message: format!("Failed to serialize sampled files: {err}"),
      }
    })?;
    let edges_json =
      serde_json::to_value(&edges).map_err(|err| YanshiError::NodeExecutionFailed {
        message: format!("Failed to serialize dependency hints: {err}"),
      })?;

    let mut outputs = HashMap::new();
    outputs.insert(
      "repo_name".to_string(),
      FlowValue::Json(json!(snapshot.repo_name)),
    );
    outputs.insert("tree".to_string(), FlowValue::Json(json!(tree_text)));
    outputs.insert("manifests".to_string(), FlowValue::Json(manifests_json));
    outputs.insert("files".to_string(), FlowValue::Json(files_json));
    outputs.insert("dependency_hints".to_string(), FlowValue::Json(edges_json));
    outputs.insert("markdown".to_string(), FlowValue::Json(json!(markdown)));
    outputs.insert("output".to_string(), FlowValue::Json(json!(markdown)));
    Ok(outputs)
  }
}

impl RepoSnapshotNode {
  fn build_http(&self) -> Result<HttpTool, YanshiError> {
    let mut http = HttpTool::new(self.policy.clone())
      .map_err(|err| YanshiError::AsyncExecutionError {
        message: format!("repo_snapshot failed to build HTTP client: {err}"),
      })?
      .with_max_response_chars(usize::MAX);
    if self.no_proxy {
      http = http.with_no_proxy();
    }
    Ok(http)
  }

  fn github_headers(&self) -> Value {
    let token = self.github_token.clone().or_else(|| {
      std::env::var("GITHUB_TOKEN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    });
    let mut map = serde_json::Map::new();
    map.insert(
      "Accept".to_string(),
      Value::String("application/vnd.github+json".to_string()),
    );
    map.insert(
      "X-GitHub-Api-Version".to_string(),
      Value::String("2022-11-28".to_string()),
    );
    if let Some(tok) = token {
      map.insert(
        "Authorization".to_string(),
        Value::String(format!("Bearer {tok}")),
      );
    }
    Value::Object(map)
  }

  async fn snapshot_github(
    &self,
    raw_source: &str,
    explicit_ref: Option<&str>,
    explicit_subpath: Option<&str>,
    max_bytes: usize,
    max_files: usize,
  ) -> Result<SnapshotData, YanshiError> {
    let parsed = parse_github_url(raw_source)?;
    let http = self.build_http()?;
    let headers = self.github_headers();
    let api_base = self.github_api_base.trim_end_matches('/');
    let raw_base = self.github_raw_base.trim_end_matches('/');

    let repo_meta_url = format!("{api_base}/repos/{}/{}", parsed.owner, parsed.repo);
    let meta_resp = http
      .send(&json!({
        "url": repo_meta_url,
        "method": "GET",
        "headers": headers,
      }))
      .await
      .map_err(|err| YanshiError::AsyncExecutionError {
        message: format!("GitHub repo lookup failed: {err}"),
      })?;
    if !meta_resp.status.is_success() {
      let excerpt: String = meta_resp.body.chars().take(400).collect();
      return Err(YanshiError::NodeExecutionFailed {
        message: format!(
          "GitHub API returned HTTP {} for '{}/{}': {excerpt}",
          meta_resp.status, parsed.owner, parsed.repo
        ),
      });
    }
    let meta_json: Value =
      serde_json::from_str(&meta_resp.body).map_err(|err| YanshiError::NodeExecutionFailed {
        message: format!("Invalid JSON from GitHub repo endpoint: {err}"),
      })?;

    let branch = explicit_ref
      .or(parsed.git_ref.as_deref())
      .or_else(|| meta_json["default_branch"].as_str())
      .unwrap_or("main")
      .to_string();
    let description = meta_json["description"]
      .as_str()
      .unwrap_or_default()
      .trim()
      .to_string();
    let effective_subpath = explicit_subpath
      .or(parsed.subpath.as_deref())
      .map(|s| s.trim_matches('/'))
      .filter(|s| !s.is_empty());

    let tree_url = format!(
      "{api_base}/repos/{}/{}/git/trees/{branch}?recursive=1",
      parsed.owner, parsed.repo
    );
    let tree_resp = http
      .send(&json!({
        "url": tree_url,
        "method": "GET",
        "headers": self.github_headers(),
      }))
      .await
      .map_err(|err| YanshiError::AsyncExecutionError {
        message: format!("GitHub git tree lookup failed: {err}"),
      })?;
    if !tree_resp.status.is_success() {
      let excerpt: String = tree_resp.body.chars().take(400).collect();
      return Err(YanshiError::NodeExecutionFailed {
        message: format!(
          "GitHub git tree API returned HTTP {} for ref '{branch}': {excerpt}",
          tree_resp.status
        ),
      });
    }
    let tree_json: Value =
      serde_json::from_str(&tree_resp.body).map_err(|err| YanshiError::NodeExecutionFailed {
        message: format!("Invalid JSON from GitHub tree endpoint: {err}"),
      })?;

    let mut blob_paths = Vec::new();
    if let Some(entries) = tree_json["tree"].as_array() {
      for entry in entries {
        if entry["type"].as_str() != Some("blob") {
          continue;
        }
        let Some(path) = entry["path"].as_str() else {
          continue;
        };
        if let Some(prefix) = effective_subpath
          && !path.starts_with(&format!("{prefix}/"))
          && path != prefix
        {
          continue;
        }
        if is_ignored_repo_path(path) {
          continue;
        }
        let size = entry["size"].as_u64().unwrap_or(0);
        if size > 512_000 {
          continue;
        }
        blob_paths.push(path.to_string());
      }
    }

    blob_paths.sort();
    let ranked = rank_candidate_paths(&blob_paths, max_files);
    let mut sampled_files = Vec::new();
    let mut used_bytes = 0usize;

    for (rel_path, kind) in ranked {
      if used_bytes >= max_bytes {
        break;
      }
      let encoded_path = rel_path
        .split('/')
        .map(|seg| urlencoding::encode(seg).into_owned())
        .collect::<Vec<_>>()
        .join("/");
      let raw_url = format!(
        "{raw_base}/{}/{}/{branch}/{encoded_path}",
        parsed.owner, parsed.repo
      );
      let file_resp = match http.send(&json!({ "url": raw_url, "method": "GET" })).await {
        Ok(r) if r.status.is_success() => r,
        _ => continue,
      };
      let excerpt = truncate_chars(&file_resp.body, MAX_SINGLE_FILE_CHARS);
      let remaining = max_bytes.saturating_sub(used_bytes);
      let clipped = truncate_bytes_utf8(&excerpt, remaining);
      if clipped.trim().is_empty() {
        continue;
      }
      used_bytes += clipped.len();
      sampled_files.push(SampledFile {
        path: rel_path,
        kind,
        content: clipped,
      });
    }

    Ok(SnapshotData {
      repo_name: format!("{}/{}", parsed.owner, parsed.repo),
      description,
      default_branch: branch,
      tree_paths: blob_paths,
      sampled_files,
    })
  }

  async fn snapshot_local(
    &self,
    root: &Path,
    explicit_subpath: Option<&str>,
    max_bytes: usize,
    max_files: usize,
  ) -> Result<SnapshotData, YanshiError> {
    self.validate_local_path(root)?;

    let scan_root = match explicit_subpath
      .map(|s| s.trim_matches('/'))
      .filter(|s| !s.is_empty())
    {
      Some(sub) => {
        let joined = root.join(sub);
        self.validate_local_path(&joined)?;
        joined
      }
      None => root.to_path_buf(),
    };

    let meta = tokio::fs::symlink_metadata(&scan_root)
      .await
      .map_err(|err| YanshiError::AsyncExecutionError {
        message: format!("Failed to stat '{}': {err}", scan_root.display()),
      })?;
    if meta.file_type().is_symlink() || !meta.file_type().is_dir() {
      return Err(YanshiError::NodeInputError {
        message: format!(
          "Local repo_snapshot source '{}' must be a regular directory (not a symlink or file)",
          scan_root.display()
        ),
      });
    }

    let mut blob_paths = Vec::new();
    let mut stack = vec![scan_root.clone()];
    while let Some(dir) = stack.pop() {
      if blob_paths.len() >= 5_000 {
        break;
      }
      let mut rd = match tokio::fs::read_dir(&dir).await {
        Ok(rd) => rd,
        Err(_) => continue,
      };
      while let Ok(Some(entry)) = rd.next_entry().await {
        let Ok(ft) = entry.file_type().await else {
          continue;
        };
        if ft.is_symlink() {
          continue;
        }
        let full_path = entry.path();
        let Ok(rel) = full_path.strip_prefix(&scan_root) else {
          continue;
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if is_ignored_repo_path(&rel_str) {
          continue;
        }
        if ft.is_dir() {
          stack.push(full_path);
        } else if ft.is_file()
          && let Ok(m) = entry.metadata().await
          && m.len() <= 512_000
        {
          blob_paths.push(rel_str);
        }
      }
    }

    blob_paths.sort();
    let ranked = rank_candidate_paths(&blob_paths, max_files);
    let mut sampled_files = Vec::new();
    let mut used_bytes = 0usize;

    for (rel_path, kind) in ranked {
      if used_bytes >= max_bytes {
        break;
      }
      let full_path = scan_root.join(&rel_path);
      let Ok(raw) = tokio::fs::read_to_string(&full_path).await else {
        continue;
      };
      let excerpt = truncate_chars(&raw, MAX_SINGLE_FILE_CHARS);
      let remaining = max_bytes.saturating_sub(used_bytes);
      let clipped = truncate_bytes_utf8(&excerpt, remaining);
      if clipped.trim().is_empty() {
        continue;
      }
      used_bytes += clipped.len();
      sampled_files.push(SampledFile {
        path: rel_path,
        kind,
        content: clipped,
      });
    }

    let repo_name = root
      .file_name()
      .map(|s| s.to_string_lossy().into_owned())
      .unwrap_or_else(|| root.display().to_string());

    Ok(SnapshotData {
      repo_name,
      description: String::new(),
      default_branch: "local".to_string(),
      tree_paths: blob_paths,
      sampled_files,
    })
  }

  fn validate_local_path(&self, path: &Path) -> Result<(), YanshiError> {
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
      return Err(YanshiError::AsyncExecutionError {
        message: format!(
          "Refusing path '{}': parent-directory components ('..') are not allowed",
          path.display()
        ),
      });
    }
    yanshi_core::run_context::resolve_confined_path(path)
      .map_err(|message| YanshiError::NodeInputError { message })?;
    if let Some(reason) = self.policy.path_denial_reason(path) {
      return Err(YanshiError::AsyncExecutionError {
        message: format!("sandbox policy denied: {reason}"),
      });
    }
    Ok(())
  }
}

struct ParsedGitHubUrl {
  owner: String,
  repo: String,
  git_ref: Option<String>,
  subpath: Option<String>,
}

fn is_remote_repo_source(source: &str) -> bool {
  source.starts_with("https://")
    || source.starts_with("http://")
    || source.starts_with("github.com/")
}

fn parse_github_url(raw: &str) -> Result<ParsedGitHubUrl, YanshiError> {
  let normalized = if raw.starts_with("github.com/") {
    format!("https://{raw}")
  } else {
    raw.to_string()
  };
  let parsed = reqwest::Url::parse(&normalized).map_err(|err| YanshiError::NodeInputError {
    message: format!("Invalid repository URL '{raw}': {err}"),
  })?;
  let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
  if host != "github.com" && host != "www.github.com" {
    return Err(YanshiError::NodeInputError {
      message: format!(
        "Unsupported remote repository host '{host}'; expected github.com or a sandboxed local directory"
      ),
    });
  }
  let segments: Vec<&str> = parsed
    .path_segments()
    .map(|it| it.filter(|s| !s.is_empty()).collect())
    .unwrap_or_default();
  if segments.len() < 2 {
    return Err(YanshiError::NodeInputError {
      message: format!("GitHub URL '{raw}' must include <owner>/<repo>"),
    });
  }
  let owner = segments[0].to_string();
  let repo = segments[1].trim_end_matches(".git").to_string();
  let (git_ref, subpath) = if segments.len() >= 4 && matches!(segments[2], "tree" | "blob") {
    let r = Some(segments[3].to_string());
    let sub = if segments.len() > 4 {
      Some(segments[4..].join("/"))
    } else {
      None
    };
    (r, sub)
  } else {
    (None, None)
  };
  Ok(ParsedGitHubUrl {
    owner,
    repo,
    git_ref,
    subpath,
  })
}

fn is_ignored_repo_path(path: &str) -> bool {
  for part in path.split('/') {
    if IGNORED_DIRS.contains(&part) {
      return true;
    }
  }
  let filename = path.rsplit('/').next().unwrap_or(path);
  if IGNORED_FILENAMES.contains(&filename) {
    return true;
  }
  if let Some((_, ext)) = filename.rsplit_once('.')
    && IGNORED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str())
  {
    return true;
  }
  false
}

fn classify_and_score_path(path: &str) -> Option<(u8, &'static str)> {
  let filename = path.rsplit('/').next().unwrap_or(path);
  let depth = path.matches('/').count();
  let lower = filename.to_ascii_lowercase();

  let is_manifest = matches!(
    filename,
    "Cargo.toml"
      | "package.json"
      | "pnpm-workspace.yaml"
      | "go.mod"
      | "pyproject.toml"
      | "requirements.txt"
      | "pom.xml"
      | "build.gradle"
      | "build.gradle.kts"
      | "Dockerfile"
  ) || lower.starts_with("docker-compose");

  if is_manifest {
    let tier = if depth == 0 { 0 } else { 2 };
    return Some((tier, "manifest"));
  }

  if depth <= 1
    && (lower.starts_with("readme")
      || lower == "architecture.md"
      || lower == "claude.md"
      || lower == "agents.md")
  {
    return Some((1, "doc"));
  }

  let is_entrypoint = matches!(
    filename,
    "lib.rs"
      | "main.rs"
      | "mod.rs"
      | "index.ts"
      | "index.tsx"
      | "main.ts"
      | "server.ts"
      | "app.ts"
      | "main.go"
      | "main.py"
      | "__init__.py"
      | "Application.java"
  );
  if is_entrypoint && depth <= 3 {
    return Some((3, "entrypoint"));
  }

  if depth <= 2
    && matches!(
      filename.rsplit_once('.').map(|(_, ext)| ext),
      Some("rs" | "ts" | "tsx" | "go" | "py" | "java" | "proto" | "sql")
    )
  {
    return Some((4, "source"));
  }

  None
}

fn rank_candidate_paths(paths: &[String], max_files: usize) -> Vec<(String, String)> {
  let mut scored: Vec<(u8, usize, &String, &'static str)> = paths
    .iter()
    .filter_map(|p| {
      let (tier, kind) = classify_and_score_path(p)?;
      Some((tier, p.len(), p, kind))
    })
    .collect();
  scored.sort_by(|a, b| {
    a.0
      .cmp(&b.0)
      .then_with(|| a.1.cmp(&b.1))
      .then_with(|| a.2.cmp(b.2))
  });
  scored
    .into_iter()
    .take(max_files)
    .map(|(_, _, path, kind)| (path.clone(), kind.to_string()))
    .collect()
}

/// Deterministically extract internal module/workspace dependencies from manifests.
pub fn extract_dependency_hints(files: &[SampledFile]) -> Vec<DependencyEdge> {
  let mut edges = BTreeSet::new();
  let mut cargo_packages: HashMap<String, String> = HashMap::new();
  let mut npm_packages: HashMap<String, String> = HashMap::new();

  for file in files.iter().filter(|f| f.kind == "manifest") {
    let filename = file.path.rsplit('/').next().unwrap_or(&file.path);
    if filename == "Cargo.toml"
      && let Some(pkg_name) = extract_cargo_package_name(&file.content)
    {
      cargo_packages.insert(pkg_name, file.path.clone());
    } else if filename == "package.json"
      && let Ok(val) = serde_json::from_str::<Value>(&file.content)
      && let Some(name) = val["name"].as_str()
    {
      npm_packages.insert(name.to_string(), file.path.clone());
    }
  }

  for file in files.iter().filter(|f| f.kind == "manifest") {
    let filename = file.path.rsplit('/').next().unwrap_or(&file.path);
    if filename == "Cargo.toml" {
      if let Some(from_pkg) = extract_cargo_package_name(&file.content) {
        for dep in extract_cargo_internal_deps(&file.content, &cargo_packages) {
          if dep != from_pkg {
            edges.insert(DependencyEdge {
              from: from_pkg.clone(),
              to: dep,
              kind: "cargo".to_string(),
            });
          }
        }
      }
    } else if filename == "package.json"
      && let Ok(val) = serde_json::from_str::<Value>(&file.content)
      && let Some(from_pkg) = val["name"].as_str()
    {
      for section in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(obj) = val[section].as_object() {
          for (dep_name, dep_ver) in obj {
            let is_workspace = dep_ver
              .as_str()
              .is_some_and(|v| v.starts_with("workspace:") || v.starts_with("file:"));
            if (is_workspace || npm_packages.contains_key(dep_name)) && dep_name != from_pkg {
              edges.insert(DependencyEdge {
                from: from_pkg.to_string(),
                to: dep_name.clone(),
                kind: "npm".to_string(),
              });
            }
          }
        }
      }
    }
  }

  edges.into_iter().collect()
}

fn extract_cargo_package_name(toml_text: &str) -> Option<String> {
  let mut in_package = false;
  for raw_line in toml_text.lines() {
    let line = raw_line.split('#').next().unwrap_or("").trim();
    if line.starts_with('[') && line.ends_with(']') {
      in_package = line == "[package]";
      continue;
    }
    if in_package
      && let Some(rest) = line.strip_prefix("name")
      && let Some((_, val)) = rest.split_once('=')
    {
      let cleaned = val.trim().trim_matches('"').trim_matches('\'');
      if !cleaned.is_empty() {
        return Some(cleaned.to_string());
      }
    }
  }
  None
}

fn extract_cargo_internal_deps(
  toml_text: &str,
  known_packages: &HashMap<String, String>,
) -> Vec<String> {
  let mut deps = Vec::new();
  let mut in_deps = false;
  for raw_line in toml_text.lines() {
    let line = raw_line.split('#').next().unwrap_or("").trim();
    if line.starts_with('[') && line.ends_with(']') {
      in_deps = line == "[dependencies]"
        || line.ends_with(".dependencies]")
        || line == "[build-dependencies]";
      continue;
    }
    if !in_deps || line.is_empty() {
      continue;
    }
    let Some((raw_key, raw_rhs)) = line.split_once('=') else {
      continue;
    };
    let key = raw_key.trim().trim_matches('"');
    if key.is_empty() {
      continue;
    }
    let has_path = raw_rhs.contains("path =") || raw_rhs.contains("path=");
    if has_path || known_packages.contains_key(key) {
      deps.push(key.to_string());
    }
  }
  deps
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
  if text.chars().count() <= max_chars {
    return text.to_string();
  }
  let end = text
    .char_indices()
    .nth(max_chars)
    .map(|(idx, _)| idx)
    .unwrap_or(text.len());
  format!("{}\n... [truncated]", &text[..end])
}

fn truncate_bytes_utf8(text: &str, max_bytes: usize) -> String {
  if text.len() <= max_bytes {
    return text.to_string();
  }
  let mut end = max_bytes;
  while end > 0 && !text.is_char_boundary(end) {
    end -= 1;
  }
  text[..end].to_string()
}

fn format_tree(paths: &[String]) -> String {
  if paths.is_empty() {
    return "(empty repository tree)".to_string();
  }
  let mut lines: Vec<String> = paths.iter().take(MAX_TREE_LINES).cloned().collect();
  if paths.len() > MAX_TREE_LINES {
    lines.push(format!(
      "... ({} additional files omitted)",
      paths.len() - MAX_TREE_LINES
    ));
  }
  lines.join("\n")
}

fn format_snapshot_markdown(
  snapshot: &SnapshotData,
  tree_text: &str,
  edges: &[DependencyEdge],
) -> String {
  let mut out = String::new();
  out.push_str(&format!(
    "# Repository Snapshot: {}\n\n",
    snapshot.repo_name
  ));
  if !snapshot.description.is_empty() {
    out.push_str(&format!("> {}\n\n", snapshot.description));
  }
  out.push_str(&format!(
    "- **Branch / Ref**: `{}`\n- **Total Indexed Files**: {}\n- **Sampled Key Files**: {}\n\n",
    snapshot.default_branch,
    snapshot.tree_paths.len(),
    snapshot.sampled_files.len()
  ));

  if !edges.is_empty() {
    out.push_str("## Deterministic Internal Module Dependencies\n\n");
    for edge in edges {
      out.push_str(&format!(
        "- `{}` -> `{}` ({})\n",
        edge.from, edge.to, edge.kind
      ));
    }
    out.push('\n');
  }

  out.push_str("## Directory & File Tree\n\n```text\n");
  out.push_str(tree_text);
  out.push_str("\n```\n\n## Key Manifests, Docs & Entrypoints\n\n");

  for file in &snapshot.sampled_files {
    out.push_str(&format!(
      "### `{}` ({})\n\n```\n{}\n```\n\n",
      file.path, file.kind, file.content
    ));
  }
  out
}

fn read_optional_string<'a>(
  inputs: &'a AsyncNodeInputs,
  key: &str,
) -> Result<Option<&'a str>, YanshiError> {
  match inputs.get(key) {
    None | Some(FlowValue::Json(Value::Null)) => Ok(None),
    Some(FlowValue::Json(Value::String(s))) => {
      let trimmed = s.trim();
      if trimmed.is_empty() {
        Ok(None)
      } else {
        Ok(Some(trimmed))
      }
    }
    _ => Err(YanshiError::NodeInputError {
      message: format!("Input '{key}' has wrong type, expected string"),
    }),
  }
}

fn read_optional_usize(inputs: &AsyncNodeInputs, key: &str) -> Result<Option<usize>, YanshiError> {
  match inputs.get(key) {
    None | Some(FlowValue::Json(Value::Null)) => Ok(None),
    Some(FlowValue::Json(Value::Number(n))) => n
      .as_u64()
      .and_then(|v| usize::try_from(v).ok())
      .map(Some)
      .ok_or_else(|| YanshiError::NodeInputError {
        message: format!("Input '{key}' must be a positive integer"),
      }),
    Some(FlowValue::Json(Value::String(s))) => {
      let trimmed = s.trim();
      if trimmed.is_empty() {
        return Ok(None);
      }
      trimmed
        .parse::<usize>()
        .map(Some)
        .map_err(|_| YanshiError::NodeInputError {
          message: format!("Input '{key}' must be an integer, got '{trimmed}'"),
        })
    }
    _ => Err(YanshiError::NodeInputError {
      message: format!("Input '{key}' has wrong type, expected integer"),
    }),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  use tokio::net::TcpListener;

  #[tokio::test]
  async fn snapshots_local_cargo_workspace_and_extracts_dependency_edges() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    tokio::fs::write(
      root.join("Cargo.toml"),
      "[workspace]\nmembers = [\"core\", \"api\"]\n",
    )
    .await
    .expect("write root cargo");
    tokio::fs::write(root.join("README.md"), "# Demo Backend\nLayered service.")
      .await
      .expect("write readme");

    tokio::fs::create_dir_all(root.join("core/src"))
      .await
      .expect("mkdir core");
    tokio::fs::write(
      root.join("core/Cargo.toml"),
      "[package]\nname = \"demo-core\"\nversion = \"0.1.0\"\n",
    )
    .await
    .expect("write core cargo");
    tokio::fs::write(root.join("core/src/lib.rs"), "pub struct Domain;")
      .await
      .expect("write core lib");

    tokio::fs::create_dir_all(root.join("api/src"))
      .await
      .expect("mkdir api");
    tokio::fs::write(
      root.join("api/Cargo.toml"),
      "[package]\nname = \"demo-api\"\nversion = \"0.1.0\"\n\n[dependencies]\ndemo-core = { path = \"../core\" }\n",
    )
    .await
    .expect("write api cargo");
    tokio::fs::write(root.join("api/src/main.rs"), "fn main() {}")
      .await
      .expect("write api main");

    // Deny-by-default must block when allowed_paths is empty
    let denied = RepoSnapshotNode::default()
      .execute(&HashMap::from([(
        "source".to_string(),
        FlowValue::Json(json!(root.to_string_lossy())),
      )]))
      .await;
    assert!(denied.is_err());

    // Allowed when root is in SandboxPolicy.allowed_paths
    let policy = Arc::new(SandboxPolicy {
      allowed_paths: vec![root.to_path_buf()],
      ..SandboxPolicy::default()
    });
    let out = RepoSnapshotNode::new(policy)
      .execute(&HashMap::from([(
        "source".to_string(),
        FlowValue::Json(json!(root.to_string_lossy())),
      )]))
      .await
      .expect("local snapshot succeeds");

    let FlowValue::Json(Value::Array(edges)) = &out["dependency_hints"] else {
      panic!("expected dependency_hints array");
    };
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["from"], "demo-api");
    assert_eq!(edges[0]["to"], "demo-core");

    let FlowValue::Json(Value::String(md)) = &out["markdown"] else {
      panic!("expected markdown string");
    };
    assert!(md.contains("`demo-api` -> `demo-core`"));
  }

  #[tokio::test]
  async fn snapshots_github_repo_via_mock_api_without_proxy() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let server = tokio::spawn(async move {
      for _ in 0..3 {
        let Ok((mut stream, _)) = listener.accept().await else {
          break;
        };
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let body = if req.contains("/git/trees/") {
          r#"{"tree":[{"path":"README.md","type":"blob","size":24},{"path":"node_modules/pkg/index.js","type":"blob","size":50}]}"#
        } else if req.contains("/repos/acme/widget") {
          r#"{"default_branch":"main","description":"Fast widget service"}"#
        } else {
          "# Acme Widget\nArchitecture overview."
        };
        let resp = format!(
          "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
          body.len()
        );
        let _ = stream.write_all(resp.as_bytes()).await;
      }
    });

    let base = format!("http://{addr}");
    let policy = Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      ..SandboxPolicy::default()
    });
    let node = RepoSnapshotNode::new(policy)
      .with_github_endpoints(&base, &base)
      .with_no_proxy();

    let out = node
      .execute(&HashMap::from([(
        "source".to_string(),
        FlowValue::Json(json!("https://github.com/acme/widget")),
      )]))
      .await
      .expect("github snapshot succeeds");
    server.await.expect("server");

    assert_eq!(out["repo_name"], FlowValue::Json(json!("acme/widget")));
    let FlowValue::Json(Value::String(md)) = &out["markdown"] else {
      panic!("expected markdown");
    };
    assert!(md.contains("Fast widget service"));
    assert!(md.contains("Acme Widget"));
    assert!(!md.contains("node_modules"));
  }
}
