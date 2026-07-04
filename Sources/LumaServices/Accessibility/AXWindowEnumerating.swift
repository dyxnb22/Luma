import Foundation

/// Injectable window enumerator for Open Apps home and unit tests.
public protocol AXWindowEnumerating: Sendable {
    var isAccessibilityGranted: Bool { get }
    /// One CGWindowListCopyWindowInfo pass, grouped by owner PID.
    func copyOnScreenWindowsByPID() -> [Int32: [CGWindowBoundsInfo]]
    func enumerateWindows(for pid: Int32, appName: String, cgWindows: [CGWindowBoundsInfo]) -> [OpenWindowSnapshot]
}

public struct LiveAXWindowEnumerator: AXWindowEnumerating {
    public init() {}

    public var isAccessibilityGranted: Bool {
        AXService.isProcessTrusted()
    }

    public func copyOnScreenWindowsByPID() -> [Int32: [CGWindowBoundsInfo]] {
        AXService.copyOnScreenWindowsByPID()
    }

    public func enumerateWindows(for pid: Int32, appName: String, cgWindows: [CGWindowBoundsInfo]) -> [OpenWindowSnapshot] {
        AXService.enumerateWindows(for: pid, appName: appName, cgWindows: cgWindows)
    }
}
