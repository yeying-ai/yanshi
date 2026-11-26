//! Pure stateless Notebook domain helpers shared by `yanshi-server` and
//! `yanshi-studio-desktop`.
//!
//! This crate intentionally owns zero HTTP framework (`axum`), zero database
//! (`sqlx`/`yanshi-db`), and zero desktop shell (`tauri`) dependencies. It
//! centralizes the pure domain logic that both surfaces execute identically:
//!
//! - [`ingest`] — source format sniffing, smart URL rewriting & GitHub
//!   repository snapshots, HTML/PDF/OCR/OOXML/SRT/CSV/JSON extraction, ASR
//!   transcription, and paragraph chunking.
//! - [`qa`] — grounded `/ask` JSON schema, reply parsing, citation extraction,
//!   and multi-turn history normalization/retrieval hints.
//! - [`studio`] — compiled-in Content Studio generator catalogue, planner
//!   schema/sanitization, batch cost-limit splitting, and BM25 + round-robin
//!   chunk budget selection.
//! - [`bundle`] — portable `NotebookExportBundle` DTOs, import validation,
//!   ID remapping, deliverable synthesis, Markdown briefing rendering, and
//!   deliverable `.zip` packaging.

pub mod bundle;
pub mod ingest;
pub mod qa;
pub mod studio;
