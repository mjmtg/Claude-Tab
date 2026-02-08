use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectorInput {
    PtyOutput,
    FileWatch,
    Socket,
    HookPayload,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub session_id: String,
    pub new_state: String,
    pub confidence: f32,
    pub detector_id: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DetectionResult {
    pub fn new(session_id: impl Into<String>, new_state: impl Into<String>, confidence: f32) -> Self {
        Self {
            session_id: session_id.into(),
            new_state: new_state.into(),
            confidence,
            detector_id: String::new(),
            metadata: HashMap::new(),
        }
    }
}

#[async_trait]
pub trait StateDetector: Send + Sync + 'static {
    fn id(&self) -> &str;
    fn input_type(&self) -> DetectorInput;
    fn priority(&self) -> u32;
    async fn on_pty_output(&mut self, session_id: &str, data: &[u8]) -> Option<DetectionResult>;
    async fn on_hook_payload(
        &mut self,
        session_id: &str,
        payload: &serde_json::Value,
    ) -> Option<DetectionResult>;
}

pub struct DetectorRegistry {
    detectors: Arc<RwLock<Vec<Box<dyn StateDetector>>>>,
}

impl DetectorRegistry {
    pub fn new() -> Self {
        Self {
            detectors: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register(&self, detector: Box<dyn StateDetector>) {
        let mut detectors = self.detectors.write().await;
        detectors.push(detector);
        detectors.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    pub async fn process_pty_output(
        &self,
        session_id: &str,
        data: &[u8],
    ) -> Option<DetectionResult> {
        let mut detectors = self.detectors.write().await;
        let mut best: Option<DetectionResult> = None;

        for detector in detectors.iter_mut() {
            if matches!(detector.input_type(), DetectorInput::PtyOutput) {
                if let Some(result) = detector.on_pty_output(session_id, data).await {
                    if best.as_ref().map_or(true, |b| result.confidence > b.confidence) {
                        best = Some(result);
                    }
                }
            }
        }

        best
    }

    pub async fn process_hook_payload(
        &self,
        session_id: &str,
        payload: &serde_json::Value,
    ) -> Option<DetectionResult> {
        let mut detectors = self.detectors.write().await;
        let mut best: Option<DetectionResult> = None;

        for detector in detectors.iter_mut() {
            if matches!(detector.input_type(), DetectorInput::HookPayload) {
                if let Some(result) = detector.on_hook_payload(session_id, payload).await {
                    if best.as_ref().map_or(true, |b| result.confidence > b.confidence) {
                        best = Some(result);
                    }
                }
            }
        }

        best
    }
}

impl Default for DetectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
