//! Workspace discovery for markdown notes (walkdir-based, no symlink follow).

use crate::notes_ignore::{rel_path_str, IgnoreMatcher};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkipReason {
    HiddenPathSegment,
    IgnoredDirectory,
    ExcludePattern,
    SkipSymlink,
    WalkError,
}

impl SkipReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkipReason::HiddenPathSegment => "hidden path segment",
            SkipReason::IgnoredDirectory => "ignored directory",
            SkipReason::ExcludePattern => "exclude pattern",
            SkipReason::SkipSymlink => "symlink skipped",
            SkipReason::WalkError => "walk error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkippedEntry {
    pub path: PathBuf,
    pub relative_path: Option<String>,
    pub reason: SkipReason,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct DiscoverResult {
    pub files: Vec<PathBuf>,
    pub skipped: Vec<SkippedEntry>,
    /// False when the root is missing/unreadable or any walk error occurred.
    /// Incomplete discoveries must not drive a global prune.
    pub complete: bool,
}

/// Discover `.md` files under `workspace_root`, honoring ignore rules and skipping symlinks.
pub fn discover(workspace_root: &Path, matcher: &IgnoreMatcher) -> DiscoverResult {
    let mut result = DiscoverResult {
        complete: true,
        ..DiscoverResult::default()
    };
    let root = workspace_root;

    match std::fs::symlink_metadata(root) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {}
        Ok(_) => {
            result.complete = false;
            result.skipped.push(SkippedEntry {
                path: root.to_path_buf(),
                relative_path: Some(".".into()),
                reason: SkipReason::WalkError,
                message: "workspace root is not a directory".into(),
            });
            return result;
        }
        Err(e) => {
            result.complete = false;
            result.skipped.push(SkippedEntry {
                path: root.to_path_buf(),
                relative_path: Some(".".into()),
                reason: SkipReason::WalkError,
                message: format!("workspace root unreadable: {e}"),
            });
            return result;
        }
    }

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if matcher.should_skip_dir_name(name) {
                    return false;
                }
            }
            if entry.file_type().is_dir() {
                if let Some(rel) = rel_path_str(root, path) {
                    let (scan, _) = matcher.should_scan(&rel);
                    if !scan {
                        return false;
                    }
                }
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                result.complete = false;
                let path = err
                    .path()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| root.to_path_buf());
                let rel = rel_path_str(root, &path);
                result.skipped.push(SkippedEntry {
                    path,
                    relative_path: rel,
                    reason: SkipReason::WalkError,
                    message: err.to_string(),
                });
                continue;
            }
        };

        let path = entry.path().to_path_buf();
        if path == root {
            continue;
        }

        let rel = match rel_path_str(root, &path) {
            Some(r) => r,
            None => continue,
        };

        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                result.complete = false;
                result.skipped.push(SkippedEntry {
                    path,
                    relative_path: Some(rel),
                    reason: SkipReason::WalkError,
                    message: e.to_string(),
                });
                continue;
            }
        };

        if meta.file_type().is_symlink() {
            result.skipped.push(SkippedEntry {
                path,
                relative_path: Some(rel),
                reason: SkipReason::SkipSymlink,
                message: "symlink not followed".into(),
            });
            continue;
        }

        if meta.is_dir() {
            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if matcher.should_skip_dir_name(&dir_name) {
                result.skipped.push(SkippedEntry {
                    path,
                    relative_path: Some(rel),
                    reason: SkipReason::IgnoredDirectory,
                    message: format!("skipped directory {dir_name}"),
                });
                continue;
            }
            let (scan, reason) = matcher.should_scan(&rel);
            if !scan {
                result.skipped.push(SkippedEntry {
                    path,
                    relative_path: Some(rel),
                    reason: skip_reason_from_static(reason),
                    message: reason.unwrap_or("skipped").into(),
                });
            }
            continue;
        }

        if !meta.is_file() {
            continue;
        }

        let (scan, reason) = matcher.should_scan(&rel);
        if !scan {
            result.skipped.push(SkippedEntry {
                path,
                relative_path: Some(rel),
                reason: skip_reason_from_static(reason),
                message: reason.unwrap_or("skipped").into(),
            });
            continue;
        }

        if is_markdown_file(&path) {
            result.files.push(path);
        }
    }

    result.files.sort();
    result
}

fn skip_reason_from_static(reason: Option<&str>) -> SkipReason {
    match reason {
        Some("hidden path segment") => SkipReason::HiddenPathSegment,
        Some("ignored directory") => SkipReason::IgnoredDirectory,
        Some("exclude pattern") => SkipReason::ExcludePattern,
        _ => SkipReason::WalkError,
    }
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes_ignore::IgnoreMatcher;

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/notes-workspaces")
            .join(name)
    }

    #[test]
    fn basic_discovers_seven_md_files() {
        let root = fixture_root("basic");
        let result = discover(&root, &IgnoreMatcher::default());
        assert_eq!(result.files.len(), 7, "files: {:?}", result.files);
    }

    #[test]
    fn hidden_ignores_git() {
        let root = fixture_root("hidden");
        let result = discover(&root, &IgnoreMatcher::default());
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("visible"));
        assert!(
            !result
                .files
                .iter()
                .any(|p| p.to_string_lossy().contains(".git")),
            "must not discover files under .git"
        );
    }

    #[test]
    fn excluded_private_pattern() {
        let root = fixture_root("excluded");
        let matcher = IgnoreMatcher::new(vec![], vec!["private/*".into()]);
        let result = discover(&root, &matcher);
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].to_string_lossy().contains("public/note.md"));
    }

    #[test]
    fn symlinks_skip_link_md() {
        let root = fixture_root("symlinks");
        let result = discover(&root, &IgnoreMatcher::default());
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("real"));
        assert!(result
            .skipped
            .iter()
            .any(|s| s.reason == SkipReason::SkipSymlink
                && s.relative_path.as_deref() == Some("link.md")));
    }
}
