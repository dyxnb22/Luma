use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Watches a notes root for markdown tree changes.
#[async_trait]
pub trait MarkdownWatchPort: Send + Sync {
    /// Send `()` on `tx` when the tree may have changed. Returns when `cancel` fires.
    async fn watch(&self, root: PathBuf, cancel: CancellationToken, tx: mpsc::Sender<()>);
}
