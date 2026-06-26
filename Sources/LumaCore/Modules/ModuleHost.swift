import Foundation

public actor ModuleHost {
    private let context: ModuleContext
    private var modules: [ModuleIdentifier: any LumaModule] = [:]
    private var enabled: Set<ModuleIdentifier> = []

    public init(context: ModuleContext) {
        self.context = context
    }

    public func register(_ module: any LumaModule) {
        let id = type(of: module).manifest.identifier
        modules[id] = module
        if type(of: module).manifest.defaultEnabled {
            enabled.insert(id)
        }
    }

    public func applyEnabledSet(_ ids: Set<ModuleIdentifier>?) {
        if let ids {
            enabled = ids
        }
    }

    public func enabledQueryableModules() -> [any LumaModule] {
        modules.values.filter { module in
            let manifest = type(of: module).manifest
            return enabled.contains(manifest.identifier) && manifest.capabilities.contains(.queryable)
        }
    }

    public func module(_ id: ModuleIdentifier) -> (any LumaModule)? {
        modules[id]
    }

    public func makeQueryContext(deadline: ContinuousClock.Instant) -> QueryContext {
        QueryContext(
            deadline: deadline,
            platform: QueryPlatformClients(
                accessibility: context.platform.accessibility,
                processMemory: context.platform.processMemory
            )
        )
    }

    public func warmupAll(budget: Duration = .seconds(1)) async {
        let context = context
        let active = enabled.compactMap { modules[$0] }
        await withTaskGroup(of: Void.self) { group in
            for module in active {
                group.addTask {
                    _ = await Timeout.run(after: budget) {
                        await module.warmup(context)
                    }
                }
            }
        }
    }
}
