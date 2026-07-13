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
    pub fn parse(raw: impl Into<String>, limit: usize) -> Self {
        let raw = raw.into();
        let normalized = raw.trim().to_lowercase();
        let scope = if let Some((prefix, rest)) = normalized.split_once(|c: char| c.is_whitespace())
        {
            if !rest.is_empty() && is_known_prefix(prefix) {
                QueryScope::Targeted {
                    module: prefix.to_string(),
                }
            } else if is_known_prefix(&normalized) {
                QueryScope::Targeted {
                    module: normalized.clone(),
                }
            } else {
                QueryScope::Global
            }
        } else if is_known_prefix(&normalized) {
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
            | "tr"
            | "translate"
            | "t"
            | "todo"
            | "sec"
            | "secret"
            | "secrets"
            | "gh"
            | "kill"
            | "quit"
            | "k"
            | "p"
            | "proj"
            | "project"
            | "rec"
            | "m"
            | "media"
            | "word"
            | "wb"
            | "win"
            | "wl"
            | "layout"
            | "mb"
            | "menu"
            | "tab"
            | "tabs"
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
}
