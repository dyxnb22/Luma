import Foundation

public actor ActionExecutor {
    private let host: ModuleHost
    private let context: ActionContext
    private let usage: UsageTracking
    private let resultCache: UsageResultCache

    public init(
        host: ModuleHost,
        context: ActionContext,
        usage: UsageTracking,
        resultCache: UsageResultCache = UsageResultCache.defaultCache()
    ) {
        self.host = host
        self.context = context
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
                await context.platform.pasteboard.write(value)
            case .focusWindow(let windowID, let pid, let title, let axTitle, let bounds):
                await context.platform.accessibility.focus(windowID: windowID, pid: pid, title: title, axTitle: axTitle, bounds: bounds)
            case .insertText(let text):
                await context.platform.accessibility.insert(text: text)
            case .applyWindowLayout(let preset):
                await context.platform.accessibility.applyWindowLayout(preset)
            case .translateText(let text):
                do {
                    let outcome = try await context.platform.translation.translate(text)
                    await context.platform.pasteboard.write(outcome.text)
                } catch {
                    await context.runtime.logger.error("Translation action failed: \(error)")
                    throw error
                }
            case .launchApp(let url):
                do {
                    try await context.platform.workspace.launchApplication(at: url)
                } catch {
                    await context.runtime.logger.error("Launch failed for \(url.path): \(error)")
                    throw error
                }
            case .openURL(let url):
                await context.platform.workspace.openURL(url)
            case .revealInFinder(let url):
                await context.platform.workspace.revealInFinder(url)
            case .custom(_, let handler):
                guard let module = await host.enabledModule(handler) else {
                    throw ModuleError.unsupportedAction(action.id)
                }
                try await module.perform(action, context: context)
            case .noop, .openModuleDetail, .replaceQuery:
                break
            }
            await usage.record(resultID, at: Date())
        } catch {
            await context.runtime.logger.error("Action failed: \(action.id.key): \(error)")
        }
    }
}
