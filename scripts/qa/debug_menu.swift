#!/usr/bin/env swift
import AppKit
import ApplicationServices

let target = CommandLine.arguments.dropFirst().first ?? "Cursor"
guard let app = NSWorkspace.shared.runningApplications.first(where: {
    ($0.localizedName?.contains(target) == true) || ($0.bundleIdentifier?.lowercased().contains(target.lowercased()) == true)
}) else {
    fputs("app-not-found: \(target)\n", stderr)
    exit(1)
}
fputs("bundle=\(app.bundleIdentifier ?? "?") pid=\(app.processIdentifier)\n", stderr)
let appElement = AXUIElementCreateApplication(app.processIdentifier)
var menuBarRef: CFTypeRef?
let status = AXUIElementCopyAttributeValue(appElement, kAXMenuBarAttribute as CFString, &menuBarRef)
fputs("menuBar status=\(status) found=\(menuBarRef != nil)\n", stderr)
if status == .success, let menuBarRef {
    var count = 0
    func walk(_ element: AXUIElement, depth: Int) {
        guard depth < 6, count < 30 else { return }
        var childrenRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef) == .success,
              let children = childrenRef as? [AXUIElement] else { return }
        for child in children {
            var roleRef: CFTypeRef?
            var titleRef: CFTypeRef?
            _ = AXUIElementCopyAttributeValue(child, kAXRoleAttribute as CFString, &roleRef)
            _ = AXUIElementCopyAttributeValue(child, kAXTitleAttribute as CFString, &titleRef)
            let role = roleRef as? String ?? ""
            let title = (titleRef as? String ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            if role == (kAXMenuItemRole as String), !title.isEmpty {
                fputs("item: \(title)\n", stderr)
                count += 1
            }
            walk(child, depth: depth + 1)
        }
    }
    walk(menuBarRef as! AXUIElement, depth: 0)
    fputs("total=\(count)\n", stderr)
}
