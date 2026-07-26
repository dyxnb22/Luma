import Foundation

/// Window sizing for the terminal view.
public enum TerminalGeometry {
    /// Restrained default; the real size is snapped down to whole cells before the window opens.
    public static let preferredContentSize = CGSize(width: 960, height: 700)

    /// Smallest window worth showing, in cells.
    public static let minimumColumns = 40
    public static let minimumRows = 12

    /// Rounds a content size down to a whole number of terminal cells so the last row and column
    /// are fully usable instead of clipped.
    ///
    /// - Parameter reservedWidth: non-cell width owned by the view (SwiftTerm's scroller).
    public static func integralContentSize(
        preferred: CGSize,
        cell: CGSize,
        reservedWidth: CGFloat = 0
    ) -> CGSize {
        guard cell.width > 0, cell.height > 0 else { return preferred }
        let usableWidth = max(0, preferred.width - reservedWidth)
        let columns = max(minimumColumns, Int(usableWidth / cell.width))
        let rows = max(minimumRows, Int(preferred.height / cell.height))
        return CGSize(
            width: CGFloat(columns) * cell.width + reservedWidth,
            height: CGFloat(rows) * cell.height
        )
    }
}
