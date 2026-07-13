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
    pub fn parse_with_prefixes(
        raw: impl Into<String>,
        limit: usize,
        is_prefix: impl Fn(&str) -> bool,
    ) -> Self {
        let raw = raw.into();
        let normalized = raw.trim().to_lowercase();
        let scope = if let Some((prefix, rest)) = normalized.split_once(|c: char| c.is_whitespace())
        {
            if !rest.is_empty() && is_prefix(prefix) {
                QueryScope::Targeted {
                    module: prefix.to_string(),
                }
            } else if is_prefix(&normalized) {
                QueryScope::Targeted {
                    module: normalized.clone(),
                }
            } else {
                QueryScope::Global
            }
        } else if is_prefix(&normalized) {
            QueryScope::Targeted {
                module: normalized.clone(),
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
            | "t"
            | "todo"
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
    }

    #[test]
    fn parse_with_prefixes_uses_injected_set() {
        let q = Query::parse_with_prefixes("custom Hello", 20, |t| t == "custom");
        assert!(matches!(q.scope, QueryScope::Targeted { ref module } if module == "custom"));
        assert_eq!(q.rest_raw(), "Hello");
    }
}
