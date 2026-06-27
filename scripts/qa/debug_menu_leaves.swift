#!/usr/bin/env swift
import AppKit
import ApplicationServices

let bid = "com.todesktop.230313mzl4w4u92"
guard let app = NSRunningApplication.runningApplications(withBundleIdentifier: bid).first else { exit(1) }
let el = AXUIElementCreateApplication(app.processIdentifier)
var ref: CFTypeRef?
guard AXUIElementCopyAttributeValue(el, kAXMenuBarAttribute as CFString, &ref) == .success, let menuBar = ref else { exit(1) }

var leaves: [String] = []
func children(_ e: AXUIElement) -> [AXUIElement] {
    var c: CFTypeRef?
    guard AXUIElementCopyAttributeValue(e, kAXChildrenAttribute as CFString, &c) == .success,
          let a = c as? [AXUIElement] else { return [] }
    return a
}
func walk(_ e: AXUIElement, path: [String], depth: Int) {
    guard depth < 6 else { return }
    for ch in children(e) {
        var roleRef: CFTypeRef?
        var titleRef: CFTypeRef?
        _ = AXUIElementCopyAttributeValue(ch, kAXRoleAttribute as CFString, &roleRef)
        _ = AXUIElementCopyAttributeValue(ch, kAXTitleAttribute as CFString, &titleRef)
        let title = (titleRef as? String ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        let next = title.isEmpty ? path : path + [title]
        let sub = children(ch)
        if (roleRef as? String) == (kAXMenuItemRole as String), !title.isEmpty, sub.isEmpty {
            leaves.append(next.joined(separator: " → "))
        }
        if !sub.isEmpty { walk(ch, path: next, depth: depth + 1) }
    }
}
walk(menuBar as! AXUIElement, path: [], depth: 0)
fputs("leaves=\(leaves.count)\n", stderr)
for l in leaves where l.lowercased().contains("fold") || l.contains("折叠") || l.contains("查看") || l.contains("View") {
    fputs("\(l)\n", stderr)
}
