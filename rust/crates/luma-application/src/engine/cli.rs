use super::*;

/// Drain lagged frames until the next event or channel close.
async fn recv_event(rx: &mut broadcast::Receiver<Event>) -> Option<Event> {
    loop {
        match rx.recv().await {
            Ok(ev) => return Some(ev),
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => return None,
        }
    }
}

/// One-shot helper for non-interactive CLI: own engine lifecycle for a single query.
pub async fn run_query(
    registry: ModuleRegistry,
    query: &str,
    settings: Option<Arc<dyn crate::ports::SettingsRepository>>,
) -> Result<(Vec<SearchItemDto>, Vec<Event>), String> {
    let triggers = registry.all_triggers();
    let query = luma_domain::Query::normalize_for_cli(query, |token| {
        is_meta_prefix(token) || triggers.iter().any(|t| t == token)
    });
    let engine = Engine::with_settings(registry, settings);
    let mut rx = engine.subscribe();

    engine.start_session().await;
    let request_id = "cli-1".to_string();
    let search = engine.handle_command(Command::Search {
        request_id: request_id.clone(),
        query,
    });

    let collect = async {
        let mut events = Vec::new();
        let mut items: HashMap<String, SearchItemDto> = HashMap::new();
        while let Some(ev) = recv_event(&mut rx).await {
            match &ev {
                Event::ResultsChunk {
                    upserts,
                    removed_ids,
                    ..
                } => {
                    for u in upserts {
                        items.insert(u.id.clone(), u.clone());
                    }
                    for id in removed_ids {
                        items.remove(id);
                    }
                }
                Event::SearchFinished { .. }
                | Event::SearchCancelled { .. }
                | Event::Fatal { .. } => {
                    events.push(ev);
                    break;
                }
                _ => events.push(ev),
            }
        }
        let mut out: Vec<_> = items.into_values().collect();
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        (out, events)
    };

    let ((), (items, events)) = tokio::join!(search, collect);
    let _ = engine.handle_command(Command::ShutdownSession).await;
    Ok((items, events))
}

/// One-shot CLI action helper. Searches and executes within one engine session so result ids remain valid.
/// Optional inputs for [`run_action`] beyond the core query/action selectors.
#[derive(Default)]
pub struct RunActionOptions {
    pub settings: Option<Arc<dyn crate::ports::SettingsRepository>>,
    pub command_runner: Option<Arc<dyn crate::ports::CommandRunnerPort>>,
    pub recipe_stdio: crate::ports::RecipeStdioMode,
}

pub async fn run_action(
    registry: ModuleRegistry,
    query: &str,
    result_id: Option<&str>,
    action_id: &str,
    confirmation: bool,
    options: RunActionOptions,
) -> Result<(SearchItemDto, luma_protocol::ActionOutcomeDto), String> {
    let RunActionOptions {
        settings,
        command_runner,
        recipe_stdio,
    } = options;
    let triggers = registry.all_triggers();
    let query = luma_domain::Query::normalize_for_cli(query, |token| {
        is_meta_prefix(token) || triggers.iter().any(|t| t == token)
    });
    let engine = Engine::with_settings(registry, settings);
    let mut rx = engine.subscribe();
    engine.start_session().await;
    let request_id = "cli-action-search".to_string();
    let search = engine.handle_command(Command::Search { request_id, query });
    let collect = async {
        let mut items = HashMap::new();
        while let Some(event) = recv_event(&mut rx).await {
            match event {
                Event::ResultsChunk {
                    upserts,
                    removed_ids,
                    ..
                } => {
                    for item in upserts {
                        items.insert(item.id.clone(), item);
                    }
                    for id in removed_ids {
                        items.remove(&id);
                    }
                }
                Event::SearchFinished { .. } | Event::SearchCancelled { .. } => break,
                _ => {}
            }
        }
        items
    };
    let ((), items) = tokio::join!(search, collect);
    let selected = match result_id {
        Some(id) => items
            .get(id)
            .cloned()
            .ok_or_else(|| format!("result not found: {id}"))?,
        None => {
            let mut values: Vec<_> = items.into_values().collect();
            values.sort_by(|a, b| b.score.total_cmp(&a.score));
            values
                .iter()
                .find(|item| {
                    !matches!(
                        item.kind.as_str(),
                        "warming"
                            | "unavailable"
                            | "not_configured"
                            | "onboarding"
                            | "status"
                            | "permission_required"
                    ) && item.primary_action_id.as_str() != "noop"
                })
                .cloned()
                .ok_or_else(|| "query returned no actionable results".to_string())?
        }
    };
    static CLI_ACTION_OP: AtomicU64 = AtomicU64::new(0);
    let operation_id = format!(
        "cli-action-{}",
        CLI_ACTION_OP.fetch_add(1, Ordering::Relaxed) + 1
    );
    engine
        .handle_command(Command::ExecuteAction {
            operation_id,
            result_id: selected.id.clone(),
            action_id: action_id.into(),
            confirmation,
        })
        .await;
    let outcome = loop {
        match recv_event(&mut rx).await {
            Some(Event::ActionFinished { outcome, .. }) => break outcome,
            Some(_) => {}
            None => return Err("engine event channel closed".into()),
        }
    };
    let outcome = match outcome {
        luma_protocol::ActionOutcomeDto::InteractiveTerminal {
            program,
            args,
            record_alias,
        } => {
            use crate::interactive_terminal::run_interactive_terminal;
            match run_interactive_terminal(&program, &args) {
                Ok(status) => {
                    if status.success() {
                        if let Some(alias) = record_alias {
                            engine
                                .handle_command(Command::SshSessionEnded {
                                    alias,
                                    exit_code: status.code().unwrap_or(0),
                                })
                                .await;
                        }
                        luma_protocol::ActionOutcomeDto::Success {
                            message: Some(format!("{program} exited")),
                        }
                    } else {
                        luma_protocol::ActionOutcomeDto::Failed {
                            kind: luma_domain::FailureKind::Unavailable {
                                reason: format!(
                                    "{program} exited with code {}",
                                    status.code().unwrap_or(1)
                                ),
                                retryable: false,
                            },
                            message: None,
                        }
                    }
                }
                Err(err) => {
                    luma_protocol::ActionOutcomeDto::failed(luma_domain::FailureKind::Unavailable {
                        reason: err.to_string(),
                        retryable: false,
                    })
                }
            }
        }
        luma_protocol::ActionOutcomeDto::InteractiveRecipeRun { plan } => {
            let Some(runner) = command_runner.as_ref() else {
                return Ok((
                    selected,
                    luma_protocol::ActionOutcomeDto::failed(
                        luma_domain::FailureKind::Unavailable {
                            reason: format!(
                                "recipe `{}` needs interactive execution; use `luma cmd run {}`",
                                plan.recipe_id, plan.recipe_id
                            ),
                            retryable: false,
                        },
                    ),
                ));
            };
            let cancel = CancellationToken::new();
            let cancel_task = crate::recipe_runner::spawn_ctrl_c_cancel(cancel.clone());
            let report_result = crate::recipe_runner::execute_recipe_plan(
                &plan,
                runner.as_ref(),
                &cancel,
                crate::recipe_runner::RecipeExecuteOptions {
                    confirmation,
                    stdio: recipe_stdio,
                },
            );
            // JoinHandle::Drop detaches a Tokio task. Abort the one-shot signal
            // listener explicitly, including the confirmation-error path below.
            cancel_task.abort();
            let report = match report_result {
                Ok(report) => report,
                Err(err) => {
                    return Ok((
                        selected,
                        luma_protocol::ActionOutcomeDto::failed(
                            luma_domain::FailureKind::InvalidInput {
                                field: "confirmation".into(),
                                message: err.to_string(),
                            },
                        ),
                    ));
                }
            };
            engine
                .handle_command(Command::RecordRecipeRun {
                    recipe_id: plan.recipe_id.clone(),
                    result: report.outcome.clone(),
                    now_unix: crate::recipe_runner::now_unix(),
                })
                .await;
            crate::recipe_runner::recipe_outcome_to_action_dto(&plan.recipe_id, &report)
        }
        other => other,
    };
    engine.handle_command(Command::ShutdownSession).await;
    Ok((selected, outcome))
}

pub async fn list_modules_json(registry: &ModuleRegistry) -> serde_json::Value {
    let rows = registry.list();
    serde_json::json!({
        "modules": rows.iter().map(|(id, enabled, name)| {
            serde_json::json!({ "id": id, "enabled": enabled, "display_name": name })
        }).collect::<Vec<_>>()
    })
}
