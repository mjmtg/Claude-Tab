use async_trait::async_trait;
use claude_tabs_core::traits::detector::{DetectionResult, DetectorInput, StateDetector};
use claude_tabs_core::traits::extension::{
    ActivationContext, Extension, ExtensionError, ExtensionManifest,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternRule {
    pub pattern: String,
    pub target_state: String,
    pub confidence: f32,
}

pub struct OutputParserExtension {
    manifest: ExtensionManifest,
}

impl OutputParserExtension {
    pub fn new() -> Self {
        Self {
            manifest: ExtensionManifest::new("output-parser", "Output Parser")
                .with_description("Regex-based terminal output state detector"),
        }
    }
}

#[async_trait]
impl Extension for OutputParserExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn activate(&mut self, ctx: &mut ActivationContext) -> Result<(), ExtensionError> {
        let rules = vec![
            PatternRule {
                pattern: r"(?i)do you want to proceed|allow|deny|yes/no".to_string(),
                target_state: "your_turn".to_string(),
                confidence: 0.7,
            },
            PatternRule {
                pattern: r"(?i)\[waiting for input\]|permission required".to_string(),
                target_state: "your_turn".to_string(),
                confidence: 0.8,
            },
        ];

        let detector = OutputDetector::new(rules);
        ctx.detector_registry.register(Box::new(detector)).await;

        Ok(())
    }

    async fn deactivate(&mut self) -> Result<(), ExtensionError> {
        Ok(())
    }
}

struct OutputDetector {
    rules: Vec<(Regex, String, f32)>,
    buffer: String,
}

impl OutputDetector {
    fn new(rules: Vec<PatternRule>) -> Self {
        let compiled: Vec<(Regex, String, f32)> = rules
            .into_iter()
            .filter_map(|r| {
                Regex::new(&r.pattern)
                    .ok()
                    .map(|re| (re, r.target_state, r.confidence))
            })
            .collect();

        Self {
            rules: compiled,
            buffer: String::new(),
        }
    }
}

#[async_trait]
impl StateDetector for OutputDetector {
    fn id(&self) -> &str {
        "output-parser-detector"
    }

    fn input_type(&self) -> DetectorInput {
        DetectorInput::PtyOutput
    }

    fn priority(&self) -> u32 {
        50
    }

    async fn on_pty_output(&mut self, session_id: &str, data: &[u8]) -> Option<DetectionResult> {
        let text = String::from_utf8_lossy(data);
        self.buffer.push_str(&text);

        if self.buffer.len() > 8192 {
            self.buffer = self.buffer[self.buffer.len() - 4096..].to_string();
        }

        let mut best: Option<DetectionResult> = None;

        for (regex, target_state, confidence) in &self.rules {
            if regex.is_match(&self.buffer) {
                debug!(
                    session_id = %session_id,
                    target = %target_state,
                    "Output pattern matched"
                );
                if best.as_ref().map_or(true, |b| *confidence > b.confidence) {
                    best = Some(DetectionResult::new(session_id, target_state.clone(), *confidence));
                }
            }
        }

        if best.is_some() {
            self.buffer.clear();
        }

        best
    }

    async fn on_hook_payload(
        &mut self,
        _session_id: &str,
        _payload: &serde_json::Value,
    ) -> Option<DetectionResult> {
        None
    }
}
