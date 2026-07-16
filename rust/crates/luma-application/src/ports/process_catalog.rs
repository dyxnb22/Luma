//! Listening TCP ports + process kill — platform I/O stays behind this port.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KillSignal {
    Term,
    Kill,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListeningEndpoint {
    pub port: u16,
    pub address: String,
    pub protocol: String,
    pub pid: u32,
    pub process_name: String,
    pub command_line: Option<String>,
    pub user: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProcessCatalogError {
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("permission required ({capability}): {guidance}")]
    PermissionRequired {
        capability: String,
        guidance: String,
    },
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input ({field}): {message}")]
    InvalidInput { field: String, message: String },
    #[error("kill failed (pid {pid}): {reason}")]
    KillFailed { pid: u32, reason: String },
}

#[async_trait]
pub trait ProcessCatalogPort: Send + Sync {
    /// Probe whether listing tools are present (e.g. `lsof`).
    fn probe(&self) -> Result<(), ProcessCatalogError>;

    fn list_listening(&self) -> Result<Vec<ListeningEndpoint>, ProcessCatalogError>;

    fn kill(&self, pid: u32, signal: KillSignal) -> Result<(), ProcessCatalogError>;
}

/// Deterministic fake for module / engine tests. Never touches real processes.
#[derive(Default)]
pub struct FakeProcessCatalog {
    pub endpoints: Mutex<Vec<ListeningEndpoint>>,
    pub probe_error: Mutex<Option<ProcessCatalogError>>,
    pub list_error: Mutex<Option<ProcessCatalogError>>,
    pub kill_error: Mutex<Option<ProcessCatalogError>>,
    pub kill_calls: Mutex<Vec<(u32, KillSignal)>>,
    pub self_pid: Mutex<u32>,
}

impl FakeProcessCatalog {
    pub fn new() -> Self {
        Self {
            self_pid: Mutex::new(std::process::id()),
            ..Self::default()
        }
    }

    pub fn with_endpoints(endpoints: Vec<ListeningEndpoint>) -> Self {
        let fake = Self::new();
        *fake.endpoints.lock().expect("lock") = endpoints;
        fake
    }
}

#[async_trait]
impl ProcessCatalogPort for FakeProcessCatalog {
    fn probe(&self) -> Result<(), ProcessCatalogError> {
        if let Some(err) = self.probe_error.lock().expect("lock").clone() {
            return Err(err);
        }
        Ok(())
    }

    fn list_listening(&self) -> Result<Vec<ListeningEndpoint>, ProcessCatalogError> {
        if let Some(err) = self.list_error.lock().expect("lock").clone() {
            return Err(err);
        }
        Ok(self.endpoints.lock().expect("lock").clone())
    }

    fn kill(&self, pid: u32, signal: KillSignal) -> Result<(), ProcessCatalogError> {
        if pid == 0 || pid == 1 {
            return Err(ProcessCatalogError::InvalidInput {
                field: "pid".into(),
                message: "refusing to signal system process".into(),
            });
        }
        let self_pid = *self.self_pid.lock().expect("lock");
        if pid == self_pid {
            return Err(ProcessCatalogError::InvalidInput {
                field: "pid".into(),
                message: "refusing to kill the Luma process".into(),
            });
        }
        if let Some(err) = self.kill_error.lock().expect("lock").clone() {
            return Err(err);
        }
        self.kill_calls.lock().expect("lock").push((pid, signal));
        let mut endpoints = self.endpoints.lock().expect("lock");
        endpoints.retain(|e| e.pid != pid);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_kill_removes_endpoints_and_refuses_self() {
        let fake = FakeProcessCatalog::with_endpoints(vec![ListeningEndpoint {
            port: 3000,
            address: "127.0.0.1".into(),
            protocol: "tcp".into(),
            pid: 4242,
            process_name: "node".into(),
            command_line: Some("node server.js".into()),
            user: Some("me".into()),
        }]);
        *fake.self_pid.lock().unwrap() = 9999;
        fake.kill(4242, KillSignal::Term).unwrap();
        assert!(fake.endpoints.lock().unwrap().is_empty());
        assert_eq!(
            fake.kill(9999, KillSignal::Term),
            Err(ProcessCatalogError::InvalidInput {
                field: "pid".into(),
                message: "refusing to kill the Luma process".into(),
            })
        );
    }
}
