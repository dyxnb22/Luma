use crate::launcher::TerminalLauncher;
use crate::model::{MenuAction, MenuSnapshot, WindowsStatus, WordbookStatus};
use luma_application::WindowCatalogPort;
use luma_platform_macos::MacWindowCatalog;
use luma_storage::{luma_next_support_dir, ConfigReadError, ConfigStore, WordbookReadOnlyStore};
use objc2_service_management::{SMAppService, SMAppServiceStatus};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

pub fn spawn_worker(
    actions: Receiver<MenuAction>,
    on_snapshot: Box<dyn Fn(MenuSnapshot) + Send + 'static>,
    on_quit: Box<dyn Fn() + Send + 'static>,
    initial_snapshot: MenuSnapshot,
) {
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
            let cli_available = luma_path.is_some();
            let launcher = luma_path.map(|path| TerminalLauncher::new(path, support_dir));
            while let Ok(action) = actions.recv() {
                match action {
                    MenuAction::Refresh => {
                        snapshot = load_snapshot(
                            &runtime,
                            window_catalog.as_ref(),
                            snapshot.launch_at_login,
                            cli_available,
                        );
                        on_snapshot(snapshot.clone());
                    }
                    MenuAction::OpenLuma => {
                        if let Some(launcher) = launcher.as_ref() {
                            let _ = launcher.launch(None);
                        }
                    }
                    MenuAction::OpenSettings => {
                        if let Some(launcher) = launcher.as_ref() {
                            let _ = launcher.launch(Some("/settings"));
                        }
                    }
                    MenuAction::ReviewDue => {
                        if let Some(launcher) = launcher.as_ref() {
                            let _ = launcher.launch(Some("/wb review due"));
                        }
                    }
                    MenuAction::FocusWindow(index) => {
                        if let WindowsStatus::Ready(windows) = &snapshot.windows {
                            if let Some(window) = windows.get(index) {
                                let _ = runtime.block_on(window_catalog.focus(&window.id));
                            }
                        }
                    }
                    MenuAction::ToggleLaunchAtLogin => {
                        let enabled = !snapshot.launch_at_login;
                        let result = set_launch_at_login(enabled);
                        if result.is_ok() {
                            snapshot.launch_at_login = enabled;
                            on_snapshot(snapshot.clone());
                        }
                    }
                    MenuAction::Quit => {
                        on_quit();
                        break;
                    }
                }
            }
        })
        .expect("menubar worker thread");
}

fn load_snapshot(
    runtime: &tokio::runtime::Runtime,
    catalog: &MacWindowCatalog,
    launch_at_login: bool,
    cli_available: bool,
) -> MenuSnapshot {
    let global_warning = match luma_next_support_dir() {
        Ok(dir) => match ConfigStore::with_path(dir.join("settings.toml")).load_existing() {
            Ok(_) | Err(ConfigReadError::NotConfigured) => None,
            Err(error) => Some(error.to_string()),
        },
        Err(error) => Some(error.to_string()),
    };
    let wordbook = match luma_next_support_dir()
        .map(|dir| dir.join("wordbook.sqlite"))
        .map_err(|e| e.to_string())
        .and_then(|path| WordbookReadOnlyStore::with_path(path).map_err(|e| e.to_string()))
    {
        Ok(store) => match store.stats() {
            Ok(stats) => WordbookStatus::Ready {
                due: stats.due,
                reviewed_today: stats.reviewed_today,
                goal: stats.goal,
            },
            Err(err) => WordbookStatus::Unavailable(err.to_string()),
        },
        Err(err) if err == "wordbook not configured" => WordbookStatus::NotConfigured,
        Err(err) => WordbookStatus::Unavailable(err),
    };
    let windows = match runtime.block_on(catalog.list_windows()) {
        Ok(mut windows) => {
            windows.retain(|window| window.is_on_screen && window.layer == 0);
            windows.sort_by(|a, b| a.app_name.cmp(&b.app_name).then(a.title.cmp(&b.title)));
            windows.truncate(5);
            WindowsStatus::Ready(windows)
        }
        Err(err) => {
            let message = if matches!(
                err,
                luma_application::WindowError::PermissionRequired { .. }
            ) {
                format!("Accessibility required — {err}")
            } else {
                err.to_string()
            };
            WindowsStatus::Unavailable(message)
        }
    };
    MenuSnapshot {
        wordbook,
        windows,
        cli_available,
        launch_at_login,
        global_warning,
    }
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

pub fn initial_snapshot() -> MenuSnapshot {
    let launch_at_login =
        unsafe { SMAppService::mainAppService().status() } == SMAppServiceStatus::Enabled;
    MenuSnapshot {
        launch_at_login,
        ..MenuSnapshot::default()
    }
}
