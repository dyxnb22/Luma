import Foundation

/// The global activation shortcut, isolated so it can become configurable later without
/// redesigning the controller around it.
///
/// Values are Carbon constants, repeated here so this file stays free of Carbon imports and
/// remains testable: `kVK_Space` is `0x31`, `cmdKey` is `1 << 8`.
public struct HotKeyDefinition: Equatable {
    public let keyCode: UInt32
    public let carbonModifiers: UInt32
    /// Human-readable form used in host-level error text.
    public let displayName: String

    public init(keyCode: UInt32, carbonModifiers: UInt32, displayName: String) {
        self.keyCode = keyCode
        self.carbonModifiers = carbonModifiers
        self.displayName = displayName
    }

    public static let commandSpace = HotKeyDefinition(
        keyCode: 0x31,
        carbonModifiers: 1 << 8,
        displayName: "⌘Space"
    )

    /// Four-character code identifying our hotkey to Carbon.
    public static let signature: UInt32 = 0x6C_75_6D_61 // 'luma'

    public func registrationRecoveryMessage(appPath: String) -> String {
        """
        To free \(displayName), open System Settings → Keyboard → Keyboard Shortcuts → Spotlight, \
        then disable or change “Show Spotlight search”.

        Quit and reopen Luma from \(appPath) to retry registration. Until then, keep this window \
        open or reopen the app from Finder.
        """
    }
}
