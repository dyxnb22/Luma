use luma_application::{
    filter_env_output, is_filtered_env_step, select_best_variant, CommandRunnerPort, PathKind,
    RecipeEnvironmentError, RecipeEnvironmentPort, RecipeStdioMode,
};
use luma_domain::{RecipeVariant, ResolvedCommandStep, StepRunResult, VariantMatch};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct MacRecipeEnvironment;

impl MacRecipeEnvironment {
    pub fn new() -> Self {
        Self
    }

    fn join_relative(base: &Path, relative: &str) -> Result<PathBuf, RecipeEnvironmentError> {
        let mut out = base.to_path_buf();
        for component in Path::new(relative).components() {
            match component {
                Component::CurDir => {}
                Component::Normal(part) => out.push(part),
                Component::ParentDir => {
                    return Err(RecipeEnvironmentError::msg(format!(
                        "cwd denied (parent dir): {relative}"
                    )));
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(RecipeEnvironmentError::msg(format!(
                        "cwd denied (absolute): {relative}"
                    )));
                }
            }
        }
        Ok(out)
    }

    fn contained_in_base(base: &Path, path: &Path) -> bool {
        let Ok(base_canon) = std::fs::canonicalize(base) else {
            return false;
        };
        let Ok(path_canon) = std::fs::canonicalize(path) else {
            return false;
        };
        path_canon.starts_with(&base_canon)
    }

    fn safe_metadata(path: &Path, want_dir: bool) -> bool {
        match std::fs::symlink_metadata(path) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return false;
                }
                if want_dir {
                    meta.is_dir()
                } else {
                    meta.is_file()
                }
            }
            Err(_) => false,
        }
    }
}

impl Default for MacRecipeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl RecipeEnvironmentPort for MacRecipeEnvironment {
    fn working_directory(&self) -> Result<PathBuf, RecipeEnvironmentError> {
        std::env::current_dir().map_err(|e| RecipeEnvironmentError::msg(e.to_string()))
    }

    fn path_exists(&self, base: &Path, relative: &str, kind: PathKind) -> bool {
        let Ok(path) = Self::join_relative(base, relative) else {
            return false;
        };
        match kind {
            PathKind::File => Self::safe_metadata(&path, false),
            PathKind::Directory => Self::safe_metadata(&path, true),
        }
    }

    fn command_available(&self, name: &str) -> bool {
        if name.contains('/') {
            return false;
        }
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let candidate = dir.join(name);
                if Self::safe_metadata(&candidate, false) {
                    return true;
                }
            }
        }
        false
    }

    fn resolve_cwd(&self, base: &Path, step_cwd: &str) -> Result<PathBuf, RecipeEnvironmentError> {
        let path = if step_cwd == "current" {
            base.to_path_buf()
        } else {
            Self::join_relative(base, step_cwd)?
        };
        if std::fs::symlink_metadata(&path)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(true)
        {
            return Err(RecipeEnvironmentError::msg(format!(
                "cwd denied (symlink): {}",
                path.display()
            )));
        }
        if !path.is_dir() {
            return Err(RecipeEnvironmentError::msg(format!(
                "cwd not found: {}",
                path.display()
            )));
        }
        if !Self::contained_in_base(base, &path) {
            return Err(RecipeEnvironmentError::msg(format!(
                "cwd outside project: {}",
                path.display()
            )));
        }
        Ok(path)
    }

    fn match_variant(&self, base: &Path, variants: &[RecipeVariant]) -> VariantMatch {
        select_best_variant(self, base, variants)
    }

    fn is_luma_repository(&self, base: &Path) -> bool {
        self.path_exists(base, "AGENTS.md", PathKind::File)
            && self.path_exists(base, "rust/Cargo.toml", PathKind::File)
    }
}

pub struct MacCommandRunner;

impl MacCommandRunner {
    pub fn new() -> Self {
        Self
    }

    fn validate_step_cwd(step: &ResolvedCommandStep) -> Result<(), String> {
        if std::fs::symlink_metadata(&step.cwd)
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(true)
        {
            return Err(format!("cwd denied (symlink): {}", step.cwd.display()));
        }
        if !step.cwd.is_dir() {
            return Err(format!("cwd not found: {}", step.cwd.display()));
        }
        if !MacRecipeEnvironment::contained_in_base(&step.root, &step.cwd) {
            return Err(format!("cwd outside project: {}", step.cwd.display()));
        }
        Ok(())
    }
}

impl Default for MacCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRunnerPort for MacCommandRunner {
    fn run_step(
        &self,
        step: &ResolvedCommandStep,
        cancel: &CancellationToken,
        stdio: RecipeStdioMode,
    ) -> StepRunResult {
        if cancel.is_cancelled() {
            return StepRunResult {
                step_id: step.id.clone(),
                exit_code: None,
                started: false,
                cancelled: true,
                message: Some("cancelled".into()),
            };
        }

        if let Err(message) = Self::validate_step_cwd(step) {
            return StepRunResult {
                step_id: step.id.clone(),
                exit_code: None,
                started: false,
                cancelled: false,
                message: Some(message),
            };
        }

        if is_filtered_env_step(step) {
            let mut lines = Vec::new();
            for (key, value) in std::env::vars() {
                let line = format!("{key}={value}");
                if filter_env_output(&line).is_empty() {
                    continue;
                }
                lines.push(line);
            }
            lines.sort();
            match stdio {
                RecipeStdioMode::Inherit => println!("{}", lines.join("\n")),
                RecipeStdioMode::Null => {}
            }
            return StepRunResult {
                step_id: step.id.clone(),
                exit_code: Some(0),
                started: true,
                cancelled: false,
                message: None,
            };
        }

        let mut command = Command::new(&step.program);
        command.args(&step.args).current_dir(&step.cwd);
        match stdio {
            RecipeStdioMode::Inherit => {
                command
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit());
            }
            RecipeStdioMode::Null => {
                command
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
            }
        }

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                return StepRunResult {
                    step_id: step.id.clone(),
                    exit_code: None,
                    started: false,
                    cancelled: false,
                    message: Some(err.to_string()),
                };
            }
        };

        loop {
            if cancel.is_cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                return StepRunResult {
                    step_id: step.id.clone(),
                    exit_code: None,
                    started: true,
                    cancelled: true,
                    message: Some("cancelled".into()),
                };
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    let exit_code = status.code();
                    let cancelled = exit_code.is_none();
                    return StepRunResult {
                        step_id: step.id.clone(),
                        exit_code,
                        started: true,
                        cancelled,
                        message: cancelled.then(|| "terminated by signal".into()),
                    };
                }
                Ok(None) => thread::sleep(Duration::from_millis(50)),
                Err(err) => {
                    return StepRunResult {
                        step_id: step.id.clone(),
                        exit_code: None,
                        started: true,
                        cancelled: false,
                        message: Some(err.to_string()),
                    };
                }
            }
        }
    }
}
