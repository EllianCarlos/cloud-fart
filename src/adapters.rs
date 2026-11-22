use crate::domain::DependencyGraph;
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct AdapterConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub kind: AdapterKind,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

impl AdapterConfig {
    pub fn directory(&self) -> Option<&str> {
        self.options.get("directory").map(String::as_str)
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    #[default]
    File,
}

pub type AdapterRegistry = BTreeMap<String, AdapterConfig>;

#[async_trait]
pub trait TopologyAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn collect(&self) -> anyhow::Result<Vec<DependencyGraph>>;
}

#[derive(Debug, Clone)]
pub struct FileTopologyAdapter {
    name: String,
    directory: PathBuf,
}

impl FileTopologyAdapter {
    pub fn from_config(name: String, config: &AdapterConfig) -> anyhow::Result<Self> {
        let directory = config
            .directory()
            .ok_or_else(|| anyhow::anyhow!("adapter '{name}' requires options.directory"))?;

        Ok(Self {
            name,
            directory: Path::new(directory).to_path_buf(),
        })
    }
}

#[async_trait]
impl TopologyAdapter for FileTopologyAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    async fn collect(&self) -> anyhow::Result<Vec<DependencyGraph>> {
        if !self.directory.exists() {
            return Ok(Vec::new());
        }

        let mut entries = tokio::fs::read_dir(&self.directory).await?;
        let mut graphs = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            let raw = tokio::fs::read_to_string(&path).await?;
            let graph: DependencyGraph = serde_json::from_str(&raw)
                .map_err(|err| anyhow::anyhow!("failed parsing '{}': {err}", path.display()))?;
            graphs.push(graph);
        }

        Ok(graphs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn file_adapter_reads_json_graphs() {
        let tempdir = tempfile::tempdir().unwrap();
        let file = tempdir.path().join("topology.json");
        tokio::fs::write(&file, r#"{"source":"x","services":[],"communications":[]}"#)
            .await
            .unwrap();

        let adapter = FileTopologyAdapter {
            name: "file-source".to_string(),
            directory: tempdir.path().to_path_buf(),
        };

        let graphs = adapter.collect().await.unwrap();
        assert_eq!(graphs.len(), 1);
        assert_eq!(graphs[0].source, "x");
    }
}
