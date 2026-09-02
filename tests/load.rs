//! Load and volume tests — §20.4.
//!
//! Every test here is `#[ignore]`d: they take minutes to hours and are run by
//! the nightly job (`just test-load`), never by the merge gate. A load test in
//! the gate is a load test that gets deleted.
//!
//! The bodies are `todo!()` until the writer actor and the lab runner exist.
//! They are committed now, named and ignored, because §20.4 is a list of
//! properties this system claims — and a claim with no test attached to it is
//! how the list quietly becomes aspirational.

/// Insert 1M games. v1.0's figure was 10k; the writer actor changes what counts
/// as a load test.
#[test]
#[ignore = "load test — minutes; run via `just test-load`"]
fn one_million_games() {
    todo!("§20.4")
}

/// 5M+ sampled position and edge rows at the default sampling density.
#[test]
#[ignore = "load test — minutes; run via `just test-load`"]
fn five_million_sampled_rows() {
    todo!("§20.4")
}

/// Batch size is a performance knob and not a correctness one: the same rows
/// must land at `db_batch_rows` of 1, 1 000, and 50 000.
#[test]
#[ignore = "load test — minutes; run via `just test-load`"]
fn commit_throughput_across_batch_sizes() {
    todo!("§20.4")
}

/// Manual checkpointing keeps the WAL under `journal_size_limit` across a
/// multi-hour run.
#[test]
#[ignore = "load test — hours; run via `just test-load`"]
fn the_wal_stays_bounded() {
    todo!("§20.4")
}

/// Status and export queries stay responsive while the writer sustains 50k-row
/// commits.
#[test]
#[ignore = "load test — minutes; run via `just test-load`"]
fn no_reader_starvation() {
    todo!("§20.4")
}

/// §11.2.5's three producer policies, driven against a full channel: lab
/// blocks, durable writes return `write_queue_saturated`, telemetry drops with
/// a counter.
#[test]
#[ignore = "load test — minutes; run via `just test-load`"]
fn queue_high_water_behaviour() {
    todo!("§20.4")
}

/// **A gate, not a metric.** A run that exceeds the §16.1 budget fails the
/// build regardless of how fast it was.
#[test]
#[ignore = "load test — minutes; run via `just test-load`"]
fn peak_rss_stays_within_the_memory_budget() {
    todo!("§20.4, §16.1")
}
