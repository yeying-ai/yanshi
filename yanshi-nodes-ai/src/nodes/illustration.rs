//! Art-directed technical & editorial illustration node (`type: illustration`).
//!
//! Bridges the gap between abstract slide/document concepts and clean, visually
//! cohesive presentation artwork:
//! - Applies curated art-direction presets (`editorial_tech`, `isometric_3d`,
//!   `flat_vector`, `blueprint_schematic`, `scientific_diagram`,
//!   `cinematic_keynote`) harmonized with the target `dark` or `light` canvas
//!   palette and strict anti-garbled-text guardrails.
//! - Optionally refines abstract technical prompts through a lightweight LLM
//!   art-director pass (`refine_prompt: true` + `prompt_model`).
//! - Operates in **single mode** (`concept` / `prompt` → `{id}_image`) OR
//!   **deck / batch mode** (`deck` / `items` → bounded concurrent generation on
//!   the current task so `run_context` artifact roots and `CostMeter` propagate
//!   intact, wiring generated `Panel::Image`s + optional upstream `arch_image`
//!   directly onto `deck.slides[*].right` with `fallback_right` graceful
//!   degradation).

use std::collections::HashMap;

use async_trait::async_trait;
use futures::StreamExt as _;
use serde_json::{Value, json};
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  run_context,
  value::FlowValue,
};
use yanshi_llm::{Yanshi, providers::modality::Text2ImageRequest};

use crate::artifacts::{self, ArtifactOutputFormat};

const DEFAULT_MAX_IMAGES: usize = 4;
const DEFAULT_MAX_CONCURRENT: usize = 3;
const DEFAULT_SIZE: &str = "16:9";

const ANTI_TEXT_GUARDRAIL: &str = "Pure visual metaphor only: strictly NO embedded text, NO letters, NO words, NO numbers, NO watermarks, NO logos, NO UI window chrome.";

/// Built-in art-direction style presets tailored for presentations and reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StylePreset {
  EditorialTech,
  Isometric3d,
  FlatVector,
  BlueprintSchematic,
  ScientificDiagram,
  CinematicKeynote,
}

impl StylePreset {
  pub fn parse(raw: Option<&str>) -> Self {
    match raw
      .unwrap_or("editorial_tech")
      .trim()
      .to_ascii_lowercase()
      .as_str()
    {
      "isometric" | "isometric_3d" | "3d" => Self::Isometric3d,
      "flat" | "flat_vector" | "vector" => Self::FlatVector,
      "blueprint" | "blueprint_schematic" | "schematic" => Self::BlueprintSchematic,
      "scientific" | "scientific_diagram" | "academic" => Self::ScientificDiagram,
      "cinematic" | "cinematic_keynote" | "keynote" => Self::CinematicKeynote,
      _ => Self::EditorialTech,
    }
  }

  fn direction(self) -> &'static str {
    match self {
      Self::EditorialTech => {
        "Modern editorial technical illustration, structured geometric forms, clear focal hierarchy, refined studio rim lighting, crisp architectural composition"
      }
      Self::Isometric3d => {
        "Clean isometric 3D architectural cutaway illustration, matte tactile surfaces, soft ambient occlusion, precision studio lighting"
      }
      Self::FlatVector => {
        "Minimalist modern editorial vector art, bold clean geometric shapes, generous negative space, crisp color contrast"
      }
      Self::BlueprintSchematic => {
        "Precision engineering exploded schematic aesthetic, clean structural linework, layered technical depth, balanced composition"
      }
      Self::ScientificDiagram => {
        "Nature/Science journal conceptual figure aesthetic, crystal-clear structural metaphor, clean spatial separation, high visual clarity"
      }
      Self::CinematicKeynote => {
        "Keynote stage hero visual, sleek sculptural focal subject, subtle depth of field, dramatic volumetric studio lighting"
      }
    }
  }
}

fn theme_palette_clause(theme: &str) -> &'static str {
  if theme.trim().eq_ignore_ascii_case("light") {
    "Color palette harmonized for a crisp light slide canvas (#F8FAFC off-white background, deep slate navy structure, warm orange and sky-blue accents)"
  } else {
    "Color palette harmonized for a deep midnight-slate presentation canvas (#0B1120 dark background, luminous cyan, warm orange, and subtle violet highlights)"
  }
}

/// Compose a deterministic, art-directed image generation prompt.
pub fn compose_art_prompt(
  concept: &str,
  context: Option<&str>,
  preset: StylePreset,
  custom_style: Option<&str>,
  theme: &str,
) -> String {
  let mut parts = Vec::with_capacity(5);
  parts.push(concept.trim().to_string());
  if let Some(ctx) = context.map(str::trim).filter(|s| !s.is_empty()) {
    parts.push(format!("Topic context: {ctx}"));
  }
  parts.push(preset.direction().to_string());
  if let Some(cs) = custom_style.map(str::trim).filter(|s| !s.is_empty()) {
    parts.push(format!("Visual nuance: {cs}"));
  }
  parts.push(theme_palette_clause(theme).to_string());
  parts.push(ANTI_TEXT_GUARDRAIL.to_string());
  parts.join(". ")
}

/// Dedicated art-directed illustration node (`type: illustration`).
#[derive(Debug, Clone)]
pub struct IllustrationNode {
  pub id: String,
  pub model: String,
  pub prompt_model: Option<String>,
  pub style_preset: Option<String>,
  pub style: Option<String>,
  pub theme: Option<String>,
  pub size: Option<String>,
  pub refine_prompt: bool,
  pub max_images: usize,
  pub max_concurrent: usize,
  pub enabled: bool,
  pub graceful_fallback: bool,
  pub output_format: ArtifactOutputFormat,
}

impl IllustrationNode {
  pub fn new(id: impl Into<String>) -> Self {
    Self {
      id: id.into(),
      model: String::new(),
      prompt_model: None,
      style_preset: None,
      style: None,
      theme: None,
      size: None,
      refine_prompt: false,
      max_images: DEFAULT_MAX_IMAGES,
      max_concurrent: DEFAULT_MAX_CONCURRENT,
      enabled: true,
      graceful_fallback: true,
      output_format: ArtifactOutputFormat::default(),
    }
  }

  pub fn with_model(mut self, model: impl Into<String>) -> Self {
    self.model = model.into();
    self
  }

  pub fn with_prompt_model(mut self, model: impl Into<String>) -> Self {
    let m = model.into();
    if !m.trim().is_empty() {
      self.prompt_model = Some(m);
    }
    self
  }

  pub fn with_style_preset(mut self, preset: impl Into<String>) -> Self {
    let p = preset.into();
    if !p.trim().is_empty() {
      self.style_preset = Some(p);
    }
    self
  }

  pub fn with_style(mut self, style: impl Into<String>) -> Self {
    let s = style.into();
    if !s.trim().is_empty() {
      self.style = Some(s);
    }
    self
  }

  pub fn with_theme(mut self, theme: impl Into<String>) -> Self {
    let t = theme.into();
    if !t.trim().is_empty() {
      self.theme = Some(t);
    }
    self
  }

  pub fn with_size(mut self, size: impl Into<String>) -> Self {
    let s = size.into();
    if !s.trim().is_empty() {
      self.size = Some(s);
    }
    self
  }

  pub fn with_refine_prompt(mut self, refine: bool) -> Self {
    self.refine_prompt = refine;
    self
  }

  pub fn with_max_images(mut self, max_images: usize) -> Self {
    self.max_images = max_images.clamp(0, 12);
    self
  }

  pub fn with_max_concurrent(mut self, max_concurrent: usize) -> Self {
    self.max_concurrent = max_concurrent.clamp(1, 6);
    self
  }

  pub fn with_enabled(mut self, enabled: bool) -> Self {
    self.enabled = enabled;
    self
  }

  pub fn with_graceful_fallback(mut self, graceful: bool) -> Self {
    self.graceful_fallback = graceful;
    self
  }

  pub fn with_output_format(mut self, format: ArtifactOutputFormat) -> Self {
    self.output_format = format;
    self
  }

  fn input_error(&self, msg: impl std::fmt::Display) -> YanshiError {
    YanshiError::NodeInputError {
      message: format!("{}: {msg}", self.id),
    }
  }

  async fn refine_visual_concept(
    &self,
    prompt_model: &str,
    raw_concept: &str,
    context: Option<&str>,
    preset: StylePreset,
    theme: &str,
  ) -> Result<String, YanshiError> {
    let system = "You are an art director for technical keynote presentations. \
      Rewrite the user's concept into ONE concise English image-generation prompt (45-85 words) \
      describing a concrete visual metaphor, focal object, spatial composition, and lighting. \
      Strictly forbid any text, letters, numbers, labels, or UI screenshots inside the image. \
      Output ONLY the prompt text, with no markdown or preamble.";
    let ctx_line = context
      .filter(|c| !c.trim().is_empty())
      .map(|c| format!("\nPresentation context: {c}"))
      .unwrap_or_default();
    let user_prompt = format!(
      "Concept: {raw_concept}{ctx_line}\nTarget visual direction: {}\nCanvas theme: {theme}",
      preset.direction()
    );

    Yanshi::ensure_init()
      .await
      .map_err(|e| YanshiError::ConfigurationError {
        message: format!("{}: failed to initialize Yanshi LLM service: {e}", self.id),
      })?;

    let response = Yanshi::model(prompt_model)
      .system(system)
      .prompt(&user_prompt)
      .max_tokens(300)
      .thinking(yanshi_llm::ThinkingConfig::Disabled)
      .execute_full()
      .await
      .map_err(|e| crate::vendor_error::vendor_error("illustration prompt refinement failed", e))?;

    crate::cost::record(prompt_model, |p| match &response.usage {
      Some(u) => p.cost_for_call(u.prompt_tokens, u.completion_tokens),
      None => 0.0,
    })?;

    let cleaned = strip_think_tags(&response.content).trim().to_string();
    if cleaned.is_empty() {
      Ok(raw_concept.to_string())
    } else {
      Ok(cleaned)
    }
  }

  #[allow(clippy::too_many_arguments)]
  async fn generate_one_illustration(
    &self,
    image_model: &str,
    raw_concept: &str,
    context: Option<&str>,
    preset: StylePreset,
    custom_style: Option<&str>,
    theme: &str,
    size: &str,
    refine: bool,
    prompt_model: Option<&str>,
  ) -> Result<(FlowValue, String), YanshiError> {
    let core_subject = if refine && let Some(pm) = prompt_model.filter(|m| !m.trim().is_empty()) {
      match self
        .refine_visual_concept(pm, raw_concept, context, preset, theme)
        .await
      {
        Ok(refined) => refined,
        Err(err @ YanshiError::CostLimitExceeded { .. }) => return Err(err),
        Err(err) if self.graceful_fallback => {
          tracing::warn!(
            node = %self.id,
            error = %err,
            "prompt refinement failed; falling back to direct art-directed prompt"
          );
          raw_concept.to_string()
        }
        Err(err) => return Err(err),
      }
    } else {
      raw_concept.to_string()
    };

    let final_prompt = compose_art_prompt(&core_subject, context, preset, custom_style, theme);

    Yanshi::ensure_init()
      .await
      .map_err(|e| YanshiError::ConfigurationError {
        message: format!("{}: failed to initialize Yanshi LLM service: {e}", self.id),
      })?;

    let provider =
      Yanshi::text2image_for(image_model)
        .await
        .map_err(|e| YanshiError::ConfigurationError {
          message: format!(
            "{}: failed to resolve text-to-image provider for '{image_model}': {e}",
            self.id
          ),
        })?;

    let request = Text2ImageRequest {
      model: image_model.to_string(),
      prompt: final_prompt.clone(),
      size: Some(size.to_string()),
      n: Some(1),
      response_format: Some("b64_json".to_string()),
      seed: None,
      steps: None,
      cfg_scale: None,
    };

    let image_response = provider
      .generate(request)
      .await
      .map_err(|e| crate::vendor_error::vendor_error("Illustration generation failed", e))?;
    crate::cost::record(image_model, |p| {
      p.cost_for_images(image_response.images.len())
    })?;

    let first_image =
      image_response
        .images
        .first()
        .ok_or_else(|| YanshiError::AsyncExecutionError {
          message: format!("{}: image provider returned empty images list", self.id),
        })?;

    let file_val = artifacts::image_output(&self.id, first_image, self.output_format).await?;
    Ok((file_val, final_prompt))
  }
}

struct DeckIllustrationJob {
  slide_index: usize,
  concept: String,
  title: String,
  caption: String,
}

fn str_input<'a>(inputs: &'a AsyncNodeInputs, key: &str) -> Option<&'a str> {
  match inputs.get(key) {
    Some(FlowValue::Json(Value::String(s))) if !s.trim().is_empty() => Some(s.trim()),
    _ => None,
  }
}

fn path_or_str_input(inputs: &AsyncNodeInputs, key: &str) -> Option<String> {
  match inputs.get(key) {
    Some(FlowValue::File { path, .. }) => Some(path.to_string_lossy().into_owned()),
    Some(FlowValue::Json(Value::String(s))) if !s.trim().is_empty() => Some(s.trim().to_string()),
    Some(FlowValue::Json(Value::Object(map))) => map
      .get("path")
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|s| !s.is_empty())
      .map(str::to_string),
    _ => None,
  }
}

fn bool_input(inputs: &AsyncNodeInputs, key: &str, default: bool) -> bool {
  match inputs.get(key) {
    Some(FlowValue::Json(Value::Bool(b))) => *b,
    Some(FlowValue::Json(Value::String(s))) => match s.trim().to_ascii_lowercase().as_str() {
      "true" | "1" | "yes" => true,
      "false" | "0" | "no" => false,
      _ => default,
    },
    _ => default,
  }
}

fn usize_input(inputs: &AsyncNodeInputs, key: &str, default: usize) -> usize {
  match inputs.get(key) {
    Some(FlowValue::Json(Value::Number(n))) => n
      .as_u64()
      .and_then(|v| usize::try_from(v).ok())
      .unwrap_or(default),
    Some(FlowValue::Json(Value::String(s))) => s.trim().parse::<usize>().unwrap_or(default),
    _ => default,
  }
}

fn strip_think_tags(s: &str) -> String {
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

async fn json_input(
  node_id: &str,
  inputs: &AsyncNodeInputs,
  key: &str,
) -> Result<Option<Value>, YanshiError> {
  match inputs.get(key) {
    None => Ok(None),
    Some(FlowValue::Json(Value::Null)) => Ok(None),
    Some(FlowValue::Json(Value::String(s))) => {
      let trimmed = s.trim();
      if trimmed.is_empty() {
        return Ok(None);
      }
      let parsed =
        serde_json::from_str::<Value>(trimmed).map_err(|e| YanshiError::NodeInputError {
          message: format!("{node_id}: '{key}' is not valid JSON: {e}"),
        })?;
      Ok(Some(parsed))
    }
    Some(FlowValue::Json(v)) => Ok(Some(v.clone())),
    Some(FlowValue::File { path, .. }) => {
      let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| YanshiError::NodeInputError {
          message: format!(
            "{node_id}: cannot read '{key}' from {}: {e}",
            path.display()
          ),
        })?;
      let parsed =
        serde_json::from_str::<Value>(&raw).map_err(|e| YanshiError::NodeInputError {
          message: format!("{node_id}: file '{key}' is not valid JSON: {e}"),
        })?;
      Ok(Some(parsed))
    }
    Some(_) => Err(YanshiError::NodeInputError {
      message: format!("{node_id}: '{key}' must be JSON or a JSON file"),
    }),
  }
}

fn apply_fallback_right(slide_obj: &mut serde_json::Map<String, Value>) {
  let missing_right = match slide_obj.get("right") {
    None | Some(Value::Null) => true,
    Some(Value::Object(m)) => m.is_empty(),
    _ => false,
  };
  if missing_right
    && let Some(fb) = slide_obj.get("fallback_right").cloned()
    && fb.is_object()
  {
    slide_obj.insert("right".to_string(), fb);
  }
}

#[async_trait]
impl AsyncNode for IllustrationNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let image_model = str_input(inputs, "model")
      .map(str::to_string)
      .unwrap_or_else(|| self.model.clone());
    let prompt_model = str_input(inputs, "prompt_model")
      .map(str::to_string)
      .or_else(|| self.prompt_model.clone());
    let preset =
      StylePreset::parse(str_input(inputs, "style_preset").or(self.style_preset.as_deref()));
    let custom_style = str_input(inputs, "style")
      .map(str::to_string)
      .or_else(|| self.style.clone());
    let size = str_input(inputs, "size")
      .or(self.size.as_deref())
      .unwrap_or(DEFAULT_SIZE)
      .to_string();
    let enabled = bool_input(inputs, "enabled", self.enabled);
    let refine = bool_input(inputs, "refine_prompt", self.refine_prompt);
    let graceful = bool_input(inputs, "graceful_fallback", self.graceful_fallback);
    let max_images = usize_input(inputs, "max_images", self.max_images).clamp(0, 12);
    let max_concurrent = usize_input(inputs, "max_concurrent", self.max_concurrent).clamp(1, 6);

    // ── Mode B1: SlideDeck Direct Assembly (`deck` input) ──────────────
    if let Some(mut deck_val) = json_input(&self.id, inputs, "deck").await? {
      let deck_obj = deck_val
        .as_object_mut()
        .ok_or_else(|| self.input_error("'deck' must be a JSON object"))?;

      let deck_title = deck_obj
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
      let theme = str_input(inputs, "theme")
        .or(self.theme.as_deref())
        .or_else(|| deck_obj.get("theme").and_then(Value::as_str))
        .unwrap_or("dark")
        .to_string();
      if !deck_obj.contains_key("theme") {
        deck_obj.insert("theme".to_string(), Value::String(theme.clone()));
      }

      let arch_path = path_or_str_input(inputs, "arch_image");
      let arch_title = str_input(inputs, "arch_title")
        .unwrap_or("系统核心架构图 (D2)")
        .to_string();

      let slides_arr = deck_obj
        .get_mut("slides")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| self.input_error("'deck.slides' must be an array"))?;

      let mut jobs: Vec<DeckIllustrationJob> = Vec::new();
      for (idx, slide_val) in slides_arr.iter_mut().enumerate() {
        let Some(s_obj) = slide_val.as_object_mut() else {
          continue;
        };
        let slide_title = s_obj
          .get("title")
          .and_then(Value::as_str)
          .unwrap_or("主题配图")
          .to_string();
        let vmode = s_obj
          .get("visual_mode")
          .and_then(Value::as_str)
          .unwrap_or("");

        // Wire upstream D2 / architecture figure onto `arch_diagram` slides
        if vmode == "arch_diagram" {
          if let Some(ref ap) = arch_path {
            let cap = str_input(inputs, "arch_caption")
              .unwrap_or(slide_title.as_str())
              .to_string();
            s_obj.insert(
              "right".to_string(),
              json!({
                "kind": "image",
                "title": arch_title,
                "src": ap,
                "caption": cap,
              }),
            );
          } else {
            apply_fallback_right(s_obj);
          }
          continue;
        }

        let ill_prompt = s_obj
          .get("illustration_prompt")
          .and_then(Value::as_str)
          .map(str::trim)
          .filter(|s| !s.is_empty());
        let wants_illustration =
          vmode == "illustration" || (vmode.is_empty() && ill_prompt.is_some());

        if wants_illustration
          && enabled
          && !image_model.trim().is_empty()
          && jobs.len() < max_images
          && let Some(concept) = ill_prompt
        {
          let caption = s_obj
            .get("illustration_caption")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(slide_title.as_str())
            .to_string();
          jobs.push(DeckIllustrationJob {
            slide_index: idx,
            concept: concept.to_string(),
            title: "主题概念插图".to_string(),
            caption,
          });
        } else {
          apply_fallback_right(s_obj);
        }
      }

      let total_jobs = jobs.len();
      let mut generated_entries: Vec<Value> = Vec::with_capacity(total_jobs);

      if total_jobs > 0 {
        // Poll concurrently on the current task so `run_context` task-locals
        // (`ARTIFACT_ROOT`, `COST_METER`, `PROGRESS`) remain active for every call.
        let results: Vec<(
          DeckIllustrationJob,
          Result<(FlowValue, String), YanshiError>,
        )> = futures::stream::iter(jobs.into_iter().map(|job| {
          let im = image_model.as_str();
          let ctx = (!deck_title.is_empty()).then_some(deck_title.as_str());
          let cs = custom_style.as_deref();
          let th = theme.as_str();
          let sz = size.as_str();
          let pm = prompt_model.as_deref();
          async move {
            let res = self
              .generate_one_illustration(im, &job.concept, ctx, preset, cs, th, sz, refine, pm)
              .await;
            (job, res)
          }
        }))
        .buffer_unordered(max_concurrent)
        .collect()
        .await;

        // Sort back by slide_index for deterministic output ordering
        let mut ordered = results;
        ordered.sort_by_key(|(j, _)| j.slide_index);

        for (done_idx, (job, outcome)) in ordered.into_iter().enumerate() {
          match outcome {
            Ok((file_val, final_prompt)) => {
              let src_str = match &file_val {
                FlowValue::File { path, .. } => path.to_string_lossy().into_owned(),
                FlowValue::Json(Value::String(s)) => s.clone(),
                _ => String::new(),
              };
              if let Some(s_obj) = slides_arr
                .get_mut(job.slide_index)
                .and_then(Value::as_object_mut)
              {
                s_obj.insert(
                  "right".to_string(),
                  json!({
                    "kind": "image",
                    "title": job.title,
                    "src": src_str,
                    "caption": job.caption,
                  }),
                );
              }
              run_context::report_progress(
                "illustration_generated",
                json!({
                  "slide_index": job.slide_index,
                  "completed": done_idx + 1,
                  "total": total_jobs,
                }),
              );
              generated_entries.push(json!({
                "slide_index": job.slide_index,
                "path": src_str,
                "caption": job.caption,
                "prompt": final_prompt,
              }));
            }
            Err(err @ YanshiError::CostLimitExceeded { .. }) => return Err(err),
            Err(err) if graceful => {
              tracing::warn!(
                node = %self.id,
                slide_index = job.slide_index,
                error = %err,
                "slide illustration failed; keeping structured fallback panel"
              );
              run_context::report_progress(
                "illustration_fallback",
                json!({
                  "slide_index": job.slide_index,
                  "error": err.to_string(),
                }),
              );
              if let Some(s_obj) = slides_arr
                .get_mut(job.slide_index)
                .and_then(Value::as_object_mut)
              {
                apply_fallback_right(s_obj);
              }
            }
            Err(err) => return Err(err),
          }
        }
      }

      let count = generated_entries.len();
      return Ok(HashMap::from([
        (format!("{}_deck", self.id), FlowValue::Json(deck_val)),
        (
          format!("{}_images", self.id),
          FlowValue::Json(Value::Array(generated_entries)),
        ),
        (format!("{}_count", self.id), FlowValue::Json(json!(count))),
      ]));
    }

    // ── Mode B2: Explicit Batch List (`items` input) ───────────────────
    if let Some(items_val) = json_input(&self.id, inputs, "items").await? {
      let arr = items_val
        .as_array()
        .ok_or_else(|| self.input_error("'items' must be a JSON array"))?;
      let theme = str_input(inputs, "theme")
        .or(self.theme.as_deref())
        .unwrap_or("dark")
        .to_string();
      let context = str_input(inputs, "context").map(str::to_string);

      if !enabled || image_model.trim().is_empty() || arr.is_empty() || max_images == 0 {
        return Ok(HashMap::from([
          (format!("{}_images", self.id), FlowValue::Json(json!([]))),
          (format!("{}_count", self.id), FlowValue::Json(json!(0))),
        ]));
      }

      let mut jobs: Vec<(usize, String, String)> = Vec::new();
      for (idx, item) in arr.iter().take(max_images).enumerate() {
        match item {
          Value::String(s) if !s.trim().is_empty() => {
            jobs.push((idx, s.trim().to_string(), String::new()));
          }
          Value::Object(map) => {
            let concept = map
              .get("concept")
              .or_else(|| map.get("visual"))
              .or_else(|| map.get("prompt"))
              .and_then(Value::as_str)
              .map(str::trim)
              .filter(|s| !s.is_empty());
            if let Some(c) = concept {
              let slide_idx = map
                .get("slide_index")
                .and_then(Value::as_u64)
                .map(|n| n as usize)
                .unwrap_or(idx);
              let cap = map
                .get("caption")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
              jobs.push((slide_idx, c.to_string(), cap));
            }
          }
          _ => {}
        }
      }

      let results: Vec<(
        (usize, String, String),
        Result<(FlowValue, String), YanshiError>,
      )> = futures::stream::iter(jobs.into_iter().map(|job| {
        let im = image_model.as_str();
        let ctx = context.as_deref();
        let cs = custom_style.as_deref();
        let th = theme.as_str();
        let sz = size.as_str();
        let pm = prompt_model.as_deref();
        async move {
          let res = self
            .generate_one_illustration(im, &job.1, ctx, preset, cs, th, sz, refine, pm)
            .await;
          (job, res)
        }
      }))
      .buffer_unordered(max_concurrent)
      .collect()
      .await;

      let mut ordered = results;
      ordered.sort_by_key(|((idx, _, _), _)| *idx);
      let mut out_items = Vec::with_capacity(ordered.len());
      for ((idx, _, cap), outcome) in ordered {
        match outcome {
          Ok((file_val, final_prompt)) => {
            let src_str = match &file_val {
              FlowValue::File { path, .. } => path.to_string_lossy().into_owned(),
              FlowValue::Json(Value::String(s)) => s.clone(),
              _ => String::new(),
            };
            out_items.push(json!({
              "slide_index": idx,
              "path": src_str,
              "caption": cap,
              "prompt": final_prompt,
            }));
          }
          Err(err @ YanshiError::CostLimitExceeded { .. }) => return Err(err),
          Err(err) if graceful => {
            tracing::warn!(node = %self.id, index = idx, error = %err, "batch illustration failed");
          }
          Err(err) => return Err(err),
        }
      }
      let count = out_items.len();
      return Ok(HashMap::from([
        (
          format!("{}_images", self.id),
          FlowValue::Json(Value::Array(out_items)),
        ),
        (format!("{}_count", self.id), FlowValue::Json(json!(count))),
      ]));
    }

    // ── Mode A: Single Illustration (`concept` or `prompt` input) ──────
    let concept = str_input(inputs, "concept")
      .or_else(|| str_input(inputs, "prompt"))
      .ok_or_else(|| {
        self.input_error(
          "provide 'concept' (or 'prompt') for single mode, or 'deck'/'items' for batch mode",
        )
      })?;
    if image_model.trim().is_empty() {
      return Err(self.input_error("missing 'model' (text_to_image model name)"));
    }
    let context = str_input(inputs, "context");
    let theme = str_input(inputs, "theme")
      .or(self.theme.as_deref())
      .unwrap_or("dark");

    let (file_val, final_prompt) = self
      .generate_one_illustration(
        &image_model,
        concept,
        context,
        preset,
        custom_style.as_deref(),
        theme,
        &size,
        refine,
        prompt_model.as_deref(),
      )
      .await?;

    Ok(HashMap::from([
      (format!("{}_image", self.id), file_val),
      (
        format!("{}_prompt", self.id),
        FlowValue::Json(Value::String(final_prompt)),
      ),
    ]))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn art_prompt_includes_preset_theme_and_anti_text_guardrail() {
    let prompt = compose_art_prompt(
      "Zero-copy ring buffer shared memory between producer and consumer cores",
      Some("Linux io_uring Architecture"),
      StylePreset::Isometric3d,
      Some("subtle glowing data packets"),
      "dark",
    );
    assert!(prompt.contains("isometric 3D"));
    assert!(prompt.contains("#0B1120"));
    assert!(prompt.contains("NO embedded text"));
    assert!(prompt.contains("subtle glowing data packets"));
  }

  #[tokio::test]
  async fn graceful_fallback_and_disabled_mode_preserve_fallback_panels() {
    let raw_deck = json!({
      "title": "Fallback Verification",
      "slides": [
        {
          "title": "Resilient Slide",
          "left": {"kind": "bullets", "items": [{"heading": "H", "body": "B"}]},
          "visual_mode": "illustration",
          "illustration_prompt": "Abstract distributed consensus ring",
          "fallback_right": {"kind": "bullets", "items": [{"heading": "Backup", "body": "Kept"}]}
        }
      ]
    });

    // 1. `enabled: false` skips vendor calls completely and promotes `fallback_right`
    let disabled_out = IllustrationNode::new("ill")
      .with_model("nonexistent-model")
      .with_enabled(false)
      .execute(&HashMap::from([(
        "deck".to_string(),
        FlowValue::Json(raw_deck.clone()),
      )]))
      .await
      .expect("disabled succeeds");
    assert_eq!(disabled_out["ill_count"], FlowValue::Json(json!(0)));
    let FlowValue::Json(d1) = &disabled_out["ill_deck"] else {
      panic!("deck json");
    };
    assert_eq!(d1["slides"][0]["right"]["items"][0]["heading"], "Backup");

    // 2. `graceful_fallback: true` catches bad/failing image model and promotes `fallback_right`
    let fallback_out = IllustrationNode::new("ill")
      .with_model("definitely-unregistered-image-model")
      .with_graceful_fallback(true)
      .execute(&HashMap::from([(
        "deck".to_string(),
        FlowValue::Json(raw_deck.clone()),
      )]))
      .await
      .expect("graceful fallback succeeds");
    assert_eq!(fallback_out["ill_count"], FlowValue::Json(json!(0)));
    let FlowValue::Json(d2) = &fallback_out["ill_deck"] else {
      panic!("deck json");
    };
    assert_eq!(d2["slides"][0]["right"]["items"][0]["heading"], "Backup");

    // 3. `graceful_fallback: false` fails loudly as expected
    let strict_err = IllustrationNode::new("ill")
      .with_model("definitely-unregistered-image-model")
      .with_graceful_fallback(false)
      .execute(&HashMap::from([(
        "deck".to_string(),
        FlowValue::Json(raw_deck),
      )]))
      .await;
    assert!(strict_err.is_err());
  }
}
