use async_trait::async_trait;
use claude_tabs_core::traits::extension::{
    ActivationContext, Extension, ExtensionError, ExtensionManifest,
};

pub struct ClaudeHooksExtension {
    manifest: ExtensionManifest,
}

impl ClaudeHooksExtension {
    pub fn new() -> Self {
        Self {
            manifest: ExtensionManifest::new("claude-hooks", "Claude Hooks")
                .with_description("Hook event integration — state transitions handled by HookListener in core"),
        }
    }
}

#[async_trait]
impl Extension for ClaudeHooksExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn activate(&mut self, _ctx: &mut ActivationContext) -> Result<(), ExtensionError> {
        // All state transitions are handled directly by HookListener in core.
        // This extension exists as a registration point and can be extended
        // to add hook-specific reactions in the future.
        Ok(())
    }

    async fn deactivate(&mut self) -> Result<(), ExtensionError> {
        Ok(())
    }
}
