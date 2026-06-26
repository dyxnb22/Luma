import Foundation

public struct ScriptRunRequest: Sendable {
    public let title: String
    public let executable: String
    public let arguments: [String]
    public let workingDirectory: String?
    public let timeoutSeconds: Int

    public init(
        title: String,
        executable: String,
        arguments: [String],
        workingDirectory: String?,
        timeoutSeconds: Int
    ) {
        self.title = title
        self.executable = executable
        self.arguments = arguments
        self.workingDirectory = workingDirectory
        self.timeoutSeconds = timeoutSeconds
    }
}

public struct ScriptRunResult: Sendable {
    public let exitCode: Int32
    public let stdoutTail: String
    public let stderrTail: String
    public let timedOut: Bool

    public init(exitCode: Int32, stdoutTail: String, stderrTail: String, timedOut: Bool) {
        self.exitCode = exitCode
        self.stdoutTail = stdoutTail
        self.stderrTail = stderrTail
        self.timedOut = timedOut
    }
}

public protocol ScriptRunnerClient: Sendable {
    func run(_ request: ScriptRunRequest) async -> ScriptRunResult
}

public struct NoopScriptRunnerClient: ScriptRunnerClient {
    public init() {}

    public func run(_ request: ScriptRunRequest) async -> ScriptRunResult {
        ScriptRunResult(exitCode: -1, stdoutTail: "", stderrTail: "Script runner unavailable", timedOut: false)
    }
}

public protocol NotificationClient: Sendable {
    func post(title: String, body: String) async
}

public struct NoopNotificationClient: NotificationClient {
    public init() {}

    public func post(title: String, body: String) async {
        _ = title
        _ = body
    }
}
