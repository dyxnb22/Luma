//! Run an interactive subprocess in the current terminal (no shell, inherited IO).

use std::io;
use std::process::{Command, ExitStatus, Stdio};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InteractiveTerminalRequest {
    pub program: String,
    pub args: Vec<String>,
    pub environment: Vec<(String, String)>,
}

#[derive(Debug)]
pub enum InteractiveTerminalError {
    Spawn(io::Error),
}

impl std::fmt::Display for InteractiveTerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(err) => write!(f, "{err}"),
        }
    }
}

impl InteractiveTerminalError {
    pub fn spawn(program: &str, err: io::Error) -> Self {
        let _ = program;
        Self::Spawn(err)
    }
}

/// Spawn `program` with `args` using inherited stdin/stdout/stderr. No shell.
pub fn run_interactive_terminal(
    program: &str,
    args: &[String],
    environment: &[(String, String)],
) -> Result<ExitStatus, InteractiveTerminalError> {
    let mut command = Command::new(program);
    command
        .args(args)
        .envs(environment.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| InteractiveTerminalError::spawn(program, e))
}

/// Build ssh connect args: `ssh -- <alias>` as discrete argv entries.
pub fn ssh_connect_args(alias: &str) -> Vec<String> {
    vec!["--".into(), alias.to_string()]
}

/// Build sftp args: `sftp -- <alias>` as discrete argv entries.
pub fn sftp_args(alias: &str) -> Vec<String> {
    vec!["--".into(), alias.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_args_use_end_of_options_before_alias() {
        assert_eq!(
            ssh_connect_args("production"),
            vec!["--".to_string(), "production".to_string()]
        );
        assert_eq!(
            sftp_args("staging"),
            vec!["--".to_string(), "staging".to_string()]
        );
    }

    #[test]
    fn run_true_exits_zero() {
        if !std::path::Path::new("/bin/true").exists() {
            return;
        }
        let status = run_interactive_terminal("/bin/true", &[], &[]).expect("spawn");
        assert!(status.success());
    }

    #[test]
    fn run_false_exits_nonzero() {
        if !std::path::Path::new("/bin/false").exists() {
            return;
        }
        let status = run_interactive_terminal("/bin/false", &[], &[]).expect("spawn");
        assert!(!status.success());
    }

    #[test]
    fn explicit_environment_is_applied_to_the_child() {
        if !std::path::Path::new("/bin/sh").exists() {
            return;
        }
        let status = run_interactive_terminal(
            "/bin/sh",
            &[
                "-c".into(),
                "test \"$LUMA_INTERACTIVE_TEST\" = exact".into(),
            ],
            &[("LUMA_INTERACTIVE_TEST".into(), "exact".into())],
        )
        .expect("spawn");
        assert!(status.success());
    }
}
