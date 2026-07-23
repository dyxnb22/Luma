use crate::appkit::SnapshotNotifier;
use crate::launcher::TerminalLauncher;
use crate::model::{
    ActionStatus, CliStatus, LoginItemState, MenuAction, MenuSnapshot, SharedMenuSnapshot,
    WindowsStatus, WordbookStatus,
};
use luma_application::WindowCatalogPort;
use luma_platform_macos::MacWindowCatalog;
use luma_storage::{luma_next_support_dir, ConfigReadError, ConfigStore, WordbookReadOnlyStore};
use objc2_service_management::{SMAppService, SMAppServiceStatus};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// The worker owns all platform I/O. It publishes plain snapshots through a thread-safe store
/// and schedules a main-thread redraw; AppKit never performs worker I/O while rendering.
pub fn spawn_worker(
    actions: Receiver<MenuAction>,
    snapshots: SharedMenuSnapshot,
    initial_snapshot: MenuSnapshot,
    notifier: SnapshotNotifier,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("luma-menubar-worker".into())
        .spawn(move || {
            let mut snapshot = initial_snapshot;
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("menubar worker runtime");
            let window_catalog = Arc::new(MacWindowCatalog::new());
            let support_dir = luma_next_support_dir().unwrap_or_else(|_| PathBuf::from("."));
            let luma_path = resolve_luma_path();
            let cli = match luma_path.as_ref() {
                Some(_) => CliStatus::Available,
                None => {
                    CliStatus::Unavailable("Luma CLI not found next to the menu-bar app".into())
                }
            };
            let launcher = luma_path.map(|path| TerminalLauncher::new(path, support_dir));
            let mut pending_action = None;
            loop {
                let action = match pending_action.take() {
                    Some(action) => action,
                    None => match actions.recv() {
                        Ok(action) => action,
                        Err(_) => break,
                    },
                };
                match action {
                    MenuAction::Refresh => {
                        snapshot = load_snapshot(
                            &runtime,
                            window_catalog.as_ref(),
                            &snapshot,
                            cli.clone(),
                        );
                        publish(&snapshots, &mut snapshot, &notifier);
                        coalesce_refreshes(&actions, &mut pending_action);
                    }
                    MenuAction::OpenLuma => {
                        snapshot.last_action = Some(launch_action(launcher.as_ref(), None));
                        publish(&snapshots, &mut snapshot, &notifier);
                    }
                    MenuAction::OpenSettings => {
                        snapshot.last_action =
                            Some(launch_action(launcher.as_ref(), Some("/settings")));
                        publish(&snapshots, &mut snapshot, &notifier);
                    }
                    MenuAction::ReviewDue => {
                        snapshot.last_action =
                            Some(launch_action(launcher.as_ref(), Some("/wb review due")));
                        publish(&snapshots, &mut snapshot, &notifier);
                    }
                    MenuAction::FocusWindow(window_id) => {
                        let result = runtime.block_on(window_catalog.focus(&window_id));
                        snapshot.last_action = Some(match result {
                            Ok(()) => ActionStatus::Succeeded("focused selected window".into()),
                            Err(luma_application::WindowError::PermissionRequired { .. }) => {
                                ActionStatus::Failed(menubar_accessibility_guidance())
                            }
                            Err(err) => ActionStatus::Failed(err.to_string()),
                        });
                        publish(&snapshots, &mut snapshot, &notifier);
                    }
                    MenuAction::ToggleLaunchAtLogin => {
                        let enable = !matches!(snapshot.login_item, LoginItemState::Enabled);
                        snapshot.last_action = Some(match set_launch_at_login(enable) {
                            Ok(()) => ActionStatus::Succeeded(if enable {
                                "launch at login enabled".into()
                            } else {
                                "launch at login disabled".into()
                            }),
                            Err(error) => ActionStatus::Failed(error),
                        });
                        snapshot.login_item = current_login_item_state();
                        publish(&snapshots, &mut snapshot, &notifier);
                    }
                    MenuAction::Quit => break,
                }
            }
        })
        .expect("menubar worker thread")
}

/// Refreshes are cheap to request from AppKit, but they should not queue up behind one another
/// while the worker is reading SQLite and the window catalog. Preserve the first non-refresh
/// action so user-initiated commands still run in their original order.
fn coalesce_refreshes(actions: &Receiver<MenuAction>, pending_action: &mut Option<MenuAction>) {
    loop {
        match actions.try_recv() {
            Ok(MenuAction::Refresh) => {}
            Ok(action) => {
                *pending_action = Some(action);
                return;
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
        }
    }
}

fn publish(
    snapshots: &SharedMenuSnapshot,
    snapshot: &mut MenuSnapshot,
    notifier: &SnapshotNotifier,
) {
    snapshot.generation = snapshot.generation.saturating_add(1);
    snapshot.captured_at_unix = unix_now();
    if let Ok(mut current) = snapshots.lock() {
        *current = snapshot.clone();
        notifier.schedule();
    }
}

fn launch_action(launcher: Option<&TerminalLauncher>, query: Option<&str>) -> ActionStatus {
    let Some(launcher) = launcher else {
        return ActionStatus::Failed(
            "Luma CLI not found; set LUMA_CLI_PATH or install the app bundle".into(),
        );
    };
    match launcher.launch(query) {
        Ok(()) => ActionStatus::Succeeded(match query {
            None => "opened Luma".into(),
            Some(query) => format!("opened {query} in Luma"),
        }),
        Err(error) => ActionStatus::Failed(error),
    }
}

fn load_snapshot(
    runtime: &tokio::runtime::Runtime,
    catalog: &MacWindowCatalog,
    previous: &MenuSnapshot,
    cli: CliStatus,
) -> MenuSnapshot {
    let global_warning = match luma_next_support_dir() {
        Ok(dir) => match ConfigStore::with_path(dir.join("settings.toml")).load_existing() {
            Ok(_) | Err(ConfigReadError::NotConfigured) => None,
            Err(error) => Some(error.to_string()),
        },
        Err(error) => Some(error.to_string()),
    };
    let wordbook = load_wordbook(previous);
    let windows = load_windows(runtime, catalog, previous);
    MenuSnapshot {
        generation: previous.generation,
        captured_at_unix: previous.captured_at_unix,
        wordbook,
        windows,
        cli,
        login_item: current_login_item_state(),
        global_warning,
        // Keep the last action visible until a later action replaces it; a refresh must not
        // erase an error before the user has had a chance to see it.
        last_action: previous.last_action.clone(),
    }
}

fn load_wordbook(previous: &MenuSnapshot) -> WordbookStatus {
    let result = luma_next_support_dir()
        .map(|dir| dir.join("wordbook.sqlite"))
        .map(|path| load_wordbook_path(&path))
        .unwrap_or_else(|error| Err(error.to_string()));
    map_wordbook_result(result, previous)
}

fn load_wordbook_path(path: &std::path::Path) -> Result<luma_storage::WordbookStats, String> {
    WordbookReadOnlyStore::with_path(path.to_path_buf())
        .map_err(|error| error.to_string())
        .and_then(|store| store.stats().map_err(|error| error.to_string()))
}

fn map_wordbook_result(
    result: Result<luma_storage::WordbookStats, String>,
    previous: &MenuSnapshot,
) -> WordbookStatus {
    match result {
        Ok(stats) => WordbookStatus::Ready {
            due: stats.due,
            reviewed_today: stats.reviewed_today,
            goal: stats.goal,
        },
        Err(error) if error == "wordbook not configured" => WordbookStatus::NotConfigured,
        Err(error) => match &previous.wordbook {
            WordbookStatus::Ready {
                due,
                reviewed_today,
                goal,
            }
            | WordbookStatus::Stale {
                due,
                reviewed_today,
                goal,
                ..
            } => WordbookStatus::Stale {
                due: *due,
                reviewed_today: *reviewed_today,
                goal: *goal,
                reason: error,
            },
            _ => WordbookStatus::Unavailable(error),
        },
    }
}

fn load_windows(
    runtime: &tokio::runtime::Runtime,
    catalog: &MacWindowCatalog,
    previous: &MenuSnapshot,
) -> WindowsStatus {
    match runtime.block_on(catalog.list_windows()) {
        Ok(windows) => WindowsStatus::Ready(project_windows(windows)),
        Err(luma_application::WindowError::PermissionRequired { .. }) => {
            WindowsStatus::PermissionRequired(menubar_accessibility_guidance())
        }
        Err(error) => match &previous.windows {
            WindowsStatus::Ready(windows) | WindowsStatus::Stale { windows, .. }
                if !windows.is_empty() =>
            {
                WindowsStatus::Stale {
                    windows: windows.clone(),
                    reason: error.to_string(),
                }
            }
            _ => WindowsStatus::Unavailable(error.to_string()),
        },
    }
}

fn project_windows(
    mut windows: Vec<luma_application::WindowEntry>,
) -> Vec<luma_application::WindowEntry> {
    windows.retain(|window| window.is_on_screen && window.layer == 0);
    windows.truncate(5);
    windows
}

fn menubar_accessibility_guidance() -> String {
    "Accessibility required — grant Accessibility to Luma Menu Bar.app in System Settings → Privacy & Security → Accessibility, then retry.".into()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn resolve_luma_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("LUMA_CLI_PATH") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("luma")))
        .filter(|path| path.is_file())
}

fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    let service = unsafe { SMAppService::mainAppService() };
    let result = if enabled {
        unsafe { service.registerAndReturnError() }
    } else {
        unsafe { service.unregisterAndReturnError() }
    };
    result.map_err(|error| {
        format!(
            "macOS rejected the login-item change ({error}); use a bundled app and System Settings approval"
        )
    })
}

fn current_login_item_state() -> LoginItemState {
    let status = unsafe { SMAppService::mainAppService().status() };
    match status {
        SMAppServiceStatus::NotRegistered => LoginItemState::NotRegistered,
        SMAppServiceStatus::Enabled => LoginItemState::Enabled,
        SMAppServiceStatus::RequiresApproval => LoginItemState::RequiresApproval,
        SMAppServiceStatus::NotFound => LoginItemState::NotFound,
        other => LoginItemState::Unavailable(format!("unknown login-item status {other:?}")),
    }
}

pub fn initial_snapshot() -> MenuSnapshot {
    MenuSnapshot {
        login_item: current_login_item_state(),
        ..MenuSnapshot::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        coalesce_refreshes, load_wordbook_path, map_wordbook_result, project_windows, unix_now,
    };
    use crate::model::{MenuAction, MenuSnapshot, WordbookStatus};
    use luma_application::WindowEntry;
    use std::sync::mpsc;
    use tempfile::tempdir;

    #[test]
    fn unix_clock_is_nonzero_on_supported_hosts() {
        assert!(unix_now() > 0);
    }

    #[test]
    fn refresh_coalescing_preserves_first_non_refresh_action() {
        let (sender, receiver) = mpsc::channel();
        sender.send(MenuAction::Refresh).unwrap();
        sender.send(MenuAction::Refresh).unwrap();
        sender.send(MenuAction::OpenSettings).unwrap();
        sender.send(MenuAction::Refresh).unwrap();

        let mut pending = None;
        coalesce_refreshes(&receiver, &mut pending);

        assert_eq!(pending, Some(MenuAction::OpenSettings));
        assert_eq!(receiver.recv().unwrap(), MenuAction::Refresh);
    }

    #[test]
    fn window_projection_preserves_front_to_back_order_and_cap() {
        let windows = (0..7)
            .map(|index| WindowEntry {
                id: format!("window-{index}"),
                app_name: format!("App {index}"),
                app_bundle_id: None,
                title: format!("Title {index}"),
                is_on_screen: true,
                layer: 0,
                owner_pid: index,
            })
            .collect();
        let projected = project_windows(windows);
        assert_eq!(projected.len(), 5);
        assert_eq!(projected[0].id, "window-0");
        assert_eq!(projected[4].id, "window-4");
    }

    #[test]
    fn wordbook_missing_and_corrupt_states_are_honest() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("wordbook.sqlite");
        let missing = load_wordbook_path(&path).unwrap_err();
        assert_eq!(missing, "wordbook not configured");
        assert_eq!(
            map_wordbook_result(Err(missing), &MenuSnapshot::default()),
            WordbookStatus::NotConfigured
        );

        std::fs::write(&path, b"not sqlite").unwrap();
        let corrupt = load_wordbook_path(&path).unwrap_err();
        assert!(corrupt.contains("wordbook sqlite"));
        assert!(matches!(
            map_wordbook_result(Err(corrupt.clone()), &MenuSnapshot::default()),
            WordbookStatus::Unavailable(reason) if reason == corrupt
        ));

        let previous = MenuSnapshot {
            wordbook: WordbookStatus::Ready {
                due: 3,
                reviewed_today: 1,
                goal: 5,
            },
            ..MenuSnapshot::default()
        };
        assert!(matches!(
            map_wordbook_result(Err(corrupt), &previous),
            WordbookStatus::Stale { due: 3, .. }
        ));
    }
}
