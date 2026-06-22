import Foundation

public struct WidgetCardStyle: Sendable, Hashable {
    public let symbolName: String
    public let topHex: String
    public let bottomHex: String

    public init(symbolName: String, topHex: String, bottomHex: String) {
        self.symbolName = symbolName
        self.topHex = topHex
        self.bottomHex = bottomHex
    }
}

public struct FeatureCard: Identifiable, Sendable, Hashable {
    public enum SizeMode: String, Sendable, Codable {
        case compact
        case normal
        case expanded
    }

    public let id: ModuleIdentifier
    public var title: String
    public var subtitle: String
    public var icon: IconRef
    public var isEnabled: Bool
    public var sizeMode: SizeMode
    public var position: CardPosition
    public var editAction: Action?
    public var widgetStyle: WidgetCardStyle?

    public let triggerKeyword: String

    public init(
        id: ModuleIdentifier,
        title: String,
        subtitle: String,
        icon: IconRef,
        triggerKeyword: String = "",
        isEnabled: Bool = true,
        sizeMode: SizeMode = .normal,
        position: CardPosition = .zero,
        editAction: Action? = nil,
        widgetStyle: WidgetCardStyle? = nil
    ) {
        self.id = id
        self.title = title
        self.subtitle = subtitle
        self.icon = icon
        self.triggerKeyword = triggerKeyword
        self.isEnabled = isEnabled
        self.sizeMode = sizeMode
        self.position = position
        self.editAction = editAction
        self.widgetStyle = widgetStyle
    }
}

public struct CardPosition: Sendable, Hashable, Codable {
    public var column: Int
    public var row: Int
    public var zIndex: Int

    public init(column: Int, row: Int, zIndex: Int = 0) {
        self.column = column
        self.row = row
        self.zIndex = zIndex
    }

    public static let zero = CardPosition(column: 0, row: 0, zIndex: 0)
}
