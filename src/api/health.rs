//! `GET /api/v1/health` — §9.2.
//!
//! `status` reports `"ok"` while the *engine and database* are healthy. A
//! tripped circuit or an unloaded model does not make the service unhealthy —
//! it makes it less entertaining. That distinction is what keeps an external
//! supervisor from restarting a perfectly functional server because a taunt
//! timed out.

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::VERSION;
use crate::api::AppState;
use crate::face::CircuitState;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub engine: &'static str,
    pub evaluator: String,
    pub sqlite_wal: bool,
    pub schema_version: u32,
    pub face: FaceHealth,
    pub transposition_table: TtHealth,
    pub writer: WriterHealth,
}

/// The `device*` and `vram_*` fields exist to answer one question without
/// reading logs: **did the Face layer get the device it asked for?**
/// `device_requested: "auto"` with `device: "cpu"` is the configuration in
/// which commentary is silently running the fallback profile (§9.2).
#[derive(Debug, Serialize)]
pub struct FaceHealth {
    pub enabled: bool,
    pub provider: &'static str,
    pub device_requested: &'static str,
    pub device: String,
    pub device_name: Option<String>,
    pub profile: &'static str,
    pub model_id: String,
    pub model_loaded: bool,
    /// Host RSS attributable to the Face layer on either device, which is why
    /// it is small when the weights are on the card.
    pub resident_mb: u64,
    pub vram_used_mb: u64,
    pub vram_budget_mb: Option<u64>,
    pub circuit: CircuitState,
    pub consecutive_failures: u32,
    pub trips_total: u64,
    pub cooldown_remaining_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct TtHealth {
    pub mode: &'static str,
    pub entries: usize,
    pub capacity: usize,
    pub resident_mb: u64,
    pub hit_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct WriterHealth {
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub rows_committed: u64,
    pub last_commit_ms: u64,
    pub last_error: Option<String>,
}

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let face = &state.face_status;
    let tt_stats = state.tt.stats();
    let now_ms = state.clock.now_ms();

    Json(HealthResponse {
        status: "ok",
        version: VERSION,
        engine: "mcts",
        evaluator: state.config.engine.play.evaluator.clone(),
        // TODO(§11.2, §16.3): `sqlite_wal` and `transposition_table.resident_mb`
        // below, and the whole `writer` block, are placeholder literals until
        // pool/writer-actor introspection exists (§5.6, §5.2). They read as
        // healthy regardless of actual state — do not treat this endpoint as a
        // real backpressure signal until that wiring lands.
        sqlite_wal: true,
        schema_version: state.schema_version,
        face: FaceHealth {
            enabled: face.enabled,
            provider: face.provider,
            device_requested: face.device_requested.as_health_string(),
            device: face.device.as_health_string(),
            device_name: face.device_name.clone(),
            profile: face.device.profile_name(),
            model_id: face.model_id.clone(),
            model_loaded: face.model_loaded,
            resident_mb: face.resident_mb,
            vram_used_mb: face.vram_used_mb,
            vram_budget_mb: face.vram_budget_mb,
            circuit: state.breaker.state(),
            consecutive_failures: state.breaker.consecutive_failures(),
            trips_total: state.breaker.trips_total(),
            cooldown_remaining_seconds: state.breaker.cooldown_remaining_seconds(now_ms),
        },
        transposition_table: TtHealth {
            // Same section `evaluator` reads above: Play Mode is what is live
            // between lab batches, which is what `/health` reports the rest
            // of the time (§9.2).
            mode: match state.config.engine.play.transposition_mode {
                crate::config::TtMode::Deterministic => "deterministic",
                crate::config::TtMode::Throughput => "throughput",
            },
            entries: state.tt.len(),
            capacity: state.tt.capacity(),
            resident_mb: 0,
            hit_rate: tt_stats.hit_rate(),
        },
        writer: WriterHealth {
            queue_depth: 0,
            queue_capacity: state.config.database.writer.channel_capacity,
            rows_committed: 0,
            last_commit_ms: 0,
            last_error: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::FaceError;

    /// §9.2: an open circuit is a *designed* steady state, and a supervisor
    /// reading `status` must not restart the process over it.
    #[tokio::test]
    async fn a_tripped_circuit_does_not_make_the_service_unhealthy() {
        let state = AppState::for_tests();
        for tick in 0..3 {
            state.breaker.on_failure(tick, &FaceError::Timeout);
        }

        let Json(response) = health(State(state)).await;

        assert_eq!(response.status, "ok");
        assert_eq!(response.face.circuit, CircuitState::Open);
        assert_eq!(response.face.trips_total, 1);
    }

    /// §20.3: with no model present, `/health` says so plainly and the service
    /// is still healthy.
    #[tokio::test]
    async fn an_unloaded_model_is_reported_rather_than_hidden() {
        let Json(response) = health(State(AppState::for_tests())).await;

        assert!(!response.face.model_loaded);
        assert_eq!(response.status, "ok");
        assert_eq!(response.face.device, "cpu");
        assert_eq!(response.face.profile, "cpu_profile");
        assert_eq!(
            response.face.vram_budget_mb, None,
            "§9.2: null on the CPU path"
        );
    }

    /// The `device_requested` vs `device` pair is the whole point of those
    /// fields: `auto`/`cpu` is the silent-fallback state an operator should be
    /// able to see rather than infer from latency.
    #[tokio::test]
    async fn health_reports_what_was_asked_for_beside_what_was_resolved() {
        let Json(response) = health(State(AppState::for_tests())).await;

        assert_eq!(response.face.device_requested, "auto");
        assert_eq!(response.face.device, "cpu");
    }
}
