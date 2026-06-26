import Foundation

public actor ActionExecutor {
    private let host: ModuleHost
    private let context: ActionContext
    private let pasteboard: any PasteboardClient
    private let accessibility: any AccessibilityClient
    private let translation: any TranslationClient
    private let workspace: any WorkspaceClient
    private let usage: UsageTracking
    private let resultCache: UsageResultCache

    public init(
        host: ModuleHost,
        context: ActionContext,
        pasteboard: any PasteboardClient,
        accessibility: any AccessibilityClient,
        translation: any TranslationClient,
        workspace: any WorkspaceClient,
        usage: UsageTracking,
        resultCache: UsageResultCache = UsageResultCache.defaultCache()
    ) {
        self.host = host
        self.context = context
        self.pasteboard = pasteboard
        self.accessibility = accessibility
        self.translation = translation
        self.workspace = workspace
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
            case .focusWindow(let windowID, let pid, let title, let axTitle, let bounds):
                await accessibility.focus(windowID: windowID, pid: pid, title: title, axTitle: axTitle, bounds: bounds)
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
                    try await workspace.launchApplication(at: url)
                } catch {
                    await context.logger.error("Launch failed for \(url.path): \(error)")
                    throw error
                }
            case .openURL(let url):
                await workspace.openURL(url)
            case .revealInFinder(let url):
                await workspace.revealInFinder(url)
            case .custom(_, let handler):
                guard let module = await host.module(handler) else {
                    throw ModuleError.unsupportedAction(action.id)
                }
                try await module.perform(action, context: context)
            case .noop, .openModuleDetail, .replaceQuery:
                break
            }
            await usage.record(resultID, at: Date())
        } catch {
            await context.logger.error("Action failed: \(action.id.key): \(error)")
        }
    }
}
