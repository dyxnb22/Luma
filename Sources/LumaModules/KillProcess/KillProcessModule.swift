import Foundation
import LumaCore
import LumaServices

public actor KillProcessModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .killProcess,
        displayName: "Kill Process",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 2,
        queryTimeout: .milliseconds(150)
    )

    private let service: RunningProcessService
    private var cachedRecords: [RunningProcessRecord] = []

    public init(service: RunningProcessService = RunningProcessService()) {
        self.service = service
    }

    public func warmup(_ context: ModuleContext) async {
        cachedRecords = await service.runningGUIApplications()
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }
        if ModuleHelp.isHelpQuery(payload) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        cachedRecords = await service.runningGUIApplications()
        let matches = KillProcessIndex.search(cachedRecords, query: payload, limit: 8)
        return ModuleResult(items: matches.map { row(for: $0.record) })
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(KillProcessAction.self, from: payload)
        switch decoded {
        case .quit(let pid):
            _ = await service.quit(pid: pid)
        case .forceKill(let pid):
            _ = await service.forceKill(pid: pid)
        case .relaunch(let bundleID, let pid):
            await service.relaunch(bundleID: bundleID, previousPID: pid, workspace: context.platform.workspace)
        }
        cachedRecords = await service.runningGUIApplications()
    }

    private func row(for record: RunningProcessRecord) -> ResultItem {
        let quitPayload = (try? ModuleActionCoding.encode(KillProcessAction.quit(pid: record.pid))) ?? Data()
        let forcePayload = (try? ModuleActionCoding.encode(KillProcessAction.forceKill(pid: record.pid))) ?? Data()
        let relaunchPayload = (try? ModuleActionCoding.encode(KillProcessAction.relaunch(bundleID: record.bundleID, pid: record.pid))) ?? Data()
        let guarded = KillProcessIndex.guardedBundleIDs.contains(record.bundleID)
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "\(record.pid)"),
            title: record.name,
            titleAttributed: AttributedString(record.name),
            subtitle: "\(record.bundleID) · \(KillProcessIndex.memoryDisplay(bytes: record.residentBytes))",
            icon: .bundleID(record.bundleID),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "quit.\(record.pid)"),
                title: "Quit \(record.name)",
                kind: .custom(payload: quitPayload, handler: Self.manifest.identifier),
                confirmation: guarded ? .requireReturn : .none
            ),
            secondaryActions: [
                Action(
                    id: ActionID(module: Self.manifest.identifier, key: "force.\(record.pid)"),
                    title: "Force Kill",
                    kind: .custom(payload: forcePayload, handler: Self.manifest.identifier),
                    confirmation: .requireSecondModifier
                ),
                Action(
                    id: ActionID(module: Self.manifest.identifier, key: "relaunch.\(record.pid)"),
                    title: "Relaunch",
                    kind: .custom(payload: relaunchPayload, handler: Self.manifest.identifier),
                    confirmation: guarded ? .requireReturn : .none
                )
            ],
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        for trigger in ["kill", "quit", "k"] {
            if lower == trigger { return "" }
            if lower.hasPrefix(trigger + " ") {
                return String(trimmed.dropFirst(trigger.count + 1)).trimmingCharacters(in: .whitespacesAndNewlines)
            }
        }
        return nil
    }
}
