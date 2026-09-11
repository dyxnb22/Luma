use super::TimersModule;
use luma_application::{ActionOutcome, TimerEntry};
use luma_domain::{FailureKind, SearchItem};

impl TimersModule {
    pub(super) async fn create_and_start(
        &self,
        name: &str,
        kind: &str,
        duration_ms: Option<i64>,
    ) -> Result<TimerEntry, String> {
        let now = self.now_ms()?;
        let entry = TimerEntry {
            id: self.store.new_id(),
            name: name.trim().to_string(),
            kind: kind.into(),
            state: "running".into(),
            duration_ms,
            accumulated_ms: 0,
            started_at_ms: Some(now),
            alerted: false,
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.store.insert(&entry).map_err(|e| e.to_string())?;
        self.refresh_index().await?;
        Ok(entry)
    }

    pub(super) async fn mutate_timer(
        &self,
        id: &str,
        f: impl FnOnce(&mut TimerEntry, i64) -> Result<(), FailureKind>,
    ) -> ActionOutcome {
        let now = match self.now_ms() {
            Ok(n) => n,
            Err(err) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::Unavailable {
                        reason: err,
                        retryable: true,
                    },
                };
            }
        };
        let mut entry = match self.store.get(id) {
            Ok(Some(e)) => e,
            Ok(None) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::NotFound {
                        entity: format!("timer:{id}"),
                    },
                };
            }
            Err(err) => {
                return ActionOutcome::Failed {
                    kind: FailureKind::Io {
                        context: err.to_string(),
                    },
                };
            }
        };
        let expected = entry.updated_at_ms;
        if let Err(kind) = f(&mut entry, now) {
            return ActionOutcome::Failed { kind };
        }
        entry.updated_at_ms = TimerEntry::next_updated_at_ms(now, expected);
        match self.store.update(&entry, expected) {
            Ok(()) => {
                let _ = self.refresh_index().await;
                ActionOutcome::Success {
                    message: Some(format!("{} · {}", entry.name, entry.state)),
                }
            }
            Err(err) if err.is_conflict() => ActionOutcome::Failed {
                kind: FailureKind::Conflict {
                    reason: "timer changed concurrently — retry".into(),
                },
            },
            Err(err) => ActionOutcome::Failed {
                kind: FailureKind::Io {
                    context: err.to_string(),
                },
            },
        }
    }
}

pub(super) fn timer_id_from_item(item: &SearchItem) -> Option<&str> {
    let id = item.id.as_str().strip_prefix("tm:")?;
    if id.is_empty()
        || id.starts_with("create:")
        || id == "manage"
        || id == "unavailable"
        || id == "help"
    {
        None
    } else {
        Some(id)
    }
}

pub(super) fn payload_str(item: &SearchItem, key: &str) -> Option<String> {
    item.action_payload
        .as_ref()
        .and_then(|p| p.get(key))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

pub(super) fn payload_u64(item: &SearchItem, key: &str) -> Option<u64> {
    item.action_payload
        .as_ref()
        .and_then(|p| p.get(key))
        .and_then(|v| v.as_u64())
}
