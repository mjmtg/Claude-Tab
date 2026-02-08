//! Session Reader
//!
//! Reads full conversation content from Claude Code JSONL session files.

use crate::models::SessionMessage;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tracing::warn;

/// Reader for Claude Code session JSONL files.
pub struct SessionReader;

impl SessionReader {
    /// Read all messages from a JSONL session file.
    pub fn read_session(jsonl_path: &Path) -> Result<Vec<SessionMessage>, std::io::Error> {
        let file = File::open(jsonl_path)?;
        let reader = BufReader::new(file);
        let mut messages = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<SessionMessage>(&line) {
                Ok(msg) => messages.push(msg),
                Err(e) => {
                    warn!("Failed to parse JSONL line: {}", e);
                }
            }
        }

        Ok(messages)
    }

    /// Stream messages from a JSONL file (lazy iterator).
    pub fn stream_session(jsonl_path: &Path) -> Result<impl Iterator<Item = SessionMessage>, std::io::Error> {
        let file = File::open(jsonl_path)?;
        let reader = BufReader::new(file);

        Ok(reader.lines().filter_map(|line| {
            let line = line.ok()?;
            if line.trim().is_empty() {
                return None;
            }
            serde_json::from_str::<SessionMessage>(&line).ok()
        }))
    }

    /// Extract human-readable conversation from messages.
    pub fn format_conversation(messages: &[SessionMessage]) -> String {
        let mut output = String::new();

        for msg in messages {
            match msg.message_type.as_str() {
                "user" | "assistant" => {
                    let role = msg.role.as_deref().unwrap_or(&msg.message_type);
                    if let Some(text) = Self::extract_text(&msg.content) {
                        output.push_str(&format!("**{}**: {}\n\n", role, text));
                    }
                }
                "summary" => {
                    if let Some(summary) = &msg.summary {
                        output.push_str(&format!("---\n**Summary**: {}\n---\n\n", summary));
                    }
                }
                _ => {}
            }
        }

        output
    }

    /// Extract plain text from message content.
    fn extract_text(content: &Option<serde_json::Value>) -> Option<String> {
        match content {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Array(arr)) => {
                let mut texts = Vec::new();
                for item in arr {
                    if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                        if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                            texts.push(text.to_string());
                        }
                    }
                    // Handle tool_use and tool_result blocks
                    if let Some(tool_type) = item.get("type").and_then(|v| v.as_str()) {
                        match tool_type {
                            "tool_use" => {
                                if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                                    texts.push(format!("[Tool: {}]", name));
                                }
                            }
                            "tool_result" => {
                                texts.push("[Tool Result]".to_string());
                            }
                            _ => {}
                        }
                    }
                }
                if texts.is_empty() {
                    None
                } else {
                    Some(texts.join("\n"))
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_string() {
        let content = Some(serde_json::Value::String("Hello world".to_string()));
        assert_eq!(SessionReader::extract_text(&content), Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_text_array() {
        let content = Some(serde_json::json!([
            {"type": "text", "text": "Hello"},
            {"type": "text", "text": "World"}
        ]));
        assert_eq!(SessionReader::extract_text(&content), Some("Hello\nWorld".to_string()));
    }

    #[test]
    fn test_extract_text_none() {
        assert_eq!(SessionReader::extract_text(&None), None);
    }
}
