import Foundation

public enum LauncherListRowReuse {
    /// True when row kinds and stable identities match in order — safe to reuse `LauncherListRow` views.
    public static func canReuseRows(_ oldRows: [LauncherListRows.Row], _ newRows: [LauncherListRows.Row]) -> Bool {
        guard oldRows.count == newRows.count else { return false }
        for (oldRow, newRow) in zip(oldRows, newRows) {
            guard reuseKey(for: oldRow) == reuseKey(for: newRow) else { return false }
            guard structurallyCompatible(oldRow, newRow) else { return false }
        }
        return true
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

    private static func reuseKey(for row: LauncherListRows.Row) -> String {
        switch row.kind {
        case .sectionHeader(let title, _):
            return "h:\(title)"
        case .item(let item, let flatIndex):
            return "i:\(item.id.module.rawValue):\(item.id.key):\(flatIndex)"
        case .placeholder(let text):
            return "p:\(text)"
        }
    }
}
