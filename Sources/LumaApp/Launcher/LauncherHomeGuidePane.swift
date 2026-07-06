@preconcurrency import AppKit
import LumaCore

/// Read-only command guide for empty-query home (right column). Not a second navigable list.
final class LauncherHomeGuidePane: NSView {
    private enum Column: String {
        case module
        case trigger
        case summary
    }

    private let footerLabel = NSTextField(labelWithString: "")
    private let scrollView = NSScrollView()
    private let tableView = NSTableView()
    private var rows: [HomeGuideEntryRow] = []

    /// When false, mouse events pass through (used during detail ↔ guide cross-fade).
    nonisolated(unsafe) var passesHitTests = true

    nonisolated override func hitTest(_ point: NSPoint) -> NSView? {
        guard passesHitTests, alphaValue > 0.01, !isHidden else { return nil }
        return super.hitTest(point)
    }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false
        clipsToBounds = true
        isHidden = true
        GeekUIKit.configureContentSurface(self)

        footerLabel.font = TypographyTokens.caption2
        footerLabel.textColor = .secondaryLabelColor.withAlphaComponent(0.72)
        footerLabel.lineBreakMode = .byTruncatingTail
        footerLabel.maximumNumberOfLines = 1
        footerLabel.translatesAutoresizingMaskIntoConstraints = false

        let headerView = NSTableHeaderView()
        headerView.frame.size.height = 28
        tableView.headerView = headerView
        GeekUIKit.configureDetailTable(tableView, rowHeight: LauncherChromeTokens.homeGuideTableRowHeight)
        tableView.intercellSpacing = NSSize(
            width: 0,
            height: LauncherChromeTokens.homeGuideTableRowSpacing
        )
        tableView.selectionHighlightStyle = .none
        tableView.allowsEmptySelection = true
        tableView.usesAlternatingRowBackgroundColors = false
        tableView.delegate = self
        tableView.dataSource = self

        for (id, title, width) in [
            (Column.module.rawValue, L10n.trZhHans("home.guide.col.module"), 78.0),
            (Column.trigger.rawValue, L10n.trZhHans("home.guide.col.trigger"), 52.0),
            (Column.summary.rawValue, L10n.trZhHans("home.guide.col.summary"), 200.0)
        ] {
            let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(id))
            column.title = title
            column.width = width
            GeekUIKit.configureDetailTableColumn(column, minWidth: width * 0.6)
            if id == Column.summary.rawValue {
                column.resizingMask = [.autoresizingMask]
            }
            tableView.addTableColumn(column)
        }

        scrollView.documentView = tableView
        GeekUIKit.configureVerticalListScroll(scrollView)
        scrollView.translatesAutoresizingMaskIntoConstraints = false

        addSubview(scrollView)
        addSubview(footerLabel)

        NSLayoutConstraint.activate([
            scrollView.topAnchor.constraint(equalTo: topAnchor, constant: 8),
            scrollView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            scrollView.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),

            footerLabel.topAnchor.constraint(equalTo: scrollView.bottomAnchor, constant: 2),
            footerLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            footerLabel.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            footerLabel.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -8)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    /// Always shows the module entry catalog — never mirrors the left Open Apps selection title.
    @MainActor
    func applyCatalog(_ commands: [CommandDefinition]) {
        footerLabel.stringValue = L10n.trZhHans("home.guide.footer")
        rows = HomeGuideCatalog.entryRows(from: commands) { L10n.trZhHans($0) }
        tableView.reloadData()
        layoutSubtreeIfNeeded()
        GeekUIKit.syncVerticalListDocumentFrame(in: scrollView)
    }
}

extension LauncherHomeGuidePane: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int { rows.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard rows.indices.contains(row), let tableColumn else { return nil }
        let entry = rows[row]
        let columnID = tableColumn.identifier.rawValue
        let text: String
        let font: NSFont
        let color: NSColor
        switch Column(rawValue: columnID) {
        case .module:
            text = entry.moduleName
            font = TypographyTokens.caption(weight: .semibold)
            color = .labelColor
        case .trigger:
            text = entry.trigger
            font = TypographyTokens.monoCaption(weight: .semibold)
            color = .secondaryLabelColor.withAlphaComponent(0.88)
        case .summary:
            text = entry.summary
            font = TypographyTokens.caption()
            color = .labelColor.withAlphaComponent(0.82)
        case .none:
            return nil
        }
        return GeekUIKit.makeDetailTableCell(text: text, font: font, color: color)
    }

    func tableView(_ tableView: NSTableView, shouldSelectRow row: Int) -> Bool { false }

    func tableView(_ tableView: NSTableView, rowViewForRow row: Int) -> NSTableRowView? {
        let rowView = GuideTableRowView()
        rowView.rowIndex = row
        return rowView
    }
}

@MainActor
private final class GuideTableRowView: NSTableRowView {
    nonisolated(unsafe) var rowIndex = -1

    nonisolated override func drawBackground(in dirtyRect: NSRect) {
        guard rowIndex >= 0 else { return }
        let fill: NSColor = rowIndex.isMultiple(of: 2)
            ? NSColor.quaternaryLabelColor.withAlphaComponent(0.35)
            : .clear
        guard fill != .clear else { return }
        fill.setFill()
        NSBezierPath(roundedRect: bounds.insetBy(dx: 2, dy: 1), xRadius: 6, yRadius: 6).fill()
    }
}
