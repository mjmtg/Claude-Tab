use crate::config::Config;
use crate::event_bus::EventBus;
use crate::session::SessionStore;
use crate::traits::detector::DetectorRegistry;
use crate::traits::extension::{ActivationContext, Extension, ExtensionError, ExtensionManifest};
use crate::traits::provider::ProviderRegistry;
use crate::traits::reaction::ReactionRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

pub struct PluginHost {
    extensions: Vec<Box<dyn Extension>>,
    activated: Vec<String>,
    event_bus: Arc<EventBus>,
    config: Arc<Config>,
    detector_registry: Arc<DetectorRegistry>,
    reaction_registry: Arc<ReactionRegistry>,
    session_store: Arc<SessionStore>,
    provider_registry: Arc<ProviderRegistry>,
}

impl PluginHost {
    pub fn new(
        event_bus: Arc<EventBus>,
        config: Arc<Config>,
        detector_registry: Arc<DetectorRegistry>,
        reaction_registry: Arc<ReactionRegistry>,
        session_store: Arc<SessionStore>,
        provider_registry: Arc<ProviderRegistry>,
    ) -> Self {
        Self {
            extensions: Vec::new(),
            activated: Vec::new(),
            event_bus,
            config,
            detector_registry,
            reaction_registry,
            session_store,
            provider_registry,
        }
    }

    pub fn register(&mut self, extension: Box<dyn Extension>) {
        let manifest = extension.manifest().clone();
        info!(
            extension_id = %manifest.id,
            extension_name = %manifest.name,
            "Registered extension"
        );
        self.extensions.push(extension);
    }

    pub async fn activate_all(&mut self) -> Result<(), ExtensionError> {
        let order = self.resolve_activation_order()?;

        for idx in order {
            let ext = &mut self.extensions[idx];
            let manifest = ext.manifest().clone();

            info!(extension_id = %manifest.id, "Activating extension");

            let mut ctx = ActivationContext {
                event_bus: self.event_bus.clone(),
                config: self.config.clone(),
                detector_registry: self.detector_registry.clone(),
                reaction_registry: self.reaction_registry.clone(),
                session_store: self.session_store.clone(),
                provider_registry: self.provider_registry.clone(),
            };

            match ext.activate(&mut ctx).await {
                Ok(()) => {
                    self.activated.push(manifest.id.clone());
                    info!(extension_id = %manifest.id, "Extension activated");
                    let event = crate::event_bus::Event::new(
                        "extension.activated",
                        serde_json::json!({ "id": manifest.id }),
                    );
                    self.event_bus.emit(event).await;
                }
                Err(e) => {
                    error!(extension_id = %manifest.id, error = %e, "Extension activation failed");
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    pub async fn deactivate_all(&mut self) {
        for ext in self.extensions.iter_mut().rev() {
            let manifest = ext.manifest().clone();
            if self.activated.contains(&manifest.id) {
                info!(extension_id = %manifest.id, "Deactivating extension");
                if let Err(e) = ext.deactivate().await {
                    error!(extension_id = %manifest.id, error = %e, "Extension deactivation failed");
                }
            }
        }
        self.activated.clear();
    }

    fn resolve_activation_order(&self) -> Result<Vec<usize>, ExtensionError> {
        let manifests: Vec<&ExtensionManifest> =
            self.extensions.iter().map(|e| e.manifest()).collect();

        let id_to_idx: HashMap<&str, usize> = manifests
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id.as_str(), i))
            .collect();

        let mut order: Vec<usize> = Vec::new();
        let mut visited: Vec<bool> = vec![false; manifests.len()];
        let mut in_stack: Vec<bool> = vec![false; manifests.len()];

        for i in 0..manifests.len() {
            if !visited[i] {
                self.topo_sort(i, &manifests, &id_to_idx, &mut visited, &mut in_stack, &mut order)?;
            }
        }

        Ok(order)
    }

    fn topo_sort(
        &self,
        idx: usize,
        manifests: &[&ExtensionManifest],
        id_to_idx: &HashMap<&str, usize>,
        visited: &mut Vec<bool>,
        in_stack: &mut Vec<bool>,
        order: &mut Vec<usize>,
    ) -> Result<(), ExtensionError> {
        visited[idx] = true;
        in_stack[idx] = true;

        for dep in &manifests[idx].dependencies {
            if let Some(&dep_idx) = id_to_idx.get(dep.as_str()) {
                if in_stack[dep_idx] {
                    return Err(ExtensionError::ActivationFailed(format!(
                        "Circular dependency involving {}",
                        dep
                    )));
                }
                if !visited[dep_idx] {
                    self.topo_sort(dep_idx, manifests, id_to_idx, visited, in_stack, order)?;
                }
            }
        }

        in_stack[idx] = false;
        order.push(idx);
        Ok(())
    }

    pub fn activated_extensions(&self) -> &[String] {
        &self.activated
    }
}
