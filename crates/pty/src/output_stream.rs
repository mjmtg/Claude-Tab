use async_trait::async_trait;
use bytes::Bytes;
use claude_tabs_core::traits::buffer::SessionBufferProvider;
use claude_tabs_core::traits::output::OutputChunk as CoreOutputChunk;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, error};

/// PTY output chunk - uses Bytes for zero-copy sharing and Arc<str> for session_id.
#[derive(Debug, Clone)]
pub struct OutputChunk {
    pub session_id: Arc<str>,
    pub data: Bytes,
}

impl OutputChunk {
    /// Get data as a slice for compatibility
    pub fn data_slice(&self) -> &[u8] {
        &self.data
    }
}

impl From<OutputChunk> for CoreOutputChunk {
    fn from(chunk: OutputChunk) -> Self {
        CoreOutputChunk {
            session_id: chunk.session_id.to_string(),
            data: chunk.data.to_vec(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

pub struct OutputStream {
    sender: broadcast::Sender<OutputChunk>,
}

impl OutputStream {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn start_reading(
        &self,
        session_id: String,
        mut reader: Box<dyn Read + Send>,
    ) -> tokio::task::JoinHandle<()> {
        let sender = self.sender.clone();
        // Use Arc<str> for zero-copy session_id sharing
        let sid: Arc<str> = session_id.into();

        tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        debug!(session_id = %sid, "PTY output stream ended");
                        let _ = sender.send(OutputChunk {
                            session_id: Arc::clone(&sid),
                            data: Bytes::new(),
                        });
                        break;
                    }
                    Ok(n) => {
                        // Use Bytes::copy_from_slice for efficient single allocation
                        // The Bytes can be cheaply cloned (reference counted)
                        let chunk = OutputChunk {
                            session_id: Arc::clone(&sid),
                            data: Bytes::copy_from_slice(&buf[..n]),
                        };
                        if sender.send(chunk).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!(session_id = %sid, error = %e, "PTY read error");
                        break;
                    }
                }
            }
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<OutputChunk> {
        self.sender.subscribe()
    }
}

impl Default for OutputStream {
    fn default() -> Self {
        Self::new(64) // Reduced from 256 for lower memory usage
    }
}

/// Per-session buffer entry that tracks output and user input.
struct BufferEntry {
    data: Vec<u8>,
    had_user_input: bool,
}

impl Default for BufferEntry {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            had_user_input: false,
        }
    }
}

/// Per-session buffer that accumulates raw PTY output bytes.
/// Used by the ArchiveManager to capture conversation content for archiving.
/// Implements `SessionBufferProvider` trait for modular architecture.
#[derive(Clone)]
pub struct SessionBuffer {
    buffers: Arc<Mutex<HashMap<String, BufferEntry>>>,
}

impl SessionBuffer {
    pub fn new() -> Self {
        Self {
            buffers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Append bytes to a session's buffer.
    pub async fn append(&self, session_id: &str, data: &[u8]) {
        let mut buffers = self.buffers.lock().await;
        buffers
            .entry(session_id.to_string())
            .or_default()
            .data
            .extend_from_slice(data);
    }

    /// Take all accumulated bytes for a session and clear its buffer.
    pub async fn drain(&self, session_id: &str) -> Vec<u8> {
        let mut buffers = self.buffers.lock().await;
        buffers.remove(session_id).map(|e| e.data).unwrap_or_default()
    }

    /// Remove a session's buffer entirely.
    pub async fn remove(&self, session_id: &str) {
        let mut buffers = self.buffers.lock().await;
        buffers.remove(session_id);
    }

    /// Mark that the session has received user input.
    pub async fn mark_user_input(&self, session_id: &str) {
        let mut buffers = self.buffers.lock().await;
        if let Some(entry) = buffers.get_mut(session_id) {
            entry.had_user_input = true;
        }
    }

    /// Check if the session has received user input.
    pub async fn had_user_input(&self, session_id: &str) -> bool {
        let buffers = self.buffers.lock().await;
        buffers.get(session_id).map(|e| e.had_user_input).unwrap_or(false)
    }

    /// Get a copy of the buffer contents for a session.
    pub async fn get_buffer(&self, session_id: &str) -> Option<Vec<u8>> {
        let buffers = self.buffers.lock().await;
        buffers.get(session_id).map(|e| e.data.clone())
    }

    /// Get the buffer size for a session.
    pub async fn get_size(&self, session_id: &str) -> Option<usize> {
        let buffers = self.buffers.lock().await;
        buffers.get(session_id).map(|e| e.data.len())
    }

    /// Clear a session's buffer data but keep the entry.
    pub async fn clear(&self, session_id: &str) {
        let mut buffers = self.buffers.lock().await;
        if let Some(entry) = buffers.get_mut(session_id) {
            entry.data.clear();
        }
    }
}

impl Default for SessionBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement SessionBufferProvider trait for modular architecture.
#[async_trait]
impl SessionBufferProvider for SessionBuffer {
    async fn create(&self, session_id: &str) {
        let mut buffers = self.buffers.lock().await;
        buffers.entry(session_id.to_string()).or_default();
    }

    async fn remove(&self, session_id: &str) {
        SessionBuffer::remove(self, session_id).await;
    }

    async fn append(&self, session_id: &str, data: &[u8]) {
        SessionBuffer::append(self, session_id, data).await;
    }

    async fn mark_user_input(&self, session_id: &str) {
        SessionBuffer::mark_user_input(self, session_id).await;
    }

    async fn had_user_input(&self, session_id: &str) -> bool {
        SessionBuffer::had_user_input(self, session_id).await
    }

    async fn get_buffer(&self, session_id: &str) -> Option<Vec<u8>> {
        SessionBuffer::get_buffer(self, session_id).await
    }

    async fn get_size(&self, session_id: &str) -> Option<usize> {
        SessionBuffer::get_size(self, session_id).await
    }

    async fn clear(&self, session_id: &str) {
        SessionBuffer::clear(self, session_id).await;
    }
}
