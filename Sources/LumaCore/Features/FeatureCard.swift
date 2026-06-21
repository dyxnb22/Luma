import Foundation

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

    public init(
        id: ModuleIdentifier,
        title: String,
        subtitle: String,
        icon: IconRef,
        isEnabled: Bool = true,
        sizeMode: SizeMode = .normal,
        position: CardPosition = .zero,
        editAction: Action? = nil
    ) {
        self.id = id
        self.title = title
        self.subtitle = subtitle
        self.icon = icon
        self.isEnabled = isEnabled
        self.sizeMode = sizeMode
        self.position = position
        self.editAction = editAction
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
