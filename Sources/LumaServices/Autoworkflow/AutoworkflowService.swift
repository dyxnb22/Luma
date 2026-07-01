import Foundation
import Darwin

/// Thin adapter around the cc-loop CLI, following the stable integration contract
/// defined in autoworkflow/docs/INTEGRATION.md. All subprocess calls use
/// `Process` (never shell), parse `status --json`, and never import cc-loop
/// Python modules.
public actor AutoworkflowService: AutoworkflowServiceProtocol {

    private let ccLoopPath: String
    private let commandTimeout: TimeInterval = 30

    public init(ccLoopPath: String = "cc-loop") {
        self.ccLoopPath = ccLoopPath
    }

    // MARK: - Health check

    public func healthCheck() async -> Result<Bool, AutoworkflowError> {
        await runSimple(command: "which", arguments: [ccLoopPath])
            .map { !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
    }

    public func sourceExists(at path: String) async -> Bool {
        var isDirectory: ObjCBool = false
        return FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
            && isDirectory.boolValue
    }

    // MARK: - Init

    public func initializeTask(
        goal: String,
        repo: String,
        taskID: String? = nil,
        testCommand: [String]? = nil,
        config: AutoworkflowConfig
    ) async -> Result<String, AutoworkflowError> {
        let resolvedTaskID = taskID ?? "task-\(UUID().uuidString.prefix(12))"

        var args: [String] = [
            "--state-root", config.stateRoot,
            "init",
            "--goal", goal,
            "--repo", repo,
            "--task-id", resolvedTaskID,
            "--planner", config.defaultPlanner,
            "--reviewer", config.defaultReviewer,
            "--implementer", config.defaultImplementer,
            "--claude-code-model", config.defaultModel
        ]

        if let testCmd = testCommand, !testCmd.isEmpty {
            args.append("--test-command")
            args.append(contentsOf: testCmd)
        }

        return await run(ccLoopPath, arguments: args)
            .map { output in
                // Extract task ID from "initialized task <id>" line
                if let match = output.range(of: #"initialized task (\S+)"#, options: .regularExpression) {
                    return String(output[match].split(separator: " ").last ?? resolvedTaskID[...])
                }
                return resolvedTaskID
            }
    }

    // MARK: - Start (auto --detach)

    public func startTask(taskID: String, config: AutoworkflowConfig) async -> Result<Int32, AutoworkflowError> {
        let args = AutoworkflowJSONCodec.detachAutoArguments(stateRoot: config.stateRoot, taskID: taskID)
        return await run(ccLoopPath, arguments: args)
            .flatMap { output in
                guard let pid = AutoworkflowJSONCodec.parseDetachedPID(from: output) else {
                    return .failure(.invalidJSON(output: output))
                }
                return .success(pid)
            }
    }

    // MARK: - Stop

    public func stopTask(taskID: String, config: AutoworkflowConfig) async -> Result<Void, AutoworkflowError> {
        // Read runner.pid
        let pidPath = runnerPIDPath(taskID: taskID, config: config)
        guard let pidString = try? String(contentsOfFile: pidPath, encoding: .utf8),
              let pid = Int32(pidString.trimmingCharacters(in: .whitespacesAndNewlines)) else {
            return .failure(.noTaskFound)
        }

        // Check if process exists
        if Darwin.kill(pid, 0) != 0 {
            // Process not alive — clean up stale PID file
            try? FileManager.default.removeItem(atPath: pidPath)
            return .success(())
        }

        // Verify PID still belongs to cc-loop (avoid killing a reused PID)
        guard await isAutoworkflowProcess(pid: pid) else {
            try? FileManager.default.removeItem(atPath: pidPath)
            return .success(())
        }

        // Send SIGTERM
        if Darwin.kill(pid, SIGTERM) != 0 {
            return .failure(.commandFailed(
                command: "kill \(pid)",
                exitCode: Int(errno),
                stderr: String(cString: strerror(errno))
            ))
        }

        for _ in 0..<50 {
            if Darwin.kill(pid, 0) != 0 { break }
            try? await Task.sleep(for: .milliseconds(100))
        }

        if Darwin.kill(pid, 0) == 0 {
            _ = Darwin.kill(pid, SIGKILL)
            for _ in 0..<20 {
                if Darwin.kill(pid, 0) != 0 { break }
                try? await Task.sleep(for: .milliseconds(100))
            }
        }

        try? FileManager.default.removeItem(atPath: pidPath)
        return .success(())
    }

    // MARK: - Resume

    public func resumeTask(taskID: String, config: AutoworkflowConfig) async -> Result<Int32, AutoworkflowError> {
        await startTask(taskID: taskID, config: config)
    }

    // MARK: - Status

    public func taskStatus(taskID: String, config: AutoworkflowConfig) async -> Result<AutoworkflowTaskSnapshot, AutoworkflowError> {
        let args: [String] = [
            "--state-root", config.stateRoot,
            "status",
            "--task-id", taskID,
            "--json"
        ]
        return await runLenient(ccLoopPath, arguments: args)
            .flatMap { result in
                guard let json = AutoworkflowJSONCodec.extractPayload(from: result.stdout),
                      let data = json.data(using: .utf8) else {
                    if result.exitCode != 0 {
                        return .failure(.commandFailed(
                            command: "\(ccLoopPath) status --json",
                            exitCode: Int(result.exitCode),
                            stderr: result.stderr
                        ))
                    }
                    return .failure(.invalidJSON(output: result.stdout))
                }
                do {
                    let snapshot = try AutoworkflowJSONCodec.decodeStatus(from: data)
                    if !snapshot.isRunning {
                        cleanupStaleRunnerPID(taskID: taskID, config: config)
                    }
                    return .success(snapshot)
                } catch {
                    return .failure(.invalidJSON(output: "Decode error: \(error.localizedDescription)"))
                }
            }
    }

    // MARK: - List tasks

    public func listTasks(config: AutoworkflowConfig) async -> Result<[AutoworkflowTaskItem], AutoworkflowError> {
        let args: [String] = [
            "--state-root", config.stateRoot,
            "list",
            "--json"
        ]
        return await runLenient(ccLoopPath, arguments: args)
            .flatMap { result in
                guard let json = AutoworkflowJSONCodec.extractPayload(from: result.stdout),
                      let data = json.data(using: .utf8) else {
                    if result.exitCode != 0 {
                        return .failure(.commandFailed(
                            command: "\(ccLoopPath) list --json",
                            exitCode: Int(result.exitCode),
                            stderr: result.stderr
                        ))
                    }
                    return .failure(.invalidJSON(output: result.stdout))
                }
                do {
                    let items = try AutoworkflowJSONCodec.decodeTaskList(from: data)
                    let reconciled = reconcileListItems(items, config: config)
                    return .success(reconciled)
                } catch {
                    return .failure(.invalidJSON(output: "Decode error: \(error.localizedDescription)"))
                }
            }
    }

    // MARK: - Logs

    private static let logTailMaxBytes: UInt64 = 64 * 1024

    public func readLog(taskID: String, config: AutoworkflowConfig, maxLines: Int = 200) async -> Result<String, AutoworkflowError> {
        let logPath = "\(config.stateRoot)/tasks/\(taskID)/runner.log"
        guard FileManager.default.fileExists(atPath: logPath) else {
            return .failure(.logUnavailable(path: logPath))
        }
        do {
            let content: String
            let attrs = try FileManager.default.attributesOfItem(atPath: logPath)
            let fileSize = attrs[.size] as? UInt64 ?? 0
            if fileSize <= Self.logTailMaxBytes {
                content = try String(contentsOfFile: logPath, encoding: .utf8)
            } else {
                let handle = try FileHandle(forReadingFrom: URL(fileURLWithPath: logPath))
                defer { try? handle.close() }
                try handle.seek(toOffset: fileSize - Self.logTailMaxBytes)
                let data = handle.readDataToEndOfFile()
                var chunk = String(data: data, encoding: .utf8) ?? ""
                // Drop partial first line from mid-file seek
                if let firstNewline = chunk.firstIndex(of: "\n") {
                    chunk = String(chunk[chunk.index(after: firstNewline)...])
                }
                content = chunk
            }
            let lines = content.split(separator: "\n", omittingEmptySubsequences: false)
            let tail = lines.suffix(maxLines).joined(separator: "\n")
            return .success(tail)
        } catch {
            return .failure(.logUnavailable(path: logPath))
        }
    }

    // MARK: - Doctor

    public func runDoctor(repo: String, config: AutoworkflowConfig) async -> Result<Bool, AutoworkflowError> {
        let args: [String] = [
            "--state-root", config.stateRoot,
            "doctor",
            "--repo", repo,
            "--planner", config.defaultPlanner,
            "--reviewer", config.defaultReviewer,
            "--implementer", config.defaultImplementer,
            "--json"
        ]
        return await run(ccLoopPath, arguments: args)
            .map { _ in true }
    }

    // MARK: - Runner PID hygiene

    @discardableResult
    private func cleanupStaleRunnerPID(taskID: String, config: AutoworkflowConfig) -> Bool {
        let pidPath = runnerPIDPath(taskID: taskID, config: config)
        guard FileManager.default.fileExists(atPath: pidPath) else { return false }
        guard let pidString = try? String(contentsOfFile: pidPath, encoding: .utf8),
              let pid = Int32(pidString.trimmingCharacters(in: .whitespacesAndNewlines)) else {
            try? FileManager.default.removeItem(atPath: pidPath)
            return true
        }
        if Darwin.kill(pid, 0) != 0 {
            try? FileManager.default.removeItem(atPath: pidPath)
            return true
        }
        return false
    }

    private func runnerPIDPath(taskID: String, config: AutoworkflowConfig) -> String {
        "\(config.stateRoot)/tasks/\(taskID)/runner.pid"
    }

    private func isRunnerProcessAlive(taskID: String, config: AutoworkflowConfig) -> Bool {
        let pidPath = runnerPIDPath(taskID: taskID, config: config)
        guard let pidString = try? String(contentsOfFile: pidPath, encoding: .utf8),
              let pid = Int32(pidString.trimmingCharacters(in: .whitespacesAndNewlines)) else {
            return false
        }
        return Darwin.kill(pid, 0) == 0
    }

    private func reconcileListItems(
        _ items: [AutoworkflowTaskItem],
        config: AutoworkflowConfig
    ) -> [AutoworkflowTaskItem] {
        items.map { item in
            cleanupStaleRunnerPID(taskID: item.taskID, config: config)
            guard item.status == "running", !isRunnerProcessAlive(taskID: item.taskID, config: config) else {
                return item
            }
            let resolved = AutoworkflowJSONCodec.readTaskStatusFromStateFile(
                stateRoot: config.stateRoot,
                taskID: item.taskID
            ) ?? "stopped"
            return item.withStatus(resolved)
        }
    }

    // MARK: - Process execution

    private func run(_ executable: String, arguments: [String]) async -> Result<String, AutoworkflowError> {
        let result = await runProcess(executable, arguments: arguments, timeout: commandTimeout)
        switch result {
        case .success(let output):
            if output.exitCode == 0 {
                return .success(output.stdout.trimmingCharacters(in: .whitespacesAndNewlines))
            }
            return .failure(.commandFailed(
                command: "\(executable) \(arguments.prefix(5).joined(separator: " "))...",
                exitCode: Int(output.exitCode),
                stderr: output.stderr.trimmingCharacters(in: .whitespacesAndNewlines)
            ))
        case .failure(let error):
            return .failure(error)
        }
    }

    /// Returns stdout/stderr even when the process exits non-zero.
    private func runLenient(
        _ executable: String,
        arguments: [String]
    ) async -> Result<(stdout: String, stderr: String, exitCode: Int32), AutoworkflowError> {
        let result = await runProcess(executable, arguments: arguments, timeout: commandTimeout)
        switch result {
        case .success(let output):
            return .success((
                stdout: output.stdout.trimmingCharacters(in: .whitespacesAndNewlines),
                stderr: output.stderr.trimmingCharacters(in: .whitespacesAndNewlines),
                exitCode: output.exitCode
            ))
        case .failure(let error):
            return .failure(error)
        }
    }

    private func isAutoworkflowProcess(pid: Int32) async -> Bool {
        await withCheckedContinuation { continuation in
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/bin/ps")
            process.arguments = ["-p", String(pid), "-o", "command="]

            let pipe = Pipe()
            process.standardOutput = pipe
            process.standardError = Pipe()

            do {
                try process.run()
                process.waitUntilExit()
                let data = pipe.fileHandleForReading.readDataToEndOfFile()
                let command = String(data: data, encoding: .utf8)?
                    .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
                let owned = command.contains("cc-loop")
                    || command.contains("cc_loop")
                    || command.contains("autoworkflow")
                continuation.resume(returning: owned)
            } catch {
                continuation.resume(returning: false)
            }
        }
    }

    private func runSimple(command: String, arguments: [String]) async -> Result<String, AutoworkflowError> {
        let result = await runProcess(command, arguments: arguments, timeout: 5)
        switch result {
        case .success(let output):
            return .success(output.stdout.trimmingCharacters(in: .whitespacesAndNewlines))
        case .failure(let error):
            return .failure(error)
        }
    }

    private struct ProcessOutput: Sendable {
        let stdout: String
        let stderr: String
        let exitCode: Int32
    }

    private final class OutputBuffer: @unchecked Sendable {
        private let lock = NSLock()
        private var data = Data()

        func append(_ chunk: Data) {
            lock.lock()
            data.append(chunk)
            lock.unlock()
        }

        func stringValue() -> String {
            lock.lock()
            let snapshot = data
            lock.unlock()
            return String(data: snapshot, encoding: .utf8) ?? ""
        }
    }

    private func runProcess(
        _ executable: String,
        arguments: [String],
        timeout: TimeInterval
    ) async -> Result<ProcessOutput, AutoworkflowError> {
        await withCheckedContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                let process = Process()
                process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
                process.arguments = [executable] + arguments
                process.environment = AutoworkflowCLIEnvironment.environment()
                process.qualityOfService = .userInitiated

                let stdoutPipe = Pipe()
                let stderrPipe = Pipe()
                let stdoutBuffer = OutputBuffer()
                let stderrBuffer = OutputBuffer()

                stdoutPipe.fileHandleForReading.readabilityHandler = { handle in
                    let data = handle.availableData
                    if data.isEmpty {
                        handle.readabilityHandler = nil
                    } else {
                        stdoutBuffer.append(data)
                    }
                }
                stderrPipe.fileHandleForReading.readabilityHandler = { handle in
                    let data = handle.availableData
                    if data.isEmpty {
                        handle.readabilityHandler = nil
                    } else {
                        stderrBuffer.append(data)
                    }
                }

                process.standardOutput = stdoutPipe
                process.standardError = stderrPipe

                do {
                    try process.run()
                } catch {
                    stdoutPipe.fileHandleForReading.readabilityHandler = nil
                    stderrPipe.fileHandleForReading.readabilityHandler = nil
                    continuation.resume(returning: .failure(.commandFailed(
                        command: "\(executable) \(arguments.joined(separator: " "))",
                        exitCode: -1,
                        stderr: error.localizedDescription
                    )))
                    return
                }

                let deadline = Date().addingTimeInterval(timeout)
                while process.isRunning && Date() < deadline {
                    Thread.sleep(forTimeInterval: 0.05)
                }

                if process.isRunning {
                    process.terminate()
                    Thread.sleep(forTimeInterval: 1)
                    if process.isRunning {
                        Darwin.kill(process.processIdentifier, SIGKILL)
                    }
                    stdoutPipe.fileHandleForReading.readabilityHandler = nil
                    stderrPipe.fileHandleForReading.readabilityHandler = nil
                    continuation.resume(returning: .failure(.processTimeout))
                    return
                }

                Thread.sleep(forTimeInterval: 0.05)
                stdoutPipe.fileHandleForReading.readabilityHandler = nil
                stderrPipe.fileHandleForReading.readabilityHandler = nil
                let stdoutRemainder = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
                let stderrRemainder = stderrPipe.fileHandleForReading.readDataToEndOfFile()
                if !stdoutRemainder.isEmpty {
                    stdoutBuffer.append(stdoutRemainder)
                }
                if !stderrRemainder.isEmpty {
                    stderrBuffer.append(stderrRemainder)
                }
                continuation.resume(returning: .success(ProcessOutput(
                    stdout: stdoutBuffer.stringValue(),
                    stderr: stderrBuffer.stringValue(),
                    exitCode: process.terminationStatus
                )))
            }
        }
    }
}
