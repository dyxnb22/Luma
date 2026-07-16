//! macOS local-calendar adapter for the application clock port.

use luma_application::{ClockError, ClockPort};

/// Local calendar date backed by the host's `localtime_r` implementation.
pub struct MacClock;

impl ClockPort for MacClock {
    fn today_ymd(&self) -> Result<String, ClockError> {
        today_ymd_local()
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

    // libc layout for macOS/Linux localtime_r.
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

    unsafe extern "C" {
        fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
    }

    let mut tm = MaybeUninit::<Tm>::uninit();
    let ptr = unsafe { localtime_r(&secs, tm.as_mut_ptr()) };
    if ptr.is_null() {
        return Err(ClockError::Unavailable("localtime_r failed".into()));
    }
    let tm = unsafe { tm.assume_init() };
    format_local_date(tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday)
}

#[cfg(not(unix))]
fn today_ymd_local() -> Result<String, ClockError> {
    Err(ClockError::Unavailable(
        "local calendar date requires Unix".into(),
    ))
}

fn format_local_date(year: i32, month: i32, day: i32) -> Result<String, ClockError> {
    if year < 1970 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(ClockError::Unavailable(format!(
            "implausible local date {year}-{month}-{day}"
        )));
    }
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_valid_date() {
        assert_eq!(format_local_date(2026, 7, 16).unwrap(), "2026-07-16");
    }

    #[test]
    fn rejects_implausible_date() {
        assert!(format_local_date(1969, 12, 31).is_err());
        assert!(format_local_date(2026, 13, 1).is_err());
        assert!(format_local_date(2026, 1, 32).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn reads_host_local_date() {
        let ymd = MacClock.today_ymd().unwrap();
        assert_eq!(ymd.len(), 10);
        assert_eq!(&ymd[4..5], "-");
        assert_eq!(&ymd[7..8], "-");
    }
}
