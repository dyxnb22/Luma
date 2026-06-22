import AppKit
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
            AppRecord(name: "Finder", bundleID: "com.apple.finder", url: URL(fileURLWithPath: "/System/Library/CoreServices/Finder.app")),
            AppRecord(name: "Safari", bundleID: "com.apple.Safari", url: URL(fileURLWithPath: "/Applications/Safari.app")),
            AppRecord(name: "System Settings", bundleID: "com.apple.systempreferences", url: URL(fileURLWithPath: "/System/Applications/System Settings.app"))
        ]
        let apps = scanned.isEmpty ? fallback : scanned
        index = AppIndex(apps: apps)
        AppIndexCache.save(apps, to: cacheURL)
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        let normalized = query.normalized
        if normalized == "app top" || normalized == "apps top" {
            return memoryTopResult()
        }
        if normalized == "app ?" || normalized == "app help" {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
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
            await MainActor.run {
                NSWorkspace.shared.runningApplications
                    .first { $0.bundleIdentifier == bundleID }?
                    .terminate()
            }
        }
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
        return ResultItem(
            id: id,
            title: app.name,
            titleAttributed: AttributedString(app.name),
            subtitle: app.bundleID,
            icon: .bundleID(app.bundleID),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "launch.\(app.bundleID)"),
                title: "Open \(app.name)",
                kind: .launchApp(app.url)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }
}
