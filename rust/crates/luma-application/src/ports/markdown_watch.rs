use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Watches a notes root for markdown tree changes.
#[async_trait]
pub trait MarkdownWatchPort: Send + Sync {
    /// Send `()` on `tx` when the tree may have changed. Returns when `cancel` fires.
    async fn watch(&self, root: PathBuf, cancel: CancellationToken, tx: mpsc::Sender<()>);
}

/// Test double: optionally emits one change event then waits for cancel.
pub struct FakeMarkdownWatcher {
    pub emit_on_start: bool,
    /// When true, `watch` returns immediately (simulates watcher crash).
    pub exit_immediately: bool,
    pub exited: Arc<AtomicBool>,
}

impl Default for FakeMarkdownWatcher {
    fn default() -> Self {
        Self {
            emit_on_start: false,
            exit_immediately: false,
            exited: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl MarkdownWatchPort for FakeMarkdownWatcher {
    async fn watch(&self, _root: PathBuf, cancel: CancellationToken, tx: mpsc::Sender<()>) {
        if self.emit_on_start {
            let _ = tx.send(()).await;
        }
        if self.exit_immediately {
            return;
        }
        cancel.cancelled().await;
        self.exited.store(true, Ordering::SeqCst);
    }
}
