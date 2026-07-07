import Foundation
import LumaCore

/// Signed-app smoke for Apps search/open using production wiring.
@MainActor
enum AppsProductionSmoke {
    struct Report: Codable {
        let generatedAt: String
        let dispatcherTargetedCount: Int
        let dispatcherGlobalCount: Int
        let viewModelTargetedCount: Int
        let viewModelGlobalCount: Int
        let appTopItemCount: Int
        let appTopDiagnostic: String?
        let noMatchCount: Int
        let launchResult: String?
    }

    static func run(
        viewModel: LauncherViewModel,
        dispatcher: QueryDispatcher,
        actionExecutor: ActionExecutor
    ) async {
        try? await Task.sleep(for: .seconds(3))

        let dispatcherTargeted = await dispatcherSnapshot(
            dispatcher: dispatcher,
            text: "app safari",
            route: .targeted(module: ModuleIdentifier(rawValue: "luma.apps"), trigger: "app", payload: "safari")
        )
        let dispatcherGlobal = await dispatcherSnapshot(
            dispatcher: dispatcher,
            text: "safari",
            route: .globalSearch("safari")
        )
        let targeted = await viewModelSnapshot(for: "app safari", viewModel: viewModel)
        try? await Task.sleep(for: .milliseconds(300))
        let global = await viewModelSnapshot(for: "safari", viewModel: viewModel)
        try? await Task.sleep(for: .milliseconds(300))
        let top = await viewModelSnapshot(for: "app top", viewModel: viewModel)
        try? await Task.sleep(for: .milliseconds(300))
        let noMatch = await viewModelSnapshot(for: "app zznonexistentxyz", viewModel: viewModel)

        var launchResult: String?
        let safariRow = dispatcherTargeted?.items.first(where: {
            $0.id.module.rawValue == "luma.apps" && $0.title.localizedCaseInsensitiveContains("Safari")
        }) ?? targeted?.items.first(where: {
            $0.id.module.rawValue == "luma.apps" && $0.title.localizedCaseInsensitiveContains("Safari")
        })
        if let safari = safariRow {
            let outcome = await actionExecutor.run(safari.primaryAction, for: safari)
            launchResult = outcome.succeeded ? "success" : (outcome.userFacingMessage ?? "failure")
        } else {
            launchResult = "skipped:no-safari-row"
        }

        let report = Report(
            generatedAt: ISO8601DateFormatter().string(from: Date()),
            dispatcherTargetedCount: dispatcherTargeted?.items.filter { $0.id.module.rawValue == "luma.apps" }.count ?? 0,
            dispatcherGlobalCount: dispatcherGlobal?.items.filter { $0.id.module.rawValue == "luma.apps" }.count ?? 0,
            viewModelTargetedCount: targeted?.items.filter { $0.id.module.rawValue == "luma.apps" }.count ?? 0,
            viewModelGlobalCount: global?.items.filter { $0.id.module.rawValue == "luma.apps" }.count ?? 0,
            appTopItemCount: top?.items.filter { $0.rowKind != .informational }.count ?? 0,
            appTopDiagnostic: top?.items.first(where: { $0.rowKind == .informational })?.title,
            noMatchCount: noMatch?.items.count ?? 0,
            launchResult: launchResult
        )

        write(report)
    }

    private static func write(_ report: Report) {
        guard let directory = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Logs/Luma", isDirectory: true) else {
            CrashLogRecording.record("apps.smoke.failed reason=logs-directory-unavailable")
            return
        }
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let url = directory.appendingPathComponent("apps-smoke.json")
            let data = try JSONEncoder().encode(report)
            try data.write(to: url, options: .atomic)
            ProductionSmokeSupport.finish(artifact: "apps-smoke.json")
        } catch {
            CrashLogRecording.record("apps.smoke.failed error=\(error.localizedDescription)")
        }
    }

    private static func dispatcherSnapshot(
        dispatcher: QueryDispatcher,
        text: String,
        route: CommandRoute
    ) async -> ResultSnapshot? {
        let query = Query(raw: text, sequence: UInt64.random(in: 1...UInt64.max))
        final class Box: @unchecked Sendable {
            var snapshot: ResultSnapshot?
        }
        let box = Box()
        switch route {
        case .targeted(let moduleID, _, _):
            await dispatcher.dispatchTargeted(query, moduleID: moduleID) { snapshot in
                box.snapshot = snapshot
            }
        case .globalSearch:
            await dispatcher.dispatch(query) { snapshot in
                box.snapshot = snapshot
            }
        default:
            break
        }
        return box.snapshot
    }

    private static func viewModelSnapshot(for text: String, viewModel: LauncherViewModel) async -> ResultSnapshot? {
        final class Box: @unchecked Sendable {
            var last: ResultSnapshot?
        }
        let box = Box()
        let prior = viewModel.onSnapshot
        viewModel.onSnapshot = { snapshot in
            box.last = snapshot
        }
        let route = viewModel.commandRouter.route(raw: text)
        let parsed = viewModel.commandRouter.registry.parsedCommand(for: text, route: route)
        viewModel.queryChanged(text, issuedAt: .now, route: route, parsedCommand: parsed)
        try? await Task.sleep(for: .milliseconds(500))
        viewModel.onSnapshot = prior
        return box.last
    }
}
