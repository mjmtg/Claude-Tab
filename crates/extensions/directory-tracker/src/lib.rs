//! Directory Tracker Extension
//!
//! Parses OSC 7 escape sequences from PTY output to track terminal
//! working directories. Updates session title and working_directory
//! for terminal sessions.
//!
//! OSC 7 format: \x1b]7;file://hostname/path\x07

use claude_tabs_core::event_bus::EventBus;
use claude_tabs_core::session::SessionStore;
use claude_tabs_core::Event;
use claude_tabs_pty::OutputStream;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Start the directory tracker
pub fn start(
    event_bus: Arc<EventBus>,
    session_store: Arc<SessionStore>,
    output_stream: Arc<OutputStream>,
) {
    let mut receiver = output_stream.subscribe();
    let last_dirs: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    // OSC 7 pattern: \x1b]7;file://hostname/path\x07 or \x1b]7;file://hostname/path\x1b\\
    let osc7_pattern =
        Regex::new(r"\x1b\]7;file://[^/]*(/.+?)(?:\x07|\x1b\\)").expect("valid regex");

    tokio::spawn(async move {
        info!("Directory tracker started");

        loop {
            match receiver.recv().await {
                Ok(chunk) => {
                    // Convert Arc<str> to String for HashMap key operations
                    let session_id: String = chunk.session_id.to_string();

                    if chunk.data.is_empty() {
                        // Session closed, clean up
                        last_dirs.lock().await.remove(&session_id);
                        continue;
                    }

                    // Only process terminal sessions
                    let session = match session_store.get(&session_id).await {
                        Some(s) if s.provider_id == "terminal" => s,
                        _ => continue,
                    };

                    // Parse output for OSC 7 sequences
                    let text = String::from_utf8_lossy(&chunk.data);
                    if let Some(captures) = osc7_pattern.captures(&text) {
                        if let Some(path_match) = captures.get(1) {
                            let path = path_match.as_str().to_string();

                            // URL decode the path (handles spaces as %20, etc.)
                            let decoded_path = url_decode(&path);

                            // Check if directory changed
                            let mut dirs = last_dirs.lock().await;
                            let changed = dirs.get(&session_id) != Some(&decoded_path);

                            if changed {
                                dirs.insert(session_id.clone(), decoded_path.clone());
                                drop(dirs);

                                // Update session working directory
                                session_store
                                    .update_working_directory(&session_id, &decoded_path)
                                    .await;

                                // Update session title to directory name (for terminals)
                                let dir_name = decoded_path
                                    .rsplit('/')
                                    .find(|s| !s.is_empty())
                                    .unwrap_or(&decoded_path);

                                // Only update title if it's still the default or was auto-set
                                let should_update_title = session.title.is_empty()
                                    || session.title == "Terminal"
                                    || session
                                        .metadata
                                        .get("dir_title")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false);

                                if should_update_title {
                                    session_store.rename(&session_id, dir_name).await;
                                    session_store
                                        .set_metadata(
                                            &session_id,
                                            "dir_title",
                                            serde_json::Value::Bool(true),
                                        )
                                        .await;
                                }

                                // Emit session.renamed event for UI refresh
                                let event = Event::new(
                                    "session.renamed",
                                    serde_json::json!({
                                        "session_id": session_id,
                                        "title": dir_name,
                                        "working_directory": decoded_path,
                                    }),
                                );
                                event_bus.emit(event).await;

                                debug!(
                                    session_id = %session_id,
                                    directory = %decoded_path,
                                    "Updated session directory"
                                );
                            }
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    debug!(lagged = n, "Directory tracker lagged");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Simple URL decoding for common escape sequences
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            // Try to read two hex digits
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            // If parsing failed, keep the original
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("/Users/test/My%20Documents"), "/Users/test/My Documents");
        assert_eq!(url_decode("/simple/path"), "/simple/path");
        assert_eq!(url_decode("/path%2Fwith%2Fslashes"), "/path/with/slashes");
    }
}
