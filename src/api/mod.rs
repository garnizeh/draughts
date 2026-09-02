//! HTTP API — §9.
//!
//! The layer holds `Arc` handles to the engine pool, the transposition table,
//! the writer channel, and the Face facade. It owns none of them.
//!
//! CPU-bound engine work is executed via `spawn_blocking` or a dedicated rayon
//! pool, and inference is dispatched to the runtime's own thread. Neither ever
//! runs on a Tokio worker (§5.2).

pub mod health;
pub mod lab;
pub mod matches;
pub mod state;

use axum::Router;
use axum::routing::{get, post};
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

pub use state::AppState;

/// Base path for the JSON API (§9).
pub const API_BASE: &str = "/api/v1";

/// Build the router.
///
/// Route paths are the contract in §9 and are listed here in the same order, so
/// that a route added to one and not the other is visible in a diff.
pub fn router(state: AppState) -> Router {
    let static_dir = state.config.server.static_dir.clone();

    let api = Router::new()
        .route("/health", get(health::health))
        .route("/matches", post(matches::create))
        .route("/matches/{match_id}", get(matches::get))
        .route("/matches/{match_id}/moves", post(matches::submit_move))
        .route("/matches/{match_id}/resign", post(matches::resign))
        .route("/matches/{match_id}/commentary", post(matches::commentary))
        .route(
            "/lab/batches",
            post(lab::create_batch).get(lab::list_batches),
        )
        .route("/lab/batches/{batch_id}", get(lab::get_batch))
        .route("/lab/batches/{batch_id}/cancel", post(lab::cancel_batch))
        .route("/lab/batches/{batch_id}/export", get(lab::export_batch))
        .with_state(state);

    Router::new()
        .nest(API_BASE, api)
        .fallback_service(ServeDir::new(static_dir))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The router must build without a database, a model, or a table — a
    /// smoke test that catches a malformed path pattern, which axum only
    /// reports at construction time.
    #[test]
    fn the_router_builds() {
        let _ = router(AppState::for_tests());
    }
}
