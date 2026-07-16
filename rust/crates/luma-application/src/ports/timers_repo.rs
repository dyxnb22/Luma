use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimerEntry {
    pub id: String,
    pub name: String,
    /// `stopwatch` | `countdown`
    pub kind: String,
    /// `idle` | `running` | `paused` | `completed`
    pub state: String,
    pub duration_ms: Option<i64>,
    pub accumulated_ms: i64,
    pub started_at_ms: Option<i64>,
    pub alerted: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl TimerEntry {
    /// Elapsed ms at `now_ms`, including the active running segment.
    pub fn elapsed_ms(&self, now_ms: i64) -> i64 {
        let mut total = self.accumulated_ms.max(0);
        if self.state == "running" {
            if let Some(started) = self.started_at_ms {
                total += (now_ms - started).max(0);
            }
        }
        total
    }

    /// Remaining ms for countdown; `None` for stopwatch.
    pub fn remaining_ms(&self, now_ms: i64) -> Option<i64> {
        let duration = self.duration_ms?;
        Some((duration - self.elapsed_ms(now_ms)).max(0))
    }

    pub fn is_countdown_finished(&self, now_ms: i64) -> bool {
        self.kind == "countdown"
            && self.state == "running"
            && self
                .duration_ms
                .is_some_and(|d| self.elapsed_ms(now_ms) >= d)
    }
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct TimersRepoError(pub String);

impl TimersRepoError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

pub trait TimersRepository: Send + Sync {
    fn list(&self) -> Result<Vec<TimerEntry>, TimersRepoError>;
    fn get(&self, id: &str) -> Result<Option<TimerEntry>, TimersRepoError>;
    fn insert(&self, entry: &TimerEntry) -> Result<(), TimersRepoError>;
    fn update(&self, entry: &TimerEntry) -> Result<(), TimersRepoError>;
    fn delete(&self, id: &str) -> Result<(), TimersRepoError>;
    fn new_id(&self) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running_countdown() -> TimerEntry {
        TimerEntry {
            id: "tm-1".into(),
            name: "Focus".into(),
            kind: "countdown".into(),
            state: "running".into(),
            duration_ms: Some(60_000),
            accumulated_ms: 10_000,
            started_at_ms: Some(100_000),
            alerted: false,
            created_at_ms: 90_000,
            updated_at_ms: 100_000,
        }
    }

    #[test]
    fn elapsed_includes_active_segment() {
        let e = running_countdown();
        assert_eq!(e.elapsed_ms(120_000), 30_000);
        assert_eq!(e.remaining_ms(120_000), Some(30_000));
        assert!(!e.is_countdown_finished(120_000));
        assert!(e.is_countdown_finished(160_000));
    }
}
