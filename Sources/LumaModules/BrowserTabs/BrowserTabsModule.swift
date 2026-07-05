import Foundation
import LumaCore
import LumaServices

public actor BrowserTabsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .browserTabs,
        displayName: "Browser Tabs",
        capabilities: [.queryable, .providesActions, .backgroundUpdater],
        defaultEnabled: false,
        priority: 3,
        queryTimeout: .milliseconds(900)
    )

    private let service: BrowserTabsService

    public init(service: BrowserTabsService = .shared) {
        self.service = service
    }

    public func warmup(_ context: ModuleContext) async {
        _ = await service.searchableTabs()
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }
        if ModuleHelp.isHelpQuery(payload) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        let tabs = await service.searchableTabs()
        let matches = BrowserTabsIndex.search(tabs, query: payload, limit: 8)
        if !matches.isEmpty {
            return ModuleResult(items: matches.map(row))
        }
        let trimmed = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        if tabs.isEmpty {
            if let diagnostic = await service.lastDiagnostic() {
                return ModuleResult(items: [], diagnostic: diagnostic)
            }
            let message = trimmed.isEmpty
                ? "No browser tabs — open Safari or Chrome, or grant automation in System Settings → Privacy → Automation"
                : "No tabs match \"\(trimmed)\""
            return ModuleResult(
                items: [],
                diagnostic: ModuleDiagnostic(kind: .degraded, message: message)
            )
        }
        if trimmed.isEmpty {
            return ModuleResult(
                items: [],
                diagnostic: ModuleDiagnostic(
                    kind: .degraded,
                    message: "No tabs available to list"
                )
            )
        }
        return ModuleResult(
            items: [],
            diagnostic: ModuleDiagnostic(
                kind: .degraded,
                message: "No tabs match \"\(trimmed)\""
            )
        )
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(BrowserTabsAction.self, from: payload)
        switch decoded {
        case .activate(let record):
            try await service.activate(record.tabRecord)
        }
    }

    private func row(_ record: TabRecord) -> ResultItem {
        let payload = (try? ModuleActionCoding.encode(BrowserTabsAction.activate(record: CodableTabRecord(record)))) ?? Data()
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "\(record.bundleID).\(record.windowIndex).\(record.tabIndex).\(record.url)"),
            title: record.title.isEmpty ? record.url : record.title,
            titleAttributed: AttributedString(record.title.isEmpty ? record.url : record.title),
            subtitle: "\(record.browserName) · \(record.url)",
            icon: .bundleID(record.bundleID),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "activate"),
                title: "Activate Tab",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            secondaryActions: [
                Action(
                    id: ActionID(module: Self.manifest.identifier, key: "copy.\(record.url)"),
                    title: "Copy URL",
                    kind: .copyToPasteboard(record.url)
                ),
                Action(
                    id: ActionID(module: Self.manifest.identifier, key: "quicklink.\(record.url)"),
                    title: "Save as Quicklink",
                    kind: .openModuleDetail(.quicklinks, payload: quicklinkPayload(for: record.url))
                )
            ],
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func quicklinkPayload(for urlString: String) -> Data? {
        guard let url = URL(string: urlString) else { return nil }
        let draft = URLQuicklinkDraft.from(url: url)
        return try? ModuleActionCoding.encode(QuicklinksAction.prepareDraft(draft))
    }

    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        for trigger in ["tab", "tabs"] {
            if lower == trigger { return "" }
            if lower.hasPrefix(trigger + " ") {
                return String(trimmed.dropFirst(trigger.count + 1)).trimmingCharacters(in: .whitespacesAndNewlines)
            }
        }
        return nil
    }
}

public enum BrowserTabsIndex {
    public static func search(_ records: [TabRecord], query: String, limit: Int = 8) -> [TabRecord] {
        let q = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !q.isEmpty else {
            var seen = Set<TabRecord>()
            return Array(records.filter { seen.insert($0).inserted }.prefix(limit))
        }
        var seen = Set<TabRecord>()
        return records.compactMap { record -> (TabRecord, Double)? in
            let target = "\(record.title) \(record.url) \(record.browserName)".lowercased()
            let score = FuzzyMatcher.score(query: q, target: target)
            guard score > 0 else { return nil }
            return (record, score)
        }
        .sorted { $0.1 > $1.1 }
        .filter { pair in
            guard seen.insert(pair.0).inserted else { return false }
            return true
        }
        .prefix(limit)
        .map(\.0)
    }
}
