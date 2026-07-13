use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[async_trait]
pub trait ProcessCatalog: Send + Sync {
    async fn list_gui_ish(&self) -> Result<Vec<ProcessEntry>, ProcessError>;
    async fn quit(&self, pid: u32, force: bool) -> Result<(), ProcessError>;
}

pub struct MacProcessCatalog;

#[async_trait]
impl ProcessCatalog for MacProcessCatalog {
    async fn list_gui_ish(&self) -> Result<Vec<ProcessEntry>, ProcessError> {
        let out = Command::new("ps")
            .args(["-axo", "pid=,comm="])
            .output()
            .await?;
        if !out.status.success() {
            return Err(ProcessError::Unavailable("ps failed".into()));
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let mut entries = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let Some(pid_s) = parts.next() else { continue };
            let Ok(pid) = pid_s.parse::<u32>() else {
                continue;
            };
            let name = parts.collect::<Vec<_>>().join(" ");
            if name.is_empty() {
                continue;
            }
            // Prefer app-like names; keep bounded.
            if name.contains(".app/") || !name.starts_with('/') {
                let short = name.rsplit('/').next().unwrap_or(&name).trim().to_string();
                entries.push(ProcessEntry { pid, name: short });
            }
            if entries.len() >= 200 {
                break;
            }
        }
        Ok(entries)
    }

    async fn quit(&self, pid: u32, force: bool) -> Result<(), ProcessError> {
        let signal = if force { "-9" } else { "-15" };
        let status = Command::new("kill")
            .args([signal, &pid.to_string()])
            .status()
            .await?;
        if status.success() {
            Ok(())
        } else {
            Err(ProcessError::Unavailable(format!("kill exited {status}")))
        }
    }
}
