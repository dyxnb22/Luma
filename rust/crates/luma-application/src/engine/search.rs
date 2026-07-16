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
            let query = Query::parse_with_prefixes(&query_raw, 50, |token| {
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
            Query::parse_with_prefixes(query_raw, 50, |token| {
                is_meta_prefix(token) || triggers.iter().any(|t| t == token)
            })
        };
        let modules: Vec<Arc<dyn LumaModule>> = {
            let g = self.inner.lock().await;
            match &query.scope {
                QueryScope::Targeted { module } => {
                    g.registry.resolve_trigger(module).into_iter().collect()
                }
                QueryScope::Global => g.registry.contributing(),
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
            let q = query.clone();
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
            async move {
                let mut sequence = 0u64;
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
                        let upserts: Vec<_> = upserts
                            .into_iter()
                            .filter(|u| g.registry.is_enabled(&u.module_id))
                            .collect();
                        let mut evicted = Vec::new();
                        for u in &upserts {
                            let id = u.id.clone();
                            evicted.extend(g.insert_result(id, u.clone().into_domain()));
                        }
                        for id in &removed_ids {
                            g.remove_result(id);
                        }
                        let mut all_removed = removed_ids;
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
                    let g = engine.lock().await;
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
