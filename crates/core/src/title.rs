use claude_tabs_storage::{ClaudeSession, SessionMessage, SessionMetadata};

/// Resolve the display title for a session.
///
/// Priority chain (highest wins):
/// 1. `custom_title` if `user_set_title == true`
/// 2. `generated_title` (Haiku-generated)
/// 3. `summary` from Claude's sessions-index.json
/// 4. `first_prompt` truncated to 80 chars
/// 5. Fallback: last segment of project_path (folder name)
pub fn resolve_title(
    metadata: Option<&SessionMetadata>,
    claude_session: Option<&ClaudeSession>,
    claude_session_id: &str,
) -> String {
    // 1. User-set custom title
    if let Some(meta) = metadata {
        if meta.user_set_title {
            if let Some(ref title) = meta.custom_title {
                if !title.is_empty() {
                    return title.clone();
                }
            }
        }
    }

    // 2. Haiku-generated title
    if let Some(meta) = metadata {
        if let Some(ref generated) = meta.generated_title {
            if !generated.is_empty() {
                return generated.clone();
            }
        }
    }

    // 3. Summary from sessions-index.json
    if let Some(session) = claude_session {
        if let Some(ref summary) = session.summary {
            if !summary.is_empty() {
                return summary.clone();
            }
        }
    }

    // 4. First prompt truncated
    if let Some(session) = claude_session {
        if let Some(ref prompt) = session.first_prompt {
            if !prompt.is_empty() {
                return truncate_title(prompt, 80);
            }
        }
    }

    // 5. Fallback: folder name from project_path
    if let Some(session) = claude_session {
        let path = &session.project_path;
        if !path.is_empty() {
            if let Some(folder) = path.rsplit('/').next() {
                if !folder.is_empty() {
                    return folder.to_string();
                }
            }
        }
    }

    // Also try project_path from metadata
    if let Some(meta) = metadata {
        if !meta.project_path.is_empty() {
            if let Some(folder) = meta.project_path.rsplit('/').next() {
                if !folder.is_empty() {
                    return folder.to_string();
                }
            }
        }
    }

    // Ultimate fallback
    let short_id = if claude_session_id.len() >= 8 {
        &claude_session_id[..8]
    } else {
        claude_session_id
    };
    format!("Session {}", short_id)
}

/// Build a prompt for Haiku to generate a short title from conversation messages.
pub fn generate_title_prompt(messages: &[SessionMessage]) -> String {
    let mut conversation = String::new();
    let mut count = 0;

    for msg in messages {
        if count >= 10 {
            break;
        }
        match msg.message_type.as_str() {
            "user" | "assistant" => {
                let role = msg.role().unwrap_or(&msg.message_type);
                if let Some(text) = extract_text_content(msg.content()) {
                    let truncated = if text.len() > 500 {
                        format!("{}...", &text[..497])
                    } else {
                        text
                    };
                    conversation.push_str(&format!("{}: {}\n", role, truncated));
                    count += 1;
                }
            }
            _ => {}
        }
    }

    format!(
        "Given the following conversation, generate a short, descriptive title (max 50 chars). \
         Return it in <title>your title here</title> format.\n\n{}",
        conversation
    )
}

/// Extract a title from Haiku's `<title>...</title>` response.
pub fn parse_title_response(response: &str) -> Option<String> {
    let start = response.find("<title>")?;
    let end = response.find("</title>")?;
    if end <= start + 7 {
        return None;
    }
    let title = response[start + 7..end].trim().to_string();
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

fn truncate_title(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

fn extract_text_content(content: Option<&serde_json::Value>) -> Option<String> {
    match content {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(serde_json::Value::Array(arr)) => {
            let texts: Vec<String> = arr
                .iter()
                .filter_map(|item| {
                    if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                        item.get("text").and_then(|v| v.as_str()).map(String::from)
                    } else {
                        None
                    }
                })
                .collect();
            if texts.is_empty() { None } else { Some(texts.join("\n")) }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_title_wins() {
        let meta = SessionMetadata {
            claude_session_id: "abc".into(),
            project_path: "/tmp".into(),
            custom_title: Some("My Custom Title".into()),
            user_set_title: true,
            generated_title: Some("Generated Title".into()),
            hidden: false,
            previous_session_id: None,
            last_known_state: None,
            last_state_change_at: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let session = ClaudeSession {
            session_id: "abc".into(),
            project_path: "/tmp/project".into(),
            jsonl_path: "/tmp/abc.jsonl".into(),
            first_prompt: Some("Hello".into()),
            summary: Some("Summary".into()),
            message_count: 1,
            created_at: String::new(),
            modified_at: String::new(),
            git_branch: None,
        };
        let title = resolve_title(Some(&meta), Some(&session), "abc");
        assert_eq!(title, "My Custom Title");
    }

    #[test]
    fn test_generated_title_over_summary() {
        let meta = SessionMetadata {
            claude_session_id: "abc".into(),
            project_path: "/tmp".into(),
            custom_title: None,
            user_set_title: false,
            generated_title: Some("Generated Title".into()),
            hidden: false,
            previous_session_id: None,
            last_known_state: None,
            last_state_change_at: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let session = ClaudeSession {
            session_id: "abc".into(),
            project_path: "/tmp/project".into(),
            jsonl_path: "/tmp/abc.jsonl".into(),
            first_prompt: Some("Hello".into()),
            summary: Some("Summary".into()),
            message_count: 1,
            created_at: String::new(),
            modified_at: String::new(),
            git_branch: None,
        };
        let title = resolve_title(Some(&meta), Some(&session), "abc");
        assert_eq!(title, "Generated Title");
    }

    #[test]
    fn test_summary_over_prompt() {
        let session = ClaudeSession {
            session_id: "abc".into(),
            project_path: "/tmp/project".into(),
            jsonl_path: "/tmp/abc.jsonl".into(),
            first_prompt: Some("Hello".into()),
            summary: Some("Summary Title".into()),
            message_count: 1,
            created_at: String::new(),
            modified_at: String::new(),
            git_branch: None,
        };
        let title = resolve_title(None, Some(&session), "abc");
        assert_eq!(title, "Summary Title");
    }

    #[test]
    fn test_folder_name_fallback() {
        let session = ClaudeSession {
            session_id: "abc".into(),
            project_path: "/Users/name/my-project".into(),
            jsonl_path: "/tmp/abc.jsonl".into(),
            first_prompt: None,
            summary: None,
            message_count: 0,
            created_at: String::new(),
            modified_at: String::new(),
            git_branch: None,
        };
        let title = resolve_title(None, Some(&session), "abc");
        assert_eq!(title, "my-project");
    }

    #[test]
    fn test_ultimate_fallback() {
        let title = resolve_title(None, None, "abcdef12-3456");
        assert_eq!(title, "Session abcdef12");
    }

    #[test]
    fn test_truncation() {
        let long = "a".repeat(100);
        let result = truncate_title(&long, 80);
        assert_eq!(result.len(), 80);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_parse_title_response() {
        assert_eq!(
            parse_title_response("Here is the title: <title>Fix auth bug</title>"),
            Some("Fix auth bug".to_string())
        );
        assert_eq!(parse_title_response("No tags here"), None);
        assert_eq!(parse_title_response("<title></title>"), None);
    }

    #[test]
    fn test_generate_title_prompt_format() {
        let prompt = generate_title_prompt(&[]);
        assert!(prompt.contains("<title>"));
        assert!(prompt.contains("max 50 chars"));
    }
}
