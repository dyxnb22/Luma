import CoreGraphics
import Foundation

/// Sendable CG window bounds used for matching AX windows without crossing actor boundaries.
public struct CGWindowBoundsInfo: Sendable, Equatable {
    public let windowID: UInt32
    public let pid: Int32
    public let title: String
    public let bounds: CGRect

    public init(windowID: UInt32, pid: Int32, title: String, bounds: CGRect) {
        self.windowID = windowID
        self.pid = pid
        self.title = title
        self.bounds = bounds
    }
}
