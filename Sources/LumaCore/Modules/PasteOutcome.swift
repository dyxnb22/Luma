import Foundation

public enum PasteOutcome: Sendable, Equatable {
    /// Text was written to the pasteboard only (no insert attempted or not applicable).
    case copiedOnly
    /// Text was inserted into the frontmost app via Accessibility.
    case pasted
    /// Direct paste was requested but Accessibility is not trusted.
    case permissionRequired
}
