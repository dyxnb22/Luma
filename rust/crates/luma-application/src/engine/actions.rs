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
        let cancel = {
            let mut g = self.inner.lock().await;
            let cancel = g.session_cancel.child_token();
            let module_id = item
                .as_ref()
                .map(|i| i.module_id.as_str().to_string())
                .unwrap_or_default();
            g.operations.insert(
                operation_id.clone(),
                OperationTask {
                    cancel: cancel.clone(),
                    module_id,
                    handle: None,
                },
            );
            cancel
        };
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
                        let needs_confirm = action.confirmation
                            || !matches!(action.risk, luma_domain::ActionRisk::Safe);
                        if needs_confirm && !confirmation {
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
                Self::emit_from_inner(
                    &g,
                    Event::ActionFinished {
                        operation_id: op_id.clone(),
                        outcome,
                    },
                );
                g.operations.remove(&op_id);
            }
        });
        if let Some(op) = self.inner.lock().await.operations.get_mut(&operation_id) {
            op.handle = Some(handle);
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
        let _ = self
            .emit(Event::PreviewLoaded {
                result_id,
                preview_id,
                body,
            })
            .await;
    }

    pub(super) async fn handle_cancel_operation(&self, operation_id: String) {
        let handle = {
            let mut g = self.inner.lock().await;
            match g.operations.get_mut(&operation_id) {
                Some(op) => {
                    op.cancel.cancel();
                    op.handle.take()
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
                {
                    let mut g = self.inner.lock().await;
                    g.operations.remove(&operation_id);
                }
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
