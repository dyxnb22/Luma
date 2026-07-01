import Foundation
import LumaCore

@MainActor
final class LauncherViewModel {
    private var sequence: UInt64 = 0
    private var task: Task<Void, Never>?
    private let dispatcher: QueryDispatcher
    private let commandUsage: CommandUsageTracker
    let commandRouter: CommandRouter
    private let workbenchCommandRouter = WorkbenchCommandRouter()
    private var issuedAtBySequence: [UInt64: ContinuousClock.Instant] = [:]
    private var latencySamples: [Double] = []
    var onSnapshot: (@MainActor (ResultSnapshot) -> Void)?

    init(
        dispatcher: QueryDispatcher,
        commandRouter: CommandRouter = CommandRouter(),
        commandUsage: CommandUsageTracker = .defaultTracker()
    ) {
        self.dispatcher = dispatcher
        self.commandRouter = commandRouter
        self.commandUsage = commandUsage
    }

    func workbenchRoute(for text: String) -> WorkbenchCommandRoute {
        workbenchCommandRouter.route(raw: text)
    }

    func queryChanged(_ text: String, issuedAt: ContinuousClock.Instant) {
        task?.cancel()
        sequence &+= 1
        issuedAtBySequence[sequence] = issuedAt
        let route = commandRouter.route(raw: text)
        let parsed = commandRouter.registry.parsedCommand(for: text, route: route)
        let query = Query(raw: text, sequence: sequence, command: parsed)
        let currentSequence = sequence
        task = Task {
            try? await Task.sleep(for: .milliseconds(12))
            guard !Task.isCancelled else { return }
            switch route {
            case .empty:
                break
            case .help(let moduleID):
                if let moduleID {
                    await dispatcher.dispatchTargeted(query, moduleID: moduleID) { [weak self] snapshot in
                        await self?.deliver(
                            snapshot: snapshot,
                            route: route,
                            sequence: currentSequence
                        )
                    }
                } else {
                    let usage = await commandUsage.snapshot()
                    let items = CommandEntryResults.globalHelp(
                        registry: commandRouter.registry,
                        usage: usage
                    )
                    await deliver(
                        snapshot: ResultSnapshot(querySequence: currentSequence, items: items),
                        route: route,
                        sequence: currentSequence
                    )
                }
            case .suggestion(let suggestions):
                let items = CommandEntryResults.suggestionRows(suggestions)
                await deliver(
                    snapshot: ResultSnapshot(querySequence: currentSequence, items: items),
                    route: route,
                    sequence: currentSequence
                )
            case .unknownPrefix(let prefix, let remainder, let suggestions):
                let items = CommandEntryResults.unknownPrefixRows(
                    prefix: prefix,
                    suggestions: suggestions,
                    remainder: remainder
                )
                await deliver(
                    snapshot: ResultSnapshot(querySequence: currentSequence, items: items),
                    route: route,
                    sequence: currentSequence
                )
            case .targeted(let moduleID, _, _):
                await dispatcher.dispatchTargeted(query, moduleID: moduleID) { [weak self] snapshot in
                    await self?.deliver(
                        snapshot: snapshot,
                        route: route,
                        sequence: currentSequence
                    )
                }
            case .globalSearch:
                await dispatcher.dispatch(query) { [weak self] snapshot in
                    await self?.deliver(
                        snapshot: snapshot,
                        route: route,
                        sequence: currentSequence
                    )
                }
            }
        }
    }

    func recordExecutedCommand(for text: String) {
        let route = commandRouter.route(raw: text)
        recordUsage(for: route)
    }

    private func recordUsage(for route: CommandRoute) {
        Task {
            switch route {
            case .targeted(_, let trigger, _):
                await commandUsage.record(trigger: trigger)
            case .help(let module?):
                if let trigger = commandRouter.registry.command(forModule: module)?.primaryTrigger {
                    await commandUsage.record(trigger: trigger)
                }
            case .empty, .globalSearch, .help(nil), .suggestion, .unknownPrefix:
                break
            }
        }
    }

    private func deliver(snapshot: ResultSnapshot, route: CommandRoute, sequence: UInt64) async {
        let limited = Array(snapshot.items.prefix(8))
        let layout = CommandListLayout.build(
            items: limited,
            route: route,
            registry: commandRouter.registry
        )
        let enriched = ResultSnapshot(
            querySequence: snapshot.querySequence,
            items: limited,
            layout: layout
        )
        await MainActor.run {
            guard enriched.querySequence == self.sequence else { return }
            self.onSnapshot?(enriched)
        }
    }

    func p95LatencyMilliseconds(for sequence: UInt64) -> Double? {
        guard let issuedAt = issuedAtBySequence.removeValue(forKey: sequence) else { return nil }
        let elapsed = issuedAt.duration(to: .now)
        let ms = Double(elapsed.components.seconds) * 1000
            + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000
        latencySamples.append(ms)
        if latencySamples.count > 100 {
            latencySamples.removeFirst(latencySamples.count - 100)
        }
        return p95(of: latencySamples)
    }

    func cancel() {
        task?.cancel()
        task = nil
        issuedAtBySequence.removeAll()
        sequence &+= 1
    }

    private func p95(of samples: [Double]) -> Double {
        guard !samples.isEmpty else { return 0 }
        let sorted = samples.sorted()
        let index = Int(Double(sorted.count - 1) * 0.95)
        return sorted[min(max(0, index), sorted.count - 1)]
    }
}
