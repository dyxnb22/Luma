//! Run an interactive subprocess in the current terminal (no shell, inherited IO).

use std::io;
use std::process::{Command, ExitStatus, Stdio};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InteractiveTerminalRequest {
    pub program: String,
    pub args: Vec<String>,
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
) -> Result<ExitStatus, InteractiveTerminalError> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| InteractiveTerminalError::spawn(program, e))
}

/// Build ssh connect args: `ssh <alias>` as discrete argv entries.
pub fn ssh_connect_args(alias: &str) -> Vec<String> {
    vec![alias.to_string()]
}

/// Build sftp args: `sftp <alias>` as discrete argv entries.
pub fn sftp_args(alias: &str) -> Vec<String> {
    vec![alias.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_args_are_alias_only() {
        assert_eq!(ssh_connect_args("production"), vec!["production"]);
        assert_eq!(sftp_args("staging"), vec!["staging"]);
    }

    #[test]
    fn run_true_exits_zero() {
        let status = run_interactive_terminal("/bin/true", &[]).expect("spawn");
        assert!(status.success());
    }

    #[test]
    fn run_false_exits_nonzero() {
        let status = run_interactive_terminal("/bin/false", &[]).expect("spawn");
        assert!(!status.success());
    }
}
