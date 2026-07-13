//! Markdown directory watch adapter (notify / poll fallback).

use async_trait::async_trait;
use luma_application::MarkdownWatchPort;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::fs_watch::watch_markdown_root;

pub struct MacMarkdownWatcher;

#[async_trait]
impl MarkdownWatchPort for MacMarkdownWatcher {
    async fn watch(&self, root: PathBuf, cancel: CancellationToken, tx: mpsc::Sender<()>) {
        watch_markdown_root(root, cancel, || {
            let tx = tx.clone();
            async move {
                let _ = tx.send(()).await;
            }
        })
        .await;
    }
}
