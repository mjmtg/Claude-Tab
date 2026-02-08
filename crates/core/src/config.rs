use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConfigLayer {
    Default = 0,
    User = 1,
    Project = 2,
    Runtime = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValue {
    pub value: serde_json::Value,
    pub layer: ConfigLayer,
}

#[derive(Clone)]
pub struct Config {
    values: Arc<RwLock<HashMap<String, Vec<ConfigValue>>>>,
    schemas: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            values: Arc::new(RwLock::new(HashMap::new())),
            schemas: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn load_from_file(&self, path: &PathBuf, layer: ConfigLayer) -> Result<(), ConfigError> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        let table: toml::Table = content
            .parse()
            .map_err(|e: toml::de::Error| ConfigError::ParseError(e.to_string()))?;

        self.merge_toml(&table, "", layer).await;
        debug!(?layer, path = %path.display(), "Loaded config");
        Ok(())
    }

    async fn merge_toml(&self, table: &toml::Table, prefix: &str, layer: ConfigLayer) {
        for (key, value) in table {
            let full_key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", prefix, key)
            };

            match value {
                toml::Value::Table(subtable) => {
                    Box::pin(self.merge_toml(subtable, &full_key, layer)).await;
                }
                _ => {
                    let json_value = toml_to_json(value);
                    self.set_value(&full_key, json_value, layer).await;
                }
            }
        }
    }

    pub async fn set_value(&self, key: &str, value: serde_json::Value, layer: ConfigLayer) {
        let mut values = self.values.write().await;
        let entry = values.entry(key.to_string()).or_insert_with(Vec::new);
        entry.retain(|v| v.layer != layer);
        entry.push(ConfigValue { value, layer });
        entry.sort_by_key(|v| v.layer);
    }

    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        let values = self.values.read().await;
        values
            .get(key)
            .and_then(|entries| entries.last())
            .map(|v| v.value.clone())
    }

    pub async fn get_or_default(&self, key: &str, default: serde_json::Value) -> serde_json::Value {
        self.get(key).await.unwrap_or(default)
    }

    pub async fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).await.and_then(|v| v.as_str().map(String::from))
    }

    pub async fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key).await.and_then(|v| v.as_u64())
    }

    pub async fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).await.and_then(|v| v.as_bool())
    }

    pub async fn register_schema(&self, extension_id: &str, schema: serde_json::Value) {
        self.schemas
            .write()
            .await
            .insert(extension_id.to_string(), schema);
    }

    pub async fn all_keys(&self) -> Vec<String> {
        self.values.read().await.keys().cloned().collect()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::json!(*i),
        toml::Value::Float(f) => serde_json::json!(*f),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(toml_to_json).collect())
        }
        toml::Value::Table(table) => {
            let map: serde_json::Map<String, serde_json::Value> = table
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
    }
}
