//! Per-modality provider traits for non-chat APIs.
//!
//! Chat-shaped models route through `LLMProvider` (in
//! [`crate::providers`]). The five trait surfaces here cover the
//! non-chat modalities Yanshi ships nodes for:
//!
//! - [`asr::AsrProvider`] — audio → text (Whisper, StepFun ASR)
//! - [`tts::TtsProvider`] — text → audio (StepFun TTS, ElevenLabs)
//! - [`text_to_image::Text2ImageProvider`] — text → image
//!   (DALL-E, Imagen, StepFun T2I)
//! - [`image_to_image::Image2ImageProvider`] — image+text → image
//! - [`image_edit::ImageEditProvider`] — image+text → image
//!   (DALL-E edit, StepFun image edit)
//! - [`text_to_video::Text2VideoProvider`] — text → video (Google Veo).
//!   Unlike the other five, this is an async job API (submit + poll),
//!   not a synchronous request/response.
//!
//! Each trait is intentionally narrow — modality shapes diverge enough
//! that a single "MultimodalProvider" abstraction would leak. Common
//! response shape (URL or base64 image bytes) is shared via
//! [`ImageGenerationResponse`] / [`GeneratedImage`].
//!
//! ## P-LLM.1 contract
//!
//! These traits are the seam that let `yanshi-nodes-ai::{asr, tts,
//! text_to_image, image_to_image, image_edit}` drop their direct
//! StepFun coupling (the pre-P-LLM.3 code called
//! `Yanshi::stepfun_client(...)` and `providers::stepfun::*Request`
//! types directly). P-LLM.3 landed this: every one of those nodes now
//! calls the vendor-agnostic `Yanshi::{asr,tts,text2image_for,
//! image2image,image_edit}` entry points (`crate::modality_dispatch`),
//! which route through these traits to resolve the right vendor by
//! model name — no node references StepFun (or any other vendor) by
//! name.

pub mod asr;
pub mod image_edit;
pub mod image_to_image;
pub mod text_to_image;
pub mod text_to_video;
pub mod tts;

pub use asr::{AsrProvider, AsrRequest, AsrResponse};
pub use image_edit::{ImageEditProvider, ImageEditRequest};
pub use image_to_image::{Image2ImageProvider, Image2ImageRequest};
pub use text_to_image::{Text2ImageProvider, Text2ImageRequest};
pub use text_to_video::{
  GeneratedVideo, Text2VideoProvider, Text2VideoRequest, VideoGenerationResponse,
  VideoGenerationStatus, VideoGenerationTask, VideoInputImage,
};
pub use tts::{TtsProvider, TtsRequest, TtsResponse};

use serde::{Deserialize, Serialize};

/// Download `url` into memory, refusing to buffer more than `max_bytes`
/// (Y1.1). Vendors hand back time-limited URLs for images and videos;
/// turning those into on-disk artifacts means fetching them inside the
/// node, and an unbounded fetch of an attacker-chosen or mis-sized body
/// is the one thing that must not happen there. The cap is checked
/// against `Content-Length` up front and again while streaming, since
/// the header is optional and untrusted.
pub async fn download_with_cap(
  client: &reqwest::Client,
  url: &str,
  headers: reqwest::header::HeaderMap,
  max_bytes: u64,
) -> crate::Result<Vec<u8>> {
  let mut response = client.get(url).headers(headers).send().await?;
  if !response.status().is_success() {
    let status_code = response.status().as_u16();
    let message = response.text().await.unwrap_or_default();
    return Err(crate::LLMError::HttpError {
      status_code,
      message,
    });
  }
  if let Some(declared) = response.content_length()
    && declared > max_bytes
  {
    return Err(crate::LLMError::ResponseParsingError {
      message: format!(
        "download of {url} declares {declared} bytes, over the {max_bytes}-byte cap"
      ),
    });
  }
  let mut body = Vec::new();
  while let Some(chunk) = response.chunk().await? {
    if body.len() as u64 + chunk.len() as u64 > max_bytes {
      return Err(crate::LLMError::ResponseParsingError {
        message: format!("download of {url} exceeded the {max_bytes}-byte cap"),
      });
    }
    body.extend_from_slice(&chunk);
  }
  Ok(body)
}

/// [`download_with_cap`] with the crate's default HTTP client and no
/// extra headers, returning the response's `Content-Type` alongside the
/// bytes so a caller can label what it stores. For callers (nodes) that
/// do not otherwise depend on `reqwest`.
pub async fn download_with_cap_default(
  url: &str,
  max_bytes: u64,
) -> crate::Result<(Vec<u8>, Option<String>)> {
  let client = crate::providers::default_http_client()?;
  let mut response = client.get(url).send().await?;
  if !response.status().is_success() {
    let status_code = response.status().as_u16();
    let message = response.text().await.unwrap_or_default();
    return Err(crate::LLMError::HttpError {
      status_code,
      message,
    });
  }
  let mime = response
    .headers()
    .get(reqwest::header::CONTENT_TYPE)
    .and_then(|v| v.to_str().ok())
    .map(|v| v.split(';').next().unwrap_or(v).trim().to_string())
    .filter(|v| !v.is_empty());
  if let Some(declared) = response.content_length()
    && declared > max_bytes
  {
    return Err(crate::LLMError::ResponseParsingError {
      message: format!(
        "download of {url} declares {declared} bytes, over the {max_bytes}-byte cap"
      ),
    });
  }
  let mut body = Vec::new();
  while let Some(chunk) = response.chunk().await? {
    if body.len() as u64 + chunk.len() as u64 > max_bytes {
      return Err(crate::LLMError::ResponseParsingError {
        message: format!("download of {url} exceeded the {max_bytes}-byte cap"),
      });
    }
    body.extend_from_slice(&chunk);
  }
  Ok((body, mime))
}

/// Common response envelope for image-generating endpoints.
///
/// Used by `Text2ImageProvider`, `Image2ImageProvider`, and
/// `ImageEditProvider` since vendor responses for these three share a
/// shape (a list of images, each either a URL or a base64 blob, plus
/// a creation timestamp). Vendor-specific bookkeeping that doesn't fit
/// here belongs in `metadata`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationResponse {
  /// Unix timestamp the provider says the image was created.
  pub created: u64,
  /// Generated images. Length depends on the request's `n` field.
  pub images: Vec<GeneratedImage>,
  /// Vendor-specific extras (request id, billing info, etc.) preserved
  /// for caller inspection without forcing every consumer to know
  /// about them.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub metadata: Option<serde_json::Value>,
}

/// A single image entry in an [`ImageGenerationResponse`].
///
/// Exactly one of `url` / `b64_json` is populated, mirroring the
/// `response_format: "url" | "b64_json"` choice in the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
  /// HTTP URL the vendor will serve the image from (time-limited on
  /// most providers).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub url: Option<String>,
  /// Base64-encoded image bytes (no `data:` prefix).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub b64_json: Option<String>,
  /// Seed used to generate this image, when known.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub seed: Option<i32>,
}

/// SSRF-guarded variant of [`download_with_cap_default`] for caller-supplied
/// `source_url` / `reference_urls` inputs (`source_image_bytes`,
/// `GoogleMediaProvider::resolve_source_image`). Rejects non-HTTP(S) schemes,
/// cloud metadata hostnames/IPs, loopback/`0.0.0.0/8`/`[::]`, link-local, and
/// RFC 1918 / RFC 6598 private ranges across up to 5 redirect hops, pinning
/// DNS resolution per hop.
pub async fn download_public_url_with_cap(
  raw_url: &str,
  max_bytes: u64,
) -> crate::Result<(Vec<u8>, Option<String>)> {
  use std::net::{IpAddr, SocketAddr};

  const MAX_REDIRECTS: usize = 5;
  let mut current =
    reqwest::Url::parse(raw_url).map_err(|e| crate::LLMError::ConfigurationError {
      message: format!("invalid source media URL '{raw_url}': {e}"),
    })?;

  for hop in 0..=MAX_REDIRECTS {
    match current.scheme() {
      "http" | "https" => {}
      other => {
        return Err(crate::LLMError::ConfigurationError {
          message: format!("unsupported source media URL scheme '{other}'"),
        });
      }
    }
    let host = current
      .host_str()
      .ok_or_else(|| crate::LLMError::ConfigurationError {
        message: format!("source media URL '{current}' has no host"),
      })?
      .to_string();
    let lower_host = host.trim_end_matches('.').to_ascii_lowercase();
    if matches!(
      lower_host.as_str(),
      "localhost"
        | "metadata"
        | "metadata.google.internal"
        | "instance-data"
        | "instance-data.ec2.internal"
    ) || lower_host.ends_with(".localhost")
      || lower_host.ends_with(".internal")
    {
      return Err(crate::LLMError::ConfigurationError {
        message: format!("source media URL host '{host}' is disallowed (internal/metadata)"),
      });
    }

    let port =
      current
        .port_or_known_default()
        .ok_or_else(|| crate::LLMError::ConfigurationError {
          message: format!("cannot infer port for source media URL '{current}'"),
        })?;

    let (addrs, is_ip_literal): (Vec<SocketAddr>, bool) =
      if let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
        (vec![SocketAddr::new(ip, port)], true)
      } else {
        let resolved: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
          .await
          .map_err(|e| crate::LLMError::NetworkError {
            message: format!("failed to resolve source media host '{host}': {e}"),
          })?
          .collect();
        if resolved.is_empty() {
          return Err(crate::LLMError::NetworkError {
            message: format!("source media host '{host}' resolved to no addresses"),
          });
        }
        (resolved, false)
      };

    for addr in &addrs {
      if !is_public_media_ip(addr.ip()) {
        return Err(crate::LLMError::ConfigurationError {
          message: format!(
            "source media URL '{current}' resolves to non-public IP {} (blocked)",
            addr.ip()
          ),
        });
      }
    }

    let mut builder = reqwest::Client::builder()
      .timeout(std::time::Duration::from_secs(30))
      .redirect(reqwest::redirect::Policy::none());
    if !is_ip_literal {
      builder = builder.resolve_to_addrs(&host, &addrs);
    }
    let client = builder
      .build()
      .map_err(|e| crate::LLMError::ConfigurationError {
        message: format!("failed to build HTTP client for source media fetch: {e}"),
      })?;

    let mut response = client.get(current.clone()).send().await?;
    if response.status().is_redirection() {
      if hop == MAX_REDIRECTS {
        return Err(crate::LLMError::NetworkError {
          message: format!("source media URL exceeded {MAX_REDIRECTS} redirects"),
        });
      }
      let loc = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| crate::LLMError::NetworkError {
          message: "source media redirect missing Location header".to_string(),
        })?;
      current = current
        .join(loc)
        .map_err(|e| crate::LLMError::ConfigurationError {
          message: format!("invalid redirect Location '{loc}': {e}"),
        })?;
      continue;
    }

    if !response.status().is_success() {
      let status_code = response.status().as_u16();
      let message = response.text().await.unwrap_or_default();
      return Err(crate::LLMError::HttpError {
        status_code,
        message,
      });
    }
    let mime = response
      .headers()
      .get(reqwest::header::CONTENT_TYPE)
      .and_then(|v| v.to_str().ok())
      .map(|v| v.split(';').next().unwrap_or(v).trim().to_string())
      .filter(|v| !v.is_empty());
    if let Some(declared) = response.content_length()
      && declared > max_bytes
    {
      return Err(crate::LLMError::ResponseParsingError {
        message: format!(
          "download of {current} declares {declared} bytes, over the {max_bytes}-byte cap"
        ),
      });
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
      if body.len() as u64 + chunk.len() as u64 > max_bytes {
        return Err(crate::LLMError::ResponseParsingError {
          message: format!("download of {current} exceeded the {max_bytes}-byte cap"),
        });
      }
      body.extend_from_slice(&chunk);
    }
    return Ok((body, mime));
  }

  Err(crate::LLMError::NetworkError {
    message: format!("source media URL exceeded {MAX_REDIRECTS} redirects"),
  })
}

fn is_public_media_ip(ip: std::net::IpAddr) -> bool {
  let ip = match ip {
    std::net::IpAddr::V6(v6) => v6
      .to_ipv4_mapped()
      .map(std::net::IpAddr::V4)
      .unwrap_or(std::net::IpAddr::V6(v6)),
    v4 => v4,
  };
  match ip {
    std::net::IpAddr::V4(v4) => {
      let o = v4.octets();
      !(v4.is_loopback()
        || o[0] == 0
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_broadcast()
        || v4.is_multicast()
        || v4.is_documentation()
        || (o[0] == 100 && (o[1] & 0b1100_0000) == 64)
        || (o[0] == 198 && (o[1] & 0xfe) == 18)
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)
        || (o[0] & 0xf0) == 240)
    }
    std::net::IpAddr::V6(v6) => {
      let s = v6.segments();
      !(v6.is_loopback()
        || v6.is_unspecified()
        || v6.is_multicast()
        || (s[0] & 0xffc0) == 0xfe80
        || (s[0] & 0xfe00) == 0xfc00
        || (s[0] == 0x2001 && s[1] == 0x0db8))
    }
  }
}

/// Y6.6: the bytes and MIME type of an image given as a `data:` URI or an
/// HTTP(S) URL, for vendors that take image bytes rather than a URL
/// (OpenAI `/images/edits`). URLs are SSRF-validated and fetched with a
/// `max_bytes` cap.
pub async fn source_image_bytes(source: &str, max_bytes: u64) -> crate::Result<(Vec<u8>, String)> {
  use base64::{Engine as _, engine::general_purpose::STANDARD};
  if let Some(rest) = source.strip_prefix("data:") {
    let (meta, data) =
      rest
        .split_once(',')
        .ok_or_else(|| crate::LLMError::ResponseParsingError {
          message: "source image data URI has no ','".to_string(),
        })?;
    let mime = meta.split(';').next().unwrap_or_default();
    if !meta.ends_with(";base64") || !mime.starts_with("image/") {
      return Err(crate::LLMError::ResponseParsingError {
        message: format!("source image data URI must be base64 image data, got '{meta}'"),
      });
    }
    let bytes =
      STANDARD
        .decode(data.trim())
        .map_err(|e| crate::LLMError::ResponseParsingError {
          message: format!("source image data URI is not valid base64: {e}"),
        })?;
    if bytes.len() as u64 > max_bytes {
      return Err(crate::LLMError::ResponseParsingError {
        message: format!("source image exceeds {max_bytes} bytes"),
      });
    }
    return Ok((bytes, mime.to_string()));
  }
  if source.starts_with("http://") || source.starts_with("https://") {
    let (bytes, content_type) = download_public_url_with_cap(source, max_bytes).await?;
    let mime = content_type
      .and_then(|c| c.split(';').next().map(str::trim).map(str::to_string))
      .filter(|c| c.starts_with("image/"))
      .unwrap_or_else(|| "image/png".to_string());
    return Ok((bytes, mime));
  }
  Err(crate::LLMError::ResponseParsingError {
    message: "source image must be a data: URI or an http(s) URL".to_string(),
  })
}
