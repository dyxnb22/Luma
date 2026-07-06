import Foundation

public enum LauncherListRows {
    public struct Row: Equatable, Sendable {
        public enum Kind: Equatable, Sendable {
            case sectionHeader(title: String, shortcutIndex: Int?)
            case item(ResultItem, flatIndex: Int)
            case placeholder(String)
        }

        public let kind: Kind

        public init(kind: Kind) {
            self.kind = kind
        }
    }

    public static func rows(for snapshot: LauncherHomeSnapshot) -> [Row] {
        var result: [Row] = []
        var flatIndex = 0
        for section in snapshot.sections where !section.items.isEmpty {
            let shortcut = flatIndex + 1
            result.append(.init(kind: .sectionHeader(title: section.kind.title, shortcutIndex: shortcut)))
            for item in section.items {
                result.append(.init(kind: .item(item, flatIndex: flatIndex)))
                flatIndex += 1
            }
        }
        if result.isEmpty,
           let openApps = snapshot.sections.first(where: { $0.kind == .openApps }) {
            result.append(.init(kind: .sectionHeader(title: openApps.kind.title, shortcutIndex: nil)))
            result.append(.init(kind: .placeholder(L10n.tr("module.warming"))))
            return result
        }
        if result.isEmpty {
            result.append(.init(kind: .placeholder("Type to search apps, clipboard, translate…")))
        }
        return result
    }

    public static let noMatchingResultsMessage = "No matching results. Press Esc to clear, or try a command prefix (clip, note, t, ?)."

    public static func rows(for results: [ResultItem], layout: ResultListLayout = .flat) -> [Row] {
        switch layout {
        case .flat:
            if results.isEmpty {
                return [Row(kind: .placeholder(noMatchingResultsMessage))]
            }
            return results.enumerated().map { index, item in
                Row(kind: .item(item, flatIndex: index))
            }
        case .sectioned(let sections):
            let sectionRows = rows(for: sections)
            if sectionRows.isEmpty {
                return [Row(kind: .placeholder(noMatchingResultsMessage))]
            }
            return sectionRows
        }
    }

    public static func rows(for sections: [ResultSection]) -> [Row] {
        var result: [Row] = []
        var flatIndex = 0
        for section in sections where !section.items.isEmpty {
            let shortcut = flatIndex + 1
            result.append(.init(kind: .sectionHeader(title: section.title, shortcutIndex: shortcut)))
            for item in section.items {
                result.append(.init(kind: .item(item, flatIndex: flatIndex)))
                flatIndex += 1
            }
        }
        return result
    }

    public static func selectableItems(from rows: [Row]) -> [ResultItem] {
        rows.compactMap { row in
            if case .item(let item, _) = row.kind { return item }
            return nil
        }
    }
}
