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

    @discardableResult
    public func run(_ action: Action, for item: ResultItem) async -> ActionExecutionResult {
        let result = await run(action, for: item.id)
        if case .success = result, Self.shouldPersistResultCache(for: action) {
            await resultCache.store(item)
        }
        return result
    }

    @discardableResult
    public func run(_ action: Action, for resultID: ResultID) async -> ActionExecutionResult {
        let performStart = ContinuousClock.now
        defer {
            let ms = LauncherDurationRecorder.durationMilliseconds(ContinuousClock.now - performStart)
            LauncherDurationRecorder.record(category: .actionPerform, key: action.id.module.rawValue, milliseconds: ms)
        }
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
                let customResult = await runCustomAction(module: module, action: action)
                guard customResult.succeeded else {
                    return customResult
                }
            case .noop, .openModuleDetail, .replaceQuery:
                return .success
            }
            await usage.record(resultID, at: Date())
            return .success
        } catch {
            await context.runtime.logger.error("Action failed: \(action.id.key): \(error)")
            if case .custom(_, let handler) = action.kind {
                CrashLogRecording.record("action.failed module=\(handler.rawValue) kind=custom")
            } else {
                CrashLogRecording.record("action.failed module=builtin kind=\(action.id.key)")
            }
            let mapped = ActionExecutionFailureMapper.message(for: error)
            return .failure(message: mapped.message, recoverable: mapped.recoverable)
        }
    }

    /// Navigation-only actions succeed without usage or result-cache side effects.
    private static func shouldPersistResultCache(for action: Action) -> Bool {
        switch action.kind {
        case .noop, .openModuleDetail, .replaceQuery:
            return false
        default:
            return true
        }
    }

    private func runCustomAction(module: any LumaModule, action: Action) async -> ActionExecutionResult {
        let result = await Timeout.run(after: .seconds(2)) {
            do {
                try await module.perform(action, context: self.context)
                return ActionExecutionResult.success
            } catch {
                let mapped = ActionExecutionFailureMapper.message(for: error)
                return ActionExecutionResult.failure(message: mapped.message, recoverable: mapped.recoverable)
            }
        }
        if let result {
            return result
        }
        let mapped = ActionExecutionFailureMapper.message(for: ModuleError.actionTimedOut)
        return .failure(message: mapped.message, recoverable: mapped.recoverable)
    }
}
