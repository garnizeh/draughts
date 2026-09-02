//! Training Lab Runner — §5.5.
//!
//! Headless CPU-vs-CPU self-play across a worker pool, all workers sharing one
//! transposition table, all writes funnelled through one batching actor.
//!
//! A lab worker holds no database connection and cannot begin a transaction.
//! That is not a convention; it is why the writer actor can batch at all.

pub mod runner;
pub mod sampling;

pub use runner::{BatchHandle, BatchProgress, BatchRequest, LabRunner};
pub use sampling::Sampler;
