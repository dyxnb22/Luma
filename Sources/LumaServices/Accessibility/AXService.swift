import AppKit
import ApplicationServices
import CoreGraphics
import Foundation
import LumaCore

public actor AXService: AccessibilityClient {
    public nonisolated static func isProcessTrusted() -> Bool {
        AXIsProcessTrusted()
    }

    public nonisolated static func requestPermission() {
        promptForPermission()
    }

    public nonisolated static func promptForPermission() {
        let opts = ["AXTrustedCheckOptionPrompt": true] as NSDictionary
        _ = AXIsProcessTrustedWithOptions(opts)
    }

    public init() {}

    public func isTrusted() async -> Bool {
        Self.isProcessTrusted()
    }

    public func requestPermission() async {
        Self.promptForPermission()
    }

    public func windows(for pid: Int32) async -> [OpenWindowSnapshot] {
        Self.enumerateWindows(for: pid)
    }

    public func focus(windowID: UInt32, pid: Int32, title: String, axTitle: String?, bounds: WindowBounds?) async {
        await Self.focusWindow(windowID: windowID, pid: pid, title: title, axTitle: axTitle, bounds: bounds?.rect)
    }

    public func focus(windowID: UInt32) async {
        guard let match = Self.cgWindows(for: nil).first(where: { $0.windowID == windowID }) else { return }
        await Self.focusWindow(windowID: match.windowID, pid: match.pid, title: match.title, axTitle: nil, bounds: match.bounds)
    }

    public func insert(text: String) async {
        await preparePasteboard(text)
        guard Self.isProcessTrusted() else { return }
        try? await Task.sleep(for: .milliseconds(80))
        await Self.postCommandV()
    }

    public func replaceSelectedText(with text: String) async -> Bool {
        guard Self.isProcessTrusted() else { return false }
        await preparePasteboard(text)
        if await setFocusedSelectedText(text) {
            return true
        }
        try? await Task.sleep(for: .milliseconds(80))
        await Self.postCommandV()
        return true
    }

    private func preparePasteboard(_ text: String) async {
        await MainActor.run {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
        }
    }

    private func setFocusedSelectedText(_ text: String) async -> Bool {
        await MainActor.run {
            guard let app = NSWorkspace.shared.frontmostApplication else { return false }
            let appElement = AXUIElementCreateApplication(app.processIdentifier)
            var focusedRef: CFTypeRef?
            guard AXUIElementCopyAttributeValue(
                appElement,
                kAXFocusedUIElementAttribute as CFString,
                &focusedRef
            ) == .success,
                let focusedRef else { return false }
            let element = focusedRef as! AXUIElement
            return AXUIElementSetAttributeValue(
                element,
                kAXSelectedTextAttribute as CFString,
                text as CFString
            ) == .success
        }
    }

    @MainActor
    private static func postCommandV() {
        let source = CGEventSource(stateID: .combinedSessionState)
        let keyDown = CGEvent(keyboardEventSource: source, virtualKey: 9, keyDown: true)!
        keyDown.flags = .maskCommand
        let keyUp = CGEvent(keyboardEventSource: source, virtualKey: 9, keyDown: false)!
        keyUp.flags = .maskCommand
        keyDown.post(tap: .cghidEventTap)
        keyUp.post(tap: .cghidEventTap)
    }

    public func applyWindowLayout(_ preset: String) async {
        guard AXIsProcessTrusted() else { return }
        guard let app = NSWorkspace.shared.frontmostApplication else { return }
        let appElement = AXUIElementCreateApplication(app.processIdentifier)
        var focused: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXFocusedWindowAttribute as CFString, &focused) == .success,
              let window = focused else { return }

        // TODO: use the screen containing the focused window instead of main screen.
        let screen = NSScreen.main?.visibleFrame ?? NSScreen.screens.first?.visibleFrame ?? .zero
        let frame = frame(for: preset, screen: screen)
        var origin = frame.origin
        var size = frame.size
        guard let positionValue = AXValueCreate(.cgPoint, &origin),
              let sizeValue = AXValueCreate(.cgSize, &size) else { return }

        AXUIElementSetAttributeValue(window as! AXUIElement, kAXPositionAttribute as CFString, positionValue)
        AXUIElementSetAttributeValue(window as! AXUIElement, kAXSizeAttribute as CFString, sizeValue)
    }

    // MARK: - Window enumeration

    nonisolated public static func enumerateWindows(for pid: Int32, appName: String = "") -> [OpenWindowSnapshot] {
        guard AXIsProcessTrusted() else { return [] }

        let appElement = AXUIElementCreateApplication(pid)
        var windowsRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &windowsRef) == .success,
              let axWindows = windowsRef as? [AXUIElement], !axWindows.isEmpty else {
            return []
        }

        let cgWindows = cgWindows(for: pid)
        let focusedBounds = focusedAXWindowBounds(for: appElement)
        let fallbackTitle = appName.isEmpty ? (NSRunningApplication(processIdentifier: pid)?.localizedName ?? "Window") : appName

        var snapshots: [OpenWindowSnapshot] = []
        var usedCGWindowIDs = Set<UInt32>()
        for axWindow in axWindows {
            guard let snapshot = snapshot(
                for: axWindow,
                pid: pid,
                cgWindows: cgWindows,
                usedCGWindowIDs: &usedCGWindowIDs,
                focusedBounds: focusedBounds,
                fallbackTitle: fallbackTitle
            ) else { continue }
            if snapshot.isMinimized { continue }
            snapshots.append(snapshot)
        }

        if snapshots.isEmpty {
            return []
        }

        let hasTitled = snapshots.contains { !$0.title.isEmpty && $0.title != fallbackTitle }
        if hasTitled {
            snapshots = snapshots.filter { !$0.title.isEmpty }
        }

        return snapshots.sorted { lhs, rhs in
            if lhs.isMain != rhs.isMain { return lhs.isMain }
            if lhs.isFocused != rhs.isFocused { return lhs.isFocused }
            return lhs.title.localizedStandardCompare(rhs.title) == .orderedAscending
        }
    }

    // MARK: - Focus

    nonisolated private static func focusWindow(
        windowID: UInt32,
        pid: Int32,
        title: String,
        axTitle: String?,
        bounds: CGRect?
    ) async {
        guard AXIsProcessTrusted() else { return }
        try? await Task.sleep(for: .milliseconds(80))

        let appElement = AXUIElementCreateApplication(pid)
        var windowsRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &windowsRef) == .success,
              let windows = windowsRef as? [AXUIElement], !windows.isEmpty else { return }

        let targetWindow = resolveFocusTarget(
            windows: windows,
            windowID: windowID,
            pid: pid,
            title: title,
            axTitle: axTitle,
            bounds: bounds
        )
        guard let window = targetWindow else { return }

        await MainActor.run {
            if let app = NSRunningApplication(processIdentifier: pid) {
                app.unhide()
                app.activate(options: [.activateAllWindows])
            }
        }

        AXUIElementSetAttributeValue(window, kAXMinimizedAttribute as CFString, kCFBooleanFalse)
        AXUIElementSetAttributeValue(appElement, kAXFocusedWindowAttribute as CFString, window)
        _ = AXUIElementPerformAction(window, kAXRaiseAction as CFString)
        AXUIElementSetAttributeValue(window, kAXMainAttribute as CFString, kCFBooleanTrue)
        AXUIElementSetAttributeValue(window, kAXFocusedAttribute as CFString, kCFBooleanTrue)
    }

    nonisolated private static func resolveFocusTarget(
        windows: [AXUIElement],
        windowID: UInt32,
        pid: Int32,
        title: String,
        axTitle: String?,
        bounds: CGRect?
    ) -> AXUIElement? {
        if windowID != 0 {
            for window in windows where axWindowNumber(window) == windowID {
                return window
            }
        }

        let app = NSRunningApplication(processIdentifier: pid)
        let bundleID = app?.bundleIdentifier ?? ""
        let appName = app?.localizedName ?? ""
        let titledWindows: [WindowFocusMatcher.TitledWindow] = windows.enumerated().compactMap { index, window in
            guard let windowTitle = axWindowTitle(window) else { return nil }
            return (index, windowTitle)
        }

        for candidate in titleCandidates(displayTitle: title, axTitle: axTitle) {
            if let windowIndex = WindowFocusMatcher.matchingIndex(
                in: titledWindows,
                queryTitle: candidate,
                bundleID: bundleID,
                appName: appName
            ), windows.indices.contains(windowIndex) {
                return windows[windowIndex]
            }
        }

        if let bounds, bounds != .zero {
            for window in windows {
                if let axBounds = axWindowBounds(window), boundsMatch(axBounds, bounds) {
                    return window
                }
            }
        }

        if windowID != 0, let targetBounds = cgWindowBounds(windowID: windowID, pid: pid) {
            for window in windows {
                if let axBounds = axWindowBounds(window), boundsMatch(axBounds, targetBounds) {
                    return window
                }
            }
        }

        return windows.count == 1 ? windows[0] : nil
    }

    nonisolated private static func titleCandidates(displayTitle: String, axTitle: String?) -> [String] {
        var candidates: [String] = []
        let display = displayTitle.trimmingCharacters(in: .whitespacesAndNewlines)
        if !display.isEmpty { candidates.append(display) }
        if let axTitle {
            let raw = axTitle.trimmingCharacters(in: .whitespacesAndNewlines)
            if !raw.isEmpty, raw != display { candidates.append(raw) }
        }
        return candidates
    }

    nonisolated private static func axWindowNumber(_ window: AXUIElement) -> UInt32? {
        var value: CFTypeRef?
        guard AXUIElementCopyAttributeValue(window, "AXWindowNumber" as CFString, &value) == .success,
              let value else { return nil }
        if let number = value as? Int, number >= 0 { return UInt32(number) }
        if let number = value as? UInt32 { return number }
        if let number = value as? Int64, number >= 0 { return UInt32(number) }
        return nil
    }

    nonisolated private static func axWindowTitle(_ window: AXUIElement) -> String? {
        var titleRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(window, kAXTitleAttribute as CFString, &titleRef) == .success,
              let raw = titleRef as? String else { return nil }
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    // MARK: - AX helpers

    nonisolated private static func snapshot(
        for axWindow: AXUIElement,
        pid: Int32,
        cgWindows: [CGWindowRecord],
        usedCGWindowIDs: inout Set<UInt32>,
        focusedBounds: CGRect?,
        fallbackTitle: String
    ) -> OpenWindowSnapshot? {
        var titleRef: CFTypeRef?
        let title: String
        if AXUIElementCopyAttributeValue(axWindow, kAXTitleAttribute as CFString, &titleRef) == .success,
           let raw = titleRef as? String,
           !raw.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            title = raw
        } else {
            title = fallbackTitle
        }

        var minimizedRef: CFTypeRef?
        let isMinimized: Bool
        if AXUIElementCopyAttributeValue(axWindow, kAXMinimizedAttribute as CFString, &minimizedRef) == .success,
           let value = minimizedRef as? Bool {
            isMinimized = value
        } else {
            isMinimized = false
        }

        var mainRef: CFTypeRef?
        let isMain: Bool
        if AXUIElementCopyAttributeValue(axWindow, kAXMainAttribute as CFString, &mainRef) == .success,
           let value = mainRef as? Bool {
            isMain = value
        } else {
            isMain = false
        }

        guard let axBounds = axWindowBounds(axWindow), axBounds.width >= 80, axBounds.height >= 40 else {
            return nil
        }

        let windowID: UInt32
        if let match = cgWindows.first(where: { cg in
            !usedCGWindowIDs.contains(cg.windowID)
                && (cg.title == title || (cg.title.isEmpty && boundsMatch(cg.bounds, axBounds)))
        }) {
            windowID = match.windowID
        } else if let match = cgWindows.first(where: { cg in
            !usedCGWindowIDs.contains(cg.windowID) && boundsMatch(cg.bounds, axBounds)
        }) {
            windowID = match.windowID
        } else {
            windowID = 0
        }
        if windowID != 0 {
            usedCGWindowIDs.insert(windowID)
        }

        let isFocused = focusedBounds.map { boundsMatch($0, axBounds) } ?? false
        return OpenWindowSnapshot(
            windowID: windowID,
            pid: pid,
            title: title,
            isMain: isMain,
            isMinimized: isMinimized,
            isFocused: isFocused,
            bounds: axBounds
        )
    }

    nonisolated private static func focusedAXWindowBounds(for appElement: AXUIElement) -> CGRect? {
        var focusedRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXFocusedWindowAttribute as CFString, &focusedRef) == .success,
              let focused = focusedRef else { return nil }
        return axWindowBounds(focused as! AXUIElement)
    }

    private struct CGWindowRecord {
        let windowID: UInt32
        let pid: Int32
        let title: String
        let bounds: CGRect
    }

    nonisolated private static func cgWindows(for pid: Int32?) -> [CGWindowRecord] {
        guard let info = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else {
            return []
        }
        let myPID = ProcessInfo.processInfo.processIdentifier
        return info.compactMap { item in
            guard let id = item[kCGWindowNumber as String] as? UInt32,
                  let ownerPID = item[kCGWindowOwnerPID as String] as? Int32,
                  ownerPID != myPID else { return nil }
            if let pid, ownerPID != pid { return nil }
            let title = item[kCGWindowName as String] as? String ?? ""
            guard let boundsDict = item[kCGWindowBounds as String] as? [String: CGFloat] else { return nil }
            let bounds = CGRect(
                x: boundsDict["X"] ?? 0,
                y: boundsDict["Y"] ?? 0,
                width: boundsDict["Width"] ?? 0,
                height: boundsDict["Height"] ?? 0
            )
            guard bounds.width >= 80, bounds.height >= 40 else { return nil }
            return CGWindowRecord(windowID: id, pid: ownerPID, title: title, bounds: bounds)
        }
    }

    nonisolated private static func cgWindowBounds(windowID: UInt32, pid: Int32) -> CGRect? {
        guard let info = CGWindowListCopyWindowInfo([.optionIncludingWindow], CGWindowID(windowID)) as? [[String: Any]],
              let item = info.first,
              let ownerPID = item[kCGWindowOwnerPID as String] as? Int32,
              ownerPID == pid,
              let boundsDict = item[kCGWindowBounds as String] as? [String: CGFloat] else { return nil }
        return CGRect(
            x: boundsDict["X"] ?? 0,
            y: boundsDict["Y"] ?? 0,
            width: boundsDict["Width"] ?? 0,
            height: boundsDict["Height"] ?? 0
        )
    }

    nonisolated private static func axWindowBounds(_ window: AXUIElement) -> CGRect? {
        var positionRef: CFTypeRef?
        var sizeRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(window, kAXPositionAttribute as CFString, &positionRef) == .success,
              AXUIElementCopyAttributeValue(window, kAXSizeAttribute as CFString, &sizeRef) == .success,
              let positionValue = positionRef, let sizeValue = sizeRef else { return nil }

        var origin = CGPoint.zero
        var size = CGSize.zero
        guard AXValueGetValue(positionValue as! AXValue, .cgPoint, &origin),
              AXValueGetValue(sizeValue as! AXValue, .cgSize, &size) else { return nil }

        guard let screen = NSScreen.main ?? NSScreen.screens.first else {
            return CGRect(origin: origin, size: size)
        }

        let flippedY = screen.frame.height - origin.y - size.height
        return CGRect(x: origin.x, y: flippedY, width: size.width, height: size.height)
    }

    nonisolated private static func boundsMatch(_ lhs: CGRect, _ rhs: CGRect, tolerance: CGFloat = 20) -> Bool {
        abs(lhs.origin.x - rhs.origin.x) <= tolerance
            && abs(lhs.origin.y - rhs.origin.y) <= tolerance
            && abs(lhs.width - rhs.width) <= tolerance
            && abs(lhs.height - rhs.height) <= tolerance
    }

    private func frame(for preset: String, screen: NSRect) -> CGRect {
        switch preset {
        case "left-half":
            return CGRect(x: screen.minX, y: screen.minY, width: screen.width / 2, height: screen.height)
        case "right-half":
            return CGRect(x: screen.midX, y: screen.minY, width: screen.width / 2, height: screen.height)
        case "top-half":
            return CGRect(x: screen.minX, y: screen.midY, width: screen.width, height: screen.height / 2)
        case "bottom-half":
            return CGRect(x: screen.minX, y: screen.minY, width: screen.width, height: screen.height / 2)
        case "center":
            let width = screen.width * 0.72
            let height = screen.height * 0.72
            return CGRect(x: screen.midX - width / 2, y: screen.midY - height / 2, width: width, height: height)
        default:
            return screen
        }
    }
}
