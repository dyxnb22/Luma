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
    private let cardLayoutStore = CardLayoutStore.defaultStore()
    private var settingsWindowController: SettingsWindowController?
    private let logger = LumaLogger()
    private let metrics = LumaMetrics()
    private let database = ApplicationSupportPaths()
    private let pasteboard = PasteboardService()
    private let accessibility = AXService()
    private let fileSystem = FSEventsService()
    private let config = ConfigurationStore()
    private lazy var translation = TranslationService(config: config)
    private lazy var context = ModuleContext(
        logger: logger,
        metrics: metrics,
        database: database,
        pasteboard: pasteboard,
        accessibility: accessibility,
        fileSystem: fileSystem,
        translation: translation,
        config: config
    )
    private lazy var host = ModuleHost(context: context)
    private let usage = PersistentUsageTracker.defaultTracker()
    private let resultCache = UsageResultCache.defaultCache()
    private lazy var dispatcher = QueryDispatcher(host: host, usage: usage, resultCache: resultCache, metrics: metrics)
    private lazy var actionExecutor = ActionExecutor(
        host: host,
        context: ActionContext(logger: logger, metrics: metrics, pasteboard: pasteboard, accessibility: accessibility),
        pasteboard: pasteboard,
        accessibility: accessibility,
        translation: translation,
        usage: usage,
        resultCache: resultCache
    )
    private lazy var viewModel = LauncherViewModel(dispatcher: dispatcher)
    private let appActivationTracker = AppActivationTracker.defaultTracker()
    private let clipboardModule = ClipboardModule()
    private let todoModule = TodoModule()
    private let secretsModule = SecretsModule()
    private let mediaModule = MediaModule()
    private var activationObserver: NSObjectProtocol?
    private var terminationObserver: NSObjectProtocol?

    func start() {
        settingsWindowController = SettingsWindowController(
            config: config,
            usage: usage,
            onModulesChanged: { [weak self] enabled in
                guard let self else { return }
                Task { await self.host.applyEnabledSet(enabled) }
            },
            onClipboardSettingsChanged: { [weak self] entries, days, kb in
                guard let self else { return }
                Task { await self.clipboardModule.applyRetentionSettings(maxEntries: entries, maxAgeDays: days, maxEntrySizeKB: kb) }
            },
            onSecretsSettingsChanged: { [weak self] autoClear, relock in
                guard let self else { return }
                Task { await self.secretsModule.applySettings(autoClearSeconds: autoClear, relockTimeoutSeconds: relock) }
            },
            onLatencyHUDChanged: { [weak self] enabled in
                self?.windowController.setLatencyHUDEnabled(enabled)
            }
        )
        let cards = cardLayoutStore.load(cards: FeatureCatalog.dashboardCoreCards())
        windowController.configure(
            cards: cards,
            viewModel: viewModel,
            actionExecutor: actionExecutor,
            appActivationTracker: appActivationTracker,
            config: config
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
            onSettings: { self.settingsWindowController?.show() }
        )
        LauncherBridge.onSecretsLockStateChanged = { [weak self] locked in
            self?.menuBarController?.setSecretsLockState(locked: locked)
        }
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
        do {
            _ = try WordbookMigrator.migrateIfNeeded()
        } catch {
            Task { await LumaLogger(category: "wordbook").error("Wordbook migration failed: \(error)") }
        }

        let wordbookModule = WordbookModule()
        let snippetsModule = SnippetsModule()
        let wordbookReviewPanel = WordbookReviewPanel()

        ModuleDetailRegistry.clipboardModule = clipboardModule
        ModuleDetailRegistry.notesModule = notesModule
        ModuleDetailRegistry.snippetsModule = snippetsModule
        ModuleDetailRegistry.secretsModule = secretsModule
        ModuleDetailRegistry.mediaModule = mediaModule
        ModuleDetailRegistry.todoModule = todoModule
        ModuleDetailRegistry.translation = translation
        ModuleDetailRegistry.config = config
        LauncherBridge.onBackFromDetail = { [weak self] in
            self?.windowController.closeDetailIfShowing()
        }
        LauncherBridge.onOpenSettings = { [weak self] in
            self?.settingsWindowController?.show()
        }
        LauncherBridge.onTranslateContentChanged = { [weak self] source, output in
            guard let self else { return }
            Task {
                await self.config.setLauncherTranslateSourceText(source)
                await self.config.setLauncherTranslateOutputText(output)
            }
        }
        LauncherBridge.openWordbookReview = { [weak self] in
            self?.windowController.hide()
            wordbookReviewPanel.startSession()
        }
        LauncherBridge.openMediaDetail = { [weak self] in
            self?.windowController.openModuleDetail(for: .media)
        }

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
            await refreshMenuBarDueCounts(wordbookModule: wordbookModule, todoModule: todoModule)
        }
    }

    private func refreshMenuBarDueCounts(wordbookModule: WordbookModule, todoModule: TodoModule) async {
        let wordbookDue = (try? await wordbookModule.storeDueTodayCount()) ?? 0
        let todoDue = (try? await todoModule.todayDueCount()) ?? 0
        menuBarController?.setDueCounts(wordbook: wordbookDue, todo: todoDue)
    }
}
