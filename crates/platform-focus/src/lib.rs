//! Platform-specific window focus handling
//!
//! This crate provides native window focus functionality that bypasses
//! Tauri's buggy window APIs (particularly on macOS where setFocus() is broken).

#[cfg(target_os = "macos")]
mod macos;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FocusError {
    #[error("Platform not supported")]
    UnsupportedPlatform,
    #[error("Failed to focus window: {0}")]
    FocusFailed(String),
    #[error("Failed to request attention: {0}")]
    AttentionFailed(String),
}

/// Result type for focus operations
pub type FocusResult<T> = Result<T, FocusError>;

/// Attention type for request_attention
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionType {
    /// Informational - subtle notification (bounce once on macOS)
    Informational,
    /// Critical - persistent notification (continuous bounce on macOS)
    Critical,
}

/// Focus the application window using native APIs.
///
/// On macOS, this uses NSApp.activateIgnoringOtherApps to ensure
/// the window comes to the front even when the app is in background.
pub fn focus_window() -> FocusResult<()> {
    #[cfg(target_os = "macos")]
    {
        macos::focus_window()
    }
    #[cfg(not(target_os = "macos"))]
    {
        // On other platforms, return success and let Tauri handle it
        // This is a fallback - Tauri's focus works better on non-macOS
        Ok(())
    }
}

/// Request user attention (dock bounce on macOS, taskbar flash on Windows).
///
/// # Arguments
/// * `attention_type` - The type of attention to request
pub fn request_attention(attention_type: AttentionType) -> FocusResult<()> {
    #[cfg(target_os = "macos")]
    {
        macos::request_attention(attention_type)
    }
    #[cfg(not(target_os = "macos"))]
    {
        // On other platforms, return success and let Tauri handle it
        let _ = attention_type;
        Ok(())
    }
}

/// Check if the application is currently the frontmost app.
pub fn is_app_active() -> FocusResult<bool> {
    #[cfg(target_os = "macos")]
    {
        macos::is_app_active()
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Default to true on unsupported platforms
        Ok(true)
    }
}
