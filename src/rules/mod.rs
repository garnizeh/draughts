//! Game Rules Core — §5.3.
//!
//! A pure domain library: no I/O, no persistence, no LLM concerns, and as few
//! dependencies as the job allows. Everything above this module trusts it to
//! own legality; nothing in it knows that anything above it exists.

pub mod board;
pub mod hashing;
pub mod moves;
pub mod zobrist;

pub use board::{Board, GameResult, GameState, GameStatus, SQUARES, Side};
pub use moves::{Move, MoveFlags, apply_move, generate_legal_moves};
pub use zobrist::Zobrist;
