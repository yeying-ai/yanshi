//! Local parallel batch execution for multiple workflows or input matrices (AJ4).
//!
//! Runs a set of YAML workflows concurrently in-process with zero database or
//! server requirement:
//! - Direct multi-file mode: `yanshi workflow run-batch wf1.yml wf2.yml ...`
//! - Matrix manifest mode: `yanshi workflow run-batch --manifest batch.yml`
//!
//! Enforces `--max-parallel` (simultaneous workflows), `--max-concurrency`
//! (ready nodes per workflow), a shared real-time `--cost-limit-usd` pool via
//! [`yanshi_core::run_context::CostMeter::root_with_limit`], optional
//! `--fail-fast`, Ctrl-C cooperative cancellation, and isolated per-job
//! `--output-dir` export with `batch_summary.json`.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Semaphore;
use uuid::Uuid;
use yanshi_config::config::v2::{FlowDefinitionV2, validate_cost_limit};
use yanshi_core::FlowCancellationToken;
use yanshi_core::FlowExt;
use yanshi_core::async_node::AsyncNodeInputs;
use yanshi_core::error::YanshiError;
use yanshi_core::flow::Flow;
use yanshi_core::run_context::{CostMeter, collect_file_outputs, with_cost_meter};
use yanshi_core::value::FlowValue;

use crate::commands::workflow::validate::print_schema_report;
use crate::config::schema::validate_flow_definition;
use crate::executor::build_flow_from_definition;
use crate::shutdown::{SIGINT_EXIT_CODE, shutdown_signal};

use super::artifacts::copy_to_output_dir;
use super::cli::MAX_CONCURRENCY_DEFAULT;
use super::run::{
  first_genuine_state_error, parse_duration, parse_execution_config, parse_input_value,
};

pub const DEFAULT_MAX_PARALLEL: usize = 4;
pub const DEFAULT_BATCH_TIMEOUT: &str = "300s";
pub const BATCH_SUMMARY_FILE: &str = "batch_summary.json";

#[derive(Args, Debug, Clone)]
pub struct RunBatchCliArgs {
  /// One or more workflow YAML files to run in parallel (optional when --manifest is set)
  pub workflow_files: Vec<String>,
  /// YAML or JSON batch manifest defining workflow jobs and per-job inputs
  #[arg(long, value_name = "FILE")]
  pub manifest: Option<String>,
  /// Shared workflow input as `--input KEY VALUE` or `--input KEY=VALUE` (repeatable)
  #[arg(short, long, num_args = 1..=2, value_names = ["KEY[=VALUE]", "VALUE"])]
  pub input: Vec<String>,
  /// Override the model used by LLM nodes unless overridden per job in --manifest
  #[arg(short = 'm', long)]
  pub model: Option<String>,
  /// Maximum number of workflows executing simultaneously (default: 4)
  #[arg(long = "max-parallel", value_name = "N")]
  pub max_parallel: Option<usize>,
  /// Maximum concurrently running nodes within each workflow (default: 8)
  #[arg(long = "max-concurrency", value_name = "N")]
  pub max_concurrency: Option<usize>,
  /// Shared real-time USD spend cap across the entire batch (`> 0`)
  #[arg(long = "cost-limit-usd", value_name = "USD")]
  pub cost_limit_usd: Option<f64>,
  /// Per-workflow timeout (e.g. `300s`, `10m`; default: `300s`)
  #[arg(long)]
  pub timeout: Option<String>,
  /// Cancel remaining jobs immediately when any job fails
  #[arg(long)]
  pub fail_fast: bool,
  /// Base directory for per-run workflow state (default: YANSHI_RUN_DIR or ~/.yanshi/runs)
  #[arg(long, value_name = "DIR")]
  pub run_dir: Option<String>,
  /// Copy each job's produced artifacts to `<DIR>/<job>/<node_id>/<file>` and write `<DIR>/batch_summary.json`
  #[arg(long, value_name = "DIR")]
  pub output_dir: Option<String>,
  /// Stdout summary format: `text` (human table) or `json` (machine-readable summary)
  #[arg(long, default_value = "text", value_parser = ["text", "json"])]
  pub format: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchManifest {
  /// Default workflow file applied to jobs that omit `workflow`.
  #[serde(default)]
  pub workflow: Option<String>,
  #[serde(default)]
  pub model: Option<String>,
  #[serde(default)]
  pub max_parallel: Option<usize>,
  #[serde(default)]
  pub max_concurrency: Option<usize>,
  #[serde(default)]
  pub cost_limit_usd: Option<f64>,
  #[serde(default)]
  pub timeout: Option<String>,
  #[serde(default)]
  pub fail_fast: bool,
  #[serde(default)]
  pub inputs: BTreeMap<String, Value>,
  #[serde(default)]
  pub jobs: Vec<ManifestJobEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestJobEntry {
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub workflow: Option<String>,
  #[serde(default)]
  pub model: Option<String>,
  #[serde(default)]
  pub cost_limit_usd: Option<f64>,
  #[serde(default)]
  pub timeout: Option<String>,
  #[serde(default)]
  pub inputs: BTreeMap<String, Value>,
}

struct PreparedJob {
  index: usize,
  slug: String,
  workflow_display: String,
  flow: Flow,
  initial_inputs: AsyncNodeInputs,
  model_override: Option<String>,
  job_cost_limit_usd: Option<f64>,
  timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchJobReport {
  pub index: usize,
  pub name: String,
  pub workflow: String,
  pub run_id: String,
  pub status: String,
  pub elapsed_ms: u64,
  pub cost_usd: f64,
  pub artifact_count: usize,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub output_dir: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchSummaryReport {
  pub status: String,
  pub total_jobs: usize,
  pub succeeded: usize,
  pub failed: usize,
  pub cancelled: usize,
  pub max_parallel: usize,
  pub max_concurrency: usize,
  pub elapsed_ms: u64,
  pub total_cost_usd: f64,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cost_limit_usd: Option<f64>,
  pub jobs: Vec<BatchJobReport>,
}

struct AbortAllOnDrop(Vec<tokio::task::AbortHandle>);

impl Drop for AbortAllOnDrop {
  fn drop(&mut self) {
    for handle in &self.0 {
      handle.abort();
    }
  }
}

pub async fn execute(args: RunBatchCliArgs) -> Result<()> {
  let json_mode = args.format == "json";
  let cli_inputs = crate::commands::parse_input_flag_tokens(&args.input)?;

  let (manifest, manifest_base_dir) = match args.manifest.as_deref() {
    Some(path_str) => {
      let path = Path::new(path_str);
      let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read batch manifest '{}'", path.display()))?;
      let parsed: BatchManifest = serde_yaml::from_str(&raw)
        .with_context(|| format!("Failed to parse batch manifest '{}'", path.display()))?;
      let base_dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf);
      (parsed, base_dir)
    }
    None => (BatchManifest::default(), None),
  };

  let max_parallel = args
    .max_parallel
    .or(manifest.max_parallel)
    .unwrap_or(DEFAULT_MAX_PARALLEL);
  if max_parallel == 0 {
    bail!("--max-parallel must be greater than zero");
  }
  let max_concurrency = args
    .max_concurrency
    .or(manifest.max_concurrency)
    .unwrap_or(MAX_CONCURRENCY_DEFAULT);
  if max_concurrency == 0 {
    bail!("--max-concurrency must be greater than zero");
  }
  let fail_fast = args.fail_fast || manifest.fail_fast;
  let batch_cost_limit_usd = combine_cost_limits(args.cost_limit_usd, manifest.cost_limit_usd)?;

  let mut raw_entries: Vec<(ManifestJobEntry, Option<&Path>)> = manifest
    .jobs
    .iter()
    .cloned()
    .map(|entry| (entry, manifest_base_dir.as_deref()))
    .collect();
  for wf in &args.workflow_files {
    raw_entries.push((
      ManifestJobEntry {
        name: None,
        workflow: Some(wf.clone()),
        model: None,
        cost_limit_usd: None,
        timeout: None,
        inputs: BTreeMap::new(),
      },
      None,
    ));
  }
  if raw_entries.is_empty() {
    bail!("no batch jobs specified: pass one or more <WORKFLOW_FILE> paths or --manifest <FILE>");
  }

  let base_exec_config =
    parse_execution_config("concurrent", max_concurrency, args.run_dir.clone())?;
  let artifact_base = base_exec_config
    .resolved_run_base_dir()
    .context("Failed to resolve the run base directory")?;

  let mut used_slugs = HashSet::new();
  let mut prepared_jobs = Vec::with_capacity(raw_entries.len());
  for (index, (entry, rel_base)) in raw_entries.into_iter().enumerate() {
    let raw_wf = entry
      .workflow
      .or_else(|| manifest.workflow.clone())
      .ok_or_else(|| {
        anyhow::anyhow!("batch job #{index} is missing 'workflow' (and manifest sets no default)")
      })?;
    let wf_path = resolve_workflow_path(&raw_wf, rel_base);
    let yaml = fs::read_to_string(&wf_path)
      .with_context(|| format!("batch job #{index}: failed to read '{}'", wf_path.display()))?;
    let flow_def: FlowDefinitionV2 = serde_yaml::from_str(&yaml).with_context(|| {
      format!(
        "batch job #{index}: failed to parse workflow YAML '{}'",
        wf_path.display()
      )
    })?;
    let schema_report = validate_flow_definition(&flow_def);
    if !schema_report.is_valid() {
      print_schema_report(&flow_def.name, &schema_report);
      bail!(
        "batch job #{index} ('{}') failed schema validation with {} issue(s)",
        flow_def.name,
        schema_report.issues.len()
      );
    }

    let model_override = entry
      .model
      .or_else(|| args.model.clone())
      .or_else(|| manifest.model.clone());
    let flow = build_flow_from_definition(&flow_def, model_override.as_deref())
      .with_context(|| format!("batch job #{index} ('{}')", flow_def.name))?;

    let mut initial_inputs = AsyncNodeInputs::new();
    for (k, v) in &manifest.inputs {
      initial_inputs.insert(k.clone(), FlowValue::Json(v.clone()));
    }
    for (k, raw_v) in &cli_inputs {
      if k.trim().is_empty() {
        bail!("Input key cannot be empty");
      }
      initial_inputs.insert(k.clone(), FlowValue::Json(parse_input_value(raw_v)));
    }
    for (k, v) in entry.inputs {
      initial_inputs.insert(k, FlowValue::Json(v));
    }
    crate::executor::seed_chat_model_from_override(
      &flow_def,
      &mut initial_inputs,
      model_override.as_deref(),
    );
    crate::executor::apply_declared_inputs(&flow_def, &mut initial_inputs)
      .with_context(|| format!("batch job #{index} ('{}')", flow_def.name))?;

    let job_cost_limit_usd = flow_def
      .effective_cost_limit(entry.cost_limit_usd)
      .map_err(anyhow::Error::msg)
      .with_context(|| format!("batch job #{index} ('{}')", flow_def.name))?;

    let timeout_raw = entry
      .timeout
      .or_else(|| args.timeout.clone())
      .or_else(|| manifest.timeout.clone())
      .unwrap_or_else(|| DEFAULT_BATCH_TIMEOUT.to_string());
    let timeout = parse_duration(&timeout_raw)
      .with_context(|| format!("batch job #{index}: invalid timeout '{timeout_raw}'"))?;

    let default_stem = wf_path
      .file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or(&flow_def.name);
    let slug = allocate_unique_slug(
      entry.name.as_deref().unwrap_or(default_stem),
      &mut used_slugs,
    );

    prepared_jobs.push(PreparedJob {
      index,
      slug,
      workflow_display: wf_path.display().to_string(),
      flow,
      initial_inputs,
      model_override,
      job_cost_limit_usd,
      timeout,
    });
  }

  let total_jobs = prepared_jobs.len();
  if !json_mode {
    println!(
      "🚀 Running {total_jobs} workflow job(s) in parallel (max_parallel={max_parallel}, max_concurrency={max_concurrency})"
    );
    if let Some(limit) = batch_cost_limit_usd {
      println!("💰 Shared batch cost limit: ${limit:.4}");
    }
  }

  let batch_started = Instant::now();
  let batch_meter = CostMeter::root_with_limit(batch_cost_limit_usd);
  let semaphore = Arc::new(Semaphore::new(max_parallel));
  let batch_cancel = FlowCancellationToken::new();
  let output_root = args.output_dir.as_deref().map(PathBuf::from);
  if let Some(root) = &output_root {
    fs::create_dir_all(root)
      .with_context(|| format!("Failed to create --output-dir '{}'", root.display()))?;
  }

  let mut handles = Vec::with_capacity(total_jobs);
  let mut abort_guard = AbortAllOnDrop(Vec::with_capacity(total_jobs));
  for job in prepared_jobs {
    let sem = semaphore.clone();
    let batch_meter = batch_meter.clone();
    let batch_cancel = batch_cancel.clone();
    let base_exec_config = base_exec_config.clone();
    let artifact_base = artifact_base.clone();
    let output_root = output_root.clone();

    let handle = tokio::spawn(async move {
      let run_id = Uuid::new_v4().to_string();
      let started = Instant::now();

      if batch_cancel.is_cancelled() {
        return cancelled_report(&job, run_id, started.elapsed());
      }
      let Ok(_permit) = sem.acquire_owned().await else {
        return cancelled_report(&job, run_id, started.elapsed());
      };
      if batch_cancel.is_cancelled() {
        return cancelled_report(&job, run_id, started.elapsed());
      }
      if let Some((spent_usd, limit_usd)) = batch_meter.exceeded_limit() {
        return BatchJobReport {
          index: job.index,
          name: job.slug,
          workflow: job.workflow_display,
          run_id,
          status: "failed".to_string(),
          elapsed_ms: started.elapsed().as_millis() as u64,
          cost_usd: 0.0,
          artifact_count: 0,
          output_dir: None,
          error: Some(
            YanshiError::CostLimitExceeded {
              spent_usd,
              limit_usd,
            }
            .to_string(),
          ),
        };
      }

      let mut exec_cfg = base_exec_config.with_cancellation_token(batch_cancel.clone());
      if let Some(model) = &job.model_override {
        exec_cfg = exec_cfg.with_model_override(model.clone());
      }
      if let Some(limit) = job.job_cost_limit_usd {
        exec_cfg = exec_cfg.with_cost_limit_usd(limit);
      }

      let job_meter = CostMeter::child(batch_meter);
      let exec_fut = job.flow.execute_from_inputs_with_id_and_config(
        run_id.clone(),
        job.initial_inputs,
        exec_cfg,
      );
      let outcome = with_cost_meter(
        Some(job_meter.clone()),
        tokio::time::timeout(job.timeout, exec_fut),
      )
      .await;
      let elapsed_ms = started.elapsed().as_millis() as u64;
      let cost_usd = job_meter.total();

      let (status, artifact_count, job_out_dir, error) = match outcome {
        Ok(Ok(state)) => match first_genuine_state_error(&state) {
          None => {
            let produced = collect_file_outputs(&state, &artifact_base);
            match &output_root {
              Some(root) => {
                let dest = root.join(&job.slug);
                match copy_to_output_dir(&produced, &dest, &run_id) {
                  Ok(copied) => (
                    "succeeded",
                    copied.len(),
                    Some(dest.display().to_string()),
                    None,
                  ),
                  Err(copy_err) => ("failed", 0, None, Some(format!("{copy_err:#}"))),
                }
              }
              None => ("succeeded", produced.len(), None, None),
            }
          }
          Some(err) => ("failed", 0, None, Some(err)),
        },
        Ok(Err(YanshiError::TaskCancelled)) => ("cancelled", 0, None, None),
        Ok(Err(err)) => {
          if batch_cancel.is_cancelled() {
            ("cancelled", 0, None, None)
          } else {
            ("failed", 0, None, Some(err.to_string()))
          }
        }
        Err(_) => (
          "failed",
          0,
          None,
          Some(format!("workflow timed out after {:?}", job.timeout)),
        ),
      };

      if status == "failed" && fail_fast {
        batch_cancel.cancel();
      }
      if !json_mode {
        let icon = match status {
          "succeeded" => "✅",
          "cancelled" => "🛑",
          _ => "❌",
        };
        println!(
          "  {icon} [{}] status={status} elapsed={elapsed_ms}ms cost=${cost_usd:.4} artifacts={artifact_count}",
          job.slug
        );
      }

      BatchJobReport {
        index: job.index,
        name: job.slug,
        workflow: job.workflow_display,
        run_id,
        status: status.to_string(),
        elapsed_ms,
        cost_usd,
        artifact_count,
        output_dir: job_out_dir,
        error,
      }
    });
    abort_guard.0.push(handle.abort_handle());
    handles.push((job_index_for_handle(&abort_guard), handle));
  }

  let join_fut = futures::future::join_all(handles.into_iter().map(|(idx, handle)| async move {
    match handle.await {
      Ok(report) => report,
      Err(join_err) => BatchJobReport {
        index: idx,
        name: format!("job-{idx}"),
        workflow: String::new(),
        run_id: String::new(),
        status: "failed".to_string(),
        elapsed_ms: 0,
        cost_usd: 0.0,
        artifact_count: 0,
        output_dir: None,
        error: Some(format!("batch task panicked or aborted: {join_err}")),
      },
    }
  }));
  tokio::pin!(join_fut);

  let mut reports = tokio::select! {
    biased;
    res = &mut join_fut => res,
    _ = shutdown_signal() => {
      eprintln!("\n🛑 Cancelled batch (received SIGINT/SIGTERM)");
      batch_cancel.cancel();
      let _ = tokio::time::timeout(Duration::from_secs(10), &mut join_fut).await;
      std::process::exit(SIGINT_EXIT_CODE);
    }
  };
  // Disarm the drop-abort guard now that all tasks have joined cleanly.
  abort_guard.0.clear();

  reports.sort_by_key(|r| r.index);
  let succeeded = reports.iter().filter(|r| r.status == "succeeded").count();
  let failed = reports.iter().filter(|r| r.status == "failed").count();
  let cancelled = reports.iter().filter(|r| r.status == "cancelled").count();
  let overall_status = if failed == 0 && cancelled == 0 {
    "succeeded"
  } else if succeeded > 0 {
    "partial_failure"
  } else {
    "failed"
  };

  let summary = BatchSummaryReport {
    status: overall_status.to_string(),
    total_jobs,
    succeeded,
    failed,
    cancelled,
    max_parallel,
    max_concurrency,
    elapsed_ms: batch_started.elapsed().as_millis() as u64,
    total_cost_usd: batch_meter.total(),
    cost_limit_usd: batch_cost_limit_usd,
    jobs: reports,
  };

  if let Some(root) = &output_root {
    let summary_path = root.join(BATCH_SUMMARY_FILE);
    let summary_json =
      serde_json::to_string_pretty(&summary).context("Failed to serialize batch summary report")?;
    fs::write(&summary_path, summary_json)
      .with_context(|| format!("Failed to write {}", summary_path.display()))?;
    if !json_mode {
      println!("📝 Batch summary written to {}", summary_path.display());
    }
  }

  if json_mode {
    println!("{}", serde_json::to_string_pretty(&summary)?);
  } else {
    println!(
      "\n🏁 Batch finished in {}ms — succeeded={succeeded}, failed={failed}, cancelled={cancelled}, total_cost=${:.4}",
      summary.elapsed_ms, summary.total_cost_usd
    );
    for job in &summary.jobs {
      if let Some(err) = &job.error {
        eprintln!("  ✗ {}: {err}", job.name);
      }
    }
  }

  if failed > 0 || cancelled > 0 {
    bail!(
      "workflow batch ended with status '{}' ({} failed, {} cancelled out of {})",
      summary.status,
      failed,
      cancelled,
      total_jobs
    );
  }
  Ok(())
}

fn job_index_for_handle(guard: &AbortAllOnDrop) -> usize {
  guard.0.len().saturating_sub(1)
}

fn cancelled_report(job: &PreparedJob, run_id: String, elapsed: Duration) -> BatchJobReport {
  BatchJobReport {
    index: job.index,
    name: job.slug.clone(),
    workflow: job.workflow_display.clone(),
    run_id,
    status: "cancelled".to_string(),
    elapsed_ms: elapsed.as_millis() as u64,
    cost_usd: 0.0,
    artifact_count: 0,
    output_dir: None,
    error: None,
  }
}

fn combine_cost_limits(cli_limit: Option<f64>, manifest_limit: Option<f64>) -> Result<Option<f64>> {
  let cli = cli_limit
    .map(validate_cost_limit)
    .transpose()
    .map_err(anyhow::Error::msg)?;
  let man = manifest_limit
    .map(validate_cost_limit)
    .transpose()
    .map_err(anyhow::Error::msg)?;
  Ok(match (cli, man) {
    (Some(a), Some(b)) => Some(a.min(b)),
    (a, b) => a.or(b),
  })
}

fn resolve_workflow_path(raw: &str, manifest_base_dir: Option<&Path>) -> PathBuf {
  let candidate = Path::new(raw);
  if candidate.is_absolute() {
    return candidate.to_path_buf();
  }
  if let Some(base) = manifest_base_dir {
    let relative_to_manifest = base.join(candidate);
    if relative_to_manifest.exists() {
      return relative_to_manifest;
    }
  }
  candidate.to_path_buf()
}

pub(crate) fn allocate_unique_slug(raw: &str, used: &mut HashSet<String>) -> String {
  let sanitized: String = raw
    .trim()
    .chars()
    .map(|ch| {
      if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
        ch
      } else {
        '-'
      }
    })
    .collect();
  let base = sanitized.trim_matches(|c| c == '-' || c == '.').to_string();
  let base = if base.is_empty() {
    "job".to_string()
  } else {
    base
  };
  if used.insert(base.clone()) {
    return base;
  }
  let mut n = 2usize;
  loop {
    let candidate = format!("{base}-{n}");
    if used.insert(candidate.clone()) {
      return candidate;
    }
    n += 1;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn slug_allocator_sanitizes_traversal_and_disambiguates_duplicates() {
    let mut used = HashSet::new();
    assert_eq!(
      allocate_unique_slug("paper_reader", &mut used),
      "paper_reader"
    );
    assert_eq!(
      allocate_unique_slug("paper_reader", &mut used),
      "paper_reader-2"
    );
    assert_eq!(
      allocate_unique_slug("../etc/passwd", &mut used),
      "etc-passwd"
    );
    assert_eq!(allocate_unique_slug("...", &mut used), "job");
  }

  #[test]
  fn combine_cost_limits_takes_tighter_bound_and_rejects_non_positive() {
    assert_eq!(
      combine_cost_limits(Some(2.0), Some(1.5)).unwrap(),
      Some(1.5)
    );
    assert_eq!(combine_cost_limits(None, Some(3.0)).unwrap(), Some(3.0));
    assert_eq!(combine_cost_limits(None, None).unwrap(), None);
    assert!(combine_cost_limits(Some(0.0), None).is_err());
    assert!(combine_cost_limits(None, Some(-1.0)).is_err());
  }

  #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  async fn execute_runs_manifest_matrix_in_parallel_and_writes_isolated_outputs_and_summary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let wf_path = dir.path().join("badge.yml");
    fs::write(
      &wf_path,
      r##"
name: badge_generator
inputs:
  label:
    required: true
nodes:
  - id: compose
    type: template
    parameters:
      template: "# Note: {{ label }}"
  - id: md_card
    type: render_markdown
    dependencies: ["compose"]
    input_mapping:
      markdown: "{{ nodes.compose.outputs.output }}"
    parameters:
      format: md
"##,
    )
    .expect("write workflow");

    let manifest_path = dir.path().join("batch.yml");
    fs::write(
      &manifest_path,
      r#"
workflow: badge.yml
max_parallel: 3
max_concurrency: 4
jobs:
  - name: paper-a
    inputs:
      label: "Attention"
  - name: paper-b
    inputs:
      label: "FlashAttention"
  - inputs:
      label: "RingAttention"
"#,
    )
    .expect("write manifest");

    let run_dir = dir.path().join("runs");
    let out_dir = dir.path().join("out");
    execute(RunBatchCliArgs {
      workflow_files: vec![],
      manifest: Some(manifest_path.display().to_string()),
      input: vec![],
      model: None,
      max_parallel: None,
      max_concurrency: None,
      cost_limit_usd: Some(1.0),
      timeout: Some("30s".to_string()),
      fail_fast: false,
      run_dir: Some(run_dir.display().to_string()),
      output_dir: Some(out_dir.display().to_string()),
      format: "json".to_string(),
    })
    .await
    .expect("batch execution succeeds");

    let summary_raw = fs::read_to_string(out_dir.join(BATCH_SUMMARY_FILE)).expect("summary exists");
    let summary: BatchSummaryReport = serde_json::from_str(&summary_raw).expect("valid summary");
    assert_eq!(summary.status, "succeeded");
    assert_eq!(summary.total_jobs, 3);
    assert_eq!(summary.succeeded, 3);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.jobs[0].name, "paper-a");
    assert_eq!(summary.jobs[1].name, "paper-b");
    assert_eq!(summary.jobs[2].name, "badge");
    for (slug, expected_text) in [
      ("paper-a", "Attention"),
      ("paper-b", "FlashAttention"),
      ("badge", "RingAttention"),
    ] {
      let job_manifest = out_dir.join(slug).join("artifacts.json");
      assert!(job_manifest.exists(), "missing {}", job_manifest.display());
      let job_md_dir = out_dir.join(slug).join("md_card");
      let entries: Vec<PathBuf> = fs::read_dir(&job_md_dir)
        .expect("md_card dir")
        .map(|e| e.expect("entry").path())
        .collect();
      assert_eq!(entries.len(), 1);
      let md_content = fs::read_to_string(&entries[0]).expect("md file");
      assert!(
        md_content.contains(expected_text),
        "expected '{expected_text}' in {md_content}"
      );
    }
  }

  #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
  async fn execute_records_partial_failure_summary_before_returning_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ok_wf = dir.path().join("ok.yml");
    fs::write(
      &ok_wf,
      "name: ok_wf\nnodes:\n  - id: a\n    type: template\n    parameters:\n      template: hello\n",
    )
    .expect("ok wf");
    let bad_wf = dir.path().join("bad.yml");
    fs::write(
      &bad_wf,
      "name: bad_wf\nnodes:\n  - id: b\n    type: file\n    parameters:\n      operation: read\n      path: /nonexistent-aj4-test-path\n      allowed_paths: [\"/\"]\n",
    )
    .expect("bad wf");

    let run_dir = dir.path().join("runs");
    let out_dir = dir.path().join("out");
    let err = execute(RunBatchCliArgs {
      workflow_files: vec![ok_wf.display().to_string(), bad_wf.display().to_string()],
      manifest: None,
      input: vec![],
      model: None,
      max_parallel: Some(2),
      max_concurrency: Some(2),
      cost_limit_usd: None,
      timeout: Some("15s".to_string()),
      fail_fast: false,
      run_dir: Some(run_dir.display().to_string()),
      output_dir: Some(out_dir.display().to_string()),
      format: "json".to_string(),
    })
    .await
    .expect_err("partial failure must return non-zero error");
    assert!(err.to_string().contains("partial_failure"), "{err}");

    let summary_raw = fs::read_to_string(out_dir.join(BATCH_SUMMARY_FILE)).expect("summary");
    let summary: BatchSummaryReport = serde_json::from_str(&summary_raw).expect("parse");
    assert_eq!(summary.status, "partial_failure");
    assert_eq!(summary.succeeded, 1);
    assert_eq!(summary.failed, 1);
  }
}
