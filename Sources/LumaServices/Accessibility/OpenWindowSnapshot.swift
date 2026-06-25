import CoreGraphics
import Foundation

/// A single on-screen window belonging to a running application.
public struct OpenWindowSnapshot: Sendable, Hashable, Identifiable {
    public let windowID: UInt32
    public let pid: Int32
    public let title: String
    public let isMain: Bool
    public let isMinimized: Bool
    public let isFocused: Bool
    public let bounds: CGRect

    public var id: String { "\(pid)-\(windowID)-\(title)" }

    public init(
        windowID: UInt32,
        pid: Int32,
        title: String,
        isMain: Bool,
        isMinimized: Bool,
        isFocused: Bool = false,
        bounds: CGRect = .zero
    ) {
        self.windowID = windowID
        self.pid = pid
        self.title = title
        self.isMain = isMain
        self.isMinimized = isMinimized
        self.isFocused = isFocused
        self.bounds = bounds
    }
}
