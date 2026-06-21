import Foundation

struct KeyCombo: Sendable, Codable, Hashable {
    var virtualKeyCode: UInt32
    var carbonModifiers: UInt32
}

enum HotkeyError: Error {
    case handlerInstallFailed(OSStatus)
    case registrationFailed(OSStatus)
}
