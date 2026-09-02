//! Cross-cutting properties that no single module owns.
//!
//! Each of these is a claim the architecture makes about the system as a whole.
//! They live here because a module test can only assert its own half.

use std::sync::Arc;

use draughts::config::{Config, HugePages, validate};
use draughts::db;
use draughts::engine::TranspositionTable;
use draughts::face::{CircuitBreaker, DeviceKind, DeviceRequest, select_device};

/// §19.6.5 property 3 and §20.10: `cargo build` with no features produces a
/// binary with no CUDA dependency, and a CUDA request on such a build resolves
/// to CPU rather than failing.
#[test]
#[cfg(not(feature = "cuda"))]
fn the_default_build_resolves_every_request_to_cpu() {
    for request in [
        DeviceRequest::Cpu,
        DeviceRequest::Auto,
        DeviceRequest::Cuda { ordinal: 0 },
    ] {
        let (_, kind) = select_device(request);
        assert_eq!(kind, DeviceKind::Cpu, "for {request:?}");
    }
}

/// §7.4.1 rule 4, restated as the invariant the whole Face layer rests on:
/// resolving a device never returns an error, on any build, for any request.
#[test]
fn device_resolution_never_fails() {
    for request in [
        DeviceRequest::Cpu,
        DeviceRequest::Auto,
        DeviceRequest::Cuda { ordinal: 0 },
        DeviceRequest::Cuda { ordinal: 7 },
    ] {
        let (_, kind) = select_device(request);
        assert!(matches!(kind, DeviceKind::Cpu | DeviceKind::Cuda { .. }));
    }
}

/// §22.3 steps 1–7, minus the listener: a default configuration must carry the
/// process all the way to "ready to serve" without a model file, a GPU, or a
/// pre-existing database.
#[test]
fn a_fresh_deployment_starts_from_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");

    let mut config = Config::default();
    config.database.path = dir.path().join("data").join("draughts.db");
    // The committed 21 GB table is not something a test runner should allocate.
    config.engine.transposition.capacity_entries = 1024;
    config.engine.transposition.shard_count = 4;
    config.engine.transposition.huge_pages = HugePages::Off;

    // 1 · Configuration validates.
    let (_, device_kind) = select_device(DeviceRequest::from_config(
        config.face.device,
        config.face.device_index,
    ));
    let report = validate::validate(&config, device_kind);
    assert!(report.is_ok(), "{:?}", report.errors);

    // 2 · The writer connection opens and migrations apply, creating the
    //     directory the operator did not.
    let mut conn = db::pool::open_writer(&config.database).expect("writer opens");
    let schema_version = db::migrations::run(&mut conn).expect("migrations apply");
    assert_eq!(schema_version, db::migrations::target_version());

    // 5 · The read pool opens against the file the writer just created.
    let reader = db::pool::open_reader(&config.database).expect("reader opens");
    let batches: i64 = reader
        .query_row("SELECT COUNT(*) FROM lab_batches", [], |row| row.get(0))
        .expect("the schema is queryable");
    assert_eq!(batches, 0);

    // 6 · The table allocates.
    let tt = TranspositionTable::with_capacity(
        config.engine.transposition.capacity_entries,
        config.engine.transposition.shard_count,
    );
    assert_eq!(tt.len(), 0);

    // 8 · The breaker exists and is closed, whether or not a model ever loads.
    let breaker = CircuitBreaker::new(&config.face.circuit_breaker);
    assert_eq!(breaker.trips_total(), 0);

    // Restarting over the same file is not a special case.
    let mut conn = db::pool::open_writer(&config.database).expect("writer reopens");
    assert_eq!(
        db::migrations::run(&mut conn).expect("migrations are idempotent"),
        schema_version
    );
}

/// §5.4 and §20.5: the table is a performance optimisation, never load-bearing
/// for correctness — which starts with the engine accepting a disabled one.
#[test]
fn the_engine_accepts_a_disabled_table() {
    use draughts::engine::{EvaluationStrategy, MctsConfig, MctsEngine, RandomRolloutEvaluator};

    let config = Config::default();
    let engine = MctsEngine::new(
        RandomRolloutEvaluator::new(0, 200),
        MctsConfig::from_search_config(&config.engine.play, 0),
        TranspositionTable::disabled(),
    );

    assert!(engine.transposition_table().is_disabled());
    assert_eq!(engine.evaluator().name(), "random_rollout");
}

/// §9.2: `/health` must be answerable by a process that loaded no model, has no
/// GPU, and has an empty database. It is the endpoint a supervisor polls, and
/// it must not depend on anything optional.
#[tokio::test]
async fn health_is_answerable_with_nothing_loaded() {
    use axum::extract::State;

    let state = draughts::api::AppState::for_tests();
    let axum::Json(response) = draughts::api::health::health(State(state)).await;

    assert_eq!(response.status, "ok");
    assert_eq!(response.version, draughts::VERSION);
    assert!(!response.face.model_loaded);
}

/// A router that does not build is a startup panic, and axum only finds a
/// malformed path pattern at construction time.
#[test]
fn the_router_builds_from_a_default_configuration() {
    let config = Arc::new(Config::default());
    assert_eq!(config.server.static_dir.to_string_lossy(), "./static");

    let _ = draughts::api::router(draughts::api::AppState::for_tests());
}
