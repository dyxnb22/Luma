import Foundation

enum HotkeyConfig {
    // Command+Space. Carbon key code for Space is 49; cmdKey is 1 << 8.
    static let defaultCombo = KeyCombo(virtualKeyCode: 49, carbonModifiers: 1 << 8)

    static func load() -> KeyCombo {
        defaultCombo
    }

    static func save(_ combo: KeyCombo) {
        // Luma's primary entry point is intentionally fixed at Command+Space.
        // Keep this no-op so older settings builds cannot persist a different chord.
    }

    static func resetToDefault() {
        UserDefaults.standard.removeObject(forKey: "luma.hotkey")
    }
}
