//! macOS-specific window focus implementation using Cocoa/AppKit APIs

use crate::{AttentionType, FocusError, FocusResult};
use cocoa::appkit::{NSApp, NSRequestUserAttentionType};
use cocoa::base::nil;
use objc::{msg_send, sel, sel_impl};
use tracing::debug;

/// Focus the application window using native macOS APIs.
///
/// This uses NSApp.activateIgnoringOtherApps which is more reliable
/// than Tauri's setFocus() on macOS, especially when the app is in
/// the background or minimized.
pub fn focus_window() -> FocusResult<()> {
    unsafe {
        let app = NSApp();
        if app == nil {
            return Err(FocusError::FocusFailed("NSApp is nil".to_string()));
        }

        // Activate the app, bringing it to the foreground
        // The boolean argument (YES = true) means ignore other apps and force activation
        let _: () = msg_send![app, activateIgnoringOtherApps: true];

        // Also try to make sure we're not hidden
        let _: () = msg_send![app, unhide: nil];

        debug!("macOS: Window focus requested via NSApp.activateIgnoringOtherApps");
        Ok(())
    }
}

/// Request user attention (dock icon bounce).
pub fn request_attention(attention_type: AttentionType) -> FocusResult<()> {
    unsafe {
        let app = NSApp();
        if app == nil {
            return Err(FocusError::AttentionFailed("NSApp is nil".to_string()));
        }

        let ns_attention_type = match attention_type {
            AttentionType::Informational => NSRequestUserAttentionType::NSInformationalRequest,
            AttentionType::Critical => NSRequestUserAttentionType::NSCriticalRequest,
        };

        let _: i64 = msg_send![app, requestUserAttention: ns_attention_type];

        debug!(
            "macOS: Requested user attention with type {:?}",
            attention_type
        );
        Ok(())
    }
}

/// Check if the application is currently the frontmost app.
pub fn is_app_active() -> FocusResult<bool> {
    unsafe {
        let app = NSApp();
        if app == nil {
            return Err(FocusError::FocusFailed("NSApp is nil".to_string()));
        }

        let is_active: bool = msg_send![app, isActive];
        Ok(is_active)
    }
}
