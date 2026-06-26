import AppKit
import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
final class AppCoordinator {
    private let windowController = LauncherWindowController()
    private var hotkeyController: HotkeyController?
    private var menuBarController: MenuBarController?
    private var settingsWindowController: SettingsWindowController?
    private let logger = LumaLogger()
    private let metrics = LumaMetrics()
    private let database = ApplicationSupportPaths()
    private let pasteboard = PasteboardService()
    private let accessibility = AXService()
    private let workspace = WorkspaceService()
    private let fileSystem = FSEventsService()
    private let config = ConfigurationStore()
    private lazy var translation = TranslationService(config: config)
    private lazy var clipboardSnapshotService = ClipboardSnapshotService()
    private lazy var launcherUIService = AppLauncherUIService(
        onSecretsLockStateChanged: { [weak self] locked in
            self?.menuBarController?.setSecretsLockState(locked: locked)
            ModuleDetailReloads.reloadSecretsDetail?()
        },
        onHideLauncher: { [weak self] in
            self?.windowController.hideImmediatelyForAction()
        }
    )
    private lazy var context = ModuleContext(
        logger: logger,
        metrics: metrics,
        database: database,
        pasteboard: pasteboard,
        accessibility: accessibility,
        fileSystem: fileSystem,
        translation: translation,
        config: config,
        workspace: workspace,
        clipboardSnapshot: clipboardSnapshotService,
        launcherUI: launcherUIService
    )
    private lazy var host = ModuleHost(context: context)
    private var hostClient: AppHostService!
    private let usage = PersistentUsageTracker.defaultTracker()
    private let commandUsage = CommandUsageTracker.defaultTracker()
    private let resultCache = UsageResultCache.defaultCache()
    private lazy var dispatcher = QueryDispatcher(host: host, usage: usage, resultCache: resultCache, metrics: metrics)
    private lazy var actionExecutor = ActionExecutor(
        host: host,
        context: ActionContext(
            logger: logger,
            metrics: metrics,
            pasteboard: pasteboard,
            accessibility: accessibility,
            workspace: workspace,
            host: hostClient,
            launcherUI: launcherUIService
        ),
        pasteboard: pasteboard,
        accessibility: accessibility,
        translation: translation,
        workspace: workspace,
        usage: usage,
        resultCache: resultCache
    )
    private lazy var viewModel = LauncherViewModel(dispatcher: dispatcher, commandUsage: commandUsage)
    private let appActivationTracker = AppActivationTracker.defaultTracker()
    private lazy var openAppsProvider = OpenAppsHomeProvider(appActivationTracker: appActivationTracker)
    private let clipboardModule = ClipboardModule()
    private let todoModule = TodoModule()
    private let secretsModule = SecretsModule()
    private let mediaModule = MediaModule()
    private lazy var homeCoordinator = LauncherHomeCoordinator(
        openApps: openAppsProvider,
        contextual: ContextualHomeProvider(todoModule: todoModule, mediaModule: mediaModule)
    )
    private var activationObserver: NSObjectProtocol?
    private var terminationObserver: NSObjectProtocol?

    func start() {
        hostClient = AppHostService(
            onOpenSettings: { [weak self] in
                self?.windowController.hideImmediatelyForAction()
                self?.settingsWindowController?.show()
            },
            onReloadModules: { [weak self] in
                guard let self else { return }
                Task {
                    await self.host.warmupAll()
                    self.windowController.refreshOpenApps()
                }
            }
        )
        settingsWindowController = SettingsWindowController(
            config: config,
            usage: usage,
            onModulesChanged: { [weak self] enabled in
                guard let self else { return }
                Task { await self.host.applyEnabledSet(enabled) }
            },
            onClipboardSettingsChanged: { [weak self] snapshot in
                guard let self else { return }
                Task {
                    await self.clipboardModule.applyRetentionSettings(
                        maxEntries: snapshot.clipboardMaxEntries,
                        maxAgeDays: snapshot.clipboardMaxAgeDays,
                        maxEntrySizeKB: snapshot.clipboardMaxEntrySizeKB
                    )
                    await self.clipboardModule.applyCaptureSettings(
                        enabled: snapshot.clipboardHistoryEnabled,
                        ignoredBundleIDs: snapshot.clipboardIgnoredBundleIDs,
                        pasteBehavior: ClipboardPasteBehavior(rawValue: snapshot.clipboardPasteBehavior) ?? .pasteDirectly
                    )
                }
            },
            onSecretsSettingsChanged: { [weak self] autoClear, relock in
                guard let self else { return }
                Task { await self.secretsModule.applySettings(autoClearSeconds: autoClear, relockTimeoutSeconds: relock) }
            },
            onLatencyHUDChanged: { [weak self] enabled in
                self?.windowController.setLatencyHUDEnabled(enabled)
            }
        )
        activationObserver = NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.didActivateApplicationNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            guard let self,
                  let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication,
                  let bundleID = app.bundleIdentifier else { return }
            Task { @MainActor in
                await self.appActivationTracker.record(bundleID: bundleID)
                self.windowController.hideIfShowingForExternalActivation(bundleID: bundleID)
                self.windowController.refreshOpenApps()
            }
        }
        terminationObserver = NotificationCenter.default.addObserver(
            forName: NSApplication.willTerminateNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            guard let self else { return }
            Task { @MainActor in
                await self.appActivationTracker.flush()
            }
        }
        menuBarController = MenuBarController(
            onShow: { self.windowController.show() },
            onSettings: { [weak self] in
                self?.windowController.hideImmediatelyForAction()
                self?.settingsWindowController?.show()
            }
        )
        do {
            let hotkeyController = try HotkeyController {
                self.windowController.toggle()
            }
            try hotkeyController.register(HotkeyConfig.load())
            self.hotkeyController = hotkeyController
            menuBarController?.markHotkeyOK()
        } catch {
            Task {
                await LumaLogger(category: "hotkey").error("Failed to register global hotkey: \(error)")
            }
            menuBarController?.markHotkeyFailed()
        }

        let notesModule = NotesModule()

        // One-way migration of the wordbot SQLite into Luma's Application Support (ADR-009).
        // Idempotent: re-running the app never overwrites the Luma copy.
        Task.detached(priority: .utility) {
            do {
                _ = try WordbookMigrator.migrateIfNeeded()
            } catch {
                WordbookMigrator.setMigrationNotice(.failed)
                await LumaLogger(category: "wordbook").error("Wordbook migration failed: \(error)")
            }
        }

        let wordbookStore = WordbookStore()
        let wordbookModule = WordbookModule(store: wordbookStore)
        let snippetsModule = SnippetsModule()

        let launcherEnv = LauncherEnvironment(
            openModuleDetail: { [weak self] id in
                self?.windowController.openModuleDetail(for: id)
            },
            openSettings: { [weak self] in
                self?.windowController.hideImmediatelyForAction()
                self?.settingsWindowController?.show()
            },
            reloadModules: { [weak self] in
                guard let self else { return }
                Task {
                    await self.host.warmupAll()
                    await MainActor.run {
                        self.windowController.refreshOpenApps()
                    }
                }
            },
            onBackFromDetail: { [weak self] in
                self?.windowController.closeDetailIfShowing()
            },
            onTranslateContentChanged: { [weak self] source, output in
                guard let self else { return }
                Task {
                    await self.config.setLauncherTranslateSourceText(source)
                    await self.config.setLauncherTranslateOutputText(output)
                }
            },
            onHideLauncher: { [weak self] in
                self?.windowController.hideImmediatelyForAction()
            },
            reloadSnippetsDetail: { ModuleDetailReloads.reloadSnippetsDetail?() },
            clipboardModule: clipboardModule,
            notesModule: notesModule,
            snippetsModule: snippetsModule,
            secretsModule: secretsModule,
            mediaModule: mediaModule,
            todoModule: todoModule,
            wordbookStore: wordbookStore,
            translation: translation,
            config: config
        )
        launcherEnv.install()

        windowController.configure(
            viewModel: viewModel,
            homeCoordinator: homeCoordinator,
            actionExecutor: actionExecutor,
            config: config,
            launcherEnvironment: launcherEnv,
            onOpenSettings: { [weak self] in
                self?.windowController.hideImmediatelyForAction()
                self?.settingsWindowController?.show()
            }
        )

        Task {
            let modules = BuiltInModules.makeAll(overrides: .init(
                clipboard: clipboardModule,
                notes: notesModule,
                todo: todoModule,
                wordbook: wordbookModule,
                snippets: snippetsModule,
                secrets: secretsModule,
                media: mediaModule
            ))
            for module in modules {
                await host.register(module)
            }
            await host.applyEnabledSet(await config.enabledModules())
            await host.warmupAll()
            windowController.setModulesReady(true)
        }
    }
}
