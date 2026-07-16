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
}

/// Fixed date for tests.
pub struct FixedClock {
    pub ymd: String,
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
}
