use crate::domain::DependencyGraph;
use async_trait::async_trait;
use neo4rs::{query, Graph};
use serde::Serialize;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct ServiceDependency {
    pub source: String,
    pub target: String,
    pub protocol: String,
}

#[async_trait]
pub trait GraphRepository: Send + Sync {
    async fn upsert_graph(&self, graph: &DependencyGraph) -> anyhow::Result<()>;
    async fn get_graph(&self) -> anyhow::Result<DependencyGraph>;
    async fn dependencies_for_service(
        &self,
        service: &str,
    ) -> anyhow::Result<Vec<ServiceDependency>>;
    async fn healthcheck(&self) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct SqliteGraphRepository {
    pool: SqlitePool,
}

impl SqliteGraphRepository {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS services (
                name TEXT PRIMARY KEY,
                metadata TEXT NOT NULL
            );
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS communications (
                source TEXT NOT NULL,
                target TEXT NOT NULL,
                protocol TEXT NOT NULL,
                metadata TEXT NOT NULL,
                PRIMARY KEY (source, target, protocol)
            );
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl GraphRepository for SqliteGraphRepository {
    async fn upsert_graph(&self, graph: &DependencyGraph) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        for service in &graph.services {
            sqlx::query(
                "INSERT INTO services(name, metadata) VALUES(?, ?) ON CONFLICT(name) DO UPDATE SET metadata=excluded.metadata",
            )
            .bind(&service.name)
            .bind(serde_json::to_string(&service.metadata)?)
            .execute(&mut *tx)
            .await?;
        }

        for edge in &graph.communications {
            sqlx::query(
                "INSERT INTO communications(source, target, protocol, metadata) VALUES(?, ?, ?, ?) ON CONFLICT(source, target, protocol) DO UPDATE SET metadata=excluded.metadata",
            )
            .bind(&edge.source)
            .bind(&edge.target)
            .bind(&edge.protocol)
            .bind(serde_json::to_string(&edge.metadata)?)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_graph(&self) -> anyhow::Result<DependencyGraph> {
        let services = sqlx::query("SELECT name, metadata FROM services")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| crate::domain::ServiceNode {
                name: row.get("name"),
                metadata: serde_json::from_str(row.get::<String, _>("metadata").as_str())
                    .unwrap_or_default(),
            })
            .collect::<Vec<_>>();

        let communications =
            sqlx::query("SELECT source, target, protocol, metadata FROM communications")
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|row| crate::domain::CommunicationEdge {
                    source: row.get("source"),
                    target: row.get("target"),
                    protocol: row.get("protocol"),
                    metadata: serde_json::from_str(row.get::<String, _>("metadata").as_str())
                        .unwrap_or_default(),
                })
                .collect::<Vec<_>>();

        Ok(DependencyGraph {
            source: "aggregated".to_string(),
            services,
            communications,
        })
    }

    async fn dependencies_for_service(
        &self,
        service: &str,
    ) -> anyhow::Result<Vec<ServiceDependency>> {
        let deps = sqlx::query(
            "SELECT source, target, protocol FROM communications WHERE source = ? ORDER BY target",
        )
        .bind(service)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| ServiceDependency {
            source: row.get("source"),
            target: row.get("target"),
            protocol: row.get("protocol"),
        })
        .collect();

        Ok(deps)
    }

    async fn healthcheck(&self) -> anyhow::Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Neo4jProjector {
    graph: Arc<Graph>,
}

impl Neo4jProjector {
    pub async fn connect(uri: &str, user: &str, password: &str) -> anyhow::Result<Self> {
        let graph = Arc::new(Graph::new(uri, user, password).await?);
        Ok(Self { graph })
    }

    pub async fn project(&self, graph: &DependencyGraph) -> anyhow::Result<()> {
        for service in &graph.services {
            self.graph
                .run(query("MERGE (:Service {name: $name})").param("name", &service.name))
                .await?;
        }

        for edge in &graph.communications {
            self.graph
                .run(
                    query(
                        "MERGE (src:Service {name: $source}) MERGE (dst:Service {name: $target}) MERGE (src)-[:CALLS {protocol: $protocol}]->(dst)",
                    )
                    .param("source", &edge.source)
                    .param("target", &edge.target)
                    .param("protocol", &edge.protocol),
                )
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CommunicationEdge, DependencyGraph, ServiceNode};
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn sqlite_roundtrip_graph() {
        let repo = SqliteGraphRepository::new("sqlite::memory:").await.unwrap();

        let graph = DependencyGraph {
            source: "unit-test".into(),
            services: vec![ServiceNode {
                name: "svc-a".into(),
                metadata: BTreeMap::new(),
            }],
            communications: vec![CommunicationEdge {
                source: "svc-a".into(),
                target: "svc-b".into(),
                protocol: "grpc".into(),
                metadata: BTreeMap::new(),
            }],
        };

        repo.upsert_graph(&graph).await.unwrap();
        let loaded = repo.get_graph().await.unwrap();
        let dependencies = repo.dependencies_for_service("svc-a").await.unwrap();

        assert_eq!(loaded.services.len(), 1);
        assert_eq!(loaded.communications.len(), 1);
        assert_eq!(dependencies.len(), 1);
        repo.healthcheck().await.unwrap();
    }
}
