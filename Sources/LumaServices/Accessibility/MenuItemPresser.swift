import AppKit
import ApplicationServices
import Foundation
import LumaCore

public struct MenuItemPresser: Sendable {
    public init() {}

    public func press(bundleID: String, axPath: [Int]) async throws {
        guard AXIsProcessTrusted() else { throw ModuleError.permissionRequired(.accessibility) }
        guard let app = await MainActor.run(body: {
            NSWorkspace.shared.frontmostApplication
        }), app.bundleIdentifier == bundleID else {
            throw ModuleError.dataUnavailable
        }

        let appElement = AXUIElementCreateApplication(app.processIdentifier)
        var menuBarRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXMenuBarAttribute as CFString, &menuBarRef) == .success,
              let menuBarRef else { throw ModuleError.dataUnavailable }
        let menuBar = menuBarRef as! AXUIElement
        var current = menuBar
        for index in axPath {
            var childrenRef: CFTypeRef?
            guard AXUIElementCopyAttributeValue(current, kAXChildrenAttribute as CFString, &childrenRef) == .success,
                  let children = childrenRef as? [AXUIElement],
                  children.indices.contains(index) else { throw ModuleError.dataUnavailable }
            current = children[index]
        }
        AXUIElementPerformAction(current, kAXPressAction as CFString)
    }
}
