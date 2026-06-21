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
        context: ActionContext(logger: logger, metrics: metrics),
        pasteboard: pasteboard,
        accessibility: accessibility,
        translation: translation,
        usage: usage,
        resultCache: resultCache
    )
    private lazy var viewModel = LauncherViewModel(dispatcher: dispatcher)

    func start() {
        settingsWindowController = SettingsWindowController(
            config: config,
            onModulesChanged: { [weak self] enabled in
                guard let self else { return }
                Task { await self.host.applyEnabledSet(enabled) }
            }
        )
        let cards = cardLayoutStore.load(cards: FeatureCatalog.defaultCards())
        windowController.configure(cards: cards, cardLayoutStore: cardLayoutStore, viewModel: viewModel, actionExecutor: actionExecutor)
        menuBarController = MenuBarController(
            onShow: { self.windowController.show() },
            onSettings: { self.settingsWindowController?.show() }
        )
        do {
            let hotkeyController = try HotkeyController {
                self.windowController.toggle()
            }
            try hotkeyController.register(HotkeyConfig.defaultCombo)
            self.hotkeyController = hotkeyController
        } catch {
            Task {
                await LumaLogger(category: "hotkey").error("Failed to register global hotkey: \(error)")
            }
            menuBarController?.update(hotkeyOK: false)
        }

        Task {
            for module in BuiltInModules.makeAll() {
                await host.register(module)
            }
            await host.applyEnabledSet(await config.enabledModules())
            await host.warmupAll()
        }
    }
}
