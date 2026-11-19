pub mod agent;
pub mod audio;
pub mod backup;
pub mod cleanup;
pub mod config;
pub mod doctor;
pub mod eval;
#[cfg(any(not(feature = "plugin"), not(feature = "rag")))]
pub mod feature_unavailable;
pub mod harness;
pub mod image;
pub mod llm;
pub mod marketplace;
pub mod mcp;
pub mod memory;
pub mod notebook;
#[cfg(feature = "plugin")]
pub mod plugin;
pub mod rag;
pub mod repl;
pub mod restore;
pub mod serve;
pub mod skill;
pub mod trace;
pub mod video;
pub mod workflow;

/// Parse `--input` CLI tokens accepting both `--input KEY VALUE` (2 tokens)
/// and `--input KEY=VALUE` / `--input=KEY=VALUE` (1 token) (`F-A4-2`).
pub fn parse_input_flag_tokens(tokens: &[String]) -> anyhow::Result<Vec<(String, String)>> {
  let mut pairs = Vec::new();
  let mut idx = 0;
  while idx < tokens.len() {
    let head = &tokens[idx];
    if let Some((raw_key, raw_val)) = head.split_once('=') {
      let key = raw_key.trim();
      if key.is_empty() {
        anyhow::bail!("--input '{head}' has an empty key (expected KEY=VALUE or KEY VALUE)");
      }
      pairs.push((key.to_string(), raw_val.to_string()));
      idx += 1;
    } else {
      let key = head.trim();
      if key.is_empty() {
        anyhow::bail!("--input key cannot be empty");
      }
      let Some(val) = tokens.get(idx + 1) else {
        anyhow::bail!(
          "--input '{head}' is missing its value (use '--input {head} <VALUE>' or '--input {head}=<VALUE>')"
        );
      };
      pairs.push((key.to_string(), val.clone()));
      idx += 2;
    }
  }
  Ok(pairs)
}

#[cfg(test)]
mod tests {
  use super::parse_input_flag_tokens;

  #[test]
  fn parses_equals_two_arg_and_mixed_input_tokens() {
    let tokens = vec![
      "url=https://example.org?a=1&b=2".to_string(),
      "sections".to_string(),
      "4".to_string(),
      "filter".to_string(),
      "x=y".to_string(),
    ];
    let parsed = parse_input_flag_tokens(&tokens).expect("valid tokens");
    assert_eq!(
      parsed,
      vec![
        ("url".into(), "https://example.org?a=1&b=2".into()),
        ("sections".into(), "4".into()),
        ("filter".into(), "x=y".into()),
      ]
    );
  }

  #[test]
  fn rejects_missing_value_and_empty_key() {
    let lonely = vec!["audio_source".to_string()];
    assert!(parse_input_flag_tokens(&lonely).is_err());

    let empty_key = vec!["=value".to_string()];
    assert!(parse_input_flag_tokens(&empty_key).is_err());
  }
}
