import Foundation

public enum LauncherHomeSectionKind: String, Sendable, Hashable, CaseIterable {
    case openApps
    case recentActions
    case continueFlow
    case create

    public var title: String {
        switch self {
        case .openApps: "OPEN APPS"
        case .recentActions: "RECENT"
        case .continueFlow: "CONTINUE"
        case .create: "CREATE"
        }
    }
}

public struct LauncherHomeSection: Sendable, Equatable {
    public let kind: LauncherHomeSectionKind
    public let items: [ResultItem]

    public init(kind: LauncherHomeSectionKind, items: [ResultItem]) {
        self.kind = kind
        self.items = items
    }
}

public struct LauncherHomeSnapshot: Sendable, Equatable {
    public let sections: [LauncherHomeSection]

    public init(sections: [LauncherHomeSection]) {
        self.sections = sections
    }

    public static let empty = LauncherHomeSnapshot(sections: [])

    public var flatItems: [ResultItem] {
        sections.flatMap(\.items)
    }
}
