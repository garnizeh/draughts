//! Match endpoints — §9.3 to §9.7.

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::error::{ApiError, ApiResult};

#[derive(Debug, Deserialize)]
pub struct CreateMatchRequest {
    /// `"black"` or `"white"`; the engine takes the other side.
    pub human_side: String,
    pub seed: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct MatchResponse {
    pub match_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitMoveRequest {
    pub from: u8,
    pub to: u8,
}

/// `POST /api/v1/matches`
pub async fn create(
    State(state): State<AppState>,
    Json(request): Json<CreateMatchRequest>,
) -> ApiResult<Json<MatchResponse>> {
    let _ = (state, request);
    Err(ApiError::Internal(anyhow::anyhow!(
        "match creation is not implemented — §9.3"
    )))
}

/// `GET /api/v1/matches/{match_id}`
pub async fn get(
    State(state): State<AppState>,
    Path(match_id): Path<String>,
) -> ApiResult<Json<MatchResponse>> {
    let _ = (state, match_id);
    Err(ApiError::NotFound { what: "match" })
}

/// `POST /api/v1/matches/{match_id}/moves`
///
/// The engine answers at engine speed. Commentary arrives separately, whenever
/// the model finishes, because a *successful* 2.5-second inference on the
/// critical path is still 2.5 seconds (§8).
pub async fn submit_move(
    State(state): State<AppState>,
    Path(match_id): Path<String>,
    Json(request): Json<SubmitMoveRequest>,
) -> ApiResult<Json<MatchResponse>> {
    let _ = (state, match_id, request);
    Err(ApiError::Internal(anyhow::anyhow!(
        "move submission is not implemented — §9.5"
    )))
}

/// `POST /api/v1/matches/{match_id}/resign`
pub async fn resign(
    State(state): State<AppState>,
    Path(match_id): Path<String>,
) -> ApiResult<Json<MatchResponse>> {
    let _ = (state, match_id);
    Err(ApiError::NotFound { what: "match" })
}

#[derive(Debug, Serialize)]
pub struct CommentaryResponse {
    pub text: String,
    pub provider: &'static str,
    pub fallback_used: bool,
    pub fallback_reason: Option<&'static str>,
}

/// `POST /api/v1/matches/{match_id}/commentary`
///
/// Always `200`. Commentary served from the canned fallback is a designed
/// steady state carrying a `fallback_reason`, never an error status (§9.1).
pub async fn commentary(
    State(state): State<AppState>,
    Path(match_id): Path<String>,
) -> ApiResult<Json<CommentaryResponse>> {
    let _ = (state, match_id);
    Err(ApiError::NotFound { what: "match" })
}
