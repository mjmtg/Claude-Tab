use crate::state_machine::Transition;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReactionTrigger {
    EnterState(String),
    ExitState(String),
    Transition { from: String, to: String },
}

impl ReactionTrigger {
    pub fn matches(&self, transition: &Transition) -> bool {
        match self {
            ReactionTrigger::EnterState(state) => &transition.to == state,
            ReactionTrigger::ExitState(state) => &transition.from == state,
            ReactionTrigger::Transition { from, to } => {
                &transition.from == from && &transition.to == to
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReactionError {
    #[error("Reaction failed: {0}")]
    Failed(String),
    #[error("Reaction cancelled")]
    Cancelled,
}

#[async_trait]
pub trait Reaction: Send + Sync + 'static {
    fn id(&self) -> &str;
    fn triggers(&self) -> Vec<ReactionTrigger>;
    async fn execute(
        &mut self,
        session_id: &str,
        transition: &Transition,
    ) -> Result<(), ReactionError>;
    fn cancellable(&self) -> bool;
    async fn cancel(&mut self) -> Result<(), ReactionError>;
}

pub struct ReactionRegistry {
    reactions: Arc<RwLock<Vec<Box<dyn Reaction>>>>,
}

impl ReactionRegistry {
    pub fn new() -> Self {
        Self {
            reactions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register(&self, reaction: Box<dyn Reaction>) {
        self.reactions.write().await.push(reaction);
    }

    pub async fn trigger_for_transition(&self, transition: &Transition) {
        let mut reactions = self.reactions.write().await;
        for reaction in reactions.iter_mut() {
            let triggers = reaction.triggers();
            for trigger in &triggers {
                if trigger.matches(transition) {
                    if let Err(e) = reaction.execute(&transition.session_id, transition).await {
                        tracing::error!(
                            reaction_id = reaction.id(),
                            error = %e,
                            "Reaction failed"
                        );
                    }
                    break;
                }
            }
        }
    }

    pub async fn cancel_all(&self) {
        let mut reactions = self.reactions.write().await;
        for reaction in reactions.iter_mut() {
            if reaction.cancellable() {
                let _ = reaction.cancel().await;
            }
        }
    }
}

impl Default for ReactionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
