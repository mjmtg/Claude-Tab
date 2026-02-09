use async_trait::async_trait;
use claude_tabs_core::event_bus::{Event, EventBus};
use claude_tabs_core::session::SessionStore;
use claude_tabs_core::state_machine::{SessionState, Transition};
use claude_tabs_core::traits::extension::{
    ActivationContext, Extension, ExtensionError, ExtensionManifest,
};
use claude_tabs_core::traits::reaction::{Reaction, ReactionError, ReactionTrigger};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::info;

pub struct AutoSwitchExtension {
    manifest: ExtensionManifest,
}

impl AutoSwitchExtension {
    pub fn new() -> Self {
        Self {
            manifest: ExtensionManifest::new("auto-switch", "Auto Switch")
                .with_description("Countdown timer + auto-switch on permission needed")
                .with_dependencies(vec!["claude-hooks".to_string()]),
        }
    }
}

#[async_trait]
impl Extension for AutoSwitchExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn activate(&mut self, ctx: &mut ActivationContext) -> Result<(), ExtensionError> {
        let countdown_duration = ctx
            .config
            .get_u64("auto_switch.countdown_seconds")
            .await
            .unwrap_or(5);

        let reaction = AutoSwitchReaction::new(
            ctx.event_bus.clone(),
            ctx.session_store.clone(),
            countdown_duration,
        );

        ctx.reaction_registry.register(Box::new(reaction)).await;

        Ok(())
    }

    async fn deactivate(&mut self) -> Result<(), ExtensionError> {
        Ok(())
    }
}

struct AutoSwitchReaction {
    event_bus: Arc<EventBus>,
    session_store: Arc<SessionStore>,
    countdown_seconds: u64,
    cancelled: Arc<RwLock<bool>>,
}

impl AutoSwitchReaction {
    fn new(event_bus: Arc<EventBus>, session_store: Arc<SessionStore>, countdown_seconds: u64) -> Self {
        Self {
            event_bus,
            session_store,
            countdown_seconds,
            cancelled: Arc::new(RwLock::new(false)),
        }
    }
}

#[async_trait]
impl Reaction for AutoSwitchReaction {
    fn id(&self) -> &str {
        "auto-switch-reaction"
    }

    fn triggers(&self) -> Vec<ReactionTrigger> {
        vec![ReactionTrigger::EnterState(SessionState::YourTurn)]
    }

    async fn execute(
        &mut self,
        session_id: &str,
        _transition: &Transition,
    ) -> Result<(), ReactionError> {
        info!(session_id = %session_id, "Starting auto-switch countdown");

        *self.cancelled.write().await = false;

        let duration = self.countdown_seconds;
        let event_bus = self.event_bus.clone();
        let session_store = self.session_store.clone();
        let cancelled = self.cancelled.clone();
        let sid = session_id.to_string();

        tokio::spawn(async move {
            event_bus
                .emit(Event::new(
                    "auto-switch.countdown.started",
                    serde_json::json!({
                        "session_id": sid,
                        "duration": duration,
                    }),
                ))
                .await;

            for i in (0..duration).rev() {
                sleep(Duration::from_secs(1)).await;

                if *cancelled.read().await {
                    event_bus
                        .emit(Event::new(
                            "auto-switch.countdown.cancelled",
                            serde_json::json!({ "session_id": sid }),
                        ))
                        .await;
                    return;
                }

                event_bus
                    .emit(Event::new(
                        "auto-switch.countdown.tick",
                        serde_json::json!({
                            "session_id": sid,
                            "remaining": i,
                        }),
                    ))
                    .await;
            }

            event_bus
                .emit(Event::new(
                    "auto-switch.countdown.completed",
                    serde_json::json!({ "session_id": sid }),
                ))
                .await;

            // Find next session that needs permission and switch
            let sessions = session_store.list().await;
            let next = sessions
                .iter()
                .find(|s| s.state == SessionState::YourTurn && s.id != sid);

            if let Some(next_session) = next {
                session_store.set_active(Some(next_session.id.clone())).await;
                event_bus
                    .emit(Event::new(
                        "session.switch_requested",
                        serde_json::json!({
                            "from": sid,
                            "to": next_session.id,
                        }),
                    ))
                    .await;
            }
        });

        Ok(())
    }

    fn cancellable(&self) -> bool {
        true
    }

    async fn cancel(&mut self) -> Result<(), ReactionError> {
        *self.cancelled.write().await = true;
        Ok(())
    }
}
