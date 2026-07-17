use super::*;

impl Engine {
    pub async fn start_session(&self) {
        {
            let mut g = self.inner.lock().await;
            if g.session_cancel.is_cancelled() {
                g.session_cancel = CancellationToken::new();
            }
        }
        if let Some(repo) = &self.settings {
            if let Ok(settings) = repo.load_or_default() {
                let modules = {
                    let g = self.inner.lock().await;
                    g.registry.all_modules()
                };
                for module in modules {
                    module.apply_settings(&settings).await;
                }
            }
        }
        let (enabled, disabled_ids) = {
            let g = self.inner.lock().await;
            let enabled = g.registry.enabled_modules();
            let disabled_ids: Vec<String> = g
                .registry
                .list()
                .into_iter()
                .filter(|(_, enabled, _)| !*enabled)
                .map(|(id, _, _)| id)
                .collect();
            (enabled, disabled_ids)
        };
        for id in disabled_ids {
            {
                let mut g = self.inner.lock().await;
                g.module_states.insert(id.clone(), "disabled".into());
            }
            let _ = self
                .emit(Event::ModuleStateChanged {
                    module_id: id,
                    state: "disabled".into(),
                })
                .await;
        }

        let mut warmup_handles = Vec::new();
        for module in enabled {
            let module_id = module.manifest().id.as_str().to_string();
            let cancel = {
                let g = self.inner.lock().await;
                g.session_cancel.child_token()
            };
            let handle = tokio::spawn(async move {
                module
                    .warmup(WarmupContext {
                        cancel: cancel.clone(),
                    })
                    .await
            });
            warmup_handles.push((module_id, handle));
        }
        for (id, handle) in warmup_handles {
            match handle.await {
                Ok(state) => {
                    let state_str = match &state {
                        ModuleState::Ready => "ready".into(),
                        ModuleState::Cold => "cold".into(),
                        ModuleState::Disabled => "disabled".into(),
                        ModuleState::Failed(msg) => format!("failed:{msg}"),
                    };
                    {
                        let mut g = self.inner.lock().await;
                        g.module_states.insert(id.clone(), state_str.clone());
                    }
                    let _ = self
                        .emit(Event::ModuleStateChanged {
                            module_id: id,
                            state: state_str,
                        })
                        .await;
                }
                Err(err) => {
                    warn!(module_id = %id, ?err, "module warmup task panicked");
                    let state_str: String = "failed:panic".into();
                    {
                        let mut g = self.inner.lock().await;
                        g.module_states.insert(id.clone(), state_str.clone());
                    }
                    let _ = self
                        .emit(Event::ModuleStateChanged {
                            module_id: id,
                            state: state_str,
                        })
                        .await;
                }
            }
        }
        let modules = {
            let g = self.inner.lock().await;
            g.registry.list_module_info().into_iter().collect()
        };
        let _ = self.emit(Event::SessionReady { modules }).await;
    }

    /// Cancel in-flight search/action work for a module, await operation termination, then teardown or warmup.
    pub(super) async fn apply_module_enabled(&self, module_id: &str, enabled: bool) -> bool {
        let (module, op_handles, removed_ids) = {
            let mut g = self.inner.lock().await;
            if !g.registry.set_enabled(module_id, enabled) {
                return false;
            }
            let mut op_handles = Vec::new();
            let mut removed_ids = Vec::new();
            if !enabled {
                for task in g.searches.values() {
                    if let Some(token) = task.module_cancels.get(module_id) {
                        token.cancel();
                    }
                }
                for op in g.operations.values_mut() {
                    if op.module_id == module_id {
                        op.cancel.cancel();
                        if let Some(handle) = op.handle.take() {
                            op_handles.push(handle);
                        }
                    }
                }
                removed_ids = g
                    .results_by_id
                    .iter()
                    .filter(|(_, item)| item.module_id.as_str() == module_id)
                    .map(|(id, _)| id.clone())
                    .collect();
                for id in &removed_ids {
                    g.remove_result(id);
                }
                g.module_states
                    .insert(module_id.to_string(), "disabled".into());
            }
            (g.registry.get(module_id), op_handles, removed_ids)
        };
        for handle in op_handles {
            let _ = Self::await_operation_handle(handle).await;
        }
        if !removed_ids.is_empty() {
            let _ = self
                .emit(Event::ResultsChunk {
                    // Empty request_id: TUI treats this as a module eviction (any active search).
                    request_id: String::new(),
                    sequence: 0,
                    upserts: vec![],
                    removed_ids,
                })
                .await;
        }
        let Some(module) = module else {
            return false;
        };
        if enabled {
            let cancel = {
                let g = self.inner.lock().await;
                g.session_cancel.child_token()
            };
            let state = module
                .warmup(WarmupContext {
                    cancel: cancel.clone(),
                })
                .await;
            let state_str = match &state {
                ModuleState::Ready => "ready".into(),
                ModuleState::Cold => "cold".into(),
                ModuleState::Disabled => "disabled".into(),
                ModuleState::Failed(msg) => format!("failed:{msg}"),
            };
            {
                let mut g = self.inner.lock().await;
                g.module_states
                    .insert(module_id.to_string(), state_str.clone());
            }
            let _ = self
                .emit(Event::ModuleStateChanged {
                    module_id: module_id.to_string(),
                    state: state_str,
                })
                .await;
        } else {
            module.teardown().await;
            let _ = self
                .emit(Event::ModuleStateChanged {
                    module_id: module_id.to_string(),
                    state: "disabled".into(),
                })
                .await;
        }
        true
    }

    pub(super) async fn emit(&self, event: Event) -> Result<(), String> {
        let broadcast_tx = {
            let g = self.inner.lock().await;
            g.event_broadcast_tx.clone()
        };
        // Broadcast never back-pressures the producer; slow subscribers may Lagged.
        let _ = broadcast_tx.send(event);
        Ok(())
    }

    pub(super) fn emit_from_inner(inner: &EngineInner, event: Event) {
        let _ = inner.event_broadcast_tx.send(event);
    }

    pub(super) fn resolve_enabled_module(
        g: &mut EngineInner,
        result_id: &str,
    ) -> (Option<luma_domain::SearchItem>, Option<Arc<dyn LumaModule>>) {
        g.touch_result(result_id);
        let item = g.results_by_id.get(result_id).cloned();
        let module = item.as_ref().and_then(|i| {
            if g.registry.is_enabled(i.module_id.as_str()) {
                g.registry.get(i.module_id.as_str())
            } else {
                None
            }
        });
        (item, module)
    }

    pub(super) async fn handle_shutdown_session(&self) {
        {
            let _lifecycle = self.search_lifecycle.lock().await;
            self.cancel_all_searches_locked().await;
        }
        let (modules, op_handles) = {
            let mut g = self.inner.lock().await;
            g.session_cancel.cancel();
            let mut op_handles = Vec::new();
            for op in g.operations.values_mut() {
                op.cancel.cancel();
                if let Some(handle) = op.handle.take() {
                    op_handles.push(handle);
                }
            }
            g.operations.clear();
            g.operation_order.clear();
            g.cancel_intents.clear();
            g.clear_results();
            g.latest_preview_id = 0;
            (g.registry.all_modules(), op_handles)
        };
        for handle in op_handles {
            let _ = Self::await_operation_handle(handle).await;
        }
        for m in modules {
            m.teardown().await;
        }
    }
}
