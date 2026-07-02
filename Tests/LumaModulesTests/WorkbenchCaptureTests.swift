import Foundation
import Testing
import LumaCore
@testable import LumaModules

@Test func workbenchCaptureEngineSkipsDisabledModule() {
    let context = WorkbenchContext(
        clipboardPreview: "buy milk",
        enabledModuleIDs: [.workbenchNotes],
        pinnedModuleIDs: []
    )
    let result = WorkbenchCaptureEngine.capture(
        source: .clipboardText,
        target: .todoDraft,
        context: context
    )
    #expect(result == nil)
}

@Test func workbenchCaptureEngineBuildsTodoFromClipboard() {
    let context = WorkbenchContext(
        clipboardPreview: "buy milk",
        enabledModuleIDs: [.workbenchTodo],
        pinnedModuleIDs: []
    )
    let result = WorkbenchCaptureEngine.capture(
        source: .clipboardText,
        target: .todoDraft,
        context: context
    )
    #expect(result?.preview == "buy milk")
}

@Test func workbenchCaptureDraftBuilderBuildsQuicklinkFromURL() {
    let context = WorkbenchContext(
        clipboardPreview: "https://example.com/docs",
        clipboardURL: URL(string: "https://example.com/docs"),
        enabledModuleIDs: [.quicklinks],
        pinnedModuleIDs: []
    )
    let text = WorkbenchCaptureDraftBuilder.captureText(source: .clipboardURL, context: context)
    #expect(text == "https://example.com/docs")
    let draft = WorkbenchCaptureDraftBuilder.buildQuicklinkDraft(text: text!)
    #expect(draft?.trigger == "example")
}

@Test func workbenchCaptureDraftBuilderBuildsTodoTextFromSelection() {
    let context = WorkbenchContext(
        selectionText: "Follow up with design",
        enabledModuleIDs: [.todo],
        pinnedModuleIDs: []
    )
    let text = WorkbenchCaptureDraftBuilder.captureText(source: .selection, context: context)
    #expect(text == "Follow up with design")
    #expect(WorkbenchCaptureDraftBuilder.buildTodoCaptureText(text!) == "Follow up with design")
}

@Test func projectContextTodoDraftPrefixesProjectSlug() {
    let project = CurrentProjectContext(
        frontAppName: "Xcode",
        bundleID: "com.apple.dt.Xcode",
        windowTitle: "Luma.xcodeproj",
        projectLabel: "Luma",
        filename: "ContentView.swift",
        matchedProjectPath: "/tmp/Luma",
        matchedProjectName: "Luma"
    )
    let draft = ProjectContextSuggestions.todoDraft(for: project, text: "ship beta")
    #expect(draft.contains("ship beta"))
    #expect(draft.lowercased().contains("luma") || draft.hasPrefix("#"))
}

@Test func captureDoesNotWarmUnrelatedModules() async {
    let host = ModuleHost(context: CaptureWarmupTestDoubles.context())
    let counter = CaptureWarmupCountingModule()
    await host.register(counter)
    await host.applyEnabledSet([CaptureWarmupCountingModule.manifest.identifier])

    let context = WorkbenchContext(
        clipboardPreview: "hello",
        enabledModuleIDs: [.workbenchSnippets],
        pinnedModuleIDs: []
    )
    let text = WorkbenchCaptureDraftBuilder.captureText(source: .clipboardText, context: context)
    #expect(text != nil)
    _ = WorkbenchCaptureDraftBuilder.buildSnippetDraft(text: text!)
    #expect(await counter.warmupCount == 0)
}

private actor CaptureWarmupCountingModule: LumaModule {
    static let manifest = ModuleManifest(
        identifier: ModuleIdentifier(rawValue: "luma.test.capture-warmup"),
        displayName: "CaptureWarmup",
        capabilities: [.queryable],
        defaultEnabled: true,
        priority: 0,
        queryTimeout: .milliseconds(40)
    )

    private(set) var warmupCount = 0

    func warmup(_ context: ModuleContext) async { warmupCount += 1 }
    func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        .empty(for: Self.manifest.identifier)
    }
}

private struct CaptureWarmupTestDoubles {
    struct Logger: LoggingClient {
        func debug(_ message: String) async {}
        func error(_ message: String) async {}
    }

    struct Database: DatabaseClient {}
    struct Pasteboard: PasteboardClient {
        func write(_ string: String) async {}
        func writeSecure(_ string: String, clearAfterSeconds: Int) async {}
        func writeImage(data: Data, pasteboardType: String) async {}
        func writeFileURLs(_ urls: [URL]) async {}
        func readString() async -> String? { nil }
    }

    struct Accessibility: AccessibilityClient {
        func isTrusted() async -> Bool { false }
        func requestPermission() async {}
        func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {}
        func insert(text: String) async {}
    func replaceSelectedText(with text: String) async -> Bool { _ = text; return false }
        func applyWindowLayout(_ preset: String) async {}
    }

    struct FileSystem: FileSystemClient {
        func watch(root: URL, debounceMillis: Int) async -> AsyncStream<[FSChangeEvent]> {
            AsyncStream { $0.finish() }
        }
        func stopWatching(root: URL) async {}
    }

    struct Translation: TranslationClient {
        func translate(_ text: String) async throws -> TranslationOutcome {
            TranslationOutcome(text: text)
        }
    }

    struct Config: ConfigurationClient {
        func enabledModules() async -> Set<ModuleIdentifier>? { nil }
        func clipboardMaxEntries() async -> Int { 500 }
        func clipboardMaxAgeDays() async -> Int { 7 }
        func clipboardMaxEntrySizeKB() async -> Int { 100 }
        func clipboardHistoryEnabled() async -> Bool { true }
        func clipboardIgnoredBundleIDs() async -> [String] { [] }
        func clipboardPasteBehavior() async -> String { "pasteDirectly" }
        func translationTargetLanguage() async -> String { "en" }
        func secretsAutoClearSeconds() async -> Int { 10 }
        func secretsRelockTimeoutSeconds() async -> Int { 300 }
        func secretsRequireUnlockOnLaunch() async -> Bool { true }
    }

    static func context() -> ModuleContext {
        ModuleContext(
            logger: Logger(),
            metrics: NoopMetricsClient(),
            database: Database(),
            pasteboard: Pasteboard(),
            accessibility: Accessibility(),
            fileSystem: FileSystem(),
            translation: Translation(),
            config: Config()
        )
    }
}
