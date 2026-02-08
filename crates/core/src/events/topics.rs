//! Event Topics
//!
//! Centralized constants for all event topic strings.
//! Use these instead of hardcoded strings to prevent typos
//! and enable refactoring.

/// Session lifecycle events.
pub mod session {
    /// Emitted when a new session is created.
    ///
    /// Payload: `{ session_id: String, provider_id: String }`
    pub const CREATED: &str = "session.created";

    /// Emitted when a session is closed.
    ///
    /// Payload: `{ session_id: String }`
    pub const CLOSED: &str = "session.closed";

    /// Emitted when a session's state changes.
    ///
    /// Payload: `{ session_id: String, from: String, to: String }`
    pub const STATE_CHANGED: &str = "session.state_changed";

    /// Emitted when a session is renamed.
    ///
    /// Payload: `{ session_id: String, title: String }`
    pub const RENAMED: &str = "session.renamed";

    /// Emitted when session metadata is updated.
    ///
    /// Payload: `{ session_id: String, key: String, value: Value }`
    pub const METADATA_CHANGED: &str = "session.metadata_changed";

    /// Emitted when the active session changes.
    ///
    /// Payload: `{ session_id: Option<String> }`
    pub const ACTIVE_CHANGED: &str = "session.active_changed";
}

/// PTY (terminal) events.
pub mod pty {
    /// Emitted when PTY output is received.
    ///
    /// Payload: `{ session_id: String, data: Vec<u8> }`
    pub const OUTPUT: &str = "pty.output";

    /// Emitted when a PTY process exits.
    ///
    /// Payload: `{ session_id: String, exit_code: Option<i32> }`
    pub const EXIT: &str = "pty.exit";

    /// Emitted when a PTY is resized.
    ///
    /// Payload: `{ session_id: String, rows: u16, cols: u16 }`
    pub const RESIZED: &str = "pty.resized";
}

/// Detection and state machine events.
pub mod detection {
    /// Emitted when a detector triggers.
    ///
    /// Payload: `{ session_id: String, detector_id: String, state: String, confidence: f32 }`
    pub const TRIGGERED: &str = "detection.triggered";

    /// Emitted when a reaction starts executing.
    ///
    /// Payload: `{ session_id: String, reaction_id: String }`
    pub const REACTION_STARTED: &str = "detection.reaction_started";

    /// Emitted when a reaction completes.
    ///
    /// Payload: `{ session_id: String, reaction_id: String, success: bool }`
    pub const REACTION_COMPLETED: &str = "detection.reaction_completed";

    /// Emitted when a reaction is cancelled.
    ///
    /// Payload: `{ session_id: String, reaction_id: String }`
    pub const REACTION_CANCELLED: &str = "detection.reaction_cancelled";
}

/// Configuration events.
pub mod config {
    /// Emitted when a configuration value changes.
    ///
    /// Payload: `{ key: String, value: Value, layer: String }`
    pub const CHANGED: &str = "config.changed";

    /// Emitted when configuration is reloaded from disk.
    ///
    /// Payload: `{ keys: Vec<String> }`
    pub const RELOADED: &str = "config.reloaded";
}

/// Extension lifecycle events.
pub mod extension {
    /// Emitted when an extension is activated.
    ///
    /// Payload: `{ extension_id: String }`
    pub const ACTIVATED: &str = "extension.activated";

    /// Emitted when an extension is deactivated.
    ///
    /// Payload: `{ extension_id: String }`
    pub const DEACTIVATED: &str = "extension.deactivated";

    /// Emitted when an extension fails to activate.
    ///
    /// Payload: `{ extension_id: String, error: String }`
    pub const ACTIVATION_FAILED: &str = "extension.activation_failed";
}

/// Hook events (from Claude tool calls).
pub mod hook {
    /// Emitted when a hook payload is received.
    ///
    /// Payload: `{ session_id: String, tool_name: String, event: String, payload: Value }`
    pub const RECEIVED: &str = "hook.received";

    /// Emitted when permission is needed.
    ///
    /// Payload: `{ session_id: String, tool_name: String }`
    pub const PERMISSION_NEEDED: &str = "hook.permission_needed";
}

/// Profile events.
pub mod profile {
    /// Emitted when a profile is saved.
    ///
    /// Payload: `{ profile_id: String }`
    pub const SAVED: &str = "profile.saved";

    /// Emitted when a profile is deleted.
    ///
    /// Payload: `{ profile_id: String }`
    pub const DELETED: &str = "profile.deleted";

    /// Emitted when a profile is launched.
    ///
    /// Payload: `{ profile_id: String, session_id: String }`
    pub const LAUNCHED: &str = "profile.launched";
}

/// Archive events.
pub mod archive {
    /// Emitted when a session is archived.
    ///
    /// Payload: `{ session_id: String }`
    pub const CREATED: &str = "archive.created";

    /// Emitted when an archived session is deleted.
    ///
    /// Payload: `{ session_id: String }`
    pub const DELETED: &str = "archive.deleted";
}

/// System events.
pub mod system {
    /// Emitted on application startup.
    ///
    /// Payload: `{ version: String }`
    pub const STARTUP: &str = "system.startup";

    /// Emitted before application shutdown.
    ///
    /// Payload: `{}`
    pub const SHUTDOWN: &str = "system.shutdown";

    /// Emitted when an unrecoverable error occurs.
    ///
    /// Payload: `{ error: String, context: String }`
    pub const ERROR: &str = "system.error";
}
