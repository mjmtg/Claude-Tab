use chrono::{DateTime, Utc};
use parking_lot::RwLock as SyncRwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub type SessionId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub provider_id: String,
    pub state: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub working_directory: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Session {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            provider_id: provider_id.into(),
            state: "active".to_string(),
            title: String::new(),
            created_at: Utc::now(),
            working_directory: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_working_directory(mut self, dir: impl Into<String>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }
}

/// Thread-safe session storage using Arc<SyncRwLock<Session>> internally.
/// This allows returning Arc references from list() to avoid cloning entire Session objects.
#[derive(Clone)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<SessionId, Arc<SyncRwLock<Session>>>>>,
    active_session: Arc<RwLock<Option<SessionId>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            active_session: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn add(&self, session: Session) -> SessionId {
        let id = session.id.clone();
        self.sessions.write().await.insert(id.clone(), Arc::new(SyncRwLock::new(session)));
        id
    }

    pub async fn remove(&self, id: &str) -> Option<Session> {
        let removed = self.sessions.write().await.remove(id);
        let mut active = self.active_session.write().await;
        if active.as_deref() == Some(id) {
            *active = None;
        }
        removed.map(|arc| {
            // Try to unwrap, otherwise clone
            Arc::try_unwrap(arc)
                .map(|lock| lock.into_inner())
                .unwrap_or_else(|arc| arc.read().clone())
        })
    }

    pub async fn get(&self, id: &str) -> Option<Session> {
        self.sessions.read().await.get(id).map(|arc| arc.read().clone())
    }

    pub async fn update_state(&self, id: &str, state: impl Into<String>) -> bool {
        if let Some(session_arc) = self.sessions.read().await.get(id) {
            session_arc.write().state = state.into();
            true
        } else {
            false
        }
    }

    pub async fn rename(&self, id: &str, title: impl Into<String>) -> bool {
        if let Some(session_arc) = self.sessions.read().await.get(id) {
            session_arc.write().title = title.into();
            true
        } else {
            false
        }
    }

    pub async fn update_working_directory(&self, id: &str, directory: impl Into<String>) -> bool {
        if let Some(session_arc) = self.sessions.read().await.get(id) {
            session_arc.write().working_directory = Some(directory.into());
            true
        } else {
            false
        }
    }

    /// Returns cloned Sessions. For high-frequency access, consider using list_refs().
    pub async fn list(&self) -> Vec<Session> {
        self.sessions.read().await.values().map(|arc| arc.read().clone()).collect()
    }

    /// Returns Arc references to sessions for zero-copy iteration.
    /// Useful for read-heavy operations that don't need ownership.
    pub async fn list_refs(&self) -> Vec<Arc<SyncRwLock<Session>>> {
        self.sessions.read().await.values().cloned().collect()
    }

    pub async fn set_active(&self, id: Option<String>) {
        *self.active_session.write().await = id;
    }

    pub async fn get_active(&self) -> Option<SessionId> {
        self.active_session.read().await.clone()
    }

    pub async fn set_metadata(&self, id: &str, key: &str, value: serde_json::Value) -> bool {
        if let Some(session_arc) = self.sessions.read().await.get(id) {
            session_arc.write().metadata.insert(key.to_string(), value);
            true
        } else {
            false
        }
    }

    pub async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}
