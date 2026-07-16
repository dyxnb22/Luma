use luma_application::{
    filter_env_output, ports::CommandRunnerPort, ports::PathKind, ports::RecipeEnvironmentError,
    ports::RecipeEnvironmentPort, select_best_variant,
};
use luma_domain::{RecipeVariant, ResolvedCommandStep, StepRunResult, VariantMatch};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use tokio_util::sync::CancellationToken;

pub struct MacRecipeEnvironment;

impl MacRecipeEnvironment {
    pub fn new() -> Self {
        Self
    }

    fn join_relative(base: &Path, relative: &str) -> PathBuf {
        let mut out = base.to_path_buf();
        for component in Path::new(relative).components() {
            match component {
                Component::CurDir => {}
                Component::Normal(part) => out.push(part),
                _ => {}
            }
        }
        out
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
        let path = Self::join_relative(base, relative);
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
            Self::join_relative(base, step_cwd)
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
}

impl Default for MacCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRunnerPort for MacCommandRunner {
    fn run_step(&self, step: &ResolvedCommandStep, cancel: &CancellationToken) -> StepRunResult {
        if cancel.is_cancelled() {
            return StepRunResult {
                step_id: step.id.clone(),
                exit_code: None,
                started: false,
                message: Some("cancelled".into()),
            };
        }

        if step.program == "env" && step.args.is_empty() {
            let mut lines = Vec::new();
            for (key, value) in std::env::vars() {
                let line = format!("{key}={value}");
                if filter_env_output(&line).is_empty() {
                    continue;
                }
                lines.push(line);
            }
            lines.sort();
            println!("{}", lines.join("\n"));
            return StepRunResult {
                step_id: step.id.clone(),
                exit_code: Some(0),
                started: true,
                message: None,
            };
        }

        let mut command = Command::new(&step.program);
        command
            .args(&step.args)
            .current_dir(&step.cwd)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        match command.status() {
            Ok(status) => StepRunResult {
                step_id: step.id.clone(),
                exit_code: status.code(),
                started: true,
                message: None,
            },
            Err(err) => StepRunResult {
                step_id: step.id.clone(),
                exit_code: None,
                started: false,
                message: Some(err.to_string()),
            },
        }
    }
}
