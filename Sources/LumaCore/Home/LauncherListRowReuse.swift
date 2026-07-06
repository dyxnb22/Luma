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
        let oldKeys = oldRows.map(identityKey(for:))
        let newKeys = newRows.map(identityKey(for:))
        guard oldKeys != newKeys else { return false }
        guard Set(oldKeys).count == oldKeys.count, Set(newKeys).count == newKeys.count else { return false }
        guard oldKeys.sorted() == newKeys.sorted() else { return false }
        var oldByKey: [String: LauncherListRows.Row] = [:]
        oldByKey.reserveCapacity(oldRows.count)
        for row in oldRows {
            let key = identityKey(for: row)
            guard oldByKey[key] == nil else { return false }
            oldByKey[key] = row
        }
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
