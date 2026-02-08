use claude_tabs_core::config::Config;
use claude_tabs_core::event_bus::EventBus;
use claude_tabs_core::hook_listener::HookListener;
use claude_tabs_core::profile::ProfileStore;
use claude_tabs_core::session::SessionStore;
use claude_tabs_core::state_machine::StateRegistry;
use claude_tabs_pty::{OutputStream, PtyManager};
use claude_tabs_storage::SqliteBackend;
use claude_tabs_tauri_bridge::commands;
use claude_tabs_tauri_bridge::ipc::{AppState, IpcBridge};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting Claude Tabs");

    let event_bus = Arc::new(EventBus::new(2048));
    let config = Arc::new(Config::new());
    let session_store = Arc::new(SessionStore::new());
    let pty_manager = Arc::new(PtyManager::new());
    let output_stream = Arc::new(OutputStream::new(512));
    let state_registry = Arc::new(StateRegistry::new());

    let storage: Arc<dyn claude_tabs_storage::StorageBackend> =
        match SqliteBackend::new("~/.claude-tabs/archive.db") {
            Ok(backend) => Arc::new(backend),
            Err(e) => {
                panic!("Storage initialization failed: {}", e);
            }
        };

    let profile_store = Arc::new(ProfileStore::new());

    let app_state = AppState {
        event_bus: event_bus.clone(),
        config: config.clone(),
        session_store: session_store.clone(),
        pty_manager: pty_manager.clone(),
        output_stream: output_stream.clone(),
        storage: storage.clone(),
        profile_store: profile_store.clone(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::create_session,
            commands::close_session,
            commands::list_sessions,
            commands::get_active_session,
            commands::set_active_session,
            commands::rename_session,
            commands::write_to_pty,
            commands::resize_pty,
            commands::submit_input,
            commands::get_config_value,
            commands::set_config_value,
            commands::setup_hooks,
            // Claude Code session history
            commands::list_claude_sessions,
            commands::get_claude_session,
            commands::get_session_content,
            commands::set_directory_preference,
            commands::get_directory_preferences,
            commands::remove_directory_preference,
            commands::delete_history_session,
            commands::delete_project_sessions,
            commands::resume_session,
            commands::fork_session,
            commands::fork_active_session,
            // Profiles
            commands::list_profiles,
            commands::get_profile,
            commands::save_profile,
            commands::delete_profile,
            commands::launch_profile,
            // Window focus (platform-specific)
            commands::focus_window,
            commands::request_attention,
            commands::is_app_active,
            // Session state management
            commands::set_session_state,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();

            let eb = event_bus.clone();
            let os = output_stream.clone();
            let ss = session_store.clone();
            let sr = state_registry.clone();

            let ps = profile_store.clone();
            tauri::async_runtime::spawn(async move {
                sr.register_core_states().await;
                ps.init().await;
                info!("Core states registered, profiles loaded");
            });

            let bridge = IpcBridge::new(app_handle, os.clone(), eb.clone());
            bridge.start_forwarding();

            let ss_hook = ss.clone();
            let eb_hook = eb.clone();
            tauri::async_runtime::spawn(async move {
                HookListener::start(ss_hook, eb_hook, None);
                info!("Hook listener started");
            });

            // Start directory tracker (OSC 7 parsing for terminal cwd)
            let ss_dir = ss.clone();
            let eb_dir = eb.clone();
            let os_dir = os.clone();
            tauri::async_runtime::spawn(async move {
                claude_tabs_ext_directory_tracker::start(eb_dir, ss_dir, os_dir);
            });
            info!("Directory tracker started");

            // PTY exit handler: remove session when process exits
            {
                let mut exit_receiver = os.subscribe();
                let ss_exit = ss.clone();
                let eb_exit = eb.clone();
                let pm_exit = pty_manager.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        match exit_receiver.recv().await {
                            Ok(chunk) => {
                                if chunk.data.is_empty() {
                                    // PTY exited for this session
                                    let session_id = chunk.session_id.clone();
                                    if ss_exit.get(&session_id).await.is_some() {
                                        let _ = pm_exit.close(&session_id);
                                        ss_exit.remove(&session_id).await;

                                        let remaining = ss_exit.list().await;
                                        let new_active = remaining.first().map(|s| s.id.clone());
                                        ss_exit.set_active(new_active).await;

                                        let event = claude_tabs_core::Event::new(
                                            "session.closed",
                                            serde_json::json!({ "session_id": session_id }),
                                        );
                                        eb_exit.emit(event).await;
                                        info!(session_id = %session_id, "Session auto-closed on PTY exit");
                                    }
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        }
                    }
                });
            }

            info!("IPC bridge started");
            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // On macOS, hide the window instead of closing
                #[cfg(target_os = "macos")]
                {
                    _window.hide().unwrap();
                    api.prevent_close();
                }
                #[cfg(not(target_os = "macos"))]
                let _ = api;
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
