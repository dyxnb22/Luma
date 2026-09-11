//! Local TCP listener inspection and guarded SIGTERM. No shell interpolation is used.

use async_trait::async_trait;
use luma_application::{RuntimeError, RuntimeListener, RuntimePort};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_LISTENERS: usize = 200;
const CRITICAL_PROCESSES: &[&str] = &[
    "launchd",
    "kernel_task",
    "loginwindow",
    "WindowServer",
    "systemstats",
];

pub struct MacRuntimeInspector;

impl MacRuntimeInspector {
    async fn run(program: &str, args: &[&str]) -> Result<Vec<u8>, RuntimeError> {
        let output =
            tokio::time::timeout(COMMAND_TIMEOUT, Command::new(program).args(args).output())
                .await
                .map_err(|_| RuntimeError::Timeout)?
                .map_err(|error| RuntimeError::Unavailable(error.to_string()))?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            let detail = String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(240)
                .collect::<String>();
            if detail.to_ascii_lowercase().contains("not permitted")
                || detail.to_ascii_lowercase().contains("permission denied")
            {
                Err(RuntimeError::PermissionRequired(detail))
            } else {
                Err(RuntimeError::Unavailable(detail))
            }
        }
    }

    async fn cwd(pid: u32) -> Option<PathBuf> {
        let pid_text = pid.to_string();
        let output = Self::run(
            "/usr/sbin/lsof",
            &["-a", "-p", &pid_text, "-d", "cwd", "-Fn"],
        )
        .await
        .ok()?;
        String::from_utf8_lossy(&output)
            .lines()
            .find_map(|line| line.strip_prefix('n'))
            .map(PathBuf::from)
            .and_then(|path| std::fs::canonicalize(path).ok())
    }

    async fn current_user() -> Option<String> {
        let output = Self::run("/usr/bin/id", &["-un"]).await.ok()?;
        let user = String::from_utf8_lossy(&output).trim().to_string();
        (!user.is_empty()).then_some(user)
    }
}

#[async_trait]
impl RuntimePort for MacRuntimeInspector {
    async fn list_tcp_listeners(&self) -> Result<Vec<RuntimeListener>, RuntimeError> {
        let output = Self::run(
            "/usr/sbin/lsof",
            &["-nP", "-iTCP", "-sTCP:LISTEN", "-FpcLn"],
        )
        .await?;
        let mut current_pid = None;
        let mut current_name = String::new();
        let mut current_user = None;
        let mut partial = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            if let Some(pid) = line
                .strip_prefix('p')
                .and_then(|value| value.parse::<u32>().ok())
            {
                current_pid = Some(pid);
            } else if let Some(name) = line.strip_prefix('c') {
                current_name = name.into();
            } else if let Some(user) = line.strip_prefix('L') {
                current_user = Some(user.into());
            } else if let Some(endpoint) = line.strip_prefix('n') {
                if let (Some(pid), Some((address, port))) = (current_pid, parse_endpoint(endpoint))
                {
                    partial.push((
                        pid,
                        current_name.clone(),
                        current_user.clone(),
                        address,
                        port,
                    ));
                    if partial.len() == MAX_LISTENERS {
                        break;
                    }
                }
            }
        }
        let mut listeners = Vec::new();
        let mut cwd_by_pid: HashMap<u32, Option<PathBuf>> = HashMap::new();
        for (pid, process_name, user, address, port) in partial {
            let cwd = if let Some(cwd) = cwd_by_pid.get(&pid) {
                cwd.clone()
            } else {
                let cwd = Self::cwd(pid).await;
                cwd_by_pid.insert(pid, cwd.clone());
                cwd
            };
            let identity = format!(
                "{pid}|{process_name}|{}|{}",
                user.clone().unwrap_or_default(),
                cwd.as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default()
            );
            listeners.push(RuntimeListener {
                port,
                address,
                pid,
                process_name,
                user,
                cwd,
                identity,
            });
        }
        listeners.sort_by(|a, b| a.port.cmp(&b.port).then_with(|| a.pid.cmp(&b.pid)));
        Ok(listeners)
    }

    async fn terminate_gracefully(&self, listener: RuntimeListener) -> Result<(), RuntimeError> {
        let current_user = Self::current_user()
            .await
            .ok_or_else(|| RuntimeError::Unavailable("could not determine current user".into()))?;
        if listener.user.as_deref() != Some(current_user.as_str()) {
            return Err(RuntimeError::SecurityDenied(
                "listener belongs to another user".into(),
            ));
        }
        if CRITICAL_PROCESSES
            .iter()
            .any(|name| *name == listener.process_name)
        {
            return Err(RuntimeError::SecurityDenied(
                "system-critical process".into(),
            ));
        }
        let current = self.list_tcp_listeners().await?;
        if !current
            .iter()
            .any(|item| item.pid == listener.pid && item.identity == listener.identity)
        {
            return Err(RuntimeError::NotFound);
        }
        let pid = listener.pid.to_string();
        Self::run("/bin/kill", &["-TERM", &pid]).await.map(|_| ())
    }
}

fn parse_endpoint(value: &str) -> Option<(String, u16)> {
    let endpoint = value.split(" (LISTEN)").next()?.trim();
    let port = endpoint.rsplit(':').next()?.parse().ok()?;
    let address = endpoint.strip_suffix(&format!(":{port}"))?.to_string();
    Some((address, port))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_ipv4_and_ipv6_listener() {
        assert_eq!(
            parse_endpoint("127.0.0.1:3000 (LISTEN)"),
            Some(("127.0.0.1".into(), 3000))
        );
        assert_eq!(
            parse_endpoint("[::1]:8080 (LISTEN)"),
            Some(("[::1]".into(), 8080))
        );
    }
}
