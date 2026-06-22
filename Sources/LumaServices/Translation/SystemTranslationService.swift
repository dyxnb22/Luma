import Foundation
import LumaCore
import NaturalLanguage

#if canImport(Translation)
import Translation
#endif

public enum SystemTranslationError: Error, Equatable {
    case shortcutUnavailable
    case shortcutTimedOut
    case emptyOutput
    case frameworkUnavailable
    case languagePackRequired

    public var allowsShortcutFallback: Bool {
        switch self {
        case .frameworkUnavailable, .emptyOutput:
            return true
        case .languagePackRequired, .shortcutUnavailable, .shortcutTimedOut:
            return false
        }
    }
}

public struct SystemTranslationService: Sendable {
  private static let shortcutTimeoutSeconds: UInt64 = 10

    public init() {}

    public func translate(_ text: String, targetLanguageIdentifier: String = "en") async throws -> TranslationOutcome {
        let detectedSource = TranslationLanguageDetector.detectedLanguageCode(for: text)
        if #available(macOS 15.0, *) {
            #if canImport(Translation)
            do {
                return try await translateOnMainActor(text, targetLanguageIdentifier: targetLanguageIdentifier, detectedSource: detectedSource)
            } catch let error as SystemTranslationError {
                guard error.allowsShortcutFallback else { throw error }
            } catch {
                // Non-system Apple bridge errors may fall back to Shortcuts.
            }
            #endif
        }
        let translated = try await translateWithShortcut(text)
        return TranslationOutcome(text: translated, detectedSourceLanguageCode: detectedSource)
    }

    @available(macOS 15.0, *)
    private func translateOnMainActor(
        _ text: String,
        targetLanguageIdentifier: String,
        detectedSource: String?
    ) async throws -> TranslationOutcome {
        let translated = try await AppleTranslationHost.shared.translate(text, targetLanguageIdentifier: targetLanguageIdentifier)
        return TranslationOutcome(text: translated, detectedSourceLanguageCode: detectedSource)
    }

    public func translateWithShortcut(_ text: String, shortcutName: String = "Luma Translate") async throws -> String {
        let processBox = ShortcutProcessBox()
        return try await withTaskCancellationHandler {
            try await withThrowingTaskGroup(of: String.self) { group in
                group.addTask {
                    try await Self.runShortcutProcess(text: text, shortcutName: shortcutName, processBox: processBox)
                }
                group.addTask {
                    try await Task.sleep(for: .seconds(Self.shortcutTimeoutSeconds))
                    processBox.terminate()
                    throw SystemTranslationError.shortcutTimedOut
                }
                guard let result = try await group.next() else {
                    throw SystemTranslationError.shortcutUnavailable
                }
                group.cancelAll()
                return result
            }
        } onCancel: {
            processBox.terminate()
        }
    }

    private static func runShortcutProcess(
        text: String,
        shortcutName: String,
        processBox: ShortcutProcessBox
    ) async throws -> String {
        try await withCheckedThrowingContinuation { continuation in
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/shortcuts")
            process.arguments = ["run", shortcutName, "--input-path", "-"]

            let input = Pipe()
            let output = Pipe()
            process.standardInput = input
            process.standardOutput = output
            process.standardError = Pipe()

            processBox.process = process

            process.terminationHandler = { process in
                processBox.process = nil
                guard process.terminationStatus == 0 else {
                    continuation.resume(throwing: SystemTranslationError.shortcutUnavailable)
                    return
                }
                let data = output.fileHandleForReading.readDataToEndOfFile()
                guard let result = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines), !result.isEmpty else {
                    continuation.resume(throwing: SystemTranslationError.emptyOutput)
                    return
                }
                continuation.resume(returning: result)
            }

            do {
                try process.run()
                input.fileHandleForWriting.write(Data(text.utf8))
                try? input.fileHandleForWriting.close()
            } catch {
                processBox.process = nil
                continuation.resume(throwing: error)
            }
        }
    }
}

private final class ShortcutProcessBox: @unchecked Sendable {
    private let lock = NSLock()
    private var _process: Process?

    var process: Process? {
        get {
            lock.lock()
            defer { lock.unlock() }
            return _process
        }
        set {
            lock.lock()
            _process = newValue
            lock.unlock()
        }
    }

    func terminate() {
        lock.lock()
        let process = _process
        lock.unlock()
        if let process, process.isRunning {
            process.terminate()
        }
    }
}
