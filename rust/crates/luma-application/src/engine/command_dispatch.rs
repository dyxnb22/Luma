use super::*;

pub(super) fn settings_event_value(
    source: &str,
    settings: &crate::ports::AppSettings,
    rows: &[(String, bool, String)],
) -> serde_json::Value {
    let mut value = serde_json::to_value(settings).unwrap_or_else(|_| serde_json::json!({}));
    if let Some(object) = value.as_object_mut() {
        object.insert("source".into(), source.into());
        object.insert(
            "modules".into(),
            rows.iter()
                .map(|(id, enabled, name)| {
                    serde_json::json!({"id": id, "enabled": enabled, "name": name})
                })
                .collect::<Vec<_>>()
                .into(),
        );
    }
    value
}

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
                let mut continue_search_items = Vec::new();
                for module in &modules {
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
                    if module.supports_hub_items()
                        && continue_search_items.len() < super::recall::HUB_CONTINUE_LIMIT
                    {
                        continue_search_items.extend(
                            module
                                .hub_items(
                                    super::recall::HUB_CONTINUE_LIMIT - continue_search_items.len(),
                                )
                                .await,
                        );
                    }
                }
                continue_search_items.truncate(super::recall::HUB_CONTINUE_LIMIT);
                let remaining =
                    super::recall::HUB_CONTINUE_LIMIT.saturating_sub(continue_search_items.len());
                let recall_repo = self.recall.clone();
                let recent_records = recall_repo
                    .as_ref()
                    .and_then(|repo| {
                        repo.list_recent(super::recall::HUB_CONTINUE_LIMIT.saturating_mul(4))
                            .ok()
                    })
                    .unwrap_or_default();
                let live_ids = continue_search_items
                    .iter()
                    .map(|item| item.id.as_str())
                    .collect::<std::collections::HashSet<_>>();
                let mut recent_items = Vec::with_capacity(remaining);
                for record in recent_records {
                    if recent_items.len() >= remaining {
                        break;
                    }
                    if live_ids.contains(record.object_id.as_str()) {
                        continue;
                    }
                    let Some(module) = modules
                        .iter()
                        .find(|module| module.manifest().id.as_str() == record.module_id)
                    else {
                        // Disabled modules retain Recall so re-enabling them can restore ranking.
                        continue;
                    };
                    match tokio::time::timeout(
                        RECALL_REHYDRATION_BOUND,
                        module.rehydrate_recall(&record.object_id),
                    )
                    .await
                    {
                        Ok(Ok(Some(item))) => {
                            if let Some(item) = super::recall::prepare_hub_item(&record, item) {
                                recent_items.push(item);
                            } else if let Some(repo) = recall_repo.as_ref() {
                                let _ = repo.forget(&record.object_id);
                            }
                        }
                        Ok(Ok(None)) => {
                            if let Some(repo) = recall_repo.as_ref() {
                                let _ = repo.forget(&record.object_id);
                            }
                        }
                        Ok(Err(_)) | Err(_) => {
                            // Temporary module/store failures must not destroy durable ranking.
                        }
                    }
                }
                continue_search_items.extend(recent_items);
                let continue_dto = continue_search_items
                    .iter()
                    .map(|item| luma_protocol::HubContinueDto {
                        id: item.id.as_str().to_string(),
                        module_id: item.module_id.as_str().to_string(),
                        kind: item.kind.clone(),
                        title: item.title.clone(),
                        primary_action_id: item.primary_action.id.as_str().to_string(),
                    })
                    .collect::<Vec<_>>();
                seeded.extend(continue_search_items);
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
                        continue_items: continue_dto,
                    })
                    .await;
            }
            Command::LoadWordbookReview { queue } => {
                self.handle_load_wordbook_review(queue).await;
            }
            Command::GetSettings => {
                let (rows, snapshot) = {
                    let g = self.inner.lock().await;
                    let rows = g.registry.list();
                    let snapshot = self
                        .settings
                        .as_ref()
                        .and_then(|repo| repo.load_or_default().ok());
                    (rows, snapshot)
                };
                let snapshot = snapshot.unwrap_or_default();
                let version = snapshot.settings_version;
                let settings = settings_event_value(
                    if self.settings.is_some() {
                        "config_store"
                    } else {
                        "engine_registry"
                    },
                    &snapshot,
                    &rows,
                );
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
                // Every persisted field is module-owned (retention, idle lock, Hub cap, proxy
                // endpoint, roots). Apply the complete saved snapshot so changes take effect in
                // the running workbench rather than only after another root mutation or restart.
                let modules = {
                    let g = self.inner.lock().await;
                    g.registry.enabled_modules().into_iter().collect::<Vec<_>>()
                };
                for module in modules {
                    module.apply_settings(&saved).await;
                }
                let rows = {
                    let g = self.inner.lock().await;
                    g.registry.list()
                };
                let _ = self
                    .emit(Event::SettingsChanged {
                        version: saved.settings_version,
                        settings: settings_event_value("config_store", &saved, &rows),
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
        }
    }
}
