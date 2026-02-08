use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: String,
    pub display_name: String,
    pub color: String,
    pub icon: Option<String>,
    pub priority: u32,
    pub category: String,
    pub valid_from: Vec<String>,
}

impl State {
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            color: "#808080".to_string(),
            icon: None,
            priority: 0,
            category: "default".to_string(),
            valid_from: Vec::new(),
        }
    }

    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    pub fn with_valid_from(mut self, from: Vec<String>) -> Self {
        self.valid_from = from;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub session_id: String,
    pub from: String,
    pub to: String,
    pub trigger: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub type TransitionGuard = Arc<dyn Fn(&Transition) -> bool + Send + Sync>;

pub struct StateRegistry {
    states: Arc<RwLock<HashMap<String, State>>>,
    guards: Arc<RwLock<Vec<TransitionGuard>>>,
}

impl StateRegistry {
    pub fn new() -> Self {
        let registry = Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            guards: Arc::new(RwLock::new(Vec::new())),
        };
        registry
    }

    pub async fn register_core_states(&self) {
        let core_states = vec![
            State::new("idle", "Idle")
                .with_color("#808080")
                .with_priority(0)
                .with_category("core"),
            State::new("active", "Active")
                .with_color("#4caf50")
                .with_priority(1)
                .with_category("core"),
            State::new("running", "Running")
                .with_color("#2196f3")
                .with_priority(2)
                .with_category("core"),
            State::new("your_turn", "Your Turn")
                .with_color("#ff9800")
                .with_priority(10)
                .with_category("core"),
        ];

        let mut states = self.states.write().await;
        for state in core_states {
            states.insert(state.id.clone(), state);
        }
    }

    pub async fn register_state(&self, state: State) {
        debug!(state_id = %state.id, "Registering state");
        self.states.write().await.insert(state.id.clone(), state);
    }

    pub async fn get_state(&self, id: &str) -> Option<State> {
        self.states.read().await.get(id).cloned()
    }

    pub async fn list_states(&self) -> Vec<State> {
        self.states.read().await.values().cloned().collect()
    }

    pub async fn add_guard(&self, guard: TransitionGuard) {
        self.guards.write().await.push(guard);
    }

    pub async fn validate_transition(&self, transition: &Transition) -> bool {
        let states = self.states.read().await;

        if let Some(target_state) = states.get(&transition.to) {
            if !target_state.valid_from.is_empty()
                && !target_state.valid_from.contains(&transition.from)
            {
                return false;
            }
        }
        drop(states);

        let guards = self.guards.read().await;
        for guard in guards.iter() {
            if !guard(transition) {
                return false;
            }
        }

        true
    }
}

impl Default for StateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StateMachine {
    pub registry: StateRegistry,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            registry: StateRegistry::new(),
        }
    }

    pub async fn init(&self) {
        self.registry.register_core_states().await;
    }

    pub async fn transition(
        &self,
        session_id: &str,
        from: &str,
        to: &str,
        trigger: &str,
    ) -> Option<Transition> {
        let transition = Transition {
            session_id: session_id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            trigger: trigger.to_string(),
            metadata: HashMap::new(),
        };

        if self.registry.validate_transition(&transition).await {
            Some(transition)
        } else {
            debug!(
                session_id = %session_id,
                from = %from,
                to = %to,
                "Transition blocked"
            );
            None
        }
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}
