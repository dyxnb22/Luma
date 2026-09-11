import Foundation

/// The global activation shortcut, isolated so it can become configurable later without
/// redesigning the controller around it.
///
/// Values are Carbon constants, repeated here so this file stays free of Carbon imports and
/// remains testable: `kVK_Space` is `0x31`, `cmdKey` is `1 << 8`.
public struct HotKeyDefinition: Equatable {
    public let identifier: String
    public let keyCode: UInt32
    public let carbonModifiers: UInt32
    /// Human-readable form used in host-level error text.
    public let displayName: String

    public init(
        identifier: String,
        keyCode: UInt32,
        carbonModifiers: UInt32,
        displayName: String
    ) {
        self.identifier = identifier
        self.keyCode = keyCode
        self.carbonModifiers = carbonModifiers
        self.displayName = displayName
    }

    public static let commandSpace = HotKeyDefinition(
        identifier: "command-space",
        keyCode: 0x31,
        carbonModifiers: 1 << 8,
        displayName: "⌘Space"
    )

    public static let optionSpace = HotKeyDefinition(
        identifier: "option-space",
        keyCode: 0x31,
        carbonModifiers: 1 << 11,
        displayName: "⌥Space"
    )

    public static let commandShiftSpace = HotKeyDefinition(
        identifier: "command-shift-space",
        keyCode: 0x31,
        carbonModifiers: (1 << 8) | (1 << 9),
        displayName: "⌘⇧Space"
    )

    /// Fresh-install default. Unlike Command+Space, Option+Space has no macOS
    /// system binding by default. Registration still detects third-party use.
    public static let defaultActivation = optionSpace

    public static let supported = [optionSpace, commandShiftSpace, commandSpace]

    public static func saved(identifier: String?) -> HotKeyDefinition {
        supported.first { $0.identifier == identifier } ?? .defaultActivation
    }

    public func alternatives() -> [HotKeyDefinition] {
        Self.supported.filter { $0 != self }
    }

    /// Four-character code identifying our hotkey to Carbon.
    public static let signature: UInt32 = 0x6C_75_6D_61 // 'luma'

    public func registrationRecoveryMessage(appPath: String) -> String {
        let conflictHint = self == .commandSpace
            ? """
              macOS usually reserves ⌘Space for Spotlight. To free it, open System Settings → \
              Keyboard → Keyboard Shortcuts → Spotlight, then disable or change “Show Spotlight search”.
              """
            : "Another application is already using \(displayName)."

        return """
        \(conflictHint)

        Choose one of Luma’s explicit alternatives below. Luma never changes the shortcut silently. \
        If you keep running without one, quit and reopen Luma from \(appPath).
        """
    }
}
