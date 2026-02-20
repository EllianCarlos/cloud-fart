use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,
    #[serde(default)]
    pub sqlite: SqliteConfig,
    pub neo4j: Option<Neo4jConfig>,
    #[serde(default)]
    pub adapters: crate::adapters::AdapterRegistry,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SqliteConfig {
    #[serde(default = "default_sqlite_path")]
    pub path: String,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: default_sqlite_path(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Neo4jConfig {
    pub uri: String,
    pub user: String,
    pub password: String,
}

fn default_bind_addr() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_sqlite_path() -> String {
    "./data/cloud-fart.db".to_string()
}

pub fn load() -> anyhow::Result<AppConfig> {
    let builder = config::Config::builder()
        .add_source(config::File::with_name("config/service").required(false))
        .add_source(config::Environment::with_prefix("CLOUD_FART").separator("__"));

    Ok(builder.build()?.try_deserialize::<AppConfig>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_defaults_to_local_data_path() {
        let config = AppConfig {
            bind_addr: "127.0.0.1:8080".to_string(),
            sqlite: SqliteConfig::default(),
            neo4j: None,
            adapters: Default::default(),
        };

        assert!(config.sqlite.path.ends_with("cloud-fart.db"));
    }
}
