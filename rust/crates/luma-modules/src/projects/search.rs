use super::preview::imported_index;
use super::ProjectsModule;
use crate::cancel::await_unless_cancelled;
use luma_application::{ProjectDirectoryListing, ProjectWorkspaceError, SearchSink};
use luma_domain::{ActionRisk, Query};
use luma_protocol::{Event, SearchItemDto, UiIntent};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

impl ProjectsModule {
    pub(super) async fn is_imported_path(&self, path: &Path) -> bool {
        let canonical_path = path.display().to_string();
        self.imported
            .read()
            .await
            .iter()
            .any(|project| project.path == canonical_path)
    }

    pub(super) async fn search_projects(
        &self,
        query: Query,
        sink: SearchSink,
        cancel: CancellationToken,
    ) {
        let roots = self.roots.read().await.clone();
        let imported = self.imported.read().await.clone();
        let rest_norm = query
            .normalized
            .split_once(|c: char| c.is_whitespace())
            .map(|(_, r)| r.trim().to_string())
            .unwrap_or_default();
        let rest_raw = query.rest_raw().trim().to_string();
        let rest_check = rest_raw.to_lowercase();

        // Drill-down: verb case-insensitive; path payload from rest_raw (preserve case).
        if rest_check == "browse"
            || rest_check.starts_with("browse ")
            || rest_check.starts_with("ls ")
        {
            if roots.is_empty() {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "proj:configure".into(),
                            module_id: "luma.projects".into(),
                            title: "Add a project browse root".into(),
                            subtitle: Some("Run: luma config set --projects-root ~/dev".into()),
                            kind: "not_configured".into(),
                            score: 0.0,
                            primary_action_id: "seed_config".into(),
                            primary_action_label: "Show command".into(),
                            ui_intent: Some(UiIntent::SeedConfig),
                            action_payload: None,
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
            let path_arg = rest_raw
                .strip_prefix("browse")
                .or_else(|| rest_raw.strip_prefix("Browse"))
                .or_else(|| rest_raw.strip_prefix("BROWSE"))
                .or_else(|| rest_raw.strip_prefix("ls"))
                .or_else(|| rest_raw.strip_prefix("LS"))
                .unwrap_or("")
                .trim();
            let target = if path_arg.is_empty() {
                None
            } else {
                Some(PathBuf::from(path_arg))
            };
            let browse_label = target
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "project roots".into());
            let mut upserts = Vec::new();
            let mut listing_truncated = false;
            if let Some(dir) = target {
                let denied_label = dir.display().to_string();
                let dir = match await_unless_cancelled(
                    &cancel,
                    self.workspace
                        .resolve_browse_path(dir, roots.clone(), cancel.clone()),
                )
                .await
                {
                    None | Some(Err(ProjectWorkspaceError::Cancelled)) => return,
                    Some(Ok(dir)) => dir,
                    Some(Err(_)) => {
                        let _ = sink
                            .send(Event::ResultsChunk {
                                request_id: String::new(),
                                sequence: 1,
                                upserts: vec![SearchItemDto {
                                    id: "proj:denied".into(),
                                    module_id: "luma.projects".into(),
                                    title: "Path outside project roots".into(),
                                    subtitle: Some(denied_label),
                                    kind: "unavailable".into(),
                                    score: 0.0,
                                    primary_action_id: "noop".into(),
                                    primary_action_label: "Unavailable".into(),
                                    ..Default::default()
                                }],
                                removed_ids: vec![],
                            })
                            .await;
                        return;
                    }
                };
                let listing = match await_unless_cancelled(
                    &cancel,
                    self.workspace.list_directory(dir, cancel.clone()),
                )
                .await
                {
                    None | Some(Err(ProjectWorkspaceError::Cancelled)) => return,
                    Some(Ok(listing)) => listing,
                    // Preserve browse's historical empty-folder result for a missing drill-down
                    // target; it remains safely contained because resolution happened first.
                    Some(Err(ProjectWorkspaceError::NotFound(_))) => {
                        ProjectDirectoryListing::default()
                    }
                    Some(Err(error)) => {
                        let _ = sink
                            .send(Event::ResultsChunk {
                                request_id: String::new(),
                                sequence: 1,
                                upserts: vec![SearchItemDto {
                                    id: "proj:browse-unavailable".into(),
                                    module_id: "luma.projects".into(),
                                    title: "Project folder unavailable".into(),
                                    subtitle: Some(error.to_string()),
                                    kind: "unavailable".into(),
                                    score: 0.0,
                                    primary_action_id: "noop".into(),
                                    primary_action_label: "Unavailable".into(),
                                    ..Default::default()
                                }],
                                removed_ids: vec![],
                            })
                            .await;
                        return;
                    }
                };
                listing_truncated = listing.truncated;
                for entry in listing.entries {
                    if cancel.is_cancelled() {
                        return;
                    }
                    let path = entry.path;
                    if entry.is_directory {
                        let imported = self.is_imported_path(&path).await;
                        upserts.push(SearchItemDto {
                            id: format!("browse:proj:{}", path.display()),
                            module_id: "luma.projects".into(),
                            title: format!("{}/", entry.name),
                            subtitle: Some(path.display().to_string()),
                            kind: if imported {
                                "imported".into()
                            } else {
                                "directory".into()
                            },
                            score: 80.0,
                            primary_action_id: if imported {
                                "browse".into()
                            } else {
                                "import_project".into()
                            },
                            primary_action_label: if imported {
                                "Browse".into()
                            } else {
                                "Import project".into()
                            },
                            ui_intent: Some(UiIntent::Browse).filter(|_| imported),
                            action_payload: None,
                            ..Default::default()
                        });
                    } else {
                        upserts.push(SearchItemDto {
                            id: format!("proj:{}", path.display()),
                            module_id: "luma.projects".into(),
                            title: entry.name,
                            subtitle: Some(path.display().to_string()),
                            kind: "file".into(),
                            score: 70.0,
                            primary_action_id: "open".into(),
                            primary_action_label: "Open".into(),
                            ..Default::default()
                        });
                    }
                }
            } else {
                for root in &roots {
                    upserts.push(SearchItemDto {
                        id: format!("browse:proj:{}", root.display()),
                        module_id: "luma.projects".into(),
                        title: format!(
                            "{}/",
                            root.file_name().and_then(|s| s.to_str()).unwrap_or("root")
                        ),
                        subtitle: Some(root.display().to_string()),
                        kind: "directory".into(),
                        score: 90.0,
                        primary_action_id: "browse".into(),
                        primary_action_label: "Browse".into(),
                        ui_intent: Some(UiIntent::Browse),
                        action_payload: None,
                        ..Default::default()
                    });
                }
            }
            if upserts.is_empty() {
                upserts.push(SearchItemDto {
                    id: if listing_truncated {
                        "proj:browse-truncated".into()
                    } else {
                        "proj:browse-empty".into()
                    },
                    module_id: "luma.projects".into(),
                    title: if listing_truncated {
                        "Folder listing is limited".into()
                    } else {
                        "Empty folder".into()
                    },
                    subtitle: Some(browse_label),
                    kind: "status".into(),
                    score: 50.0,
                    primary_action_id: "noop".into(),
                    primary_action_label: "OK".into(),
                    ..Default::default()
                });
            }
            upserts.truncate(query.limit);
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts,
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        if rest_check == "add"
            || rest_check == "import"
            || rest_check.starts_with("add ")
            || rest_check.starts_with("import ")
        {
            let path_str = rest_raw
                .split_once(char::is_whitespace)
                .map(|(_, tail)| tail.trim())
                .unwrap_or("");
            if path_str.is_empty() {
                let usage = if rest_check == "import" {
                    "Usage: proj import /path/to/project"
                } else {
                    "Usage: proj add /path/to/project"
                };
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "proj:import-usage".into(),
                            module_id: "luma.projects".into(),
                            title: "Import a project directory".into(),
                            subtitle: Some(usage.into()),
                            kind: "status".into(),
                            score: 50.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "OK".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
            let path = PathBuf::from(path_str);
            match await_unless_cancelled(
                &cancel,
                self.workspace.resolve_import_path(path, cancel.clone()),
            )
            .await
            {
                None | Some(Err(ProjectWorkspaceError::Cancelled)) => return,
                Some(Ok(canon)) => {
                    let title = canon
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("project")
                        .to_string();
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 1,
                            upserts: vec![SearchItemDto {
                                id: format!("proj:import:{}", canon.display()),
                                module_id: "luma.projects".into(),
                                title: format!("Import {title}"),
                                subtitle: Some(canon.display().to_string()),
                                kind: "command".into(),
                                score: 95.0,
                                primary_action_id: "import_project".into(),
                                primary_action_label: "Import".into(),
                                action_payload: Some(serde_json::json!({
                                    "path": canon.display().to_string()
                                })),
                                ..Default::default()
                            }],
                            removed_ids: vec![],
                        })
                        .await;
                }
                Some(Err(err)) => {
                    let _ = sink
                        .send(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 1,
                            upserts: vec![SearchItemDto {
                                id: "proj:import-denied".into(),
                                module_id: "luma.projects".into(),
                                title: "Cannot import project".into(),
                                subtitle: Some(err.to_string()),
                                kind: "unavailable".into(),
                                score: 0.0,
                                primary_action_id: "noop".into(),
                                primary_action_label: "Unavailable".into(),
                                ..Default::default()
                            }],
                            removed_ids: vec![],
                        })
                        .await;
                }
            }
            return;
        }

        if rest_check == "remove" || rest_check.starts_with("remove ") {
            let key = rest_raw
                .split_once(char::is_whitespace)
                .map(|(_, tail)| tail.trim())
                .unwrap_or("");
            if key.is_empty() {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "proj:remove-usage".into(),
                            module_id: "luma.projects".into(),
                            title: "Remove an imported project".into(),
                            subtitle: Some("Usage: proj remove project-name".into()),
                            kind: "status".into(),
                            score: 50.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "OK".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: format!("proj:remove:{key}"),
                        module_id: "luma.projects".into(),
                        title: format!("Remove {key}"),
                        subtitle: Some("Removes import config only — not the directory".into()),
                        kind: "command".into(),
                        score: 95.0,
                        primary_action_id: "remove_project".into(),
                        primary_action_label: "Remove".into(),
                        primary_action_confirmation: true,
                        primary_action_risk: ActionRisk::Confirm,
                        action_payload: Some(serde_json::json!({ "name": key })),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let needle = rest_norm;
        if imported.is_empty() && needle.is_empty() {
            let _ = sink
                .send(Event::ResultsChunk {
                    request_id: String::new(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: "proj:not-configured".into(),
                        module_id: "luma.projects".into(),
                        title: "No imported projects".into(),
                        subtitle: Some(
                            "proj add /path · proj browse · luma config set --projects-root ~/dev"
                                .into(),
                        ),
                        kind: "not_configured".into(),
                        score: 0.0,
                        primary_action_id: "seed_config".into(),
                        primary_action_label: "Show command".into(),
                        ui_intent: Some(UiIntent::SeedConfig),
                        action_payload: None,
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            return;
        }

        let imported_paths = imported
            .iter()
            .map(|project| PathBuf::from(&project.path))
            .collect();
        let statuses = match await_unless_cancelled(
            &cancel,
            self.workspace
                .imported_project_statuses(imported_paths, cancel.clone()),
        )
        .await
        {
            None | Some(Err(ProjectWorkspaceError::Cancelled)) => return,
            Some(Ok(statuses)) => statuses,
            Some(Err(error)) => {
                let _ = sink
                    .send(Event::ResultsChunk {
                        request_id: String::new(),
                        sequence: 1,
                        upserts: vec![SearchItemDto {
                            id: "proj:workspace-unavailable".into(),
                            module_id: "luma.projects".into(),
                            title: "Project workspace unavailable".into(),
                            subtitle: Some(error.to_string()),
                            kind: "unavailable".into(),
                            score: 0.0,
                            primary_action_id: "noop".into(),
                            primary_action_label: "Unavailable".into(),
                            ..Default::default()
                        }],
                        removed_ids: vec![],
                    })
                    .await;
                return;
            }
        };
        let index = imported_index(&imported, &statuses);
        let mut upserts = Vec::new();
        for p in index {
            if cancel.is_cancelled() {
                return;
            }
            if needle.is_empty() || p.name.to_lowercase().contains(&needle) {
                let (kind, primary, label) = if p.missing {
                    ("unavailable", "remove_project", "Remove missing project")
                } else {
                    ("project", "open", "Open")
                };
                upserts.push(SearchItemDto {
                    id: format!("proj:{}", p.path.display()),
                    module_id: "luma.projects".into(),
                    title: p.name.clone(),
                    subtitle: Some(if p.missing {
                        format!("{} — path missing", p.path.display())
                    } else {
                        p.path.display().to_string()
                    }),
                    kind: kind.into(),
                    score: if p.missing { 40.0 } else { 65.0 },
                    primary_action_id: primary.into(),
                    primary_action_label: label.into(),
                    primary_action_confirmation: p.missing,
                    primary_action_risk: if p.missing {
                        ActionRisk::Confirm
                    } else {
                        ActionRisk::Safe
                    },
                    action_payload: if p.missing {
                        Some(serde_json::json!({ "name": p.name }))
                    } else {
                        None
                    },
                    ..Default::default()
                });
            }
        }
        if upserts.is_empty() {
            let (title, subtitle) = if needle.is_empty() {
                (
                    "No imported projects".into(),
                    "proj add /path · proj browse".into(),
                )
            } else {
                (
                    format!("No projects matching \"{needle}\""),
                    "Try another query · proj browse".into(),
                )
            };
            upserts.push(SearchItemDto {
                id: "proj:no-matches".into(),
                module_id: "luma.projects".into(),
                title,
                subtitle: Some(subtitle),
                kind: "status".into(),
                score: 50.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            });
        }
        upserts.truncate(query.limit);
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts,
                removed_ids: vec![],
            })
            .await;
    }
}
