#!/usr/bin/env swift
import AppKit
import ApplicationServices
import Carbon

guard CommandLine.arguments.count > 1 else {
    fputs("usage: paste_query.swift <text>\n", stderr)
    exit(2)
}
let text = CommandLine.arguments.dropFirst().joined(separator: " ")

let keyCodes: [Character: CGKeyCode] = [
    "a": 0, "b": 11, "c": 8, "d": 2, "e": 14, "f": 3, "g": 5, "h": 4, "i": 34, "j": 38,
    "k": 40, "l": 37, "m": 46, "n": 45, "o": 31, "p": 35, "q": 12, "r": 15, "s": 1, "t": 17,
    "u": 32, "v": 9, "w": 13, "x": 7, "y": 16, "z": 6,
    "0": 29, "1": 18, "2": 19, "3": 20, "4": 21, "5": 23, "6": 22, "7": 26, "8": 28, "9": 25,
    " ": 49, "-": 27, ".": 47, "/": 44, ":": 41,
]

func selectABC() {
    let list = TISCreateInputSourceList(nil, true).takeRetainedValue() as! [TISInputSource]
    for source in list {
        guard let raw = TISGetInputSourceProperty(source, kTISPropertyInputSourceID) else { continue }
        let id = Unmanaged<CFString>.fromOpaque(raw).takeUnretainedValue() as String
        if id == "com.apple.keylayout.ABC" {
            TISSelectInputSource(source)
            return
        }
    }
}

func postKey(_ keyCode: CGKeyCode, flags: CGEventFlags = []) {
    let src = CGEventSource(stateID: .hidSystemState)
    let down = CGEvent(keyboardEventSource: src, virtualKey: keyCode, keyDown: true)!
    let up = CGEvent(keyboardEventSource: src, virtualKey: keyCode, keyDown: false)!
    down.flags = flags
    up.flags = flags
    down.post(tap: .cghidEventTap)
    up.post(tap: .cghidEventTap)
    usleep(22_000)
}

func findTextField(in element: AXUIElement) -> AXUIElement? {
    var roleRef: CFTypeRef?
    if AXUIElementCopyAttributeValue(element, kAXRoleAttribute as CFString, &roleRef) == .success,
       let role = roleRef as? String, role == "AXTextField" {
        return element
    }
    var childrenRef: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef) == .success,
          let children = childrenRef as? [AXUIElement] else { return nil }
    for child in children {
        if let found = findTextField(in: child) { return found }
    }
    return nil
}

func clickSearchField(in window: AXUIElement) {
    var positionRef: CFTypeRef?
    var sizeRef: CFTypeRef?
    guard AXUIElementCopyAttributeValue(window, kAXPositionAttribute as CFString, &positionRef) == .success,
          AXUIElementCopyAttributeValue(window, kAXSizeAttribute as CFString, &sizeRef) == .success,
          let positionRef, let sizeRef,
          CFGetTypeID(positionRef) == AXValueGetTypeID(),
          CFGetTypeID(sizeRef) == AXValueGetTypeID() else { return }
    var point = CGPoint.zero
    var size = CGSize.zero
    AXValueGetValue(positionRef as! AXValue, .cgPoint, &point)
    AXValueGetValue(sizeRef as! AXValue, .cgSize, &size)
    let clickPoint = CGPoint(x: point.x + size.width / 2, y: point.y + 72)
    let src = CGEventSource(stateID: .hidSystemState)
    let down = CGEvent(mouseEventSource: src, mouseType: .leftMouseDown, mouseCursorPosition: clickPoint, mouseButton: .left)!
    let up = CGEvent(mouseEventSource: src, mouseType: .leftMouseUp, mouseCursorPosition: clickPoint, mouseButton: .left)!
    down.post(tap: .cghidEventTap)
    up.post(tap: .cghidEventTap)
    usleep(150_000)
}

guard let luma = NSRunningApplication.runningApplications(withBundleIdentifier: "app.luma").first else {
    fputs("luma-not-running\n", stderr)
    exit(1)
}
luma.activate()
usleep(300_000)
selectABC()
usleep(120_000)

let app = AXUIElementCreateApplication(luma.processIdentifier)
var windowValue: CFTypeRef?
if AXUIElementCopyAttributeValue(app, kAXFocusedWindowAttribute as CFString, &windowValue) != .success {
    var windowsValue: CFTypeRef?
    guard AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &windowsValue) == .success,
          let windows = windowsValue as? [AXUIElement], let first = windows.first else {
        fputs("no-window\n", stderr)
        exit(1)
    }
    windowValue = first
}
let window = windowValue as! AXUIElement
clickSearchField(in: window)
if let field = findTextField(in: window) {
    _ = AXUIElementSetAttributeValue(field, kAXValueAttribute as CFString, "" as CFString)
}
usleep(80_000)
postKey(0, flags: .maskCommand)
postKey(51)
usleep(80_000)
for ch in text.lowercased() {
    guard let code = keyCodes[ch] else { continue }
    postKey(code)
}
if let field = findTextField(in: window) {
    _ = AXUIElementSetAttributeValue(field, kAXValueAttribute as CFString, text as CFString)
}
usleep(300_000)
fputs("ok\n", stderr)
