import Foundation
import NaturalLanguage

#if canImport(Translation)
import Translation
#endif

public enum SystemTranslationError: Error, Equatable {
    case shortcutUnavailable
    case emptyOutput
    case frameworkUnavailable
}

public struct SystemTranslationService: Sendable {
    public init() {}

    public func translate(_ text: String, targetLanguageIdentifier: String = "en") async throws -> String {
        if #available(macOS 26.0, *) {
            do {
                return try await translateWithAppleFramework(text, targetLanguageIdentifier: targetLanguageIdentifier)
            } catch {
                // Fall back to Shortcuts when languages are not installed or the session fails.
            }
        }
        return try await translateWithShortcut(text)
    }

    @available(macOS 26.0, *)
    private func translateWithAppleFramework(_ text: String, targetLanguageIdentifier: String) async throws -> String {
        #if canImport(Translation)
        let target = Locale.Language(identifier: targetLanguageIdentifier)
        let recognizer = NLLanguageRecognizer()
        recognizer.processString(text)
        let sourceIdentifier = recognizer.dominantLanguage?.rawValue ?? "en"
        let source = Locale.Language(identifier: sourceIdentifier)
        let session = TranslationSession(installedSource: source, target: target)
        let response = try await session.translate(text)
        let translated = response.targetText.trimmingCharacters(in: CharacterSet.whitespacesAndNewlines)
        guard !translated.isEmpty else { throw SystemTranslationError.emptyOutput }
        return translated
        #else
        throw SystemTranslationError.frameworkUnavailable
        #endif
    }

    public func translateWithShortcut(_ text: String, shortcutName: String = "Luma Translate") async throws -> String {
        try await withCheckedThrowingContinuation { continuation in
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/shortcuts")
            process.arguments = ["run", shortcutName, "--input-path", "-"]

            let input = Pipe()
            let output = Pipe()
            process.standardInput = input
            process.standardOutput = output
            process.standardError = Pipe()

            process.terminationHandler = { process in
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
                continuation.resume(throwing: error)
            }
        }
    }
}
