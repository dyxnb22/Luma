import CoreGraphics
import Foundation

public enum WindowLayoutPreset: String, Sendable, CaseIterable {
    case leftHalf = "left-half"
    case rightHalf = "right-half"
    case topHalf = "top-half"
    case bottomHalf = "bottom-half"
    case maximize
    case center
}

public struct WindowLayoutCommand: Sendable, Hashable {
    public let preset: WindowLayoutPreset
    public let title: String
    public let aliases: [String]
    public let symbol: String

    public init(preset: WindowLayoutPreset, title: String, aliases: [String], symbol: String) {
        self.preset = preset
        self.title = title
        self.aliases = aliases
        self.symbol = symbol
    }
}

public enum WindowLayoutCatalog {
    public static let commands: [WindowLayoutCommand] = [
        WindowLayoutCommand(preset: .leftHalf, title: "Left Half", aliases: ["left", "left half", "lh"], symbol: "rectangle.leadinghalf.inset.filled"),
        WindowLayoutCommand(preset: .rightHalf, title: "Right Half", aliases: ["right", "right half", "rh"], symbol: "rectangle.trailinghalf.inset.filled"),
        WindowLayoutCommand(preset: .topHalf, title: "Top Half", aliases: ["top", "top half"], symbol: "rectangle.tophalf.inset.filled"),
        WindowLayoutCommand(preset: .bottomHalf, title: "Bottom Half", aliases: ["bottom", "bottom half"], symbol: "rectangle.bottomhalf.inset.filled"),
        WindowLayoutCommand(preset: .maximize, title: "Maximize", aliases: ["max", "full", "fullscreen"], symbol: "arrow.up.left.and.arrow.down.right"),
        WindowLayoutCommand(preset: .center, title: "Center", aliases: ["centre", "middle"], symbol: "rectangle.inset.filled")
    ]

    public static func matching(payload: String) -> [WindowLayoutCommand] {
        let trimmed = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return commands }
        let lowered = trimmed.lowercased()
        return commands.filter { command in
            if command.title.lowercased().contains(lowered) { return true }
            if command.preset.rawValue.contains(lowered) { return true }
            return command.aliases.contains { alias in
                alias.contains(lowered) || lowered.contains(alias)
            }
        }
    }
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
