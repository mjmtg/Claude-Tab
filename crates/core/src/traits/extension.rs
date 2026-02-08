use crate::config::Config;
use crate::event_bus::EventBus;
use crate::session::SessionStore;
use crate::state_machine::StateRegistry;
use crate::traits::detector::DetectorRegistry;
use crate::traits::provider::ProviderRegistry;
use crate::traits::reaction::ReactionRegistry;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub dependencies: Vec<String>,
}

impl ExtensionManifest {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: "0.1.0".to_string(),
            description: String::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }
}

/// Context provided to extensions during activation.
///
/// Contains references to all core services that extensions may need:
/// - `event_bus`: Publish/subscribe event system
/// - `config`: Application configuration
/// - `state_registry`: State machine states
/// - `detector_registry`: State detectors (output parsers, hooks)
/// - `reaction_registry`: State change reactions
/// - `session_store`: Active session management
/// - `provider_registry`: Session providers (bash, claude-code)
pub struct ActivationContext {
    /// Event bus for pub/sub communication
    pub event_bus: Arc<EventBus>,

    /// Application configuration
    pub config: Arc<Config>,

    /// State registry for state machine
    pub state_registry: Arc<StateRegistry>,

    /// Detector registry for state detection
    pub detector_registry: Arc<DetectorRegistry>,

    /// Reaction registry for state change reactions
    pub reaction_registry: Arc<ReactionRegistry>,

    /// Session store for active sessions
    pub session_store: Arc<SessionStore>,

    /// Provider registry for session providers
    pub provider_registry: Arc<ProviderRegistry>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("Activation failed: {0}")]
    ActivationFailed(String),
    #[error("Deactivation failed: {0}")]
    DeactivationFailed(String),
    #[error("Dependency missing: {0}")]
    DependencyMissing(String),
}

#[async_trait]
pub trait Extension: Send + Sync + 'static {
    fn manifest(&self) -> &ExtensionManifest;
    async fn activate(&mut self, ctx: &mut ActivationContext) -> Result<(), ExtensionError>;
    async fn deactivate(&mut self) -> Result<(), ExtensionError>;
}
