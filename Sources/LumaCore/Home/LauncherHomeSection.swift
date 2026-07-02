import Foundation

public enum LauncherHomeSectionKind: String, Sendable, Hashable, CaseIterable {
    case setup
    case openApps
    case recentActions
    case continueFlow
    case create

    public var title: String {
        switch self {
        case .setup: L10n.tr("home.section.setup")
        case .openApps: L10n.tr("home.section.openApps")
        case .recentActions: L10n.tr("home.section.recent")
        case .continueFlow: L10n.tr("home.section.continue")
        case .create: L10n.tr("home.section.create")
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
