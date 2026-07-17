use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryScope {
    Global,
    Targeted { module: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Query {
    pub raw: String,
    pub normalized: String,
    pub scope: QueryScope,
    pub limit: usize,
    #[serde(skip)]
    command_mode: bool,
}

impl Query {
    /// Parse with the built-in legacy prefix table. Prefer
    /// [`Self::parse_with_prefixes`] when a registry trigger set is available.
    pub fn parse(raw: impl Into<String>, limit: usize) -> Self {
        Self::parse_internal(raw, limit, is_known_prefix, true)
    }

    /// Parse using an injected prefix predicate (registry triggers + meta tokens).
    ///
    /// A bare trigger (`n`, `clip`) without a trailing space is incomplete → [`QueryScope::Global`].
    /// This compatibility parser accepts the historical trigger syntax; interactive and CLI
    /// input should use [`Self::parse_with_prefixes_strict`].
    pub fn parse_with_prefixes(
        raw: impl Into<String>,
        limit: usize,
        is_prefix: impl Fn(&str) -> bool,
    ) -> Self {
        Self::parse_internal(raw, limit, is_prefix, true)
    }

    /// Parse user input using the explicit command format. Only a leading `/` can
    /// target a module; an unprefixed input remains a global search.
    pub fn parse_with_prefixes_strict(
        raw: impl Into<String>,
        limit: usize,
        is_prefix: impl Fn(&str) -> bool,
    ) -> Self {
        Self::parse_internal(raw, limit, is_prefix, false)
    }

    fn parse_internal(
        raw: impl Into<String>,
        limit: usize,
        is_prefix: impl Fn(&str) -> bool,
        legacy_commands: bool,
    ) -> Self {
        let raw = raw.into();
        let slash_command = raw.trim_start().starts_with('/');
        let command_mode = slash_command || legacy_commands;
        let command_body = if slash_command {
            strip_command_prefix(&raw)
        } else {
            raw.as_str()
        };
        let committed = command_body.ends_with(|c: char| c.is_whitespace());
        let normalized = command_body.trim().to_lowercase();
        let scope = if let Some((prefix, _)) = normalized.split_once(|c: char| c.is_whitespace()) {
            if command_mode && is_prefix(prefix) {
                QueryScope::Targeted {
                    module: prefix.to_string(),
                }
            } else {
                QueryScope::Global
            }
        } else if command_mode && is_prefix(&normalized) {
            if committed {
                QueryScope::Targeted {
                    module: normalized.clone(),
                }
            } else {
                QueryScope::Global
            }
        } else {
            QueryScope::Global
        };
        Self {
            raw,
            normalized,
            scope,
            limit,
            command_mode,
        }
    }

    /// Normalize a query for non-interactive CLI: slash-prefixed bare triggers become committed
    /// targeted searches (`/clip` -> `/clip `). Unprefixed input remains a global search.
    pub fn normalize_for_cli(raw: impl Into<String>, is_prefix: impl Fn(&str) -> bool) -> String {
        let raw = raw.into();
        let probe = Self::parse_with_prefixes_strict(&raw, 50, &is_prefix);
        if probe.is_incomplete_trigger(&is_prefix) {
            format!("{} ", raw.trim_end())
        } else {
            raw
        }
    }

    /// True when input is exactly a module trigger with no trailing space yet (`n`, not `n `).
    pub fn is_incomplete_trigger(&self, is_prefix: impl Fn(&str) -> bool) -> bool {
        if !self.command_mode {
            return false;
        }
        let command_body = strip_command_prefix(&self.raw);
        let trimmed = command_body.trim();
        !command_body.ends_with(|c: char| c.is_whitespace())
            && !trimmed.chars().any(|c| c.is_whitespace())
            && is_prefix(&trimmed.to_lowercase())
    }

    /// Case-preserving text after the first whitespace token in the trimmed raw input.
    pub fn rest_raw(&self) -> &str {
        if !self.command_mode {
            return self.raw.trim();
        }
        strip_command_prefix(&self.raw)
            .trim()
            .split_once(|c: char| c.is_whitespace())
            .map(|(_, rest)| rest.trim())
            .unwrap_or("")
    }

    /// Lowercased [`Self::rest_raw`] for case-insensitive matching.
    pub fn rest_normalized(&self) -> String {
        self.rest_raw().to_lowercase()
    }

    pub fn is_command(&self) -> bool {
        self.command_mode
    }
}

/// Remove the optional command marker when it appears at the beginning of an
/// input. A slash in a later token (for example a path or URL) is preserved.
pub fn strip_command_prefix(raw: &str) -> &str {
    let leading_trimmed = raw.trim_start();
    leading_trimmed.strip_prefix('/').unwrap_or(raw)
}

fn is_known_prefix(token: &str) -> bool {
    matches!(
        token,
        "app"
            | "apps"
            | "clip"
            | "cb"
            | "n"
            | "note"
            | "notes"
            | "help"
            | "fake"
            | "echo"
            | "ql"
            | "quicklinks"
            | "s"
            | "snip"
            | "sec"
            | "secret"
            | "secrets"
            | "p"
            | "proj"
            | "project"
            | "wb"
            | "wordbook"
            | "words"
            | "win"
            | "window"
            | "windows"
            | "ssh"
            | "proxy"
            | "px"
            | "rec"
            | "record"
            | "tm"
            | "timer"
            | "timers"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targeted_app_query() {
        let q = Query::parse("app safari", 20);
        assert!(matches!(q.scope, QueryScope::Targeted { ref module } if module == "app"));
    }

    #[test]
    fn global_query() {
        let q = Query::parse("safari", 20);
        assert_eq!(q.scope, QueryScope::Global);
    }

    #[test]
    fn rest_raw_preserves_payload_case() {
        let q = Query::parse("ql https://Example.COM/Path", 20);
        assert_eq!(q.rest_raw(), "https://Example.COM/Path");
        assert_eq!(q.rest_normalized(), "https://example.com/path");
        assert!(q.normalized.contains("https://example.com/path"));
    }

    #[test]
    fn rest_raw_empty_for_bare_trigger() {
        let q = Query::parse("clip", 20);
        assert_eq!(q.rest_raw(), "");
        assert_eq!(q.rest_normalized(), "");
        assert_eq!(q.scope, QueryScope::Global);
    }

    #[test]
    fn trailing_space_commits_bare_trigger() {
        let q = Query::parse("n ", 20);
        assert!(matches!(q.scope, QueryScope::Targeted { ref module } if module == "n"));
        assert_eq!(q.rest_raw(), "");
    }

    #[test]
    fn timers_trigger_is_known_prefix() {
        let q = Query::parse("tm ", 20);
        assert!(matches!(q.scope, QueryScope::Targeted { ref module } if module == "tm"));
        let q2 = Query::parse("tm pomo 25", 20);
        assert!(matches!(q2.scope, QueryScope::Targeted { ref module } if module == "tm"));
        assert_eq!(q2.rest_normalized(), "pomo 25");
    }

    #[test]
    fn normalize_for_cli_commits_slash_trigger() {
        let n = Query::normalize_for_cli("/clip", is_known_prefix);
        assert_eq!(n, "/clip ");
        let q = Query::parse_with_prefixes_strict(&n, 20, is_known_prefix);
        assert!(matches!(q.scope, QueryScope::Targeted { ref module } if module == "clip"));
        let unchanged = Query::normalize_for_cli("app safari", is_known_prefix);
        assert_eq!(unchanged, "app safari");
    }

    #[test]
    fn bare_trigger_is_incomplete() {
        let q = Query::parse("n", 20);
        assert!(q.is_incomplete_trigger(is_known_prefix));
        let q2 = Query::parse("n ", 20);
        assert!(!q2.is_incomplete_trigger(is_known_prefix));
        let q3 = Query::parse("n docker", 20);
        assert!(!q3.is_incomplete_trigger(is_known_prefix));
    }

    #[test]
    fn parse_with_prefixes_uses_injected_set() {
        let q = Query::parse_with_prefixes("custom Hello", 20, |t| t == "custom");
        assert!(matches!(q.scope, QueryScope::Targeted { ref module } if module == "custom"));
        assert_eq!(q.rest_raw(), "Hello");
    }

    #[test]
    fn leading_slash_targets_a_module_and_preserves_raw_input() {
        let q = Query::parse_with_prefixes_strict("/ssh production", 20, |t| t == "ssh");
        assert!(matches!(q.scope, QueryScope::Targeted { ref module } if module == "ssh"));
        assert_eq!(q.raw, "/ssh production");
        assert_eq!(q.normalized, "ssh production");
        assert_eq!(q.rest_raw(), "production");
        assert!(q.is_command());
    }

    #[test]
    fn strict_parser_rejects_unprefixed_module_commands() {
        let q = Query::parse_with_prefixes_strict("ssh production", 20, |t| t == "ssh");
        assert_eq!(q.scope, QueryScope::Global);
        assert_eq!(q.rest_raw(), "ssh production");
        assert!(!q.is_command());
    }

    #[test]
    fn leading_slash_bare_trigger_is_incomplete() {
        let q = Query::parse("/clip", 20);
        assert!(q.is_incomplete_trigger(is_known_prefix));
        let q = Query::parse("/clip ", 20);
        assert!(!q.is_incomplete_trigger(is_known_prefix));
    }

    #[test]
    fn slash_in_payload_is_not_removed() {
        let q = Query::parse("ql https://example.com/path", 20);
        assert_eq!(q.rest_raw(), "https://example.com/path");
        let q = Query::parse("app /Applications/Safari.app", 20);
        assert_eq!(q.rest_raw(), "/Applications/Safari.app");
    }
}
