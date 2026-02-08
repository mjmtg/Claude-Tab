use crate::event_bus::EventBus;
use crate::session::SessionStore;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 600; // 10 minutes

const HOOK_SCRIPT: &str = include_str!("../../../src-tauri/resources/claude-tabs-hook.sh");
const SHELL_INTEGRATION_BASH: &str =
    include_str!("../../../src-tauri/resources/shell-integration.bash");
const SHELL_INTEGRATION_ZSH: &str =
    include_str!("../../../src-tauri/resources/shell-integration.zsh");

#[derive(Debug, Deserialize)]
struct HookMessage {
    session_id: String,
    hook_event_name: String,
    claude_session_id: Option<String>,
    // For Notification hook
    #[serde(default)]
    notification_type: Option<String>,
    // For PostToolUseFailure hook
    #[serde(default)]
    is_interrupt: Option<bool>,
}

pub struct HookListener;

impl HookListener {
    pub fn socket_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let pid = std::process::id();
        PathBuf::from(home)
            .join(".claude-tabs")
            .join(format!("hook-{}.sock", pid))
    }

    /// Get the path to the shell integration script for the given shell.
    pub fn shell_integration_path(shell: &str) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let filename = match shell {
            "zsh" => "shell-integration.zsh",
            _ => "shell-integration.bash",
        };
        PathBuf::from(home).join(".claude-tabs").join(filename)
    }

    /// Remove stale socket files from dead processes.
    pub fn cleanup_stale_sockets() {
        let home = match std::env::var("HOME") {
            Ok(h) => h,
            Err(_) => return,
        };
        let dir = PathBuf::from(home).join(".claude-tabs");
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(pid_str) = name_str
                .strip_prefix("hook-")
                .and_then(|s| s.strip_suffix(".sock"))
            {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    // Check if process is alive via kill(pid, 0)
                    let alive = unsafe { libc::kill(pid as i32, 0) } == 0;
                    if !alive {
                        let path = entry.path();
                        if std::fs::remove_file(&path).is_ok() {
                            info!(path = %path.display(), "Removed stale hook socket");
                        }
                    }
                }
            }
        }
    }

    pub fn install_hooks() {
        let home = match std::env::var("HOME") {
            Ok(h) => h,
            Err(_) => return,
        };
        let home_path = PathBuf::from(&home);

        // Install hook script
        let hook_dir = home_path.join(".claude-tabs");
        let _ = std::fs::create_dir_all(&hook_dir);
        let hook_script_path = hook_dir.join("claude-tabs-hook.sh");
        if std::fs::write(&hook_script_path, HOOK_SCRIPT).is_err() {
            warn!("Failed to write hook script");
            return;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(
                &hook_script_path,
                std::fs::Permissions::from_mode(0o755),
            );
        }

        // Install shell integration scripts
        let bash_integration_path = hook_dir.join("shell-integration.bash");
        if let Err(e) = std::fs::write(&bash_integration_path, SHELL_INTEGRATION_BASH) {
            warn!(error = %e, "Failed to write bash shell integration");
        }

        let zsh_integration_path = hook_dir.join("shell-integration.zsh");
        if let Err(e) = std::fs::write(&zsh_integration_path, SHELL_INTEGRATION_ZSH) {
            warn!(error = %e, "Failed to write zsh shell integration");
        }

        // Update ~/.claude/settings.json
        let claude_dir = home_path.join(".claude");
        let _ = std::fs::create_dir_all(&claude_dir);
        let settings_path = claude_dir.join("settings.json");
        let hook_cmd = hook_script_path.to_string_lossy().to_string();

        let mut settings: serde_json::Value = if settings_path.exists() {
            std::fs::read_to_string(&settings_path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        let hook_entry = serde_json::json!([{
            "matcher": "*",
            "hooks": [{ "type": "command", "command": hook_cmd }]
        }]);

        if let Some(obj) = settings.as_object_mut() {
            let hooks = obj.entry("hooks").or_insert(serde_json::json!({}));
            if let Some(hooks_obj) = hooks.as_object_mut() {
                hooks_obj.insert("SessionStart".to_string(), hook_entry.clone());
                hooks_obj.insert("UserPromptSubmit".to_string(), hook_entry.clone());
                hooks_obj.insert("PermissionRequest".to_string(), hook_entry.clone());
                hooks_obj.insert("PostToolUse".to_string(), hook_entry.clone());
                hooks_obj.insert("PostToolUseFailure".to_string(), hook_entry.clone());
                hooks_obj.insert("Notification".to_string(), hook_entry.clone());
                hooks_obj.insert("Stop".to_string(), hook_entry);
            }
        }

        if let Ok(output) = serde_json::to_string_pretty(&settings) {
            let _ = std::fs::write(&settings_path, output);
        }

        info!(path = %settings_path.display(), "Hooks auto-configured");
    }

    pub fn start(
        session_store: Arc<SessionStore>,
        event_bus: Arc<EventBus>,
        idle_timeout_secs: Option<u64>,
    ) {
        Self::install_hooks();
        Self::cleanup_stale_sockets();

        let socket_path = Self::socket_path();
        let idle_timeout =
            Duration::from_secs(idle_timeout_secs.unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS));

        if let Some(parent) = socket_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::remove_file(&socket_path);

        let last_activity: Arc<Mutex<HashMap<String, Instant>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Task 1: Socket listener
        let ss = session_store.clone();
        let eb = event_bus.clone();
        let activity = last_activity.clone();
        let path = socket_path.clone();

        tokio::spawn(async move {
            let listener = match UnixListener::bind(&path) {
                Ok(l) => l,
                Err(e) => {
                    error!(path = %path.display(), error = %e, "Failed to bind hook socket");
                    return;
                }
            };

            info!(path = %path.display(), "Hook listener started");

            loop {
                match listener.accept().await {
                    Ok((mut stream, _)) => {
                        let ss = ss.clone();
                        let eb = eb.clone();
                        let activity = activity.clone();

                        tokio::spawn(async move {
                            let mut buf = Vec::with_capacity(4096);
                            if let Err(e) = stream.read_to_end(&mut buf).await {
                                debug!(error = %e, "Failed to read from hook connection");
                                return;
                            }
                            let data = String::from_utf8_lossy(&buf);
                            Self::handle_message(&data, &ss, &eb, &activity).await;
                        });
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to accept hook connection");
                    }
                }
            }
        });

        // Task 2: Idle timeout checker (every 30s)
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;

                let now = Instant::now();
                let timestamps = last_activity.lock().await;
                let timed_out: Vec<String> = timestamps
                    .iter()
                    .filter(|(_, &ts)| now.duration_since(ts) > idle_timeout)
                    .map(|(id, _)| id.clone())
                    .collect();
                drop(timestamps);

                for session_id in timed_out {
                    let session = session_store.get(&session_id).await;
                    if let Some(s) = session {
                        if s.state == "active" {
                            session_store.update_state(&session_id, "idle").await;
                            let event = crate::Event::new(
                                "session.state_changed",
                                serde_json::json!({
                                    "session_id": session_id,
                                    "from": "active",
                                    "to": "idle",
                                }),
                            );
                            event_bus.emit(event).await;
                            info!(session_id = %session_id, "Session went idle (timeout)");
                        }
                    }
                    last_activity.lock().await.remove(&session_id);
                }
            }
        });
    }

    async fn handle_message(
        data: &str,
        session_store: &SessionStore,
        event_bus: &EventBus,
        last_activity: &Mutex<HashMap<String, Instant>>,
    ) {
        let msg: HookMessage = match serde_json::from_str(data.trim()) {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, "Invalid hook message");
                return;
            }
        };

        debug!(
            session_id = %msg.session_id,
            event = %msg.hook_event_name,
            claude_session_id = ?msg.claude_session_id,
            "Hook event received"
        );

        // Store claude_session_id in session metadata if present
        if let Some(ref claude_sid) = msg.claude_session_id {
            if !claude_sid.is_empty() {
                session_store
                    .set_metadata(
                        &msg.session_id,
                        "claude_session_id",
                        serde_json::Value::String(claude_sid.clone()),
                    )
                    .await;
            }
        }

        // Update activity timestamp
        last_activity
            .lock()
            .await
            .insert(msg.session_id.clone(), Instant::now());

        let session = match session_store.get(&msg.session_id).await {
            Some(s) => s,
            None => {
                warn!(session_id = %msg.session_id, "Hook event for unknown session");
                return;
            }
        };

        let from = session.state.clone();

        // Mark session as having received user input (for archive gating)
        if msg.hook_event_name == "UserPromptSubmit" {
            session_store
                .set_metadata(&msg.session_id, "had_user_input", serde_json::Value::Bool(true))
                .await;
        }

        // Emit hook-specific event for extensions to react to
        let hook_event = crate::Event::new(
            &format!("hook.{}", msg.hook_event_name),
            serde_json::json!({
                "session_id": msg.session_id,
                "claude_session_id": msg.claude_session_id,
            }),
        );
        event_bus.emit(hook_event).await;

        let new_state = match msg.hook_event_name.as_str() {
            "UserPromptSubmit" => "running",
            "PostToolUse" => "running",
            "PermissionRequest" => "your_turn",
            "Stop" => "active",
            // Handle interrupts/cancellations - transition to active so user knows they can interact
            "PostToolUseFailure" => {
                if msg.is_interrupt.unwrap_or(false) {
                    info!(session_id = %msg.session_id, "Tool use interrupted by user");
                    "active"
                } else {
                    // Non-interrupt failure, stay in running state
                    debug!(session_id = %msg.session_id, "Tool use failed (not interrupt)");
                    return;
                }
            }
            // Handle notifications - permission_prompt and idle_prompt indicate waiting for input
            "Notification" => {
                match msg.notification_type.as_deref() {
                    Some("permission_prompt") => "your_turn",
                    Some("idle_prompt") => "active",
                    _ => {
                        debug!(notification_type = ?msg.notification_type, "Ignoring notification");
                        return;
                    }
                }
            }
            other => {
                debug!(event = %other, "Ignoring unhandled hook event");
                return;
            }
        };

        if from == new_state {
            return;
        }

        session_store
            .update_state(&msg.session_id, new_state)
            .await;

        let event = crate::Event::new(
            "session.state_changed",
            serde_json::json!({
                "session_id": msg.session_id,
                "from": from,
                "to": new_state,
            }),
        );
        event_bus.emit(event).await;

        info!(
            session_id = %msg.session_id,
            from = %from,
            to = %new_state,
            "Session state changed via hook"
        );
    }
}
