use claude_tabs_core::traits::provider::PtySize;
use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tracing::{debug, error, info};

struct PtyInstance {
    master: Box<dyn MasterPty + Send>,
    writer: Option<Box<dyn Write + Send>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    size: PtySize,
}

// Safety: PtyInstance is only accessed through Mutex, so Send is sufficient.
// MasterPty is Send but not Sync, which is fine under Mutex.
unsafe impl Sync for PtyInstance {}

pub struct PtyManager {
    instances: Arc<Mutex<HashMap<String, PtyInstance>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn spawn(
        &self,
        session_id: &str,
        command: &str,
        args: &[String],
        working_dir: Option<&str>,
        env: &HashMap<String, String>,
        size: PtySize,
    ) -> Result<Box<dyn Read + Send>, PtyError> {
        let pty_system = native_pty_system();

        let pty_size = portable_pty::PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(pty_size)
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))?;

        let mut cmd = CommandBuilder::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        if let Some(dir) = working_dir {
            cmd.cwd(dir);
        }
        for (key, value) in env {
            cmd.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))?;

        info!(session_id = %session_id, command = %command, "PTY spawned");

        let instance = PtyInstance {
            master: pair.master,
            writer: Some(writer),
            child,
            size,
        };

        self.instances
            .lock()
            .insert(session_id.to_string(), instance);

        Ok(reader)
    }

    pub fn write_data(&self, session_id: &str, data: &[u8]) -> Result<(), PtyError> {
        let mut instances = self.instances.lock();
        let instance = instances
            .get_mut(session_id)
            .ok_or_else(|| PtyError::NotFound(session_id.to_string()))?;

        if let Some(writer) = &mut instance.writer {
            writer
                .write_all(data)
                .map_err(|e| PtyError::WriteFailed(e.to_string()))?;
            writer
                .flush()
                .map_err(|e| PtyError::WriteFailed(e.to_string()))?;
        }
        Ok(())
    }

    pub fn resize(&self, session_id: &str, size: PtySize) -> Result<(), PtyError> {
        let mut instances = self.instances.lock();
        let instance = instances
            .get_mut(session_id)
            .ok_or_else(|| PtyError::NotFound(session_id.to_string()))?;

        let pty_size = portable_pty::PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        instance
            .master
            .resize(pty_size)
            .map_err(|e| PtyError::ResizeFailed(e.to_string()))?;
        instance.size = size;

        debug!(session_id = %session_id, rows = size.rows, cols = size.cols, "PTY resized");
        Ok(())
    }

    pub fn close(&self, session_id: &str) -> Result<(), PtyError> {
        let mut instances = self.instances.lock();
        if let Some(mut instance) = instances.remove(session_id) {
            instance.writer.take();
            let _ = instance.child.kill();
            info!(session_id = %session_id, "PTY closed");
            Ok(())
        } else {
            Err(PtyError::NotFound(session_id.to_string()))
        }
    }

    pub fn is_alive(&self, session_id: &str) -> bool {
        let mut instances = self.instances.lock();
        if let Some(instance) = instances.get_mut(session_id) {
            match instance.child.try_wait() {
                Ok(Some(_)) => false,
                Ok(None) => true,
                Err(e) => {
                    error!(session_id = %session_id, error = %e, "Error checking PTY status");
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn session_count(&self) -> usize {
        self.instances.lock().len()
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("Spawn failed: {0}")]
    SpawnFailed(String),
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Write failed: {0}")]
    WriteFailed(String),
    #[error("Resize failed: {0}")]
    ResizeFailed(String),
}
