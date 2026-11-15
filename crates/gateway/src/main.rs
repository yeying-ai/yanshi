//! Pingora-based TLS terminator and reverse proxy for Yanshi (`yanshi.yeying.dev`).
//!
//! Modeled after `rustineverything.app/crates/gateway` (`rie-gateway`):
//! - Listens on `:443` (TLS 1.2/1.3 + HTTP/2 + HTTP/1.1 ALPN) and `:80` (HTTP -> HTTPS 301).
//! - Supports multi-domain SNI certificate routing (`SNI_CERTS`), subdomain upstream routing
//!   (`HOST_UPSTREAMS`), and canonical domain 301 redirects (`HOST_REDIRECTS`).
//! - Tuned for Yanshi's long-lived SSE streams (`/v1/runs/:id/events`,
//!   `/v1/harness/sessions/:id/events`) and multimodal artifact transfers: zero proxy response
//!   buffering with explicit 1-hour read timeout and 60s idle pool reuse.
//! - Enforces XFF hygiene (`insert` overwrite, strip RFC 7239 `Forwarded`), coalesces split
//!   HTTP/2 `cookie` headers (RFC 9113 §8.2.3), and injects HSTS + baseline security headers
//!   (`X-Frame-Options: SAMEORIGIN` so Studio same-origin HTML slide `<iframe>` previews work).

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use pingora_core::Result;
use pingora_core::apps::HttpServerOptions;
use pingora_core::listeners::TlsAccept;
use pingora_core::protocols::tls::TlsRef;
use pingora_core::server::Server;
use pingora_core::server::configuration::{Opt, ServerConf};
use pingora_core::tls::ext::{ssl_add_chain_cert, ssl_use_certificate, ssl_use_private_key};
use pingora_core::tls::pkey::{PKey, Private};
use pingora_core::tls::ssl::NameType;
use pingora_core::tls::x509::X509;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_proxy::{ProxyHttp, Session, http_proxy_service};

/// Default loopback upstream (`yanshi-server` container port-forwarded on host loopback).
const DEFAULT_UPSTREAM_ADDR: &str = "127.0.0.1:18080";
const UPSTREAM_SNI: &str = "";

const DEFAULT_HTTPS_BIND: &str = "0.0.0.0:443";
const DEFAULT_HTTP_BIND: &str = "0.0.0.0:80";

const HSTS_VALUE: &str = "max-age=31536000; includeSubDomains";

/// Maximum idle seconds between requests on downstream HTTP/1.1 keep-alive and h2 connections.
const DOWNSTREAM_IDLE_SECS: u64 = 60;

/// Upstream socket timeouts: 3600s read timeout keeps SSE streams (`/v1/runs/:id/events`
/// with 15s server heartbeats) and long multimodal workflow calls open without premature resets.
const UPSTREAM_CONNECT_TIMEOUT_SECS: u64 = 10;
const UPSTREAM_READ_TIMEOUT_SECS: u64 = 3600;
const UPSTREAM_WRITE_TIMEOUT_SECS: u64 = 300;
const UPSTREAM_IDLE_TIMEOUT_SECS: u64 = 60;

/// Pingora resets HTTP/1.1 keep-alive to infinite (`Some(0)`) after reading headers;
/// cap it back to a finite idle timeout while leaving `None` (Connection: close) untouched.
fn cap_keepalive(session: &mut Session) {
  if session.get_keepalive() == Some(0) {
    session.set_keepalive(Some(DOWNSTREAM_IDLE_SECS));
  }
}

struct AppGateway {
  default_upstream: String,
  host_upstreams: HashMap<String, String>,
  host_redirects: HashMap<String, String>,
}

impl AppGateway {
  fn from_env() -> Self {
    let default_upstream = std::env::var("UPSTREAM_ADDR")
      .ok()
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
      .unwrap_or_else(|| DEFAULT_UPSTREAM_ADDR.to_string());
    let host_upstreams = std::env::var("HOST_UPSTREAMS")
      .ok()
      .map(|s| parse_host_upstreams(&s))
      .unwrap_or_default();
    let host_redirects = std::env::var("HOST_REDIRECTS")
      .ok()
      .map(|s| parse_host_redirects(&s))
      .unwrap_or_default();
    Self {
      default_upstream,
      host_upstreams,
      host_redirects,
    }
  }

  fn select_upstream(&self, req: &RequestHeader) -> &str {
    if !self.host_upstreams.is_empty()
      && let Some(host) = extract_request_host(req)
      && let Some(addr) = self.host_upstreams.get(&host)
    {
      return addr.as_str();
    }
    &self.default_upstream
  }

  fn select_redirect_location(&self, req: &RequestHeader) -> Option<String> {
    if self.host_redirects.is_empty() {
      return None;
    }
    let host = extract_request_host(req)?;
    let target_base = self.host_redirects.get(&host)?;
    let path = req.uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    if path.starts_with('/') {
      Some(format!("{target_base}{path}"))
    } else {
      Some(format!("{target_base}/{path}"))
    }
  }
}

/// Parse `HOST_REDIRECTS` (`old.example.com=https://new.dev,www.new.dev=https://new.dev`).
fn parse_host_redirects(raw: &str) -> HashMap<String, String> {
  let mut map = HashMap::new();
  for pair in raw.split(',') {
    let Some((host, target)) = pair.trim().split_once('=') else {
      continue;
    };
    let Some(host) = normalize_host(host) else {
      continue;
    };
    let target = target.trim().trim_end_matches('/');
    if target.starts_with("https://") || target.starts_with("http://") {
      map.insert(host, target.to_string());
    }
  }
  map
}

struct CertBundle {
  leaf: X509,
  intermediates: Vec<X509>,
  key: PKey<Private>,
}

impl CertBundle {
  fn from_pem(cert_pem: &[u8], key_pem: &[u8]) -> std::result::Result<Self, String> {
    let mut chain =
      X509::stack_from_pem(cert_pem).map_err(|e| format!("parse cert chain PEM failed: {e}"))?;
    if chain.is_empty() {
      return Err("certificate PEM contains no X509 certificates".to_string());
    }
    let leaf = chain.remove(0);
    let key = PKey::private_key_from_pem(key_pem)
      .map_err(|e| format!("parse private key PEM failed: {e}"))?;
    Ok(Self {
      leaf,
      intermediates: chain,
      key,
    })
  }

  fn load(cert_path: &str, key_path: &str) -> std::result::Result<Self, String> {
    let cert_pem =
      std::fs::read(cert_path).map_err(|e| format!("read cert {cert_path} failed: {e}"))?;
    let key_pem =
      std::fs::read(key_path).map_err(|e| format!("read key {key_path} failed: {e}"))?;
    Self::from_pem(&cert_pem, &key_pem)
  }
}

/// Parse `SNI_CERTS` (`domain1=/certs/d1/fullchain.pem:/certs/d1/privkey.pem,...`).
fn parse_sni_cert_specs(raw: &str) -> Vec<(String, String, String)> {
  let mut specs = Vec::new();
  for entry in raw.split(',') {
    let Some((domain, paths)) = entry.trim().split_once('=') else {
      continue;
    };
    let Some(domain) = normalize_host(domain.trim_start_matches("*.")) else {
      continue;
    };
    let Some((cert_path, key_path)) = paths.trim().split_once(':') else {
      continue;
    };
    let cert_path = cert_path.trim();
    let key_path = key_path.trim();
    if !cert_path.is_empty() && !key_path.is_empty() {
      specs.push((domain, cert_path.to_string(), key_path.to_string()));
    }
  }
  // Longer, more specific subdomains match before shorter parent domains.
  specs.sort_by_key(|a| std::cmp::Reverse(a.0.len()));
  specs
}

fn host_matches_domain(sni_host: &str, domain: &str) -> bool {
  sni_host == domain
    || sni_host
      .strip_suffix(domain)
      .is_some_and(|prefix| prefix.ends_with('.'))
}

struct SniCertResolver {
  default_bundle: CertBundle,
  routes: Vec<(String, CertBundle)>,
}

impl SniCertResolver {
  fn load(
    default_cert: &str,
    default_key: &str,
    sni_raw: &str,
  ) -> std::result::Result<Option<Self>, String> {
    let specs = parse_sni_cert_specs(sni_raw);
    if specs.is_empty() {
      return Ok(None);
    }
    let default_bundle = CertBundle::load(default_cert, default_key)?;
    let mut routes = Vec::with_capacity(specs.len());
    for (domain, cert_path, key_path) in specs {
      let bundle = CertBundle::load(&cert_path, &key_path)?;
      routes.push((domain, bundle));
    }
    Ok(Some(Self {
      default_bundle,
      routes,
    }))
  }

  fn select_bundle(&self, sni_raw: Option<&str>) -> &CertBundle {
    if let Some(host) = sni_raw.and_then(normalize_host) {
      for (domain, bundle) in &self.routes {
        if host_matches_domain(&host, domain) {
          return bundle;
        }
      }
    }
    &self.default_bundle
  }
}

#[async_trait]
impl TlsAccept for SniCertResolver {
  async fn certificate_callback(&self, ssl: &mut TlsRef) {
    let bundle = self.select_bundle(ssl.servername(NameType::HOST_NAME));
    if let Err(e) = ssl_use_certificate(ssl, &bundle.leaf) {
      log::error!("SNI ssl_use_certificate failed: {e}");
      return;
    }
    for inter in &bundle.intermediates {
      if let Err(e) = ssl_add_chain_cert(ssl, inter) {
        log::error!("SNI ssl_add_chain_cert failed: {e}");
        return;
      }
    }
    if let Err(e) = ssl_use_private_key(ssl, &bundle.key) {
      log::error!("SNI ssl_use_private_key failed: {e}");
    }
  }
}

/// Parse `HOST_UPSTREAMS` (`yanshi.yeying.dev=127.0.0.1:18080,other.yeying.dev=127.0.0.1:18081`).
fn parse_host_upstreams(raw: &str) -> HashMap<String, String> {
  let mut map = HashMap::new();
  for pair in raw.split(',') {
    let Some((host, addr)) = pair.trim().split_once('=') else {
      continue;
    };
    let Some(host) = normalize_host(host) else {
      continue;
    };
    let addr = addr.trim();
    if !addr.is_empty() {
      map.insert(host, addr.to_string());
    }
  }
  map
}

/// Normalize a host string: strip port, strip trailing `.`, lowercase ASCII.
fn normalize_host(raw: &str) -> Option<String> {
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return None;
  }
  let host_only = if trimmed.starts_with('[') {
    let end = trimmed.find(']')?;
    &trimmed[..=end]
  } else {
    trimmed.split(':').next().unwrap_or("")
  };
  let normalized = host_only.trim().trim_end_matches('.').to_ascii_lowercase();
  if normalized.is_empty() {
    None
  } else {
    Some(normalized)
  }
}

/// Extract normalized request host: prefer HTTP/2 `:authority` (`req.uri.host()`),
/// falling back to HTTP/1.1 `Host` header.
fn extract_request_host(req: &RequestHeader) -> Option<String> {
  if let Some(uri_host) = req.uri.host()
    && let Some(h) = normalize_host(uri_host)
  {
    return Some(h);
  }
  let header_host = req.headers.get("host").and_then(|v| v.to_str().ok())?;
  normalize_host(header_host)
}

fn build_upstream_peer(addr: &str) -> Box<HttpPeer> {
  let mut peer = HttpPeer::new(addr, false, UPSTREAM_SNI.into());
  peer.options.connection_timeout = Some(Duration::from_secs(UPSTREAM_CONNECT_TIMEOUT_SECS));
  peer.options.read_timeout = Some(Duration::from_secs(UPSTREAM_READ_TIMEOUT_SECS));
  peer.options.write_timeout = Some(Duration::from_secs(UPSTREAM_WRITE_TIMEOUT_SECS));
  peer.options.idle_timeout = Some(Duration::from_secs(UPSTREAM_IDLE_TIMEOUT_SECS));
  Box::new(peer)
}

#[async_trait]
impl ProxyHttp for AppGateway {
  type CTX = ();

  fn new_ctx(&self) -> Self::CTX {}

  async fn early_request_filter(&self, session: &mut Session, _ctx: &mut ()) -> Result<()> {
    cap_keepalive(session);
    Ok(())
  }

  async fn request_filter(&self, session: &mut Session, _ctx: &mut ()) -> Result<bool> {
    if let Some(location) = self.select_redirect_location(session.req_header()) {
      let mut resp = ResponseHeader::build(301, None)
        .map_err(|_| pingora_core::Error::new_str("failed to build 301 response header"))?;
      resp.insert_header("Location", location).ok();
      resp
        .insert_header("Strict-Transport-Security", HSTS_VALUE)
        .ok();
      resp.insert_header("Server", "yanshi-gateway").ok();
      resp.insert_header("Content-Length", "0").ok();
      session.write_response_header_ref(&resp, true).await.ok();
      return Ok(true);
    }
    Ok(false)
  }

  async fn upstream_peer(&self, session: &mut Session, _ctx: &mut ()) -> Result<Box<HttpPeer>> {
    let addr = self.select_upstream(session.req_header());
    Ok(build_upstream_peer(addr))
  }

  async fn upstream_request_filter(
    &self,
    session: &mut Session,
    req: &mut RequestHeader,
    _ctx: &mut (),
  ) -> Result<()> {
    req.insert_header("X-Forwarded-Proto", "https").ok();
    if !req.headers.contains_key("host")
      && let Some(authority) = req.uri.authority().map(|a| a.as_str().to_string())
    {
      req.insert_header("Host", authority).ok();
    }
    req.remove_header("Forwarded");

    if let Some(ip) = session.client_addr().and_then(forwarded_ip) {
      req.insert_header("X-Real-IP", ip.clone()).ok();
      req.insert_header("X-Forwarded-For", ip).ok();
    } else {
      req.remove_header("X-Forwarded-For");
    }
    coalesce_cookie_headers(req);
    Ok(())
  }

  async fn response_filter(
    &self,
    _session: &mut Session,
    upstream_response: &mut ResponseHeader,
    _ctx: &mut (),
  ) -> Result<()> {
    apply_security_headers(upstream_response);
    Ok(())
  }
}

fn forwarded_ip(addr: &pingora_core::protocols::l4::socket::SocketAddr) -> Option<String> {
  Some(addr.as_inet()?.ip().to_string())
}

/// RFC 9113 §8.2.3: coalesce split HTTP/2 `cookie` headers into a single `; `-joined `Cookie`
/// header before forwarding over HTTP/1.1 to upstream.
fn coalesce_cookie_headers(req: &mut RequestHeader) {
  let mut iter = req.headers.get_all("cookie").into_iter();
  let Some(first) = iter.next() else { return };
  let Some(second) = iter.next() else { return };
  let mut joined = String::new();
  for v in [first, second].into_iter().chain(iter) {
    if let Ok(s) = v.to_str() {
      let s = s.trim();
      if !s.is_empty() {
        if !joined.is_empty() {
          joined.push_str("; ");
        }
        joined.push_str(s);
      }
    }
  }
  if !joined.is_empty() {
    req.insert_header("Cookie", joined).ok();
  }
}

/// Inject HSTS and fill missing baseline security headers.
/// Uses `X-Frame-Options: SAMEORIGIN` by default so Yanshi Studio's same-origin HTML slide /
/// deliverable `<iframe>` previews work while still blocking cross-origin framing.
fn apply_security_headers(resp: &mut ResponseHeader) {
  resp
    .insert_header("Strict-Transport-Security", HSTS_VALUE)
    .ok();
  resp.insert_header("Server", "yanshi-gateway").ok();
  for (name, value) in [
    ("X-Content-Type-Options", "nosniff"),
    ("X-Frame-Options", "SAMEORIGIN"),
    ("Referrer-Policy", "strict-origin-when-cross-origin"),
  ] {
    if !resp.headers.contains_key(name) {
      resp.insert_header(name, value).ok();
    }
  }
}

/// Port 80 service: redirects all plain HTTP traffic to `https://<host><path>`.
struct HttpToHttps;

#[async_trait]
impl ProxyHttp for HttpToHttps {
  type CTX = ();

  fn new_ctx(&self) -> Self::CTX {}

  async fn request_filter(&self, session: &mut Session, _ctx: &mut ()) -> Result<bool> {
    cap_keepalive(session);
    let req = session.req_header();
    let host = req
      .headers
      .get("host")
      .and_then(|v| v.to_str().ok())
      .unwrap_or("");
    let path = req.uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    let location = format!("https://{host}{path}");

    let mut resp = ResponseHeader::build(301, None)
      .map_err(|_| pingora_core::Error::new_str("failed to build HTTP->HTTPS 301 header"))?;
    resp.append_header("Location", location).ok();
    resp.append_header("Server", "yanshi-gateway").ok();
    resp.append_header("Content-Length", "0").ok();
    session.write_response_header_ref(&resp, true).await.ok();
    Ok(true)
  }

  async fn upstream_peer(&self, _session: &mut Session, _ctx: &mut ()) -> Result<Box<HttpPeer>> {
    Err(pingora_core::Error::new_str(
      "unreachable: HttpToHttps short-circuits in request_filter",
    ))
  }
}

fn env_or_default(key: &str, default: &str) -> String {
  std::env::var(key)
    .ok()
    .map(|v| v.trim().to_string())
    .filter(|v| !v.is_empty())
    .unwrap_or_else(|| default.to_string())
}

fn main() {
  env_logger::init();

  let https_bind = env_or_default("HTTPS_BIND", DEFAULT_HTTPS_BIND);
  let http_bind = env_or_default("HTTP_BIND", DEFAULT_HTTP_BIND);

  let conf = ServerConf {
    grace_period_seconds: Some(5),
    graceful_shutdown_timeout_seconds: Some(1),
    ..Default::default()
  };
  let mut server = Server::new_with_opt_and_conf(None::<Opt>, conf);
  server.bootstrap();

  let gateway = AppGateway::from_env();
  let default_upstream_log = gateway.default_upstream.clone();
  let host_routes_log = gateway.host_upstreams.len();
  let host_redirects_log = gateway.host_redirects.len();
  let mut proxy = http_proxy_service(&server.configuration, gateway);

  let Ok(cert) = std::env::var("TLS_CERT_PATH") else {
    eprintln!("yanshi-gateway: TLS_CERT_PATH is required");
    std::process::exit(1);
  };
  let Ok(key) = std::env::var("TLS_KEY_PATH") else {
    eprintln!("yanshi-gateway: TLS_KEY_PATH is required");
    std::process::exit(1);
  };
  let sni_raw = std::env::var("SNI_CERTS").unwrap_or_default();
  let sni_resolver = match SniCertResolver::load(&cert, &key, &sni_raw) {
    Ok(r) => r,
    Err(e) => {
      eprintln!("yanshi-gateway: failed to load SNI_CERTS: {e}");
      std::process::exit(1);
    }
  };
  let sni_certs_log = sni_resolver.as_ref().map_or(0, |r| r.routes.len());
  let tls_res = match sni_resolver {
    Some(resolver) => pingora_core::listeners::tls::TlsSettings::with_callbacks(Box::new(resolver)),
    None => pingora_core::listeners::tls::TlsSettings::intermediate(&cert, &key),
  };
  let mut tls = match tls_res {
    Ok(t) => t,
    Err(e) => {
      eprintln!("yanshi-gateway: failed to initialize TLS settings: {e}");
      std::process::exit(1);
    }
  };
  tls.enable_h2();
  proxy.add_tls_with_settings(&https_bind, None, tls);

  let mut opts = HttpServerOptions::default();
  opts.h2_idle_timeout = Some(Duration::from_secs(DOWNSTREAM_IDLE_SECS));
  if let Some(app) = proxy.app_logic_mut() {
    app.server_options = Some(opts);
  }
  server.add_service(proxy);

  let mut redirect = http_proxy_service(&server.configuration, HttpToHttps);
  redirect.add_tcp(&http_bind);
  server.add_service(redirect);

  log::info!(
    "yanshi-gateway: HTTPS={} (default_tls={}, sni_certs={}), HTTP={} -> 301 https; default_upstream={}, host_routes={}, host_redirects={}",
    https_bind,
    cert,
    sni_certs_log,
    http_bind,
    default_upstream_log,
    host_routes_log,
    host_redirects_log
  );
  server.run_forever();
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn forwarded_ip_strips_client_port() {
    let v4 =
      pingora_core::protocols::l4::socket::SocketAddr::Inet("203.0.113.7:51234".parse().unwrap());
    assert_eq!(forwarded_ip(&v4).as_deref(), Some("203.0.113.7"));
    let v6 =
      pingora_core::protocols::l4::socket::SocketAddr::Inet("[2001:db8::1]:443".parse().unwrap());
    assert_eq!(forwarded_ip(&v6).as_deref(), Some("2001:db8::1"));
  }

  fn header<'a>(resp: &'a ResponseHeader, name: &str) -> Option<&'a str> {
    resp.headers.get(name).and_then(|v| v.to_str().ok())
  }

  #[test]
  fn security_headers_preserve_csp_and_default_sameorigin() {
    let app_csp = "default-src 'self'";
    let mut resp = ResponseHeader::build(200, None).unwrap();
    resp
      .insert_header("Content-Security-Policy", app_csp)
      .unwrap();
    apply_security_headers(&mut resp);
    assert_eq!(header(&resp, "content-security-policy"), Some(app_csp));
    assert_eq!(header(&resp, "x-frame-options"), Some("SAMEORIGIN"));
    assert_eq!(header(&resp, "x-content-type-options"), Some("nosniff"));
    assert_eq!(
      header(&resp, "referrer-policy"),
      Some("strict-origin-when-cross-origin")
    );
    assert_eq!(header(&resp, "strict-transport-security"), Some(HSTS_VALUE));
  }

  #[test]
  fn upstream_peer_sets_sse_friendly_timeouts() {
    let peer = build_upstream_peer("127.0.0.1:18080");
    assert_eq!(
      peer.options.read_timeout,
      Some(Duration::from_secs(UPSTREAM_READ_TIMEOUT_SECS))
    );
    assert_eq!(
      peer.options.write_timeout,
      Some(Duration::from_secs(UPSTREAM_WRITE_TIMEOUT_SECS))
    );
    assert_eq!(
      peer.options.connection_timeout,
      Some(Duration::from_secs(UPSTREAM_CONNECT_TIMEOUT_SECS))
    );
  }

  #[test]
  fn coalesces_multiple_h2_cookie_headers() {
    let mut req = RequestHeader::build("GET", b"/v1/whoami", None).unwrap();
    req.append_header("cookie", "lang=zh").unwrap();
    req.append_header("cookie", "theme=dark").unwrap();
    coalesce_cookie_headers(&mut req);
    assert_eq!(req.headers.get_all("cookie").iter().count(), 1);
    assert_eq!(
      req.headers.get("cookie").and_then(|v| v.to_str().ok()),
      Some("lang=zh; theme=dark")
    );
  }

  #[test]
  fn host_upstreams_routes_yanshi_subdomain_and_falls_back() {
    let gw = AppGateway {
      default_upstream: "127.0.0.1:18080".to_string(),
      host_upstreams: parse_host_upstreams(
        " yanshi.yeying.dev = 127.0.0.1:18080 , api.yeying.dev=127.0.0.1:18081 ",
      ),
      host_redirects: parse_host_redirects(" www.yanshi.yeying.dev = https://yanshi.yeying.dev/ "),
    };

    let mut h1 = RequestHeader::build("GET", b"/ui/studio", None).unwrap();
    h1.insert_header("Host", "Yanshi.Yeying.DEV:443").unwrap();
    assert_eq!(gw.select_upstream(&h1), "127.0.0.1:18080");

    let mut h2 = RequestHeader::build("GET", b"/v1/runs", None).unwrap();
    h2.set_uri("https://api.yeying.dev:443/v1/runs".parse().unwrap());
    assert_eq!(gw.select_upstream(&h2), "127.0.0.1:18081");

    let mut redir = RequestHeader::build("GET", b"/ui/notebooks?tab=1", None).unwrap();
    redir
      .insert_header("Host", "www.yanshi.yeying.dev")
      .unwrap();
    assert_eq!(
      gw.select_redirect_location(&redir).as_deref(),
      Some("https://yanshi.yeying.dev/ui/notebooks?tab=1")
    );
  }

  #[test]
  fn sni_domain_matching_handles_wildcard_and_dot_boundaries() {
    let specs = parse_sni_cert_specs(
      " *.yeying.dev = /certs/yeying.dev.cert:/certs/yeying.dev.key , sub.yeying.dev=/certs/sub.cert:/certs/sub.key ",
    );
    assert_eq!(specs.len(), 2);
    assert_eq!(specs[0].0, "sub.yeying.dev");
    assert_eq!(specs[1].0, "yeying.dev");

    assert!(host_matches_domain("yanshi.yeying.dev", "yeying.dev"));
    assert!(host_matches_domain("yeying.dev", "yeying.dev"));
    assert!(!host_matches_domain("fakeyeying.dev", "yeying.dev"));
  }
}
