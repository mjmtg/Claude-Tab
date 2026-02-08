use async_trait::async_trait;
use claude_tabs_core::event_bus::Event;
use claude_tabs_core::traits::detector::{DetectionResult, DetectorInput, StateDetector};
use claude_tabs_core::traits::extension::{
    ActivationContext, Extension, ExtensionError, ExtensionManifest,
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;
use tokio::net::UnixListener;
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HookPayload {
    session_id: String,
    tool_name: String,
    event: String,
}

pub struct ClaudeHooksExtension {
    manifest: ExtensionManifest,
    socket_path: Option<String>,
}

impl ClaudeHooksExtension {
    pub fn new() -> Self {
        Self {
            manifest: ExtensionManifest::new("claude-hooks", "Claude Hooks")
                .with_description("PreToolUse hook detector via Unix socket"),
            socket_path: None,
        }
    }
}

#[async_trait]
impl Extension for ClaudeHooksExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn activate(&mut self, ctx: &mut ActivationContext) -> Result<(), ExtensionError> {
        let socket_path = format!("/tmp/claude-tabs-{}.sock", std::process::id());
        self.socket_path = Some(socket_path.clone());

        let event_bus = ctx.event_bus.clone();
        let session_store = ctx.session_store.clone();

        tokio::spawn(async move {
            let _ = tokio::fs::remove_file(&socket_path).await;

            let listener = match UnixListener::bind(&socket_path) {
                Ok(l) => l,
                Err(e) => {
                    error!(error = %e, "Failed to bind Unix socket for hooks");
                    return;
                }
            };

            info!(path = %socket_path, "Hook socket listening");

            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let eb = event_bus.clone();
                        let ss = session_store.clone();
                        tokio::spawn(async move {
                            let reader = tokio::io::BufReader::new(stream);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                if let Ok(payload) = serde_json::from_str::<HookPayload>(&line) {
                                    info!(
                                        session_id = %payload.session_id,
                                        tool = %payload.tool_name,
                                        "Hook payload received"
                                    );

                                    ss.update_state(
                                        &payload.session_id,
                                        "core.permission_needed",
                                    )
                                    .await;

                                    let event = Event::new(
                                        "session.state_changed",
                                        serde_json::json!({
                                            "session_id": payload.session_id,
                                            "from": "core.active",
                                            "to": "core.permission_needed",
                                            "tool_name": payload.tool_name,
                                        }),
                                    )
                                    .with_session(payload.session_id);

                                    eb.emit(event).await;
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to accept hook connection");
                    }
                }
            }
        });

        let detector = HookDetector::new();
        ctx.detector_registry
            .register(Box::new(detector))
            .await;

        Ok(())
    }

    async fn deactivate(&mut self) -> Result<(), ExtensionError> {
        if let Some(path) = &self.socket_path {
            let _ = tokio::fs::remove_file(path).await;
        }
        Ok(())
    }
}

struct HookDetector;

impl HookDetector {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl StateDetector for HookDetector {
    fn id(&self) -> &str {
        "claude-hooks-detector"
    }

    fn input_type(&self) -> DetectorInput {
        DetectorInput::HookPayload
    }

    fn priority(&self) -> u32 {
        100
    }

    async fn on_pty_output(&mut self, _session_id: &str, _data: &[u8]) -> Option<DetectionResult> {
        None
    }

    async fn on_hook_payload(
        &mut self,
        session_id: &str,
        payload: &serde_json::Value,
    ) -> Option<DetectionResult> {
        if payload.get("event").and_then(|v| v.as_str()) == Some("permission_needed") {
            Some(DetectionResult::new(
                session_id,
                "core.permission_needed",
                1.0,
            ))
        } else {
            None
        }
    }
}
