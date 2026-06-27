import Foundation
import LumaCore

public struct AppleScriptRunner: Sendable {
    public enum RunnerError: Error, Sendable {
        case failed(String)
        case timedOut
    }

    public init() {}

    public func run(_ script: String, timeout: TimeInterval = 0.8) async throws -> String {
        try await withCheckedThrowingContinuation { continuation in
            let box = ContinuationBox(continuation)
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
            process.arguments = ["-e", script]

            let output = Pipe()
            let error = Pipe()
            process.standardOutput = output
            process.standardError = error

            process.terminationHandler = { proc in
                let data = output.fileHandleForReading.readDataToEndOfFile()
                let err = error.fileHandleForReading.readDataToEndOfFile()
                let text = String(data: data, encoding: .utf8) ?? ""
                if proc.terminationStatus == 0 {
                    box.resume(.success(text))
                } else {
                    let message = String(data: err, encoding: .utf8) ?? "osascript failed"
                    box.resume(.failure(RunnerError.failed(message)))
                }
            }

            do {
                try process.run()
            } catch {
                box.resume(.failure(error))
                return
            }

            DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + timeout) {
                if process.isRunning {
                    process.terminate()
                    DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + 0.2) {
                        if process.isRunning { process.interrupt() }
                    }
                    box.resume(.failure(RunnerError.timedOut))
                }
            }
        }
    }
}

private final class ContinuationBox: @unchecked Sendable {
    private let lock = NSLock()
    private var resumed = false
    private let continuation: CheckedContinuation<String, Error>

    init(_ continuation: CheckedContinuation<String, Error>) {
        self.continuation = continuation
    }

    func resume(_ result: Result<String, Error>) {
        lock.lock()
        defer { lock.unlock() }
        guard !resumed else { return }
        resumed = true
        switch result {
        case .success(let value):
            continuation.resume(returning: value)
        case .failure(let error):
            continuation.resume(throwing: error)
        }
    }
}
