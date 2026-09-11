use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, PackageError,
    PackageKind, PackageManagerPort, PackageMutation, PackageQuery, PackageRecord, PasteboardPort,
    SearchMode, SearchSink, WarmupContext, MAX_PACKAGE_RESULTS,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, SearchItem,
};
use luma_protocol::{ActionDescriptorDto, Event, SearchItemDto};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.packages";

pub struct PackagesModule {
    manifest: ModuleManifest,
    packages: Arc<dyn PackageManagerPort>,
    pasteboard: Arc<dyn PasteboardPort>,
}

impl PackagesModule {
    pub fn with_deps(
        packages: Arc<dyn PackageManagerPort>,
        pasteboard: Arc<dyn PasteboardPort>,
    ) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Packages".into(),
                triggers: vec!["pkg".into(), "packages".into(), "brew".into()],
                default_enabled: true,
                search_mode: SearchMode::TargetedOnly,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("P".into()),
                    suggested_query: Some("/pkg ".into()),
                    empty_hint: Some("/pkg installed · outdated · search <name>".into()),
                    supports_browse: false,
                    commands: vec![
                        crate::ux::command_spec(
                            "/pkg installed",
                            "List installed Homebrew formulae and casks",
                            "/pkg installed",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/pkg outdated",
                            "List outdated Homebrew packages",
                            "/pkg outdated",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/pkg formulae",
                            "List available Homebrew formulae",
                            "/pkg formulae",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/pkg casks",
                            "List available Homebrew casks",
                            "/pkg casks",
                            None,
                        ),
                        crate::ux::command_spec(
                            "/pkg search <name>",
                            "Search Homebrew packages",
                            "/pkg search ",
                            Some("/pkg search ripgrep"),
                        ),
                        crate::ux::command_spec(
                            "/pkg info <exact-name>",
                            "Show package metadata and mutation actions",
                            "/pkg info ",
                            Some("/pkg info ripgrep"),
                        ),
                    ],
                },
            },
            packages,
            pasteboard,
        }
    }
}

#[async_trait]
impl LumaModule for PackagesModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        // Homebrew is queried only on a targeted visit.
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let package_query = match parse_query(query.rest_raw()) {
            Ok(query) => query,
            Err(message) => {
                send_status(
                    &sink,
                    "pkg:invalid",
                    "Packages command is invalid",
                    message,
                    "command_error",
                )
                .await;
                return;
            }
        };
        let is_info = matches!(package_query, PackageQuery::Info(_));
        let records = match self
            .packages
            .query(
                package_query,
                query.limit.min(MAX_PACKAGE_RESULTS),
                cancel.clone(),
            )
            .await
        {
            Ok(records) => records,
            Err(PackageError::Cancelled) => return,
            Err(error) => {
                send_package_error(&sink, error).await;
                return;
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        if records.is_empty() {
            send_status(
                &sink,
                "pkg:empty",
                "No Homebrew packages found",
                "The selected Homebrew query returned no packages",
                "status",
            )
            .await;
            return;
        }
        let upserts = records
            .into_iter()
            .map(|record| package_item(record, is_info))
            .collect();
        let _ = sink
            .send(Event::ResultsChunk {
                request_id: String::new(),
                sequence: 1,
                upserts,
                removed_ids: vec![],
            })
            .await;
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if !matches!(result.kind.as_str(), "package" | "package_info") {
            return vec![safe_action("noop", "OK")];
        }
        let Some(payload) = result.action_payload.as_ref() else {
            return vec![safe_action("noop", "Unavailable")];
        };
        let installed = payload
            .get("installed")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let outdated = payload
            .get("outdated")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let mut actions = if result.kind == "package" {
            vec![safe_action("show_info", "Info")]
        } else {
            vec![safe_action("copy_name", "Copy name")]
        };
        if payload
            .get("homepage")
            .and_then(|value| value.as_str())
            .is_some()
        {
            actions.push(safe_action("copy_homepage", "Copy homepage"));
        }
        if !installed {
            actions.push(confirm_action("install", "Install"));
        }
        if outdated {
            actions.push(confirm_action("upgrade", "Upgrade"));
        }
        if installed {
            actions.push(confirm_action("uninstall", "Uninstall"));
        }
        actions
    }

    async fn perform(&self, request: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        if request.action.id.as_str() == "noop" {
            return ActionOutcome::Success { message: None };
        }
        let Some(payload) = request.result.action_payload.as_ref() else {
            return invalid_action("missing package payload");
        };
        let Some(name) = payload.get("name").and_then(|value| value.as_str()) else {
            return invalid_action("missing package name");
        };
        let Some(kind) = payload
            .get("kind")
            .and_then(|value| value.as_str())
            .and_then(parse_kind)
        else {
            return invalid_action("missing package kind");
        };
        match request.action.id.as_str() {
            "show_info" => ActionOutcome::OpenSurface {
                query: format!("/pkg info {name}"),
            },
            "copy_name" => self.copy(name, &cancel).await,
            "copy_homepage" => {
                let Some(homepage) = payload.get("homepage").and_then(|value| value.as_str())
                else {
                    return invalid_action("package has no homepage");
                };
                self.copy(homepage, &cancel).await
            }
            action @ ("install" | "upgrade" | "uninstall") => {
                if !request.confirmation {
                    return ActionOutcome::Failed {
                        kind: FailureKind::SecurityDenied {
                            reason: format!(
                                "confirmation required before Homebrew {action} of {} {name}",
                                kind.label()
                            ),
                        },
                    };
                }
                let mutation = match action {
                    "install" => PackageMutation::Install,
                    "upgrade" => PackageMutation::Upgrade,
                    "uninstall" => PackageMutation::Uninstall,
                    _ => unreachable!(),
                };
                // Adapter resolves exact name+kind and validates live state before returning argv.
                let plan = match self
                    .packages
                    .mutation_plan(mutation, name, kind, cancel.clone())
                    .await
                {
                    Ok(plan) => plan,
                    Err(error) => return package_outcome(error),
                };
                if cancel.is_cancelled() {
                    return ActionOutcome::Cancelled;
                }
                if plan.package.name != name || plan.package.kind != kind {
                    return ActionOutcome::Failed {
                        kind: FailureKind::Conflict {
                            reason: "Homebrew package identity changed; search again".into(),
                        },
                    };
                }
                ActionOutcome::InteractiveTerminal {
                    program: plan.program,
                    args: plan.args,
                    environment: vec![
                        ("HOMEBREW_NO_AUTO_UPDATE".into(), "1".into()),
                        ("HOMEBREW_NO_INSTALL_CLEANUP".into(), "1".into()),
                    ],
                    record_alias: None,
                }
            }
            _ => invalid_action("unknown package action"),
        }
    }

    async fn teardown(&self) {}
}

impl PackagesModule {
    async fn copy(&self, text: &str, cancel: &CancellationToken) -> ActionOutcome {
        match await_unless_cancelled(cancel, self.pasteboard.write_text(text)).await {
            None => ActionOutcome::Cancelled,
            Some(Ok(())) => ActionOutcome::Success {
                message: Some("copied package value".into()),
            },
            Some(Err(error)) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: error.to_string(),
                    retryable: true,
                },
            },
        }
    }
}

fn parse_query(rest: &str) -> Result<PackageQuery, &'static str> {
    let normalized = rest.trim();
    if normalized.is_empty() || normalized.eq_ignore_ascii_case("installed") {
        return Ok(PackageQuery::Installed);
    }
    if normalized.eq_ignore_ascii_case("outdated") {
        return Ok(PackageQuery::Outdated);
    }
    if normalized.eq_ignore_ascii_case("formulae") {
        return Ok(PackageQuery::Formulae);
    }
    if normalized.eq_ignore_ascii_case("casks") {
        return Ok(PackageQuery::Casks);
    }
    if let Some(value) = strip_command(normalized, "search") {
        if value.is_empty() {
            return Err("expected: /pkg search <name>");
        }
        return Ok(PackageQuery::Search(value.into()));
    }
    if let Some(value) = strip_command(normalized, "info") {
        if value.is_empty() {
            return Err("expected: /pkg info <exact-name>");
        }
        return Ok(PackageQuery::Info(value.into()));
    }
    Err("use installed, outdated, formulae, casks, search <name>, or info <name>")
}

fn strip_command<'a>(input: &'a str, command: &str) -> Option<&'a str> {
    let (word, rest) = input.split_once(char::is_whitespace).unwrap_or((input, ""));
    word.eq_ignore_ascii_case(command).then_some(rest.trim())
}

fn package_item(record: PackageRecord, info: bool) -> SearchItemDto {
    let mut state = Vec::new();
    if record.installed {
        state.push("installed");
    }
    if record.outdated {
        state.push("outdated");
    }
    if let Some(version) = &record.version {
        state.push(version);
    }
    let subtitle = format!(
        "{}{}{}",
        record.kind.label(),
        if state.is_empty() {
            String::new()
        } else {
            format!(" · {}", state.join(" · "))
        },
        record
            .description
            .as_ref()
            .map(|description| format!(" · {description}"))
            .unwrap_or_default()
    );
    let primary = if info { "copy_name" } else { "show_info" };
    let primary_label = if info { "Copy name" } else { "Info" };
    let mut secondary_actions = Vec::new();
    if record.homepage.is_some() {
        secondary_actions.push(action_dto(
            "copy_homepage",
            "Copy homepage",
            ActionRisk::Safe,
            false,
        ));
    }
    if !record.installed {
        secondary_actions.push(action_dto("install", "Install", ActionRisk::Confirm, true));
    }
    if record.outdated {
        secondary_actions.push(action_dto("upgrade", "Upgrade", ActionRisk::Confirm, true));
    }
    if record.installed {
        secondary_actions.push(action_dto(
            "uninstall",
            "Uninstall",
            ActionRisk::Confirm,
            true,
        ));
    }
    SearchItemDto {
        id: format!("pkg:{}:{}", record.kind.label(), record.name),
        module_id: MODULE_ID.into(),
        title: record.name.clone(),
        subtitle: Some(subtitle),
        kind: if info {
            "package_info".into()
        } else {
            "package".into()
        },
        score: 75.0,
        primary_action_id: primary.into(),
        primary_action_label: primary_label.into(),
        secondary_actions,
        action_payload: Some(serde_json::json!({
            "name": record.name,
            "kind": record.kind.label(),
            "homepage": record.homepage,
            "installed": record.installed,
            "outdated": record.outdated,
        })),
        ..Default::default()
    }
}

fn parse_kind(value: &str) -> Option<PackageKind> {
    match value {
        "formula" => Some(PackageKind::Formula),
        "cask" => Some(PackageKind::Cask),
        _ => None,
    }
}

fn action_dto(id: &str, label: &str, risk: ActionRisk, confirmation: bool) -> ActionDescriptorDto {
    ActionDescriptorDto {
        id: id.into(),
        label: label.into(),
        risk,
        confirmation,
    }
}

fn safe_action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        risk: ActionRisk::Safe,
        confirmation: false,
    }
}

fn confirm_action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        risk: ActionRisk::Confirm,
        confirmation: true,
    }
}

fn invalid_action(message: &str) -> ActionOutcome {
    ActionOutcome::Failed {
        kind: FailureKind::InvalidInput {
            field: "action".into(),
            message: message.into(),
        },
    }
}

fn package_outcome(error: PackageError) -> ActionOutcome {
    if error == PackageError::Cancelled {
        return ActionOutcome::Cancelled;
    }
    ActionOutcome::Failed {
        kind: match error {
            PackageError::NotConfigured => FailureKind::NotConfigured {
                remediation: "Install Homebrew, then retry /pkg".into(),
            },
            PackageError::Unavailable(reason) => FailureKind::Unavailable {
                reason,
                retryable: true,
            },
            PackageError::Timeout => FailureKind::Timeout {
                operation: "Homebrew query".into(),
            },
            PackageError::CommandFailed(reason)
            | PackageError::Malformed(reason)
            | PackageError::Conflict(reason) => FailureKind::Conflict { reason },
            PackageError::OutputTooLarge(limit) => FailureKind::Unavailable {
                reason: format!("Homebrew output exceeded {limit} bytes"),
                retryable: false,
            },
            PackageError::NotFound => FailureKind::NotFound {
                entity: "Homebrew package".into(),
            },
            PackageError::Ambiguous => FailureKind::Conflict {
                reason: "Homebrew package identity is ambiguous".into(),
            },
            PackageError::Cancelled => unreachable!(),
        },
    }
}

async fn send_status(sink: &SearchSink, id: &str, title: &str, subtitle: &str, kind: &str) {
    let _ = sink
        .send(Event::ResultsChunk {
            request_id: String::new(),
            sequence: 1,
            upserts: vec![SearchItemDto {
                id: id.into(),
                module_id: MODULE_ID.into(),
                title: title.into(),
                subtitle: Some(subtitle.into()),
                kind: kind.into(),
                score: 0.0,
                primary_action_id: "noop".into(),
                primary_action_label: "OK".into(),
                ..Default::default()
            }],
            removed_ids: vec![],
        })
        .await;
}

async fn send_package_error(sink: &SearchSink, error: PackageError) {
    match error {
        PackageError::NotConfigured => {
            send_status(
                sink,
                "pkg:not-configured",
                "Homebrew is not installed",
                "Install Homebrew, then retry /pkg",
                "not_configured",
            )
            .await
        }
        PackageError::Unavailable(reason) => {
            send_status(
                sink,
                "pkg:unavailable",
                "Homebrew is unavailable",
                &reason,
                "unavailable",
            )
            .await
        }
        PackageError::Timeout => {
            send_status(
                sink,
                "pkg:timeout",
                "Homebrew query timed out",
                "Retry the targeted query",
                "unavailable",
            )
            .await
        }
        PackageError::Cancelled => {}
        other => {
            send_status(
                sink,
                "pkg:failed",
                "Homebrew query failed",
                &other.to_string(),
                "command_error",
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{FakePackageManager, FakePasteboard};

    fn record(name: &str, kind: PackageKind, installed: bool, outdated: bool) -> PackageRecord {
        PackageRecord {
            name: name.into(),
            kind,
            description: Some("fixture package".into()),
            homepage: Some("https://example.test".into()),
            version: Some("1.0".into()),
            installed,
            outdated,
        }
    }

    #[tokio::test]
    async fn mutation_requires_confirmation_and_returns_exact_terminal_plan() {
        let packages = Arc::new(FakePackageManager::new(vec![record(
            "ripgrep",
            PackageKind::Formula,
            false,
            false,
        )]));
        let module = PackagesModule::with_deps(packages.clone(), Arc::new(FakePasteboard::new()));
        let item =
            package_item(record("ripgrep", PackageKind::Formula, false, false), true).into_domain();
        let install = confirm_action("install", "Install");
        let denied = module
            .perform(
                ActionRequest {
                    result: item.clone(),
                    action: install.clone(),
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            denied,
            ActionOutcome::Failed {
                kind: FailureKind::SecurityDenied { .. }
            }
        ));
        assert!(packages.mutations.lock().unwrap().is_empty());

        let outcome = module
            .perform(
                ActionRequest {
                    result: item,
                    action: install,
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert_eq!(
            outcome,
            ActionOutcome::InteractiveTerminal {
                program: "/fixture/brew".into(),
                args: vec!["install".into(), "ripgrep".into()],
                environment: vec![
                    ("HOMEBREW_NO_AUTO_UPDATE".into(), "1".into()),
                    ("HOMEBREW_NO_INSTALL_CLEANUP".into(), "1".into()),
                ],
                record_alias: None,
            }
        );
    }

    #[tokio::test]
    async fn stale_package_state_blocks_terminal_plan() {
        let packages = Arc::new(FakePackageManager::new(vec![record(
            "ripgrep",
            PackageKind::Formula,
            true,
            false,
        )]));
        let module = PackagesModule::with_deps(packages, Arc::new(FakePasteboard::new()));
        let item =
            package_item(record("ripgrep", PackageKind::Formula, false, false), true).into_domain();
        let outcome = module
            .perform(
                ActionRequest {
                    result: item,
                    action: confirm_action("install", "Install"),
                    confirmation: true,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(
            outcome,
            ActionOutcome::Failed {
                kind: FailureKind::Conflict { .. }
            }
        ));
    }

    #[tokio::test]
    async fn cancellation_prevents_mutation_discovery() {
        let packages = Arc::new(FakePackageManager::new(vec![record(
            "zed",
            PackageKind::Cask,
            true,
            true,
        )]));
        let module = PackagesModule::with_deps(packages.clone(), Arc::new(FakePasteboard::new()));
        let cancel = CancellationToken::new();
        cancel.cancel();
        let outcome = module
            .perform(
                ActionRequest {
                    result: package_item(record("zed", PackageKind::Cask, true, true), true)
                        .into_domain(),
                    action: confirm_action("upgrade", "Upgrade"),
                    confirmation: true,
                },
                cancel,
            )
            .await;
        assert_eq!(outcome, ActionOutcome::Cancelled);
        assert!(packages.mutations.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn missing_brew_and_command_failure_are_distinct() {
        for (error, expected) in [
            (PackageError::NotConfigured, "not_configured"),
            (
                PackageError::CommandFailed("fixture".into()),
                "command_error",
            ),
        ] {
            let packages = Arc::new(FakePackageManager::new(vec![]));
            packages.fail_with(error);
            let module = PackagesModule::with_deps(packages, Arc::new(FakePasteboard::new()));
            let (sink, mut receiver) = tokio::sync::mpsc::channel(2);
            module
                .search(
                    Query::parse_with_prefixes_strict("/pkg ", 50, |value| value == "pkg"),
                    sink,
                    CancellationToken::new(),
                )
                .await;
            let Event::ResultsChunk { upserts, .. } = receiver.recv().await.unwrap() else {
                panic!("expected results");
            };
            assert_eq!(upserts[0].kind, expected);
        }
    }
}
