//! Training lab endpoints — §9.8 to §9.12.

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::error::{ApiError, ApiResult};

#[derive(Debug, Deserialize)]
pub struct CreateBatchRequest {
    pub name: String,
    pub target_games: u64,
    pub seed: Option<u64>,
    #[serde(default)]
    pub reproducible: bool,
}

#[derive(Debug, Serialize)]
pub struct BatchResponse {
    pub batch_id: i64,
    pub status: &'static str,
    pub target_games: u64,
    pub completed_games: u64,
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub after_id: Option<i64>,
}

/// `POST /api/v1/lab/batches`
pub async fn create_batch(
    State(state): State<AppState>,
    Json(request): Json<CreateBatchRequest>,
) -> ApiResult<Json<BatchResponse>> {
    let _ = (state, request);
    Err(ApiError::Internal(anyhow::anyhow!(
        "batch creation is not implemented — §9.8"
    )))
}

/// `GET /api/v1/lab/batches`
pub async fn list_batches(State(state): State<AppState>) -> ApiResult<Json<Vec<BatchResponse>>> {
    let _ = state;
    Ok(Json(Vec::new()))
}

/// `GET /api/v1/lab/batches/{batch_id}`
pub async fn get_batch(
    State(state): State<AppState>,
    Path(batch_id): Path<i64>,
) -> ApiResult<Json<BatchResponse>> {
    let _ = (state, batch_id);
    Err(ApiError::NotFound { what: "batch" })
}

/// `POST /api/v1/lab/batches/{batch_id}/cancel`
///
/// Two-phase: this moves the batch to `cancelling` and returns. It becomes
/// `cancelled` once every worker has reached a game boundary and the drain has
/// completed (§17.6).
pub async fn cancel_batch(
    State(state): State<AppState>,
    Path(batch_id): Path<i64>,
) -> ApiResult<Json<BatchResponse>> {
    let _ = (state, batch_id);
    Err(ApiError::NotFound { what: "batch" })
}

/// `GET /api/v1/lab/batches/{batch_id}/export`
///
/// Every exported JSONL line carries a `format_version` field (§20.8).
pub async fn export_batch(
    State(state): State<AppState>,
    Path(batch_id): Path<i64>,
    Query(query): Query<ExportQuery>,
) -> ApiResult<String> {
    let _ = (state, batch_id, query);
    Err(ApiError::NotFound { what: "batch" })
}
