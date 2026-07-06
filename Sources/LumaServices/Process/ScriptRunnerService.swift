@preconcurrency import Dispatch
import Foundation
import LumaCore
import UserNotifications

public actor ScriptRunnerService: ScriptRunnerClient {
    public static let maxTimeoutSeconds = 600
    public static let outputTailLimit = 4096

    private let notificationClient: any NotificationClient

    public init(notificationClient: any NotificationClient = NotificationService()) {
        self.notificationClient = notificationClient
    }

    public func run(_ request: ScriptRunRequest) async -> ScriptRunResult {
        let timeout = min(max(request.timeoutSeconds, 1), Self.maxTimeoutSeconds)
        let result = await execute(request: request, timeoutSeconds: timeout)
        CrashLogRecording.record(
            ScriptRunnerSecurityPolicy.redactedRunMetadata(
                executable: request.executable,
                exitCode: result.exitCode
            )
        )
        await postCompletionNotification(request: request, result: result)
        return result
    }

    public func runSilently(_ request: ScriptRunRequest) async -> ScriptRunResult {
        let timeout = min(max(request.timeoutSeconds, 1), Self.maxTimeoutSeconds)
        return await execute(request: request, timeoutSeconds: timeout)
    }

    private func execute(request: ScriptRunRequest, timeoutSeconds: Int) async -> ScriptRunResult {
        do {
            try ScriptRunnerSecurityPolicy.validateExecutable(request.executable)
            if let cwd = request.workingDirectory, !cwd.isEmpty {
                try ScriptRunnerSecurityPolicy.validateWorkingDirectory(cwd)
            }
        } catch {
            return ScriptRunResult(
                exitCode: -1,
                stdoutTail: "",
                stderrTail: "Script validation failed: \(error)",
                timedOut: false
            )
        }
        let environment = ScriptRunnerSecurityPolicy.sanitizedEnvironment()
        return await withCheckedContinuation { continuation in
            DispatchQueue.global(qos: .utility).async {
                let process = Process()
                do {
                    try ScriptRunnerSecurityPolicy.validateExecutable(request.executable)
                    if let cwd = request.workingDirectory, !cwd.isEmpty {
                        try ScriptRunnerSecurityPolicy.validateWorkingDirectory(cwd)
                    }
                } catch {
                    continuation.resume(returning: ScriptRunResult(
                        exitCode: -1,
                        stdoutTail: "",
                        stderrTail: "Script validation failed: \(error)",
                        timedOut: false
                    ))
                    return
                }
                process.executableURL = URL(fileURLWithPath: request.executable)
                process.arguments = request.arguments
                process.environment = environment

                if let cwd = request.workingDirectory, !cwd.isEmpty {
                    process.currentDirectoryURL = URL(fileURLWithPath: cwd, isDirectory: true)
                }

                let stdoutPipe = Pipe()
                let stderrPipe = Pipe()
                process.standardOutput = stdoutPipe
                process.standardError = stderrPipe

                var timedOut = false
                let group = DispatchGroup()
                group.enter()

                let timeoutWork = DispatchWorkItem {
                    if process.isRunning {
                        timedOut = true
                        process.terminate()
                    }
                }
                DispatchQueue.global().asyncAfter(deadline: .now() + .seconds(timeoutSeconds), execute: timeoutWork)

                process.terminationHandler = { _ in
                    timeoutWork.cancel()
                    group.leave()
                }

                do {
                    try process.run()
                } catch {
                    group.leave()
                    continuation.resume(returning: ScriptRunResult(
                        exitCode: -1,
                        stdoutTail: "",
                        stderrTail: String(error.localizedDescription.prefix(Self.outputTailLimit)),
                        timedOut: false
                    ))
                    return
                }

                group.wait()
                let stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
                let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
                let stdout = String(data: stdoutData, encoding: .utf8) ?? ""
                let stderr = String(data: stderrData, encoding: .utf8) ?? ""

                continuation.resume(returning: ScriptRunResult(
                    exitCode: process.terminationStatus,
                    stdoutTail: Self.tail(stdout),
                    stderrTail: Self.tail(stderr),
                    timedOut: timedOut
                ))
            }
        }
    }

    private func postCompletionNotification(request: ScriptRunRequest, result: ScriptRunResult) async {
        let status: String
        if result.timedOut {
            status = "Timed out after \(request.timeoutSeconds)s"
        } else {
            status = "Exit code \(result.exitCode)"
        }
        var body = status
        let detail = !result.stderrTail.isEmpty ? result.stderrTail : result.stdoutTail
        if !detail.isEmpty {
            body += "\n" + detail
        }
        await notificationClient.post(title: request.title, body: body)
    }

    private static func tail(_ text: String) -> String {
        guard text.count > outputTailLimit else { return text }
        return String(text.suffix(outputTailLimit))
    }
}
