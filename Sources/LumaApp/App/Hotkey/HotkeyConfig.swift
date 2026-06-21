import Foundation

enum HotkeyConfig {
    // Command+Space. Carbon key code for Space is 49; cmdKey is 1 << 8.
    static let defaultCombo = KeyCombo(virtualKeyCode: 49, carbonModifiers: 1 << 8)
}
