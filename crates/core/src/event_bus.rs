use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, trace};

pub type EventHandler = Arc<dyn Fn(&Event) + Send + Sync>;
pub type Middleware = Arc<dyn Fn(&Event) -> bool + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub topic: String,
    pub payload: serde_json::Value,
    #[serde(default = "default_session_id")]
    pub session_id: Option<String>,
    #[serde(skip)]
    pub timestamp: u64,
}

fn default_session_id() -> Option<String> {
    None
}

impl Event {
    pub fn new(topic: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            topic: topic.into(),
            payload,
            session_id: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

#[derive(Clone)]
pub struct Subscription {
    pub id: u64,
    pub pattern: String,
}

pub struct EventBus {
    sender: broadcast::Sender<Event>,
    handlers: Arc<RwLock<HashMap<u64, (String, EventHandler)>>>,
    middleware: Arc<RwLock<Vec<Middleware>>>,
    next_id: AtomicU64,
    // Topic index for O(1) dispatch - maps exact topics and pattern prefixes to handler IDs
    topic_index: Arc<RwLock<TopicIndex>>,
}

/// Index structure for fast topic-based handler lookup
struct TopicIndex {
    // Exact topic matches: topic -> handler IDs
    exact: HashMap<String, Vec<u64>>,
    // Wildcard pattern handlers (*.  and **.)
    wildcard_single: HashMap<String, Vec<u64>>,  // prefix.* patterns
    wildcard_deep: HashMap<String, Vec<u64>>,    // prefix.** patterns
    // Global wildcard (*) handlers
    global: Vec<u64>,
}

impl TopicIndex {
    fn new() -> Self {
        Self {
            exact: HashMap::new(),
            wildcard_single: HashMap::new(),
            wildcard_deep: HashMap::new(),
            global: Vec::new(),
        }
    }

    fn add(&mut self, id: u64, pattern: &str) {
        if pattern == "*" {
            self.global.push(id);
        } else if pattern.ends_with(".**") {
            let prefix = &pattern[..pattern.len() - 3];
            self.wildcard_deep.entry(prefix.to_string()).or_default().push(id);
        } else if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            self.wildcard_single.entry(prefix.to_string()).or_default().push(id);
        } else {
            self.exact.entry(pattern.to_string()).or_default().push(id);
        }
    }

    fn remove(&mut self, id: u64, pattern: &str) {
        if pattern == "*" {
            self.global.retain(|&h| h != id);
        } else if pattern.ends_with(".**") {
            let prefix = &pattern[..pattern.len() - 3];
            if let Some(ids) = self.wildcard_deep.get_mut(prefix) {
                ids.retain(|&h| h != id);
            }
        } else if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            if let Some(ids) = self.wildcard_single.get_mut(prefix) {
                ids.retain(|&h| h != id);
            }
        } else if let Some(ids) = self.exact.get_mut(pattern) {
            ids.retain(|&h| h != id);
        }
    }

    fn get_matching_handlers(&self, topic: &str) -> Vec<u64> {
        let mut result = Vec::new();

        // Global handlers
        result.extend(&self.global);

        // Exact match
        if let Some(ids) = self.exact.get(topic) {
            result.extend(ids);
        }

        // Check wildcard patterns
        // For "session.created", check if "session.*" or "session.**" matches
        if let Some(dot_pos) = topic.rfind('.') {
            let prefix = &topic[..dot_pos];

            // Single wildcard: prefix.* matches prefix.X but not prefix.X.Y
            if !topic[dot_pos + 1..].contains('.') {
                if let Some(ids) = self.wildcard_single.get(prefix) {
                    result.extend(ids);
                }
            }

            // Deep wildcard: check all prefixes
            let mut current = topic;
            while let Some(pos) = current.rfind('.') {
                let check_prefix = &current[..pos];
                if let Some(ids) = self.wildcard_deep.get(check_prefix) {
                    result.extend(ids);
                }
                current = check_prefix;
            }
            // Also check empty prefix for patterns like ".**"
            if let Some(ids) = self.wildcard_deep.get("") {
                result.extend(ids);
            }
        }

        result
    }
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            middleware: Arc::new(RwLock::new(Vec::new())),
            next_id: AtomicU64::new(1),
            topic_index: Arc::new(RwLock::new(TopicIndex::new())),
        }
    }

    pub async fn emit(&self, event: Event) {
        let middleware = self.middleware.read().await;
        for mw in middleware.iter() {
            if !mw(&event) {
                trace!(topic = %event.topic, "Event blocked by middleware");
                return;
            }
        }
        drop(middleware);

        debug!(topic = %event.topic, "Emitting event");

        // Use topic index for O(1) handler lookup instead of iterating all handlers
        let topic_index = self.topic_index.read().await;
        let matching_ids = topic_index.get_matching_handlers(&event.topic);
        drop(topic_index);

        let handlers = self.handlers.read().await;
        for id in matching_ids {
            if let Some((_, handler)) = handlers.get(&id) {
                handler(&event);
            }
        }
        drop(handlers);

        let _ = self.sender.send(event);
    }

    pub async fn subscribe(
        &self,
        pattern: impl Into<String>,
        handler: EventHandler,
    ) -> Subscription {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let pattern = pattern.into();

        // Add to handlers map
        self.handlers
            .write()
            .await
            .insert(id, (pattern.clone(), handler));

        // Add to topic index for O(1) lookup
        self.topic_index.write().await.add(id, &pattern);

        Subscription { id, pattern }
    }

    pub async fn unsubscribe(&self, subscription: &Subscription) {
        self.handlers.write().await.remove(&subscription.id);
        self.topic_index.write().await.remove(subscription.id, &subscription.pattern);
    }

    pub async fn add_middleware(&self, middleware: Middleware) {
        self.middleware.write().await.push(middleware);
    }

    pub fn receiver(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_index_matching() {
        let mut index = TopicIndex::new();

        // Register handlers with different patterns
        index.add(1, "*");                    // global wildcard
        index.add(2, "core.*");               // single-level wildcard
        index.add(3, "core.**");              // deep wildcard
        index.add(4, "session.created");      // exact match

        // Global wildcard matches anything
        let matches = index.get_matching_handlers("anything");
        assert!(matches.contains(&1));

        // Single-level wildcard: core.* matches core.session but not core.session.created
        let matches = index.get_matching_handlers("core.session");
        assert!(matches.contains(&1)); // global
        assert!(matches.contains(&2)); // core.*
        assert!(matches.contains(&3)); // core.**

        let matches = index.get_matching_handlers("core.session.created");
        assert!(matches.contains(&1));  // global
        assert!(!matches.contains(&2)); // core.* should NOT match
        assert!(matches.contains(&3));  // core.** should match

        // Exact match
        let matches = index.get_matching_handlers("session.created");
        assert!(matches.contains(&1)); // global
        assert!(matches.contains(&4)); // exact

        let matches = index.get_matching_handlers("session.closed");
        assert!(matches.contains(&1));  // global
        assert!(!matches.contains(&4)); // exact should NOT match
    }
}
