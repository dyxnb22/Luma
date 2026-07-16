use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClockError {
    #[error("clock unavailable: {0}")]
    Unavailable(String),
}

/// Local calendar date for modules (no subprocesses inside modules).
pub trait ClockPort: Send + Sync {
    /// Returns `YYYY-MM-DD` in the local timezone, or an error (never a silent epoch fallback).
    fn today_ymd(&self) -> Result<String, ClockError>;
    /// Returns an RFC3339 UTC timestamp for connection metadata.
    fn now_rfc3339(&self) -> Result<String, ClockError>;
}

/// Fixed date for tests.
pub struct FixedClock {
    pub ymd: String,
    pub now: String,
}

impl FixedClock {
    pub fn new(ymd: &str, now: &str) -> Self {
        Self {
            ymd: ymd.into(),
            now: now.into(),
        }
    }
}

impl ClockPort for FixedClock {
    fn today_ymd(&self) -> Result<String, ClockError> {
        if self.ymd.len() == 10 && self.ymd.chars().nth(4) == Some('-') {
            Ok(self.ymd.clone())
        } else {
            Err(ClockError::Unavailable(format!(
                "bad fixed date {}",
                self.ymd
            )))
        }
    }

    fn now_rfc3339(&self) -> Result<String, ClockError> {
        if !self.now.is_empty() {
            return Ok(self.now.clone());
        }
        if self.ymd.len() == 10 {
            return Ok(format!("{}T00:00:00Z", self.ymd));
        }
        Err(ClockError::Unavailable("fixed clock now unset".into()))
    }
}
