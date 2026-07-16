use luma_application::{
    run_action, run_interactive_terminal, run_query, sftp_args, ssh_connect_args, EnginePort,
    ModuleRegistry, SettingsRepository,
};
use luma_protocol::Command;
use std::process::ExitStatus;
use std::sync::Arc;

async fn ssh_meta_action(
    registry: ModuleRegistry,
    settings: Option<Arc<dyn SettingsRepository>>,
    query: &str,
    result_id: &str,
    action_id: &str,
) -> Result<(), String> {
    let (_, outcome) = run_action(
        registry,
        query,
        Some(result_id),
        action_id,
        false,
        luma_application::RunActionOptions {
            settings,
            ..Default::default()
        },
    )
    .await?;
    match outcome {
        luma_protocol::ActionOutcomeDto::Success { .. } => Ok(()),
        other => Err(other.display_message()),
    }
}

pub async fn ssh_set_favorite(
    registry: ModuleRegistry,
    settings: Option<Arc<dyn SettingsRepository>>,
    alias: &str,
    favorite: bool,
) -> Result<(), String> {
    let action = if favorite { "favorite" } else { "unfavorite" };
    ssh_meta_action(registry, settings, "ssh", &format!("ssh:{alias}"), action).await
}

pub async fn ssh_set_display_name(
    registry: ModuleRegistry,
    settings: Option<Arc<dyn SettingsRepository>>,
    alias: &str,
    name: &str,
) -> Result<(), String> {
    ssh_meta_action(
        registry,
        settings,
        &format!("ssh rename {alias} {name}"),
        &format!("ssh:rename:{alias}"),
        "rename",
    )
    .await
}

pub async fn ssh_list_json(
    registry: ModuleRegistry,
    settings: Option<Arc<dyn SettingsRepository>>,
) -> Result<serde_json::Value, String> {
    let (items, _) = run_query(registry, "ssh", settings).await?;
    Ok(serde_json::json!({ "results": items }))
}

pub async fn ssh_connect_cli(
    registry: ModuleRegistry,
    alias: &str,
    program: &str,
    settings: Option<Arc<dyn SettingsRepository>>,
    engine: Option<Arc<dyn EnginePort>>,
) -> Result<ExitStatus, String> {
    let args = if program == "sftp" {
        sftp_args(alias)
    } else {
        ssh_connect_args(alias)
    };
    let status = run_interactive_terminal(program, &args).map_err(|e| e.to_string())?;
    if status.success() {
        if let Some(engine) = engine {
            let _ = engine
                .submit(Command::SshSessionEnded {
                    alias: alias.to_string(),
                    exit_code: status.code().unwrap_or(0),
                })
                .await;
        } else {
            let engine = luma_application::Engine::with_settings(registry, settings);
            engine.start_session().await;
            let _ = engine
                .handle_command(Command::SshSessionEnded {
                    alias: alias.to_string(),
                    exit_code: status.code().unwrap_or(0),
                })
                .await;
            engine.handle_command(Command::ShutdownSession).await;
        }
    }
    Ok(status)
}
