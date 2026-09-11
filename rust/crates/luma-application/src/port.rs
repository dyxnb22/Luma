use async_trait::async_trait;
use luma_protocol::{Command, Event};
use tokio::sync::broadcast;

/// Port used by TUI and non-interactive CLI. Phase 2 is in-process only.
#[async_trait]
pub trait EnginePort: Send + Sync {
    async fn submit(&self, command: Command) -> Result<(), String>;
    fn subscribe(&self) -> broadcast::Receiver<Event>;
}
