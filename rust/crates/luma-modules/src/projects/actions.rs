use super::preview::format_projects_directory_preview;
use super::ProjectsModule;
use crate::cancel::await_unless_cancelled;
use luma_application::{
    ActionOutcome, ActionRequest, ProjectDirectoryListing, ProjectOpenScope, ProjectWorkspaceError,
};
use luma_domain::{ActionDescriptor, ActionId, ActionRisk, FailureKind, SearchItem};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

impl ProjectsModule {
    pub(super) async fn module_actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.id.as_str() == "proj:configure" || result.kind == "not_configured" {
            return vec![ActionDescriptor {
                id: ActionId::new("seed_config"),
                label: "Show command".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.kind == "directory"
            || result.kind == "imported"
            || result.primary_action.id.as_str() == "browse"
        {
            let mut actions = vec![
                ActionDescriptor {
                    id: ActionId::new("browse"),
                    label: "Browse".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
                ActionDescriptor {
                    id: ActionId::new("open"),
                    label: "Open in Finder".into(),
                    risk: ActionRisk::Safe,
                    confirmation: false,
                },
            ];
            if result.kind == "directory" && result.primary_action.id.as_str() == "import_project" {
                actions.insert(
                    0,
                    ActionDescriptor {
                        id: ActionId::new("import_project"),
                        label: "Import project".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                );
            }
            return actions;
        }
        if result.primary_action.id.as_str() == "import_project" {
            return vec![ActionDescriptor {
                id: ActionId::new("import_project"),
                label: "Import".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        if result.primary_action.id.as_str() == "remove_project" {
            return vec![ActionDescriptor {
                id: ActionId::new("remove_project"),
                label: "Remove".into(),
                risk: ActionRisk::Confirm,
                confirmation: true,
            }];
        }
        if result.kind == "status"
            || result.kind == "onboarding"
            || result.kind == "unavailable"
            || result.primary_action.id.as_str() == "noop"
        {
            return vec![ActionDescriptor {
                id: ActionId::new("noop"),
                label: "OK".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            }];
        }
        let mut actions = vec![ActionDescriptor {
            id: ActionId::new("open"),
            label: "Open".into(),
            risk: ActionRisk::Safe,
            confirmation: false,
        }];
        if result.kind == "project" {
            actions.push(ActionDescriptor {
                id: ActionId::new("browse"),
                label: "Browse".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            });
        }
        actions
    }

    pub(super) async fn module_preview(&self, result: &SearchItem) -> Option<String> {
        if result.kind == "directory" || result.id.as_str().starts_with("browse:proj:") {
            let path_part = result
                .subtitle
                .as_deref()?
                .split(" — ")
                .next()
                .unwrap_or("");
            let cancel = CancellationToken::new();
            let roots = self.roots.read().await.clone();
            let directory = self
                .workspace
                .resolve_browse_path(PathBuf::from(path_part), roots, cancel.clone())
                .await
                .ok()?;
            let listing = match self.workspace.list_directory(directory, cancel).await {
                Ok(listing) => listing,
                Err(ProjectWorkspaceError::NotFound(_)) => ProjectDirectoryListing::default(),
                Err(_) => return None,
            };
            return Some(format_projects_directory_preview(listing));
        }
        result
            .subtitle
            .clone()
            .or_else(|| Some(result.title.clone()))
    }

    pub(super) async fn module_perform(
        &self,
        action: ActionRequest,
        cancel: CancellationToken,
    ) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        match action.action.id.as_str() {
            "noop" => ActionOutcome::Success {
                message: Some("ok".into()),
            },
            "configure" => ActionOutcome::Failed {
                kind: FailureKind::NotConfigured {
                    remediation: "Run: luma config set --projects-root ~/dev".into(),
                },
            },
            "browse" => ActionOutcome::Failed {
                kind: FailureKind::InvalidInput {
                    field: "action".into(),
                    message: "browse is search-driven; use `/proj browse <path>`".into(),
                },
            },
            "import_project" => {
                let path_str = action
                    .result
                    .action_payload
                    .as_ref()
                    .and_then(|p| p.get("path"))
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        action
                            .result
                            .subtitle
                            .as_deref()
                            .map(|s| s.split(" — ").next().unwrap_or(s))
                    })
                    .or_else(|| action.result.id.as_str().strip_prefix("proj:import:"));
                let Some(path_str) = path_str else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "path".into(),
                            message: "missing import path".into(),
                        },
                    };
                };
                match await_unless_cancelled(
                    &cancel,
                    self.workspace
                        .resolve_import_path(PathBuf::from(path_str), cancel.clone()),
                )
                .await
                {
                    None | Some(Err(ProjectWorkspaceError::Cancelled)) => ActionOutcome::Cancelled,
                    Some(Ok(canon)) => ActionOutcome::SettingsMutation {
                        patch: serde_json::json!({
                            "import_project": canon.display().to_string()
                        }),
                    },
                    Some(Err(error)) => ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: error.to_string(),
                        },
                    },
                }
            }
            "remove_project" => {
                let key = action
                    .result
                    .action_payload
                    .as_ref()
                    .and_then(|p| p.get("name"))
                    .and_then(|v| v.as_str())
                    .or_else(|| action.result.id.as_str().strip_prefix("proj:remove:"));
                let Some(key) = key else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "name".into(),
                            message: "missing project name".into(),
                        },
                    };
                };
                ActionOutcome::SettingsMutation {
                    patch: serde_json::json!({ "remove_project": key }),
                }
            }
            "open" => {
                let path_str = action
                    .result
                    .id
                    .as_str()
                    .strip_prefix("browse:proj:")
                    .or_else(|| action.result.id.as_str().strip_prefix("proj:"))
                    .or(action
                        .result
                        .subtitle
                        .as_deref()
                        .map(|s| s.split(" — ").next().unwrap_or(s)));
                let Some(path_str) = path_str else {
                    return ActionOutcome::Failed {
                        kind: FailureKind::InvalidInput {
                            field: "result_id".into(),
                            message: "expected proj:<path>".into(),
                        },
                    };
                };
                let path = PathBuf::from(path_str);
                let imported = self.imported.read().await.clone();
                let is_imported = imported.iter().any(|p| p.path == path_str);
                let scope = if is_imported {
                    ProjectOpenScope::ImportedProject
                } else {
                    ProjectOpenScope::ProjectRoots
                };
                let roots = self.roots.read().await.clone();
                let open_path = match await_unless_cancelled(
                    &cancel,
                    self.workspace
                        .resolve_open_path(path, scope, roots, cancel.clone()),
                )
                .await
                {
                    None | Some(Err(ProjectWorkspaceError::Cancelled)) => {
                        return ActionOutcome::Cancelled;
                    }
                    Some(Ok(path)) => path,
                    Some(Err(ProjectWorkspaceError::Denied(reason))) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::SecurityDenied { reason },
                        };
                    }
                    Some(Err(ProjectWorkspaceError::NotFound(_))) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::NotFound {
                                entity: path_str.into(),
                            },
                        };
                    }
                    Some(Err(ProjectWorkspaceError::Unavailable(reason))) => {
                        return ActionOutcome::Failed {
                            kind: FailureKind::Unavailable {
                                reason,
                                retryable: true,
                            },
                        };
                    }
                };
                match await_unless_cancelled(&cancel, self.opener.open(&open_path)).await {
                    None => ActionOutcome::Cancelled,
                    Some(Ok(())) => ActionOutcome::Success {
                        message: Some("opened".into()),
                    },
                    Some(Err(error)) => ActionOutcome::Failed {
                        kind: FailureKind::Unavailable {
                            reason: error.to_string(),
                            retryable: true,
                        },
                    },
                }
            }
            other => ActionOutcome::Failed {
                kind: FailureKind::NotFound {
                    entity: other.into(),
                },
            },
        }
    }

    pub(super) async fn apply_module_settings(&self, settings: &luma_application::AppSettings) {
        let roots: Vec<PathBuf> = settings.projects_roots.iter().map(PathBuf::from).collect();
        *self.roots.write().await = roots;
        *self.imported.write().await = settings.imported_projects.clone();
    }
}
