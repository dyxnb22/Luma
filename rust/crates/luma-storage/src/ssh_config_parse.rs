//! Pure OpenSSH config parsing (no I/O). Used by platform adapters and tests.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

const MAX_INCLUDE_DEPTH: usize = 8;

/// Whether a Host pattern should be ignored (wildcards / negation).
pub fn host_pattern_is_wildcard(pattern: &str) -> bool {
    let p = pattern.trim();
    p.contains('*') || p.contains('?') || p.contains('%') || p.starts_with('!')
}

/// Host aliases that look like CLI flags must not be passed to `ssh`/`sftp` argv.
pub fn host_alias_is_unsafe(alias: &str) -> bool {
    alias.trim().starts_with('-')
}

/// Parse config text and collect concrete Host aliases (no wildcards).
pub fn parse_host_aliases(content: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or(line).trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        if !key.eq_ignore_ascii_case("host") {
            continue;
        }
        for pattern in parts {
            if host_pattern_is_wildcard(pattern) {
                continue;
            }
            let alias = pattern.trim().to_string();
            if alias.is_empty() || host_alias_is_unsafe(&alias) || seen.contains(&alias) {
                continue;
            }
            seen.insert(alias.clone());
            out.push(alias);
        }
    }
    out.sort();
    out
}

/// Collect Include directives from config text (paths as written, not resolved).
pub fn parse_include_paths(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or(line).trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        if !key.eq_ignore_ascii_case("include") {
            continue;
        }
        for path in parts {
            let p = path.trim();
            if !p.is_empty() {
                out.push(p.to_string());
            }
        }
    }
    out
}

/// Resolve an include path relative to the containing config file directory.
/// `~/…` is expanded to the current user's home, as OpenSSH does for a normal
/// user config. `~other/…` is deliberately left alone because Luma must not
/// guess another account's home directory.
pub fn resolve_include_path(base_dir: &Path, include: &str) -> PathBuf {
    resolve_include_path_with_home(base_dir, include, dirs::home_dir().as_deref())
}

fn resolve_include_path_with_home(base_dir: &Path, include: &str, home: Option<&Path>) -> PathBuf {
    let expanded = if include == "~" {
        home.map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(include))
    } else if let Some(rest) = include.strip_prefix("~/") {
        home.map(|path| path.join(rest))
            .unwrap_or_else(|| PathBuf::from(include))
    } else {
        PathBuf::from(include)
    };
    let path = expanded;
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

/// Expand a resolved Include path. OpenSSH accepts glob patterns; unreadable or
/// unmatched paths simply contribute no aliases, just like an optional Include.
pub fn expand_include_paths(base_dir: &Path, include: &str) -> Vec<PathBuf> {
    let path = resolve_include_path(base_dir, include);
    let pattern = path.to_string_lossy();
    if !pattern.contains(['*', '?', '[', ']']) {
        return vec![path];
    }
    let mut paths: Vec<_> = glob::glob(&pattern)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .collect();
    paths.sort();
    paths
}

/// Read a config file and merge aliases from Include directives (bounded depth).
pub fn collect_aliases_from_file(
    path: &Path,
    read_file: &dyn Fn(&Path) -> Result<String, String>,
    depth: usize,
) -> Result<Vec<String>, String> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err("ssh config Include depth exceeded".into());
    }
    let content = read_file(path)?;
    let mut seen: HashSet<String> = parse_host_aliases(&content).into_iter().collect();
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    for include in parse_include_paths(&content) {
        for include_path in expand_include_paths(base_dir, &include) {
            let nested = collect_aliases_from_file(&include_path, read_file, depth + 1);
            match nested {
                Ok(nested) => {
                    for alias in nested {
                        seen.insert(alias);
                    }
                }
                Err(_) => continue,
            }
        }
    }
    let mut out: Vec<_> = seen.into_iter().collect();
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[test]
    fn ignores_wildcard_hosts() {
        let content = r#"
Host *
    User root

Host production staging
    HostName example.com

Host dev* prod?
    HostName bad
"#;
        let aliases = parse_host_aliases(content);
        assert_eq!(aliases, vec!["production", "staging"]);
    }

    #[test]
    fn collects_concrete_aliases() {
        let content = "Host prod\n  HostName 1.2.3.4\nHost staging\n";
        assert_eq!(parse_host_aliases(content), vec!["prod", "staging"]);
    }

    #[test]
    fn include_merges_aliases() {
        let files: Mutex<HashMap<String, String>> = Mutex::new(HashMap::from([
            (
                "/cfg".into(),
                "Host main\nInclude extra\nHost local\n".into(),
            ),
            ("/extra".into(), "Host included\n".into()),
        ]));
        let read = |p: &Path| {
            let key = p.to_string_lossy().to_string();
            files
                .lock()
                .unwrap()
                .get(&key)
                .cloned()
                .ok_or_else(|| "missing".into())
        };
        let aliases = collect_aliases_from_file(Path::new("/cfg"), &read, 0).unwrap();
        assert!(aliases.contains(&"main".to_string()));
        assert!(aliases.contains(&"local".to_string()));
        assert!(aliases.contains(&"included".to_string()));
    }

    #[test]
    fn include_glob_merges_all_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("config");
        let includes = dir.path().join("conf.d");
        std::fs::create_dir(&includes).unwrap();
        std::fs::write(&main, "Host main\nInclude conf.d/*.conf\n").unwrap();
        std::fs::write(includes.join("work.conf"), "Host work\n").unwrap();
        std::fs::write(includes.join("prod.conf"), "Host prod\n").unwrap();

        let aliases = collect_aliases_from_file(
            &main,
            &|path| std::fs::read_to_string(path).map_err(|err| err.to_string()),
            0,
        )
        .unwrap();
        assert_eq!(aliases, vec!["main", "prod", "work"]);
    }

    #[test]
    fn tilde_include_resolves_against_the_user_home() {
        let home = Path::new("/Users/tester");
        assert_eq!(
            resolve_include_path_with_home(Path::new("/cfg"), "~/.ssh/conf.d/a", Some(home)),
            PathBuf::from("/Users/tester/.ssh/conf.d/a")
        );
    }

    #[test]
    fn host_pattern_is_wildcard_detects_patterns() {
        assert!(host_pattern_is_wildcard("*"));
        assert!(host_pattern_is_wildcard("dev*"));
        assert!(host_pattern_is_wildcard("?"));
        assert!(!host_pattern_is_wildcard("production"));
    }

    #[test]
    fn rejects_aliases_that_look_like_flags() {
        let content = "Host production\nHost -oProxyCommand=evil\nHost --bad\n";
        assert_eq!(parse_host_aliases(content), vec!["production"]);
        assert!(host_alias_is_unsafe("-oProxyCommand=evil"));
        assert!(host_alias_is_unsafe("--bad"));
    }
}
