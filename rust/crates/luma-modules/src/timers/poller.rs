use super::TimersModule;
use luma_application::{ClockPort, SpeechAccent, SpeechPort, TimerEntry, TimersRepository};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

impl TimersModule {
    pub(super) async fn refresh_index(&self) -> Result<(), String> {
        let generation = self.refresh_generation.load(Ordering::SeqCst);
        match self.store.list() {
            Ok(rows) => {
                if self.refresh_generation.load(Ordering::SeqCst) != generation {
                    return Ok(());
                }
                *self.index.write().await = rows;
                *self.store_error.write().await = None;
                Ok(())
            }
            Err(err) => {
                let msg = err.to_string();
                if self.refresh_generation.load(Ordering::SeqCst) == generation {
                    *self.store_error.write().await = Some(msg.clone());
                }
                Err(msg)
            }
        }
    }

    pub(super) async fn start_poller(&self, parent: CancellationToken) {
        self.stop_poller().await;
        let cancel = parent.child_token();
        let store = self.store.clone();
        let index = self.index.clone();
        let store_error = self.store_error.clone();
        let clock = self.clock.clone();
        let speech = self.speech.clone();
        let refresh_generation = self.refresh_generation.clone();
        let generation = refresh_generation.load(Ordering::SeqCst);
        let token = cancel.clone();
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                        Self::tick_completions(
                            store.as_ref(),
                            &index,
                            &store_error,
                            clock.as_ref(),
                            speech.as_ref(),
                            generation,
                            &refresh_generation,
                        ).await;
                    }
                }
            }
        });
        *self.poll_cancel.lock().await = Some(cancel);
        *self.poll_handle.lock().await = Some(handle);
    }

    pub(super) async fn stop_poller(&self) {
        if let Some(cancel) = self.poll_cancel.lock().await.take() {
            cancel.cancel();
        }
        if let Some(handle) = self.poll_handle.lock().await.take() {
            let _ = handle.await;
        }
    }

    pub(super) async fn tick_completions(
        store: &dyn TimersRepository,
        index: &RwLock<Vec<TimerEntry>>,
        store_error: &RwLock<Option<String>>,
        clock: &dyn ClockPort,
        speech: &dyn SpeechPort,
        generation: u64,
        refresh_generation: &AtomicU64,
    ) {
        if refresh_generation.load(Ordering::SeqCst) != generation {
            return;
        }
        let Ok(now) = clock.now_unix_ms() else {
            return;
        };
        let Ok(rows) = store.list() else {
            return;
        };
        let mut changed = false;
        for mut entry in rows {
            if !entry.is_countdown_finished(now) || entry.alerted {
                continue;
            }
            entry.state = "completed".into();
            entry.accumulated_ms = entry.duration_ms.unwrap_or(entry.accumulated_ms);
            entry.started_at_ms = None;
            entry.alerted = true;
            entry.updated_at_ms = now;
            if store.update(&entry).is_err() {
                continue;
            }
            changed = true;
            let phrase = format!("{} done", entry.name);
            let _ = speech.speak(&phrase, SpeechAccent::Default).await;
        }
        if changed {
            if refresh_generation.load(Ordering::SeqCst) != generation {
                return;
            }
            match store.list() {
                Ok(rows) => {
                    if refresh_generation.load(Ordering::SeqCst) == generation {
                        *index.write().await = rows;
                        *store_error.write().await = None;
                    }
                }
                Err(err) => {
                    if refresh_generation.load(Ordering::SeqCst) == generation {
                        *store_error.write().await = Some(err.to_string());
                    }
                }
            }
        }
    }

    /// Freeze running timers so wall-clock does not advance after process exit.
    pub(super) async fn pause_all_running(&self) {
        let Ok(now) = self.clock.now_unix_ms() else {
            return;
        };
        let Ok(rows) = self.store.list() else {
            return;
        };
        for mut entry in rows {
            if entry.state != "running" {
                continue;
            }
            entry.accumulated_ms = entry.elapsed_ms(now);
            entry.started_at_ms = None;
            entry.state = "paused".into();
            entry.updated_at_ms = now;
            let _ = self.store.update(&entry);
        }
        let _ = self.refresh_index().await;
    }

    pub(super) fn now_ms(&self) -> Result<i64, String> {
        self.clock.now_unix_ms().map_err(|e| e.to_string())
    }
}
