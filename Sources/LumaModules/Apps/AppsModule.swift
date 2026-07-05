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
    private var runningApplications: any RunningApplicationsClient = NoopRunningApplicationsClient()

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
        runningApplications = context.platform.runningApplications
        await runningApplications.startMonitoring()
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        if let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) {
            return await handlePayload(payload, context: context)
        }
        let running = await context.platform.runningApplications.runningBundleIDs()
        let matches = index.search(query.raw).map { result(for: $0, isRunning: running.contains($0.bundleID)) }
        return ModuleResult(items: matches)
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(AppsAction.self, from: payload)
        switch decoded {
        case .quit(let bundleID):
            await context.platform.workspace.terminateApplication(bundleID: bundleID)
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

    private func handlePayload(_ payload: String, context: QueryContext) async -> ModuleResult {
        let trimmed = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        if ModuleHelp.isHelpQuery(trimmed) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }
        if trimmed.lowercased() == "top" {
            return await memoryTopResult(context: context)
        }
        let searchText = trimmed
        let running = await context.platform.runningApplications.runningBundleIDs()
        let matches = index.search(searchText).map { result(for: $0, isRunning: running.contains($0.bundleID)) }
        return ModuleResult(items: matches)
    }

    private func memoryTopResult(context: QueryContext) async -> ModuleResult {
        let samples = await context.platform.processMemory.topApplications(limit: 8)
        if samples.isEmpty {
            return ModuleResult(
                items: [],
                diagnostic: ModuleDiagnostic(
                    kind: .degraded,
                    message: "Could not read memory usage for running apps"
                )
            )
        }
        return ModuleResult(items: samples.map(memoryRow))
    }

    private func memoryRow(_ sample: RunningApplicationMemory) -> ResultItem {
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

    private func secondaryActions(for app: AppRecord, isRunning: Bool) -> [Action] {
        var actions: [Action] = [
            Action(
                id: ActionID(module: Self.manifest.identifier, key: "reveal.\(app.bundleID)"),
                title: "Reveal in Finder",
                kind: .revealInFinder(app.url)
            ),
            Action(
                id: ActionID(module: Self.manifest.identifier, key: "copyPath.\(app.bundleID)"),
                title: "Copy App Path",
                kind: .copyToPasteboard(app.url.path)
            )
        ]
        if let loginURL = URL(string: "x-apple.systempreferences:com.apple.LoginItems-Settings.extension") {
            actions.append(Action(
                id: ActionID(module: Self.manifest.identifier, key: "loginItems.\(app.bundleID)"),
                title: "Open Login Items Settings",
                kind: .openURL(loginURL)
            ))
        }
        if isRunning, let quitPayload = try? ModuleActionCoding.encode(AppsAction.quit(bundleID: app.bundleID)) {
            actions.append(Action(
                id: ActionID(module: Self.manifest.identifier, key: "quit.\(app.bundleID)"),
                title: "Quit \(app.displayTitle)",
                kind: .custom(payload: quitPayload, handler: Self.manifest.identifier),
                confirmation: .requireReturn
            ))
        }
        return actions
    }

    private func result(for app: AppRecord, isRunning: Bool) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: app.bundleID)
        let title = app.displayTitle
        let subtitle: String
        if isRunning {
            subtitle = app.subtitlePath.isEmpty ? "Running" : "\(app.subtitlePath) · Running"
        } else {
            subtitle = app.subtitlePath
        }
        return ResultItem(
            id: id,
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: subtitle.isEmpty ? nil : subtitle,
            icon: .bundleID(app.bundleID),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "launch.\(app.bundleID)"),
                title: isRunning ? "Activate \(title)" : "Open \(title)",
                kind: .launchApp(app.url)
            ),
            secondaryActions: secondaryActions(for: app, isRunning: isRunning),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }
}
