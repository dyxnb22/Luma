import CoreGraphics
import Foundation

public enum WindowLayoutPreset: String, Sendable, CaseIterable {
    case leftHalf
    case rightHalf
    case topHalf
    case bottomHalf
    case maximize
    case center
}

public enum WindowLayoutEngine {
    public static func frame(for preset: WindowLayoutPreset, screen: CGRect) -> CGRect {
        switch preset {
        case .leftHalf:
            return CGRect(x: screen.minX, y: screen.minY, width: screen.width / 2, height: screen.height)
        case .rightHalf:
            return CGRect(x: screen.midX, y: screen.minY, width: screen.width / 2, height: screen.height)
        case .topHalf:
            return CGRect(x: screen.minX, y: screen.midY, width: screen.width, height: screen.height / 2)
        case .bottomHalf:
            return CGRect(x: screen.minX, y: screen.minY, width: screen.width, height: screen.height / 2)
        case .maximize:
            return screen
        case .center:
            let width = screen.width * 0.72
            let height = screen.height * 0.72
            return CGRect(x: screen.midX - width / 2, y: screen.midY - height / 2, width: width, height: height)
        }
    }
}
