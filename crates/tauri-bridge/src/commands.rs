use crate::ipc::AppState;
use claude_tabs_core::hook_listener::HookListener;
use claude_tabs_core::profile::{self, Profile, ProfileLaunchRequest};
use claude_tabs_core::session::Session;
use claude_tabs_core::traits::provider::PtySize;
use claude_tabs_platform_focus::{self as platform_focus, AttentionType};
use claude_tabs_storage::{ClaudeSession, DirectoryPreference, SessionFilter, SessionMessage};
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub provider_id: String,
    pub state: String,
    pub title: String,
    pub working_directory: Option<String>,
    pub subtitle: Option<String>,
    pub tags: Option<Vec<String>>,
    pub summary: Option<String>,
}

impl From<&Session> for SessionInfo {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.clone(),
            provider_id: session.provider_id.clone(),
            state: session.state.clone(),
            title: session.title.clone(),
            working_directory: session.working_directory.clone(),
            subtitle: session
                .metadata
                .get("subtitle")
                .and_then(|v| v.as_str())
                .map(String::from),
            tags: session.metadata.get("tags").and_then(|v| {
                v.as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            }),
            summary: session
                .metadata
                .get("summary")
                .and_then(|v| v.as_str())
                .map(String::from),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub provider_id: String,
    pub working_directory: Option<String>,
    pub title: Option<String>,
    pub resume_claude_session_id: Option<String>,
    pub fork: Option<bool>,
    pub initial_prompt: Option<String>,
    pub mcp_config_path: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
}

#[tauri::command]
pub async fn create_session(
    state: State<'_, AppState>,
    request: CreateSessionRequest,
) -> Result<SessionInfo, String> {
    info!(provider_id = %request.provider_id, "Creating session");

    let mut session = Session::new(&request.provider_id);
    if let Some(title) = &request.title {
        session.title = title.clone();
    }
    if let Some(dir) = &request.working_directory {
        session.working_directory = Some(dir.clone());
    }

    let session_id = session.id.clone();

    let mut env = std::collections::HashMap::new();
    env.insert("TERM".to_string(), "xterm-256color".to_string());
    env.insert("CLAUDE_TABS_SESSION_ID".to_string(), session_id.clone());
    env.insert(
        "CLAUDE_TABS_SOCKET".to_string(),
        HookListener::socket_path().to_string_lossy().to_string(),
    );

    // For bash sessions, set BASH_ENV to source shell integration
    if request.provider_id != "claude-code" {
        env.insert(
            "BASH_ENV".to_string(),
            HookListener::shell_integration_path("bash")
                .to_string_lossy()
                .to_string(),
        );
    }

    let size = PtySize { rows: 24, cols: 80 };

    let command = if request.provider_id == "claude-code" {
        "claude"
    } else {
        "bash"
    };

    let args: Vec<String> = if request.provider_id == "claude-code" {
        let mut a = Vec::new();
        if let Some(ref id) = request.resume_claude_session_id {
            a.push("--resume".to_string());
            a.push(id.clone());
            if request.fork.unwrap_or(false) {
                a.push("--fork-session".to_string());
            }
        }
        if let Some(ref mcp_path) = request.mcp_config_path {
            a.push("--mcp-config".to_string());
            a.push(mcp_path.clone());
        }
        if let Some(ref tools) = request.allowed_tools {
            if !tools.is_empty() {
                a.push("--allowedTools".to_string());
                a.push(tools.join(","));
            }
        }
        if let Some(ref model) = request.model {
            a.push("--model".to_string());
            a.push(model.clone());
        }
        if let Some(ref sys_prompt) = request.system_prompt {
            a.push("--append-system-prompt".to_string());
            a.push(sys_prompt.clone());
        }
        // Add prompt as positional arg (interactive mode) instead of -p (print mode)
        if request.resume_claude_session_id.is_none() {
            if let Some(ref prompt) = request.initial_prompt {
                a.push(prompt.clone());
            }
        }
        a
    } else {
        Vec::new()
    };

    let reader = state
        .pty_manager
        .spawn(
            &session_id,
            command,
            &args,
            request.working_directory.as_deref(),
            &env,
            size,
        )
        .map_err(|e| e.to_string())?;

    state.output_stream.start_reading(session_id.clone(), reader);
    state.session_store.add(session.clone()).await;
    state.session_store.set_active(Some(session_id.clone())).await;

    let event = claude_tabs_core::Event::new(
        "session.created",
        serde_json::json!({
            "session_id": session_id,
            "provider_id": request.provider_id,
        }),
    );
    state.event_bus.emit(event).await;

    Ok(SessionInfo::from(&session))
}

#[tauri::command]
pub async fn close_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    info!(session_id = %session_id, "Closing session");

    // Cleanup temp MCP config file if this session had one
    profile::cleanup_temp_mcp_config(&session_id);

    // Remove from store
    state.session_store.remove(&session_id).await;

    state.pty_manager.close(&session_id).map_err(|e| e.to_string())?;

    let sessions = state.session_store.list().await;
    let new_active = sessions.first().map(|s| s.id.clone());
    state.session_store.set_active(new_active).await;

    let event = claude_tabs_core::Event::new(
        "session.closed",
        serde_json::json!({ "session_id": session_id }),
    );
    state.event_bus.emit(event).await;

    Ok(())
}

#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionInfo>, String> {
    let sessions = state.session_store.list().await;
    Ok(sessions.iter().map(SessionInfo::from).collect())
}

#[tauri::command]
pub async fn get_active_session(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.session_store.get_active().await)
}

#[tauri::command]
pub async fn set_active_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    debug!(session_id = %session_id, "Setting active session");
    state.session_store.set_active(Some(session_id.clone())).await;

    // Emit active_changed event for UI sync (sidebar, declined sessions, etc.)
    let active_event = claude_tabs_core::Event::new(
        "session.active_changed",
        serde_json::json!({ "session_id": session_id }),
    );
    state.event_bus.emit(active_event).await;

    // If session is idle, transition to active on focus
    if let Some(session) = state.session_store.get(&session_id).await {
        if session.state == "idle" {
            state.session_store.update_state(&session_id, "active").await;
            let event = claude_tabs_core::Event::new(
                "session.state_changed",
                serde_json::json!({
                    "session_id": session_id,
                    "from": "idle",
                    "to": "active",
                }),
            );
            state.event_bus.emit(event).await;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn rename_session(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<(), String> {
    info!(session_id = %session_id, title = %title, "Renaming session");
    let success = state.session_store.rename(&session_id, &title).await;
    if !success {
        return Err("Session not found".to_string());
    }
    let event = claude_tabs_core::Event::new(
        "session.renamed",
        serde_json::json!({ "session_id": session_id, "title": title }),
    );
    state.event_bus.emit(event).await;
    Ok(())
}

#[tauri::command]
pub fn write_to_pty(
    state: State<'_, AppState>,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    state
        .pty_manager
        .write_data(&session_id, &data)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resize_pty(
    state: State<'_, AppState>,
    session_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let size = PtySize { rows, cols };
    state
        .pty_manager
        .resize(&session_id, size)
        .map_err(|e| e.to_string())
}

/// Submit user input to the PTY with a trailing newline.
/// This is used by the hybrid terminal's input area.
#[tauri::command]
pub fn submit_input(
    state: State<'_, AppState>,
    session_id: String,
    input: String,
) -> Result<(), String> {
    let data = format!("{}\n", input);
    state
        .pty_manager
        .write_data(&session_id, data.as_bytes())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_config_value(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<serde_json::Value>, String> {
    Ok(state.config.get(&key).await)
}

#[tauri::command]
pub async fn set_config_value(
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    state
        .config
        .set_value(&key, value, claude_tabs_core::ConfigLayer::Runtime)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn setup_hooks() -> Result<String, String> {
    HookListener::install_hooks();
    Ok("Hooks installed".to_string())
}

// ============================================================================
// Claude Code Session History Commands
// ============================================================================

#[tauri::command]
pub async fn list_claude_sessions(
    state: State<'_, AppState>,
    filter: Option<SessionFilter>,
) -> Result<Vec<ClaudeSession>, String> {
    let filter = filter.unwrap_or(SessionFilter {
        limit: 100,
        ..Default::default()
    });
    state
        .storage
        .list_sessions(filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_claude_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<ClaudeSession>, String> {
    state
        .storage
        .get_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_session_content(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<SessionMessage>, String> {
    state
        .storage
        .get_session_content(&session_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_directory_preference(
    state: State<'_, AppState>,
    project_path: String,
    pinned: Option<bool>,
    hidden: Option<bool>,
    display_name: Option<String>,
) -> Result<(), String> {
    // Get existing preference or create new one
    let prefs = state.storage.get_directory_preferences().await.map_err(|e| e.to_string())?;
    let existing = prefs.iter().find(|p| p.project_path == project_path);

    let pref = DirectoryPreference {
        project_path: project_path.clone(),
        pinned: pinned.unwrap_or_else(|| existing.map(|e| e.pinned).unwrap_or(false)),
        hidden: hidden.unwrap_or_else(|| existing.map(|e| e.hidden).unwrap_or(false)),
        display_name: display_name.or_else(|| existing.and_then(|e| e.display_name.clone())),
    };

    state
        .storage
        .set_directory_preference(pref)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_directory_preferences(
    state: State<'_, AppState>,
) -> Result<Vec<DirectoryPreference>, String> {
    state
        .storage
        .get_directory_preferences()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_directory_preference(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<(), String> {
    state
        .storage
        .remove_directory_preference(&project_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_history_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    info!(session_id = %session_id, "Deleting history session");

    // Get session to find the jsonl path
    let session = state
        .storage
        .get_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // Delete the file if it exists
    if let Some(s) = session {
        let path = std::path::Path::new(&s.jsonl_path);
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| e.to_string())?;
            info!(path = %s.jsonl_path, "Deleted session file");
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_project_sessions(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<u32, String> {
    info!(project_path = %project_path, "Deleting all sessions for project");

    // Get all sessions for this project
    let filter = claude_tabs_storage::SessionFilter {
        project_path: Some(project_path.clone()),
        limit: 10000,
        ..Default::default()
    };
    let sessions = state
        .storage
        .list_sessions(filter)
        .await
        .map_err(|e| e.to_string())?;

    let mut deleted_count = 0u32;
    for session in &sessions {
        // Delete file
        let path = std::path::Path::new(&session.jsonl_path);
        if path.exists() {
            if std::fs::remove_file(path).is_ok() {
                deleted_count += 1;
            } else {
                debug!(path = %session.jsonl_path, "Failed to delete session file");
            }
        }
    }

    info!(project_path = %project_path, deleted_count = deleted_count, "Deleted project sessions");
    Ok(deleted_count)
}

#[tauri::command]
pub async fn resume_session(
    state: State<'_, AppState>,
    claude_session_id: String,
) -> Result<SessionInfo, String> {
    // Look up the session to get project path
    let session = state
        .storage
        .get_session(&claude_session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Session not found".to_string())?;

    // Use summary (Claude's auto-title) or first_prompt for display
    let title = session
        .summary
        .clone()
        .or_else(|| session.first_prompt.clone())
        .or_else(|| Some(format!("Session {}", &claude_session_id[..8])));

    let request = CreateSessionRequest {
        provider_id: "claude-code".to_string(),
        working_directory: Some(session.project_path.clone()),
        title,
        resume_claude_session_id: Some(claude_session_id.clone()),
        fork: None,
        initial_prompt: None,
        mcp_config_path: None,
        allowed_tools: None,
        model: None,
        system_prompt: None,
    };

    let result = create_session(state.clone(), request).await?;

    // Store the claude_session_id so the session can be resumed again
    state
        .session_store
        .set_metadata(
            &result.id,
            "claude_session_id",
            serde_json::Value::String(claude_session_id),
        )
        .await;

    Ok(result)
}

#[tauri::command]
pub async fn fork_session(
    state: State<'_, AppState>,
    claude_session_id: String,
) -> Result<SessionInfo, String> {
    let session = state
        .storage
        .get_session(&claude_session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Session not found".to_string())?;

    // Use summary (Claude's auto-title) or first_prompt for display
    let fallback = format!("Session {}", &claude_session_id[..8]);
    let base_title = session
        .summary
        .as_deref()
        .or(session.first_prompt.as_deref())
        .unwrap_or(&fallback);
    let title = format!("{} (fork)", base_title);

    let request = CreateSessionRequest {
        provider_id: "claude-code".to_string(),
        working_directory: Some(session.project_path.clone()),
        title: Some(title),
        resume_claude_session_id: Some(claude_session_id),
        fork: Some(true),
        initial_prompt: None,
        mcp_config_path: None,
        allowed_tools: None,
        model: None,
        system_prompt: None,
    };

    create_session(state, request).await
}

#[tauri::command]
pub async fn fork_active_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<SessionInfo, String> {
    let session = state.session_store.get(&session_id).await
        .ok_or_else(|| "Session not found".to_string())?;
    let claude_session_id = session.metadata
        .get("claude_session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "No Claude session ID available for fork".to_string())?
        .to_string();
    let title = format!("{} (fork)", session.title);
    let request = CreateSessionRequest {
        provider_id: session.provider_id.clone(),
        working_directory: session.working_directory.clone(),
        title: Some(title),
        resume_claude_session_id: Some(claude_session_id),
        fork: Some(true),
        initial_prompt: None,
        mcp_config_path: None,
        allowed_tools: None,
        model: None,
        system_prompt: None,
    };
    create_session(state, request).await
}

// --- Profile commands ---

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>, String> {
    Ok(state.profile_store.list().await)
}

#[tauri::command]
pub async fn get_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Profile, String> {
    state
        .profile_store
        .get(&profile_id)
        .await
        .ok_or_else(|| format!("Profile not found: {}", profile_id))
}

#[tauri::command]
pub async fn save_profile(
    state: State<'_, AppState>,
    profile: Profile,
) -> Result<(), String> {
    state.profile_store.save(profile).await
}

#[tauri::command]
pub async fn delete_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<(), String> {
    state.profile_store.delete(&profile_id).await
}

#[tauri::command]
pub async fn launch_profile(
    state: State<'_, AppState>,
    request: ProfileLaunchRequest,
) -> Result<SessionInfo, String> {
    let profile = state
        .profile_store
        .get(&request.profile_id)
        .await
        .ok_or_else(|| format!("Profile not found: {}", request.profile_id))?;

    // Resolve working directory
    let working_directory = match &profile.working_directory {
        Some(claude_tabs_core::profile::WorkingDirConfig::Fixed { path }) => Some(path.clone()),
        Some(claude_tabs_core::profile::WorkingDirConfig::FromInput { key }) => {
            request.input_values.get(key).cloned()
        }
        _ => request.working_directory.clone(),
    };

    // Resolve prompt template
    let initial_prompt = profile.prompt_template.as_ref().map(|template| {
        state.profile_store.resolve_prompt(template, &request.input_values)
    });

    // Handle MCP config
    let session_id_for_mcp = uuid::Uuid::new_v4().to_string();
    let mcp_config_path = if let Some(ref mcp) = profile.mcp_servers {
        state
            .profile_store
            .write_temp_mcp_config(&session_id_for_mcp, mcp)?
            .map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    let title = profile.name.clone();

    let create_request = CreateSessionRequest {
        provider_id: "claude-code".to_string(),
        working_directory,
        title: Some(title),
        resume_claude_session_id: None,
        fork: None,
        initial_prompt,
        mcp_config_path,
        allowed_tools: profile.allowed_tools.clone(),
        model: profile.model.clone(),
        system_prompt: profile.system_prompt.clone(),
    };

    let result = create_session(state.clone(), create_request).await?;

    // Store profile metadata on the session
    state
        .session_store
        .set_metadata(
            &result.id,
            "profile_id",
            serde_json::Value::String(profile.id.clone()),
        )
        .await;

    Ok(result)
}

// ============================================================================
// Window Focus Commands (Platform-specific)
// ============================================================================

/// Focus the application window using native platform APIs.
/// This bypasses Tauri's buggy setFocus() on macOS.
#[tauri::command]
pub fn focus_window() -> Result<(), String> {
    debug!("focus_window command called");
    platform_focus::focus_window().map_err(|e| e.to_string())
}

/// Request user attention (dock bounce on macOS, taskbar flash on Windows).
///
/// # Arguments
/// * `critical` - If true, uses critical/persistent attention. If false, uses informational/brief.
#[tauri::command]
pub fn request_attention(critical: bool) -> Result<(), String> {
    debug!(critical = critical, "request_attention command called");
    let attention_type = if critical {
        AttentionType::Critical
    } else {
        AttentionType::Informational
    };
    platform_focus::request_attention(attention_type).map_err(|e| e.to_string())
}

/// Check if the application is currently the frontmost/active app.
#[tauri::command]
pub fn is_app_active() -> Result<bool, String> {
    platform_focus::is_app_active().map_err(|e| e.to_string())
}

/// Manually set a session's state.
/// Used for manual state changes like marking a session as idle.
#[tauri::command]
pub async fn set_session_state(
    state: State<'_, AppState>,
    session_id: String,
    new_state: String,
) -> Result<(), String> {
    info!(session_id = %session_id, new_state = %new_state, "Manually setting session state");

    let session = state
        .session_store
        .get(&session_id)
        .await
        .ok_or_else(|| "Session not found".to_string())?;

    let from = session.state.clone();
    if from == new_state {
        return Ok(());
    }

    // Validate the state is one of our known states
    let valid_states = ["idle", "active", "running", "your_turn"];
    if !valid_states.contains(&new_state.as_str()) {
        return Err(format!("Invalid state: {}. Valid states: {:?}", new_state, valid_states));
    }

    state.session_store.update_state(&session_id, &new_state).await;

    let event = claude_tabs_core::Event::new(
        "session.state_changed",
        serde_json::json!({
            "session_id": session_id,
            "from": from,
            "to": new_state,
            "manual": true,
        }),
    );
    state.event_bus.emit(event).await;

    Ok(())
}
