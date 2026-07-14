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
}

impl Query {
    /// Parse with the built-in legacy prefix table. Prefer
    /// [`Self::parse_with_prefixes`] when a registry trigger set is available.
    pub fn parse(raw: impl Into<String>, limit: usize) -> Self {
        Self::parse_with_prefixes(raw, limit, is_known_prefix)
    }

    /// Parse using an injected prefix predicate (registry triggers + meta tokens).
    ///
    /// A bare trigger (`n`, `clip`) without a trailing space is incomplete → [`QueryScope::Global`].
    /// Commit with a trailing space (`n `) or a payload (`n docker`).
    pub fn parse_with_prefixes(
        raw: impl Into<String>,
        limit: usize,
        is_prefix: impl Fn(&str) -> bool,
    ) -> Self {
        let raw = raw.into();
        let committed = raw.ends_with(|c: char| c.is_whitespace());
        let normalized = raw.trim().to_lowercase();
        let scope = if let Some((prefix, _)) = normalized.split_once(|c: char| c.is_whitespace()) {
            if is_prefix(prefix) {
                QueryScope::Targeted {
                    module: prefix.to_string(),
                }
            } else {
                QueryScope::Global
            }
        } else if is_prefix(&normalized) {
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
        }
    }

    /// True when input is exactly a module trigger with no trailing space yet (`n`, not `n `).
    pub fn is_incomplete_trigger(&self, is_prefix: impl Fn(&str) -> bool) -> bool {
        let trimmed = self.raw.trim();
        !self.raw.ends_with(|c: char| c.is_whitespace())
            && !trimmed.chars().any(|c| c.is_whitespace())
            && is_prefix(&trimmed.to_lowercase())
    }

    /// Case-preserving text after the first whitespace token in the trimmed raw input.
    pub fn rest_raw(&self) -> &str {
        self.raw
            .trim()
            .split_once(|c: char| c.is_whitespace())
            .map(|(_, rest)| rest.trim())
            .unwrap_or("")
    }

    /// Lowercased [`Self::rest_raw`] for case-insensitive matching.
    pub fn rest_normalized(&self) -> String {
        self.rest_raw().to_lowercase()
    }
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
            | "doctor"
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
            | "kill"
            | "quit"
            | "k"
            | "p"
            | "proj"
            | "project"
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
}
