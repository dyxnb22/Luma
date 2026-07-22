use super::*;

impl Engine {
    pub async fn handle_command(&self, command: Command) {
        match command {
            Command::StartSession => self.start_session().await,
            Command::Search { request_id, query } => {
                self.handle_search(request_id, query).await;
            }
            Command::CancelSearch { request_id } => {
                self.cancel_search(&request_id).await;
            }
            Command::ShutdownSession => self.handle_shutdown_session().await,
            Command::SetModuleEnabled { module_id, enabled } => {
                let _ = self.apply_module_enabled(&module_id, enabled).await;
            }
            Command::ExecuteAction {
                operation_id,
                result_id,
                action_id,
                confirmation,
            } => {
                self.handle_execute_action(operation_id, result_id, action_id, confirmation)
                    .await;
            }
            Command::ListActions { result_id } => {
                self.handle_list_actions(result_id).await;
            }
            Command::GetSnapshot => {
                let (items, module_states) = {
                    let g = self.inner.lock().await;
                    let mut items: Vec<SearchItemDto> =
                        g.results_by_id.values().map(SearchItemDto::from).collect();
                    // HashMap iteration order is unstable; match search-chunk ranking.
                    items.sort_by(|a, b| {
                        b.score
                            .partial_cmp(&a.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| a.id.cmp(&b.id))
                    });
                    (items, g.module_states.clone())
                };
                let _ = self
                    .emit(Event::SnapshotLoaded {
                        items,
                        module_states,
                    })
                    .await;
            }
            Command::LoadPreview {
                result_id,
                preview_id,
            } => {
                self.handle_load_preview(result_id, preview_id).await;
            }
            Command::LoadHub => {
                let modules = {
                    let g = self.inner.lock().await;
                    g.registry.enabled_modules()
                };
                let mut windows_dto: Option<luma_protocol::HubWindowsDto> = None;
                let mut seeded: Vec<luma_domain::SearchItem> = Vec::new();
                for module in modules {
                    if windows_dto.is_none() && module.supports_hub_windows() {
                        if let Some(slice) = module.hub_windows().await {
                            for row in &slice.windows {
                                seeded.push(luma_domain::SearchItem {
                                    id: luma_domain::ResultId::new(row.id.clone()),
                                    module_id: luma_domain::ModuleId::new("luma.windows"),
                                    title: row.title.clone(),
                                    subtitle: Some(slice.app_name.clone()),
                                    kind: "window".into(),
                                    score: 50.0,
                                    primary_action: luma_domain::ActionDescriptor {
                                        id: luma_domain::ActionId::new("focus"),
                                        label: "Focus".into(),
                                        risk: luma_domain::ActionRisk::Safe,
                                        confirmation: false,
                                    },
                                    secondary_actions: vec![],
                                    ui_intent: None,
                                    action_payload: None,
                                });
                            }
                            windows_dto = Some(luma_protocol::HubWindowsDto {
                                app_name: slice.app_name,
                                windows: slice
                                    .windows
                                    .into_iter()
                                    .map(|w| luma_protocol::HubWindowDto {
                                        id: w.id,
                                        title: w.title,
                                    })
                                    .collect(),
                                more: slice.more,
                                status: slice.status.map(|s| luma_protocol::HubWindowsStatusDto {
                                    kind: s.kind,
                                    title: s.title,
                                    subtitle: s.subtitle,
                                }),
                            });
                        }
                    }
                }
                let evicted = {
                    let mut g = self.inner.lock().await;
                    g.insert_results_batch(
                        seeded
                            .into_iter()
                            .map(|item| (item.id.as_str().to_string(), item)),
                    )
                };
                if !evicted.is_empty() {
                    let _ = self
                        .emit(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 0,
                            upserts: vec![],
                            removed_ids: evicted,
                        })
                        .await;
                }
                let _ = self
                    .emit(Event::HubLoaded {
                        windows: windows_dto,
                    })
                    .await;
            }
            Command::LoadWordbookReview { queue } => {
                self.handle_load_wordbook_review(queue).await;
            }
            Command::GetSettings => {
                let (rows, version, notes_root, projects_roots, imported_projects) = {
                    let g = self.inner.lock().await;
                    let rows = g.registry.list();
                    let snapshot = self
                        .settings
                        .as_ref()
                        .and_then(|repo| repo.load_or_default().ok());
                    let version = snapshot.as_ref().map(|s| s.settings_version).unwrap_or(0);
                    let notes_root = snapshot.as_ref().and_then(|s| s.notes_root.clone());
                    let projects_roots = snapshot
                        .as_ref()
                        .map(|s| s.projects_roots.clone())
                        .unwrap_or_default();
                    let imported_projects = snapshot
                        .as_ref()
                        .map(|s| s.imported_projects.clone())
                        .unwrap_or_default();
                    (rows, version, notes_root, projects_roots, imported_projects)
                };
                let settings = serde_json::json!({
                    "source": if self.settings.is_some() { "config_store" } else { "engine_registry" },
                    "notes_root": notes_root,
                    "projects_roots": projects_roots,
                    "imported_projects": imported_projects,
                    "modules": rows.iter().map(|(id, enabled, name)| {
                        serde_json::json!({"id": id, "enabled": enabled, "name": name})
                    }).collect::<Vec<_>>(),
                });
                let _ = self
                    .emit(Event::SettingsChanged { version, settings })
                    .await;
            }
            Command::UpdateSettings {
                patch,
                expected_version,
            } => {
                let Some(settings_repo) = &self.settings else {
                    let _ = self.emit(Event::DiagnosticRaised {
                        diagnostic: serde_json::json!({
                            "settings_update": "failed",
                            "message": "no SettingsRepository configured; refusing non-persistent update"
                        }),
                    }).await;
                    return;
                };
                let current = match settings_repo.load_or_default() {
                    Ok(value) => value,
                    Err(err) => {
                        let _ = self.emit(Event::DiagnosticRaised {
                            diagnostic: serde_json::json!({"settings_update": "failed", "message": err.to_string()}),
                        }).await;
                        return;
                    }
                };
                let mut next = current.clone();
                if let Err(err) = next.apply_settings_patch(&patch) {
                    let _ = self
                        .emit(Event::DiagnosticRaised {
                            diagnostic: serde_json::json!({
                                "settings_update": "failed",
                                "message": err,
                            }),
                        })
                        .await;
                    return;
                }
                let roots_changed = next.notes_root != current.notes_root
                    || next.records_root != current.records_root
                    || next.projects_roots != current.projects_roots
                    || next.imported_projects != current.imported_projects
                    || next.notes_exclude_patterns != current.notes_exclude_patterns;
                let saved = match settings_repo.update_cas(expected_version, next) {
                    Ok(value) => value,
                    Err(err) => {
                        let _ = self
                            .emit(Event::DiagnosticRaised {
                                diagnostic: serde_json::json!({
                                    "settings_update": "failed",
                                    "expected_version": expected_version,
                                    "message": err.to_string()
                                }),
                            })
                            .await;
                        return;
                    }
                };
                let changes: Vec<(String, bool)> = {
                    let g = self.inner.lock().await;
                    saved
                        .enabled_modules
                        .iter()
                        .filter(|(id, enabled)| g.registry.is_enabled(id) != **enabled)
                        .map(|(id, enabled)| (id.clone(), *enabled))
                        .collect()
                };
                for (id, enabled) in changes {
                    let _ = self.apply_module_enabled(&id, enabled).await;
                }
                if roots_changed {
                    let modules = {
                        let g = self.inner.lock().await;
                        g.registry.enabled_modules().into_iter().collect::<Vec<_>>()
                    };
                    for module in modules {
                        module.apply_settings(&saved).await;
                    }
                }
                let rows = {
                    let g = self.inner.lock().await;
                    g.registry.list()
                };
                let _ = self
                    .emit(Event::SettingsChanged {
                        version: saved.settings_version,
                        settings: serde_json::json!({
                            "source": "config_store",
                            "modules": rows.iter().map(|(id, enabled, name)| {
                                serde_json::json!({"id": id, "enabled": enabled, "name": name})
                            }).collect::<Vec<_>>(),
                            "notes_root": saved.notes_root,
                            "projects_roots": saved.projects_roots,
                            "imported_projects": saved.imported_projects,
                            "notes_exclude_patterns": saved.notes_exclude_patterns,
                        }),
                    })
                    .await;
            }
            Command::CancelOperation { operation_id } => {
                self.handle_cancel_operation(operation_id).await;
            }
            Command::RecordRecipeRun {
                recipe_id,
                result,
                now_unix,
            } => {
                self.handle_record_recipe_run(recipe_id, result, now_unix)
                    .await;
            }
            Command::RefreshWordbookReviewStats => {
                self.handle_refresh_wordbook_review_stats().await;
            }
            Command::SshSessionEnded { alias, exit_code } => {
                self.handle_ssh_session_ended(alias, exit_code).await;
            }
        }
    }
}
