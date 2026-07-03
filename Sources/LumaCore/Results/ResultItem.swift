import Foundation
import CoreGraphics

public struct ResultID: Hashable, Sendable, Codable {
    public let module: ModuleIdentifier
    public let key: String

    public init(module: ModuleIdentifier, key: String) {
        self.module = module
        self.key = key
    }
}

public enum ResultDisplayDensity: Sendable, Hashable, Codable {
    case compact
    case regular
    case expanded
}

public enum ResultListNest: Sendable, Hashable, Codable {
    case none
    case child(isLast: Bool)
}

public enum ResultRowKind: Sendable, Hashable, Codable {
    /// Return runs a side effect or opens detail.
    case actionable
    /// Status or help text; Return does not dismiss the launcher.
    case informational
}

public struct ResultItem: Identifiable, Sendable, Hashable {
    public let id: ResultID
    public let title: String
    public let titleAttributed: AttributedString
    public let subtitle: String?
    public let icon: IconRef
    public let primaryAction: Action
    public let secondaryActions: [Action]
    public var rankingHints: RankingHints
    public var displayDensity: ResultDisplayDensity
    public var listNest: ResultListNest
    public var rowKind: ResultRowKind

    public init(
        id: ResultID,
        title: String,
        titleAttributed: AttributedString,
        subtitle: String? = nil,
        icon: IconRef,
        primaryAction: Action,
        secondaryActions: [Action] = [],
        rankingHints: RankingHints,
        displayDensity: ResultDisplayDensity = .regular,
        listNest: ResultListNest = .none,
        rowKind: ResultRowKind = .actionable
    ) {
        self.id = id
        self.title = title
        self.titleAttributed = titleAttributed
        self.subtitle = subtitle
        self.icon = icon
        self.primaryAction = primaryAction
        self.secondaryActions = secondaryActions
        self.rankingHints = rankingHints
        self.displayDensity = displayDensity
        self.listNest = listNest
        self.rowKind = rowKind
    }
}

public enum IconRef: Sendable, Hashable, Codable {
    case bundleID(String)
    case symbol(String)
    case file(URL)
    case none
}

public struct ResultSnapshot: Sendable, Equatable {
    public let querySequence: UInt64
    public let items: [ResultItem]
    public let layout: ResultListLayout

    public init(querySequence: UInt64, items: [ResultItem], layout: ResultListLayout = .flat) {
        self.querySequence = querySequence
        self.items = items
        self.layout = layout
    }

    public static let empty = ResultSnapshot(querySequence: 0, items: [])
}

public struct UsageRecord: Sendable, Codable, Hashable {
    public let id: ResultID
    public var count: Int
    public var lastUsed: Date

    public init(id: ResultID, count: Int, lastUsed: Date) {
        self.id = id
        self.count = count
        self.lastUsed = lastUsed
    }
}

public struct RankingHints: Sendable, Hashable {
    public var basePriority: Int
    public var fuzzyScore: Double
    public var frequency: Int
    public var lastUsed: Date?
    public var finalScore: Double

    public init(
        basePriority: Int = 0,
        fuzzyScore: Double = 0,
        frequency: Int = 0,
        lastUsed: Date? = nil,
        finalScore: Double = 0
    ) {
        self.basePriority = basePriority
        self.fuzzyScore = fuzzyScore
        self.frequency = frequency
        self.lastUsed = lastUsed
        self.finalScore = finalScore
    }
}
