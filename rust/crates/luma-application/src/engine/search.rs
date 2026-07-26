use super::*;

impl Engine {
    pub(super) async fn cancel_search_task(task: SearchTask) {
        task.cancel.cancel();
        let abort = task.handle.abort_handle();
        match tokio::time::timeout(SEARCH_CANCEL_BOUND, task.handle).await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                if !err.is_cancelled() {
                    warn!(?err, "search supervisor ended with error during cancel");
                }
            }
            Err(_) => {
                abort.abort();
            }
        }
    }

    /// Bounded await for an operation JoinHandle (mirrors search cancel).
    pub(super) async fn await_operation_handle(handle: JoinHandle<()>) -> bool {
        let abort = handle.abort_handle();
        match tokio::time::timeout(OPERATION_CANCEL_BOUND, handle).await {
            Ok(Ok(())) => true,
            Ok(Err(err)) => {
                if !err.is_cancelled() {
                    warn!(?err, "operation task ended with error during cancel");
                }
                true
            }
            Err(_) => {
                abort.abort();
                false
            }
        }
    }

    /// Cancel one search under `search_lifecycle`. Emits `SearchCancelled` exactly once when
    /// the request was known (running, pending, or pre-registered intent).
    pub(super) async fn cancel_search(&self, request_id: &str) {
        let _lifecycle = self.search_lifecycle.lock().await;
        self.cancel_search_locked(request_id).await;
    }

    pub(super) async fn cancel_search_locked(&self, request_id: &str) {
        if let Some(cancel) = {
            let mut g = self.inner.lock().await;
            g.pending_searches.remove(request_id)
        } {
            cancel.cancel();
            let _ = self
                .emit(Event::SearchCancelled {
                    request_id: request_id.to_string(),
                })
                .await;
            return;
        }

        let task = {
            let mut g = self.inner.lock().await;
            g.searches.remove(request_id)
        };
        if let Some(task) = task {
            Self::cancel_search_task(task).await;
            let _ = self
                .emit(Event::SearchCancelled {
                    request_id: request_id.to_string(),
                })
                .await;
            return;
        }

        // Search not registered yet — remember so a racing handle_search aborts,
        // and emit now so clients are not left without a terminal event.
        {
            let mut g = self.inner.lock().await;
            g.record_cancel_intent(request_id.to_string());
        }
        let _ = self
            .emit(Event::SearchCancelled {
                request_id: request_id.to_string(),
            })
            .await;
    }

    /// Cancel every running search and emit one `SearchCancelled` per request.
    /// Caller must hold `search_lifecycle`.
    pub(super) async fn cancel_all_searches_locked(&self) {
        let tasks = {
            let mut g = self.inner.lock().await;
            g.searches.drain().collect::<Vec<_>>()
        };
        for (request_id, task) in tasks {
            Self::cancel_search_task(task).await;
            let _ = self
                .emit(Event::SearchCancelled {
                    request_id: request_id.clone(),
                })
                .await;
        }
        let pending = {
            let mut g = self.inner.lock().await;
            g.pending_searches.drain().collect::<Vec<_>>()
        };
        for (request_id, cancel) in pending {
            cancel.cancel();
            let _ = self.emit(Event::SearchCancelled { request_id }).await;
        }
    }

    pub(super) async fn handle_search(&self, request_id: String, query_raw: String) {
        let _lifecycle = self.search_lifecycle.lock().await;

        // Cancel-before-registration: honor intent (SearchCancelled already emitted).
        let search_started = std::time::Instant::now();
        let pre_cancelled = {
            let mut g = self.inner.lock().await;
            g.take_cancel_intent(&request_id)
        };
        if pre_cancelled {
            return;
        }

        // Bare trigger (`n`) — finish early before SearchStarted/ResultsReset flash.
        let incomplete = {
            let g = self.inner.lock().await;
            let triggers = g.registry.all_triggers();
            let query = Query::parse_with_prefixes_strict(&query_raw, 50, |token| {
                is_meta_prefix(token) || triggers.iter().any(|t| t == token)
            });
            query.is_incomplete_trigger(|token| {
                is_meta_prefix(token) || triggers.iter().any(|t| t == token)
            })
        };
        if incomplete {
            self.cancel_all_searches_locked().await;
            let _ = self
                .emit(Event::SearchFinished {
                    request_id,
                    total: 0,
                    elapsed_ms: search_started.elapsed().as_millis() as u64,
                })
                .await;
            return;
        }

        self.cancel_all_searches_locked().await;
        {
            let mut g = self.inner.lock().await;
            g.clear_results();
        }

        let cancel = {
            let g = self.inner.lock().await;
            g.session_cancel.child_token()
        };
        {
            let mut g = self.inner.lock().await;
            g.pending_searches
                .insert(request_id.clone(), cancel.clone());
        }

        // Intent recorded while we held lifecycle is impossible; token cancel means
        // cancel_search_locked already emitted for this pending id.
        if cancel.is_cancelled() {
            let mut g = self.inner.lock().await;
            g.pending_searches.remove(&request_id);
            return;
        }

        let _ = self
            .emit(Event::SearchStarted {
                request_id: request_id.clone(),
            })
            .await;
        let _ = self
            .emit(Event::ResultsReset {
                request_id: request_id.clone(),
            })
            .await;

        let query = {
            let g = self.inner.lock().await;
            let triggers = g.registry.all_triggers();
            Query::parse_with_prefixes_strict(query_raw, 50, |token| {
                is_meta_prefix(token) || triggers.iter().any(|t| t == token)
            })
        };
        if let QueryScope::InvalidCommand { command } = &query.scope {
            let title = if command.is_empty() {
                "Enter a slash command".to_string()
            } else {
                format!("Unknown command: /{command}")
            };
            let _ = self
                .emit(Event::ResultsChunk {
                    request_id: request_id.clone(),
                    sequence: 1,
                    upserts: vec![SearchItemDto {
                        id: format!("system:invalid-command:{command}"),
                        module_id: "luma.system".into(),
                        title,
                        subtitle: Some("Use /help to view available commands.".into()),
                        kind: "command_error".into(),
                        score: 0.0,
                        primary_action_id: "status".into(),
                        primary_action_label: "Status".into(),
                        ..Default::default()
                    }],
                    removed_ids: vec![],
                })
                .await;
            {
                let mut g = self.inner.lock().await;
                g.pending_searches.remove(&request_id);
            }
            let _ = self
                .emit(Event::SearchFinished {
                    request_id,
                    total: 1,
                    elapsed_ms: search_started.elapsed().as_millis() as u64,
                })
                .await;
            return;
        }
        let is_global_search = matches!(query.scope, QueryScope::Global);
        let recall_records = if is_global_search {
            self.recall
                .as_ref()
                .and_then(|repo| repo.list_recent(1_000).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let recent_project = recall_records
            .iter()
            .find_map(|record| record.project_path.clone());
        let recall_records = recall_records
            .into_iter()
            .map(|record| (record.object_id.clone(), record))
            .collect::<HashMap<_, _>>();
        let modules: Vec<Arc<dyn LumaModule>> = {
            let g = self.inner.lock().await;
            match &query.scope {
                QueryScope::Targeted { module } => {
                    g.registry.resolve_trigger(module).into_iter().collect()
                }
                QueryScope::Global => g.registry.contributing(),
                QueryScope::InvalidCommand { .. } => Vec::new(),
            }
        };

        if modules.is_empty() {
            {
                let mut g = self.inner.lock().await;
                g.pending_searches.remove(&request_id);
            }
            let _ = self
                .emit(Event::SearchFinished {
                    request_id,
                    total: 0,
                    elapsed_ms: search_started.elapsed().as_millis() as u64,
                })
                .await;
            return;
        }

        if cancel.is_cancelled() {
            {
                let mut g = self.inner.lock().await;
                g.pending_searches.remove(&request_id);
            }
            // cancel_search_locked already emitted SearchCancelled for pending.
            return;
        }

        let (chunk_tx, mut chunk_rx) = mpsc::channel::<Event>(64);
        let engine = self.clone_inner();
        let request_for_task = request_id.clone();
        let cancel_for_task = cancel.clone();

        let mut module_cancels = HashMap::new();
        let mut set = JoinSet::new();
        for module in modules {
            let mut q = query.clone();
            if is_global_search {
                q.limit = q.limit.min(super::recall::GLOBAL_RESULTS_PER_MODULE);
            }
            let sink = chunk_tx.clone();
            let module_id = module.manifest().id.as_str().to_string();
            let token = cancel_for_task.child_token();
            module_cancels.insert(module_id, token.clone());
            set.spawn(async move {
                module.search(q, sink, token).await;
            });
        }
        drop(chunk_tx);

        let collector_handle = tokio::spawn({
            let request_id = request_id.clone();
            let engine = engine.clone();
            let cancel_for_collect = cancel_for_task.clone();
            let started = search_started;
            let recall_records = recall_records.clone();
            let recent_project = recent_project.clone();
            async move {
                let mut sequence = 0u64;
                let now_unix = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_secs() as i64)
                    .unwrap_or(0);
                while let Some(ev) = chunk_rx.recv().await {
                    if cancel_for_collect.is_cancelled() {
                        break;
                    }
                    if let Event::ResultsChunk {
                        upserts,
                        removed_ids,
                        ..
                    } = ev
                    {
                        sequence += 1;
                        let mut g = engine.lock().await;
                        let mut upserts: Vec<_> = upserts
                            .into_iter()
                            .filter(|u| g.registry.is_enabled(&u.module_id))
                            .filter(|u| {
                                !is_global_search
                                    || super::recall::visible_in_global_search(
                                        &u.clone().into_domain(),
                                    )
                            })
                            .collect();
                        let mut all_removed = removed_ids;
                        for id in &all_removed {
                            g.remove_result(id);
                        }
                        let batch: Vec<_> = upserts
                            .iter_mut()
                            .map(|u| {
                                let mut item = u.clone().into_domain();
                                if is_global_search {
                                    super::recall::apply_recall_score(
                                        &mut item,
                                        &recall_records,
                                        now_unix,
                                        recent_project.as_deref(),
                                    );
                                    u.score = item.score;
                                }
                                (u.id.clone(), item)
                            })
                            .collect();
                        let evicted = g.insert_results_batch(batch);
                        all_removed.extend(evicted);
                        if upserts.is_empty() && all_removed.is_empty() {
                            continue;
                        }
                        Self::emit_from_inner(
                            &g,
                            Event::ResultsChunk {
                                request_id: request_id.clone(),
                                sequence,
                                upserts,
                                removed_ids: all_removed,
                            },
                        );
                    }
                }
                if !cancel_for_collect.is_cancelled() {
                    let mut g = engine.lock().await;
                    if is_global_search {
                        let ranked = super::recall::fair_global_results(
                            g.results_by_id.values().cloned().collect(),
                        );
                        let keep = ranked
                            .iter()
                            .map(|item| item.id.as_str().to_string())
                            .collect::<std::collections::HashSet<_>>();
                        let removed_ids = g
                            .results_by_id
                            .keys()
                            .filter(|id| !keep.contains(*id))
                            .cloned()
                            .collect::<Vec<_>>();
                        g.clear_results();
                        g.insert_results_batch(
                            ranked
                                .iter()
                                .cloned()
                                .map(|item| (item.id.as_str().to_string(), item)),
                        );
                        sequence += 1;
                        Self::emit_from_inner(
                            &g,
                            Event::ResultsChunk {
                                request_id: request_id.clone(),
                                sequence,
                                upserts: ranked.iter().map(SearchItemDto::from).collect(),
                                removed_ids,
                            },
                        );
                    }
                    let total = g.results_by_id.len();
                    Self::emit_from_inner(
                        &g,
                        Event::SearchFinished {
                            request_id,
                            total,
                            elapsed_ms: started.elapsed().as_millis() as u64,
                        },
                    );
                }
            }
        });

        let engine_for_supervisor = engine.clone();
        let request_for_supervisor = request_id.clone();
        let supervisor = tokio::spawn(async move {
            let deadline = tokio::time::sleep(SEARCH_COMPLETION_BOUND);
            tokio::pin!(deadline);
            tokio::select! {
                _ = cancel_for_task.cancelled() => {
                    set.abort_all();
                    while let Some(joined) = set.join_next().await {
                        if let Err(err) = joined {
                            if !err.is_cancelled() {
                                warn!(?err, "search JoinSet task ended with error after abort");
                            }
                        }
                    }
                }
                _ = &mut deadline => {
                    warn!("search completion bound exceeded — aborting module tasks");
                    set.abort_all();
                    while let Some(joined) = set.join_next().await {
                        let _ = joined;
                    }
                }
                _ = async {
                    while let Some(joined) = set.join_next().await {
                        if let Err(err) = joined {
                            if !err.is_cancelled() {
                                warn!(?err, "search JoinSet task ended with error");
                            }
                        }
                    }
                } => {}
            }
            let _ = collector_handle.await;
            // Drop completed search entry so `searches` does not retain finished tasks.
            let mut g = engine_for_supervisor.lock().await;
            g.searches.remove(&request_for_supervisor);
        });

        {
            let mut g = self.inner.lock().await;
            g.pending_searches.remove(&request_for_task);
            g.searches.insert(
                request_for_task,
                SearchTask {
                    cancel,
                    module_cancels,
                    handle: supervisor,
                },
            );
        }
    }
}
