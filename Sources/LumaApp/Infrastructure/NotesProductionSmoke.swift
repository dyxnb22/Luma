import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

/// Signed-app smoke for Notes open/create/search using production wiring.
/// Triggered only when `LUMA_QA_NOTES=1`; uses a temp root and restores config afterward.
@MainActor
enum NotesProductionSmoke {
    struct Report: Codable {
        let generatedAt: String
        let bareOpensDetail: Bool
        let registryHasNotesDetail: Bool
        let tempRootPath: String
        let createQueryItemCount: Int
        let createdFileExists: Bool
        let searchResultCount: Int
        let openPathRejectedOutsideRoot: Bool
        let mindMapReloadSucceeded: Bool
        let noMatchResultCount: Int
        let configRestored: Bool
        let diskRootMatchesBackup: Bool
    }

    static func run(
        viewModel: LauncherViewModel,
        notesModule: NotesModule,
        workspace: WorkspaceService
    ) async {
        let configStore = NotesRootConfigStore()
        let originalConfig = await configStore.load()
        var configRestored = false

        let tempRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("luma-notes-smoke-\(UUID().uuidString)", isDirectory: true)
        var smokeConfig = NotesRootConfig.empty
        smokeConfig.root = tempRoot

        var createQueryItemCount = 0
        var createdFileExists = false
        var searchResultCount = 0
        var openPathRejectedOutsideRoot = false
        var mindMapReloadSucceeded = false
        var noMatchResultCount = 0

        do {
            try FileManager.default.createDirectory(at: tempRoot, withIntermediateDirectories: true)
            try await notesModule.saveConfig(smokeConfig)
            await notesModule.reloadFromConfig()

            let noteTitle = "smoke-\(UUID().uuidString.prefix(8))"
            let createContext = QueryContext(deadline: .now + .seconds(2))
            let createResult = await notesModule.handle(
                Query(raw: "n new \(noteTitle)", sequence: 1),
                context: createContext
            )
            createQueryItemCount = createResult.items.count

            if let item = createResult.items.first {
                let context = ActionContext(
                    logger: LumaLogger(),
                    metrics: LumaMetrics(),
                    pasteboard: NoopPasteboardClient(),
                    accessibility: NoopAccessibilityClient(),
                    translation: NoopTranslationClient(),
                    workspace: workspace
                )
                try await notesModule.perform(item.primaryAction, context: context)
                createdFileExists = Self.inboxNoteExists(title: String(noteTitle), root: tempRoot)
            }

            let searchResult = await notesModule.handle(
                Query(raw: "n \(noteTitle)", sequence: 2),
                context: createContext
            )
            searchResultCount = searchResult.items.count

            let noMatch = await notesModule.handle(
                Query(raw: "n zznonexistentnotetitle", sequence: 3),
                context: createContext
            )
            noMatchResultCount = noMatch.items.count

            do {
                let outside = tempRoot.deletingLastPathComponent().appendingPathComponent("outside.md").path
                let payload = try ModuleActionCoding.encode(NotesAction.open(path: outside))
                let action = Action(
                    id: ActionID(module: .notes, key: "open-outside"),
                    title: "Open",
                    kind: .custom(payload: payload, handler: .notes)
                )
                let context = ActionContext(
                    logger: LumaLogger(),
                    metrics: LumaMetrics(),
                    pasteboard: NoopPasteboardClient(),
                    accessibility: NoopAccessibilityClient(),
                    translation: NoopTranslationClient(),
                    workspace: workspace
                )
                try await notesModule.perform(action, context: context)
            } catch NotesActionError.pathOutsideRoot {
                openPathRejectedOutsideRoot = true
            }

            if let snapshot = await notesModule.snapshot() {
                let mindMap = NotesMindMapView()
                mindMap.reload(root: snapshot)
                mindMap.expandAll()
                mindMapReloadSucceeded = true
            }

            try await notesModule.saveConfig(originalConfig)
            await notesModule.reloadFromConfig()
            configRestored = true
            try? FileManager.default.removeItem(at: tempRoot)
        } catch {
            CrashLogRecording.record("notes.smoke.failed error=\(error.localizedDescription)")
            try? FileManager.default.removeItem(at: tempRoot)
        }

        if !configRestored {
            try? await notesModule.saveConfig(originalConfig)
            await notesModule.reloadFromConfig()
            configRestored = true
        }

        let restoredFromDisk = await configStore.load()
        let diskRootMatchesBackup = restoredFromDisk.root == originalConfig.root

        let report = Report(
            generatedAt: ISO8601DateFormatter().string(from: Date()),
            bareOpensDetail: viewModel.commandRouter.isBareOpenDetailReturn(raw: "n"),
            registryHasNotesDetail: ModuleDetailRegistry.makeDefault()
                .hasFactory(for: .notes),
            tempRootPath: tempRoot.path,
            createQueryItemCount: createQueryItemCount,
            createdFileExists: createdFileExists,
            searchResultCount: searchResultCount,
            openPathRejectedOutsideRoot: openPathRejectedOutsideRoot,
            mindMapReloadSucceeded: mindMapReloadSucceeded,
            noMatchResultCount: noMatchResultCount,
            configRestored: configRestored && diskRootMatchesBackup,
            diskRootMatchesBackup: diskRootMatchesBackup
        )
        write(report)
    }

    private static func inboxNoteExists(title: String, root: URL) -> Bool {
        let inbox = root.appendingPathComponent("Inbox", isDirectory: true)
        let slug = title.lowercased().replacingOccurrences(of: " ", with: "-")
        let candidates = [
            inbox.appendingPathComponent("\(title).md"),
            inbox.appendingPathComponent("\(slug).md")
        ]
        return candidates.contains { FileManager.default.fileExists(atPath: $0.path) }
            || (try? FileManager.default.contentsOfDirectory(at: inbox, includingPropertiesForKeys: nil))?
                .contains { $0.lastPathComponent.localizedCaseInsensitiveContains(title) } == true
    }

    private static func write(_ report: Report) {
        guard let directory = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Logs/Luma", isDirectory: true) else {
            CrashLogRecording.record("notes.smoke.failed reason=logs-directory-unavailable")
            return
        }
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let url = directory.appendingPathComponent("notes-smoke.json")
            try JSONEncoder().encode(report).write(to: url, options: .atomic)
            ProductionSmokeSupport.finish(artifact: "notes-smoke.json")
        } catch {
            CrashLogRecording.record("notes.smoke.failed error=\(error.localizedDescription)")
        }
    }
}
