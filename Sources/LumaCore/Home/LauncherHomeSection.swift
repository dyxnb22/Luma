import Foundation

public enum LauncherHomeSectionKind: String, Sendable, Hashable {
    case openApps

    public var title: String {
        switch self {
        case .openApps: L10n.tr("home.section.openApps")
        }
    }
}

public struct LauncherHomeSection: Sendable, Equatable {
    public let kind: LauncherHomeSectionKind
    public let items: [ResultItem]
    public let isWarming: Bool

    public init(kind: LauncherHomeSectionKind, items: [ResultItem], isWarming: Bool = false) {
        self.kind = kind
        self.items = items
        self.isWarming = isWarming
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
