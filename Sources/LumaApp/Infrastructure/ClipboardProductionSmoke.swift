import Foundation
import LumaCore
import LumaModules
import LumaServices

/// Signed-app smoke for Clipboard search/detail/copy using production wiring.
/// Triggered only when `LUMA_QA_CLIPBOARD=1`; does not run on normal launch.
@MainActor
enum ClipboardProductionSmoke {
    struct Report: Codable {
        let generatedAt: String
        let bareClipOpensDetail: Bool
        let registryHasClipboardDetail: Bool
        let historyEntryCount: Int
        let searchResultCount: Int
        let searchQuerySample: String?
        let detailListMs: Double?
        let copySucceeded: Bool
        let pasteOutcome: String
        let noMatchResultCount: Int
        let corruptBackupPresent: Bool
    }

    static func run(
        viewModel: LauncherViewModel,
        clipboardModule: ClipboardModule,
        pasteboard: PasteboardService,
        accessibility: AXService
    ) async {
        let bareOpensDetail = viewModel.commandRouter.isBareOpenDetailReturn(raw: "clip")
        let registryOK = ModuleDetailRegistry.makeDefault().hasFactory(for: ModuleIdentifier(rawValue: "luma.clipboard"))

        let recent = await clipboardModule.recentEntries(limit: 1)
        let sample = recent.first
        let searchQuery = sample.flatMap { entry -> String? in
            let text = entry.plainTextForCopy.trimmingCharacters(in: .whitespacesAndNewlines)
            guard text.count >= 4 else { return nil }
            return String(text.prefix(12))
        }

        var searchCount = 0
        if let searchQuery {
            let snapshot = await viewModelSnapshot(for: "clip \(searchQuery)", viewModel: viewModel)
            searchCount = snapshot?.items.filter { $0.id.module.rawValue == "luma.clipboard" }.count ?? 0
        }

        let listStart = ContinuousClock.now
        let detailList = await clipboardModule.filteredEntries(filter: .all, query: searchQuery ?? "", limit: 200)
        let listMs = Double(LauncherDurationRecorder.durationMilliseconds(ContinuousClock.now - listStart))

        var copyOK = false
        if let entry = sample {
            do {
                try await clipboardModule.copyEntry(id: entry.id, pasteboard: pasteboard)
                let readBack = await pasteboard.readString()
                copyOK = readBack?.isEmpty == false
            } catch {
                copyOK = false
            }
        }

        let pasteOutcome: String
        if let entry = sample {
            do {
                let outcome = try await clipboardModule.pasteEntry(id: entry.id)
                switch outcome {
                case .permissionRequired:
                    pasteOutcome = "permissionRequired"
                case .pasted:
                    pasteOutcome = "pasted"
                case .copiedOnly:
                    pasteOutcome = "copiedOnly"
                }
            } catch {
                let mapped = ActionExecutionFailureMapper.message(for: error)
                pasteOutcome = "failed:\(mapped.message ?? error.localizedDescription)"
            }
        } else {
            pasteOutcome = "skipped:no-history"
        }

        let noMatch = await viewModelSnapshot(for: "clip zznonexistentxyz", viewModel: viewModel)
        let noMatchCount = noMatch?.items.count ?? 0

        let corruptBackup: Bool = {
            guard let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first else {
                return false
            }
            guard let files = try? FileManager.default.contentsOfDirectory(
                at: base.appendingPathComponent("Luma"),
                includingPropertiesForKeys: nil
            ) else { return false }
            return files.contains {
                $0.lastPathComponent.contains(".corrupt-") && $0.lastPathComponent.hasSuffix(".bak")
            }
        }()

        let report = Report(
            generatedAt: ISO8601DateFormatter().string(from: Date()),
            bareClipOpensDetail: bareOpensDetail,
            registryHasClipboardDetail: registryOK,
            historyEntryCount: await clipboardModule.statistics().total,
            searchResultCount: searchCount,
            searchQuerySample: searchQuery,
            detailListMs: listMs,
            copySucceeded: copyOK,
            pasteOutcome: pasteOutcome,
            noMatchResultCount: noMatchCount,
            corruptBackupPresent: corruptBackup
        )
        write(report)
    }

    private static func write(_ report: Report) {
        guard let directory = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Logs/Luma", isDirectory: true) else {
            CrashLogRecording.record("clipboard.smoke.failed reason=logs-directory-unavailable")
            return
        }
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let url = directory.appendingPathComponent("clipboard-smoke.json")
            try JSONEncoder().encode(report).write(to: url, options: .atomic)
        } catch {
            CrashLogRecording.record("clipboard.smoke.failed error=\(error.localizedDescription)")
        }
    }

    private static func viewModelSnapshot(for text: String, viewModel: LauncherViewModel) async -> ResultSnapshot? {
        final class Box: @unchecked Sendable { var last: ResultSnapshot? }
        let box = Box()
        let prior = viewModel.onSnapshot
        viewModel.onSnapshot = { snapshot in box.last = snapshot }
        let route = viewModel.commandRouter.route(raw: text)
        let parsed = viewModel.commandRouter.registry.parsedCommand(for: text, route: route)
        viewModel.queryChanged(text, issuedAt: .now, route: route, parsedCommand: parsed)
        try? await Task.sleep(for: .milliseconds(500))
        viewModel.onSnapshot = prior
        return box.last
    }
}
