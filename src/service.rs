use crate::adapters::TopologyAdapter;
use crate::domain::{normalize_graph, DependencyGraph};
use crate::storage::{GraphRepository, Neo4jProjector, ServiceDependency};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct SyncReport {
    pub adapters: usize,
    pub graphs_ingested: usize,
}

#[derive(Clone)]
pub struct GraphIngestionService<R: GraphRepository + 'static> {
    repository: Arc<R>,
    projector: Option<Neo4jProjector>,
    adapters: Vec<Arc<dyn TopologyAdapter>>,
}

impl<R: GraphRepository + 'static> GraphIngestionService<R> {
    pub fn new(
        repository: Arc<R>,
        projector: Option<Neo4jProjector>,
        adapters: Vec<Arc<dyn TopologyAdapter>>,
    ) -> Self {
        Self {
            repository,
            projector,
            adapters,
        }
    }

    pub async fn ingest(&self, graph: DependencyGraph) -> anyhow::Result<()> {
        let normalized = normalize_graph(graph);
        self.repository.upsert_graph(&normalized).await?;

        if let Some(projector) = &self.projector {
            projector.project(&normalized).await?;
        }

        Ok(())
    }

    pub async fn sync_all(&self) -> anyhow::Result<SyncReport> {
        let mut graphs_ingested = 0usize;

        for adapter in &self.adapters {
            let graphs = adapter.collect().await?;
            for graph in graphs {
                self.ingest(graph).await?;
                graphs_ingested += 1;
            }
        }

        Ok(SyncReport {
            adapters: self.adapters.len(),
            graphs_ingested,
        })
    }

    pub async fn snapshot(&self) -> anyhow::Result<DependencyGraph> {
        self.repository.get_graph().await
    }

    pub async fn service_dependencies(
        &self,
        service: &str,
    ) -> anyhow::Result<Vec<ServiceDependency>> {
        self.repository.dependencies_for_service(service).await
    }

    pub async fn healthcheck(&self) -> anyhow::Result<()> {
        self.repository.healthcheck().await
    }
}
