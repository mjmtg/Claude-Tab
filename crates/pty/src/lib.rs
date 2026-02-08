//! PTY Crate
//!
//! Provides terminal (PTY) management functionality:
//! - `PtyManager`: Spawns and manages PTY processes
//! - `OutputStream`: Broadcasts PTY output to subscribers
//! - `SessionBuffer`: Per-session output buffering

pub mod manager;
pub mod output_stream;

// Core types
pub use manager::{PtyError, PtyManager};
pub use output_stream::{OutputChunk, OutputStream, SessionBuffer};
