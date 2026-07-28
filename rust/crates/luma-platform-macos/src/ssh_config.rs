use luma_application::{ResolvedSshHost, SshConfigError, SshConfigPort, SshConfigState};
use luma_storage::collect_aliases_from_file;
use std::path::{Path, PathBuf};
use std::process::Command;

const SSH_EXECUTABLE: &str = "/usr/bin/ssh";
const SFTP_EXECUTABLE: &str = "/usr/bin/sftp";

pub struct MacSshConfig {
    config_path: PathBuf,
    explicit_config: bool,
}

impl MacSshConfig {
    pub fn system_default() -> Self {
        if let Some(path) = std::env::var_os("SSH_CONFIG").map(PathBuf::from) {
            return Self {
                config_path: path,
                explicit_config: true,
            };
        }
        Self {
            config_path: dirs::home_dir()
                .map(|h| h.join(".ssh").join("config"))
                .unwrap_or_else(|| PathBuf::from("/dev/null")),
            explicit_config: false,
        }
    }

    pub fn with_config_path(path: PathBuf) -> Self {
        Self {
            config_path: path,
            explicit_config: true,
        }
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
        let Ok(metadata) = std::fs::metadata(name) else {
            return false;
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            metadata.is_file()
        }
    }

    fn invocation_args(&self, alias: &str) -> Vec<String> {
        let mut args = Vec::with_capacity(if self.explicit_config { 4 } else { 2 });
        if self.explicit_config {
            args.push("-F".into());
            args.push(self.config_path.to_string_lossy().into_owned());
        }
        args.push("--".into());
        args.push(alias.into());
        args
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
        if alias.trim().starts_with('-') {
            return Err(SshConfigError::msg(format!(
                "refusing ssh host alias that looks like a flag: {alias}"
            )));
        }
        let output = Command::new(SSH_EXECUTABLE)
            .arg("-G")
            .args(self.invocation_args(alias))
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
        Self::command_available(SSH_EXECUTABLE)
    }

    fn sftp_available(&self) -> bool {
        Self::command_available(SFTP_EXECUTABLE)
    }

    fn ssh_invocation_args(&self, alias: &str) -> Vec<String> {
        self.invocation_args(alias)
    }

    fn sftp_invocation_args(&self, alias: &str) -> Vec<String> {
        self.invocation_args(alias)
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

    #[test]
    fn explicit_config_is_used_for_resolution_and_interactive_connections() {
        let config = MacSshConfig::with_config_path(PathBuf::from("/tmp/luma ssh/config"));
        let expected = vec![
            "-F".to_string(),
            "/tmp/luma ssh/config".to_string(),
            "--".to_string(),
            "local-test".to_string(),
        ];
        assert_eq!(config.invocation_args("local-test"), expected);
        assert_eq!(config.ssh_invocation_args("local-test"), expected);
        assert_eq!(config.sftp_invocation_args("local-test"), expected);
    }

    #[test]
    fn default_config_keeps_openssh_system_config_layer() {
        let config = MacSshConfig {
            config_path: PathBuf::from("/Users/test/.ssh/config"),
            explicit_config: false,
        };
        assert_eq!(
            config.invocation_args("production"),
            vec!["--".to_string(), "production".to_string()]
        );
    }

    #[test]
    fn resolve_reads_the_same_explicit_config_that_will_be_connected() {
        if !Path::new(SSH_EXECUTABLE).is_file() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("isolated-config");
        std::fs::write(
            &path,
            "Host local-test\n  HostName 127.0.0.9\n  User luma-e2e\n  Port 22222\n",
        )
        .unwrap();
        let config = MacSshConfig::with_config_path(path.clone());

        let resolved = config.resolve("local-test").unwrap();

        assert_eq!(resolved.hostname.as_deref(), Some("127.0.0.9"));
        assert_eq!(resolved.user.as_deref(), Some("luma-e2e"));
        assert_eq!(resolved.port, Some(22222));
        assert_eq!(
            config.ssh_invocation_args("local-test"),
            vec![
                "-F".to_string(),
                path.to_string_lossy().into_owned(),
                "--".to_string(),
                "local-test".to_string(),
            ]
        );
    }

    #[test]
    fn command_availability_checks_executable_files_not_version_exit_codes() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("sftp");
        std::fs::write(&executable, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(MacSshConfig::command_available(
            executable.to_str().unwrap()
        ));

        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(!MacSshConfig::command_available(
            executable.to_str().unwrap()
        ));
        assert!(!MacSshConfig::command_available(
            dir.path().join("missing").to_str().unwrap()
        ));
    }
}
