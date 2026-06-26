import Foundation
import LumaCore

public actor AppsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .apps,
        displayName: "Apps",
        capabilities: [.queryable, .providesActions, .backgroundUpdater],
        defaultEnabled: true,
        priority: 5,
        queryTimeout: .milliseconds(40)
    )

    private var index = AppIndex(apps: [])

    public init() {}

    public func warmup(_ context: ModuleContext) async {
        let cacheURL = AppIndexCache.defaultURL()
        if let cached = AppIndexCache.load(from: cacheURL) {
            index = AppIndex(apps: cached)
        }

        let scanned = AppScanner.scan()
        let fallback = [
            AppRecord(bundleID: "com.apple.finder", url: URL(fileURLWithPath: "/System/Library/CoreServices/Finder.app"), name: "Finder", localizedName: "Finder", aliases: [], pinyinFull: "", pinyinInitials: ""),
            AppRecord(bundleID: "com.apple.Safari", url: URL(fileURLWithPath: "/Applications/Safari.app"), name: "Safari", localizedName: "Safari", aliases: [], pinyinFull: "", pinyinInitials: ""),
            AppRecord(bundleID: "com.apple.systempreferences", url: URL(fileURLWithPath: "/System/Applications/System Settings.app"), name: "System Settings", localizedName: "System Settings", aliases: [], pinyinFull: "", pinyinInitials: "")
        ]
        let apps = scanned.isEmpty ? fallback : scanned
        index = AppIndex(apps: apps)
        AppIndexCache.save(apps, to: cacheURL)
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        if let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) {
            return handlePayload(payload)
        }
        let matches = index.search(query.raw).map(result)
        return ModuleResult(items: matches)
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(AppsAction.self, from: payload)
        switch decoded {
        case .quit(let bundleID):
            await context.workspace.terminateApplication(bundleID: bundleID)
        }
    }

    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "app" || lower == "apps" {
            return ""
        }
        if lower.hasPrefix("app ") {
            return String(trimmed.dropFirst(4)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("apps ") {
            return String(trimmed.dropFirst(5)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }

    private func handlePayload(_ payload: String) -> ModuleResult {
        let trimmed = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        if ModuleHelp.isHelpQuery(trimmed) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        if trimmed.lowercased() == "top" {
            return memoryTopResult()
        }
        let searchText = trimmed
        let matches = index.search(searchText).map(result)
        return ModuleResult(items: matches)
    }

    private func memoryTopResult() -> ModuleResult {
        let samples = AppMemorySampler.topApplications()
        if samples.isEmpty {
            return ModuleResult(items: [])
        }
        return ModuleResult(items: samples.map(memoryRow))
    }

    private func memoryRow(_ sample: AppMemorySample) -> ResultItem {
        let mb = String(format: "%.0f MB", sample.residentMB)
        let url = index.search(sample.name).first?.url
            ?? URL(fileURLWithPath: "/Applications")
        let quitPayload = (try? ModuleActionCoding.encode(AppsAction.quit(bundleID: sample.bundleID))) ?? Data()
        return ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "mem.\(sample.bundleID)"),
            title: sample.name,
            titleAttributed: AttributedString(sample.name),
            subtitle: mb,
            icon: .bundleID(sample.bundleID),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "launch.\(sample.bundleID)"),
                title: "Activate \(sample.name)",
                kind: .launchApp(url)
            ),
            secondaryActions: [
                Action(
                    id: ActionID(module: Self.manifest.identifier, key: "quit.\(sample.bundleID)"),
                    title: "Quit \(sample.name)",
                    kind: .custom(payload: quitPayload, handler: Self.manifest.identifier),
                    confirmation: .requireReturn
                )
            ],
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func result(for app: AppRecord) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: app.bundleID)
        let title = app.displayTitle
        let subtitle = app.subtitlePath
        return ResultItem(
            id: id,
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: subtitle,
            icon: .bundleID(app.bundleID),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "launch.\(app.bundleID)"),
                title: "Open \(title)",
                kind: .launchApp(app.url)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }
}
