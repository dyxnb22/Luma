//! macOS listening-port catalog via `lsof` + `kill`.
//!
//! Never shells through `sh -c`. Arguments are always argv-separated.
//! Tests must use [`luma_application::FakeProcessCatalog`].

use luma_application::{KillSignal, ListeningEndpoint, ProcessCatalogError, ProcessCatalogPort};
use std::collections::BTreeMap;
use std::process::{Command, Stdio};

pub struct MacProcessCatalog;

impl MacProcessCatalog {
    pub fn new() -> Self {
        Self
    }

    fn command_available(name: &str) -> bool {
        Command::new(name)
            .arg("-h")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    fn run_lsof() -> Result<String, ProcessCatalogError> {
        let output = Command::new("lsof")
            .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-FpcPn"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| ProcessCatalogError::Unavailable(format!("failed to run lsof: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // lsof exits 1 when there are no matching sockets — treat as empty.
            if output.stdout.is_empty() {
                return Ok(String::new());
            }
            return Err(ProcessCatalogError::Unavailable(format!(
                "lsof failed: {}",
                stderr.trim()
            )));
        }
        String::from_utf8(output.stdout)
            .map_err(|e| ProcessCatalogError::Unavailable(format!("lsof output not utf-8: {e}")))
    }

    /// Parse `lsof -F` field output into listening endpoints.
    pub fn parse_lsof_fields(stdout: &str) -> Vec<ListeningEndpoint> {
        let mut current_pid: Option<u32> = None;
        let mut current_name: Option<String> = None;
        let mut current_cmd: Option<String> = None;
        let mut endpoints = Vec::new();

        for line in stdout.lines() {
            if line.is_empty() {
                continue;
            }
            let (tag, value) = line.split_at(1);
            match tag {
                "p" => {
                    current_pid = value.parse().ok();
                    current_name = None;
                    current_cmd = None;
                }
                "c" => {
                    current_name = Some(value.to_string());
                    current_cmd = Some(value.to_string());
                }
                "P" => {
                    // protocol — we filter TCP via argv; ignore
                }
                "n" => {
                    let Some(pid) = current_pid else {
                        continue;
                    };
                    let Some((address, port)) = parse_listen_name(value) else {
                        continue;
                    };
                    let process_name = current_name.clone().unwrap_or_else(|| "?".into());
                    endpoints.push(ListeningEndpoint {
                        port,
                        address,
                        protocol: "tcp".into(),
                        pid,
                        process_name: process_name.clone(),
                        command_line: current_cmd.clone().or(Some(process_name)),
                        user: None,
                    });
                }
                _ => {}
            }
        }

        dedupe_endpoints(endpoints)
    }
}

impl Default for MacProcessCatalog {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_listen_name(name: &str) -> Option<(String, u16)> {
    // Examples: `*:3000`, `127.0.0.1:8080`, `[::1]:5173`
    if let Some(rest) = name.strip_prefix('[') {
        let (host, port_part) = rest.split_once("]:")?;
        let port = port_part.parse().ok()?;
        return Some((host.to_string(), port));
    }
    let (host, port_part) = name.rsplit_once(':')?;
    let port = port_part.parse().ok()?;
    Some((host.to_string(), port))
}

fn dedupe_endpoints(endpoints: Vec<ListeningEndpoint>) -> Vec<ListeningEndpoint> {
    let mut by_key: BTreeMap<(u16, u32, String), ListeningEndpoint> = BTreeMap::new();
    for ep in endpoints {
        by_key.insert((ep.port, ep.pid, ep.address.clone()), ep);
    }
    by_key.into_values().collect()
}

impl ProcessCatalogPort for MacProcessCatalog {
    fn probe(&self) -> Result<(), ProcessCatalogError> {
        if !Self::command_available("lsof") {
            return Err(ProcessCatalogError::Unavailable(
                "lsof not found — install Xcode CLT or ensure /usr/sbin/lsof is on PATH".into(),
            ));
        }
        if !Self::command_available("kill") {
            return Err(ProcessCatalogError::Unavailable(
                "kill not found on PATH".into(),
            ));
        }
        Ok(())
    }

    fn list_listening(&self) -> Result<Vec<ListeningEndpoint>, ProcessCatalogError> {
        self.probe()?;
        let stdout = Self::run_lsof()?;
        Ok(Self::parse_lsof_fields(&stdout))
    }

    fn kill(&self, pid: u32, signal: KillSignal) -> Result<(), ProcessCatalogError> {
        if pid == 0 || pid == 1 {
            return Err(ProcessCatalogError::InvalidInput {
                field: "pid".into(),
                message: "refusing to signal system process".into(),
            });
        }
        if pid == std::process::id() {
            return Err(ProcessCatalogError::InvalidInput {
                field: "pid".into(),
                message: "refusing to kill the Luma process".into(),
            });
        }
        self.probe()?;
        let sig = match signal {
            KillSignal::Term => "-TERM",
            KillSignal::Kill => "-KILL",
        };
        let output = Command::new("kill")
            .args([sig, &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| ProcessCatalogError::KillFailed {
                pid,
                reason: e.to_string(),
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let reason = stderr.trim();
            if reason.contains("Operation not permitted") || reason.contains("permission") {
                return Err(ProcessCatalogError::PermissionRequired {
                    capability: "process_signal".into(),
                    guidance: format!(
                        "Cannot signal pid {pid}. Run Luma as a user that owns the process, or use Activity Monitor."
                    ),
                });
            }
            if reason.contains("No such process") {
                return Err(ProcessCatalogError::NotFound(format!("pid {pid}")));
            }
            return Err(ProcessCatalogError::KillFailed {
                pid,
                reason: if reason.is_empty() {
                    format!("kill exited {}", output.status)
                } else {
                    reason.to_string()
                },
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lsof_field_output() {
        let raw = "\
p4242
cnode
PTCP
n*:3000
p5151
cPython
PTCP
n127.0.0.1:8080
n[::1]:8080
";
        let eps = MacProcessCatalog::parse_lsof_fields(raw);
        assert_eq!(eps.len(), 3);
        assert!(eps
            .iter()
            .any(|e| e.port == 3000 && e.process_name == "node"));
        assert!(eps
            .iter()
            .any(|e| e.port == 8080 && e.address == "127.0.0.1"));
        assert!(eps.iter().any(|e| e.port == 8080 && e.address == "::1"));
    }

    #[test]
    fn parse_listen_name_variants() {
        assert_eq!(parse_listen_name("*:5173"), Some(("*".into(), 5173)));
        assert_eq!(parse_listen_name("[::1]:443"), Some(("::1".into(), 443)));
    }
}
