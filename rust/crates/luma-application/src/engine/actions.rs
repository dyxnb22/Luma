use super::*;
use luma_protocol::Event;

impl Engine {
    pub(super) async fn handle_execute_action(
        &self,
        operation_id: String,
        result_id: String,
        action_id: String,
        confirmation: bool,
    ) {
        let (item, module) = {
            let mut g = self.inner.lock().await;
            Self::resolve_enabled_module(&mut g, &result_id)
        };

        // Cancel any prior operation with the same id, and drop oldest ops if at capacity.
        let superseded = {
            let mut g = self.inner.lock().await;
            let mut handles = Vec::new();
            if let Some(mut old) = g.operations.remove(&operation_id) {
                g.operation_order.retain(|id| id != &operation_id);
                old.cancel.cancel();
                if let Some(handle) = old.handle.take() {
                    handles.push(handle);
                }
            }
            while g.operations.len() >= MAX_OPERATIONS {
                let Some(id) = g.operation_order.pop_front() else {
                    break;
                };
                if let Some(mut old) = g.operations.remove(&id) {
                    old.cancel.cancel();
                    if let Some(handle) = old.handle.take() {
                        handles.push(handle);
                    }
                }
            }
            let cancel = g.session_cancel.child_token();
            g.next_operation_generation = g.next_operation_generation.wrapping_add(1);
            let generation = g.next_operation_generation;
            let module_id = item
                .as_ref()
                .map(|i| i.module_id.as_str().to_string())
                .unwrap_or_default();
            g.operations.insert(
                operation_id.clone(),
                OperationTask {
                    cancel: cancel.clone(),
                    module_id,
                    generation,
                    handle: None,
                },
            );
            g.operation_order.push_back(operation_id.clone());
            (cancel, generation, handles)
        };
        let (cancel, generation, superseded_handles) = superseded;
        for handle in superseded_handles {
            let _ = Self::await_operation_handle(handle).await;
        }

        let settings_repo = self.settings.clone();
        let inner = self.inner.clone();
        let engine = self.clone_inner();
        let op_id = operation_id.clone();
        let handle = tokio::spawn(async move {
            {
                let g = engine.lock().await;
                Self::emit_from_inner(
                    &g,
                    Event::ActionStarted {
                        operation_id: op_id.clone(),
                    },
                );
            }

            let outcome = match (item, module) {
                (Some(result), Some(module)) => {
                    let actions = module.actions(&result).await;
                    if let Some(action) = actions.into_iter().find(|a| a.id.as_str() == action_id) {
                        if action.needs_confirmation() && !confirmation {
                            luma_protocol::ActionOutcomeDto::failed(
                                luma_domain::FailureKind::SecurityDenied {
                                    reason: "confirmation required".into(),
                                },
                            )
                        } else {
                            let out = module
                                .perform(
                                    crate::module::ActionRequest {
                                        result,
                                        action,
                                        confirmation,
                                    },
                                    cancel,
                                )
                                .await;
                            match out {
                                crate::module::ActionOutcome::Success { message } => {
                                    luma_protocol::ActionOutcomeDto::Success { message }
                                }
                                crate::module::ActionOutcome::Cancelled => {
                                    luma_protocol::ActionOutcomeDto::Cancelled
                                }
                                crate::module::ActionOutcome::InteractiveTerminal {
                                    program,
                                    args,
                                    record_alias,
                                } => luma_protocol::ActionOutcomeDto::InteractiveTerminal {
                                    program,
                                    args,
                                    record_alias,
                                },
                                crate::module::ActionOutcome::SettingsMutation { patch } => {
                                    match apply_settings_mutation(
                                        settings_repo.as_ref(),
                                        &inner,
                                        patch,
                                    )
                                    .await
                                    {
                                        Ok(msg) => luma_protocol::ActionOutcomeDto::Success {
                                            message: Some(msg),
                                        },
                                        Err(kind) => luma_protocol::ActionOutcomeDto::failed(kind),
                                    }
                                }
                                crate::module::ActionOutcome::Failed { kind } => {
                                    luma_protocol::ActionOutcomeDto::failed(kind)
                                }
                                crate::module::ActionOutcome::InteractiveRecipeRun { plan } => {
                                    luma_protocol::ActionOutcomeDto::InteractiveRecipeRun { plan }
                                }
                            }
                        }
                    } else {
                        luma_protocol::ActionOutcomeDto::failed(
                            luma_domain::FailureKind::NotFound {
                                entity: format!("action:{action_id}"),
                            },
                        )
                    }
                }
                _ => luma_protocol::ActionOutcomeDto::failed(luma_domain::FailureKind::NotFound {
                    entity: format!("result:{result_id}"),
                }),
            };
            {
                let mut g = engine.lock().await;
                let is_current = g
                    .operations
                    .get(&op_id)
                    .is_some_and(|operation| operation.generation == generation);
                if is_current {
                    Self::emit_from_inner(
                        &g,
                        Event::ActionFinished {
                            operation_id: op_id.clone(),
                            outcome,
                        },
                    );
                    g.operations.remove(&op_id);
                    g.operation_order.retain(|id| id != &op_id);
                }
            }
        });
        let mut handle = Some(handle);
        let should_abort = {
            let mut g = self.inner.lock().await;
            if let Some(op) = g.operations.get_mut(&operation_id) {
                if op.generation == generation {
                    op.handle = handle.take();
                    false
                } else {
                    true
                }
            } else {
                true
            }
        };
        if should_abort {
            if let Some(handle) = handle {
                handle.abort();
            }
        }
    }

    pub(super) async fn handle_list_actions(&self, result_id: String) {
        let (item, module) = {
            let mut g = self.inner.lock().await;
            Self::resolve_enabled_module(&mut g, &result_id)
        };
        let actions = match (item, module) {
            (Some(result), Some(module)) => {
                let descriptors = module.actions(&result).await;
                descriptors
                    .into_iter()
                    .map(|a| luma_protocol::ActionDescriptorDto::from(&a))
                    .collect::<Vec<_>>()
            }
            _ => Vec::new(),
        };
        let _ = self
            .emit(Event::ActionsAvailable { result_id, actions })
            .await;
    }

    pub(super) async fn handle_load_preview(&self, result_id: String, preview_id: u64) {
        let (item, module) = {
            let mut g = self.inner.lock().await;
            g.latest_preview_id = preview_id;
            Self::resolve_enabled_module(&mut g, &result_id)
        };
        let body = match (item, module) {
            (Some(result), Some(module)) => {
                let raw = module
                    .preview(&result)
                    .await
                    .unwrap_or_else(|| result.title.clone());
                super::preview::truncate_preview_body(&raw)
            }
            _ => String::new(),
        };
        {
            let g = self.inner.lock().await;
            if g.latest_preview_id != preview_id {
                // Newer LoadPreview superseded this work — do not emit stale body.
                return;
            }
        }
        let _ = self
            .emit(Event::PreviewLoaded {
                result_id,
                preview_id,
                body,
            })
            .await;
    }

    pub(super) async fn handle_cancel_operation(&self, operation_id: String) {
        let (handle, generation) = {
            let mut g = self.inner.lock().await;
            match g.operations.get_mut(&operation_id) {
                Some(op) => {
                    op.cancel.cancel();
                    (op.handle.take(), op.generation)
                }
                None => {
                    drop(g);
                    let _ = self
                        .emit(Event::ActionFinished {
                            operation_id,
                            outcome: luma_protocol::ActionOutcomeDto::failed(
                                luma_domain::FailureKind::NotFound {
                                    entity: "operation".into(),
                                },
                            ),
                        })
                        .await;
                    return;
                }
            }
        };
        // Bounded await so Esc/cancel cannot hang on a non-cooperative perform.
        if let Some(handle) = handle {
            let finished_cleanly = Self::await_operation_handle(handle).await;
            if !finished_cleanly {
                // Abort skipped the task's ActionFinished emit — synthesize Cancelled.
                let should_emit = {
                    let mut g = self.inner.lock().await;
                    let is_current = g
                        .operations
                        .get(&operation_id)
                        .is_some_and(|operation| operation.generation == generation);
                    if is_current {
                        g.operations.remove(&operation_id);
                        g.operation_order.retain(|id| id != &operation_id);
                    }
                    is_current
                };
                if should_emit {
                    let _ = self
                        .emit(Event::ActionFinished {
                            operation_id,
                            outcome: luma_protocol::ActionOutcomeDto::Cancelled,
                        })
                        .await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{EngineInner, ModuleRegistry, OperationTask, MAX_OPERATIONS};
    use std::collections::{HashMap, VecDeque};
    use tokio_util::sync::CancellationToken;

    #[test]
    fn operation_order_evicts_oldest_first() {
        let (tx, _) = tokio::sync::broadcast::channel(8);
        let mut inner = EngineInner {
            registry: ModuleRegistry::new(),
            event_broadcast_tx: tx,
            session_cancel: CancellationToken::new(),
            searches: HashMap::new(),
            cancel_intents: HashMap::new(),
            pending_searches: HashMap::new(),
            operations: HashMap::new(),
            operation_order: VecDeque::new(),
            next_operation_generation: 0,
            latest_preview_id: 0,
            results_by_id: HashMap::new(),
            result_order: VecDeque::new(),
            module_states: HashMap::new(),
        };
        for i in 0..MAX_OPERATIONS {
            let id = format!("op-{i}");
            inner.operations.insert(
                id.clone(),
                OperationTask {
                    cancel: CancellationToken::new(),
                    module_id: "m".into(),
                    generation: i as u64 + 1,
                    handle: None,
                },
            );
            inner.operation_order.push_back(id);
        }
        assert_eq!(inner.operations.len(), MAX_OPERATIONS);
        while inner.operations.len() >= MAX_OPERATIONS {
            let id = inner.operation_order.pop_front().expect("order");
            inner.operations.remove(&id);
        }
        assert!(!inner.operations.contains_key("op-0"));
        assert_eq!(inner.operations.len(), MAX_OPERATIONS - 1);
        inner.operations.insert(
            "op-new".into(),
            OperationTask {
                cancel: CancellationToken::new(),
                module_id: "m".into(),
                generation: MAX_OPERATIONS as u64 + 1,
                handle: None,
            },
        );
        inner.operation_order.push_back("op-new".into());
        assert!(inner.operations.contains_key("op-new"));
        assert!(inner
            .operations
            .contains_key(&format!("op-{}", MAX_OPERATIONS - 1)));
    }
}
