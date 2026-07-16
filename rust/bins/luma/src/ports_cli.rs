//! CLI helpers for Ports (`luma ports …`).

use luma_application::{run_action, run_query, ModuleRegistry, SettingsRepository};
use std::sync::Arc;

pub async fn ports_list_json(
    registry: ModuleRegistry,
    settings: Option<Arc<dyn SettingsRepository>>,
    needle: Option<&str>,
) -> Result<serde_json::Value, String> {
    let query = match needle {
        Some(n) if !n.trim().is_empty() => format!("port {}", n.trim()),
        _ => "port ".into(),
    };
    let (items, _) = run_query(registry, &query, settings).await?;
    Ok(serde_json::json!({ "results": items }))
}

pub async fn ports_name(
    registry: ModuleRegistry,
    settings: Option<Arc<dyn SettingsRepository>>,
    port: u16,
    name: &str,
) -> Result<(), String> {
    let query = format!("port name {port} {name}");
    let result_id = format!("port:name:{port}");
    let (_, outcome) = run_action(
        registry,
        &query,
        Some(&result_id),
        "set_name",
        false,
        settings,
    )
    .await?;
    match outcome {
        luma_protocol::ActionOutcomeDto::Success { .. } => Ok(()),
        other => Err(other.display_message()),
    }
}

pub async fn ports_set_favorite(
    registry: ModuleRegistry,
    settings: Option<Arc<dyn SettingsRepository>>,
    port: u16,
    favorite: bool,
) -> Result<(), String> {
    let query = format!("port {port}");
    let (items, _) = run_query(registry.clone(), &query, settings.clone()).await?;
    let result_id = items
        .iter()
        .find(|i| i.kind == "port")
        .map(|i| i.id.clone())
        .ok_or_else(|| format!("no listening process on :{port}"))?;
    let action = if favorite { "favorite" } else { "unfavorite" };
    let (_, outcome) =
        run_action(registry, &query, Some(&result_id), action, false, settings).await?;
    match outcome {
        luma_protocol::ActionOutcomeDto::Success { .. } => Ok(()),
        other => Err(other.display_message()),
    }
}

pub async fn ports_kill(
    registry: ModuleRegistry,
    settings: Option<Arc<dyn SettingsRepository>>,
    port_or_query: &str,
    force: bool,
    yes: bool,
) -> Result<String, String> {
    if !yes {
        return Err("refusing to kill without --yes".into());
    }
    let query = format!("port {port_or_query}");
    let (items, _) = run_query(registry.clone(), &query, settings.clone()).await?;
    let result_id = items
        .iter()
        .find(|i| i.kind == "port")
        .map(|i| i.id.clone())
        .ok_or_else(|| format!("no matching listening port for '{port_or_query}'"))?;
    let action = if force { "force_kill" } else { "kill" };
    let (_, outcome) =
        run_action(registry, &query, Some(&result_id), action, true, settings).await?;
    match outcome {
        luma_protocol::ActionOutcomeDto::Success { message } => {
            Ok(message.unwrap_or_else(|| "killed".into()))
        }
        other => Err(other.display_message()),
    }
}
