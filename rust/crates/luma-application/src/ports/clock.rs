use std::sync::atomic::{AtomicI64, Ordering};
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
    /// Unix epoch milliseconds (UTC) for elapsed / deadline math.
    fn now_unix_ms(&self) -> Result<i64, ClockError>;
}

/// Fixed date for tests.
pub struct FixedClock {
    pub ymd: String,
    pub now: String,
    pub unix_ms: i64,
}

impl FixedClock {
    pub fn new(ymd: &str, now: &str) -> Self {
        let unix_ms = parse_basic_rfc3339_z_ms(now).unwrap_or(0);
        Self {
            ymd: ymd.into(),
            now: now.into(),
            unix_ms,
        }
    }

    pub fn at_ms(ymd: &str, now: &str, unix_ms: i64) -> Self {
        Self {
            ymd: ymd.into(),
            now: now.into(),
            unix_ms,
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

    fn now_unix_ms(&self) -> Result<i64, ClockError> {
        Ok(self.unix_ms)
    }
}

/// Mutable clock for timer tests (advance wall time without sleeping).
pub struct ControllableClock {
    ymd: String,
    unix_ms: AtomicI64,
}

impl ControllableClock {
    pub fn new(ymd: &str, unix_ms: i64) -> Self {
        Self {
            ymd: ymd.into(),
            unix_ms: AtomicI64::new(unix_ms),
        }
    }

    pub fn set_ms(&self, ms: i64) {
        self.unix_ms.store(ms, Ordering::SeqCst);
    }

    pub fn advance_ms(&self, delta: i64) {
        self.unix_ms.fetch_add(delta, Ordering::SeqCst);
    }

    pub fn ms(&self) -> i64 {
        self.unix_ms.load(Ordering::SeqCst)
    }
}

impl ClockPort for ControllableClock {
    fn today_ymd(&self) -> Result<String, ClockError> {
        Ok(self.ymd.clone())
    }

    fn now_rfc3339(&self) -> Result<String, ClockError> {
        let ms = self.ms();
        let secs = ms.div_euclid(1000);
        Ok(format_unix_secs_rfc3339_z(secs))
    }

    fn now_unix_ms(&self) -> Result<i64, ClockError> {
        Ok(self.ms())
    }
}

/// Parse `YYYY-MM-DDTHH:MM:SSZ` (optional fractional seconds ignored) into unix ms.
pub fn parse_basic_rfc3339_z_ms(s: &str) -> Result<i64, ClockError> {
    let s = s.trim();
    let base = s.split('.').next().unwrap_or(s);
    let base = base.strip_suffix('Z').unwrap_or(base);
    if base.len() < 19 {
        return Err(ClockError::Unavailable(format!("bad rfc3339 {s}")));
    }
    let year: i64 = base[0..4]
        .parse()
        .map_err(|_| ClockError::Unavailable(format!("bad year in {s}")))?;
    let month: i64 = base[5..7]
        .parse()
        .map_err(|_| ClockError::Unavailable(format!("bad month in {s}")))?;
    let day: i64 = base[8..10]
        .parse()
        .map_err(|_| ClockError::Unavailable(format!("bad day in {s}")))?;
    let hour: i64 = base[11..13]
        .parse()
        .map_err(|_| ClockError::Unavailable(format!("bad hour in {s}")))?;
    let min: i64 = base[14..16]
        .parse()
        .map_err(|_| ClockError::Unavailable(format!("bad minute in {s}")))?;
    let sec: i64 = base[17..19]
        .parse()
        .map_err(|_| ClockError::Unavailable(format!("bad second in {s}")))?;
    let days = days_from_civil(year, month, day)?;
    let secs = days * 86_400 + hour * 3_600 + min * 60 + sec;
    Ok(secs * 1000)
}

fn format_unix_secs_rfc3339_z(secs: i64) -> String {
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let tod = secs.rem_euclid(86_400);
    let hour = tod / 3_600;
    let min = (tod % 3_600) / 60;
    let sec = tod % 60;
    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Howard Hinnant civil-from-days / days-from-civil (proleptic Gregorian).
fn days_from_civil(year: i64, month: i64, day: i64) -> Result<i64, ClockError> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(ClockError::Unavailable(format!(
            "implausible date {year}-{month}-{day}"
        )));
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(era * 146_097 + doe - 719_468)
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_round_trips_via_unix_ms() {
        let ms = parse_basic_rfc3339_z_ms("2026-01-01T00:00:00Z").unwrap();
        assert_eq!(
            format_unix_secs_rfc3339_z(ms / 1000),
            "2026-01-01T00:00:00Z"
        );
        let ms2 = parse_basic_rfc3339_z_ms("2026-07-16T12:34:56Z").unwrap();
        assert_eq!(
            format_unix_secs_rfc3339_z(ms2 / 1000),
            "2026-07-16T12:34:56Z"
        );
    }

    #[test]
    fn controllable_advances() {
        let c = ControllableClock::new("2026-01-01", 1_000);
        c.advance_ms(500);
        assert_eq!(c.now_unix_ms().unwrap(), 1_500);
    }
}
