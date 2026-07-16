use crate::ports::{PathKind, RecipeEnvironmentError, RecipeEnvironmentPort};
use luma_domain::{RecipeVariant, VariantMatch};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct FakeRecipeEnvironment {
    pub working_dir: PathBuf,
    files: Arc<Mutex<BTreeSet<PathBuf>>>,
    directories: Arc<Mutex<BTreeSet<PathBuf>>>,
    commands: Arc<Mutex<BTreeSet<String>>>,
    luma_repo: bool,
}

impl FakeRecipeEnvironment {
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
            ..Default::default()
        }
    }

    pub fn add_file(&self, relative: impl Into<PathBuf>) {
        self.files.lock().expect("lock").insert(relative.into());
    }

    pub fn add_directory(&self, relative: impl Into<PathBuf>) {
        self.directories
            .lock()
            .expect("lock")
            .insert(relative.into());
    }

    pub fn add_command(&self, name: impl Into<String>) {
        self.commands.lock().expect("lock").insert(name.into());
    }

    pub fn set_luma_repository(mut self, yes: bool) -> Self {
        self.luma_repo = yes;
        self
    }

    fn join_relative(&self, base: &Path, relative: &str) -> PathBuf {
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
}

impl RecipeEnvironmentPort for FakeRecipeEnvironment {
    fn working_directory(&self) -> Result<PathBuf, RecipeEnvironmentError> {
        Ok(self.working_dir.clone())
    }

    fn path_exists(&self, base: &Path, relative: &str, kind: PathKind) -> bool {
        let path = self.join_relative(base, relative);
        match kind {
            PathKind::File => self.files.lock().expect("lock").contains(&path),
            PathKind::Directory => self.directories.lock().expect("lock").contains(&path),
        }
    }

    fn command_available(&self, name: &str) -> bool {
        self.commands.lock().expect("lock").contains(name)
    }

    fn resolve_cwd(&self, base: &Path, step_cwd: &str) -> Result<PathBuf, RecipeEnvironmentError> {
        if step_cwd == "current" {
            return Ok(base.to_path_buf());
        }
        Ok(self.join_relative(base, step_cwd))
    }

    fn match_variant(&self, base: &Path, variants: &[RecipeVariant]) -> VariantMatch {
        super::recipe_environment::select_best_variant(self, base, variants)
    }

    fn is_luma_repository(&self, base: &Path) -> bool {
        self.luma_repo
            || (self.path_exists(base, "AGENTS.md", PathKind::File)
                && self.path_exists(base, "rust/Cargo.toml", PathKind::File))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_domain::CommandStep;

    fn variant(id: &str, files: &[&str], commands: &[&str]) -> RecipeVariant {
        RecipeVariant {
            id: id.into(),
            description: id.into(),
            requires_files: files.iter().map(|s| (*s).to_string()).collect(),
            requires_directories: vec![],
            requires_commands: commands.iter().map(|s| (*s).to_string()).collect(),
            steps: vec![CommandStep {
                id: "s".into(),
                label: "s".into(),
                program: "echo".into(),
                args: vec![],
                cwd: "current".into(),
                continue_on_error: false,
            }],
        }
    }

    #[test]
    fn specific_variant_wins() {
        let env = FakeRecipeEnvironment::new("/proj");
        env.add_file(PathBuf::from("/proj/Cargo.toml"));
        env.add_file(PathBuf::from("/proj/package.json"));
        env.add_command("cargo");
        env.add_command("npm");
        let variants = vec![
            variant("rust", &["Cargo.toml"], &["cargo"]),
            variant("node", &["package.json"], &["npm"]),
        ];
        match env.match_variant(Path::new("/proj"), &variants) {
            VariantMatch::Matched(v) => assert_eq!(v.id, "rust"),
            VariantMatch::NoMatch => panic!("expected match"),
        }
    }

    #[test]
    fn missing_command_does_not_match() {
        let env = FakeRecipeEnvironment::new("/proj");
        env.add_file(PathBuf::from("/proj/Cargo.toml"));
        let variants = vec![variant("rust", &["Cargo.toml"], &["cargo"])];
        assert!(matches!(
            env.match_variant(Path::new("/proj"), &variants),
            VariantMatch::NoMatch
        ));
    }
}
