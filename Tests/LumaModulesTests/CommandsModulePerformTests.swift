import Foundation
import LumaCore
import LumaModules
import Testing

private actor RecordingScriptRunnerClient: ScriptRunnerClient {
    private(set) var requests: [ScriptRunRequest] = []
    private var storedResult = ScriptRunResult(exitCode: 0, stdoutTail: "ok", stderrTail: "", timedOut: false)

    func setResult(_ result: ScriptRunResult) {
        storedResult = result
    }

    func run(_ request: ScriptRunRequest) async -> ScriptRunResult {
        requests.append(request)
        return storedResult
    }

    func requestCount() -> Int { requests.count }

    func lastResult() -> ScriptRunResult { storedResult }
}

@Test func commandsModulePerformSchedulesScriptRunner() async throws {
    let runner = RecordingScriptRunnerClient()
    let tempDir = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-commands-test-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: tempDir) }

    let configURL = tempDir.appendingPathComponent("commands.json")
    let config = CommandsConfig(commands: [
        ScriptCommand(
            id: "echo-test",
            title: "Echo Test",
            trigger: "echo-test",
            exec: "/bin/echo",
            args: ["hello"],
            cwd: nil,
            timeoutSec: 5
        )
    ])
    let data = try JSONEncoder().encode(config)
    try data.write(to: configURL)

    let module = CommandsModule(store: CommandsStore(fileURL: configURL))
    await module.warmup(ModuleContext(
        logger: NoopLoggingClient(),
        metrics: NoopMetricsClient(),
        database: NoopDatabaseClient(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: NoopConfigurationClient(),
        scriptRunner: runner
    ))

    let payload = try ModuleActionCoding.encode(CommandsAction.run(id: "echo-test"))
    let action = Action(
        id: ActionID(module: .commands, key: "run.echo-test"),
        title: "Run",
        kind: .custom(payload: payload, handler: CommandsModule.manifest.identifier)
    )
    let context = ActionContext(
        logger: NoopLoggingClient(),
        metrics: NoopMetricsClient(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        scriptRunner: runner
    )

    try await module.perform(action, context: context)

    for _ in 0..<50 where await runner.requestCount() == 0 {
        try await Task.sleep(for: .milliseconds(10))
    }
    #expect(await runner.requestCount() == 1)
}

@Test func commandsModulePerformSurfacesScriptFailure() async throws {
    let runner = RecordingScriptRunnerClient()
    await runner.setResult(ScriptRunResult(exitCode: 1, stdoutTail: "", stderrTail: "boom", timedOut: false))

    let tempDir = FileManager.default.temporaryDirectory
        .appendingPathComponent("luma-commands-test-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: tempDir) }

    let configURL = tempDir.appendingPathComponent("commands.json")
    let config = CommandsConfig(commands: [
        ScriptCommand(
            id: "fail-test",
            title: "Fail Test",
            trigger: "fail-test",
            exec: "/bin/false",
            args: [],
            cwd: nil,
            timeoutSec: 5
        )
    ])
    try JSONEncoder().encode(config).write(to: configURL)

    let module = CommandsModule(store: CommandsStore(fileURL: configURL))
    await module.warmup(ModuleContext(
        logger: NoopLoggingClient(),
        metrics: NoopMetricsClient(),
        database: NoopDatabaseClient(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        fileSystem: NoopFileSystemClient(),
        translation: NoopTranslationClient(),
        config: NoopConfigurationClient(),
        scriptRunner: runner
    ))

    let payload = try ModuleActionCoding.encode(CommandsAction.run(id: "fail-test"))
    let action = Action(
        id: ActionID(module: .commands, key: "run.fail-test"),
        title: "Run",
        kind: .custom(payload: payload, handler: CommandsModule.manifest.identifier)
    )
    let context = ActionContext(
        logger: NoopLoggingClient(),
        metrics: NoopMetricsClient(),
        pasteboard: NoopPasteboardClient(),
        accessibility: NoopAccessibilityClient(),
        scriptRunner: runner
    )

    try await module.perform(action, context: context)

    for _ in 0..<50 where await runner.requestCount() == 0 {
        try await Task.sleep(for: .milliseconds(10))
    }
    let result = await runner.lastResult()
    #expect(result.exitCode == 1)
    #expect(result.stderrTail == "boom")
}
private struct NoopLoggingClient: LoggingClient {
    func debug(_ message: String) async {}
    func error(_ message: String) async {}
}

private struct NoopDatabaseClient: DatabaseClient {}

private struct NoopConfigurationClient: ConfigurationClient {
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
