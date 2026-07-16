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
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            if looks_secret(line) {
                return false;
            }
            let key = lower.split('=').next().unwrap_or("");
            !(key.contains("password")
                || key.contains("token")
                || key.contains("secret")
                || key.contains("key") && !key.contains("keyboard"))
        })
        .collect::<Vec<_>>()
        .join("\n")
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
            continue_on_error: false,
        };
        let result = runner.run_step(&step, &CancellationToken::new());
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(runner.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn env_filter_hides_secrets() {
        let raw = "HOME=/Users/me\nAPI_TOKEN=secret\nPATH=/bin";
        let filtered = filter_env_output(raw);
        assert!(filtered.contains("HOME="));
        assert!(filtered.contains("PATH="));
        assert!(!filtered.contains("API_TOKEN"));
    }
}
