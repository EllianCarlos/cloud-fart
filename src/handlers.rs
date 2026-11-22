use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

use crate::domain::DependencyGraph;
use crate::error::ApiError;
use crate::service::{GraphIngestionService, SyncReport};
use crate::storage::{ServiceDependency, SqliteGraphRepository};

#[derive(Clone)]
pub struct AppState {
    pub service: GraphIngestionService<SqliteGraphRepository>,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub status: &'static str,
}

pub async fn ingest(
    State(state): State<AppState>,
    Json(payload): Json<DependencyGraph>,
) -> Result<Json<IngestResponse>, ApiError> {
    if payload.source.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "source cannot be empty for ingestion payload".to_string(),
        ));
    }

    state.service.ingest(payload).await?;
    Ok(Json(IngestResponse { status: "ok" }))
}

pub async fn sync(State(state): State<AppState>) -> Result<Json<SyncReport>, ApiError> {
    Ok(Json(state.service.sync_all().await?))
}

pub async fn graph(State(state): State<AppState>) -> Result<Json<DependencyGraph>, ApiError> {
    Ok(Json(state.service.snapshot().await?))
}

pub async fn dependencies(
    Path(service): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<ServiceDependency>>, ApiError> {
    Ok(Json(state.service.service_dependencies(&service).await?))
}

pub async fn health(State(state): State<AppState>) -> Result<&'static str, ApiError> {
    state.service.healthcheck().await?;
    Ok("ok")
}
