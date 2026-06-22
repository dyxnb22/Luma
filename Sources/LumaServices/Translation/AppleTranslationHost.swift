import AppKit
import NaturalLanguage
import Observation
import SwiftUI

#if canImport(Translation)
@preconcurrency import Translation

@available(macOS 15.0, *)
@MainActor
@Observable
final class TranslationHostModel {
    var configuration: TranslationSession.Configuration?
    private(set) var pendingText = ""
    private var continuation: CheckedContinuation<String, Error>?

    func translate(text: String, source: Locale.Language?, target: Locale.Language) async throws -> String {
        cancelPendingRequest()
        return try await withCheckedThrowingContinuation { continuation in
            self.continuation = continuation
            self.pendingText = text
            let next = TranslationSession.Configuration(source: source, target: target)
            if configuration != nil {
                configuration = next
                configuration?.invalidate()
            } else {
                configuration = next
            }
        }
    }

    private func cancelPendingRequest() {
        guard let continuation else { return }
        self.continuation = nil
        pendingText = ""
        continuation.resume(throwing: CancellationError())
    }

    fileprivate func finish(_ result: Result<String, Error>) {
        continuation?.resume(with: result)
        continuation = nil
        pendingText = ""
    }
}

@available(macOS 15.0, *)
@MainActor
private struct TranslationHostRootView: View {
    @Bindable var model: TranslationHostModel

    var body: some View {
        Color.clear
            .frame(width: 1, height: 1)
            .translationTask(model.configuration) { session in
                let text = model.pendingText
                guard !text.isEmpty else {
                    model.finish(.failure(SystemTranslationError.emptyOutput))
                    return
                }
                do {
                    try await session.prepareTranslation()
                    let response = try await session.translate(text)
                    model.finish(.success(response.targetText))
                } catch {
                    model.finish(.failure(error))
                }
            }
    }
}

@available(macOS 15.0, *)
@MainActor
private final class TranslationHostRuntime {
    let model = TranslationHostModel()

    lazy var hostingView: NSHostingView<TranslationHostRootView> = {
        let view = NSHostingView(rootView: TranslationHostRootView(model: model))
        view.frame = NSRect(x: 0, y: 0, width: 1, height: 1)
        return view
    }()

    func translate(text: String, targetLanguageIdentifier: String) async throws -> String {
        let source = TranslationLanguageDetector.detectSourceLanguage(for: text)
        let target = Locale.Language(identifier: targetLanguageIdentifier)

        if let installed = try await translateWithInstalledSession(text: text, source: source, target: target) {
            return installed
        }

        let result = try await model.translate(text: text, source: source, target: target)
        let trimmed = result.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw SystemTranslationError.emptyOutput }
        return trimmed
    }

    private func translateWithInstalledSession(
        text: String,
        source: Locale.Language?,
        target: Locale.Language
    ) async throws -> String? {
        guard #available(macOS 26.0, *) else { return nil }
        let resolvedSource = source ?? Locale.Language(identifier: "en")
        let availability = LanguageAvailability()
        let status = await availability.status(from: resolvedSource, to: target)
        guard status == .installed else { return nil }

        let session = TranslationSession(installedSource: resolvedSource, target: target)
        let response = try await session.translate(text)
        let translated = response.targetText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !translated.isEmpty else { throw SystemTranslationError.emptyOutput }
        return translated
    }
}
#endif

@MainActor
public final class AppleTranslationHost {
    public static let shared = AppleTranslationHost()

    #if canImport(Translation)
    private var runtime: Any?
    #endif

    private let fallbackHostingView = NSHostingView(rootView: AnyView(Color.clear.frame(width: 1, height: 1)))

    private init() {
        fallbackHostingView.frame = NSRect(x: 0, y: 0, width: 1, height: 1)
    }

    private var activeHostingView: NSView {
        #if canImport(Translation)
        if #available(macOS 15.0, *), let runtime = ensureRuntime() {
            return runtime.hostingView
        }
        #endif
        return fallbackHostingView
    }

    public func attach(to parent: NSView) {
        let view = activeHostingView
        guard view.superview == nil else { return }
        view.translatesAutoresizingMaskIntoConstraints = false
        parent.addSubview(view)
        NSLayoutConstraint.activate([
            view.leadingAnchor.constraint(equalTo: parent.leadingAnchor),
            view.bottomAnchor.constraint(equalTo: parent.bottomAnchor),
            view.widthAnchor.constraint(equalToConstant: 1),
            view.heightAnchor.constraint(equalToConstant: 1)
        ])
    }

    public func translate(_ text: String, targetLanguageIdentifier: String) async throws -> String {
        #if canImport(Translation)
        if #available(macOS 15.0, *), let runtime = ensureRuntime() {
            return try await runtime.translate(text: text, targetLanguageIdentifier: targetLanguageIdentifier)
        }
        #endif
        throw SystemTranslationError.frameworkUnavailable
    }

    #if canImport(Translation)
    @available(macOS 15.0, *)
    private func ensureRuntime() -> TranslationHostRuntime? {
        if runtime == nil {
            runtime = TranslationHostRuntime()
        }
        return runtime as? TranslationHostRuntime
    }
    #endif
}
