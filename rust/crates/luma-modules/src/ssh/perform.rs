use super::SshModule;
use crate::cancel::await_unless_cancelled;
use luma_application::ActionOutcome;
use luma_domain::FailureKind;
use tokio_util::sync::CancellationToken;

impl SshModule {
    pub(super) async fn set_favorite(&self, alias: &str, favorite: bool) -> ActionOutcome {
        let Some(meta) = &self.meta else {
            return ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: self
                        .meta_error
                        .read()
                        .await
                        .clone()
                        .unwrap_or_else(|| "ssh metadata store unavailable".into()),
                    retryable: false,
                },
            };
        };
        match meta.set_favorite(alias, favorite) {
            Ok(()) => {
                let _ = self.refresh().await;
                ActionOutcome::Success {
                    message: Some(if favorite {
                        "favorited".into()
                    } else {
                        "unfavorited".into()
                    }),
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

    pub(super) async fn copy_alias(
        &self,
        alias: &str,
        cancel: &CancellationToken,
    ) -> ActionOutcome {
        match await_unless_cancelled(cancel, self.pasteboard.write_text(alias)).await {
            Some(Ok(())) => ActionOutcome::Success {
                message: Some("copied alias".into()),
            },
            Some(Err(err)) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: err.to_string(),
                    retryable: false,
                },
            },
            None => ActionOutcome::Cancelled,
        }
    }

    pub(super) async fn reload_config(&self) -> ActionOutcome {
        self.resolved_cache.write().await.clear();
        let _ = self.refresh().await;
        ActionOutcome::Success {
            message: Some("SSH config cache reloaded".into()),
        }
    }

    pub(super) async fn delete_metadata(&self, alias: &str) -> ActionOutcome {
        let Some(meta) = &self.meta else {
            return ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: "ssh metadata store unavailable".into(),
                    retryable: false,
                },
            };
        };
        match meta.delete(alias) {
            Ok(()) => {
                let _ = self.refresh().await;
                ActionOutcome::Success {
                    message: Some("local metadata deleted".into()),
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
