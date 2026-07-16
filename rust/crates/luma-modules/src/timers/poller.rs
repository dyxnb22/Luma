use super::TimersModule;
use luma_application::{
    ClockPort, SpeechAccent, SpeechPort, TimerEntry, TimersRepoError, TimersRepository,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// In-process monotonic deadlines for running countdown segments.
///
/// Wall-clock (`started_at_ms` / `ClockPort`) remains the source of truth for
/// display and persistence; the poller seeds an [`Instant`] deadline once per
/// running segment so NTP / sleep jumps do not skip or reverse remaining time.
#[derive(Default)]
pub(super) struct DeadlineTracker {
    /// timer_id → (segment `started_at_ms`, deadline)
    deadlines: HashMap<String, (i64, Instant)>,
}

impl DeadlineTracker {
    /// Whether a running countdown has reached its monotonic deadline.
    ///
    /// On first observation of a segment, remaining is seeded from wall clock
    /// (so ControllableClock tests that advance past duration still complete).
    pub(super) fn countdown_finished(
        &mut self,
        entry: &TimerEntry,
        wall_now_ms: i64,
        mono_now: Instant,
    ) -> bool {
        if entry.kind != "countdown" || entry.state != "running" || entry.alerted {
            self.deadlines.remove(&entry.id);
            return false;
        }
        let Some(duration) = entry.duration_ms else {
            self.deadlines.remove(&entry.id);
            return false;
        };
        let Some(started) = entry.started_at_ms else {
            self.deadlines.remove(&entry.id);
            return false;
        };

        match self.deadlines.get(&entry.id).copied() {
            Some((seg, deadline)) if seg == started => mono_now >= deadline,
            _ => {
                let remaining = (duration - entry.elapsed_ms(wall_now_ms)).max(0) as u64;
                let deadline = mono_now + Duration::from_millis(remaining);
                self.deadlines.insert(entry.id.clone(), (started, deadline));
                remaining == 0
            }
        }
    }

    pub(super) fn forget(&mut self, id: &str) {
        self.deadlines.remove(id);
    }
}

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
            let mut deadlines = DeadlineTracker::default();
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
                            &mut deadlines,
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

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn tick_completions(
        store: &dyn TimersRepository,
        index: &RwLock<Vec<TimerEntry>>,
        store_error: &RwLock<Option<String>>,
        clock: &dyn ClockPort,
        speech: &dyn SpeechPort,
        generation: u64,
        refresh_generation: &AtomicU64,
        deadlines: &mut DeadlineTracker,
    ) {
        if refresh_generation.load(Ordering::SeqCst) != generation {
            return;
        }
        let Ok(wall_now) = clock.now_unix_ms() else {
            return;
        };
        let mono_now = Instant::now();
        let Ok(rows) = store.list() else {
            return;
        };
        let mut changed = false;
        for mut entry in rows {
            if !deadlines.countdown_finished(&entry, wall_now, mono_now) {
                continue;
            }
            // Teardown bumps generation before stop_poller; never complete/alert after that.
            if refresh_generation.load(Ordering::SeqCst) != generation {
                return;
            }
            let expected = entry.updated_at_ms;
            entry.state = "completed".into();
            entry.accumulated_ms = entry.duration_ms.unwrap_or(entry.accumulated_ms);
            entry.started_at_ms = None;
            entry.alerted = true;
            entry.updated_at_ms = TimerEntry::next_updated_at_ms(wall_now, expected);
            match store.update(&entry, expected) {
                Ok(()) => {
                    deadlines.forget(&entry.id);
                    changed = true;
                }
                Err(TimersRepoError::Conflict) => {
                    // Concurrent pause/mutate won — retry next tick if still due.
                    deadlines.forget(&entry.id);
                    continue;
                }
                Err(_) => continue,
            }
            if refresh_generation.load(Ordering::SeqCst) != generation {
                // Persist may have landed, but do not speak or refresh index across teardown.
                return;
            }
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
            let expected = entry.updated_at_ms;
            entry.accumulated_ms = entry.elapsed_ms(now);
            entry.started_at_ms = None;
            entry.state = "paused".into();
            entry.updated_at_ms = TimerEntry::next_updated_at_ms(now, expected);
            let _ = self.store.update(&entry, expected);
        }
        let _ = self.refresh_index().await;
    }

    pub(super) fn now_ms(&self) -> Result<i64, String> {
        self.clock.now_unix_ms().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod deadline_tests {
    use super::*;

    fn running(duration_ms: i64, accumulated: i64, started: i64) -> TimerEntry {
        TimerEntry {
            id: "tm-1".into(),
            name: "Focus".into(),
            kind: "countdown".into(),
            state: "running".into(),
            duration_ms: Some(duration_ms),
            accumulated_ms: accumulated,
            started_at_ms: Some(started),
            alerted: false,
            created_at_ms: started,
            updated_at_ms: started,
        }
    }

    #[test]
    fn seeds_from_wall_and_honors_instant() {
        let mut tracker = DeadlineTracker::default();
        let entry = running(60_000, 0, 1_000);
        let t0 = Instant::now();
        // 10s wall elapsed → 50s remaining
        assert!(!tracker.countdown_finished(&entry, 11_000, t0));
        // Wall jumps far ahead — monotonic deadline still governs.
        assert!(!tracker.countdown_finished(&entry, 1_000_000, t0 + Duration::from_secs(1)));
        assert!(tracker.countdown_finished(&entry, 1_000_000, t0 + Duration::from_secs(51)));
    }

    #[test]
    fn zero_remaining_on_first_observe_is_finished() {
        let mut tracker = DeadlineTracker::default();
        let entry = running(2_000, 0, 0);
        let t0 = Instant::now();
        assert!(tracker.countdown_finished(&entry, 2_500, t0));
    }

    #[test]
    fn reseeds_when_segment_restarts() {
        let mut tracker = DeadlineTracker::default();
        let mut entry = running(60_000, 0, 1_000);
        let t0 = Instant::now();
        assert!(!tracker.countdown_finished(&entry, 11_000, t0));
        entry.started_at_ms = Some(50_000);
        entry.accumulated_ms = 0;
        // New segment: wall remaining still 60s at t0+1s observation.
        assert!(!tracker.countdown_finished(&entry, 50_000, t0 + Duration::from_secs(1)));
        assert!(!tracker.countdown_finished(&entry, 50_000, t0 + Duration::from_secs(30)));
        assert!(tracker.countdown_finished(&entry, 50_000, t0 + Duration::from_secs(61)));
    }
}
