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

/// System local date via `localtime_r` (Unix).
pub struct SystemClock;

impl ClockPort for SystemClock {
    fn today_ymd(&self) -> Result<String, ClockError> {
        today_ymd_local()
    }
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

#[cfg(unix)]
fn today_ymd_local() -> Result<String, ClockError> {
    use std::mem::MaybeUninit;
    use std::time::{SystemTime, UNIX_EPOCH};

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| ClockError::Unavailable(e.to_string()))?
        .as_secs() as i64;

    // libc layout for macOS/Linux localtime_r
    #[repr(C)]
    struct Tm {
        tm_sec: i32,
        tm_min: i32,
        tm_hour: i32,
        tm_mday: i32,
        tm_mon: i32,
        tm_year: i32,
        tm_wday: i32,
        tm_yday: i32,
        tm_isdst: i32,
        tm_gmtoff: i64,
        tm_zone: *const i8,
    }

    extern "C" {
        fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
    }

    let mut tm = MaybeUninit::<Tm>::uninit();
    let ptr = unsafe { localtime_r(&secs, tm.as_mut_ptr()) };
    if ptr.is_null() {
        return Err(ClockError::Unavailable("localtime_r failed".into()));
    }
    let tm = unsafe { tm.assume_init() };
    let year = tm.tm_year + 1900;
    let month = tm.tm_mon + 1;
    let day = tm.tm_mday;
    if year < 1970 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(ClockError::Unavailable(format!(
            "implausible local date {year}-{month}-{day}"
        )));
    }
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

#[cfg(not(unix))]
fn today_ymd_local() -> Result<String, ClockError> {
    Err(ClockError::Unavailable(
        "local calendar date requires Unix".into(),
    ))
}
