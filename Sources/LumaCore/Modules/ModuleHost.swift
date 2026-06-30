import Foundation

public actor ModuleHost {
    private let context: ModuleContext
    private var modules: [ModuleIdentifier: any LumaModule] = [:]
    private var enabled: Set<ModuleIdentifier> = []
    private var warmupStates: [ModuleIdentifier: WarmupState] = [:]
    private var lastUsedAt: [ModuleIdentifier: ContinuousClock.Instant] = [:]
    private var pinnedIDs: Set<ModuleIdentifier> = []

    public init(context: ModuleContext) {
        self.context = context
    }

    public func configureWarmupPolicy(pinned: Set<ModuleIdentifier>) {
        pinnedIDs = pinned
    }

    public func warmupState(for id: ModuleIdentifier) -> WarmupState {
        warmupStates[id] ?? .cold
    }

    public func register(_ module: any LumaModule) {
        let id = type(of: module).manifest.identifier
        modules[id] = module
        if type(of: module).manifest.defaultEnabled {
            enabled.insert(id)
        }
    }

    public func applyEnabledSet(_ ids: Set<ModuleIdentifier>?) async {
        guard let ids else { return }

        let removed = enabled.subtracting(ids)
        let added = ids.subtracting(enabled)

        for id in removed {
            if let module = modules[id] {
                await module.teardown()
                warmupStates[id] = .tornDown
            }
        }

        enabled = ids

        for id in added {
            if pinnedIDs.contains(id) {
                await warmupIfNeeded(id: id, reason: .startup)
            }
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

    public func warmupIfNeeded(id: ModuleIdentifier, reason: WarmupReason, budget: Duration = .seconds(1)) async {
        guard enabled.contains(id), let module = modules[id] else { return }
        let state = warmupStates[id] ?? .cold
        if state == .warm || state == .warming { return }
        warmupStates[id] = .warming
        let ctx = context
        _ = await Timeout.run(after: budget) {
            await module.warmup(ctx)
        }
        warmupStates[id] = .warm
        lastUsedAt[id] = ContinuousClock().now
    }

    public func warmupIfNeeded(ids: Set<ModuleIdentifier>, reason: WarmupReason, budget: Duration = .seconds(1)) async {
        await withTaskGroup(of: Void.self) { group in
            for id in ids {
                group.addTask {
                    await self.warmupIfNeeded(id: id, reason: reason, budget: budget)
                }
            }
        }
    }

    public func warmupRemainingEnabled(budget: Duration = .seconds(1)) async {
        let cold = enabled.filter { (warmupStates[$0] ?? .cold) != .warm }
        await warmupIfNeeded(ids: Set(cold), reason: .startup, budget: budget)
    }

    public func markUsed(id: ModuleIdentifier) {
        lastUsedAt[id] = ContinuousClock().now
    }

    public func teardownIdleModules(olderThan: Duration, pinned: Set<ModuleIdentifier>) async {
        let now = ContinuousClock().now
        for id in enabled {
            guard !pinned.contains(id) else { continue }
            guard warmupStates[id] == .warm else { continue }
            if let last = lastUsedAt[id], now - last < olderThan { continue }
            if let module = modules[id] {
                await module.teardown()
                warmupStates[id] = .tornDown
            }
        }
    }

    /// Warm only the specified module identifiers (skips those not registered or not enabled).
    public func warmup(ids: Set<ModuleIdentifier>, budget: Duration = .seconds(1)) async {
        await warmupIfNeeded(ids: ids, reason: .startup, budget: budget)
    }

    public func warmupAll(budget: Duration = .seconds(1)) async {
        await warmupRemainingEnabled(budget: budget)
    }
}
