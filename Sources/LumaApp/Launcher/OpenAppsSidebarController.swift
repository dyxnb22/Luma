import AppKit
import LumaCore
import LumaModules
import LumaServices

@MainActor
final class OpenAppsSidebarController {
    private let stack: NSStackView
    private let appActivationTracker: AppActivationTracker
    private let actionExecutor: ActionExecutor
    private let onActionDismiss: () -> Void
    private var rowByKey: [String: NSView] = [:]
    private var showsAllSidebarRows = false
    private static let collapsedRowLimit = 14

    init(
        stack: NSStackView,
        appActivationTracker: AppActivationTracker,
        actionExecutor: ActionExecutor,
        onActionDismiss: @escaping () -> Void
    ) {
        self.stack = stack
        self.appActivationTracker = appActivationTracker
        self.actionExecutor = actionExecutor
        self.onActionDismiss = onActionDismiss
    }

    func resetExpanded() {
        showsAllSidebarRows = false
    }

    func refresh() async {
        let selfPID = ProcessInfo.processInfo.processIdentifier
        let runningApps = NSWorkspace.shared.runningApplications.filter { app in
            app.activationPolicy == .regular
                && app.bundleIdentifier != nil
                && app.processIdentifier != selfPID
        }
        // Deduplicate bundleIDs: multiple processes with the same bundleID (e.g. debug + release)
        // would otherwise produce duplicate rows via rankedBundleIDs.
        var seenIDs = Set<String>()
        let bundleIDs = runningApps.compactMap(\.bundleIdentifier).filter { seenIDs.insert($0).inserted }
        let rankedIDs = await appActivationTracker.rankedBundleIDs(from: bundleIDs)
        var appsByBundleID: [String: NSRunningApplication] = [:]
        for app in runningApps {
            guard let bundleID = app.bundleIdentifier else { continue }
            appsByBundleID[bundleID] = appsByBundleID[bundleID] ?? app
        }
        let orderedApps = rankedIDs.compactMap { appsByBundleID[$0] }
        let frontmostApp = NSWorkspace.shared.frontmostApplication
        let axGranted = AXService.isProcessTrusted()

        var orderedRows: [(String, SidebarAppRow)] = []

        for app in orderedApps {
            let bundleID = app.bundleIdentifier ?? UUID().uuidString
            let appName = app.localizedName ?? bundleID
            let windows: [OpenWindowSnapshot] = axGranted
                ? AXService.enumerateWindows(for: app.processIdentifier, appName: appName)
                : []
            let icon = IconCache.shared.runningAppIcon(app)

            if windows.count <= 1 {
                let key = "app:\(bundleID)"
                let isActive = bundleID == frontmostApp?.bundleIdentifier
                let row = makeAppRow(key: key, app: app, isActive: isActive)
                orderedRows.append((key, row))
                continue
            }

            let appKey = "app:\(bundleID)"
            let appIsFrontmost = bundleID == frontmostApp?.bundleIdentifier
            let appRow = makeAppRow(
                key: appKey,
                app: app,
                isActive: appIsFrontmost && !windows.contains(where: { $0.isFocused }),
                windowCount: windows.count
            )
            orderedRows.append((appKey, appRow))

            for (index, window) in windows.enumerated() {
                let windowKey = "win:\(window.windowID)"
                let windowRow = makeWindowRow(key: windowKey, window: window, icon: icon, shortcutIndex: index + 1)
                orderedRows.append((windowKey, windowRow))
            }
        }

        var rowsToShow: [(String, NSView)] = orderedRows.map { ($0.0, $0.1) }
        if !showsAllSidebarRows, orderedRows.count > Self.collapsedRowLimit {
            let hiddenCount = orderedRows.count - Self.collapsedRowLimit
            rowsToShow = Array(orderedRows.prefix(Self.collapsedRowLimit)).map { ($0.0, $0.1) }
            let moreRow = SidebarMoreRow(label: "+\(hiddenCount) more") { [weak self] in
                self?.showsAllSidebarRows = true
                Task { await self?.refresh() }
            }
            rowsToShow.append(("__sidebar_more__", moreRow))
        }

        applyDiff(orderedRows: rowsToShow)
    }

    private func makeAppRow(
        key: String,
        app: NSRunningApplication,
        isActive: Bool,
        windowCount: Int? = nil
    ) -> SidebarAppRow {
        if let existing = rowByKey[key] as? SidebarAppRow {
            existing.setHighlighted(isActive)
            return existing
        }
        let row = SidebarAppRow(app: app, isActive: isActive, windowCount: windowCount) { [weak self] in
            self?.onActionDismiss()
            app.unhide()
            app.activate(from: NSRunningApplication.current, options: [.activateAllWindows])
        }
        return row
    }

    private func makeWindowRow(
        key: String,
        window: OpenWindowSnapshot,
        icon: NSImage?,
        shortcutIndex: Int
    ) -> SidebarAppRow {
        if let existing = rowByKey[key] as? SidebarAppRow {
            existing.setHighlighted(window.isFocused)
            return existing
        }
        return SidebarAppRow(window: window, icon: icon, shortcutIndex: shortcutIndex) { [weak self] in
            guard let self else { return }
            self.onActionDismiss()
            Task {
                await self.actionExecutor.run(
                    Action(
                        id: ActionID(module: .windows, key: "sidebar.focus.\(window.windowID)"),
                        title: "Focus Window",
                        kind: .focusWindow(windowID: window.windowID, pid: window.pid, title: window.title)
                    ),
                    for: ResultID(module: .apps, key: "sidebar.\(window.id)")
                )
            }
        }
    }

    private func applyDiff(orderedRows: [(String, NSView)]) {
        let desiredKeys = Set(orderedRows.map(\.0))
        for key in rowByKey.keys where !desiredKeys.contains(key) {
            if let row = rowByKey.removeValue(forKey: key) {
                stack.removeArrangedSubview(row)
                row.removeFromSuperview()
            }
        }

        for (index, pair) in orderedRows.enumerated() {
            let (key, row) = pair
            rowByKey[key] = row
            let currentIndex = stack.arrangedSubviews.firstIndex(of: row)
            if currentIndex == nil {
                stack.insertArrangedSubview(row, at: min(index, stack.arrangedSubviews.count))
                bindSidebarRow(row, to: stack)
            } else if currentIndex != index {
                stack.removeArrangedSubview(row)
                stack.insertArrangedSubview(row, at: min(index, stack.arrangedSubviews.count))
            }
        }
    }

    private func bindSidebarRow(_ row: NSView, to stack: NSStackView) {
        if let appRow = row as? SidebarAppRow {
            appRow.bindFullWidth(to: stack)
        } else if let moreRow = row as? SidebarMoreRow {
            moreRow.bindFullWidth(to: stack)
        }
    }
}
