import Foundation
import LumaCore
import LumaServices

public actor WindowLayoutsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .windowLayouts,
        displayName: "Window Layouts",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 3,
        queryTimeout: .milliseconds(40)
    )

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }

        if !AXService.isProcessTrusted() {
            return ModuleResult(items: [permissionRow()])
        }

        let matches = WindowLayoutCatalog.matching(payload: payload)
        return ModuleResult(items: matches.map { commandRow($0) })
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(WindowLayoutsAction.self, from: payload)
        switch decoded {
        case .grantPermission:
            AXService.requestPermission()
            if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
                await context.workspace.openURL(url)
            }
        }
    }

    private func commandRow(_ command: WindowLayoutCommand) -> ResultItem {
        let key = command.preset.rawValue
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: key),
            title: command.title,
            titleAttributed: AttributedString(command.title),
            subtitle: "Window Layout",
            icon: .symbol(command.symbol),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: key),
                title: command.title,
                kind: .applyWindowLayout(key)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func permissionRow() -> ResultItem {
        let payload = (try? ModuleActionCoding.encode(WindowLayoutsAction.grantPermission)) ?? Data()
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "grant"),
            title: "Grant Accessibility Permission",
            titleAttributed: AttributedString("Grant Accessibility Permission"),
            subtitle: "Required to move the focused window",
            icon: .symbol("lock.shield"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "grant"),
                title: "Open Settings",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "layout" || lower == "win" || lower == "wl" {
            return ""
        }
        if lower.hasPrefix("layout ") {
            return String(trimmed.dropFirst("layout ".count))
        }
        if lower.hasPrefix("win ") {
            return String(trimmed.dropFirst("win ".count))
        }
        if lower.hasPrefix("wl ") {
            return String(trimmed.dropFirst("wl ".count))
        }
        return nil
    }
}
