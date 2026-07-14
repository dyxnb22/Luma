//! Path ignore rules for notes workspace scanning (Ariadne-equivalent matcher).

use std::path::Path;

/// Default directory names to skip during discovery.
pub const DEFAULT_IGNORE_DIRS: &[&str] = &[".git", ".obsidian", ".vscode", ".cache"];

#[derive(Clone, Debug, Default)]
pub struct IgnoreMatcher {
    ignore_dirs: Vec<String>,
    exclude_patterns: Vec<String>,
}

impl IgnoreMatcher {
    pub fn new(ignore_dirs: Vec<String>, exclude_patterns: Vec<String>) -> Self {
        Self {
            ignore_dirs,
            exclude_patterns,
        }
    }

    /// Returns `(should_scan, skip_reason)`. `should_scan` is `true` when the relative path
    /// should be visited (directories) or indexed (files).
    pub fn should_scan(&self, rel: &str) -> (bool, Option<&'static str>) {
        let rel = rel.trim_start_matches("./");
        if rel.is_empty() {
            return (true, None);
        }

        let normalized = rel.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();

        for segment in &segments {
            if segment.starts_with('.') {
                return (false, Some("hidden path segment"));
            }
        }

        for segment in &segments {
            if DEFAULT_IGNORE_DIRS.contains(segment)
                || self.ignore_dirs.iter().any(|d| d == *segment)
            {
                return (false, Some("ignored directory"));
            }
        }

        let basename = segments.last().copied().unwrap_or("");
        for pattern in &self.exclude_patterns {
            let matched = if pattern.contains('/') {
                path_match(pattern, &normalized)
            } else {
                path_match(pattern, basename)
            };
            if matched {
                return (false, Some("exclude pattern"));
            }
        }

        (true, None)
    }

    /// Returns `true` when a directory name alone should be pruned (no descent).
    pub fn should_skip_dir_name(&self, dir_name: &str) -> bool {
        if dir_name.starts_with('.') {
            return true;
        }
        if DEFAULT_IGNORE_DIRS.contains(&dir_name) || self.ignore_dirs.iter().any(|d| d == dir_name)
        {
            return true;
        }
        false
    }
}

/// Go `filepath.Match`-style glob: `*` matches any run of non-`/` chars, `?` one non-`/` char.
pub fn path_match(pattern: &str, value: &str) -> bool {
    match_segments(pattern.as_bytes(), value.as_bytes())
}

fn match_segments(pat: &[u8], val: &[u8]) -> bool {
    if pat.is_empty() {
        return val.is_empty();
    }
    if pat[0] == b'*' {
        if pat.len() == 1 {
            return !val.contains(&b'/');
        }
        let rest = &pat[1..];
        if rest.is_empty() {
            return true;
        }
        if rest[0] != b'/' {
            for (i, &ch) in val.iter().enumerate() {
                if ch == b'/' {
                    continue;
                }
                if match_segments(rest, &val[i..]) {
                    return true;
                }
            }
            return false;
        }
        return match_segments(&pat[1..], val);
    }

    if val.is_empty() {
        return false;
    }

    if pat[0] == b'?' {
        if val[0] == b'/' {
            return false;
        }
        return match_segments(&pat[1..], &val[1..]);
    }

    if pat[0] == val[0] {
        return match_segments(&pat[1..], &val[1..]);
    }

    false
}

/// Normalize a path relative to workspace root as a forward-slash string.
pub fn rel_path_str(root: &Path, abs: &Path) -> Option<String> {
    let rel = abs.strip_prefix(root).ok()?;
    let s = rel.to_string_lossy().replace('\\', "/");
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_match_star_and_question() {
        assert!(path_match("*.md", "note.md"));
        assert!(!path_match("*.md", "note.txt"));
        assert!(path_match("note?.md", "notes.md"));
        assert!(!path_match("note?.md", "note.md"));
        assert!(path_match("private/*", "private/secret.md"));
        assert!(!path_match("private/*", "public/note.md"));
    }

    #[test]
    fn hidden_segment_skipped() {
        let m = IgnoreMatcher::default();
        let (ok, reason) = m.should_scan(".git/config");
        assert!(!ok);
        assert_eq!(reason, Some("hidden path segment"));
    }

    #[test]
    fn default_ignore_dirs() {
        let m = IgnoreMatcher::default();
        let (ok, _) = m.should_scan("foo/.obsidian/x.md");
        assert!(!ok);
    }

    #[test]
    fn exclude_pattern_private() {
        let m = IgnoreMatcher::new(vec![], vec!["private/*".into()]);
        let (ok_pub, _) = m.should_scan("public/note.md");
        assert!(ok_pub);
        let (ok_priv, reason) = m.should_scan("private/secret.md");
        assert!(!ok_priv);
        assert_eq!(reason, Some("exclude pattern"));
    }
}
