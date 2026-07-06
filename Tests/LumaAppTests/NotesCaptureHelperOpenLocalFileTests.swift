import Foundation
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices
import Testing
@testable import LumaApp

private final class CaptureHelperRecordingWorkspace: WorkspaceClient, @unchecked Sendable {
    private let lock = NSLock()
    private(set) var openURLCalls: [URL] = []
    private(set) var openLocalFileURLCalls: [URL] = []

    func launchApplication(at url: URL) async throws { _ = url }

    func openURL(_ url: URL) async throws {
        lock.withLock { openURLCalls.append(url) }
    }

    func openLocalFileURL(_ url: URL) async throws {
        lock.withLock { openLocalFileURLCalls.append(url) }
    }

    func revealInFinder(_ url: URL) async throws { _ = url }

    func terminateApplication(bundleID: String) async { _ = bundleID }

    func openApplication(bundleID: String, arguments: [String]) async {
        _ = bundleID
        _ = arguments
    }
}

@Test @MainActor func notesCaptureHelperOpenAfterCaptureUsesOpenLocalFileURL() async throws {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }

    let index = NotesTreeIndex()
    await index.setRoot(root)
    await index.warmup()

    let configURL = FileManager.default.temporaryDirectory.appendingPathComponent("notes-config-\(UUID().uuidString).json")
    defer { try? FileManager.default.removeItem(at: configURL) }
    let configStore = NotesRootConfigStore(fileURL: configURL)
    try await configStore.save(NotesRootConfig(root: root, expandedFolders: []))

    let notesModule = NotesModule(index: index, config: configStore)
    await notesModule.reloadFromConfig()

    var hideCount = 0
    let env = LauncherEnvironment(
        openModuleDetail: { _ in },
        openSettings: {},
        reloadModules: {},
        onBackFromDetail: {},
        onTranslateContentChanged: { _, _ in },
        onHideLauncher: { hideCount += 1 },
        showStatus: { _ in },
        detailReloadRouter: ModuleDetailReloadRouter(),
        clipboardModule: ClipboardModule(pasteboard: NoopPasteboardClient(), accessibility: NoopAccessibilityClient()),
        notesModule: notesModule,
        snippetsModule: SnippetsModule(),
        secretsModule: SecretsModule(),
        mediaModule: MediaModule(),
        todoModule: TodoModule(),
        wordbookStore: WordbookStore(),
        projectsModule: ProjectsModule(),
        quicklinksModule: QuicklinksModule(),
        translation: NoopTranslationClient(),
        config: ConfigurationStore(),
        accessibility: NoopAccessibilityClient(),
        runProjectAction: { _, _ in }
    )
    env.install()
    defer { LauncherEnvironment.current = nil }

    let workspace = CaptureHelperRecordingWorkspace()
    let outcome = await NotesCaptureHelper.appendToDailyNote("capture line", workspace: workspace)

    guard case .appended = outcome else {
        Issue.record("Expected appended outcome, got \(outcome)")
        return
    }
    #expect(workspace.openLocalFileURLCalls.count == 1)
    #expect(workspace.openURLCalls.isEmpty)
    #expect(hideCount == 1)
}
