import Foundation
import LumaCore
import LumaServices

public actor AutoworkflowModule: LumaModule {
    private static let configStore = AutoworkflowConfigStore()
    public static let manifest = ModuleManifest(
        identifier: .autoworkflow,
        displayName: "Auto Workflow",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: false,
        priority: 18,
        queryTimeout: .milliseconds(100)
    )

    public nonisolated let service: any AutoworkflowServiceProtocol
    private var config: AutoworkflowConfig = .init()
    private var isWarm = false

    public init(service: any AutoworkflowServiceProtocol = AutoworkflowService()) {
        self.service = service
    }

    public func warmup(_ context: ModuleContext) async {
        config = await loadConfig(from: context.runtime.config)
        isWarm = true
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        let normalized = query.normalized

        // Help query
        if ModuleHelp.isHelpQuery(normalized) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

        // Bare trigger — show open-detail row
        if normalized.isEmpty {
            return ModuleResult(items: [openDetailRow()])
        }

        // Sub-commands: status, start, stop
        return await handleSubCommand(normalized)
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        throw ModuleError.unsupportedAction(action.id)
    }

    public func teardown() async {
        isWarm = false
    }

    // MARK: - Public API for detail view

    public func getConfig() -> AutoworkflowConfig { config }

    public func updateConfig(_ new: AutoworkflowConfig) { config = new }

    // MARK: - Private

    private func openDetailRow() -> ResultItem {
        ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "open-detail"),
            title: "Auto Workflow",
            titleAttributed: AttributedString("Auto Workflow"),
            subtitle: "Manage AI coding automation",
            icon: .symbol("gearshape.2"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open"),
                title: "Open Auto Workflow",
                kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
        )
    }

    private func handleSubCommand(_ query: String) async -> ModuleResult {
        let lower = query.lowercased().trimmingCharacters(in: .whitespaces)

        switch lower {
        case "status":
            return await statusResult()
        case "start", "new", "init":
            return ModuleResult(items: [
                openDetailRow(),
                ResultItem(
                    id: ResultID(module: Self.manifest.identifier, key: "hint-start"),
                    title: "Open Auto Workflow to configure and start",
                    titleAttributed: AttributedString("Open Auto Workflow to configure and start"),
                    subtitle: "Press Return to open the control panel",
                    icon: .symbol("info.circle"),
                    primaryAction: Action(
                        id: ActionID(module: Self.manifest.identifier, key: "open"),
                        title: "Open",
                        kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
                    ),
                    rankingHints: RankingHints(basePriority: Self.manifest.priority - 1),
                    rowKind: .informational
                )
            ])
        case "list":
            return await listResult()
        default:
            return ModuleResult(items: [openDetailRow()])
        }
    }

    private func statusResult() async -> ModuleResult {
        // Quick status check for most recent task
        let listResult = await service.listTasks(config: config)
        switch listResult {
        case .success(let tasks) where !tasks.isEmpty:
            var items: [ResultItem] = [openDetailRow()]
            for task in tasks.prefix(3) {
                items.append(ResultItem(
                    id: ResultID(module: Self.manifest.identifier, key: "task.\(task.taskID)"),
                    title: "\(statusIcon(task.status)) \(task.goal.prefix(50))",
                    titleAttributed: AttributedString("\(statusIcon(task.status)) \(task.goal.prefix(50))"),
                    subtitle: "[\(task.status)] \(task.taskID) · \(task.targetRepo.split(separator: "/").last ?? "")",
                    icon: .symbol("arrow.triangle.branch"),
                    primaryAction: Action(
                        id: ActionID(module: Self.manifest.identifier, key: "open"),
                        title: "Open Auto Workflow",
                        kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
                    ),
                    rankingHints: RankingHints(basePriority: Self.manifest.priority),
                    rowKind: .informational
                ))
            }
            return ModuleResult(items: items)
        case .success:
            return ModuleResult(items: [
                openDetailRow(),
                ResultItem(
                    id: ResultID(module: Self.manifest.identifier, key: "empty"),
                    title: "No workflows yet",
                    titleAttributed: AttributedString("No workflows yet"),
                    subtitle: "Open Auto Workflow to create one",
                    icon: .symbol("tray"),
                    primaryAction: Action(
                        id: ActionID(module: Self.manifest.identifier, key: "open"),
                        title: "Open",
                        kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
                    ),
                    rankingHints: RankingHints(basePriority: Self.manifest.priority - 1),
                    rowKind: .informational
                )
            ])
        case .failure(let err):
            return ModuleResult(items: [
                openDetailRow(),
                ResultItem(
                    id: ResultID(module: Self.manifest.identifier, key: "status-error"),
                    title: "Error checking status",
                    titleAttributed: AttributedString("Error checking status"),
                    subtitle: err.localizedDescription,
                    icon: .symbol("exclamationmark.triangle"),
                    primaryAction: Action(
                        id: ActionID(module: Self.manifest.identifier, key: "open"),
                        title: "Open",
                        kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
                    ),
                    rankingHints: RankingHints(basePriority: Self.manifest.priority - 1),
                    rowKind: .informational
                )
            ])
        }
    }

    private func listResult() async -> ModuleResult {
        let result = await service.listTasks(config: config)
        switch result {
        case .success(let tasks):
            var items: [ResultItem] = [openDetailRow()]
            for task in tasks.prefix(5) {
                items.append(ResultItem(
                    id: ResultID(module: Self.manifest.identifier, key: "task.\(task.taskID)"),
                    title: "\(statusIcon(task.status)) \(task.goal.prefix(60))",
                    titleAttributed: AttributedString("\(statusIcon(task.status)) \(task.goal.prefix(60))"),
                    subtitle: "[\(task.status)] Iter \(task.iteration) · \(task.taskID)",
                    icon: .symbol("arrow.triangle.branch"),
                    primaryAction: Action(
                        id: ActionID(module: Self.manifest.identifier, key: "open"),
                        title: "Open",
                        kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
                    ),
                    rankingHints: RankingHints(basePriority: Self.manifest.priority),
                    rowKind: .informational
                ))
            }
            return ModuleResult(items: items)
        case .failure(let err):
            return ModuleResult(items: [
                openDetailRow(),
                ResultItem(
                    id: ResultID(module: Self.manifest.identifier, key: "list-error"),
                    title: "Error listing tasks",
                    titleAttributed: AttributedString("Error listing tasks"),
                    subtitle: err.localizedDescription,
                    icon: .symbol("exclamationmark.triangle"),
                    primaryAction: Action(
                        id: ActionID(module: Self.manifest.identifier, key: "open"),
                        title: "Open",
                        kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
                    ),
                    rankingHints: RankingHints(basePriority: Self.manifest.priority - 1),
                    rowKind: .informational
                )
            ])
        }
    }

    private func statusIcon(_ status: String) -> String {
        switch status {
        case "running": return "●"
        case "done": return "✓"
        case "failed": return "✗"
        case "stopped": return "■"
        case "initialized": return "○"
        default: return "○"
        }
    }

    private func loadConfig(from configClient: any ConfigurationClient) async -> AutoworkflowConfig {
        await Self.configStore.load()
    }
}
