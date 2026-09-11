use super::SshModule;
use luma_application::ActionOutcome;
use luma_domain::{FailureKind, SearchItem};

pub(super) struct RenamePayload {
    pub(super) alias: String,
    pub(super) display_name: String,
}

pub(super) fn parse_rename_query(rest: &str) -> Option<RenamePayload> {
    let rest = rest.trim();
    if rest.len() < 7 || !rest[..7].eq_ignore_ascii_case("rename ") {
        return None;
    }
    let after_prefix = rest[7..].trim_start();
    let mut parts = after_prefix.splitn(2, char::is_whitespace);
    let alias = parts.next()?.trim();
    let display_name = parts.next()?.trim();
    if alias.is_empty() || display_name.is_empty() {
        return None;
    }
    Some(RenamePayload {
        alias: alias.to_string(),
        display_name: display_name.to_string(),
    })
}

impl SshModule {
    pub(super) async fn apply_rename(&self, result: &SearchItem) -> ActionOutcome {
        let Some(payload) = &result.action_payload else {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "rename".into(),
                    message: "missing rename payload".into(),
                },
            };
        };
        let alias = payload
            .get("alias")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim();
        let display_name = payload
            .get("display_name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if alias.is_empty() {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "alias".into(),
                    message: "missing alias".into(),
                },
            };
        }
        if !self.alias_is_known(alias).await {
            return ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "alias".into(),
                    message: format!("unknown ssh host alias: {alias}"),
                },
            };
        }
        let Some(meta) = &self.meta else {
            return ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: "ssh metadata store unavailable".into(),
                    retryable: false,
                },
            };
        };
        match meta.set_display_name(alias, display_name) {
            Ok(()) => {
                let _ = self.refresh().await;
                ActionOutcome::Success {
                    message: Some("display name saved".into()),
                }
            }
            Err(err) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: err.to_string(),
                    retryable: false,
                },
            },
        }
    }
}
