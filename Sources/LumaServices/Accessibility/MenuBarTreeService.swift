import AppKit
import ApplicationServices
import Foundation
import LumaCore

public struct MenuItemRecord: Sendable, Hashable {
    public let bundleID: String
    public let titlePath: [String]
    public let shortcutDisplay: String?
    public let axPath: [Int]
    public let isEnabled: Bool

    public init(bundleID: String, titlePath: [String], shortcutDisplay: String?, axPath: [Int], isEnabled: Bool) {
        self.bundleID = bundleID
        self.titlePath = titlePath
        self.shortcutDisplay = shortcutDisplay
        self.axPath = axPath
        self.isEnabled = isEnabled
    }
}

public actor MenuBarTreeService {
    public static let shared = MenuBarTreeService()

    private var cache: [String: (date: Date, records: [MenuItemRecord])] = [:]
    private var disabledBundleIDs: Set<String> = []
    private var launcherContextBundleID: String?
    private var observer: NSObjectProtocol?
    private var walkTask: Task<Void, Never>?
    private let ttl: TimeInterval = 30

    public init() {}

    public func start(disabledBundleIDs: Set<String> = []) {
        self.disabledBundleIDs = disabledBundleIDs
        guard observer == nil else {
            scheduleRefreshForFrontmost()
            return
        }
        observer = NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.didActivateApplicationNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { await self?.scheduleRefreshForFrontmost() }
        }
        scheduleRefreshForFrontmost()
    }

    public func setLauncherContextBundleID(_ bundleID: String?) {
        launcherContextBundleID = bundleID
    }

    public func cachedRecordsForFrontmost() -> [MenuItemRecord] {
        guard let bundleID = effectiveTargetBundleID(),
              !disabledBundleIDs.contains(bundleID),
              let cached = cache[bundleID],
              Date().timeIntervalSince(cached.date) <= ttl else { return [] }
        return cached.records
    }

    public func recordsForTarget(deadline: ContinuousClock.Instant) async -> [MenuItemRecord] {
        let existing = cachedRecordsForFrontmost()
        if !existing.isEmpty { return existing }
        scheduleRefreshForFrontmost()
        while ContinuousClock.now < deadline {
            let records = cachedRecordsForFrontmost()
            if !records.isEmpty { return records }
            try? await Task.sleep(for: .milliseconds(25))
        }
        return cachedRecordsForFrontmost()
    }

    private func effectiveTargetBundleID() -> String? {
        let lumaBundleID = Bundle.main.bundleIdentifier
        if let context = LauncherMenuTarget.current(),
           context != lumaBundleID {
            return context
        }
        if let launcherContextBundleID,
           launcherContextBundleID != lumaBundleID {
            return launcherContextBundleID
        }
        guard let app = NSWorkspace.shared.frontmostApplication,
              let bundleID = app.bundleIdentifier,
              bundleID != lumaBundleID else { return nil }
        return bundleID
    }

    public func frontmostBundleID() -> String? {
        effectiveTargetBundleID()
    }

    public func scheduleRefreshForFrontmost() {
        walkTask?.cancel()
        guard AXIsProcessTrusted(),
              let bundleID = effectiveTargetBundleID(),
              !disabledBundleIDs.contains(bundleID),
              let app = NSRunningApplication.runningApplications(withBundleIdentifier: bundleID).first else { return }
        walkTask = Task.detached(priority: .utility) { [weak self] in
            let records = Self.walk(app: app, bundleID: bundleID)
            guard !Task.isCancelled else { return }
            await self?.store(records: records, bundleID: bundleID)
        }
    }

    private func store(records: [MenuItemRecord], bundleID: String) {
        cache[bundleID] = (Date(), records)
    }

    private nonisolated static func walk(app: NSRunningApplication, bundleID: String) -> [MenuItemRecord] {
        let start = Date()
        let appElement = AXUIElementCreateApplication(app.processIdentifier)
        var menuBarRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXMenuBarAttribute as CFString, &menuBarRef) == .success,
              let menuBar = menuBarRef else { return [] }

        var output: [MenuItemRecord] = []
        var queue: [(element: AXUIElement, path: [String], axPath: [Int], depth: Int)] = [(menuBar as! AXUIElement, [], [], 0)]
        while !queue.isEmpty, Date().timeIntervalSince(start) < 0.5 {
            let current = queue.removeFirst()
            guard current.depth < 6 else { continue }
            let children = childElements(of: current.element)
            for (index, child) in children.enumerated() {
                guard Date().timeIntervalSince(start) < 0.5 else { break }
                let role = stringAttribute(child, kAXRoleAttribute as CFString) ?? ""
                if role == "AXSeparator" { continue }
                let title = (stringAttribute(child, kAXTitleAttribute as CFString) ?? "")
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                let nextPath = title.isEmpty ? current.path : current.path + [title]
                let nextAXPath = current.axPath + [index]
                let enabled = boolAttribute(child, kAXEnabledAttribute as CFString) ?? true
                let nested = childElements(of: child)
                let hasSubmenu = !nested.isEmpty
                if role == kAXMenuItemRole as String, enabled, !title.isEmpty, !hasSubmenu {
                    output.append(MenuItemRecord(
                        bundleID: bundleID,
                        titlePath: nextPath,
                        shortcutDisplay: shortcutDisplay(for: child),
                        axPath: nextAXPath,
                        isEnabled: enabled
                    ))
                }
                if hasSubmenu {
                    queue.append((child, nextPath, nextAXPath, current.depth + 1))
                }
            }
        }
        return output
    }

    private nonisolated static func childElements(of element: AXUIElement) -> [AXUIElement] {
        var childrenRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef) == .success,
              let children = childrenRef as? [AXUIElement] else { return [] }
        return children
    }

    private nonisolated static func stringAttribute(_ element: AXUIElement, _ name: CFString) -> String? {
        var ref: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element, name, &ref) == .success else { return nil }
        return ref as? String
    }

    private nonisolated static func boolAttribute(_ element: AXUIElement, _ name: CFString) -> Bool? {
        var ref: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element, name, &ref) == .success else { return nil }
        return ref as? Bool
    }

    private nonisolated static func shortcutDisplay(for element: AXUIElement) -> String? {
        guard let char = stringAttribute(element, kAXMenuItemCmdCharAttribute as CFString), !char.isEmpty else {
            return nil
        }
        var pieces: [String] = []
        if let raw = numberAttribute(element, kAXMenuItemCmdModifiersAttribute as CFString) {
            let flags = Int(raw)
            if flags & 8 != 0 { pieces.append("⌃") }
            if flags & 4 != 0 { pieces.append("⌥") }
            if flags & 2 != 0 { pieces.append("⇧") }
            if flags & 1 != 0 { pieces.append("⌘") }
        } else {
            pieces.append("⌘")
        }
        pieces.append(char.uppercased())
        return pieces.joined()
    }

    private nonisolated static func numberAttribute(_ element: AXUIElement, _ name: CFString) -> Int? {
        var ref: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element, name, &ref) == .success else { return nil }
        if let number = ref as? NSNumber { return number.intValue }
        return nil
    }
}
