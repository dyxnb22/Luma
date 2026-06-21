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
                let translated = try await translation.translate(text)
                await pasteboard.write(translated)
            case .launchApp(let url):
                await MainActor.run {
                    let configuration = NSWorkspace.OpenConfiguration()
                    NSWorkspace.shared.openApplication(at: url, configuration: configuration)
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
}
