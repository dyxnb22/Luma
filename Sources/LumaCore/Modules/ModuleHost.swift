import Foundation

public enum TeardownReason: String, Sendable {
    case idle
    case memoryPressure
    case disabled
    case manual
}

public struct ModuleHostDebugSnapshot: Sendable {
    public let warmModuleIDs: Set<ModuleIdentifier>
    public let reservedModuleIDs: Set<ModuleIdentifier>
    public let enabledModuleIDs: Set<ModuleIdentifier>
    public let globalSearchModuleCount: Int
    public let lastUsedAt: [ModuleIdentifier: Date]

    public init(
        warmModuleIDs: Set<ModuleIdentifier>,
        reservedModuleIDs: Set<ModuleIdentifier>,
        enabledModuleIDs: Set<ModuleIdentifier>,
        globalSearchModuleCount: Int,
        lastUsedAt: [ModuleIdentifier: Date]
    ) {
        self.warmModuleIDs = warmModuleIDs
        self.reservedModuleIDs = reservedModuleIDs
        self.enabledModuleIDs = enabledModuleIDs
        self.globalSearchModuleCount = globalSearchModuleCount
        self.lastUsedAt = lastUsedAt
    }
}

public actor ModuleHost {
    private let context: ModuleContext
    private var modules: [ModuleIdentifier: any LumaModule] = [:]
    private var enabled: Set<ModuleIdentifier> = []
    private var warmupStates: [ModuleIdentifier: WarmupState] = [:]
    private var lastUsedAt: [ModuleIdentifier: ContinuousClock.Instant] = [:]
    private var pinnedIDs: Set<ModuleIdentifier> = []
    /// When set, only these module IDs participate in global (non-targeted) query dispatch.
    private var globalSearchModuleIDs: Set<ModuleIdentifier>?
    private var reservedModuleIDs: Set<ModuleIdentifier> = []
    private var lastTeardownReason: TeardownReason?

    public init(context: ModuleContext) {
        self.context = context
    }

    public func configureWarmupPolicy(pinned: Set<ModuleIdentifier>) {
        pinnedIDs = pinned
    }

    /// Limits global search fan-out to hot-path modules. When nil, all enabled queryable modules participate.
    public func configureGlobalSearchModuleIDs(_ ids: Set<ModuleIdentifier>?) {
        globalSearchModuleIDs = ids
    }

    public func setReservedModuleIDs(_ ids: Set<ModuleIdentifier>) {
        reservedModuleIDs = ids
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
                lastTeardownReason = .disabled
            }
        }

        enabled = ids

        for id in added {
            if pinnedIDs.contains(id) {
                await warmupIfNeeded(id: id, reason: .startup)
            }
        }
    }

    public func enabledQueryableModules(forGlobalSearch: Bool = false) -> [any LumaModule] {
        modules.values.filter { module in
            let manifest = type(of: module).manifest
            guard enabled.contains(manifest.identifier),
                  manifest.capabilities.contains(.queryable) else { return false }
            if forGlobalSearch, let globalSearchModuleIDs {
                return globalSearchModuleIDs.contains(manifest.identifier)
            }
            return true
        }
    }

    public func module(_ id: ModuleIdentifier) -> (any LumaModule)? {
        modules[id]
    }

    /// Returns the module only when it is registered and enabled in Settings.
    public func enabledModule(_ id: ModuleIdentifier) -> (any LumaModule)? {
        guard enabled.contains(id), let module = modules[id] else { return nil }
        return module
    }

    public func makeQueryContext(deadline: ContinuousClock.Instant) -> QueryContext {
        QueryContext(
            deadline: deadline,
            platform: QueryPlatformClients(
                pasteboard: context.platform.pasteboard,
                accessibility: context.platform.accessibility,
                processMemory: context.platform.processMemory,
                currentProject: context.platform.currentProject,
                selectionSnapshot: context.platform.selectionSnapshot,
                runningApplications: context.platform.runningApplications
            )
        )
    }

    public func warmupIfNeeded(id: ModuleIdentifier, reason: WarmupReason, budget: Duration = .seconds(1)) async {
        guard enabled.contains(id), let module = modules[id] else { return }
        let state = warmupStates[id] ?? .cold
        if state == .warm || state == .warming { return }
        warmupStates[id] = .warming
        let ctx = context
        let completed: Void? = await Timeout.run(after: budget) {
            await module.warmup(ctx)
        }
        if completed != nil {
            warmupStates[id] = .warm
            lastUsedAt[id] = ContinuousClock().now
        } else {
            warmupStates[id] = .cold
        }
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

    public func teardownIdleModules(
        olderThan: Duration,
        pinned: Set<ModuleIdentifier>,
        reason: TeardownReason = .idle
    ) async {
        lastTeardownReason = reason
        let now = ContinuousClock().now
        for id in enabled {
            guard !pinned.contains(id) else { continue }
            guard !reservedModuleIDs.contains(id) else { continue }
            guard warmupStates[id] == .warm else { continue }
            if let last = lastUsedAt[id], now - last < olderThan { continue }
            if let module = modules[id] {
                await module.teardown()
                warmupStates[id] = .tornDown
            }
        }
    }

    public func debugSnapshot(now: ContinuousClock.Instant = ContinuousClock().now) -> ModuleHostDebugSnapshot {
        let warm = Set(warmupStates.filter { $0.value == .warm }.map(\.key))
        let globalCount = globalSearchModuleIDs?.count ?? modules.count
        let usedAt = lastUsedAt.mapValues { instant in
            let elapsed = now - instant
            let seconds = Double(elapsed.components.seconds)
                + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000
            return Date(timeIntervalSinceNow: -seconds)
        }
        return ModuleHostDebugSnapshot(
            warmModuleIDs: warm,
            reservedModuleIDs: reservedModuleIDs,
            enabledModuleIDs: enabled,
            globalSearchModuleCount: globalCount,
            lastUsedAt: usedAt
        )
    }

    public func lastTeardownReasonSnapshot() -> TeardownReason? {
        lastTeardownReason
    }

    /// Warm only the specified module identifiers (skips those not registered or not enabled).
    public func warmup(ids: Set<ModuleIdentifier>, budget: Duration = .seconds(1)) async {
        await warmupIfNeeded(ids: ids, reason: .startup, budget: budget)
    }

    public func warmupAll(budget: Duration = .seconds(1)) async {
        await warmupRemainingEnabled(budget: budget)
    }
}
