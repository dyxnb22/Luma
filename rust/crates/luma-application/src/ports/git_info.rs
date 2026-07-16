use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitSnapshot {
    pub repo_root: PathBuf,
    pub branch: Option<String>,
    pub worktree_path: PathBuf,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GitInfoError {
    #[error("not a git repository")]
    NotARepo,
    #[error("git unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait GitInfoPort: Send + Sync {
    async fn inspect(&self, cwd: &Path) -> Result<GitSnapshot, GitInfoError>;
}

/// Controllable fake — never shells out to git.
#[derive(Default)]
pub struct FakeGitInfo {
    pub snapshot: Mutex<Option<Result<GitSnapshot, GitInfoError>>>,
    pub calls: Arc<Mutex<Vec<PathBuf>>>,
}

impl FakeGitInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_snapshot(snapshot: GitSnapshot) -> Self {
        let f = Self::new();
        *f.snapshot.lock().expect("lock") = Some(Ok(snapshot));
        f
    }

    pub fn with_error(err: GitInfoError) -> Self {
        let f = Self::new();
        *f.snapshot.lock().expect("lock") = Some(Err(err));
        f
    }
}

#[async_trait]
impl GitInfoPort for FakeGitInfo {
    async fn inspect(&self, cwd: &Path) -> Result<GitSnapshot, GitInfoError> {
        self.calls.lock().expect("lock").push(cwd.to_path_buf());
        match self.snapshot.lock().expect("lock").clone() {
            Some(result) => result,
            None => Err(GitInfoError::NotARepo),
        }
    }
}
