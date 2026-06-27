#!/usr/bin/env swift
import AppKit
import ApplicationServices

let bid = CommandLine.arguments.dropFirst().first ?? "com.todesktop.230313mzl4w4u92"
guard let app = NSRunningApplication.runningApplications(withBundleIdentifier: bid).first else {
    fputs("no app for \(bid)\n", stderr)
    exit(1)
}
let el = AXUIElementCreateApplication(app.processIdentifier)
var ref: CFTypeRef?
guard AXUIElementCopyAttributeValue(el, kAXMenuBarAttribute as CFString, &ref) == .success, let menuBar = ref else {
    fputs("no menubar\n", stderr)
    exit(1)
}
var hits: [String] = []
func walk(_ element: AXUIElement, path: [String], depth: Int) {
    guard depth < 6 else { return }
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
        let next = title.isEmpty ? path : path + [title]
        let nested = childElements(child)
        let hasSub = !nested.isEmpty
        if role == (kAXMenuItemRole as String), !title.isEmpty, !hasSub {
            let line = next.joined(separator: " → ")
            if line.lowercased().contains("fold") { hits.append(line) }
        }
        if hasSub { walk(child, path: next, depth: depth + 1) }
    }
}
func childElements(_ element: AXUIElement) -> [AXUIElement] {
    var childrenRef: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef) == .success,
          let children = childrenRef as? [AXUIElement] else { return [] }
    return children
}
walk(menuBar as! AXUIElement, path: [], depth: 0)
fputs("fold hits: \(hits.count)\n", stderr)
for h in hits.prefix(20) { fputs("\(h)\n", stderr) }
