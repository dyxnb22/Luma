use super::rows::watch_warning_row;
use super::NotesModule;
use async_trait::async_trait;
use luma_application::{MarkdownWatchPort, NotesIndexError, SearchSink};
use luma_protocol::{Event, SearchItemDto};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// No-op watcher for tests: waits on cancel without emitting change events.
pub(crate) struct NullMarkdownWatcher;

#[async_trait]
impl MarkdownWatchPort for NullMarkdownWatcher {
    async fn watch(&self, _root: PathBuf, cancel: CancellationToken, _tx: mpsc::Sender<()>) {
        cancel.cancelled().await;
    }
}

impl NotesModule {
    pub async fn set_root(&self, root: Option<PathBuf>) {
        self.stop_watch().await;
        self.stop_exclude_rebuild().await;
        *self.root.write().await = root.clone();
        if let Some(root) = root {
            let _ = self.rebuild_index(&root, None).await;
        }
    }

    /// Test helper wrapping [`Self::set_root`].
    pub async fn set_root_for_tests(&self, root: PathBuf) {
        self.set_root(Some(root)).await;
    }

    /// Bridge a Tokio cancellation token into a scan-level atomic flag.
    /// Drop the returned handle (or abort it) when the scan finishes so waiters do not leak.
    pub(super) fn scan_cancel_bridge(
        token: &CancellationToken,
    ) -> (Arc<AtomicBool>, Option<JoinHandle<()>>) {
        let flag = Arc::new(AtomicBool::new(token.is_cancelled()));
        if token.is_cancelled() {
            return (flag, None);
        }
        let flag2 = flag.clone();
        let token = token.clone();
        let handle = tokio::spawn(async move {
            token.cancelled().await;
            flag2.store(true, Ordering::Relaxed);
        });
        (flag, Some(handle))
    }

    pub(super) async fn rebuild_index(
        &self,
        root: &Path,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<(), NotesIndexError> {
        let root = root.to_path_buf();
        let index = self.index.clone();
        let docs = index.document_count()?;
        let fts = index.fts_count().unwrap_or(0);
        // Empty index, or documents/FTS mismatch → authoritative full scan.
        let need_full = docs == 0 || (docs > 0 && (fts == 0 || fts != docs));
        let report = tokio::task::spawn_blocking(move || {
            if need_full {
                index.full_scan(&root, cancel)
            } else {
                index.incremental_check(&root, cancel)
            }
        })
        .await
        .map_err(|e| NotesIndexError::msg(format!("scan join failed: {e}")))??;
        if report.cancelled {
            return Err(NotesIndexError::msg("scan cancelled"));
        }
        let mut gen = self.index_generation.write().await;
        *gen = gen.saturating_add(1);
        Ok(())
    }

    async fn prepend_watch_warning(&self, upserts: &mut Vec<SearchItemDto>) {
        if let Some(msg) = self.watch_warning.lock().await.clone() {
            upserts.insert(0, watch_warning_row(&msg));
        }
    }

    pub(super) async fn emit_results(
        &self,
        sink: &SearchSink,
        mut upserts: Vec<SearchItemDto>,
        removed_ids: Vec<String>,
    ) {
        self.prepend_watch_warning(&mut upserts).await;
        sink.send(Event::ResultsChunk {
            request_id: String::new(),
            sequence: 1,
            upserts,
            removed_ids,
        })
        .await
        .ok();
    }

    pub(super) async fn start_watch(&self, parent: CancellationToken) {
        self.stop_watch().await;
        *self.watch_warning.lock().await = None;
        let Some(root) = self.root.read().await.clone() else {
            return;
        };
        let cancel = parent.child_token();
        let token = cancel.clone();
        let module_root = self.root.clone();
        let index = self.index.clone();
        let generation = self.index_generation.clone();
        let watch_warning = self.watch_warning.clone();
        let (tx, mut rx) = mpsc::channel(8);
        let watcher = self.watcher.clone();
        let handle = tokio::spawn(async move {
            let (flag, mut bridge) = NotesModule::scan_cancel_bridge(&token);
            let debounce = Duration::from_millis(200);
            let mut debounce_until: Option<tokio::time::Instant> = None;
            let mut restart_used = false;

            async fn flush_pending_incremental(
                debounce_until: &mut Option<tokio::time::Instant>,
                module_root: &tokio::sync::RwLock<Option<PathBuf>>,
                root: &Path,
                generation: &tokio::sync::RwLock<u64>,
                index: &Arc<dyn luma_application::NotesIndexRepository>,
                flag: &Arc<AtomicBool>,
            ) {
                if debounce_until.take().is_none() {
                    return;
                }
                let current = module_root.read().await.clone();
                if current.as_deref() != Some(root) {
                    return;
                }
                let gen_before = *generation.read().await;
                let index = index.clone();
                let root_clone = root.to_path_buf();
                let flag = flag.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    index.incremental_check(&root_clone, Some(flag))
                })
                .await;
                let mut gen = generation.write().await;
                if *gen == gen_before {
                    *gen = gen.saturating_add(1);
                }
            }

            loop {
                let watch_fut = watcher.watch(root.clone(), token.clone(), tx.clone());
                tokio::pin!(watch_fut);
                let watch_done = false;
                let mut restart_requested = false;
                loop {
                    let debounce_sleep = async {
                        match debounce_until {
                            Some(deadline) => tokio::time::sleep_until(deadline).await,
                            None => std::future::pending().await,
                        }
                    };
                    tokio::select! {
                        _ = token.cancelled() => {
                            flush_pending_incremental(
                                &mut debounce_until,
                                &module_root,
                                &root,
                                &generation,
                                &index,
                                &flag,
                            )
                            .await;
                            if let Some(b) = bridge.take() {
                                b.abort();
                            }
                            return;
                        }
                        _ = &mut watch_fut, if !watch_done => {
                            if token.is_cancelled() {
                                flush_pending_incremental(
                                    &mut debounce_until,
                                    &module_root,
                                    &root,
                                    &generation,
                                    &index,
                                    &flag,
                                )
                                .await;
                                if let Some(b) = bridge.take() {
                                    b.abort();
                                }
                                return;
                            }
                            if !restart_used {
                                restart_used = true;
                                restart_requested = true;
                                // Keep pending debounce across one-shot restart.
                                break;
                            }
                            flush_pending_incremental(
                                &mut debounce_until,
                                &module_root,
                                &root,
                                &generation,
                                &index,
                                &flag,
                            )
                            .await;
                            *watch_warning.lock().await = Some(
                                "Notes file watcher stopped; Enter to restart, or run `n check`"
                                    .into(),
                            );
                            break;
                        }
                        Some(()) = rx.recv() => {
                            debounce_until = Some(tokio::time::Instant::now() + debounce);
                        }
                        () = debounce_sleep, if debounce_until.is_some() => {
                            flush_pending_incremental(
                                &mut debounce_until,
                                &module_root,
                                &root,
                                &generation,
                                &index,
                                &flag,
                            )
                            .await;
                        }
                    }
                }
                if !restart_requested {
                    break;
                }
            }
            if let Some(b) = bridge.take() {
                b.abort();
            }
        });
        *self.watch_cancel.lock().await = Some(cancel);
        *self.watch_handle.lock().await = Some(handle);
    }

    pub(super) async fn stop_watch(&self) {
        if let Some(cancel) = self.watch_cancel.lock().await.take() {
            cancel.cancel();
        }
        if let Some(handle) = self.watch_handle.lock().await.take() {
            let abort = handle.abort_handle();
            if tokio::time::timeout(Duration::from_secs(2), handle)
                .await
                .is_err()
            {
                abort.abort();
            }
        }
    }
}
