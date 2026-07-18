use std::fs::{File, OpenOptions};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstanceLockError {
    #[error("could not open menu-bar instance lock: {0}")]
    Io(#[from] std::io::Error),
    #[error("another Luma menu-bar instance is already running")]
    AlreadyRunning,
}

pub struct InstanceLock {
    _file: File,
}

impl InstanceLock {
    pub fn acquire(path: &Path) -> Result<Self, InstanceLockError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)?;
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            const LOCK_EX: i32 = 0x2;
            const LOCK_NB: i32 = 0x4;
            unsafe extern "C" {
                fn flock(fd: std::os::fd::RawFd, operation: i32) -> i32;
            }
            if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } != 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    return Err(InstanceLockError::AlreadyRunning);
                }
                return Err(InstanceLockError::Io(error));
            }
        }
        Ok(Self { _file: file })
    }
}
