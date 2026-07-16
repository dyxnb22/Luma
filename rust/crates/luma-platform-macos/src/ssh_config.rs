use luma_application::{ResolvedSshHost, SshConfigError, SshConfigPort, SshConfigState};
use luma_storage::collect_aliases_from_file;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct MacSshConfig {
    config_path: PathBuf,
}

impl MacSshConfig {
    pub fn system_default() -> Self {
        let path = std::env::var("SSH_CONFIG")
            .map(PathBuf::from)
            .ok()
            .or_else(|| dirs::home_dir().map(|h| h.join(".ssh").join("config")))
            .unwrap_or_else(|| PathBuf::from("/dev/null"));
        Self { config_path: path }
    }

    pub fn with_config_path(path: PathBuf) -> Self {
        Self { config_path: path }
    }

    fn read_file(path: &Path) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    }

    fn collect_aliases(&self) -> Result<Vec<String>, SshConfigError> {
        if !self.config_path.exists() {
            return Err(SshConfigError::msg("ssh config not found"));
        }
        collect_aliases_from_file(&self.config_path, &Self::read_file, 0)
            .map_err(SshConfigError::msg)
    }

    fn parse_ssh_g_output(alias: &str, stdout: &str) -> ResolvedSshHost {
        let mut hostname = None;
        let mut user = None;
        let mut port = None;
        let mut identity_file = None;
        let mut proxy_jump = None;
        let mut connect_timeout = None;

        for line in stdout.lines() {
            let line = line.trim();
            let Some((key, value)) = line.split_once(' ') else {
                continue;
            };
            let value = value.trim();
            match key {
                "hostname" if hostname.is_none() => hostname = Some(value.to_string()),
                "user" if user.is_none() => user = Some(value.to_string()),
                "port" if port.is_none() => port = value.parse().ok(),
                "identityfile" if identity_file.is_none() => {
                    identity_file = Some(value.to_string());
                }
                "proxyjump" if proxy_jump.is_none() => proxy_jump = Some(value.to_string()),
                "connecttimeout" if connect_timeout.is_none() => {
                    connect_timeout = value.parse().ok();
                }
                _ => {}
            }
        }

        ResolvedSshHost {
            alias: alias.to_string(),
            hostname,
            user,
            port,
            identity_file,
            proxy_jump,
            connect_timeout,
        }
    }

    fn command_available(name: &str) -> bool {
        Command::new(name)
            .arg("-V")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

impl SshConfigPort for MacSshConfig {
    fn config_state(&self) -> SshConfigState {
        if !self.config_path.exists() {
            return SshConfigState::NotConfigured;
        }
        match self.collect_aliases() {
            Ok(_) => SshConfigState::Found,
            Err(err) => SshConfigState::Unavailable(err.0),
        }
    }

    fn list_aliases(&self) -> Result<Vec<String>, SshConfigError> {
        if !self.config_path.exists() {
            return Err(SshConfigError::msg("ssh config not found"));
        }
        self.collect_aliases()
    }

    fn resolve(&self, alias: &str) -> Result<ResolvedSshHost, SshConfigError> {
        if !self.ssh_available() {
            return Err(SshConfigError::msg("ssh command unavailable"));
        }
        let aliases = self.collect_aliases()?;
        if !aliases.iter().any(|a| a == alias) {
            return Err(SshConfigError::msg(format!(
                "unknown ssh host alias: {alias}"
            )));
        }
        let output = Command::new("ssh")
            .args(["-G", alias])
            .output()
            .map_err(|e| SshConfigError::msg(format!("ssh -G failed: {e}")))?;
        if !output.status.success() {
            return Err(SshConfigError::msg(format!(
                "ssh -G exited with {}",
                output.status
            )));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(Self::parse_ssh_g_output(alias, &stdout))
    }

    fn ssh_available(&self) -> bool {
        Self::command_available("ssh")
    }

    fn sftp_available(&self) -> bool {
        Self::command_available("sftp")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ssh_g_extracts_fields() {
        let stdout = "hostname 203.0.113.10\nuser ubuntu\nport 22\nidentityfile /home/u/.ssh/id_rsa\nproxyjump bastion\nconnecttimeout 30\n";
        let host = MacSshConfig::parse_ssh_g_output("prod", stdout);
        assert_eq!(host.hostname.as_deref(), Some("203.0.113.10"));
        assert_eq!(host.user.as_deref(), Some("ubuntu"));
        assert_eq!(host.port, Some(22));
        assert_eq!(host.identity_file.as_deref(), Some("/home/u/.ssh/id_rsa"));
        assert_eq!(host.proxy_jump.as_deref(), Some("bastion"));
        assert_eq!(host.connect_timeout, Some(30));
    }
}
