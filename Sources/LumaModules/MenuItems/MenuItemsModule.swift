import Foundation
import LumaCore
import LumaServices

public enum MenuItemsAction: Codable, Sendable, Hashable {
    case press(bundleID: String, axPath: [Int])
}

public actor MenuItemsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .menuItems,
        displayName: "Menu Bar Search",
        capabilities: [.queryable, .providesActions, .backgroundUpdater],
        defaultEnabled: true,
        priority: 3,
        queryTimeout: .milliseconds(800)
    )

    private let service: MenuBarTreeService
    private let presser: MenuItemPresser
    private let configURL: URL
    private var disabledBundleIDs: Set<String> = []

    public init(
        service: MenuBarTreeService = .shared,
        presser: MenuItemPresser = MenuItemPresser(),
        configURL: URL = MenuItemsModule.defaultConfigURL()
    ) {
        self.service = service
        self.presser = presser
        self.configURL = configURL
    }

    public func warmup(_ context: ModuleContext) async {
        disabledBundleIDs = Self.loadConfig(url: configURL).map { Set($0.disabledBundleIDs) } ?? []
        await service.start(disabledBundleIDs: disabledBundleIDs)
    }

    public func teardown() async {
        await service.stop()
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }
        if ModuleHelp.isHelpQuery(payload) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        let records = await service.recordsForTarget(deadline: context.deadline)
        let matches = MenuItemsIndex.search(records, query: payload, limit: 8)
        if matches.isEmpty {
            let diagnostic = await menuSearchDiagnostic(records: records, query: payload, context: context)
            return ModuleResult(items: [], diagnostic: diagnostic)
        }
        return ModuleResult(items: matches.map { row(for: $0.record) })
    }

    private func menuSearchDiagnostic(
        records: [MenuItemRecord],
        query: String,
        context: QueryContext
    ) async -> ModuleDiagnostic? {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty, records.isEmpty {
            return ModuleDiagnostic(
                kind: .degraded,
                message: "No cached menu items for the frontmost app yet"
            )
        }
        if records.isEmpty {
            if await context.platform.accessibility.isTrusted() == false {
                return ModuleDiagnostic(
                    kind: .permissionRequired,
                    message: "Grant Accessibility access to read the frontmost app menu"
                )
            }
            return ModuleDiagnostic(
                kind: .degraded,
                message: "Menu cache is empty — activate a target app and try again"
            )
        }
        return ModuleDiagnostic(
            kind: .degraded,
            message: "No menu items match \"\(trimmed)\""
        )
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(MenuItemsAction.self, from: payload)
        switch decoded {
        case .press(let bundleID, let axPath):
            try await presser.press(bundleID: bundleID, axPath: axPath)
        }
    }

    private func row(for record: MenuItemRecord) -> ResultItem {
        let payload = (try? ModuleActionCoding.encode(MenuItemsAction.press(bundleID: record.bundleID, axPath: record.axPath))) ?? Data()
        let leaf = record.titlePath.last ?? "Menu Item"
        let prefix = record.titlePath.dropLast().joined(separator: " → ")
        let subtitle: String
        if let shortcut = record.shortcutDisplay, !prefix.isEmpty {
            subtitle = "\(prefix) · \(shortcut)"
        } else if let shortcut = record.shortcutDisplay {
            subtitle = shortcut
        } else {
            subtitle = prefix
        }
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "\(record.bundleID).\(record.axPath.map(String.init).joined(separator: "."))"),
            title: leaf,
            titleAttributed: AttributedString(leaf),
            subtitle: subtitle.isEmpty ? nil : subtitle,
            icon: .symbol("menubar.rectangle"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "press"),
                title: "Press Menu Item",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        for trigger in ["mb", "menu"] {
            if lower == trigger { return "" }
            if lower.hasPrefix(trigger + " ") {
                return String(trimmed.dropFirst(trigger.count + 1)).trimmingCharacters(in: .whitespacesAndNewlines)
            }
        }
        return nil
    }

    public static func defaultConfigURL() -> URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support/Luma/menu-items.json")
    }

    private static func loadConfig(url: URL) -> MenuItemsConfig? {
        if !FileManager.default.fileExists(atPath: url.path) {
            let config = MenuItemsConfig()
            try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            if let data = try? JSONEncoder().encode(config) {
                try? data.write(to: url, options: .atomic)
            }
            return config
        }
        guard let data = try? Data(contentsOf: url) else { return nil }
        return try? JSONDecoder().decode(MenuItemsConfig.self, from: data)
    }
}
