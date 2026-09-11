use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitProjectRoot {
    pub project_name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitFile {
    pub path: String,
    pub previous_path: Option<String>,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
    pub conflicted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitCommit {
    pub short_sha: String,
    pub subject: String,
    pub authored_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitBranch {
    pub name: String,
    pub current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitRepositoryState {
    pub project_name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<GitFile>,
    pub last_commit: Option<GitCommit>,
    pub unavailable: Option<String>,
}

impl GitRepositoryState {
    pub fn staged_count(&self) -> usize {
        self.files.iter().filter(|file| file.staged).count()
    }
    pub fn unstaged_count(&self) -> usize {
        self.files.iter().filter(|file| file.unstaged).count()
    }
    pub fn untracked_count(&self) -> usize {
        self.files.iter().filter(|file| file.untracked).count()
    }
    pub fn conflicted_count(&self) -> usize {
        self.files.iter().filter(|file| file.conflicted).count()
    }
    pub fn is_dirty(&self) -> bool {
        !self.files.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitDiff {
    pub text: String,
    pub truncated: bool,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum GitError {
    #[error("git unavailable: {0}")]
    Unavailable(String),
    #[error("repository unavailable: {0}")]
    NotRepository(String),
    #[error("operation timed out")]
    Timeout,
    #[error("operation cancelled")]
    Cancelled,
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("operation blocked: {0}")]
    Blocked(String),
}

#[async_trait]
pub trait GitRepositoryPort: Send + Sync {
    /// Discover only below explicit imported project roots; implementations must never widen
    /// this to a home-directory or disk scan.
    async fn discover(&self, projects: Vec<GitProjectRoot>) -> Vec<GitRepositoryState>;
    async fn inspect(&self, project: GitProjectRoot) -> Result<GitRepositoryState, GitError>;
    async fn diff(&self, repo: PathBuf, path: String, staged: bool) -> Result<GitDiff, GitError>;
    async fn stage(&self, repo: PathBuf, path: String) -> Result<(), GitError>;
    async fn unstage(&self, repo: PathBuf, path: String) -> Result<(), GitError>;
    async fn stage_all(&self, repo: PathBuf) -> Result<(), GitError>;
    async fn unstage_all(&self, repo: PathBuf) -> Result<(), GitError>;
    async fn discard(&self, repo: PathBuf, path: String) -> Result<(), GitError>;
    async fn commit(&self, repo: PathBuf, message: String) -> Result<(), GitError>;
    async fn branches(&self, repo: PathBuf) -> Result<Vec<GitBranch>, GitError>;
    async fn switch_branch(&self, repo: PathBuf, branch: String) -> Result<(), GitError>;
    async fn log(&self, repo: PathBuf, limit: usize) -> Result<Vec<GitCommit>, GitError>;
}

/// Deterministic fake for module tests. It neither invokes Git nor touches a checkout.
pub struct FakeGitRepository {
    pub repositories: Mutex<Vec<GitRepositoryState>>,
    pub diffs: Mutex<std::collections::HashMap<(String, String, bool), GitDiff>>,
    pub branches_by_repo: Mutex<std::collections::HashMap<String, Vec<GitBranch>>>,
    pub logs_by_repo: Mutex<std::collections::HashMap<String, Vec<GitCommit>>>,
    pub calls: Mutex<Vec<String>>,
    pub error: Mutex<Option<GitError>>,
}

impl FakeGitRepository {
    pub fn new(repositories: Vec<GitRepositoryState>) -> Arc<Self> {
        Arc::new(Self {
            repositories: Mutex::new(repositories),
            diffs: Mutex::new(std::collections::HashMap::new()),
            branches_by_repo: Mutex::new(std::collections::HashMap::new()),
            logs_by_repo: Mutex::new(std::collections::HashMap::new()),
            calls: Mutex::new(Vec::new()),
            error: Mutex::new(None),
        })
    }

    async fn check(&self) -> Result<(), GitError> {
        self.error.lock().await.clone().map_or(Ok(()), Err)
    }
}

#[async_trait]
impl GitRepositoryPort for FakeGitRepository {
    async fn discover(&self, projects: Vec<GitProjectRoot>) -> Vec<GitRepositoryState> {
        self.calls.lock().await.push("discover".into());
        if self.check().await.is_err() {
            return projects
                .into_iter()
                .map(|project| GitRepositoryState {
                    project_name: project.project_name,
                    path: project.path,
                    branch: None,
                    upstream: None,
                    ahead: 0,
                    behind: 0,
                    files: vec![],
                    last_commit: None,
                    unavailable: Some("fake git unavailable".into()),
                })
                .collect();
        }
        let repos = self.repositories.lock().await.clone();
        repos
            .into_iter()
            .filter(|repo| projects.iter().any(|project| repo.path == project.path))
            .collect()
    }

    async fn inspect(&self, project: GitProjectRoot) -> Result<GitRepositoryState, GitError> {
        self.check().await?;
        self.repositories
            .lock()
            .await
            .iter()
            .find(|repo| repo.path == project.path)
            .cloned()
            .ok_or_else(|| GitError::NotRepository(project.path.display().to_string()))
    }

    async fn diff(&self, repo: PathBuf, path: String, staged: bool) -> Result<GitDiff, GitError> {
        self.check().await?;
        Ok(self
            .diffs
            .lock()
            .await
            .get(&(repo.display().to_string(), path, staged))
            .cloned()
            .unwrap_or(GitDiff {
                text: "(no diff)".into(),
                truncated: false,
            }))
    }

    async fn stage(&self, repo: PathBuf, path: String) -> Result<(), GitError> {
        self.check().await?;
        self.calls
            .lock()
            .await
            .push(format!("stage:{}:{path}", repo.display()));
        Ok(())
    }
    async fn unstage(&self, repo: PathBuf, path: String) -> Result<(), GitError> {
        self.check().await?;
        self.calls
            .lock()
            .await
            .push(format!("unstage:{}:{path}", repo.display()));
        Ok(())
    }
    async fn stage_all(&self, repo: PathBuf) -> Result<(), GitError> {
        self.check().await?;
        self.calls
            .lock()
            .await
            .push(format!("stage_all:{}", repo.display()));
        Ok(())
    }
    async fn unstage_all(&self, repo: PathBuf) -> Result<(), GitError> {
        self.check().await?;
        self.calls
            .lock()
            .await
            .push(format!("unstage_all:{}", repo.display()));
        Ok(())
    }
    async fn discard(&self, repo: PathBuf, path: String) -> Result<(), GitError> {
        self.check().await?;
        self.calls
            .lock()
            .await
            .push(format!("discard:{}:{path}", repo.display()));
        Ok(())
    }
    async fn commit(&self, repo: PathBuf, message: String) -> Result<(), GitError> {
        self.check().await?;
        if message.trim().is_empty() {
            return Err(GitError::InvalidInput("commit message is required".into()));
        }
        self.calls
            .lock()
            .await
            .push(format!("commit:{}:{message}", repo.display()));
        Ok(())
    }
    async fn branches(&self, repo: PathBuf) -> Result<Vec<GitBranch>, GitError> {
        self.check().await?;
        Ok(self
            .branches_by_repo
            .lock()
            .await
            .get(&repo.display().to_string())
            .cloned()
            .unwrap_or_default())
    }
    async fn switch_branch(&self, repo: PathBuf, branch: String) -> Result<(), GitError> {
        self.check().await?;
        self.calls
            .lock()
            .await
            .push(format!("switch:{}:{branch}", repo.display()));
        Ok(())
    }
    async fn log(&self, repo: PathBuf, limit: usize) -> Result<Vec<GitCommit>, GitError> {
        self.check().await?;
        Ok(self
            .logs_by_repo
            .lock()
            .await
            .get(&repo.display().to_string())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .take(limit)
            .collect())
    }
}
