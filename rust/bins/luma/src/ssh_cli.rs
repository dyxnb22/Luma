use luma_application::{
    run_action, run_interactive_terminal, run_query, EnginePort, ModuleRegistry,
    SettingsRepository, SshConfigPort,
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
    ssh_meta_action(registry, settings, "/ssh", &format!("ssh:{alias}"), action).await
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
        &format!("/ssh rename {alias} {name}"),
        &format!("ssh:rename:{alias}"),
        "rename",
    )
    .await
}

pub async fn ssh_list_json(
    registry: ModuleRegistry,
    settings: Option<Arc<dyn SettingsRepository>>,
) -> Result<serde_json::Value, String> {
    let (items, _) = run_query(registry, "/ssh", settings).await?;
    Ok(serde_json::json!({ "results": items }))
}

pub async fn ssh_connect_cli(
    registry: ModuleRegistry,
    alias: &str,
    program: &str,
    settings: Option<Arc<dyn SettingsRepository>>,
    engine: Option<Arc<dyn EnginePort>>,
    ssh_config: Arc<dyn SshConfigPort>,
) -> Result<ExitStatus, String> {
    // Same resolve gate as TUI (unknown / `-`-prefixed aliases refused before spawn).
    let _ = ssh_config.resolve(alias).map_err(|e| e.to_string())?;
    if program == "sftp" && !ssh_config.sftp_available() {
        return Err("sftp command unavailable".into());
    }
    if program != "sftp" && !ssh_config.ssh_available() {
        return Err("ssh command unavailable".into());
    }
    let (executable, args) = connection_command(ssh_config.as_ref(), alias, program)?;
    let status = run_interactive_terminal(executable, &args, &[]).map_err(|e| e.to_string())?;
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

fn connection_command(
    ssh_config: &dyn SshConfigPort,
    alias: &str,
    program: &str,
) -> Result<(&'static str, Vec<String>), String> {
    match program {
        "sftp" => Ok(("/usr/bin/sftp", ssh_config.sftp_invocation_args(alias))),
        "ssh" => Ok(("/usr/bin/ssh", ssh_config.ssh_invocation_args(alias))),
        _ => Err(format!("unsupported SSH executable: {program}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{
        FakeSshConfigPort, ModuleRegistry, ResolvedSshHost, SshConfigError, SshConfigState,
    };

    struct ExplicitConfig;

    impl SshConfigPort for ExplicitConfig {
        fn config_state(&self) -> SshConfigState {
            SshConfigState::Found
        }

        fn list_aliases(&self) -> Result<Vec<String>, SshConfigError> {
            Ok(vec!["loopback".into()])
        }

        fn resolve(&self, alias: &str) -> Result<ResolvedSshHost, SshConfigError> {
            Ok(ResolvedSshHost {
                alias: alias.into(),
                hostname: Some("127.0.0.1".into()),
                user: None,
                port: None,
                identity_file: None,
                proxy_jump: None,
                connect_timeout: None,
            })
        }

        fn ssh_available(&self) -> bool {
            true
        }

        fn sftp_available(&self) -> bool {
            true
        }

        fn ssh_invocation_args(&self, alias: &str) -> Vec<String> {
            vec![
                "-F".into(),
                "/fixture/config".into(),
                "--".into(),
                alias.into(),
            ]
        }

        fn sftp_invocation_args(&self, alias: &str) -> Vec<String> {
            vec![
                "-F".into(),
                "/fixture/config".into(),
                "--".into(),
                alias.into(),
            ]
        }
    }

    #[tokio::test]
    async fn ssh_connect_cli_rejects_dash_alias() {
        let config = Arc::new(FakeSshConfigPort::new());
        let err = ssh_connect_cli(ModuleRegistry::new(), "-ofoo", "ssh", None, None, config)
            .await
            .unwrap_err();
        assert!(err.contains("flag") || err.contains("refusing"), "{err}");
    }

    #[tokio::test]
    async fn ssh_connect_cli_rejects_unknown_alias() {
        let config = Arc::new(FakeSshConfigPort::new());
        let err = ssh_connect_cli(
            ModuleRegistry::new(),
            "no-such-host",
            "ssh",
            None,
            None,
            config,
        )
        .await
        .unwrap_err();
        assert!(err.contains("unknown"), "{err}");
    }

    #[test]
    fn cli_connection_reuses_the_config_ports_exact_invocation() {
        let config = ExplicitConfig;
        for program in ["ssh", "sftp"] {
            let (_, args) = connection_command(&config, "loopback", program).unwrap();
            assert_eq!(
                args,
                ["-F", "/fixture/config", "--", "loopback"],
                "{program} must use the same config that was resolved"
            );
        }
    }
}
