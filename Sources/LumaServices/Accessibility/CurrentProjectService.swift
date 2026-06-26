import AppKit
import ApplicationServices
import Foundation
import LumaCore

public actor CurrentProjectService {
    nonisolated(unsafe) private static var sharedInstance = CurrentProjectService(matcher: NoopProjectMatcherClient())

    public static var shared: CurrentProjectService {
        sharedInstance
    }

    /// Replaces the process-wide service with a matcher wired at app launch. Call from `AppCoordinator.init` before the launcher is usable.
    @MainActor
    public static func bootstrap(matcher: any ProjectMatcherClient) {
        sharedInstance = CurrentProjectService(matcher: matcher)
    }

    private var cached: CurrentProjectContext?
    private var cachedAt: Date?
    private var refreshTask: Task<Void, Never>?
    private let matcher: any ProjectMatcherClient
    private let ttl: TimeInterval = 1.5

    public init(matcher: any ProjectMatcherClient = NoopProjectMatcherClient()) {
        self.matcher = matcher
    }

    /// Applies project-index matching to a raw IDE context. Exposed for tests.
    func applyProjectMatch(to context: CurrentProjectContext) async -> CurrentProjectContext {
        await Self.enrich(context, matcher: matcher)!
    }

    public func snapshot() async -> CurrentProjectContext? {
        await refreshIfNeeded()
        return cached
    }

    public func refreshIfNeeded() async {
        if let cachedAt, Date().timeIntervalSince(cachedAt) < ttl { return }
        if let refreshTask {
            await refreshTask.value
            return
        }

        let task = Task { [matcher] in
            let raw = await MainActor.run { Self.readCurrentProject() }
            let enriched = await Self.enrich(raw, matcher: matcher)
            self.store(enriched)
        }
        refreshTask = task
        await task.value
        refreshTask = nil
    }

    private func store(_ context: CurrentProjectContext?) {
        cached = context
        cachedAt = Date()
    }

    private static func enrich(
        _ context: CurrentProjectContext?,
        matcher: any ProjectMatcherClient
    ) async -> CurrentProjectContext? {
        guard let context else { return nil }
        if let match = await matcher.match(label: context.projectLabel) {
            return CurrentProjectContext(
                frontAppName: context.frontAppName,
                bundleID: context.bundleID,
                windowTitle: context.windowTitle,
                projectLabel: context.projectLabel,
                filename: context.filename,
                matchedProjectPath: match.path,
                matchedProjectName: match.name
            )
        }
        return CurrentProjectContext(
            frontAppName: context.frontAppName,
            bundleID: context.bundleID,
            windowTitle: context.windowTitle,
            projectLabel: context.projectLabel,
            filename: context.filename,
            matchedProjectPath: nil,
            matchedProjectName: nil
        )
    }

    @MainActor
    private static func readCurrentProject() -> CurrentProjectContext? {
        guard AXService.isProcessTrusted(),
              let app = NSWorkspace.shared.frontmostApplication,
              let bundleID = app.bundleIdentifier,
              IDEWindowTitle.isIDE(bundleID: bundleID) else {
            return nil
        }

        let appName = app.localizedName ?? ""
        guard let windowTitle = focusedWindowTitle(for: app.processIdentifier) else { return nil }

        let projectLabel = IDEWindowTitle.sidebarLabel(
            rawTitle: windowTitle,
            bundleID: bundleID,
            appName: appName
        )
        guard !projectLabel.isEmpty else { return nil }

        let filename = IDEWindowTitle.filename(
            rawTitle: windowTitle,
            bundleID: bundleID,
            appName: appName
        )

        return CurrentProjectContext(
            frontAppName: appName,
            bundleID: bundleID,
            windowTitle: windowTitle,
            projectLabel: projectLabel,
            filename: filename
        )
    }

    @MainActor
    private static func focusedWindowTitle(for pid: pid_t) -> String? {
        let appElement = AXUIElementCreateApplication(pid)
        var focused: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXFocusedWindowAttribute as CFString, &focused) == .success,
              let window = focused else { return nil }
        var titleRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(window as! AXUIElement, kAXTitleAttribute as CFString, &titleRef) == .success,
              let title = titleRef as? String else { return nil }
        let trimmed = title.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}
