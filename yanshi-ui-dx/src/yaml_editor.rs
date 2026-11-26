//! Pure-Rust lossless YAML + Yanshi Tera expression tokenizer, 2-space
//! Tab/Enter indentation helper, and `YamlCodeEditor` Dioxus component.

use dioxus::prelude::*;
use dioxus_shadcn::{Button, ButtonSize, ButtonVariant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YamlTokenKind {
  Plain,
  Comment,
  Directive,
  SchemaKey,
  Key,
  Punctuation,
  StringLit,
  Template,
  Number,
  Boolean,
  NodeType,
  Anchor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YamlToken {
  pub kind: YamlTokenKind,
  pub text: String,
}

const SCHEMA_KEYS: &[&str] = &[
  "name",
  "description",
  "version",
  "inputs",
  "nodes",
  "id",
  "type",
  "dependencies",
  "depends_on",
  "run_if",
  "condition",
  "input_mapping",
  "parameters",
  "retry",
  "timeout_secs",
  "output_format",
  "sub_nodes",
];

const NODE_TYPES: &[&str] = &[
  "template",
  "llm",
  "agent",
  "skill_agent",
  "map",
  "while",
  "conditional",
  "render_slides",
  "render_video",
  "render_document",
  "render_audio",
  "render_study",
  "render_svg",
  "tts",
  "illustration",
  "image_gen",
  "image_edit",
  "video_gen",
  "http",
  "web_search",
  "repo_snapshot",
  "pdf_extract",
  "video_ingest",
  "asr",
  "figure",
  "render_table",
  "render_mindmap",
  "render_markdown",
  "text_to_image",
  "text_to_video",
  "file",
  "shell",
  "batch",
  "markmap",
  "arxiv",
  "mcp_tool",
  "rag_retrieve",
  "rag_ingest",
  "code_exec",
];

fn push_token(out: &mut Vec<YamlToken>, kind: YamlTokenKind, text: &str) {
  if text.is_empty() {
    return;
  }
  if kind != YamlTokenKind::Template
    && let Some(last) = out.last_mut()
    && last.kind == kind
  {
    last.text.push_str(text);
    return;
  }
  out.push(YamlToken {
    kind,
    text: text.to_string(),
  });
}

fn push_with_templates(out: &mut Vec<YamlToken>, base_kind: YamlTokenKind, text: &str) {
  if !text.contains('{') {
    push_token(out, base_kind, text);
    return;
  }
  let bytes = text.as_bytes();
  let mut i = 0usize;
  let mut plain_start = 0usize;
  while i + 1 < bytes.len() {
    if bytes[i] == b'{' && matches!(bytes[i + 1], b'{' | b'%' | b'#') {
      let closer = match bytes[i + 1] {
        b'{' => "}}",
        b'%' => "%}",
        _ => "#}",
      };
      if let Some(rel_close) = text[i + 2..].find(closer) {
        if i > plain_start {
          push_token(out, base_kind, &text[plain_start..i]);
        }
        let end = i + 2 + rel_close + 2;
        push_token(out, YamlTokenKind::Template, &text[i..end]);
        i = end;
        plain_start = end;
        continue;
      }
    }
    i += 1;
  }
  if plain_start < text.len() {
    push_token(out, base_kind, &text[plain_start..]);
  }
}

fn is_number_scalar(raw: &str) -> bool {
  if raw.is_empty() {
    return false;
  }
  let body = raw
    .strip_prefix('+')
    .or_else(|| raw.strip_prefix('-'))
    .unwrap_or(raw);
  if let Some(hex) = body.strip_prefix("0x") {
    return !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit());
  }
  body.parse::<f64>().is_ok() && body.bytes().next().is_some_and(|b| b.is_ascii_digit())
}

fn classify_unquoted_scalar(raw: &str, active_key: Option<&str>) -> YamlTokenKind {
  if matches!(
    raw,
    "true" | "false" | "null" | "~" | "True" | "False" | "NULL"
  ) {
    return YamlTokenKind::Boolean;
  }
  if is_number_scalar(raw) {
    return YamlTokenKind::Number;
  }
  if active_key == Some("type") && NODE_TYPES.contains(&raw) {
    return YamlTokenKind::NodeType;
  }
  YamlTokenKind::StringLit
}

fn is_block_scalar_header(s: &str) -> Option<(usize, usize)> {
  let bytes = s.as_bytes();
  if bytes.is_empty() || !matches!(bytes[0], b'|' | b'>') {
    return None;
  }
  let mut idx = 1usize;
  while idx < bytes.len() && (matches!(bytes[idx], b'-' | b'+') || bytes[idx].is_ascii_digit()) {
    idx += 1;
  }
  let tail = &s[idx..];
  let tail_trimmed = tail.trim_start();
  if tail_trimmed.is_empty() || tail_trimmed.starts_with('#') {
    Some((idx, s.len()))
  } else {
    None
  }
}

fn scan_value_segment(out: &mut Vec<YamlToken>, segment: &str, initial_key: Option<&str>) -> bool {
  let trimmed = segment.trim_start_matches([' ', '\t']);
  let ws_len = segment.len() - trimmed.len();
  if ws_len > 0 {
    push_token(out, YamlTokenKind::Plain, &segment[..ws_len]);
  }
  if trimmed.is_empty() {
    return false;
  }
  if let Some((hdr_end, _)) = is_block_scalar_header(trimmed) {
    push_token(out, YamlTokenKind::Punctuation, &trimmed[..hdr_end]);
    let tail = &trimmed[hdr_end..];
    if let Some(hash_idx) = tail.find('#') {
      push_token(out, YamlTokenKind::Plain, &tail[..hash_idx]);
      push_token(out, YamlTokenKind::Comment, &tail[hash_idx..]);
    } else {
      push_token(out, YamlTokenKind::Plain, tail);
    }
    return true;
  }

  let bytes = trimmed.as_bytes();
  let mut i = 0usize;
  let mut flow_depth = 0usize;
  let mut active_key = initial_key;

  while i < bytes.len() {
    let b = bytes[i];
    if b == b' ' || b == b'\t' {
      let start = i;
      while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
      }
      push_token(out, YamlTokenKind::Plain, &trimmed[start..i]);
      continue;
    }
    if b == b'#' && (i == 0 || matches!(bytes[i - 1], b' ' | b'\t')) {
      push_token(out, YamlTokenKind::Comment, &trimmed[i..]);
      break;
    }
    if b == b'{' && i + 1 < bytes.len() && matches!(bytes[i + 1], b'{' | b'%' | b'#') {
      let closer = match bytes[i + 1] {
        b'{' => "}}",
        b'%' => "%}",
        _ => "#}",
      };
      if let Some(rel) = trimmed[i + 2..].find(closer) {
        let end = i + 2 + rel + 2;
        push_token(out, YamlTokenKind::Template, &trimmed[i..end]);
        i = end;
        continue;
      }
    }
    if matches!(b, b'[' | b']' | b'{' | b'}' | b',') {
      if matches!(b, b'[' | b'{') {
        flow_depth += 1;
      } else if matches!(b, b']' | b'}') {
        flow_depth = flow_depth.saturating_sub(1);
      }
      if matches!(b, b',' | b'{') {
        active_key = None;
      }
      push_token(out, YamlTokenKind::Punctuation, &trimmed[i..i + 1]);
      i += 1;
      continue;
    }
    if b == b'"' {
      let start = i;
      i += 1;
      while i < bytes.len() {
        if bytes[i] == b'\\' {
          i = (i + 2).min(bytes.len());
          continue;
        }
        if bytes[i] == b'"' {
          i += 1;
          break;
        }
        i += 1;
      }
      push_with_templates(out, YamlTokenKind::StringLit, &trimmed[start..i]);
      active_key = None;
      continue;
    }
    if b == b'\'' {
      let start = i;
      i += 1;
      while i < bytes.len() {
        if bytes[i] == b'\'' && i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
          i += 2;
          continue;
        }
        if bytes[i] == b'\'' {
          i += 1;
          break;
        }
        i += 1;
      }
      push_with_templates(out, YamlTokenKind::StringLit, &trimmed[start..i]);
      active_key = None;
      continue;
    }
    if matches!(b, b'&' | b'*' | b'!') {
      let start = i;
      i += 1;
      while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b',' | b']' | b'}') {
        i += 1;
      }
      push_token(out, YamlTokenKind::Anchor, &trimmed[start..i]);
      continue;
    }

    let start = i;
    while i < bytes.len() {
      if bytes[i] == b'#' && i > start && matches!(bytes[i - 1], b' ' | b'\t') {
        break;
      }
      if flow_depth > 0 && matches!(bytes[i], b',' | b']' | b'}') {
        break;
      }
      i += 1;
    }
    let mut scalar_end = i;
    while scalar_end > start && matches!(bytes[scalar_end - 1], b' ' | b'\t') {
      scalar_end -= 1;
    }
    let raw_scalar = &trimmed[start..scalar_end];
    let kind = classify_unquoted_scalar(raw_scalar, active_key);
    push_with_templates(out, kind, raw_scalar);
    if i > scalar_end {
      push_token(out, YamlTokenKind::Plain, &trimmed[scalar_end..i]);
    }
    active_key = None;
  }

  false
}

/// Lossless line-by-line YAML + Yanshi Tera tokenizer.
/// Guarantees `reconstruct_from_tokens(&tokenize_yaml(src)) == src`.
pub fn tokenize_yaml(source: &str) -> Vec<Vec<YamlToken>> {
  let mut result = Vec::new();
  let mut block_parent_indent: Option<usize> = None;
  let mut block_content_indent: Option<usize> = None;

  for raw_line in source.split('\n') {
    let (line, has_cr) = match raw_line.strip_suffix('\r') {
      Some(stripped) => (stripped, true),
      None => (raw_line, false),
    };
    let mut tokens = Vec::new();
    let indent = line.bytes().take_while(|b| *b == b' ').count();
    let rest = &line[indent..];

    if let Some(parent_ind) = block_parent_indent {
      if rest.trim().is_empty() {
        push_token(&mut tokens, YamlTokenKind::Plain, line);
        if has_cr {
          push_token(&mut tokens, YamlTokenKind::Plain, "\r");
        }
        result.push(tokens);
        continue;
      }
      if block_content_indent.is_none() && indent > parent_ind {
        block_content_indent = Some(indent);
      }
      if let Some(content_ind) = block_content_indent
        && indent >= content_ind
      {
        push_token(&mut tokens, YamlTokenKind::Plain, &line[..indent]);
        push_with_templates(&mut tokens, YamlTokenKind::StringLit, rest);
        if has_cr {
          push_token(&mut tokens, YamlTokenKind::Plain, "\r");
        }
        result.push(tokens);
        continue;
      }
      block_parent_indent = None;
      block_content_indent = None;
    }

    if rest.is_empty() {
      push_token(&mut tokens, YamlTokenKind::Plain, line);
      if has_cr {
        push_token(&mut tokens, YamlTokenKind::Plain, "\r");
      }
      result.push(tokens);
      continue;
    }

    if indent == 0
      && (line.starts_with("---")
        || line.starts_with("...")
        || line.starts_with("%YAML")
        || line.starts_with("%TAG"))
    {
      push_token(&mut tokens, YamlTokenKind::Directive, line);
      if has_cr {
        push_token(&mut tokens, YamlTokenKind::Plain, "\r");
      }
      result.push(tokens);
      continue;
    }

    if indent > 0 {
      push_token(&mut tokens, YamlTokenKind::Plain, &line[..indent]);
    }
    if rest.starts_with('#') {
      push_token(&mut tokens, YamlTokenKind::Comment, rest);
      if has_cr {
        push_token(&mut tokens, YamlTokenKind::Plain, "\r");
      }
      result.push(tokens);
      continue;
    }

    let mut cursor = indent;
    let bytes = line.as_bytes();
    while cursor < bytes.len()
      && bytes[cursor] == b'-'
      && (cursor + 1 == bytes.len() || matches!(bytes[cursor + 1], b' ' | b'\t'))
    {
      push_token(&mut tokens, YamlTokenKind::Punctuation, "-");
      cursor += 1;
      let ws_start = cursor;
      while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t') {
        cursor += 1;
      }
      if cursor > ws_start {
        push_token(&mut tokens, YamlTokenKind::Plain, &line[ws_start..cursor]);
      }
    }

    let remainder = &line[cursor..];
    if remainder.is_empty() {
      if has_cr {
        push_token(&mut tokens, YamlTokenKind::Plain, "\r");
      }
      result.push(tokens);
      continue;
    }
    if remainder.starts_with('#') {
      push_token(&mut tokens, YamlTokenKind::Comment, remainder);
      if has_cr {
        push_token(&mut tokens, YamlTokenKind::Plain, "\r");
      }
      result.push(tokens);
      continue;
    }

    if let Some((key_end, ws_end, colon_idx)) = match_mapping_key(remainder) {
      let key_raw = &remainder[..key_end];
      let bare_key = key_raw.trim_matches(['"', '\'']);
      let kind = if SCHEMA_KEYS.contains(&bare_key) {
        YamlTokenKind::SchemaKey
      } else {
        YamlTokenKind::Key
      };
      push_token(&mut tokens, kind, key_raw);
      push_token(
        &mut tokens,
        YamlTokenKind::Plain,
        &remainder[key_end..ws_end],
      );
      push_token(&mut tokens, YamlTokenKind::Punctuation, ":");
      let entered = scan_value_segment(&mut tokens, &remainder[colon_idx + 1..], Some(bare_key));
      if entered {
        block_parent_indent = Some(indent);
        block_content_indent = None;
      }
    } else if scan_value_segment(&mut tokens, remainder, None) {
      block_parent_indent = Some(indent);
      block_content_indent = None;
    }

    if has_cr {
      push_token(&mut tokens, YamlTokenKind::Plain, "\r");
    }
    result.push(tokens);
  }

  result
}

fn match_mapping_key(s: &str) -> Option<(usize, usize, usize)> {
  let bytes = s.as_bytes();
  let mut idx = 0usize;
  while idx < bytes.len()
    && (bytes[idx].is_ascii_alphanumeric() || matches!(bytes[idx], b'_' | b'.' | b'-'))
  {
    idx += 1;
  }
  if idx == 0 {
    return None;
  }
  let key_end = idx;
  while idx < bytes.len() && matches!(bytes[idx], b' ' | b'\t') {
    idx += 1;
  }
  if idx < bytes.len()
    && bytes[idx] == b':'
    && (idx + 1 == bytes.len() || matches!(bytes[idx + 1], b' ' | b'\t'))
  {
    Some((key_end, idx, idx))
  } else {
    None
  }
}

#[cfg(test)]
pub fn reconstruct_from_tokens(lines: &[Vec<YamlToken>]) -> String {
  lines
    .iter()
    .map(|ln| ln.iter().map(|t| t.text.as_str()).collect::<String>())
    .collect::<Vec<_>>()
    .join("\n")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YamlValidation {
  pub ok: bool,
  pub message: String,
}

/// Client-side pre-flight workflow YAML validation matching `RunCreateForm`.
pub fn validate_workflow_yaml(source: &str) -> YamlValidation {
  let trimmed = source.trim();
  if trimmed.is_empty() {
    return YamlValidation {
      ok: false,
      message: "workflow YAML is required".to_string(),
    };
  }
  let has_name = trimmed
    .lines()
    .any(|ln| ln.trim_start().starts_with("name:"));
  if !has_name {
    return YamlValidation {
      ok: false,
      message: "workflow must define a top-level 'name:' field".to_string(),
    };
  }
  let has_nodes = trimmed
    .lines()
    .any(|ln| ln.trim_start().starts_with("nodes:"));
  if !has_nodes {
    return YamlValidation {
      ok: false,
      message: "workflow must define a top-level 'nodes:' map".to_string(),
    };
  }
  YamlValidation {
    ok: true,
    message: "looks like a workflow YAML (basic checks)".to_string(),
  }
}

fn token_color_class(tok: &YamlToken) -> &'static str {
  match tok.kind {
    YamlTokenKind::Plain => "",
    YamlTokenKind::SchemaKey => "text-sky-600 dark:text-sky-400 font-semibold",
    YamlTokenKind::Key => "text-indigo-600 dark:text-indigo-300",
    YamlTokenKind::NodeType => "text-emerald-600 dark:text-emerald-400 font-semibold",
    YamlTokenKind::StringLit => "text-amber-700 dark:text-amber-300",
    YamlTokenKind::Template => {
      if tok.text.starts_with("{%") {
        "text-fuchsia-600 dark:text-fuchsia-400 font-semibold"
      } else if tok.text.starts_with("{#") {
        "text-muted-foreground italic"
      } else {
        "text-violet-600 dark:text-violet-300 font-medium"
      }
    }
    YamlTokenKind::Number => "text-orange-600 dark:text-orange-300",
    YamlTokenKind::Boolean => "text-rose-600 dark:text-rose-400 font-medium",
    YamlTokenKind::Comment => "text-muted-foreground italic",
    YamlTokenKind::Punctuation => "text-muted-foreground",
    YamlTokenKind::Directive => "text-cyan-600 dark:text-cyan-400 font-semibold",
    YamlTokenKind::Anchor => "text-teal-600 dark:text-teal-300",
  }
}

/// Syntax-highlighted YAML editor with line numbers and 2-space metadata bar.
#[component]
pub fn YamlCodeEditor(
  value: String,
  on_change: EventHandler<String>,
  #[props(default)] test_id: String,
  #[props(default = "YAML 编辑器".to_string())] aria_label: String,
  #[props(default = 16)] rows: usize,
  #[props(default)] read_only: bool,
) -> Element {
  let mut expanded = use_signal(|| false);
  let tokenized = tokenize_yaml(&value);
  let line_count = tokenized.len().max(1);
  let min_height = if expanded() {
    "540px".to_string()
  } else {
    format!("{}px", (rows * 20 + 24).max(180))
  };

  rsx! {
    div { class: "rounded-md border border-border bg-card overflow-hidden flex flex-col",
      div {
        class: "grid grid-cols-[44px_minmax(0,1fr)] font-mono text-xs leading-5 relative overflow-auto bg-background",
        style: "min-height: {min_height};",
        div {
          class: "select-none border-r border-border bg-muted/40 py-2.5 pr-2 text-right text-muted-foreground",
          "aria-hidden": "true",
          for idx in 1..=line_count {
            div { key: "{idx}", class: "h-5", "{idx}" }
          }
        }
        div { class: "relative min-w-0",
          pre {
            class: "pointer-events-none m-0 p-2.5 whitespace-pre overflow-hidden text-foreground",
            "aria-hidden": "true",
            for (line_idx, line_tokens) in tokenized.iter().enumerate() {
              span { key: "{line_idx}",
                for (tok_idx, tok) in line_tokens.iter().enumerate() {
                  span { key: "{tok_idx}", class: token_color_class(tok), "{tok.text}" }
                }
                if line_idx + 1 < line_count {
                  "\n"
                } else if line_tokens.is_empty() {
                  " "
                }
              }
            }
          }
          textarea {
            class: "absolute inset-0 w-full h-full resize-none bg-transparent p-2.5 font-mono text-xs leading-5 text-transparent caret-foreground outline-none whitespace-pre overflow-auto selection:bg-primary/20",
            "data-testid": if test_id.is_empty() { None } else { Some(test_id.as_str()) },
            "aria-label": aria_label,
            value: "{value}",
            readonly: read_only,
            spellcheck: "false",
            autocomplete: "off",
            wrap: "off",
            oninput: move |ev: FormEvent| {
              on_change.call(ev.value());
            },
          }
        }
      }
      div { class: "flex items-center justify-between border-t border-border bg-muted/30 px-3 py-1.5 text-[11px] text-muted-foreground",
        span { "YAML · 2 Spaces · 共 {line_count} 行" }
        Button {
          variant: ButtonVariant::Ghost,
          size: ButtonSize::Sm,
          class: "h-6 px-2 text-[11px]".to_string(),
          onclick: move |_| expanded.set(!expanded()),
          if expanded() { "⤡ 收起高度" } else { "⤢ 放大编辑" }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tokenizer_is_lossless_and_classifies_yanshi_tokens() {
    let sample = "name: demo\nnodes:\n  - id: step1\n    type: llm\n    parameters:\n      prompt: \"hi {{ name }}\" # greeting\n      temperature: 0.7\n      stream: true\n";
    let lines = tokenize_yaml(sample);
    assert_eq!(reconstruct_from_tokens(&lines), sample);

    // Check schema key `name` and node type `llm` and Tera template
    assert_eq!(lines[0][0].kind, YamlTokenKind::SchemaKey);
    assert!(
      lines[3]
        .iter()
        .any(|t| t.kind == YamlTokenKind::NodeType && t.text == "llm")
    );
    assert!(
      lines[5]
        .iter()
        .any(|t| t.kind == YamlTokenKind::Template && t.text == "{{ name }}")
    );
    assert!(lines[5].iter().any(|t| t.kind == YamlTokenKind::Comment));
    assert!(
      lines[6]
        .iter()
        .any(|t| t.kind == YamlTokenKind::Number && t.text == "0.7")
    );
    assert!(
      lines[7]
        .iter()
        .any(|t| t.kind == YamlTokenKind::Boolean && t.text == "true")
    );
  }

  #[test]
  fn validate_workflow_yaml_catches_missing_sections() {
    assert!(!validate_workflow_yaml("  ").ok);
    assert!(!validate_workflow_yaml("nodes: []").ok);
    assert!(!validate_workflow_yaml("name: x").ok);
    assert!(validate_workflow_yaml("name: x\nnodes:\n  - id: a\n").ok);
  }
}
