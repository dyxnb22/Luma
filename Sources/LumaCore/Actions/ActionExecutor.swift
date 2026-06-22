import AppKit
import Foundation

public actor ActionExecutor {
    private let host: ModuleHost
    private let context: ActionContext
    private let pasteboard: any PasteboardClient
    private let accessibility: any AccessibilityClient
    private let translation: any TranslationClient
    private let usage: UsageTracking
    private let resultCache: UsageResultCache

    public init(
        host: ModuleHost,
        context: ActionContext,
        pasteboard: any PasteboardClient,
        accessibility: any AccessibilityClient,
        translation: any TranslationClient,
        usage: UsageTracking,
        resultCache: UsageResultCache = UsageResultCache.defaultCache()
    ) {
        self.host = host
        self.context = context
        self.pasteboard = pasteboard
        self.accessibility = accessibility
        self.translation = translation
        self.usage = usage
        self.resultCache = resultCache
    }

    public func run(_ action: Action, for item: ResultItem) async {
        await run(action, for: item.id)
        await resultCache.store(item)
    }

    public func run(_ action: Action, for resultID: ResultID) async {
        do {
            switch action.kind {
            case .copyToPasteboard(let value):
                await pasteboard.write(value)
            case .focusWindow(let windowID, let pid, let title):
                await accessibility.focus(windowID: windowID, pid: pid, title: title)
            case .insertText(let text):
                await accessibility.insert(text: text)
            case .applyWindowLayout(let preset):
                await accessibility.applyWindowLayout(preset)
            case .translateText(let text):
                do {
                    let outcome = try await translation.translate(text)
                    await pasteboard.write(outcome.text)
                } catch {
                    await context.logger.error("Translation action failed: \(error)")
                    throw error
                }
            case .launchApp(let url):
                do {
                    try await Self.launchAndActivateApplication(at: url)
                } catch {
                    await context.logger.error("Launch failed for \(url.path): \(error)")
                    throw error
                }
            case .openURL(let url):
                await MainActor.run {
                    _ = NSWorkspace.shared.open(url)
                }
            case .revealInFinder(let url):
                await MainActor.run {
                    NSWorkspace.shared.activateFileViewerSelecting([url])
                }
            case .custom(_, let handler):
                guard let module = await host.module(handler) else {
                    throw ModuleError.unsupportedAction(action.id)
                }
                try await module.perform(action, context: context)
            case .noop:
                break
            }
            await usage.record(resultID, at: Date())
        } catch {
            await context.logger.error("Action failed: \(action.id.key): \(error)")
        }
    }

    @MainActor
    private static func launchAndActivateApplication(at url: URL) async throws {
        if let running = runningApplication(for: url) {
            activate(running)
            return
        }

        let configuration = NSWorkspace.OpenConfiguration()
        configuration.activates = true
        configuration.addsToRecentItems = true

        let launched: NSRunningApplication? = try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<NSRunningApplication?, any Error>) in
            NSWorkspace.shared.openApplication(at: url, configuration: configuration) { app, error in
                if let error {
                    continuation.resume(throwing: error)
                    return
                }
                continuation.resume(returning: app)
            }
        }

        if let launched {
            activate(launched)
        } else if let running = runningApplication(for: url) {
            activate(running)
        }
    }

    @MainActor
    private static func runningApplication(for url: URL) -> NSRunningApplication? {
        let targetPath = url.resolvingSymlinksInPath().standardizedFileURL.path
        return NSWorkspace.shared.runningApplications.first { app in
            app.bundleURL?.resolvingSymlinksInPath().standardizedFileURL.path == targetPath
        }
    }

    @MainActor
    private static func activate(_ app: NSRunningApplication) {
        app.unhide()
        app.activate(from: NSRunningApplication.current, options: [.activateAllWindows])
    }
}
