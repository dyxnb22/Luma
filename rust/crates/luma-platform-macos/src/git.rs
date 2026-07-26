//! Structured, bounded Git I/O for explicitly imported project roots.

use async_trait::async_trait;
use luma_application::{
    GitBranch, GitCommit, GitDiff, GitError, GitFile, GitProjectRoot, GitRepositoryPort,
    GitRepositoryState,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

pub const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(4);
pub const MAX_GIT_DISCOVERY_DIRECTORIES: usize = 128;
pub const MAX_GIT_DISCOVERY_DEPTH: usize = 3;
pub const MAX_GIT_LOG_ENTRIES: usize = 50;
pub const MAX_GIT_DIFF_BYTES: usize = 64 * 1024;
pub const MAX_GIT_DIFF_LINES: usize = 1_200;

pub struct MacGitRepository;

impl MacGitRepository {
    async fn run(repo: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
        let mut command = Command::new("git");
        command
            .args(args)
            .current_dir(repo)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_PAGER", "cat")
            .env("GIT_EDITOR", "true")
            .env("LC_ALL", "C");
        let output = tokio::time::timeout(GIT_COMMAND_TIMEOUT, command.output())
            .await
            .map_err(|_| GitError::Timeout)?
            .map_err(|error| GitError::Unavailable(error.to_string()))?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            let stderr = bounded_text(&output.stderr, 400);
            Err(GitError::Blocked(if stderr.is_empty() {
                format!("git exited with {}", output.status)
            } else {
                stderr
            }))
        }
    }

    async fn state_for(
        project_name: String,
        path: PathBuf,
    ) -> Result<GitRepositoryState, GitError> {
        let root = Self::run(
            &canonical_directory(&path)?,
            &["rev-parse", "--show-toplevel"],
        )
        .await?;
        let root = canonical_directory(Path::new(bounded_text(&root, 8192).trim()))?;
        let output = Self::run(&root, &["status", "--porcelain=v2", "--branch", "-z"]).await?;
        let (branch, upstream, ahead, behind, files) = parse_status_v2(&output);
        Ok(GitRepositoryState {
            project_name,
            path: root.clone(),
            branch,
            upstream,
            ahead,
            behind,
            files,
            last_commit: Self::last_commit(&root).await.ok().flatten(),
            unavailable: None,
        })
    }

    async fn last_commit(repo: &Path) -> Result<Option<GitCommit>, GitError> {
        let output = Self::run(repo, &["log", "-1", "--format=%h%x1f%s%x1f%aI"]).await?;
        Ok(parse_commit_line(&bounded_text(&output, 8192)))
    }
}

#[async_trait]
impl GitRepositoryPort for MacGitRepository {
    async fn discover(&self, projects: Vec<GitProjectRoot>) -> Vec<GitRepositoryState> {
        // Deliberately sequential: this is a strict process-concurrency bound and prevents a
        // dashboard refresh from competing with the developer's own Git commands.
        let mut results = Vec::new();
        let mut seen = BTreeSet::new();
        for project in projects {
            let candidates = discover_candidates(&project.path);
            if candidates.is_empty() {
                results.push(unavailable(project, "not a Git repository"));
                continue;
            }
            for candidate in candidates {
                match Self::state_for(project.project_name.clone(), candidate).await {
                    Ok(state) if seen.insert(state.path.display().to_string()) => {
                        results.push(state)
                    }
                    Ok(_) => {}
                    Err(error) => results.push(unavailable(project.clone(), &error.to_string())),
                }
            }
        }
        results
    }

    async fn inspect(&self, project: GitProjectRoot) -> Result<GitRepositoryState, GitError> {
        Self::state_for(project.project_name, project.path).await
    }

    async fn diff(&self, repo: PathBuf, path: String, staged: bool) -> Result<GitDiff, GitError> {
        validate_relative_path(&path)?;
        let mut args = vec!["diff", "--no-ext-diff", "--no-color"];
        if staged {
            args.push("--cached");
        }
        args.extend(["--", &path]);
        Ok(truncate_diff(&Self::run(&repo, &args).await?))
    }

    async fn stage(&self, repo: PathBuf, path: String) -> Result<(), GitError> {
        validate_relative_path(&path)?;
        Self::run(&repo, &["add", "--", &path]).await.map(|_| ())
    }
    async fn unstage(&self, repo: PathBuf, path: String) -> Result<(), GitError> {
        validate_relative_path(&path)?;
        Self::run(&repo, &["restore", "--staged", "--", &path])
            .await
            .map(|_| ())
    }
    async fn stage_all(&self, repo: PathBuf) -> Result<(), GitError> {
        Self::run(&repo, &["add", "-A"]).await.map(|_| ())
    }
    async fn unstage_all(&self, repo: PathBuf) -> Result<(), GitError> {
        Self::run(&repo, &["restore", "--staged", "."])
            .await
            .map(|_| ())
    }
    async fn discard(&self, repo: PathBuf, path: String) -> Result<(), GitError> {
        validate_relative_path(&path)?;
        // Never uses `git clean`: untracked files remain untouched.
        // Restore from the index so a staged version is preserved when the file also has
        // unstaged edits. The module does not offer this action for untracked/conflicted rows.
        Self::run(&repo, &["restore", "--worktree", "--", &path])
            .await
            .map(|_| ())
    }
    async fn commit(&self, repo: PathBuf, message: String) -> Result<(), GitError> {
        if message.trim().is_empty() || message.contains('\0') || message.len() > 4_000 {
            return Err(GitError::InvalidInput(
                "commit message is required and must be bounded".into(),
            ));
        }
        Self::run(&repo, &["commit", "-m", &message])
            .await
            .map(|_| ())
    }
    async fn branches(&self, repo: PathBuf) -> Result<Vec<GitBranch>, GitError> {
        let output = Self::run(&repo, &["branch", "--format=%(HEAD)%00%(refname:short)"]).await?;
        Ok(output
            .split(|byte| *byte == b'\n')
            .filter_map(|line| {
                let separator = line.iter().position(|byte| *byte == b'\0')?;
                let (head, rest) = line.split_at(separator);
                let name = &rest[1..];
                let name = String::from_utf8_lossy(name).trim().to_string();
                (!name.is_empty()).then(|| GitBranch {
                    name,
                    current: head == b"*",
                })
            })
            .collect())
    }
    async fn switch_branch(&self, repo: PathBuf, branch: String) -> Result<(), GitError> {
        if branch.trim().is_empty() || branch.starts_with('-') || branch.contains('\0') {
            return Err(GitError::InvalidInput("branch name is invalid".into()));
        }
        let state = Self::state_for("current".into(), repo.clone()).await?;
        if state.is_dirty() || state.conflicted_count() > 0 {
            return Err(GitError::Blocked(
                "commit, stash, or clean changes before switching branches".into(),
            ));
        }
        Self::run(&repo, &["switch", &branch]).await.map(|_| ())
    }
    async fn log(&self, repo: PathBuf, limit: usize) -> Result<Vec<GitCommit>, GitError> {
        let limit = limit.clamp(1, MAX_GIT_LOG_ENTRIES).to_string();
        let output = Self::run(&repo, &["log", "--format=%h%x1f%s%x1f%aI", "-n", &limit]).await?;
        Ok(bounded_text(&output, 32 * 1024)
            .lines()
            .filter_map(parse_commit_line)
            .collect())
    }
}

fn unavailable(project: GitProjectRoot, reason: &str) -> GitRepositoryState {
    GitRepositoryState {
        project_name: project.project_name,
        path: project.path,
        branch: None,
        upstream: None,
        ahead: 0,
        behind: 0,
        files: vec![],
        last_commit: None,
        unavailable: Some(reason.into()),
    }
}

fn canonical_directory(path: &Path) -> Result<PathBuf, GitError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| GitError::NotRepository(format!("{}: {error}", path.display())))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(GitError::NotRepository(path.display().to_string()));
    }
    std::fs::canonicalize(path).map_err(|error| GitError::NotRepository(error.to_string()))
}

fn discover_candidates(root: &Path) -> Vec<PathBuf> {
    let Ok(root) = canonical_directory(root) else {
        return vec![];
    };
    let mut out = Vec::new();
    let mut pending = vec![(root, 0usize)];
    let mut examined = 0;
    while let Some((path, depth)) = pending.pop() {
        if examined >= MAX_GIT_DISCOVERY_DIRECTORIES {
            break;
        }
        examined += 1;
        if path.join(".git").exists() {
            out.push(path);
            continue;
        }
        if depth >= MAX_GIT_DISCOVERY_DEPTH {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                if let Ok(kind) = entry.file_type() {
                    if kind.is_dir() && !kind.is_symlink() && entry.file_name() != ".git" {
                        pending.push((entry.path(), depth + 1));
                    }
                }
            }
        }
    }
    out
}

fn validate_relative_path(path: &str) -> Result<(), GitError> {
    let candidate = Path::new(path);
    if path.is_empty()
        || candidate.is_absolute()
        || path.contains('\0')
        || candidate
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(GitError::InvalidInput(
            "file path must stay inside the repository".into(),
        ));
    }
    Ok(())
}

fn bounded_text(bytes: &[u8], max: usize) -> String {
    String::from_utf8_lossy(&bytes[..bytes.len().min(max)])
        .replace('\0', "")
        .trim()
        .to_string()
}

fn parse_status_v2(bytes: &[u8]) -> (Option<String>, Option<String>, u32, u32, Vec<GitFile>) {
    let mut branch = None;
    let mut upstream = None;
    let mut ahead = 0;
    let mut behind = 0;
    let mut files = vec![];
    let mut records = bytes.split(|byte| *byte == b'\0');
    while let Some(record) = records.next() {
        let line = String::from_utf8_lossy(record);
        if let Some(value) = line.strip_prefix("# branch.head ") {
            if value != "(detached)" {
                branch = Some(value.into());
            }
        } else if let Some(value) = line.strip_prefix("# branch.upstream ") {
            upstream = Some(value.into());
        } else if let Some(value) = line.strip_prefix("# branch.ab ") {
            for part in value.split_whitespace() {
                if let Some(v) = part.strip_prefix('+') {
                    ahead = v.parse().unwrap_or(0);
                } else if let Some(v) = part.strip_prefix('-') {
                    behind = v.parse().unwrap_or(0);
                }
            }
        } else if let Some(path) = line.strip_prefix("? ") {
            files.push(GitFile {
                path: path.into(),
                previous_path: None,
                staged: false,
                unstaged: false,
                untracked: true,
                conflicted: false,
            });
        } else if line.starts_with("1 ") || line.starts_with("2 ") || line.starts_with("u ") {
            let typ = line.split_once(' ').map(|(typ, _)| typ).unwrap_or_default();
            let xy = line.split_whitespace().nth(1).unwrap_or("..");
            let path = match typ {
                "1" => line.splitn(9, ' ').nth(8),
                "2" => line.splitn(10, ' ').nth(9),
                "u" => line.splitn(11, ' ').nth(10),
                _ => None,
            }
            .unwrap_or_default();
            let previous_path = if typ == "2" {
                records
                    .next()
                    .map(|raw| String::from_utf8_lossy(raw).to_string())
            } else {
                None
            };
            files.push(GitFile {
                path: path.into(),
                previous_path,
                staged: xy.chars().next().is_some_and(|c| c != '.'),
                unstaged: xy.chars().nth(1).is_some_and(|c| c != '.'),
                untracked: false,
                conflicted: typ == "u" || xy.contains('U'),
            });
        }
    }
    (branch, upstream, ahead, behind, files)
}

fn parse_commit_line(line: &str) -> Option<GitCommit> {
    let mut parts = line.split('\u{1f}');
    Some(GitCommit {
        short_sha: parts.next()?.trim().into(),
        subject: parts.next()?.trim().into(),
        authored_at: parts.next()?.trim().into(),
    })
}

fn truncate_diff(output: &[u8]) -> GitDiff {
    let text = String::from_utf8_lossy(&output[..output.len().min(MAX_GIT_DIFF_BYTES)]);
    let mut lines = text.lines().take(MAX_GIT_DIFF_LINES).collect::<Vec<_>>();
    let truncated = output.len() > MAX_GIT_DIFF_BYTES || text.lines().count() > MAX_GIT_DIFF_LINES;
    if truncated {
        lines.push("… diff preview truncated …");
    }
    GitDiff {
        text: lines.join("\n"),
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(root: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .expect("git is available for platform adapter tests");
        assert!(status.success(), "git {:?} failed", args);
    }
    #[test]
    fn parses_status_categories() {
        let input = b"# branch.head main\0# branch.ab +2 -1\x001 M. N... 100644 100644 100644 a b file.txt\0? new file.txt\0u UU N... 100644 100644 100644 100644 a b c conflict.txt\0";
        let (branch, _, ahead, behind, files) = parse_status_v2(input);
        assert_eq!(branch.as_deref(), Some("main"));
        assert_eq!((ahead, behind), (2, 1));
        assert!(files.iter().any(|file| file.staged));
        assert!(files.iter().any(|file| file.untracked));
        assert!(files
            .iter()
            .any(|file| file.conflicted && file.path == "conflict.txt"));
    }
    #[test]
    fn rejects_path_escape_and_allows_unicode_space() {
        assert!(validate_relative_path("../x").is_err());
        assert!(validate_relative_path("/x").is_err());
        assert!(validate_relative_path("日本 space.rs").is_ok());
    }

    #[tokio::test]
    async fn real_repository_reports_staged_unstaged_and_untracked_without_cleaning_untracked() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        git(root, &["init"]);
        git(root, &["config", "user.email", "luma@example.invalid"]);
        git(root, &["config", "user.name", "Luma Test"]);
        std::fs::write(root.join("tracked.txt"), "base\n").unwrap();
        git(root, &["add", "tracked.txt"]);
        git(root, &["commit", "-m", "initial"]);
        std::fs::write(root.join("tracked.txt"), "changed\n").unwrap();
        std::fs::write(root.join("staged.txt"), "staged\n").unwrap();
        git(root, &["add", "staged.txt"]);
        std::fs::write(root.join("staged.txt"), "staged plus unstaged\n").unwrap();
        std::fs::write(root.join("untracked.txt"), "keep me\n").unwrap();

        let adapter = MacGitRepository;
        let state = adapter
            .inspect(GitProjectRoot {
                project_name: "fixture".into(),
                path: root.to_path_buf(),
            })
            .await
            .unwrap();
        assert!(state
            .files
            .iter()
            .any(|file| file.path == "tracked.txt" && file.unstaged));
        assert!(state
            .files
            .iter()
            .any(|file| file.path == "staged.txt" && file.staged));
        assert!(state
            .files
            .iter()
            .any(|file| file.path == "untracked.txt" && file.untracked));

        adapter
            .discard(state.path.clone(), "tracked.txt".into())
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("tracked.txt")).unwrap(),
            "base\n"
        );
        adapter
            .discard(state.path.clone(), "staged.txt".into())
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("staged.txt")).unwrap(),
            "staged\n",
            "discard must preserve the version already staged in the index"
        );
        assert!(adapter
            .discard(state.path.clone(), "untracked.txt".into())
            .await
            .is_err());
        assert!(root.join("untracked.txt").exists());
    }

    #[tokio::test]
    async fn dirty_repository_blocks_branch_switch() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        git(root, &["init"]);
        git(root, &["config", "user.email", "luma@example.invalid"]);
        git(root, &["config", "user.name", "Luma Test"]);
        std::fs::write(root.join("tracked.txt"), "base\n").unwrap();
        git(root, &["add", "tracked.txt"]);
        git(root, &["commit", "-m", "initial"]);
        git(root, &["branch", "other"]);
        std::fs::write(root.join("tracked.txt"), "dirty\n").unwrap();
        let error = MacGitRepository
            .switch_branch(root.to_path_buf(), "other".into())
            .await
            .unwrap_err();
        assert!(matches!(error, GitError::Blocked(_)));
    }
}
