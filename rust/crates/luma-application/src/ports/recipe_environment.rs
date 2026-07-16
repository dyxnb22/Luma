use luma_domain::{RecipeVariant, ResolvedCommandStep, StepRunResult, VariantMatch};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathKind {
    File,
    Directory,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct RecipeEnvironmentError(pub String);

impl RecipeEnvironmentError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

pub trait RecipeEnvironmentPort: Send + Sync {
    fn working_directory(&self) -> Result<PathBuf, RecipeEnvironmentError>;
    fn path_exists(&self, base: &Path, relative: &str, kind: PathKind) -> bool;
    fn command_available(&self, name: &str) -> bool;
    fn resolve_cwd(&self, base: &Path, step_cwd: &str) -> Result<PathBuf, RecipeEnvironmentError>;
    fn match_variant(&self, base: &Path, variants: &[RecipeVariant]) -> VariantMatch;
    fn is_luma_repository(&self, base: &Path) -> bool;
}

pub fn select_best_variant(
    env: &dyn RecipeEnvironmentPort,
    base: &Path,
    variants: &[RecipeVariant],
) -> VariantMatch {
    let mut matches: Vec<(usize, RecipeVariant)> = Vec::new();
    for variant in variants {
        if variant_matches(env, base, variant) {
            let specificity = variant.requires_files.len()
                + variant.requires_directories.len()
                + variant.requires_commands.len();
            matches.push((specificity, variant.clone()));
        }
    }
    if matches.is_empty() {
        return VariantMatch::NoMatch;
    }
    matches.sort_by_key(|b| std::cmp::Reverse(b.0));
    VariantMatch::Matched(matches[0].1.clone())
}

fn variant_matches(env: &dyn RecipeEnvironmentPort, base: &Path, variant: &RecipeVariant) -> bool {
    variant
        .requires_files
        .iter()
        .all(|f| env.path_exists(base, f, PathKind::File))
        && variant
            .requires_directories
            .iter()
            .all(|d| env.path_exists(base, d, PathKind::Directory))
        && variant
            .requires_commands
            .iter()
            .all(|c| env.command_available(c))
}

pub fn resolve_steps(
    env: &dyn RecipeEnvironmentPort,
    base: &Path,
    variant: &RecipeVariant,
) -> Result<Vec<ResolvedCommandStep>, RecipeEnvironmentError> {
    variant
        .steps
        .iter()
        .map(|step| {
            Ok(ResolvedCommandStep {
                id: step.id.clone(),
                label: step.label.clone(),
                program: step.program.clone(),
                args: step.args.clone(),
                cwd: env.resolve_cwd(base, &step.cwd)?,
                continue_on_error: step.continue_on_error,
            })
        })
        .collect()
}

pub trait CommandRunnerPort: Send + Sync {
    fn run_step(&self, step: &ResolvedCommandStep, cancel: &CancellationToken) -> StepRunResult;
}
