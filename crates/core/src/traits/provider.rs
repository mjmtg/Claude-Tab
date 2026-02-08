use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub provider_id: String,
    pub working_directory: Option<String>,
    pub environment: HashMap<String, String>,
    pub args: Vec<String>,
}

impl SessionConfig {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            working_directory: None,
            environment: HashMap::new(),
            args: Vec::new(),
        }
    }
}

pub struct PtyHandle {
    pub master_writer: Box<dyn std::io::Write + Send>,
    pub master_reader: Box<dyn std::io::Read + Send>,
    pub size: PtySize,
}

impl fmt::Debug for PtyHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PtyHandle")
            .field("size", &self.size)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

impl Default for PtySize {
    fn default() -> Self {
        Self { rows: 24, cols: 80 }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Spawn failed: {0}")]
    SpawnFailed(String),
    #[error("Provider not found: {0}")]
    NotFound(String),
}

#[async_trait]
pub trait SessionProvider: Send + Sync + 'static {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    async fn spawn(&self, config: &SessionConfig) -> Result<PtyHandle, ProviderError>;
    fn prepare_environment(&self, env: &mut HashMap<String, String>);
}

pub struct ProviderRegistry {
    providers: Arc<RwLock<HashMap<String, Box<dyn SessionProvider>>>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, provider: Box<dyn SessionProvider>) {
        let id = provider.id().to_string();
        self.providers.write().await.insert(id, provider);
    }

    pub async fn get_provider_ids(&self) -> Vec<String> {
        self.providers.read().await.keys().cloned().collect()
    }

    pub async fn spawn(
        &self,
        config: &SessionConfig,
    ) -> Result<PtyHandle, ProviderError> {
        let providers = self.providers.read().await;
        let provider = providers
            .get(&config.provider_id)
            .ok_or_else(|| ProviderError::NotFound(config.provider_id.clone()))?;
        provider.spawn(config).await
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
