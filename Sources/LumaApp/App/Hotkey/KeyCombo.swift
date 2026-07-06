import AppKit
import Carbon.HIToolbox
import Foundation

struct KeyCombo: Sendable, Codable, Hashable {
    var virtualKeyCode: UInt32
    var carbonModifiers: UInt32

    func matches(_ event: NSEvent) -> Bool {
        guard UInt32(event.keyCode) == virtualKeyCode else { return false }
        return Self.carbonFlags(from: event) == carbonModifiers
    }

    private static func carbonFlags(from event: NSEvent) -> UInt32 {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        var carbon: UInt32 = 0
        if flags.contains(.command) { carbon |= UInt32(cmdKey) }
        if flags.contains(.shift) { carbon |= UInt32(shiftKey) }
        if flags.contains(.option) { carbon |= UInt32(optionKey) }
        if flags.contains(.control) { carbon |= UInt32(controlKey) }
        return carbon
    }
}

enum HotkeyError: Error {
    case handlerInstallFailed(OSStatus)
    case registrationFailed(OSStatus)
}
