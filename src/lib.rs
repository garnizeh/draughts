//! Draughts — a single-binary draughts engine and self-play training lab.
//!
//! The crate is laid out along the seams in [§4 Separation of Concerns]. Calls
//! run downward only: `api` → services → (`rules`, `engine`, `lab`, `face`) →
//! (`db::writer`, `engine::transposition`). No module below reaches back up.
//!
//! Three invariants hold across the whole tree, and each is checked by CI:
//!
//! 1. Reading a persisted BLOB without dispatching on its `format_version` is a
//!    review-blocking defect ([§13.7]).
//! 2. The transposition table may change how long a search takes and must never
//!    change what it returns ([§20.5]).
//! 3. `candle_core::Device` is constructed in exactly one function —
//!    [`face::device::select_device`] ([§19.6.5]).
//!
//! [§4 Separation of Concerns]: ../docs/architecture/04-separation-of-concerns.md
//! [§13.7]: ../docs/architecture/13-data-dictionary.md
//! [§20.5]: ../docs/architecture/20-testing-strategy.md
//! [§19.6.5]: ../docs/architecture/19-extensibility-roadmap.md

pub mod api;
pub mod config;
pub mod db;
pub mod engine;
pub mod error;
pub mod face;
pub mod lab;
pub mod rules;
pub mod telemetry;

/// Version reported by `GET /api/v1/health` (§9.2).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The `format_version` written to every new row (§13.7).
///
/// Bump this whenever a persisted BLOB encoding changes — including a
/// regenerated Zobrist key table, which silently invalidates every stored
/// `positions.board_hash`.
pub const CURRENT_FORMAT_VERSION: u32 = 1;
