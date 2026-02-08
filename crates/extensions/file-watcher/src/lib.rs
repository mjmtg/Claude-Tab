use async_trait::async_trait;
use claude_tabs_core::traits::extension::{
    ActivationContext, Extension, ExtensionError, ExtensionManifest,
};

pub struct FileWatcherExtension {
    manifest: ExtensionManifest,
}

impl FileWatcherExtension {
    pub fn new() -> Self {
        Self {
            manifest: ExtensionManifest::new("file-watcher", "File Watcher")
                .with_description("File-based state detection (future)"),
        }
    }
}

#[async_trait]
impl Extension for FileWatcherExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn activate(&mut self, _ctx: &mut ActivationContext) -> Result<(), ExtensionError> {
        // Future: Watch for file changes and trigger state detection
        Ok(())
    }

    async fn deactivate(&mut self) -> Result<(), ExtensionError> {
        Ok(())
    }
}
