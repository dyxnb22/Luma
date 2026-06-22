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
        let opts = ["AXTrustedCheckOptionPrompt": true] as NSDictionary
        _ = AXIsProcessTrustedWithOptions(opts)
    }

    public init() {}

    public func windows(for pid: Int32) async -> [OpenWindowSnapshot] {
        Self.enumerateWindows(for: pid)
    }

    public func focus(windowID: UInt32, pid: Int32, title: String) async {
        await Self.focusWindow(windowID: windowID, pid: pid, title: title)
    }

    public func focus(windowID: UInt32) async {
        guard let match = Self.cgWindows(for: nil).first(where: { $0.windowID == windowID }) else { return }
        await Self.focusWindow(windowID: match.windowID, pid: match.pid, title: match.title)
    }

    public func insert(text: String) async {
        await MainActor.run {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
        }
    }

    public func applyWindowLayout(_ preset: String) async {
        guard AXIsProcessTrusted() else { return }
        guard let app = NSWorkspace.shared.frontmostApplication else { return }
        let appElement = AXUIElementCreateApplication(app.processIdentifier)
        var focused: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXFocusedWindowAttribute as CFString, &focused) == .success,
              let window = focused else { return }

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
        for axWindow in axWindows {
            guard let snapshot = snapshot(
                for: axWindow,
                pid: pid,
                cgWindows: cgWindows,
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

    nonisolated private static func focusWindow(windowID: UInt32, pid: Int32, title: String) async {
        guard AXIsProcessTrusted() else { return }
        let appElement = AXUIElementCreateApplication(pid)
        var windowsRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &windowsRef) == .success,
              let windows = windowsRef as? [AXUIElement], !windows.isEmpty else { return }

        let targetBounds = cgWindowBounds(windowID: windowID, pid: pid)
        var targetWindow: AXUIElement?

        if let targetBounds {
            for window in windows {
                if let axBounds = axWindowBounds(window), boundsMatch(axBounds, targetBounds) {
                    targetWindow = window
                    break
                }
            }
        }

        if targetWindow == nil, !title.isEmpty {
            for window in windows {
                var titleRef: CFTypeRef?
                guard AXUIElementCopyAttributeValue(window, kAXTitleAttribute as CFString, &titleRef) == .success,
                      let windowTitle = titleRef as? String else { continue }
                if windowTitle == title {
                    targetWindow = window
                    break
                }
            }
        }

        let window = targetWindow ?? windows[0]
        _ = AXUIElementPerformAction(window, kAXRaiseAction as CFString)
        AXUIElementSetAttributeValue(window, kAXMainAttribute as CFString, kCFBooleanTrue)
        AXUIElementSetAttributeValue(window, kAXFocusedAttribute as CFString, kCFBooleanTrue)
        _ = await MainActor.run {
            if let app = NSRunningApplication(processIdentifier: pid) {
                app.unhide()
                app.activate(from: NSRunningApplication.current, options: [.activateAllWindows])
            }
        }
    }

    // MARK: - AX helpers

    nonisolated private static func snapshot(
        for axWindow: AXUIElement,
        pid: Int32,
        cgWindows: [CGWindowRecord],
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
            cg.title == title || (cg.title.isEmpty && boundsMatch(cg.bounds, axBounds))
        }) {
            windowID = match.windowID
        } else if let match = cgWindows.first(where: { boundsMatch($0.bounds, axBounds) }) {
            windowID = match.windowID
        } else {
            windowID = 0
        }

        let isFocused = focusedBounds.map { boundsMatch($0, axBounds) } ?? false
        return OpenWindowSnapshot(
            windowID: windowID,
            pid: pid,
            title: title,
            isMain: isMain,
            isMinimized: isMinimized,
            isFocused: isFocused
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

    nonisolated private static func boundsMatch(_ lhs: CGRect, _ rhs: CGRect, tolerance: CGFloat = 2) -> Bool {
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
