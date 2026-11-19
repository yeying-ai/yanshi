use std::{
  net::{IpAddr, SocketAddr},
  sync::Arc,
  time::Duration,
};

use async_trait::async_trait;
use reqwest::{Client, StatusCode, Url, header::LOCATION, redirect::Policy};
use serde_json::{Value, json};

use crate::{
  Tool, ToolError, ToolIdempotency, ToolMetadata, ToolOutput,
  sandbox::{NetworkAddressClass, SandboxPolicy},
};

const MAX_REDIRECTS: usize = 10;
const DEFAULT_MAX_TEXT_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const CLOUD_METADATA_HOSTS: &[&str] = &[
  "metadata.google.internal",
  "metadata",
  "instance-data",
  "instance-data.ec2.internal",
];
const CLOUD_METADATA_IPS: &[IpAddr] = &[
  IpAddr::V4(std::net::Ipv4Addr::new(169, 254, 169, 254)),
  IpAddr::V4(std::net::Ipv4Addr::new(169, 254, 170, 2)),
  IpAddr::V4(std::net::Ipv4Addr::new(100, 100, 100, 200)),
  IpAddr::V6(std::net::Ipv6Addr::new(
    0xfd00, 0x0ec2, 0, 0, 0, 0, 0, 0x0254,
  )),
];
const SENSITIVE_REDIRECT_HEADERS: &[&str] = &[
  "authorization",
  "cookie",
  "proxy-authorization",
  "x-api-key",
  "x-goog-api-key",
];

/// Final status and full body of a request made by [`HttpTool::send`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
  pub status: StatusCode,
  pub body: String,
}

/// Final status, optional `Content-Type`, and raw body bytes of a request
/// made by [`HttpTool::send_bytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpBytesResponse {
  pub status: StatusCode,
  pub content_type: Option<String>,
  pub body: Vec<u8>,
}

/// Make HTTP GET / POST requests with domain sandbox enforcement.
pub struct HttpTool {
  policy: Arc<SandboxPolicy>,
  /// Maximum response body size to return (truncate beyond this).
  max_response_chars: usize,
  /// Hard upper bound on response bytes buffered by `send` to prevent OOM.
  max_response_bytes: usize,
  user_agent: String,
  timeout: Duration,
  no_proxy: bool,
}

impl HttpTool {
  /// Perform the request (sandbox checks, pinned DNS, per-hop redirect
  /// re-validation) and return the final status with the text body.
  /// `execute` builds the agent-facing [`ToolOutput`] from this; callers
  /// that need the real status code (the workflow `http` node) use it
  /// directly.
  pub async fn send(&self, params: &Value) -> Result<HttpResponse, ToolError> {
    let raw = self
      .send_bytes(params, Some(self.max_response_bytes))
      .await?;
    Ok(HttpResponse {
      status: raw.status,
      body: String::from_utf8_lossy(&raw.body).into_owned(),
    })
  }

  /// Perform the request with all sandbox/DNS-pinning/redirect checks and
  /// return raw bytes without UTF-8 lossy conversion, optionally enforcing
  /// an upper bound `max_bytes` while streaming the response body.
  pub async fn send_bytes(
    &self,
    params: &Value,
    max_bytes: Option<usize>,
  ) -> Result<HttpBytesResponse, ToolError> {
    let url = params["url"]
      .as_str()
      .ok_or_else(|| ToolError::InvalidParams {
        message: "Missing required parameter 'url'".to_string(),
      })?;

    let mut current_url = Url::parse(url).map_err(|error| ToolError::InvalidParams {
      message: format!("Invalid URL '{}': {}", url, error),
    })?;

    let initial_origin = url_origin_key(&current_url);
    let mut active_method = params["method"].as_str().unwrap_or("GET").to_uppercase();
    let mut include_body = true;

    for redirect_count in 0..=MAX_REDIRECTS {
      let validated_addrs = self.validate_url_allowed(&current_url).await?;
      // V3.3: a redirect hop can land on a different host entirely, so
      // the pinned client is rebuilt fresh every iteration from this
      // iteration's own validated addresses — never hoisted out of the
      // loop or reused across hops.
      let host = current_url
        .host_str()
        .ok_or_else(|| ToolError::InvalidParams {
          message: format!("Cannot parse host from URL: {}", current_url),
        })?;
      let is_ip_literal = matches!(
        current_url.host(),
        Some(url::Host::Ipv4(_) | url::Host::Ipv6(_))
      );
      let client = if is_ip_literal {
        self
          .base_client_builder()
          .build()
          .map_err(|err| ToolError::ExecutionFailed {
            message: format!("failed to build reqwest client for literal '{host}': {err}"),
          })?
      } else {
        self.build_pinned_client(host, &validated_addrs)?
      };

      let mut builder = match active_method.as_str() {
        "GET" => client.get(current_url.clone()),
        "POST" => client.post(current_url.clone()),
        "PUT" => client.put(current_url.clone()),
        "DELETE" => client.delete(current_url.clone()),
        "PATCH" => client.patch(current_url.clone()),
        "HEAD" => client.head(current_url.clone()),
        other => {
          return Err(ToolError::InvalidParams {
            message: format!(
              "Unsupported HTTP method '{}'. Use GET / POST / PUT / DELETE / PATCH / HEAD",
              other
            ),
          });
        }
      };

      let same_origin = url_origin_key(&current_url) == initial_origin;
      if let Some(headers) = params["headers"].as_object() {
        for (k, v) in headers {
          if !same_origin
            && SENSITIVE_REDIRECT_HEADERS
              .iter()
              .any(|sensitive| k.eq_ignore_ascii_case(sensitive))
          {
            continue;
          }
          if let Some(v_str) = v.as_str() {
            builder = builder.header(k.as_str(), v_str);
          }
        }
      }

      // Attach body for methods that can carry one (POST/PUT/PATCH), unless
      // a prior redirect rewrote the method to GET/HEAD.
      if include_body
        && matches!(active_method.as_str(), "POST" | "PUT" | "PATCH")
        && let Some(body) = params["body"].as_str()
      {
        builder = builder.body(body.to_string());
      }

      let mut response = builder.send().await.map_err(|e| ToolError::HttpError {
        message: e.to_string(),
      })?;

      if is_redirect(response.status()) {
        let status = response.status();
        let location = response
          .headers()
          .get(LOCATION)
          .ok_or_else(|| ToolError::HttpError {
            message: format!(
              "HTTP redirect from '{}' did not include Location",
              current_url
            ),
          })?
          .to_str()
          .map_err(|error| ToolError::HttpError {
            message: format!("Invalid redirect Location header: {}", error),
          })?;

        if redirect_count == MAX_REDIRECTS {
          return Err(ToolError::HttpError {
            message: format!("Too many redirects after {}", MAX_REDIRECTS),
          });
        }

        if status == StatusCode::SEE_OTHER {
          if active_method != "HEAD" {
            active_method = "GET".to_string();
          }
          include_body = false;
        } else if matches!(status, StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND)
          && active_method == "POST"
        {
          active_method = "GET".to_string();
          include_body = false;
        }

        current_url = current_url
          .join(location)
          .map_err(|error| ToolError::HttpError {
            message: format!("Invalid redirect Location '{}': {}", location, error),
          })?;
        continue;
      }

      if let (Some(limit), Some(len)) = (max_bytes, response.content_length())
        && len > limit as u64
      {
        return Err(ToolError::HttpError {
          message: format!(
            "Response Content-Length ({len} bytes) exceeds maximum allowed ({limit} bytes)"
          ),
        });
      }

      let status = response.status();
      let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

      let mut body = Vec::new();
      while let Some(chunk) = response.chunk().await.map_err(|e| ToolError::HttpError {
        message: e.to_string(),
      })? {
        if let Some(limit) = max_bytes
          && body.len().saturating_add(chunk.len()) > limit
        {
          return Err(ToolError::HttpError {
            message: format!("Response body exceeded maximum allowed size ({limit} bytes)"),
          });
        }
        body.extend_from_slice(&chunk);
      }

      return Ok(HttpBytesResponse {
        status,
        content_type,
        body,
      });
    }

    Err(ToolError::HttpError {
      message: format!("Too many redirects after {}", MAX_REDIRECTS),
    })
  }

  /// Build the tool (30 s timeout, no auto-redirects, Yanshi
  /// user-agent — the actual `reqwest::Client` used for each request is
  /// built lazily per redirect hop, see `Self::build_pinned_client`).
  /// Q1.2.2: returns the build error instead of panicking — TLS init
  /// failures, OS resource exhaustion, or a fingerprint-cert load
  /// problem should never abort the host process. Validated eagerly
  /// here via a throwaway build so that contract still holds even
  /// though the real per-request clients are built later.
  pub fn new(policy: Arc<SandboxPolicy>) -> Result<Self, ToolError> {
    let tool = Self {
      policy,
      max_response_chars: 8_000,
      max_response_bytes: DEFAULT_MAX_TEXT_RESPONSE_BYTES,
      user_agent: "Yanshi/0.1".to_string(),
      timeout: Duration::from_secs(30),
      no_proxy: false,
    };
    tool
      .base_client_builder()
      .build()
      .map_err(|err| ToolError::ExecutionFailed {
        message: format!("failed to build reqwest client for HttpTool: {err}"),
      })?;
    Ok(tool)
  }

  /// Route every request through no proxy, bypassing any
  /// `HTTP_PROXY`/`HTTPS_PROXY`/system proxy configuration. Used by
  /// tests that need to reach loopback servers directly (Q1.2.3) — a
  /// system HTTP proxy (Clash / V2Ray / corporate proxy) would
  /// otherwise route `127.0.0.1:<port>` through the proxy and turn test
  /// failures into confusing `IncompleteMessage` errors.
  pub fn with_no_proxy(mut self) -> Self {
    self.no_proxy = true;
    self
  }

  /// Override the maximum response size returned in the tool output.
  /// Default is 8 000 characters. Callers like `HttpNode` (Q1.3.2)
  /// disable truncation by setting `usize::MAX`.
  pub fn with_max_response_chars(mut self, max_response_chars: usize) -> Self {
    self.max_response_chars = max_response_chars;
    self
  }

  /// Override the hard byte ceiling buffered by [`Self::send`] (default: 8 MiB).
  pub fn with_max_response_bytes(mut self, max_response_bytes: usize) -> Self {
    self.max_response_bytes = max_response_bytes;
    self
  }

  /// Override the per-request HTTP timeout (default: 30s).
  pub fn with_timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  pub fn default_policy() -> Result<Self, ToolError> {
    Self::new(Arc::new(SandboxPolicy::default()))
  }

  fn base_client_builder(&self) -> reqwest::ClientBuilder {
    let mut builder = Client::builder()
      .timeout(self.timeout)
      .redirect(Policy::none())
      .user_agent(self.user_agent.clone());
    if self.no_proxy {
      builder = builder.no_proxy();
    }
    builder
  }

  /// V3.3: a fresh client, DNS-pinned to exactly the `addrs` that
  /// [`Self::validate_url_allowed`] just validated for `host` — closes
  /// the DNS-rebinding TOCTOU where `reqwest` would otherwise
  /// independently re-resolve `host` at connect time, potentially
  /// landing on a different (attacker-controlled) address than the one
  /// actually checked against the sandbox policy. `reqwest` only
  /// exposes DNS overrides at `ClientBuilder` time
  /// (`resolve`/`resolve_to_addrs`/`dns_resolver`), not per-request, so
  /// this rebuilds a client (and pays a fresh TLS/connection-pool setup
  /// cost) on every redirect hop rather than reusing one pooled client
  /// across calls — acceptable here since `HttpTool` isn't a
  /// high-throughput hot path, and a shared resolver would need
  /// per-request scoping that `reqwest::dns::Resolve` has no hook for.
  fn build_pinned_client(&self, host: &str, addrs: &[SocketAddr]) -> Result<Client, ToolError> {
    self
      .base_client_builder()
      .resolve_to_addrs(host, addrs)
      .build()
      .map_err(|err| ToolError::ExecutionFailed {
        message: format!("failed to build pinned reqwest client for host '{host}': {err}"),
      })
  }

  /// Validate a URL string against the configured [`SandboxPolicy`] (scheme,
  /// cloud-metadata hosts, domain allowlist, and resolved IP address classes)
  /// without sending a request. Used by media downloaders (`video_ingest`)
  /// before invoking an external fetcher.
  pub async fn validate_url_str(&self, raw_url: &str) -> Result<Vec<SocketAddr>, ToolError> {
    let parsed = Url::parse(raw_url).map_err(|error| ToolError::InvalidParams {
      message: format!("Invalid URL '{raw_url}': {error}"),
    })?;
    self.validate_url_allowed(&parsed).await
  }

  async fn validate_url_allowed(&self, url: &Url) -> Result<Vec<SocketAddr>, ToolError> {
    match url.scheme() {
      "http" | "https" => {}
      scheme => {
        return Err(ToolError::SandboxViolation {
          message: format!("HTTP tool does not allow '{}' URLs", scheme),
        });
      }
    }

    let host = url.host_str().ok_or_else(|| ToolError::InvalidParams {
      message: format!("Cannot parse host from URL: {}", url),
    })?;

    if is_cloud_metadata_host(host)
      && !self
        .policy
        .is_network_address_class_allowed(NetworkAddressClass::CloudMetadata)
    {
      return Err(ToolError::SandboxViolation {
        message: format!("Host '{}' is a cloud metadata endpoint", host),
      });
    }

    if !self.policy.is_domain_allowed(host) {
      return Err(ToolError::SandboxViolation {
        message: format!("Domain '{}' is not in the allowed-domains list", host),
      });
    }

    let addresses = resolve_host_ips(url, host).await?;
    for address in &addresses {
      for class in classify_network_address(address.ip()) {
        if !self.policy.is_network_address_class_allowed(class) {
          return Err(ToolError::SandboxViolation {
            message: format!(
              "Address '{}' for host '{}' is denied by sandbox policy ({:?})",
              address, host, class
            ),
          });
        }
      }
    }

    Ok(addresses)
  }
}

#[async_trait]
impl Tool for HttpTool {
  fn name(&self) -> &str {
    "http"
  }

  fn description(&self) -> &str {
    "Make HTTP GET or POST requests to fetch web content or call REST APIs. \
        Returns the response body as text (truncated to 8 000 characters)."
  }

  fn parameters_schema(&self) -> Value {
    json!({
        "type": "object",
        "properties": {
            "url": {
                "type": "string",
                "description": "Full URL to request"
            },
            "method": {
                "type": "string",
                "enum": ["GET", "POST"],
                "description": "HTTP method (default: GET)"
            },
            "body": {
                "type": "string",
                "description": "Request body string (for POST)"
            },
            "headers": {
                "type": "object",
                "description": "Optional key-value map of additional request headers"
            }
        },
        "required": ["url"]
    })
  }

  fn metadata(&self) -> ToolMetadata {
    ToolMetadata::builtin_named(self.name())
  }

  fn idempotency(&self, params: &Value) -> ToolIdempotency {
    match params["method"]
      .as_str()
      .unwrap_or("GET")
      .to_uppercase()
      .as_str()
    {
      // Only the RFC 7231 *safe* methods. PUT/DELETE are idempotent on the
      // wire but mutate remote state, and `Idempotent` here also means
      // "no approval under the production profile, may run in a parallel
      // batch, may be auto-replayed on resume".
      "GET" | "HEAD" => ToolIdempotency::Idempotent,
      "POST" | "PATCH" | "PUT" | "DELETE" => ToolIdempotency::NonIdempotent,
      _ => ToolIdempotency::Unknown,
    }
  }

  async fn execute(&self, params: Value) -> Result<ToolOutput, ToolError> {
    let HttpResponse { status, body } = self.send(&params).await?;

    // Truncate very long responses. Byte length can't stand in for char
    // count on multi-byte UTF-8 bodies, and slicing by byte offset can
    // land mid-character and panic — walk char_indices to find a safe
    // boundary at the Nth char instead.
    let char_count = body.chars().count();
    let content = if char_count > self.max_response_chars {
      let truncate_at = body
        .char_indices()
        .nth(self.max_response_chars)
        .map(|(byte_index, _)| byte_index)
        .unwrap_or(body.len());
      format!(
        "{}... [truncated — total {} chars]",
        &body[..truncate_at],
        char_count
      )
    } else {
      body
    };

    if status.is_success() {
      Ok(ToolOutput::success(content))
    } else {
      Ok(ToolOutput::error(format!("HTTP {}: {}", status, content)))
    }
  }
}

fn is_redirect(status: StatusCode) -> bool {
  matches!(
    status,
    StatusCode::MOVED_PERMANENTLY
      | StatusCode::FOUND
      | StatusCode::SEE_OTHER
      | StatusCode::TEMPORARY_REDIRECT
      | StatusCode::PERMANENT_REDIRECT
  )
}

fn url_origin_key(url: &Url) -> (String, String, Option<u16>) {
  (
    url.scheme().to_ascii_lowercase(),
    url.host_str().unwrap_or_default().to_ascii_lowercase(),
    url.port_or_known_default(),
  )
}

async fn resolve_host_ips(url: &Url, host: &str) -> Result<Vec<SocketAddr>, ToolError> {
  let port = url
    .port_or_known_default()
    .ok_or_else(|| ToolError::InvalidParams {
      message: format!("Cannot infer port for URL: {}", url),
    })?;

  match url.host() {
    Some(url::Host::Ipv4(v4)) => return Ok(vec![SocketAddr::new(IpAddr::V4(v4), port)]),
    Some(url::Host::Ipv6(v6)) => return Ok(vec![SocketAddr::new(IpAddr::V6(v6), port)]),
    _ => {}
  }

  if let Ok(address) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
    return Ok(vec![SocketAddr::new(address, port)]);
  }

  // V3.3: the port is kept on the resolved addresses (previously
  // dropped via `.ip()`) so the caller can pin `reqwest` to exactly
  // these `SocketAddr`s via `ClientBuilder::resolve_to_addrs` — closing
  // the DNS-rebinding TOCTOU between this validation lookup and
  // reqwest's own independent connect-time resolution.
  let resolved = tokio::net::lookup_host((host, port))
    .await
    .map_err(|error| ToolError::HttpError {
      message: format!("Failed to resolve host '{}': {}", host, error),
    })?
    .collect::<Vec<_>>();

  if resolved.is_empty() {
    return Err(ToolError::HttpError {
      message: format!("Host '{}' resolved to no addresses", host),
    });
  }

  Ok(resolved)
}

fn is_cloud_metadata_host(host: &str) -> bool {
  let lower = host.trim_end_matches('.').to_ascii_lowercase();
  CLOUD_METADATA_HOSTS
    .iter()
    .any(|metadata_host| lower == *metadata_host || lower.ends_with(&format!(".{}", metadata_host)))
}

fn is_non_global_private_v4(address: std::net::Ipv4Addr) -> bool {
  let octets = address.octets();
  address.is_private()
    || (octets[0] == 100 && (octets[1] & 0b1100_0000) == 64) // RFC 6598 CGNAT 100.64.0.0/10
    || address.is_broadcast()
    || address.is_multicast()
    || address.is_documentation()
    || (octets[0] == 198 && (octets[1] & 0xfe) == 18) // RFC 2544 benchmarking 198.18.0.0/15
    || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0) // IETF protocol assignments 192.0.0.0/24
    || (octets[0] & 0xf0) == 240 // Reserved Class E 240.0.0.0/4
}

fn classify_network_address(address: IpAddr) -> Vec<NetworkAddressClass> {
  // V3.3: normalize IPv4-mapped IPv6 (`::ffff:a.b.c.d`, RFC 4291
  // §2.5.5.2) to its IPv4 form first.
  let address = match address {
    IpAddr::V6(v6) => v6
      .to_ipv4_mapped()
      .map(IpAddr::V4)
      .unwrap_or(IpAddr::V6(v6)),
    v4 => v4,
  };

  if CLOUD_METADATA_IPS.contains(&address) {
    return vec![NetworkAddressClass::CloudMetadata];
  }

  let mut classes = Vec::new();

  match address {
    IpAddr::V4(address) => {
      // RFC 1122 §3.2.1.3: `0.0.0.0/8` ("this host on this network") connects
      // to local listeners on Linux and macOS, so treat it identically to Loopback.
      if address.is_loopback() || address.octets()[0] == 0 {
        classes.push(NetworkAddressClass::Loopback);
      }
      if address.is_link_local() {
        classes.push(NetworkAddressClass::LinkLocal);
      }
      if is_non_global_private_v4(address) {
        classes.push(NetworkAddressClass::Private);
      }
    }
    IpAddr::V6(address) => {
      if address.is_loopback() || address.is_unspecified() {
        classes.push(NetworkAddressClass::Loopback);
      }
      if (address.segments()[0] & 0xffc0) == 0xfe80 {
        classes.push(NetworkAddressClass::LinkLocal);
      }
      if (address.segments()[0] & 0xfe00) == 0xfc00
        || address.is_multicast()
        || (address.segments()[0] == 0x2001 && address.segments()[1] == 0x0db8)
      {
        classes.push(NetworkAddressClass::Private);
      }
    }
  }

  classes
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
  };

  #[test]
  fn only_safe_methods_are_idempotent() {
    let tool = HttpTool::new(Arc::new(SandboxPolicy::default())).unwrap();
    for (method, expected) in [
      ("GET", ToolIdempotency::Idempotent),
      ("head", ToolIdempotency::Idempotent),
      ("PUT", ToolIdempotency::NonIdempotent),
      ("DELETE", ToolIdempotency::NonIdempotent),
      ("POST", ToolIdempotency::NonIdempotent),
      ("PATCH", ToolIdempotency::NonIdempotent),
      ("OPTIONS", ToolIdempotency::Unknown),
    ] {
      assert_eq!(
        tool.idempotency(&json!({"method": method})),
        expected,
        "{method}"
      );
    }
  }

  /// `.with_no_proxy()` is required because a developer or CI runner
  /// with a system HTTP proxy (Clash / V2Ray / corporate proxy) would
  /// otherwise route `127.0.0.1:<port>` through the proxy and turn the
  /// test failures into confusing `IncompleteMessage` errors. See
  /// CLAUDE.md's "Rust HTTP Testing Guidelines" — Q1.2.3.
  fn test_tool(policy: Arc<SandboxPolicy>) -> HttpTool {
    HttpTool::new(policy)
      .expect("test reqwest client must build")
      .with_no_proxy()
  }

  fn test_tool_default_policy() -> HttpTool {
    test_tool(Arc::new(SandboxPolicy::default()))
  }

  #[tokio::test]
  async fn default_policy_blocks_loopback_ip() {
    let tool = test_tool_default_policy();

    let result = tool
      .execute(json!({
        "url": "http://127.0.0.1:9"
      }))
      .await;

    assert!(matches!(result, Err(ToolError::SandboxViolation { .. })));
    assert!(result.unwrap_err().to_string().contains("Loopback"));
  }

  #[tokio::test]
  async fn default_policy_blocks_localhost_dns_resolution() {
    let tool = test_tool_default_policy();

    let result = tool
      .execute(json!({
        "url": "http://localhost:9"
      }))
      .await;

    assert!(matches!(result, Err(ToolError::SandboxViolation { .. })));
    assert!(result.unwrap_err().to_string().contains("Loopback"));
  }

  #[tokio::test]
  async fn default_policy_blocks_private_ip() {
    let tool = test_tool_default_policy();

    let result = tool
      .execute(json!({
        "url": "http://10.0.0.1"
      }))
      .await;

    assert!(matches!(result, Err(ToolError::SandboxViolation { .. })));
    assert!(result.unwrap_err().to_string().contains("Private"));
  }

  #[tokio::test]
  async fn default_policy_blocks_cloud_metadata_ip() {
    let tool = test_tool_default_policy();

    let result = tool
      .execute(json!({
        "url": "http://169.254.169.254/latest/meta-data/"
      }))
      .await;

    assert!(matches!(result, Err(ToolError::SandboxViolation { .. })));
    let message = result.unwrap_err().to_string();
    assert!(message.contains("CloudMetadata") || message.contains("LinkLocal"));
  }

  // ── V3.3: SSRF hardening ────────────────────────────────────────────

  #[test]
  fn classify_network_address_normalizes_ipv4_mapped_ipv6_cloud_metadata() {
    let address: IpAddr = "::ffff:169.254.169.254".parse().unwrap();
    let classes = classify_network_address(address);
    assert!(
      classes.contains(&NetworkAddressClass::CloudMetadata),
      "expected CloudMetadata, got {classes:?}"
    );
  }

  #[test]
  fn classify_network_address_normalizes_ipv4_mapped_ipv6_private() {
    let address: IpAddr = "::ffff:10.0.0.1".parse().unwrap();
    let classes = classify_network_address(address);
    assert!(
      classes.contains(&NetworkAddressClass::Private),
      "expected Private, got {classes:?}"
    );
  }

  #[test]
  fn classify_network_address_still_handles_native_ipv6_ranges() {
    // Regression guard: normalization must not swallow genuine (non
    // IPv4-mapped) IPv6 addresses — a real IPv6 loopback/link-local/ULA
    // address should classify exactly as it did before this change.
    let loopback: IpAddr = "::1".parse().unwrap();
    assert!(classify_network_address(loopback).contains(&NetworkAddressClass::Loopback));

    let link_local: IpAddr = "fe80::1".parse().unwrap();
    assert!(classify_network_address(link_local).contains(&NetworkAddressClass::LinkLocal));

    let unique_local: IpAddr = "fd00::1".parse().unwrap();
    assert!(classify_network_address(unique_local).contains(&NetworkAddressClass::Private));

    let public: IpAddr = "2001:4860:4860::8888".parse().unwrap();
    assert!(classify_network_address(public).is_empty());
  }

  #[tokio::test]
  async fn pinned_client_ignores_real_dns_and_connects_to_the_pinned_address() {
    // Proves the core rebinding-defeating property directly: a client
    // built via `build_pinned_client` never independently re-resolves
    // the host it was pinned for, no matter what real DNS says about
    // it. `example.com` is a real, unrelated domain deliberately chosen
    // here — if pinning didn't work, this request would either hang or
    // hit the real example.com, not our local listener.
    let (url, server_task) =
      spawn_one_response_server("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
    let server_addr: SocketAddr = url
      .strip_prefix("http://")
      .expect("server url is http")
      .parse()
      .expect("server url is host:port");

    let policy = Arc::new(SandboxPolicy::default());
    let tool = HttpTool::new(policy).unwrap().with_no_proxy();
    let client = tool
      .build_pinned_client("example.com", &[server_addr])
      .expect("pinned client builds");

    let pinned_url = format!("http://example.com:{}/", server_addr.port());
    let response = client
      .get(&pinned_url)
      .send()
      .await
      .expect("request reaches the pinned address, not real DNS for example.com");
    assert!(response.status().is_success());

    server_task.await.unwrap();
  }

  #[tokio::test]
  async fn explicit_policy_allows_loopback() {
    let (url, server_task) =
      spawn_one_response_server("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
    let policy = Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      ..SandboxPolicy::default()
    });
    let tool = test_tool(policy);

    let output = tool.execute(json!({ "url": url })).await.unwrap();

    assert_eq!(output.content, "ok");
    server_task.await.unwrap();
  }

  #[tokio::test]
  async fn send_reports_the_real_status() {
    let policy = Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      ..SandboxPolicy::default()
    });
    let tool = test_tool(policy);
    for (raw, code, is_error) in [
      (
        "HTTP/1.1 201 Created\r\nContent-Length: 2\r\n\r\nok",
        201,
        false,
      ),
      (
        "HTTP/1.1 404 Not Found\r\nContent-Length: 4\r\n\r\ngone",
        404,
        true,
      ),
    ] {
      let (url, server_task) = spawn_one_response_server(raw).await;
      let response = tool.send(&json!({ "url": url })).await.unwrap();
      assert_eq!(response.status.as_u16(), code);
      server_task.await.unwrap();

      let (url, server_task) = spawn_one_response_server(raw).await;
      let output = tool.execute(json!({ "url": url })).await.unwrap();
      assert_eq!(output.is_error, is_error, "{code}: {}", output.content);
      server_task.await.unwrap();
    }
  }

  #[tokio::test]
  async fn redirect_destination_is_checked_before_following() {
    let (url, server_task) = spawn_one_response_server(
      "HTTP/1.1 302 Found\r\nLocation: http://169.254.169.254/latest/meta-data/\r\nContent-Length: 0\r\n\r\n",
    )
    .await;
    let policy = Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      ..SandboxPolicy::default()
    });
    let tool = test_tool(policy);

    let result = tool.execute(json!({ "url": url })).await;

    assert!(matches!(result, Err(ToolError::SandboxViolation { .. })));
    server_task.await.unwrap();
  }

  /// Q1.2.2: HttpTool::new must propagate client-build failures via
  /// `Result` rather than panicking. We cannot easily force a real
  /// `Client::build()` failure in a unit test, so we exercise the
  /// happy path and assert the type signature carries `Result` (the
  /// audit's load-bearing claim was that the panic existed at all).
  #[tokio::test]
  async fn new_returns_result_so_callers_can_handle_build_failures() {
    let policy = Arc::new(SandboxPolicy::default());
    let tool: Result<HttpTool, ToolError> = HttpTool::new(policy);
    assert!(tool.is_ok());
  }

  #[tokio::test]
  async fn truncates_multi_byte_utf8_body_on_a_char_boundary() {
    // 6 four-byte emoji chars — byte length (24) is far from char count
    // (6), and a naive `&body[..max_response_chars]` byte-slice at
    // max=5 would land mid-character (byte offset 5 sits inside the
    // second emoji's 4-byte encoding) and panic.
    let body = "🎉".repeat(6);
    let response = format!(
      "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
      body.len(),
      body
    );
    let (url, server_task) = spawn_one_response_server(Box::leak(response.into_boxed_str())).await;
    let policy = Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      ..SandboxPolicy::default()
    });
    let tool = test_tool(policy).with_max_response_chars(5);

    let output = tool.execute(json!({ "url": url })).await.unwrap();

    assert_eq!(
      output.content,
      format!("{}... [truncated — total 6 chars]", "🎉".repeat(5))
    );
    server_task.await.unwrap();
  }

  #[tokio::test]
  async fn does_not_truncate_body_within_the_char_limit() {
    let (url, server_task) =
      spawn_one_response_server("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok").await;
    let policy = Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      ..SandboxPolicy::default()
    });
    let tool = test_tool(policy);

    let output = tool.execute(json!({ "url": url })).await.unwrap();

    assert_eq!(output.content, "ok");
    server_task.await.unwrap();
  }

  #[tokio::test]
  async fn blocks_unspecified_cgnat_bracketed_ipv6_and_aws_ipv6_imds() {
    let tool = test_tool_default_policy();
    for (url, expected_class) in [
      ("http://0.0.0.0:3000/health", "Loopback"),
      ("http://[::]:3000/health", "Loopback"),
      ("http://[::1]:3000/health", "Loopback"),
      ("http://[::ffff:0.0.0.0]:3000/health", "Loopback"),
      ("http://100.64.0.1:8080/internal", "Private"),
      ("http://[fd00:ec2::254]/latest/meta-data/", "CloudMetadata"),
    ] {
      let err = tool.execute(json!({ "url": url })).await.unwrap_err();
      assert!(
        err.to_string().contains(expected_class),
        "url={url} expected {expected_class}, got {err}"
      );
    }

    // Even when Private network access is allowed, AWS IPv6 IMDS (`fd00:ec2::254`)
    // must still be blocked unless `allow_cloud_metadata_access` is explicitly true.
    let private_allowed = test_tool(Arc::new(SandboxPolicy {
      allow_private_network_access: true,
      allow_cloud_metadata_access: false,
      ..SandboxPolicy::default()
    }));
    let imds_err = private_allowed
      .execute(json!({ "url": "http://[fd00:ec2::254]/latest/meta-data/" }))
      .await
      .unwrap_err();
    assert!(
      imds_err.to_string().contains("CloudMetadata"),
      "got {imds_err}"
    );
  }

  #[tokio::test]
  async fn send_enforces_max_response_bytes_ceiling() {
    let body = "x".repeat(2048);
    let response = format!(
      "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
      body.len(),
      body
    );
    let (url, server_task) = spawn_one_response_server(Box::leak(response.into_boxed_str())).await;
    let policy = Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      ..SandboxPolicy::default()
    });
    let tool = test_tool(policy)
      .with_max_response_chars(usize::MAX)
      .with_max_response_bytes(512);

    let err = tool.send(&json!({ "url": url })).await.unwrap_err();
    assert!(
      err.to_string().contains("exceeds maximum allowed"),
      "got {err}"
    );
    server_task.await.unwrap();
  }

  #[tokio::test]
  async fn cross_origin_redirect_strips_credentials_and_rewrites_post_302_to_get() {
    let target_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_addr = target_listener.local_addr().unwrap();
    let target_task = tokio::spawn(async move {
      let (mut stream, _) = target_listener.accept().await.unwrap();
      let mut buf = [0_u8; 2048];
      let n = stream.read(&mut buf).await.unwrap();
      let raw_req = String::from_utf8_lossy(&buf[..n]).to_string();
      stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\ndone")
        .await
        .unwrap();
      raw_req
    });

    let redirect_resp = format!(
      "HTTP/1.1 302 Found\r\nLocation: http://{target_addr}/landed\r\nContent-Length: 0\r\n\r\n"
    );
    let (origin_url, origin_task) =
      spawn_one_response_server(Box::leak(redirect_resp.into_boxed_str())).await;

    let policy = Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      ..SandboxPolicy::default()
    });
    let tool = test_tool(policy);
    let res = tool
      .send(&json!({
        "url": origin_url,
        "method": "POST",
        "headers": {
          "Authorization": "Bearer top-secret",
          "Cookie": "sid=abc",
          "X-Custom": "safe-header"
        },
        "body": "secret-post-payload"
      }))
      .await
      .unwrap();
    assert_eq!(res.body, "done");

    origin_task.await.unwrap();
    let landed_req = target_task.await.unwrap();
    let lower = landed_req.to_ascii_lowercase();
    assert!(
      landed_req.starts_with("GET /landed HTTP/1.1"),
      "expected POST->GET on 302, got: {landed_req}"
    );
    assert!(
      !lower.contains("authorization:") && !lower.contains("top-secret"),
      "Authorization leaked across origins: {landed_req}"
    );
    assert!(
      !lower.contains("cookie:"),
      "Cookie leaked across origins: {landed_req}"
    );
    assert!(
      lower.contains("x-custom: safe-header"),
      "non-sensitive header should stay: {landed_req}"
    );
    assert!(
      !landed_req.contains("secret-post-payload"),
      "POST body leaked after 302->GET: {landed_req}"
    );
  }

  async fn spawn_one_response_server(
    response: &'static str,
  ) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
      let (mut stream, _) = listener.accept().await.unwrap();
      let mut buffer = [0_u8; 1024];
      let _ = stream.read(&mut buffer).await.unwrap();
      stream.write_all(response.as_bytes()).await.unwrap();
    });

    (format!("http://{}", address), task)
  }
}
