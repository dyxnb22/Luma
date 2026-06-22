import AppKit
import ApplicationServices
import CoreGraphics
import Foundation
import LumaCore

public actor AXService: AccessibilityClient {
    public init() {}

    public func focus(windowID: UInt32, pid: Int32, title: String) async {
        guard AXIsProcessTrusted() else { return }
        let appElement = AXUIElementCreateApplication(pid)
        var windowsRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &windowsRef) == .success,
              let windows = windowsRef as? [AXUIElement], !windows.isEmpty else { return }

        let targetBounds = Self.cgWindowBounds(windowID: windowID, pid: pid)
        var targetWindow: AXUIElement?

        if let targetBounds {
            for window in windows {
                if let axBounds = Self.axWindowBounds(window), Self.boundsMatch(axBounds, targetBounds) {
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
        AXUIElementSetAttributeValue(window, kAXMainAttribute as CFString, kCFBooleanTrue)
        AXUIElementSetAttributeValue(window, kAXFocusedAttribute as CFString, kCFBooleanTrue)
        // NSRunningApplication.activate is documented as thread-safe; hop to MainActor for AppKit consistency.
        _ = await MainActor.run {
            NSRunningApplication(processIdentifier: pid)?.activate(options: [.activateAllWindows])
        }
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

    private static func cgWindowBounds(windowID: UInt32, pid: Int32) -> CGRect? {
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

    private static func axWindowBounds(_ window: AXUIElement) -> CGRect? {
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

    private static func boundsMatch(_ lhs: CGRect, _ rhs: CGRect, tolerance: CGFloat = 2) -> Bool {
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
