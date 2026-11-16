//! `figure` (Y3.4, `docs/RFC_CONTENT_STUDIO.md` §5): an LLM writes the
//! source of a figure in a figure language, a real renderer compiles it,
//! and a compile error is fed back for another attempt.
//!
//! Rendering *is* the validation: a source that does not compile is a
//! failed attempt whose error text becomes part of the next prompt, up to
//! `max_attempts` (default 3). Each retry is reported on the run's event
//! stream as `node.progress` with `kind: "figure_retry"`.
//!
//! Backends (`language`):
//! - `svg`: the source is the SVG; checked in-process with resvg.
//! - `tikz`: a standalone LaTeX document compiled with `lualatex` (no
//!   shell escape), then `pdftocairo -svg`. circuitikz / chemfig /
//!   pgfplots / tikz-cd are whatever the TeX Live install provides. The
//!   font cache (`TEXMFVAR`) lives in the run directory.
//! - `typst`: `typst compile` straight to SVG; packages must already be
//!   in typst's cache (the sandbox has no network).
//! - `d2`: `d2 fig.d2 fig.svg`, laid out automatically; for boxes and
//!   arrows where the model should not place coordinates. d2's error
//!   lines (`fig.d2:<line>:<col>: ...`) go back to the model with the work
//!   directory stripped. mermaid is not offered: `mmdc` drives headless
//!   Chromium, which the OS sandbox does not let start.
//! - `python` (Y3.5): a script that saves `figure.svg`, run in the
//!   `code_exec` container tier (no network, read-only rootfs, only the
//!   work directory writable) on the image built from
//!   `scripts/figure_python/Containerfile`: matplotlib, RDKit,
//!   dna_features_viewer, biopython, networkx. `YANSHI_FIGURE_PYTHON_IMAGE`
//!   overrides the tag.
//!
//! Domains (`domain`) add conventions to the prompt and, with a `spec`,
//! check the source against it before every render, so a figure that
//! compiles but leaves something out is sent back too: `circuit` (Y6.5,
//! circuitikz, a parts list), `chemistry` (Y8.4, chemfig, molecules with
//! formulas) and `block` (Y8.4, TikZ or D2, blocks and links). See the
//! module of each.
//!
//! The external renderers run under the OS sandbox
//! (`yanshi_nodes::common::sandboxed`), writing only inside a per-call
//! work directory. Outputs: `<id>_svg` (File), `<id>_png` (File, when
//! `png: true`), `<id>_source` (the source that rendered) and
//! `<id>_attempts`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  run_context,
  value::FlowValue,
};
use yanshi_llm::Yanshi;
use yanshi_nodes::common::{artifacts, sandboxed};

mod block;
mod chemistry;
mod circuit;

const DEFAULT_MAX_ATTEMPTS: u64 = 3;
/// Image for `language: python`; see `scripts/figure_python/Containerfile`.
const DEFAULT_PYTHON_IMAGE: &str = "yanshi-figure-python:latest";
const PYTHON_TIMEOUT: Duration = Duration::from_secs(180);
const RENDER_TIMEOUT: Duration = Duration::from_secs(120);

/// The figure languages this node renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
  Svg,
  Tikz,
  Typst,
  D2,
  Python,
}

impl Language {
  fn parse(raw: &str) -> Option<Self> {
    match raw {
      "svg" => Some(Self::Svg),
      "tikz" => Some(Self::Tikz),
      "typst" => Some(Self::Typst),
      "d2" => Some(Self::D2),
      "python" => Some(Self::Python),
      _ => None,
    }
  }

  fn as_str(self) -> &'static str {
    match self {
      Self::Svg => "svg",
      Self::Tikz => "tikz",
      Self::Typst => "typst",
      Self::D2 => "d2",
      Self::Python => "python",
    }
  }

  /// What the model is told about the language it must write.
  fn instructions(self) -> &'static str {
    match self {
      Self::Svg => {
        "Write one complete standalone SVG document: an <svg> root with \
         xmlns=\"http://www.w3.org/2000/svg\" and explicit width and height. \
         No scripts and no external references."
      }
      Self::Tikz => {
        "Write one complete LaTeX document using \\documentclass[border=4pt]{standalone}. \
         It is compiled with lualatex without shell escape; tikz, circuitikz, chemfig, \
         pgfplots and tikz-cd are available."
      }
      Self::Typst => {
        "Write one complete Typst document that starts with \
         #set page(width: auto, height: auto, margin: 4pt) and produces exactly one page. \
         There is no network: only packages already cached (such as @preview/cetz) can be imported."
      }
      Self::D2 => {
        "Write one valid, concise D2 diagram (d2lang.com — 30 to 70 lines max, 6–15 core nodes, \
         never Mermaid/PlantUML). Keep syntax simple and flat:\n\
         - Comments start with `#` on their own line (never `//`).\n\
         - Valid `shape:` values: `rectangle`, `cylinder` (for databases/caches), `queue`, \
           `cloud`, `person`, `diamond`, `hexagon`, `oval`, `package`, `class`, `sql_table`, `sequence_diagram`.\n\
         - Containers & nodes: `core: \"Core Layer\" { order: \"Order Service\" }`; single-line edges: `a -> b: \"label\"`.\n\
         - Always double-quote labels, hex colors (`style.fill: \"#e3f2fd\"`), and UML class member keys/types (`Order: { shape: class; \"+id\": \"UUID\"; \"+create(cmd)\": \"Result\" }`).\n\
         - Sequence diagrams: `flow: { shape: sequence_diagram; client; api; db; client -> api: \"POST /v1\" }` (no `alt`/`else`/`end`).\n\
         - SQL ER tables: `users: { shape: sql_table; id: bigint {constraint: primary_key} }` and `orders.user_id -> users.id`.\n\
         - Never use `|md` block strings, custom `classes`, external icons, or non-reserved `{ ... }` maps on edges."
      }
      Self::Python => {
        "Write one complete Python 3 script that draws the figure and saves it as SVG to \
         ./figure.svg. matplotlib (Agg backend: plt.savefig(\"figure.svg\")), rdkit (use \
         rdkit.Chem.Draw.rdMolDraw2D.MolDraw2DSVG and write GetDrawingText()), \
         dna_features_viewer, biopython (Bio.Phylo trees drawn with matplotlib) and networkx \
         are installed; nothing else is, and there is no network."
      }
    }
  }
}

/// A figure domain with its parsed `spec`.
enum Domain {
  Circuit(Vec<circuit::Component>),
  Chemistry(chemistry::Scheme),
  Block(block::Diagram, Language),
}

impl Domain {
  fn parse(name: &str, language: Language, spec: Option<&Value>) -> Result<Self, String> {
    let domain = match name {
      "circuit" => Self::Circuit(circuit::parse_spec(spec)?),
      "chemistry" => Self::Chemistry(chemistry::parse_spec(spec)?),
      "block" => Self::Block(block::parse_spec(spec)?, language),
      other => {
        return Err(format!(
          "unknown domain '{other}': expected circuit, chemistry or block"
        ));
      }
    };
    match (&domain, language) {
      (_, Language::Tikz) | (Self::Block(..), Language::D2) => Ok(domain),
      (Self::Block(..), _) => Err(format!("domain {name} needs language tikz or d2")),
      _ => Err(format!("domain {name} needs language tikz")),
    }
  }

  fn instructions(&self) -> String {
    match self {
      Self::Circuit(parts) => circuit::instructions(parts),
      Self::Chemistry(scheme) => chemistry::instructions(scheme),
      Self::Block(diagram, language) => block::instructions(diagram, *language),
    }
  }

  fn check(&self, source: &str) -> Result<(), String> {
    match self {
      Self::Circuit(parts) => circuit::check(source, parts),
      Self::Chemistry(scheme) => chemistry::check(source, scheme),
      Self::Block(diagram, language) => block::check(source, diagram, *language),
    }
  }
}

pub struct FigureNode {
  id: String,
}

impl FigureNode {
  pub fn new(id: impl Into<String>) -> Self {
    Self { id: id.into() }
  }

  fn input_error(&self, message: impl std::fmt::Display) -> YanshiError {
    YanshiError::NodeInputError {
      message: format!("{}: {message}", self.id),
    }
  }
}

fn string_input<'a>(inputs: &'a AsyncNodeInputs, key: &str) -> Option<&'a str> {
  match inputs.get(key) {
    Some(FlowValue::Json(Value::String(s))) if !s.trim().is_empty() => Some(s.as_str()),
    _ => None,
  }
}

/// The source inside the reply's fenced code block (stripping any `<think>`
/// reasoning prefix and preferring language-tagged fences over scratch fences),
/// or the whole reply when there is no fence.
fn extract_source(reply: &str) -> String {
  let stripped = strip_think_blocks(reply);
  let segments: Vec<&str> = stripped.split("```").collect();
  if segments.len() >= 2 {
    let mut first_body: Option<String> = None;
    for block in segments.iter().skip(1).step_by(2) {
      let (tag, body) = match block.split_once('\n') {
        Some((info, rest)) if !info.contains('<') && !info.contains('\\') => {
          (info.trim().to_ascii_lowercase(), rest.trim())
        }
        _ => (String::new(), block.trim()),
      };
      if body.is_empty() {
        continue;
      }
      if matches!(
        tag.as_str(),
        "d2" | "svg" | "xml" | "tikz" | "latex" | "tex" | "typst" | "typ" | "python" | "py"
      ) {
        return body.to_string();
      }
      if first_body.is_none() {
        first_body = Some(body.to_string());
      }
    }
    if let Some(body) = first_body {
      return body;
    }
  }
  stripped.trim().to_string()
}

fn strip_think_blocks(s: &str) -> String {
  let mut cur = s.to_string();
  while let Some(start) = cur.find("<think>") {
    match cur[start..].find("</think>") {
      Some(rel_end) => {
        let after = start + rel_end + "</think>".len();
        cur = format!("{}{}", &cur[..start], &cur[after..]);
      }
      None => break,
    }
  }
  cur
}

fn prompt_for(language: Language, description: &str, failed: Option<(&str, &str)>) -> String {
  let mut prompt = format!("Draw this figure:\n\n{description}\n");
  if let Some((source, error)) = failed {
    prompt.push_str(&format!(
      "\nYour previous source did not render.\n\nSource:\n```\n{source}\n```\n\n\
       Renderer error:\n```\n{error}\n```\n\nReturn the corrected, complete {} source.\n",
      language.as_str()
    ));
  }
  prompt
}

/// The `!`-lines of a LaTeX log and the two lines after each: the part
/// of a failed lualatex run a model can act on.
fn latex_errors(log: &str) -> Option<String> {
  let lines: Vec<&str> = log.lines().collect();
  let mut picked = Vec::new();
  for (i, line) in lines.iter().enumerate() {
    if line.starts_with('!') {
      picked.extend(lines[i..lines.len().min(i + 3)].iter().copied());
    }
  }
  (!picked.is_empty()).then(|| picked.join("\n"))
}

/// `text` with the work directory taken out of file paths, so an error
/// reads `fig.d2:2:1: ...` (d2 prints absolute paths, and on macOS the
/// temp dir may appear in its canonical `/private` form).
fn strip_workdir(text: &str, workdir: &Path) -> String {
  let mut dirs = vec![workdir.to_path_buf()];
  if let Ok(canonical) = workdir.canonicalize() {
    dirs.push(canonical);
  }
  // Longest first: the canonical form can contain the other one.
  dirs.sort_by_key(|dir| std::cmp::Reverse(dir.as_os_str().len()));
  dirs.iter().fold(text.to_string(), |text, dir| {
    text.replace(&format!("{}/", dir.display()), "")
  })
}

/// typst's package cache and data directories (read-only in the sandbox).
fn typst_package_dirs() -> Vec<PathBuf> {
  let mut dirs = Vec::new();
  if let Some(cache) = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from) {
    dirs.push(cache.join("typst"));
  }
  if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
    dirs.extend([
      home.join("Library/Caches/typst"),
      home.join("Library/Application Support/typst"),
      home.join(".cache/typst"),
      home.join(".local/share/typst"),
    ]);
  }
  dirs
}

/// Outcome of one render: the SVG, or an error message for the model.
type Rendered = Result<String, String>;

impl FigureNode {
  async fn render(&self, language: Language, source: &str) -> Result<Rendered, YanshiError> {
    if language == Language::Svg {
      return Ok(yanshi_nodes::nodes::render::check_svg(source).map(|()| source.to_string()));
    }
    let node_dir = artifacts::artifact_dir(&self.id);
    let workdir = node_dir.join(format!(".figure-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&workdir)
      .await
      .map_err(|e| self.input_error(format!("cannot create {}: {e}", workdir.display())))?;
    let result = match language {
      Language::Tikz => self.render_tikz(&node_dir, &workdir, source).await,
      Language::Typst => self.render_typst(&workdir, source).await,
      Language::D2 => self.render_d2(&workdir, source).await,
      Language::Python => self.render_python(&workdir, source).await,
      Language::Svg => unreachable!("handled before the work directory is created"),
    };
    let _ = tokio::fs::remove_dir_all(&workdir).await;
    if let Ok(Ok(ref svg)) = result
      && let Err(e) = yanshi_nodes::nodes::render::check_svg(svg)
    {
      return Ok(Err(format!("rendered SVG failed XML validation: {e}")));
    }
    result
  }

  fn tool(&self, name: &str) -> Result<PathBuf, YanshiError> {
    sandboxed::find_tool(name).ok_or_else(|| YanshiError::NodeExecutionFailed {
      message: format!(
        "{}: {name} is not installed (`yanshi doctor` lists renderer binaries)",
        self.id
      ),
    })
  }

  async fn render_tikz(
    &self,
    node_dir: &Path,
    workdir: &Path,
    source: &str,
  ) -> Result<Rendered, YanshiError> {
    let lualatex = self.tool("lualatex")?;
    let pdftocairo = self.tool("pdftocairo")?;
    tokio::fs::write(workdir.join("fig.tex"), source)
      .await
      .map_err(|e| self.input_error(format!("cannot stage the source: {e}")))?;
    // lualatex's font cache: one per run (the artifacts dir), so only the
    // first tikz figure of a run pays for building it.
    let texmf_var = node_dir.parent().unwrap_or(node_dir).join(".texmf-var");
    tokio::fs::create_dir_all(&texmf_var)
      .await
      .map_err(|e| self.input_error(format!("cannot create {}: {e}", texmf_var.display())))?;
    let latex = sandboxed::run(sandboxed::SandboxedRun {
      node: &self.id,
      program: lualatex,
      args: [
        "-interaction=nonstopmode",
        "-halt-on-error",
        "-no-shell-escape",
        "fig.tex",
      ]
      .map(Into::into)
      .to_vec(),
      workdir,
      read_paths: sandboxed::font_dirs(),
      write_paths: vec![texmf_var.clone()],
      env: vec![("TEXMFVAR", texmf_var.into_os_string())],
      timeout: RENDER_TIMEOUT,
    })
    .await?;
    if !latex.status.success() {
      let log = tokio::fs::read_to_string(workdir.join("fig.log"))
        .await
        .unwrap_or_default();
      return Ok(Err(
        latex_errors(&log).unwrap_or_else(|| sandboxed::output_tail(&latex, 20)),
      ));
    }
    let cairo = sandboxed::run(sandboxed::SandboxedRun {
      node: &self.id,
      program: pdftocairo,
      args: ["-svg", "fig.pdf", "fig.svg"].map(Into::into).to_vec(),
      workdir,
      read_paths: sandboxed::font_dirs(),
      write_paths: Vec::new(),
      env: Vec::new(),
      timeout: RENDER_TIMEOUT,
    })
    .await?;
    if !cairo.status.success() {
      return Ok(Err(format!(
        "pdftocairo failed: {}",
        sandboxed::output_tail(&cairo, 10)
      )));
    }
    self.read_svg(workdir).await
  }

  async fn render_typst(&self, workdir: &Path, source: &str) -> Result<Rendered, YanshiError> {
    let typst = self.tool("typst")?;
    tokio::fs::write(workdir.join("fig.typ"), source)
      .await
      .map_err(|e| self.input_error(format!("cannot stage the source: {e}")))?;
    let mut read_paths = sandboxed::font_dirs();
    read_paths.extend(typst_package_dirs());
    let run = sandboxed::run(sandboxed::SandboxedRun {
      node: &self.id,
      program: typst,
      args: ["compile", "fig.typ", "fig.svg"].map(Into::into).to_vec(),
      workdir,
      read_paths,
      write_paths: Vec::new(),
      env: Vec::new(),
      timeout: RENDER_TIMEOUT,
    })
    .await?;
    if !run.status.success() {
      return Ok(Err(sandboxed::output_tail(&run, 20)));
    }
    self.read_svg(workdir).await
  }

  async fn render_d2(&self, workdir: &Path, source: &str) -> Result<Rendered, YanshiError> {
    let d2 = self.tool("d2")?;
    // 1. Try compiling the exact raw `source` first so valid D2 (including
    //    single-quoted strings, globs, or nested styles) is never mutated.
    tokio::fs::write(workdir.join("fig.d2"), source)
      .await
      .map_err(|e| self.input_error(format!("cannot stage the source: {e}")))?;
    let raw_run = sandboxed::run(sandboxed::SandboxedRun {
      node: &self.id,
      program: d2.clone(),
      args: ["fig.d2", "fig.svg"].map(Into::into).to_vec(),
      workdir,
      read_paths: sandboxed::font_dirs(),
      write_paths: Vec::new(),
      env: Vec::new(),
      timeout: RENDER_TIMEOUT,
    })
    .await?;
    if raw_run.status.success() {
      return self.read_svg(workdir).await;
    }

    // 2. Fall back to `sanitize_d2_source` when raw `source` contains common
    //    Mermaid/PlantUML/LLM syntax slip-ups.
    let cleaned = sanitize_d2_source(source);
    if cleaned != source {
      tokio::fs::write(workdir.join("fig.d2"), &cleaned)
        .await
        .map_err(|e| self.input_error(format!("cannot stage the source: {e}")))?;
      let cleaned_run = sandboxed::run(sandboxed::SandboxedRun {
        node: &self.id,
        program: d2,
        args: ["fig.d2", "fig.svg"].map(Into::into).to_vec(),
        workdir,
        read_paths: sandboxed::font_dirs(),
        write_paths: Vec::new(),
        env: Vec::new(),
        timeout: RENDER_TIMEOUT,
      })
      .await?;
      if cleaned_run.status.success() {
        return self.read_svg(workdir).await;
      }
    }

    // Report raw `source` errors so line numbers match the model's own source.
    Ok(Err(strip_workdir(
      &sandboxed::output_tail(&raw_run, 100),
      workdir,
    )))
  }

  /// Final-attempt D2 salvage: when all LLM retries are exhausted and `d2`
  /// reports specific offending `fig.d2:<line>:` line numbers, comment out
  /// those broken lines (re-balancing braces) as long as at least 4 active
  /// diagram lines remain.
  async fn salvage_d2(
    &self,
    source: &str,
    initial_error: &str,
  ) -> Result<Option<(String, String)>, YanshiError> {
    let mut current = source.to_string();
    let mut last_err = initial_error.to_string();
    for _ in 0..20 {
      let bad_lines = extract_d2_error_lines(&last_err);
      if bad_lines.is_empty() {
        return Ok(None);
      }
      let mut lines: Vec<String> = current.lines().map(str::to_string).collect();
      let mut changed = false;
      for line_no in bad_lines {
        if let Some(slot) = line_no.checked_sub(1).and_then(|idx| lines.get_mut(idx)) {
          let ts = slot.trim_start();
          if !ts.is_empty() && !ts.starts_with('#') {
            let indent = &slot[..slot.len() - ts.len()];
            *slot = format!("{indent}# {ts}");
            changed = true;
          }
        }
      }
      if !changed {
        return Ok(None);
      }
      let balanced = balance_d2_braces(&lines);
      let active_count = balanced
        .iter()
        .map(|l| l.trim())
        .filter(|t| !t.is_empty() && !t.starts_with('#') && *t != "{" && *t != "}")
        .count();
      if active_count < 4 {
        return Ok(None);
      }
      current = balanced.join("\n");
      match self.render(Language::D2, &current).await? {
        Ok(svg) => return Ok(Some((svg, current))),
        Err(next_err) => last_err = next_err,
      }
    }
    Ok(None)
  }

  /// Run the script in the code_exec container tier. Unlike the other
  /// renderers there is no degraded mode: without a container engine the
  /// node fails rather than running model-written code on the host.
  async fn render_python(&self, workdir: &Path, source: &str) -> Result<Rendered, YanshiError> {
    use yanshi_tools::{
      Capability,
      sandbox::{ContainerBackend, SandboxBackend, SandboxScope},
    };
    let image = std::env::var("YANSHI_FIGURE_PYTHON_IMAGE")
      .ok()
      .filter(|v| !v.trim().is_empty())
      .unwrap_or_else(|| DEFAULT_PYTHON_IMAGE.to_string());
    let backend = ContainerBackend::new(image);
    if !backend.is_enforcing() {
      return Err(YanshiError::NodeExecutionFailed {
        message: format!(
          "{}: language python needs a container engine (Apple `container` or rootless Podman); \
           `yanshi doctor` reports it",
          self.id
        ),
      });
    }
    tokio::fs::write(workdir.join("fig.py"), source)
      .await
      .map_err(|e| self.input_error(format!("cannot stage the script: {e}")))?;
    let canonical = workdir
      .canonicalize()
      .map_err(|e| self.input_error(format!("cannot resolve {}: {e}", workdir.display())))?;
    let scope = SandboxScope::new()
      .with_working_directory(&canonical)
      .with_max_memory_bytes(1024 * 1024 * 1024)
      .with_max_cpu_secs(120)
      .with_max_pids(64)
      .with_container_name(format!("yanshi-figure-{}", uuid::Uuid::new_v4()));
    let mut command = tokio::process::Command::new("python3");
    command
      .arg("fig.py")
      .current_dir(&canonical)
      .kill_on_drop(true);
    backend
      .wrap_command(
        &mut command,
        &[Capability::FsRead, Capability::FsWrite, Capability::Exec],
        &scope,
      )
      .map_err(|e| YanshiError::NodeExecutionFailed {
        message: format!("{}: preparing the container failed: {e}", self.id),
      })?;
    let output = match tokio::time::timeout(PYTHON_TIMEOUT, command.output()).await {
      Ok(output) => output.map_err(|e| YanshiError::NodeExecutionFailed {
        message: format!("{}: starting the container failed: {e}", self.id),
      })?,
      Err(_) => {
        backend.terminate(&scope);
        return Ok(Err(format!(
          "the script ran longer than {}s",
          PYTHON_TIMEOUT.as_secs()
        )));
      }
    };
    if !output.status.success() {
      return Ok(Err(sandboxed::output_tail(&output, 25)));
    }
    match tokio::fs::read_to_string(canonical.join("figure.svg")).await {
      Ok(svg) if !svg.trim().is_empty() => Ok(Ok(svg)),
      Ok(_) => Ok(Err("the script wrote an empty figure.svg".to_string())),
      Err(_) => Ok(Err(
        "the script finished but wrote no ./figure.svg".to_string(),
      )),
    }
  }

  async fn read_svg(&self, workdir: &Path) -> Result<Rendered, YanshiError> {
    match tokio::fs::read_to_string(workdir.join("fig.svg")).await {
      Ok(svg) if !svg.trim().is_empty() => Ok(Ok(svg)),
      Ok(_) => Ok(Err("the renderer produced an empty SVG".to_string())),
      Err(e) => Ok(Err(format!("the renderer produced no SVG: {e}"))),
    }
  }

  async fn generate(
    &self,
    model: &str,
    prompt: String,
    language: Language,
    domain: Option<&str>,
  ) -> Result<String, YanshiError> {
    Yanshi::ensure_init()
      .await
      .map_err(|e| YanshiError::ConfigurationError {
        message: format!("{}: failed to initialize Yanshi LLM service: {e}", self.id),
      })?;
    let mut system = format!(
      "You draw figures by writing source code. {} Reply with exactly one fenced code block \
       containing the complete source and nothing else.",
      language.instructions()
    );
    if let Some(domain) = domain {
      system.push_str("\n\n");
      system.push_str(domain);
    }
    let max_tokens = if language == Language::D2 { 2048 } else { 4096 };
    let response = Yanshi::model(model)
      .system(&system)
      .prompt(&prompt)
      .max_tokens(max_tokens)
      .thinking(yanshi_llm::ThinkingConfig::Disabled)
      .execute_full()
      .await
      .map_err(|e| YanshiError::NodeExecutionFailed {
        message: format!("{}: the model call failed: {e}", self.id),
      })?;
    crate::cost::record(model, |p| match &response.usage {
      Some(u) => p.cost_for_call(u.prompt_tokens, u.completion_tokens),
      None => 0.0,
    })?;
    Ok(extract_source(&response.content))
  }
}

#[async_trait]
impl AsyncNode for FigureNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let language = string_input(inputs, "language").ok_or_else(|| {
      self.input_error("missing input 'language' (svg, tikz, typst, d2 or python)")
    })?;
    let language = Language::parse(language.trim()).ok_or_else(|| {
      self.input_error(format!(
        "unsupported language '{language}': expected svg, tikz, typst, d2 or python"
      ))
    })?;
    let description = string_input(inputs, "description");
    let model = string_input(inputs, "model");
    let max_attempts = match inputs.get("max_attempts") {
      Some(FlowValue::Json(Value::Number(n))) => n.as_u64().unwrap_or(DEFAULT_MAX_ATTEMPTS),
      Some(FlowValue::Json(Value::String(s))) => s.trim().parse().unwrap_or(DEFAULT_MAX_ATTEMPTS),
      _ => DEFAULT_MAX_ATTEMPTS,
    }
    .clamp(1, 10);
    // Y6.5 / Y8.4: a domain's spec, checked before every render.
    let domain = match string_input(inputs, "domain").map(str::trim) {
      None => None,
      Some(name) => {
        let spec = match inputs.get("spec") {
          Some(FlowValue::Json(Value::String(text))) if !text.trim().is_empty() => Some(
            serde_json::from_str::<Value>(text)
              .map_err(|e| self.input_error(format!("spec is not JSON: {e}")))?,
          ),
          Some(FlowValue::Json(Value::String(_))) | None => None,
          Some(FlowValue::Json(value)) => Some(value.clone()),
          Some(_) => return Err(self.input_error("spec must be JSON")),
        };
        Some(Domain::parse(name, language, spec.as_ref()).map_err(|e| self.input_error(e))?)
      }
    };
    let domain_text = domain.as_ref().map(Domain::instructions);
    let can_generate = description.is_some();
    if string_input(inputs, "source").is_none() && !can_generate {
      return Err(self.input_error("give 'source' or 'description'"));
    }

    let mut source = match string_input(inputs, "source") {
      Some(source) => source.to_string(),
      None => {
        let prompt = prompt_for(language, description.unwrap_or_default(), None);
        self
          .generate(
            model.unwrap_or_default(),
            prompt,
            language,
            domain_text.as_deref(),
          )
          .await?
      }
    };
    let mut attempt: u64 = 1;
    let svg = loop {
      let rendered = match domain.as_ref().map(|domain| domain.check(&source)) {
        Some(Err(violations)) => Err(violations),
        _ => self.render(language, &source).await?,
      };
      match rendered {
        Ok(svg) => break svg,
        Err(error) if attempt < max_attempts && can_generate => {
          run_context::report_progress(
            "figure_retry",
            json!({ "attempt": attempt, "language": language.as_str(), "error": error }),
          );
          let prompt = prompt_for(
            language,
            description.unwrap_or_default(),
            Some((&source, &error)),
          );
          source = self
            .generate(
              model.unwrap_or_default(),
              prompt,
              language,
              domain_text.as_deref(),
            )
            .await?;
          attempt += 1;
        }
        Err(error) => {
          if language == Language::D2
            && domain.is_none()
            && let Some((salvaged_svg, salvaged_src)) = self.salvage_d2(&source, &error).await?
          {
            source = salvaged_src;
            break salvaged_svg;
          }
          return Err(YanshiError::NodeExecutionFailed {
            message: format!(
              "{}: the {} source did not render after {attempt} attempt(s): {error}",
              self.id,
              language.as_str()
            ),
          });
        }
      }
    };

    let mut outputs = HashMap::from([
      (
        format!("{}_svg", self.id),
        artifacts::write_artifact(&self.id, svg.as_bytes(), "image/svg+xml").await?,
      ),
      (
        format!("{}_source", self.id),
        FlowValue::Json(Value::String(source)),
      ),
      (
        format!("{}_attempts", self.id),
        FlowValue::Json(json!(attempt)),
      ),
    ]);
    if matches!(inputs.get("png"), Some(FlowValue::Json(Value::Bool(true)))) {
      let width = match inputs.get("png_width") {
        Some(FlowValue::Json(Value::Number(n))) => n.as_u64().and_then(|w| u32::try_from(w).ok()),
        _ => None,
      };
      let png =
        tokio::task::spawn_blocking(move || yanshi_nodes::nodes::render::svg_to_png(&svg, width))
          .await
          .map_err(|e| YanshiError::NodeExecutionFailed {
            message: format!("{}: PNG task failed: {e}", self.id),
          })?
          .map_err(|e| YanshiError::NodeExecutionFailed {
            message: format!("{}: PNG copy failed: {e}", self.id),
          })?;
      outputs.insert(
        format!("{}_png", self.id),
        artifacts::write_artifact(&self.id, &png, "image/png").await?,
      );
    }
    Ok(outputs)
  }
}

/// Normalize common LLM D2 syntax slip-ups before invoking `d2`:
/// - Convert leading or unquoted trailing `//` comments to D2 `#` comments (or strip inline `//`).
/// - Normalize hallucinated database shape aliases (`database`, `db`, `sql`) to `cylinder`
///   without touching valid `sql_table`.
/// - Normalize underscore variants of hyphenated D2 style/arrowhead keys and strip invalid
///   non-reserved `{ ... }` maps attached to edge declarations (`edge map keys must be reserved keywords`).
fn sanitize_d2_source(source: &str) -> String {
  let collapsed = join_d2_continuation_arrows(&collapse_d2_block_strings(source));
  let mut cleaned_lines = Vec::new();
  let mut brace_depth = 0_i32;
  for raw_line in collapsed.lines() {
    let trimmed_start = raw_line.trim_start();
    if let Some(rest) = trimmed_start.strip_prefix("//") {
      let indent = &raw_line[..raw_line.len() - trimmed_start.len()];
      cleaned_lines.push(format!("{indent}#{rest}"));
      continue;
    }
    if trimmed_start.starts_with('#') {
      cleaned_lines.push(raw_line.to_string());
      continue;
    }
    if trimmed_start.starts_with("---") || trimmed_start.starts_with("-- ") {
      let indent = &raw_line[..raw_line.len() - trimmed_start.len()];
      cleaned_lines.push(format!("{indent}# {trimmed_start}"));
      continue;
    }
    if trimmed_start.starts_with('<')
      && trimmed_start
        .as_bytes()
        .get(1)
        .is_some_and(|b| b.is_ascii_alphabetic() || matches!(b, b'/' | b'!' | b'?'))
    {
      let indent = &raw_line[..raw_line.len() - trimmed_start.len()];
      cleaned_lines.push(format!("{indent}# {trimmed_start}"));
      continue;
    }

    // Strip unquoted inline ` // ...` comments (preceded by whitespace so URLs stay intact).
    let mut in_double = false;
    let mut in_single = false;
    let mut escaped = false;
    let mut cut_at: Option<usize> = None;
    let bytes = raw_line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
      if escaped {
        escaped = false;
        continue;
      }
      if b == b'\\' && (in_double || in_single) {
        escaped = true;
        continue;
      }
      if b == b'"' && !in_single {
        in_double = !in_double;
        continue;
      }
      if b == b'\'' && !in_double {
        in_single = !in_single;
        continue;
      }
      if !in_double
        && !in_single
        && b == b'/'
        && bytes.get(i + 1) == Some(&b'/')
        && i > 0
        && bytes[i - 1].is_ascii_whitespace()
      {
        cut_at = Some(i);
        break;
      }
    }
    let line = match cut_at {
      Some(idx) => raw_line[..idx].trim_end(),
      None => raw_line,
    };

    let mermaid_fixed = normalize_d2_mermaidisms_and_hex(line);
    // Replace JSON/CSS-style `, key:` separators inside `{ ... }` maps with `; key:`.
    let comma_fixed = replace_d2_map_commas(&mermaid_fixed, &mut brace_depth);
    let mut normalized = normalize_d2_shapes_in_line(&comma_fixed);

    for (bad, good) in [
      ("source_arrowhead", "source-arrowhead"),
      ("target_arrowhead", "target-arrowhead"),
      ("stroke_dash", "stroke-dash"),
      ("stroke_width", "stroke-width"),
      ("font_size", "font-size"),
      ("border_radius", "border-radius"),
    ] {
      if normalized.contains(bad) {
        normalized = normalized.replace(bad, good);
      }
    }

    let after_map = fix_d2_trailing_text_after_map(&normalized);
    cleaned_lines.push(quote_unquoted_d2_special_kv(&after_map));
  }

  let edge_cleaned = sanitize_d2_edge_maps(&cleaned_lines);
  balance_d2_braces(&edge_cleaned).join("\n")
}

/// Merge wrapped continuation edges (`\n  -> target: "lbl"`) or wrapped `(arg)`
/// lines onto the preceding line so D2 never fails with `connection missing source`.
fn join_d2_continuation_arrows(source: &str) -> String {
  let mut out: Vec<String> = Vec::new();
  for line in source.lines() {
    let ts = line.trim_start();
    let te = ts.trim_end();
    let starts_with_arrow = ts.starts_with("->") || ts.starts_with("<->") || ts.starts_with("<-");
    let is_wrapped_paren_note = line.starts_with(' ')
      && te.starts_with('(')
      && te.ends_with(')')
      && !te.contains('{')
      && !te.contains('}')
      && !has_unquoted_d2_arrow(te);
    if starts_with_arrow || is_wrapped_paren_note {
      if let Some(prev) = out.last_mut() {
        let pt = prev.trim();
        if !pt.is_empty()
          && !pt.starts_with('#')
          && !pt.ends_with('{')
          && !pt.ends_with('}')
          && !pt.ends_with(';')
        {
          prev.push(' ');
          prev.push_str(te);
          continue;
        }
      }
      if starts_with_arrow {
        let indent = &line[..line.len() - ts.len()];
        out.push(format!("{indent}# {ts}"));
        continue;
      }
    }
    out.push(line.to_string());
  }
  out.join("\n")
}

fn is_simple_d2_bare_atom(s: &str) -> bool {
  !s.is_empty()
    && s
      .bytes()
      .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    && !matches!(s.as_bytes()[0], b'-' | b'.')
}

/// Quote unquoted D2 keys/values that contain Rust/SQL/HTTP punctuation (`<`, `>`,
/// `&`, `(`, `)`, `[`, `]`, `/`, `\`, `:`, `+`, `,`, spaces) so D2 never misparses
/// method signatures (`+create(ctx: &Ctx): Result<T>`) or REST routes (`POST /v1/:id`)
/// as nested map keys or invalid tokens.
fn quote_unquoted_d2_special_kv(line: &str) -> String {
  let trimmed = line.trim();
  if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "{" || trimmed == "}" {
    return line.to_string();
  }

  // Single-line edge with unquoted label containing `;` or payload `{...}`
  // (e.g. `order_svc -> pg: BEGIN; INSERT orders; COMMIT` or
  // `redis -> inv: {1, "SUCCESS"} / {-2, "OUT_OF_STOCK"}` or
  // `order_svc -> seckill: Order {status: CREATED}`).
  if brace_delta_unquoted(line) == 0
    && let Some(colon_idx) = find_top_level_unquoted_colon(trimmed)
  {
    let lhs = trimmed[..colon_idx].trim();
    let rhs = trimmed[colon_idx + 1..].trim();
    if has_unquoted_d2_arrow(lhs) && !rhs.is_empty() && !rhs.starts_with('"') {
      let is_reserved_map =
        if let Some(inner) = rhs.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
          find_unquoted_brace(inner).is_none() && edge_block_entries_all_valid(inner)
        } else {
          false
        };
      if !is_reserved_map {
        // Check if `rhs` ends with a valid reserved edge map `label_part { style... }`.
        if let Some(open_b) = find_unquoted_brace(rhs)
          && let Some(inner) = rhs[open_b + 1..].strip_suffix('}')
          && find_unquoted_brace(inner).is_none()
          && edge_block_entries_all_valid(inner)
        {
          let label_part = rhs[..open_b].trim();
          let indent = &line[..line.len() - line.trim_start().len()];
          return format!(
            "{indent}{lhs}: \"{}\" {{{inner}}}",
            escape_d2_double_quoted(label_part)
          );
        }
        if !is_simple_d2_bare_atom(rhs) {
          let indent = &line[..line.len() - line.trim_start().len()];
          return format!("{indent}{lhs}: \"{}\"", escape_d2_double_quoted(rhs));
        }
      }
    }
  }

  // Single-line map `id: { shape: rectangle; label: foo/bar }`: sanitize inner `;` segments.
  if let Some(open_b) = find_unquoted_brace(line) {
    let after_open = &line[open_b + 1..];
    if find_unquoted_brace(after_open).is_none()
      && let Some(rel_close) = find_unquoted_close_brace(after_open)
    {
      let head = &line[..open_b];
      let inner = &after_open[..rel_close];
      let tail = &after_open[rel_close + 1..];
      if !has_unquoted_d2_arrow(head) && !has_unquoted_d2_arrow(inner) {
        let fixed_segs: Vec<String> = inner
          .split(';')
          .map(quote_unquoted_d2_scalar_stmt)
          .collect();
        return format!("{head}{{{}}}{tail}", fixed_segs.join(";"));
      }
    }
    return line.to_string();
  }

  if find_unquoted_close_brace(line).is_some() || has_unquoted_semicolon(line) {
    return line.to_string();
  }

  quote_unquoted_d2_scalar_stmt(line)
}

fn quote_unquoted_d2_scalar_stmt(stmt: &str) -> String {
  let ts = stmt.trim_start();
  if ts.is_empty() || ts.starts_with('#') {
    return stmt.to_string();
  }
  let indent = &stmt[..stmt.len() - ts.len()];
  let te = ts.trim_end();
  let trailing_ws = &ts[te.len()..];

  if let Some(colon_idx) = find_top_level_unquoted_colon(te) {
    let lhs_raw = te[..colon_idx].trim();
    let lhs = lhs_raw
      .strip_prefix('\'')
      .and_then(|x| x.strip_suffix('\''))
      .unwrap_or(lhs_raw);
    let rhs_raw = te[colon_idx + 1..].trim();
    let rhs = rhs_raw
      .strip_prefix('\'')
      .and_then(|x| x.strip_suffix('\''))
      .unwrap_or(rhs_raw);
    let lhs_is_edge = has_unquoted_d2_arrow(lhs);
    let fixed_lhs = if lhs_is_edge || lhs.starts_with('"') || is_simple_d2_bare_atom(lhs) {
      lhs.to_string()
    } else {
      format!("\"{}\"", escape_d2_double_quoted(lhs))
    };

    if rhs.is_empty() {
      return format!("{indent}{fixed_lhs}:{trailing_ws}");
    }
    let fixed_rhs = if let Some(after_open_q) = rhs.strip_prefix('"') {
      if let Some(end_q) = find_closing_quote(after_open_q) {
        let after_q = after_open_q[end_q + 1..].trim();
        if after_q.is_empty() {
          rhs.to_string()
        } else {
          let inner = &after_open_q[..end_q];
          let merged_tail = escape_d2_double_quoted(after_q);
          format!("\"{inner} {merged_tail}\"")
        }
      } else {
        format!("\"{}\"", escape_d2_double_quoted(rhs.trim_matches('"')))
      }
    } else if is_simple_d2_bare_atom(rhs) && rhs_raw == rhs {
      rhs.to_string()
    } else {
      format!("\"{}\"", escape_d2_double_quoted(rhs))
    };
    return format!("{indent}{fixed_lhs}: {fixed_rhs}{trailing_ws}");
  }

  // Statement without top-level `:`: keep edges and simple bare identifiers as-is,
  // quote UML/prose lines starting with `+`/`-` or containing parens/angle-brackets.
  if has_unquoted_d2_arrow(te) || te.starts_with('"') || is_simple_d2_bare_atom(te) {
    return stmt.to_string();
  }
  format!("{indent}\"{}\"{trailing_ws}", escape_d2_double_quoted(te))
}

fn escape_d2_double_quoted(raw: &str) -> String {
  let mut out = String::with_capacity(raw.len());
  let chars: Vec<char> = raw.chars().collect();
  let mut i = 0;
  while i < chars.len() {
    let c = chars[i];
    if c == '\\' && i + 1 < chars.len() {
      let next = chars[i + 1];
      if matches!(next, 'n' | 't' | '\\' | '"' | '$') {
        out.push('\\');
        out.push(next);
        i += 2;
        continue;
      }
      out.push('\\');
      out.push('\\');
      i += 1;
      continue;
    }
    if c == '"' {
      out.push('\'');
    } else {
      out.push(c);
    }
    i += 1;
  }
  out
}

fn find_top_level_unquoted_colon(s: &str) -> Option<usize> {
  let mut in_quotes = false;
  let mut esc = false;
  let mut paren_depth = 0_i32;
  let mut bracket_depth = 0_i32;
  for (i, b) in s.bytes().enumerate() {
    if esc {
      esc = false;
      continue;
    }
    if b == b'\\' && in_quotes {
      esc = true;
      continue;
    }
    if b == b'"' {
      in_quotes = !in_quotes;
      continue;
    }
    if !in_quotes {
      match b {
        b'(' => paren_depth += 1,
        b')' => paren_depth = (paren_depth - 1).max(0),
        b'[' => bracket_depth += 1,
        b']' => bracket_depth = (bracket_depth - 1).max(0),
        b':' if paren_depth == 0 && bracket_depth == 0 => return Some(i),
        _ => {}
      }
    }
  }
  None
}

fn find_unquoted_close_brace(s: &str) -> Option<usize> {
  let mut in_quotes = false;
  let mut esc = false;
  for (i, b) in s.bytes().enumerate() {
    if esc {
      esc = false;
      continue;
    }
    if b == b'\\' && in_quotes {
      esc = true;
      continue;
    }
    if b == b'"' {
      in_quotes = !in_quotes;
      continue;
    }
    if !in_quotes && b == b'}' {
      return Some(i);
    }
  }
  None
}

fn has_unquoted_semicolon(s: &str) -> bool {
  let mut in_quotes = false;
  let mut esc = false;
  for b in s.bytes() {
    if esc {
      esc = false;
      continue;
    }
    if b == b'\\' && in_quotes {
      esc = true;
      continue;
    }
    if b == b'"' {
      in_quotes = !in_quotes;
      continue;
    }
    if !in_quotes && b == b';' {
      return true;
    }
  }
  false
}

fn balance_d2_braces(lines: &[String]) -> Vec<String> {
  let mut out = Vec::with_capacity(lines.len() + 2);
  let mut depth = 0_i32;
  for line in lines {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
      out.push(line.clone());
      continue;
    }
    let delta = brace_delta_unquoted(line);
    if depth + delta < 0 {
      if trimmed == "}" {
        continue;
      }
      let indent = &line[..line.len() - trimmed.len()];
      out.push(format!("{indent}# {trimmed}"));
      continue;
    }
    depth += delta;
    out.push(line.clone());
  }
  while depth > 0 {
    out.push("}".to_string());
    depth -= 1;
  }
  out
}

/// Convert multi-line D2 `|md ... |` / `| ... |` block strings into single-line
/// escaped double-quoted strings so unterminated, Markdown-table-bearing, or
/// inline block strings never fail D2 parsing.
fn collapse_d2_block_strings(source: &str) -> String {
  let raw_lines: Vec<&str> = source.lines().collect();
  let mut out = Vec::with_capacity(raw_lines.len());
  let mut idx = 0;
  while idx < raw_lines.len() {
    let line = raw_lines[idx];
    if let Some((prefix, after_colon)) = split_unquoted_colon_for_block(line) {
      let trimmed = after_colon.trim();
      let pipe_count = trimmed.bytes().take_while(|&b| b == b'|').count();
      if pipe_count >= 1 {
        let close_delim = "|".repeat(pipe_count);
        let after_pipes = &trimmed[pipe_count..];
        let lang_end = after_pipes
          .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
          .unwrap_or(after_pipes.len());
        let after_lang = after_pipes[lang_end..].trim();
        // Single-line `key: |md text |`
        if !after_lang.is_empty()
          && !after_lang.starts_with('|')
          && let Some(inline_body) = after_lang.strip_suffix(&close_delim)
        {
          let escaped = inline_body
            .trim()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
          out.push(format!("{prefix}: \"{escaped}\""));
          idx += 1;
          continue;
        }
        // If `after_lang` has text on the opening line without a closing delimiter on a
        // subsequent line, quote the single line rather than swallowing the file.
        let has_closing_line = after_lang.is_empty()
          || raw_lines[idx + 1..].iter().any(|cand| {
            let ct = cand.trim();
            ct == close_delim
              || ct.starts_with(&format!("{close_delim};"))
              || ct.starts_with(&format!("{close_delim}}}"))
              || (!ct.starts_with('|') && ct.ends_with(&close_delim))
          });
        if !has_closing_line {
          let escaped = after_lang.replace('\\', "\\\\").replace('"', "\\\"");
          out.push(format!("{prefix}: \"{escaped}\""));
          idx += 1;
          continue;
        }
        // Multi-line `key: |md\n ... \n|`
        let mut body_parts = Vec::new();
        if !after_lang.is_empty() {
          body_parts.push(after_lang.to_string());
        }
        let mut end_idx = idx + 1;
        while end_idx < raw_lines.len() {
          let l = raw_lines[end_idx].trim();
          if l == close_delim
            || l.starts_with(&format!("{close_delim};"))
            || l.starts_with(&format!("{close_delim}}}"))
          {
            end_idx += 1;
            break;
          }
          // Markdown table rows (`| col1 | col2 |` or `|------|------|`) inside a `|md`
          // block string both start and end with `|`; never mistake them for `close_delim`.
          if l.starts_with('|') {
            let is_table_sep = l.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'));
            if !is_table_sep {
              let row = l
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
              if !row.is_empty() {
                body_parts.push(row);
              }
            }
            end_idx += 1;
            continue;
          }
          if let Some(before_close) = l.strip_suffix(&close_delim) {
            if !before_close.trim().is_empty() {
              body_parts.push(before_close.trim().to_string());
            }
            end_idx += 1;
            break;
          }
          if !l.is_empty() {
            body_parts.push(l.to_string());
          }
          end_idx += 1;
        }
        let joined = body_parts
          .join(" / ")
          .replace('\\', "\\\\")
          .replace('"', "\\\"");
        out.push(format!("{prefix}: \"{joined}\""));
        idx = end_idx;
        continue;
      }
    }
    // Any remaining standalone line starting with `|` outside a `key: |` block (e.g. a
    // raw Markdown table appended by an LLM) would trigger D2's custom block-string parser
    // (`block string must be terminated with ------|------||`); comment it out safely.
    let trimmed_start = line.trim_start();
    if trimmed_start.starts_with('|') {
      let indent = &line[..line.len() - trimmed_start.len()];
      out.push(format!("{indent}# {trimmed_start}"));
      idx += 1;
      continue;
    }
    out.push(line.to_string());
    idx += 1;
  }
  out.join("\n")
}

fn split_unquoted_colon_for_block(line: &str) -> Option<(&str, &str)> {
  let mut in_quotes = false;
  let mut escaped = false;
  for (i, b) in line.bytes().enumerate() {
    if escaped {
      escaped = false;
      continue;
    }
    if b == b'\\' && in_quotes {
      escaped = true;
      continue;
    }
    if b == b'"' {
      in_quotes = !in_quotes;
      continue;
    }
    if !in_quotes && b == b':' {
      return Some((&line[..i], &line[i + 1..]));
    }
  }
  None
}

/// Normalize Mermaidisms (`id["label"]`, `-->|label|`, `-->`) and unquoted
/// `#hex` color literals (`style.fill: #e3f2fd` -> `style.fill: "#e3f2fd"`).
fn normalize_d2_mermaidisms_and_hex(line: &str) -> String {
  let mut s = quote_unquoted_d2_hex_colors(line);

  // Mermaid edge label `a -->|label| b` or `a ->|label| b` -> `a -> b: "label"`.
  if let Some(arrow_pos) = find_mermaid_pipe_arrow(&s) {
    let (arrow_idx, pipe_start) = arrow_pos;
    let src = s[..arrow_idx].trim_end();
    let after_first_pipe = &s[pipe_start + 1..];
    if let Some(close_pipe) = after_first_pipe.find('|') {
      let label = after_first_pipe[..close_pipe]
        .trim()
        .trim_matches('"')
        .replace('"', "'");
      let rest = after_first_pipe[close_pipe + 1..].trim_start();
      let dst_end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, ':' | '{' | ';'))
        .unwrap_or(rest.len());
      let dst = &rest[..dst_end];
      let tail = &rest[dst_end..];
      if !src.is_empty() && !dst.is_empty() {
        s = format!("{src} -> {dst}: \"{label}\"{tail}");
      }
    }
  }

  // Normalize PlantUML / Mermaid UML & sequence arrows (`..>`, `--|>`, `->>`, etc.).
  for (from, to) in [
    ("..|>", "->"),
    ("--|>", "->"),
    ("<|..", "<-"),
    ("<|--", "<-"),
    ("-->>", "->"),
    ("<<--", "<-"),
    ("..>", "->"),
    ("<..", "<-"),
    ("->>", "->"),
    ("<<-", "<-"),
    ("--*", "->"),
    ("*--", "<-"),
    ("--o", "->"),
    ("o--", "<-"),
    ("-->", "->"),
  ] {
    if s.contains(from) {
      s = replace_unquoted_token(&s, from, to);
    }
  }

  // Comment out bare PlantUML / Mermaid sequence control keywords (`alt ...`,
  // `else ...`, `opt ...`, `loop ...`, `par ...`, `end`, `activate ...`, `deactivate ...`)
  // when written without D2 `:` or `{` so inner sequence messages still compile.
  let trimmed_kw = s.trim();
  if !trimmed_kw.contains(':')
    && !trimmed_kw.contains('{')
    && !trimmed_kw.contains('}')
    && !has_unquoted_d2_arrow(trimmed_kw)
  {
    let first_word = trimmed_kw.split_whitespace().next().unwrap_or("");
    let is_seq_kw = matches!(trimmed_kw, "end" | "autonumber")
      || (matches!(
        first_word,
        "alt" | "else" | "opt" | "loop" | "par" | "critical" | "break" | "activate" | "deactivate"
      ) && trimmed_kw.len() > first_word.len());
    if is_seq_kw {
      let indent = &s[..s.len() - s.trim_start().len()];
      return format!("{indent}# {trimmed_kw}");
    }
  }

  // PlantUML `"Label" as id` or `"Label" as id: {` -> `id: "Label"` / `id: "Label" {`.
  let trimmed = s.trim_start();
  let indent = &s[..s.len() - trimmed.len()];
  if trimmed.starts_with('"')
    && let Some(end_q) = find_closing_quote(&trimmed[1..]).map(|p| p + 1)
  {
    let label = &trimmed[1..end_q];
    let after_q = trimmed[end_q + 1..].trim_start();
    if let Some(after_as) = after_q.strip_prefix("as ") {
      let after_as = after_as.trim_start();
      let id_end = after_as
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')))
        .unwrap_or(after_as.len());
      let id = &after_as[..id_end];
      let tail = after_as[id_end..].trim_start();
      let tail = tail.strip_prefix(':').map_or(tail, str::trim_start);
      if !id.is_empty() {
        let sep = if tail.is_empty() { "" } else { " " };
        return format!("{indent}{id}: \"{label}\"{sep}{tail}");
      }
    }
  }

  // Mermaid bracketed node `id["label"]: {` / `id[label]: {` / standalone `id[label]`.
  let trimmed = s.trim_start();
  let indent = &s[..s.len() - trimmed.len()];
  if !trimmed.starts_with("classes") && !has_unquoted_d2_arrow(trimmed) {
    let id_end = trimmed
      .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')))
      .unwrap_or(trimmed.len());
    if id_end > 0 {
      let id = &trimmed[..id_end];
      let after_id = trimmed[id_end..].trim_start();
      if let Some(after_open) = after_id.strip_prefix('[')
        && let Some(close_br) = after_open.find(']')
      {
        let raw_lbl = after_open[..close_br]
          .trim()
          .trim_matches('"')
          .replace('"', "'");
        let after_br = after_open[close_br + 1..].trim_start();
        if after_br.is_empty() {
          return format!("{indent}{id}: \"{raw_lbl}\"");
        }
        let after_colon = after_br.strip_prefix(':').map_or(after_br, str::trim_start);
        if after_colon.starts_with('{') {
          return format!("{indent}{id}: \"{raw_lbl}\" {after_colon}");
        }
      }
    } else if let Some(after_open) = trimmed.strip_prefix('[')
      && let Some(close_br) = after_open.find(']')
    {
      let raw_id = after_open[..close_br]
        .trim()
        .trim_matches('"')
        .replace('"', "'");
      let after_br = after_open[close_br + 1..].trim_start();
      if after_br.is_empty() {
        return format!("{indent}\"{raw_id}\"");
      }
      if after_br.starts_with(':') || after_br.starts_with('{') {
        let sep = if after_br.starts_with(':') { "" } else { " " };
        return format!("{indent}\"{raw_id}\"{sep}{after_br}");
      }
    }
  }

  s
}

fn extract_d2_error_lines(error_text: &str) -> Vec<usize> {
  let mut line_nos = Vec::new();
  let mut rest = error_text;
  while let Some(pos) = rest.find("fig.d2:") {
    let after = &rest[pos + "fig.d2:".len()..];
    let digits_end = after
      .find(|c: char| !c.is_ascii_digit())
      .unwrap_or(after.len());
    if digits_end > 0
      && after.as_bytes().get(digits_end) == Some(&b':')
      && let Ok(n) = after[..digits_end].parse::<usize>()
      && n > 0
      && !line_nos.contains(&n)
    {
      line_nos.push(n);
    }
    rest = &after[digits_end..];
  }
  line_nos
}

fn find_closing_quote(s: &str) -> Option<usize> {
  let mut esc = false;
  for (i, b) in s.bytes().enumerate() {
    if esc {
      esc = false;
      continue;
    }
    if b == b'\\' {
      esc = true;
      continue;
    }
    if b == b'"' {
      return Some(i);
    }
  }
  None
}

fn find_mermaid_pipe_arrow(s: &str) -> Option<(usize, usize)> {
  let bytes = s.as_bytes();
  let mut in_quotes = false;
  let mut esc = false;
  let mut i = 0;
  while i < bytes.len() {
    let b = bytes[i];
    if esc {
      esc = false;
      i += 1;
      continue;
    }
    if b == b'\\' && in_quotes {
      esc = true;
      i += 1;
      continue;
    }
    if b == b'"' {
      in_quotes = !in_quotes;
      i += 1;
      continue;
    }
    if !in_quotes && b == b'-' {
      let start = i;
      while i < bytes.len() && bytes[i] == b'-' {
        i += 1;
      }
      if i < bytes.len() && bytes[i] == b'>' {
        i += 1;
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
          i += 1;
        }
        if i < bytes.len() && bytes[i] == b'|' {
          return Some((start, i));
        }
      }
      continue;
    }
    i += 1;
  }
  None
}

fn replace_unquoted_token(s: &str, from: &str, to: &str) -> String {
  let mut out = String::with_capacity(s.len());
  let bytes = s.as_bytes();
  let fb = from.as_bytes();
  let mut in_quotes = false;
  let mut esc = false;
  let mut i = 0;
  while i < bytes.len() {
    let b = bytes[i];
    if esc {
      out.push(b as char);
      esc = false;
      i += 1;
      continue;
    }
    if b == b'\\' && in_quotes {
      out.push(b as char);
      esc = true;
      i += 1;
      continue;
    }
    if b == b'"' {
      in_quotes = !in_quotes;
      out.push('"');
      i += 1;
      continue;
    }
    if !in_quotes && bytes[i..].starts_with(fb) {
      out.push_str(to);
      i += fb.len();
      continue;
    }
    let ch = s[i..].chars().next().unwrap_or(b as char);
    out.push(ch);
    i += ch.len_utf8();
  }
  out
}

fn quote_unquoted_d2_hex_colors(line: &str) -> String {
  let chars: Vec<char> = line.chars().collect();
  let n = chars.len();
  let mut out = String::with_capacity(line.len() + 4);
  let mut in_quotes = false;
  let mut esc = false;
  let mut i = 0;
  while i < n {
    let ch = chars[i];
    if esc {
      out.push(ch);
      esc = false;
      i += 1;
      continue;
    }
    if ch == '\\' && in_quotes {
      out.push(ch);
      esc = true;
      i += 1;
      continue;
    }
    if ch == '"' {
      in_quotes = !in_quotes;
      out.push(ch);
      i += 1;
      continue;
    }
    if !in_quotes && ch == ':' {
      out.push(':');
      i += 1;
      while i < n && (chars[i] == ' ' || chars[i] == '\t') {
        out.push(chars[i]);
        i += 1;
      }
      if i < n && chars[i] == '#' {
        let mut h = i + 1;
        while h < n && chars[h].is_ascii_hexdigit() {
          h += 1;
        }
        let hex_len = h - (i + 1);
        let at_boundary = h == n || chars[h].is_whitespace() || matches!(chars[h], ';' | '}' | ',');
        if (3..=8).contains(&hex_len) && at_boundary {
          let hex: String = chars[i..h].iter().collect();
          out.push('"');
          out.push_str(&hex);
          out.push('"');
          i = h;
          continue;
        }
      }
      continue;
    }
    out.push(ch);
    i += 1;
  }
  out
}

fn replace_d2_map_commas(line: &str, brace_depth: &mut i32) -> String {
  let chars: Vec<char> = line.chars().collect();
  let n = chars.len();
  let mut out = String::with_capacity(line.len());
  let mut in_quotes = false;
  let mut escaped = false;
  let mut bracket_depth = 0_i32;
  let mut paren_depth = 0_i32;
  let mut i = 0;

  while i < n {
    let ch = chars[i];
    if escaped {
      out.push(ch);
      escaped = false;
      i += 1;
      continue;
    }
    if ch == '\\' && in_quotes {
      out.push(ch);
      escaped = true;
      i += 1;
      continue;
    }
    if ch == '"' {
      in_quotes = !in_quotes;
      out.push(ch);
      i += 1;
      continue;
    }
    if !in_quotes {
      match ch {
        '{' => *brace_depth += 1,
        '}' => *brace_depth = (*brace_depth - 1).max(0),
        '[' => bracket_depth += 1,
        ']' => bracket_depth = (bracket_depth - 1).max(0),
        '(' => paren_depth += 1,
        ')' => paren_depth = (paren_depth - 1).max(0),
        ',' if *brace_depth > 0 && bracket_depth == 0 && paren_depth == 0 => {
          // Look ahead for `<ident_or_quoted_key>:` before any `;`, `}`, `{`, or newline.
          let mut k = i + 1;
          while k < n && (chars[k] == ' ' || chars[k] == '\t') {
            k += 1;
          }
          let mut key_end = k;
          if k < n && chars[k] == '"' {
            key_end += 1;
            while key_end < n && chars[key_end] != '"' {
              if chars[key_end] == '\\' {
                key_end += 1;
              }
              key_end += 1;
            }
            if key_end < n {
              key_end += 1;
            }
          } else {
            while key_end < n
              && (chars[key_end].is_ascii_alphanumeric()
                || matches!(chars[key_end], '_' | '-' | '.'))
            {
              key_end += 1;
            }
          }
          let has_key = key_end > k;
          while key_end < n && (chars[key_end] == ' ' || chars[key_end] == '\t') {
            key_end += 1;
          }
          if has_key && key_end < n && chars[key_end] == ':' {
            out.push(';');
            i += 1;
            continue;
          }
        }
        _ => {}
      }
    }
    out.push(ch);
    i += 1;
  }
  out
}

fn map_d2_shape(token: &str) -> &'static str {
  match token {
    "rectangle" | "square" | "page" | "parallelogram" | "document" | "cylinder" | "queue"
    | "package" | "step" | "callout" | "stored_data" | "person" | "diamond" | "oval" | "circle"
    | "hexagon" | "cloud" | "c4-person" | "text" | "code" | "class" | "sql_table" | "image"
    | "sequence_diagram" => {
      // Return static str matching the exact valid D2 shape.
      match token {
        "square" => "square",
        "page" => "page",
        "parallelogram" => "parallelogram",
        "document" => "document",
        "cylinder" => "cylinder",
        "queue" => "queue",
        "package" => "package",
        "step" => "step",
        "callout" => "callout",
        "stored_data" => "stored_data",
        "person" => "person",
        "diamond" => "diamond",
        "oval" => "oval",
        "circle" => "circle",
        "hexagon" => "hexagon",
        "cloud" => "cloud",
        "c4-person" => "c4-person",
        "text" => "text",
        "code" => "code",
        "class" => "class",
        "sql_table" => "sql_table",
        "image" => "image",
        "sequence_diagram" => "sequence_diagram",
        _ => "rectangle",
      }
    }
    "database" | "db" | "sql" | "storage" | "cache" | "redis" | "bucket" | "disk" => "cylinder",
    "actor" | "user" | "human" | "client_user" => "person",
    "mq" | "broker" | "topic" | "kafka" | "stream" | "channel" => "queue",
    "decision" | "condition" | "branch" => "diamond",
    "pill" | "stadium" | "start" | "end" | "terminal" => "oval",
    _ => "rectangle",
  }
}

fn normalize_d2_shapes_in_line(line: &str) -> String {
  let mut out = String::with_capacity(line.len());
  let mut rest = line;
  while let Some(pos) = rest.find("shape:") {
    // Only match when `shape:` is not part of a longer identifier like `my_shape:`.
    let valid_boundary = pos == 0
      || !rest.as_bytes()[pos - 1].is_ascii_alphanumeric() && rest.as_bytes()[pos - 1] != b'_';
    let head_end = pos + "shape:".len();
    out.push_str(&rest[..head_end]);
    let after = &rest[head_end..];
    if !valid_boundary {
      rest = after;
      continue;
    }
    let trimmed = after.trim_start();
    let token_end = trimmed
      .find(|c: char| c.is_whitespace() || matches!(c, ';' | '}' | '{' | '#'))
      .unwrap_or(trimmed.len());
    let raw_token = trimmed[..token_end].trim_matches('"').trim();
    if !raw_token.is_empty() {
      let canonical = map_d2_shape(&raw_token.to_ascii_lowercase());
      out.push(' ');
      out.push_str(canonical);
      rest = &trimmed[token_end..];
    } else {
      rest = after;
    }
  }
  out.push_str(rest);
  out
}

/// Repair D2 `unexpected text after map` errors:
/// - `a -> b { style.stroke-dash: 3 }: "label"` -> `a -> b: "label" { style.stroke-dash: 3 }`
/// - `id: bigint {constraint: primary_key} NOT NULL` -> `id: bigint {constraint: primary_key}`
/// - `} "trailing"` on a multi-line closing brace -> `}`
fn fix_d2_trailing_text_after_map(line: &str) -> String {
  let trimmed_start = line.trim_start();
  if let Some(after_close) = trimmed_start.strip_prefix('}') {
    let rest = after_close.trim_start();
    if !rest.is_empty()
      && !rest.starts_with(';')
      && !rest.starts_with('}')
      && !rest.starts_with('#')
    {
      let indent = &line[..line.len() - trimmed_start.len()];
      return format!("{indent}}}");
    }
  }

  let chars: Vec<char> = line.chars().collect();
  let n = chars.len();
  let mut out = String::with_capacity(line.len());
  let mut i = 0;
  let mut in_quotes = false;
  let mut escaped = false;

  while i < n {
    let ch = chars[i];
    if escaped {
      out.push(ch);
      escaped = false;
      i += 1;
      continue;
    }
    if ch == '\\' && in_quotes {
      out.push(ch);
      escaped = true;
      i += 1;
      continue;
    }
    if ch == '"' {
      in_quotes = !in_quotes;
      out.push(ch);
      i += 1;
      continue;
    }
    if !in_quotes && ch == '{' {
      let mut depth = 1_i32;
      let mut j = i + 1;
      let mut q2 = false;
      let mut e2 = false;
      while j < n && depth > 0 {
        let c2 = chars[j];
        if e2 {
          e2 = false;
        } else if c2 == '\\' && q2 {
          e2 = true;
        } else if c2 == '"' {
          q2 = !q2;
        } else if !q2 {
          if c2 == '{' {
            depth += 1;
          } else if c2 == '}' {
            depth -= 1;
          }
        }
        j += 1;
      }
      if depth == 0 {
        let inner_raw: String = chars[i + 1..j - 1].iter().collect();
        let inner_fixed = fix_d2_trailing_text_after_map(&inner_raw);
        let block = format!("{{{inner_fixed}}}");
        let mut k = j;
        while k < n && (chars[k] == ' ' || chars[k] == '\t') {
          k += 1;
        }
        if k < n && !matches!(chars[k], ';' | '}' | '#') {
          let mut m = k;
          let mut q3 = false;
          let mut e3 = false;
          while m < n {
            let c3 = chars[m];
            if e3 {
              e3 = false;
            } else if c3 == '\\' && q3 {
              e3 = true;
            } else if c3 == '"' {
              q3 = !q3;
            } else if !q3 && matches!(c3, ';' | '}' | '#') {
              break;
            }
            m += 1;
          }
          let trailing: String = chars[k..m].iter().collect();
          let trailing = trailing.trim();
          if trailing.starts_with(':') {
            while out.ends_with(' ') || out.ends_with('\t') {
              out.pop();
            }
            out.push_str(trailing);
            out.push(' ');
            out.push_str(&block);
          } else {
            out.push_str(&block);
          }
          i = m;
          continue;
        }
        out.push_str(&block);
        i = j;
        continue;
      }
    }
    out.push(ch);
    i += 1;
  }
  out
}

fn find_unquoted_brace(s: &str) -> Option<usize> {
  let mut in_quotes = false;
  let mut escaped = false;
  for (i, b) in s.bytes().enumerate() {
    if escaped {
      escaped = false;
      continue;
    }
    if b == b'\\' && in_quotes {
      escaped = true;
      continue;
    }
    if b == b'"' {
      in_quotes = !in_quotes;
      continue;
    }
    if !in_quotes && b == b'{' {
      return Some(i);
    }
  }
  None
}

fn has_unquoted_d2_arrow(s: &str) -> bool {
  let mut in_quotes = false;
  let mut escaped = false;
  let bytes = s.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    let b = bytes[i];
    if escaped {
      escaped = false;
      i += 1;
      continue;
    }
    if b == b'\\' && in_quotes {
      escaped = true;
      i += 1;
      continue;
    }
    if b == b'"' {
      in_quotes = !in_quotes;
      i += 1;
      continue;
    }
    if !in_quotes {
      if bytes[i..].starts_with(b"<->") {
        return true;
      }
      if bytes[i..].starts_with(b"->")
        || bytes[i..].starts_with(b"<-")
        || bytes[i..].starts_with(b"--")
      {
        return true;
      }
    }
    i += 1;
  }
  false
}

fn brace_delta_unquoted(s: &str) -> i32 {
  let mut in_quotes = false;
  let mut escaped = false;
  let mut delta = 0_i32;
  for b in s.bytes() {
    if escaped {
      escaped = false;
      continue;
    }
    if b == b'\\' && in_quotes {
      escaped = true;
      continue;
    }
    if b == b'"' {
      in_quotes = !in_quotes;
      continue;
    }
    if !in_quotes {
      if b == b'{' {
        delta += 1;
      } else if b == b'}' {
        delta -= 1;
      }
    }
  }
  delta
}

fn is_reserved_d2_edge_entry(stmt: &str) -> bool {
  let s = stmt.trim();
  if s.is_empty() || s.starts_with('#') {
    return true;
  }
  if has_unquoted_d2_arrow(s) {
    return false;
  }
  let Some((raw_key, _)) = s.split_once(':') else {
    return false;
  };
  let root_key = raw_key
    .trim()
    .trim_matches('"')
    .split('.')
    .next()
    .unwrap_or("");
  matches!(
    root_key,
    "style"
      | "source-arrowhead"
      | "target-arrowhead"
      | "label"
      | "tooltip"
      | "link"
      | "icon"
      | "class"
      | "classes"
      | "width"
      | "height"
  )
}

fn edge_block_entries_all_valid(body: &str) -> bool {
  for line in body.lines() {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') || t == "{" || t == "}" {
      continue;
    }
    for part in t.split(';') {
      let part = part.trim().trim_end_matches('}').trim();
      if part.is_empty() {
        continue;
      }
      // Nested sub-blocks like `source-arrowhead: { shape: cf-many }` are valid when root key is reserved.
      if let Some(brace_idx) = find_unquoted_brace(part) {
        let header = &part[..brace_idx];
        if !is_reserved_d2_edge_entry(&format!("{header} x")) && !is_reserved_d2_edge_entry(header)
        {
          return false;
        }
      } else if !is_reserved_d2_edge_entry(part) {
        return false;
      }
    }
  }
  true
}

fn strip_trailing_empty_colon(header: &str) -> String {
  let trimmed = header.trim_end();
  if let Some(without_colon) = trimmed.strip_suffix(':') {
    without_colon.trim_end().to_string()
  } else {
    trimmed.to_string()
  }
}

fn sanitize_d2_edge_maps(lines: &[String]) -> Vec<String> {
  let mut out = Vec::with_capacity(lines.len());
  let mut idx = 0;
  while idx < lines.len() {
    let line = &lines[idx];
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
      out.push(line.clone());
      idx += 1;
      continue;
    }
    if let Some(brace_pos) = find_unquoted_brace(line) {
      let header = &line[..brace_pos];
      if has_unquoted_d2_arrow(header) {
        let mut depth = brace_delta_unquoted(&line[brace_pos..]);
        if depth <= 0 {
          // Single-line edge block: `a -> b: "lbl" { ... }`
          let inner = line[brace_pos + 1..]
            .trim()
            .strip_suffix('}')
            .unwrap_or("")
            .trim();
          if edge_block_entries_all_valid(inner) {
            out.push(line.clone());
          } else {
            out.push(strip_trailing_empty_colon(header));
          }
          idx += 1;
          continue;
        } else {
          // Multi-line edge block: collect through matching `}`.
          let mut end_idx = idx + 1;
          let mut body_lines = vec![line[brace_pos + 1..].to_string()];
          while end_idx < lines.len() && depth > 0 {
            depth += brace_delta_unquoted(&lines[end_idx]);
            body_lines.push(lines[end_idx].clone());
            end_idx += 1;
          }
          let body = body_lines.join("\n");
          if edge_block_entries_all_valid(&body) {
            for l in &lines[idx..end_idx] {
              out.push(l.clone());
            }
          } else {
            out.push(strip_trailing_empty_colon(header));
          }
          idx = end_idx;
          continue;
        }
      }
    }
    out.push(line.clone());
    idx += 1;
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sanitizes_d2_slash_comments_and_database_shape_aliases() {
    let raw = "// top comment\ndb: {\n  shape: sql  // Corrected shape from 'database'\n}\nusers: { shape: sql_table; url: \"https://example.com//a\" }";
    let cleaned = sanitize_d2_source(raw);
    assert!(cleaned.starts_with("# top comment\n"));
    assert!(cleaned.contains("  shape: cylinder\n"));
    assert!(cleaned.contains("shape: sql_table; url: \"https://example.com//a\""));
  }

  #[test]
  fn sanitizes_d2_edge_maps_normalizing_arrowheads_and_stripping_non_reserved_keys() {
    let raw = concat!(
      "orders.user_id -> users.id: { source_arrowhead: cf-many; target_arrowhead: cf-one }\n",
      "Order -> OrderItem: \"1..*\" { source: \"1\"; target: \"1..*\" }\n",
      "api -> db: {\n  constraint: foreign_key\n  multiplicity: \"N:1\"\n}\n",
      "gateway -> auth: \"RPC\" {\n  style.stroke_dash: 3\n}\n"
    );
    let cleaned = sanitize_d2_source(raw);
    assert!(cleaned.contains("source-arrowhead: cf-many; target-arrowhead: cf-one"));
    assert!(cleaned.contains("Order -> OrderItem: \"1..*\"\n"));
    assert!(!cleaned.contains("multiplicity"));
    assert!(cleaned.contains("api -> db\n"));
    assert!(cleaned.contains("style.stroke-dash: 3"));
  }

  #[test]
  fn sanitizes_d2_unexpected_text_after_map() {
    let raw = concat!(
      "a -> b { style.stroke-dash: 3 }: \"async callback\"\n",
      "users: {\n",
      "  shape: sql_table\n",
      "  id: bigint {constraint: primary_key} NOT NULL\n",
      "} \"Users Table\"\n"
    );
    let cleaned = sanitize_d2_source(raw);
    assert!(cleaned.contains("a -> b: \"async callback\" { style.stroke-dash: 3 }\n"));
    assert!(cleaned.contains("  id: bigint {constraint: primary_key}\n"));
    assert!(cleaned.ends_with("}\n") || cleaned.ends_with('}'));
  }

  #[test]
  fn sanitizes_d2_map_commas_and_unknown_shapes() {
    let raw = concat!(
      "sock: { shape: diamond, label: \"Shared Listen Socket\" }\n",
      "client: { class: actor, label: \"客户端\" }\n",
      "worker: { shape: rounded_box }\n"
    );
    let cleaned = sanitize_d2_source(raw);
    assert!(cleaned.contains("sock: { shape: diamond; label: \"Shared Listen Socket\" }"));
    assert!(cleaned.contains("client: { class: actor; label: \"客户端\" }"));
    assert!(cleaned.contains("worker: { shape: rectangle }"));
  }

  #[test]
  fn sanitizes_d2_mermaid_brackets_pipe_labels_hex_colors_and_block_strings() {
    let raw = concat!(
      "gw[\"API 网关\"]: {\n",
      "  style.fill: #e3f2fd\n",
      "  auth[JWT 鉴权]\n",
      "}\n",
      "\"PostgreSQL 主库\" as db: {\n",
      "  shape: database\n",
      "}\n",
      "note: |md\n",
      "  High concurrency\n",
      "|\n",
      "gw -->|SQL 查询| db\n"
    );
    let cleaned = sanitize_d2_source(raw);
    assert!(cleaned.contains("gw: \"API 网关\" {"));
    assert!(cleaned.contains("style.fill: \"#e3f2fd\""));
    assert!(cleaned.contains("auth: \"JWT 鉴权\""));
    assert!(cleaned.contains("db: \"PostgreSQL 主库\" {"));
    assert!(cleaned.contains("note: \"High concurrency\""));
    assert!(cleaned.contains("gw -> db: \"SQL 查询\""));
  }

  #[test]
  fn sanitizes_d2_markdown_tables_inside_and_outside_block_strings() {
    let raw = concat!(
      "legend: |md\n",
      "  ### 订单状态说明\n",
      "  | 状态 | 说明 |\n",
      "  |------|------|\n",
      "  | PAID | 已支付 |\n",
      "|\n",
      "| stray | table |\n",
      "|-------|-------|\n",
      "order -> payment: \"pay\"\n"
    );
    let cleaned = sanitize_d2_source(raw);
    assert!(cleaned.contains("legend: \"### 订单状态说明 / 状态 · 说明 / PAID · 已支付\""));
    assert!(!cleaned.contains("\n|------|------|"));
    assert!(cleaned.contains("\n# | stray | table |"));
    assert!(cleaned.contains("\n# |-------|-------|"));
    assert!(cleaned.contains("order -> payment: \"pay\""));
  }

  #[test]
  fn extracts_the_first_fenced_block_without_its_info_string() {
    assert_eq!(
      extract_source("Here:\n```svg\n<svg/>\n```\nthanks"),
      "<svg/>"
    );
    assert_eq!(
      extract_source("```\n\\documentclass{standalone}\n```"),
      "\\documentclass{standalone}"
    );
    assert_eq!(extract_source("  <svg/>  "), "<svg/>");
  }

  #[test]
  fn latex_errors_keep_the_bang_lines_and_context() {
    let log = "This is LuaHBTeX\n! Undefined control sequence.\nl.4 \\drawx\n (0,0)\nmore\n";
    assert_eq!(
      latex_errors(log).as_deref(),
      Some("! Undefined control sequence.\nl.4 \\drawx\n (0,0)")
    );
    assert_eq!(latex_errors("all good"), None);
  }

  #[test]
  fn the_block_domain_takes_tikz_or_d2_and_the_others_tikz_only() {
    let parse = |name, language| Domain::parse(name, language, None).map(|_| ());
    assert_eq!(parse("block", Language::D2), Ok(()));
    assert_eq!(parse("block", Language::Tikz), Ok(()));
    assert_eq!(
      parse("block", Language::Typst),
      Err("domain block needs language tikz or d2".to_string())
    );
    assert_eq!(
      parse("chemistry", Language::D2),
      Err("domain chemistry needs language tikz".to_string())
    );
    assert_eq!(parse("circuit", Language::Tikz), Ok(()));
  }

  #[test]
  fn strip_workdir_leaves_file_names_relative() {
    let dir = Path::new("/nonexistent/run/.figure-1");
    assert_eq!(
      strip_workdir(
        "err: /nonexistent/run/.figure-1/fig.d2:2:1: connection missing destination",
        dir
      ),
      "err: fig.d2:2:1: connection missing destination"
    );
  }

  #[test]
  fn retry_prompt_carries_the_failed_source_and_error() {
    let prompt = prompt_for(
      Language::Tikz,
      "an RC circuit",
      Some(("\\bad", "! Undefined")),
    );
    assert!(prompt.contains("an RC circuit"));
    assert!(prompt.contains("\\bad") && prompt.contains("! Undefined"));
    assert!(prompt.contains("corrected, complete tikz source"));
  }

  #[tokio::test]
  async fn renders_d2_uml_class_sequence_and_sql_er_when_d2_is_installed() {
    if sandboxed::find_tool("d2").is_none() {
      return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let d2_source = r#"
direction: right

Order: {
  shape: class
  "+id": "UUID"
  "+create(cmd)": "Result"
}

flow: {
  shape: sequence_diagram
  client
  api
  db
  client -> api: "POST /v1/orders"
  api -> db: "INSERT INTO orders"
}

users: {
  shape: sql_table
  id: bigint {constraint: primary_key}
  email: varchar {constraint: unique}
}

orders: {
  shape: sql_table
  id: bigint {constraint: primary_key}
  user_id: bigint {constraint: foreign_key}
}

orders.user_id -> users.id
"#;
    let node = FigureNode::new("uml_er_test");
    let inputs = HashMap::from([
      ("language".to_string(), FlowValue::Json(json!("d2"))),
      ("source".to_string(), FlowValue::Json(json!(d2_source))),
      ("png".to_string(), FlowValue::Json(json!(true))),
    ]);
    let out = run_context::with_artifact_root(dir.path().to_path_buf(), node.execute(&inputs))
      .await
      .expect("d2 UML + ER compiles under OS sandbox");
    assert!(matches!(
      out.get("uml_er_test_svg"),
      Some(FlowValue::File { .. })
    ));
    assert!(matches!(
      out.get("uml_er_test_png"),
      Some(FlowValue::File { .. })
    ));
  }

  #[tokio::test]
  async fn sanitizes_and_renders_d2_rust_generics_routes_continuation_arrows_and_extra_braces() {
    let raw = r#"
direction: right
ingestion-api: {shape: rectangle; label: ingestion-api\n同步受理/校验/幂等}
OrderService: {
  shape: class
  +place_order(ctx: &Context, req: OrderReq): Result<Order, AppError>
  route: POST /v1/orders/:id
  note: "Core Service" (High QPS)
}
}
ingestion-api
  -> OrderService: flush + _SUCCESS (async: 1s)
"#;
    let cleaned = sanitize_d2_source(raw);
    assert!(
      cleaned
        .contains("\"+place_order(ctx: &Context, req: OrderReq)\": \"Result<Order, AppError>\"")
    );
    assert!(cleaned.contains("route: \"POST /v1/orders/:id\""));
    assert!(cleaned.contains("note: \"Core Service (High QPS)\""));
    assert!(cleaned.contains("ingestion-api -> OrderService: \"flush + _SUCCESS (async: 1s)\""));

    if sandboxed::find_tool("d2").is_none() {
      return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let node = FigureNode::new("complex_d2_test");
    let inputs = HashMap::from([
      ("language".to_string(), FlowValue::Json(json!("d2"))),
      ("source".to_string(), FlowValue::Json(json!(raw))),
    ]);
    let out = run_context::with_artifact_root(dir.path().to_path_buf(), node.execute(&inputs))
      .await
      .expect("sanitized complex d2 compiles under OS sandbox");
    assert!(matches!(
      out.get("complex_d2_test_svg"),
      Some(FlowValue::File { .. })
    ));
  }

  #[tokio::test]
  async fn sanitizes_and_renders_plantuml_dashed_arrows_and_alt_sequence_blocks() {
    let raw_uml_class = r#"
direction: right
OrderService: {
  shape: class
  label: |md
    <<trait>> OrderService
    +create(cmd): Result<Order>
  |
}
Order: { shape: class; "-id": "String" }
OrderService ..> Order: manages
RedisInventoryService --|> InventoryService: implements
"#;
    let raw_uml_seq = r#"
shape: sequence_diagram
client: Client
gw: API Gateway
redis: Redis Cluster
pg: PostgreSQL
client -> gw: POST /api/seckill/apply
  (Idempotency-Key, JWT)
gw ->> redis: EVAL DEDUCT_SCRIPT
redis -->> gw: {1, "SUCCESS"} / {-2, "OUT_OF_STOCK"}
alt success
  gw -> pg: BEGIN; INSERT orders; COMMIT
  pg -> gw: Order {status: CREATED}
else failure
  gw -> client: 409 Conflict
end
"#;
    let c1 = sanitize_d2_source(raw_uml_class);
    assert!(c1.contains("OrderService -> Order: manages"));
    assert!(c1.contains("RedisInventoryService -> InventoryService: implements"));

    let c2 = sanitize_d2_source(raw_uml_seq);
    assert!(c2.contains("# alt success"));
    assert!(c2.contains("# else failure"));
    assert!(c2.contains("# end"));
    assert!(c2.contains("gw -> pg: \"BEGIN; INSERT orders; COMMIT\""));

    if sandboxed::find_tool("d2").is_none() {
      return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    for (id, src) in [("uml_cls", raw_uml_class), ("uml_seq", raw_uml_seq)] {
      let node = FigureNode::new(id);
      let inputs = HashMap::from([
        ("language".to_string(), FlowValue::Json(json!("d2"))),
        ("source".to_string(), FlowValue::Json(json!(src))),
      ]);
      let out = run_context::with_artifact_root(dir.path().to_path_buf(), node.execute(&inputs))
        .await
        .expect("sanitized PlantUML/Mermaid hybrid D2 compiles under OS sandbox");
      assert!(matches!(
        out.get(&format!("{id}_svg")),
        Some(FlowValue::File { .. })
      ));
    }
  }

  #[tokio::test]
  async fn sanitizes_single_quoted_hex_bracketed_keys_and_salvages_trailing_broken_d2_lines() {
    let raw = r#"
direction: down
client: {shape: circle; style.fill: '#e1f5fe'}
[svc_user]: {shape: rectangle; label: user-svc\nJava SB3}
pg: {shape: cylinder; label: PostgreSQL}
client -> "svc_user": HTTPS
"svc_user" -> pg: rw
@broken_import_at_bottom
.orphan_leading_dot: { shape: rectangle }
"#;
    let cleaned = sanitize_d2_source(raw);
    assert!(cleaned.contains("style.fill: \"#e1f5fe\""));
    assert!(cleaned.contains("\"svc_user\": {shape: rectangle; label: \"user-svc\\nJava SB3\"}"));

    if sandboxed::find_tool("d2").is_none() {
      return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let node = FigureNode::new("salvage_test");
    let inputs = HashMap::from([
      ("language".to_string(), FlowValue::Json(json!("d2"))),
      ("source".to_string(), FlowValue::Json(json!(raw))),
    ]);
    let out = run_context::with_artifact_root(dir.path().to_path_buf(), node.execute(&inputs))
      .await
      .expect("final-attempt salvage strips broken trailing D2 lines and compiles");
    assert!(matches!(
      out.get("salvage_test_svg"),
      Some(FlowValue::File { .. })
    ));
  }
}
