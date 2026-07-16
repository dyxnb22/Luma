use crate::ports::CommandRunnerPort;
use luma_domain::{looks_secret, ResolvedCommandStep, StepRunResult};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

type RunnerCall = (String, String, Vec<String>);

#[derive(Default)]
pub struct FakeCommandRunner {
    pub calls: Arc<Mutex<Vec<RunnerCall>>>,
    exit_codes: Mutex<Vec<i32>>,
}

impl FakeCommandRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_exit_code(&self, code: i32) {
        self.exit_codes.lock().expect("lock").push(code);
    }
}

impl CommandRunnerPort for FakeCommandRunner {
    fn run_step(&self, step: &ResolvedCommandStep, _cancel: &CancellationToken) -> StepRunResult {
        self.calls.lock().expect("lock").push((
            step.id.clone(),
            step.program.clone(),
            step.args.clone(),
        ));
        let code = self.exit_codes.lock().expect("lock").pop().unwrap_or(0);
        StepRunResult {
            step_id: step.id.clone(),
            exit_code: Some(code),
            started: true,
            message: None,
        }
    }
}

/// Filter env output for show-env recipe (used by macOS runner).
pub fn filter_env_output(raw: &str) -> String {
    raw.lines()
        .filter(|line| env_line_allowed(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn env_line_allowed(line: &str) -> bool {
    if looks_secret(line) {
        return false;
    }
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    if looks_secret(value) {
        return false;
    }
    if value.contains("://") && value.contains('@') {
        return false;
    }
    env_key_allowed(key)
}

fn env_key_allowed(key: &str) -> bool {
    let key_lower = key.to_ascii_lowercase();
    if looks_secret(key)
        || key_lower.contains("token")
        || key_lower.contains("secret")
        || key_lower.contains("password")
        || key_lower.contains("credential")
        || key_lower.contains("private")
    {
        return false;
    }
    let key = key.to_ascii_uppercase();
    const EXACT: &[&str] = &[
        "HOME",
        "USER",
        "PATH",
        "PWD",
        "OLDPWD",
        "SHELL",
        "TERM",
        "LANG",
        "LOGNAME",
        "HOSTNAME",
        "TMPDIR",
        "EDITOR",
        "VISUAL",
        "COLORTERM",
        "DISPLAY",
        "SSH_AUTH_SOCK",
        "USER_ZDOTDIR",
        "VIRTUAL_ENV",
        "CONDA_DEFAULT_ENV",
        "GOPATH",
        "GOROOT",
        "RUSTUP_HOME",
        "CARGO_HOME",
        "NODE_ENV",
        "NPM_CONFIG_PREFIX",
    ];
    if EXACT.contains(&key.as_str()) {
        return true;
    }
    const PREFIXES: &[&str] = &[
        "LC_",
        "XDG_",
        "CARGO_",
        "RUST",
        "NODE_",
        "NPM_",
        "PYTHON",
        "HOMEBREW_",
        "LDFLAGS",
        "CPPFLAGS",
        "PKG_CONFIG_",
        "CMAKE_",
        "ANDROID_",
        "JAVA_",
    ];
    PREFIXES.iter().any(|prefix| key.starts_with(prefix))
}

pub fn is_filtered_env_step(step: &ResolvedCommandStep) -> bool {
    if step.id == "env" {
        return step.args.is_empty();
    }
    program_basename(&step.program) == "env" && step.args.is_empty()
}

fn program_basename(program: &str) -> &str {
    program.rsplit('/').next().unwrap_or(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn fake_runner_records_calls() {
        let runner = FakeCommandRunner::new();
        let step = ResolvedCommandStep {
            id: "s1".into(),
            label: "test".into(),
            program: "cargo".into(),
            args: vec!["test".into()],
            cwd: PathBuf::from("/tmp"),
            root: PathBuf::from("/tmp"),
            continue_on_error: false,
        };
        let result = runner.run_step(&step, &CancellationToken::new());
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(runner.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn env_filter_hides_secret_keys_and_values() {
        let raw = "HOME=/Users/me\nAPI_TOKEN=secret\nPATH=/bin\nCARGO_REGISTRY_TOKEN=abc\nDATABASE_URL=postgres://user:pass@host/db";
        let filtered = filter_env_output(raw);
        assert!(filtered.contains("HOME="));
        assert!(filtered.contains("PATH="));
        assert!(!filtered.contains("API_TOKEN"));
        assert!(!filtered.contains("CARGO_REGISTRY_TOKEN"));
        assert!(!filtered.contains("postgres://"));
    }

    #[test]
    fn filtered_env_step_matches_builtin_and_absolute_program() {
        let builtin = ResolvedCommandStep {
            id: "env".into(),
            label: "env".into(),
            program: "env".into(),
            args: vec![],
            cwd: PathBuf::from("/tmp"),
            root: PathBuf::from("/tmp"),
            continue_on_error: false,
        };
        assert!(is_filtered_env_step(&builtin));

        let absolute = ResolvedCommandStep {
            program: "/usr/bin/env".into(),
            ..builtin.clone()
        };
        assert!(is_filtered_env_step(&absolute));

        let with_args = ResolvedCommandStep {
            args: vec!["FOO=bar".into()],
            ..builtin
        };
        assert!(!is_filtered_env_step(&with_args));
    }
}
