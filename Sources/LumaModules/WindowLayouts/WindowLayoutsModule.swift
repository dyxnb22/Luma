import Foundation
import LumaCore

public actor WindowLayoutsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .windowLayouts,
        displayName: "Window Layouts",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 3,
        queryTimeout: .milliseconds(40)
    )

    private var cachedTrusted: Bool?
    private var trustedCheckedAt: ContinuousClock.Instant?
    private static let trustTTL: Duration = .seconds(5)

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }

        if !(await isAccessibilityTrusted(context: context)) {
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
        case .requestAccess:
            invalidateTrustCache()
            await context.platform.accessibility.requestPermission()
        case .openSettings:
            if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
                await context.platform.workspace.openURL(url)
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
        let requestPayload = (try? ModuleActionCoding.encode(WindowLayoutsAction.requestAccess)) ?? Data()
        let settingsPayload = (try? ModuleActionCoding.encode(WindowLayoutsAction.openSettings)) ?? Data()
        return PermissionResultBuilder.row(
            spec: PermissionCardSpec(
                module: Self.manifest.identifier,
                title: "Accessibility access needed",
                explanation: "Luma needs Accessibility to move and resize the focused window",
                icon: .symbol("accessibility"),
                requestAction: Action(
                    id: ActionID(module: Self.manifest.identifier, key: "request"),
                    title: "Allow Accessibility",
                    kind: .custom(payload: requestPayload, handler: Self.manifest.identifier)
                ),
                settingsAction: Action(
                    id: ActionID(module: Self.manifest.identifier, key: "grant"),
                    title: "Open System Settings",
                    kind: .custom(payload: settingsPayload, handler: Self.manifest.identifier)
                ),
                accessDenied: false
            )
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

    private func isAccessibilityTrusted(context: QueryContext) async -> Bool {
        if let cachedTrusted = cachedTrusted,
           let trustedCheckedAt,
           ContinuousClock.now - trustedCheckedAt <= Self.trustTTL {
            return cachedTrusted
        }
        let trusted = await context.platform.accessibility.isTrusted()
        cachedTrusted = trusted
        trustedCheckedAt = .now
        return trusted
    }

    private func invalidateTrustCache() {
        cachedTrusted = nil
        trustedCheckedAt = nil
    }
}
