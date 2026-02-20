mod adapters;
mod app_config;
mod domain;
mod error;
mod handlers;
mod service;
mod storage;

use adapters::{AdapterKind, FileTopologyAdapter, TopologyAdapter};
use anyhow::Context;
use axum::{routing::get, routing::post, Router};
use handlers::{dependencies, graph, health, ingest, sync, AppState};
use std::sync::Arc;
use storage::{Neo4jProjector, SqliteGraphRepository};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app_config = app_config::load().context("failed loading app config")?;

    if let Some(parent) = std::path::Path::new(&app_config.sqlite.path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let sqlite =
        SqliteGraphRepository::new(&format!("sqlite://{}", app_config.sqlite.path)).await?;

    let neo4j = match app_config.neo4j {
        Some(neo4j_cfg) => Some(
            Neo4jProjector::connect(&neo4j_cfg.uri, &neo4j_cfg.user, &neo4j_cfg.password)
                .await
                .context("failed to connect to neo4j")?,
        ),
        None => None,
    };

    let enabled_adapters: Vec<Arc<dyn TopologyAdapter>> = app_config
        .adapters
        .iter()
        .filter(|(_, cfg)| cfg.enabled)
        .map(|(name, cfg)| match cfg.kind {
            AdapterKind::File => FileTopologyAdapter::from_config(name.clone(), cfg),
        })
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .map(|adapter| Arc::new(adapter) as Arc<dyn TopologyAdapter>)
        .collect();

    let state = AppState {
        service: service::GraphIngestionService::new(Arc::new(sqlite), neo4j, enabled_adapters),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/ingest", post(ingest))
        .route("/v1/graph", get(graph))
        .route("/v1/sync", post(sync))
        .route("/v1/services/:service/dependencies", get(dependencies))
        .with_state(state);

    info!(bind = %app_config.bind_addr, "starting cloud-fart service");

    let listener = tokio::net::TcpListener::bind(&app_config.bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
