//! Git inspection via `git` CLI. Tests must use [`FakeGitInfo`].

use async_trait::async_trait;
use luma_application::{GitInfoError, GitInfoPort, GitSnapshot};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// macOS / Unix git adapter. Do **not** use in automated tests.
pub struct MacGitInfo;

impl MacGitInfo {
    async fn git_output(cwd: &Path, args: &[&str]) -> Result<String, GitInfoError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .await
            .map_err(|e| GitInfoError::Unavailable(e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let combined = format!("{stderr} {}", String::from_utf8_lossy(&output.stdout));
            let lower = combined.to_ascii_lowercase();
            if lower.contains("not a git repository")
                || lower.contains("not a git repo")
                || output.status.code() == Some(128)
            {
                return Err(GitInfoError::NotARepo);
            }
            return Err(GitInfoError::Unavailable(if stderr.is_empty() {
                format!("git exited {}", output.status)
            } else {
                stderr
            }));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait]
impl GitInfoPort for MacGitInfo {
    async fn inspect(&self, cwd: &Path) -> Result<GitSnapshot, GitInfoError> {
        let repo_root = Self::git_output(cwd, &["rev-parse", "--show-toplevel"]).await?;
        if repo_root.is_empty() {
            return Err(GitInfoError::NotARepo);
        }
        let branch = match Self::git_output(cwd, &["branch", "--show-current"]).await {
            Ok(b) if !b.is_empty() => Some(b),
            Ok(_) => None,
            Err(GitInfoError::NotARepo) => return Err(GitInfoError::NotARepo),
            Err(_) => None,
        };
        let worktree_path = PathBuf::from(&repo_root);
        Ok(GitSnapshot {
            repo_root: PathBuf::from(repo_root),
            branch,
            worktree_path,
        })
    }
}
