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
    private let detailReloadRouter = ModuleDetailReloadRouter()
    private lazy var clipboardSnapshotService = ClipboardSnapshotService()
    private lazy var launcherUIService = AppLauncherUIService(
        detailReloadRouter: detailReloadRouter,
        onSecretsLockStateChanged: { [weak self] locked in
            self?.menuBarController?.setSecretsLockState(locked: locked)
            self?.detailReloadRouter.reload(.secrets)
        },
        onHideLauncher: { [weak self] in
            self?.windowController.hideImmediatelyForAction()
        }
    )
    private let reminders = RemindersService()
    private let scriptRunner = ScriptRunnerService()
    private lazy var menuBarTreeService = MenuBarTreeService.shared
    private lazy var menuBarTreeClient = MenuBarTreeClientAdapter(service: menuBarTreeService)
    private lazy var currentProjectClient = CurrentProjectClientAdapter(service: CurrentProjectService.shared)
    private let selectionClient = SelectionSnapshotClientAdapter()
    /// Dedicated instance for `CurrentProjectService` path matching before `ModuleHost` registers the module.
    private let projectsModule = ProjectsModule()

    init() {
        CurrentProjectService.bootstrap(matcher: ProjectsModuleMatcher(module: projectsModule))
        CrashLogRecording.setHandler { message in
            Task { await CrashLogBuffer.shared.record(message) }
        }
    }

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
        launcherUI: launcherUIService,
        processMemory: ProcessMemoryService(sampler: processMemorySampler),
        reminders: reminders,
        scriptRunner: scriptRunner,
        notifications: NotificationService(),
        currentProject: currentProjectClient,
        selectionSnapshot: selectionClient,
        menuBarTree: menuBarTreeClient,
        runningApplications: runningApplicationsCache
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
            translation: translation,
            workspace: workspace,
            host: hostClient,
            launcherUI: launcherUIService,
            scriptRunner: scriptRunner,
            currentProject: currentProjectClient,
            selectionSnapshot: selectionClient
        ),
        usage: usage,
        resultCache: resultCache
    )
    private lazy var viewModel = LauncherViewModel(
        dispatcher: dispatcher,
        commandRouter: CommandRouter(registry: ModuleRegistry.makeCommandRegistry()),
        commandUsage: commandUsage
    )
    private let appActivationTracker = AppActivationTracker.defaultTracker()
    private let runningApplicationsCache = RunningApplicationsCache.shared
    private let processMemorySampler = ProcessMemorySampler.shared
    private lazy var openAppsProvider = OpenAppsHomeProvider(appActivationTracker: appActivationTracker)
    private lazy var clipboardModule = ClipboardModule(pasteboard: pasteboard, accessibility: accessibility)
    private lazy var notesModule = NotesModule()
    private lazy var todoModule = TodoModule()
    private let secretsModule = SecretsModule()
    private let mediaModule = MediaModule()
    private let quicklinksModule = QuicklinksModule()
    private lazy var wordbookStore = WordbookStore()
    private lazy var wordbookModule = WordbookModule(store: wordbookStore)
    private lazy var snippetsModule = SnippetsModule()
    private lazy var menuItemsModule = MenuItemsModule(service: menuBarTreeService)
    private lazy var homeCoordinator = LauncherHomeCoordinator(
        openApps: openAppsProvider,
        onHomeDataUpdated: { [weak self] in
            Task { @MainActor in self?.windowController.refreshHomeForBackgroundDataUpdate() }
        }
    )
    private var activationObserver: NSObjectProtocol?
    private var terminationObserver: NSObjectProtocol?
    private var notificationObservers: [NSObjectProtocol] = []
    private var memoryPressureSource: DispatchSourceMemoryPressure?
    private var idleTeardownTask: Task<Void, Never>?

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
                    self.windowController.refreshHome()
                }
            }
        )
        settingsWindowController = SettingsCoordinator(
            onModulesChanged: { [weak self] enabled in
                guard let self else { return }
                Task {
                    let previous = await self.config.enabledModules()
                        ?? ModuleRegistry.defaultEnabledModuleIDs
                    await self.host.applyEnabledSet(enabled)
                    let removed = previous.subtracting(enabled)
                    await MainActor.run {
                        if !removed.isEmpty {
                            self.windowController.handleModulesDisabled(removed: removed)
                        }
                    }
                }
            },
            onPinnedChanged: { [weak self] pinned in
                guard let self else { return }
                Task {
                    await self.host.configureWarmupPolicy(pinned: pinned)
                    await self.host.warmupIfNeeded(ids: pinned, reason: .startup)
                }
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
        ).makeWindowController(
            config: config,
            usage: usage,
            notesModule: notesModule,
            projectsModule: projectsModule,
            onNotesRootChosen: { [weak self] url in
                guard let self else { return }
                Task {
                    var config = await self.notesModule.loadConfig()
                    config.root = url.standardizedFileURL
                    try? await self.notesModule.saveConfig(config)
                    await self.host.warmupIfNeeded(ids: [.notes], reason: .startup)
                    self.detailReloadRouter.reload(.notes)
                }
            },
            onProjectsRootChosen: { [weak self] url in
                guard let self else { return }
                Task {
                    let payload = (try? ModuleActionCoding.encode(ProjectAction.addRoot(url.path))) ?? Data()
                    let action = Action(
                        id: ActionID(module: .projects, key: "settings.addRoot"),
                        title: "Add Root",
                        kind: .custom(payload: payload, handler: .projects)
                    )
                    _ = await self.actionExecutor.run(action, for: ResultID(module: .projects, key: "settings.addRoot"))
                    self.detailReloadRouter.reload(.projects)
                }
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
                guard bundleID != Bundle.main.bundleIdentifier else { return }
                let wasVisible = self.windowController.isPanelVisible
                self.windowController.hideIfShowingForExternalActivation(bundleID: bundleID)
                if wasVisible {
                    self.windowController.refreshHome()
                }
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
            let hotkeyController = try HotkeyController { [weak self] in
                self?.windowController.showFromCarbonHotkey()
            }
            try hotkeyController.register(HotkeyConfig.load())
            self.hotkeyController = hotkeyController
            LauncherRuntimeState.hotkeyRegistered = true
            menuBarController?.markHotkeyOK()
        } catch {
            LauncherRuntimeState.hotkeyRegistered = false
            let hotkeyMessage: String
            if case HotkeyError.registrationFailed(let status) = error {
                hotkeyMessage = "hotkey.registrationFailed status=\(status)"
            } else if case HotkeyError.handlerInstallFailed(let status) = error {
                hotkeyMessage = "hotkey.handlerInstallFailed status=\(status)"
            } else {
                hotkeyMessage = "hotkey.registrationFailed status=unknown"
            }
            CrashLogRecording.record(hotkeyMessage)
            Task {
                await LumaLogger(category: "hotkey").error("Failed to register global hotkey: \(error)")
            }
            menuBarController?.markHotkeyFailed()
        }

        notificationObservers.append(
            NSWorkspace.shared.notificationCenter.addObserver(
                forName: NSWorkspace.activeSpaceDidChangeNotification,
                object: nil,
                queue: .main
            ) { [weak self] _ in
                self?.reRegisterHotkey()
            }
        )
        notificationObservers.append(
            NSWorkspace.shared.notificationCenter.addObserver(
                forName: NSWorkspace.didWakeNotification,
                object: nil,
                queue: .main
            ) { [weak self] _ in
                Task { @MainActor in
                    try? await Task.sleep(for: .seconds(1))
                    self?.windowController.refreshHomeForBackgroundDataUpdate()
                }
            }
        )

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

        let wordbookStore = self.wordbookStore
        let wordbookModule = self.wordbookModule
        let snippetsModule = self.snippetsModule

        let launcherEnv = LauncherEnvironment(
            openModuleDetail: { [weak self] id in
                self?.windowController.openModuleDetail(for: id)
            },
            openSettings: { [weak self] in
                self?.windowController.hideImmediatelyForAction()
                self?.settingsWindowController?.show()
            },
            openTranslationSettings: { [weak self] in
                self?.windowController.hideImmediatelyForAction()
                self?.settingsWindowController?.show(section: .translation)
            },
            reloadModules: { [weak self] in
                guard let self else { return }
                self.windowController.invalidatePanelSignalsCache()
                self.windowController.invalidatePermissionModuleCache()
                Task {
                    await self.host.warmupAll()
                    await MainActor.run {
                        self.windowController.refreshHome()
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
            showStatus: { [weak self] message in
                self?.windowController.showStatus(message)
            },
            detailReloadRouter: detailReloadRouter,
            warmModuleForDetail: { [weak self] id in
                await self?.host.warmupIfNeeded(id: id, reason: .detail)
                await self?.host.setReservedModuleIDs([id])
            },
            reserveDetailModule: { [weak self] id in
                if let id {
                    await self?.host.setReservedModuleIDs([id])
                } else {
                    await self?.host.setReservedModuleIDs([])
                }
            },
            clipboardModule: clipboardModule,
            notesModule: notesModule,
            snippetsModule: snippetsModule,
            secretsModule: secretsModule,
            mediaModule: mediaModule,
            todoModule: todoModule,
            wordbookStore: wordbookStore,
            projectsModule: projectsModule,
            quicklinksModule: quicklinksModule,
            translation: translation,
            config: config,
            accessibility: accessibility,
            runProjectAction: { [weak self] action, completion in
                guard let self else { return }
                if action.hidesLauncher {
                    self.windowController.hideImmediatelyForAction()
                }
                Task {
                    let payload = (try? ModuleActionCoding.encode(action)) ?? Data()
                    let act = Action(
                        id: ActionID(module: .projects, key: "detail"),
                        title: "Project",
                        kind: .custom(payload: payload, handler: .projects)
                    )
                    let result = await self.actionExecutor.run(act, for: ResultID(module: .projects, key: "detail"))
                    await MainActor.run {
                        if !result.succeeded {
                            let message = result.userFacingMessage ?? LauncherStatusMessages.operationFailed
                            self.windowController.showStatus(message)
                        }
                        completion()
                    }
                }
            },
            runWorkbenchCapture: { [weak self] source, target in
                self?.windowController.runWorkbenchCaptureFromDetail(source: source, target: target)
            },
            runWorkspaceRow: { [weak self] action in
                self?.windowController.runWorkspaceRowActionFromDetail(action)
            }
        )
        launcherEnv.install()

        windowController.configure(
            viewModel: viewModel,
            homeCoordinator: homeCoordinator,
            actionExecutor: actionExecutor,
            config: config,
            launcherEnvironment: launcherEnv,
            onWillShow: { [weak self] in
                self?.cancelIdleModuleTeardown()
            },
            onDidHide: { [weak self] in
                self?.scheduleIdleModuleTeardown()
            },
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
                media: mediaModule,
                projects: projectsModule,
                quicklinks: quicklinksModule,
                menuItems: menuItemsModule
            ))
            await ModuleBootstrapper.registerAndWarmup(
                host: host,
                config: config,
                modules: modules,
                processMemorySampler: processMemorySampler,
                onModulesReady: { [weak self] in
                    self?.windowController.setModulesReady(true)
                },
                onMemoryPressureReady: { [weak self] in
                    self?.installMemoryPressureHandler()
                }
            )
        }
    }

    private func installMemoryPressureHandler() {
        let source = DispatchSource.makeMemoryPressureSource(eventMask: [.warning, .critical], queue: .main)
        source.setEventHandler { [weak self] in
            guard let self else { return }
            IconCache.shared.trimForMemoryPressure()
            Task {
                let pinned = await self.config.pinnedModuleIDs()
                await self.host.teardownIdleModules(
                    olderThan: .seconds(60),
                    pinned: pinned,
                    reason: .memoryPressure
                )
            }
        }
        source.resume()
        memoryPressureSource = source
    }

    private func cancelIdleModuleTeardown() {
        idleTeardownTask?.cancel()
        idleTeardownTask = nil
    }

    private func scheduleIdleModuleTeardown() {
        idleTeardownTask?.cancel()
        idleTeardownTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(30))
            guard !Task.isCancelled, let self else { return }
            let pinned = await self.config.pinnedModuleIDs()
            await self.host.teardownIdleModules(
                olderThan: .seconds(300),
                pinned: pinned,
                reason: .idle
            )
        }
    }

    private func reRegisterHotkey() {
        guard let hotkeyController else { return }
        do {
            try hotkeyController.register(HotkeyConfig.load())
            LauncherRuntimeState.hotkeyRegistered = true
            menuBarController?.markHotkeyOK()
        } catch {
            LauncherRuntimeState.hotkeyRegistered = false
            CrashLogRecording.record("hotkey.reregisterFailed")
            menuBarController?.markHotkeyFailed()
        }
    }
}
