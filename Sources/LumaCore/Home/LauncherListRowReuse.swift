import Foundation

public enum LauncherListRowReuse {
    /// True when row kinds and stable identities match in order — safe to reuse `LauncherListRow` views.
    public static func canReuseRows(_ oldRows: [LauncherListRows.Row], _ newRows: [LauncherListRows.Row]) -> Bool {
        guard oldRows.count == newRows.count else { return false }
        for (oldRow, newRow) in zip(oldRows, newRows) {
            guard identityKey(for: oldRow) == identityKey(for: newRow) else { return false }
            guard structurallyCompatible(oldRow, newRow) else { return false }
        }
        return true
    }

    /// True when identities are unchanged but row order changed — views can be reordered in place.
    public static func canReorderRows(_ oldRows: [LauncherListRows.Row], _ newRows: [LauncherListRows.Row]) -> Bool {
        guard oldRows.count == newRows.count else { return false }
        guard oldRows.map(identityKey(for:)) != newRows.map(identityKey(for:)) else { return false }
        guard oldRows.map(identityKey(for:)).sorted() == newRows.map(identityKey(for:)).sorted() else { return false }
        let oldByKey = Dictionary(uniqueKeysWithValues: oldRows.map { (identityKey(for: $0), $0) })
        for newRow in newRows {
            guard let oldRow = oldByKey[identityKey(for: newRow)] else { return false }
            guard structurallyCompatible(oldRow, newRow) else { return false }
        }
        return true
    }

    public static func identityKey(for row: LauncherListRows.Row) -> String {
        switch row.kind {
        case .sectionHeader(let title, _):
            return "h:\(title)"
        case .item(let item, _):
            return "i:\(item.id.module.rawValue):\(item.id.key)"
        case .placeholder(let text):
            return "p:\(text)"
        }
    }

    private static func structurallyCompatible(
        _ oldRow: LauncherListRows.Row,
        _ newRow: LauncherListRows.Row
    ) -> Bool {
        switch (oldRow.kind, newRow.kind) {
        case (.item(let oldItem, _), .item(let newItem, _)):
            return oldItem.listNest == newItem.listNest
                && oldItem.displayDensity == newItem.displayDensity
        case (.sectionHeader, .sectionHeader), (.placeholder, .placeholder):
            return true
        default:
            return false
        }
    }
}
