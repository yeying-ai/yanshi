//! Live integration suite for DeepSeek `/v1/tools/{search,fetch,ocr}`.
//!
//! Offline `cargo test` skips cleanly when `YANSHI_LIVE_LLM_TESTS` is unset or
//! `DEEPSEEK_API_KEY` is missing, matching `yanshi-llm/tests/provider_consistency_live.rs`.

use std::sync::Arc;

use serde_json::json;
use yanshi_tools::builtin::{DeepSeekOcrPurpose, DeepSeekToolsClient, SearchConfig, WebSearchTool};
use yanshi_tools::sandbox::SandboxPolicy;

/// Valid 32x32 solid-red RGB PNG (strictly valid IHDR + zlib IDAT + CRC).
const RED_PNG_32: &[u8] = &[
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
  0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x20, 0x08, 0x02, 0x00, 0x00, 0x00, 0xfc, 0x18, 0xed,
  0xa3, 0x00, 0x00, 0x00, 0x27, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0xed, 0xcd, 0xb1, 0x09, 0x00,
  0x00, 0x08, 0xc0, 0xb0, 0xfe, 0xff, 0xb4, 0x5e, 0xe1, 0x20, 0x04, 0xb2, 0xa7, 0xa9, 0x53, 0x09,
  0x04, 0x02, 0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x82, 0x2f, 0xc1, 0x02, 0x32, 0xc3, 0xfc, 0x2e,
  0x70, 0xfd, 0x23, 0x3d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

fn live_enabled() -> Option<String> {
  let gate = std::env::var("YANSHI_LIVE_LLM_TESTS").unwrap_or_default();
  if !matches!(
    gate.trim().to_ascii_lowercase().as_str(),
    "1" | "true" | "yes" | "on"
  ) {
    eprintln!("[live] deepseek_tools_live: skipped (YANSHI_LIVE_LLM_TESTS not enabled)");
    return None;
  }
  let key = std::env::var("DEEPSEEK_API_KEY").unwrap_or_default();
  let trimmed = key.trim().to_string();
  if trimmed.is_empty() {
    eprintln!("[live] deepseek_tools_live: skipped (DEEPSEEK_API_KEY missing)");
    return None;
  }
  Some(trimmed)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deepseek_native_search_fetch_and_ocr_live() {
  let Some(api_key) = live_enabled() else {
    return;
  };

  // Detect local TUN Fake-IP DNS (RFC 2544 `198.18.0.0/15`, used by Clash/Surge
  // on developer workstations) so the live test can exercise `WebSearchTool` and
  // `DeepSeekToolsClient` while keeping loopback/link-local/cloud-metadata denied.
  let has_fake_ip_tun = tokio::net::lookup_host(("api.deepseek.com", 443))
    .await
    .map(|addrs| {
      addrs.into_iter().any(|sa| match sa.ip() {
        std::net::IpAddr::V4(v4) => {
          let o = v4.octets();
          o[0] == 198 && (o[1] & 0xfe) == 18
        }
        std::net::IpAddr::V6(_) => false,
      })
    })
    .unwrap_or(false);
  let policy = Arc::new(SandboxPolicy {
    allow_private_network_access: has_fake_ip_tun,
    ..SandboxPolicy::default()
  });
  let cfg = SearchConfig {
    deepseek_api_key: Some(api_key.clone()),
    ..SearchConfig::default()
  };
  let search_tool = WebSearchTool::new(Arc::clone(&policy))
    .expect("build WebSearchTool")
    .with_config(cfg);
  let search_out = search_tool
    .search(&json!({
      "query": "Rust programming language",
      "provider": "deepseek",
      "max_results": 2
    }))
    .await
    .expect("live DeepSeek web_search succeeded");
  assert_eq!(search_out.provider, "deepseek");
  assert!(
    !search_out.results.is_empty() && search_out.results.len() <= 2,
    "expected 1..=2 search results, got {}",
    search_out.results.len()
  );
  assert!(
    search_out.results[0].url.starts_with("http"),
    "expected valid result URL"
  );

  // 2. POST /v1/tools/fetch via DeepSeekToolsClient
  let client = DeepSeekToolsClient::new(api_key, None)
    .expect("build DeepSeekToolsClient")
    .with_policy(policy);
  let fetched = client
    .fetch_markdown("https://example.com")
    .await
    .expect("live DeepSeek /v1/tools/fetch succeeded");
  assert!(
    !fetched.content.trim().is_empty(),
    "expected non-empty Markdown content from example.com"
  );

  // 3. POST /v1/tools/ocr via DeepSeekToolsClient (valid 32x32 RGB PNG)
  let ocr_resp = client
    .ocr_bytes(
      RED_PNG_32,
      "red_32x32.png",
      DeepSeekOcrPurpose::RichLayoutRecognition,
    )
    .await
    .expect("live DeepSeek /v1/tools/ocr succeeded");
  assert_eq!(ocr_resp.page_count, 1);
  // Grounding tags on pure-image regions (`<｜ref｜>image<｜/ref｜><｜box｜>...<｜/box｜>`)
  // must be stripped cleanly so no raw `<｜ref｜>` or `<｜box｜>` leaks into formatted_text.
  let cleaned = ocr_resp.formatted_text(None);
  assert!(
    !cleaned.contains("<｜ref｜>") && !cleaned.contains("<｜box｜>"),
    "grounding tags leaked into formatted_text: {cleaned:?}"
  );
}
