import AppKit
import LumaCore

@MainActor
final class LauncherListView: NSView {
    private let scrollView = NSScrollView()
    private let stack = FlippedStackView()
    private var rowViews: [NSView] = []
    private(set) var rows: [LauncherListRows.Row] = []
    private(set) var selectedFlatIndex = 0

    var onRun: ((ResultItem) -> Void)?
    var onRightClick: ((ResultItem) -> Void)?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false
        stack.orientation = .vertical
        stack.spacing = 4
        stack.alignment = .leading
        stack.translatesAutoresizingMaskIntoConstraints = false

        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = true
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        scrollView.documentView = stack

        addSubview(scrollView)
        NSLayoutConstraint.activate([
            scrollView.topAnchor.constraint(equalTo: topAnchor),
            scrollView.leadingAnchor.constraint(equalTo: leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: trailingAnchor),
            scrollView.bottomAnchor.constraint(equalTo: bottomAnchor),
            stack.topAnchor.constraint(equalTo: scrollView.contentView.topAnchor),
            stack.leadingAnchor.constraint(equalTo: scrollView.contentView.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: scrollView.contentView.trailingAnchor),
            stack.widthAnchor.constraint(equalTo: scrollView.contentView.widthAnchor)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    var currentItems: [ResultItem] {
        LauncherListRows.selectableItems(from: rows)
    }

    func renderHome(_ snapshot: LauncherHomeSnapshot) {
        apply(rows: LauncherListRows.rows(for: snapshot), preserveSelection: false)
    }

    func renderResults(_ items: [ResultItem], preserveSelectionID: ResultID? = nil) {
        let limited = Array(items.prefix(8))
        let previouslySelected = preserveSelectionID
        apply(rows: LauncherListRows.rows(for: limited), preserveSelection: previouslySelected != nil)
        if let previouslySelected,
           let index = currentItems.firstIndex(where: { $0.id == previouslySelected }) {
            updateSelection(to: index)
        }
    }

    func clear() {
        apply(rows: [], preserveSelection: false)
    }

    func updateSelection(to flatIndex: Int) {
        guard !currentItems.isEmpty else { return }
        let clamped = min(max(0, flatIndex), currentItems.count - 1)
        let oldIndex = selectedFlatIndex
        selectedFlatIndex = clamped
        updateRowHighlight(oldFlatIndex: oldIndex, newFlatIndex: clamped)
        scrollToSelected()
    }

    private func apply(rows newRows: [LauncherListRows.Row], preserveSelection: Bool) {
        let previousID = preserveSelection ? currentItems[safe: selectedFlatIndex]?.id : nil
        rows = newRows
        stack.arrangedSubviews.forEach { view in
            stack.removeArrangedSubview(view)
            view.removeFromSuperview()
        }
        rowViews.removeAll()

        for (index, row) in newRows.enumerated() {
            let view = makeView(for: row)
            rowViews.append(view)
            stack.addArrangedSubview(view)
            view.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
            if index > 0, isNestedChildRow(row), isNestedChildRow(newRows[index - 1]) {
                stack.setCustomSpacing(1, after: rowViews[index - 1])
            } else if index > 0, isNestedChildRow(row), isAppParentRow(newRows[index - 1]) {
                stack.setCustomSpacing(2, after: rowViews[index - 1])
            }
        }

        if let previousID, let restored = currentItems.firstIndex(where: { $0.id == previousID }) {
            selectedFlatIndex = restored
        } else {
            selectedFlatIndex = 0
        }
        updateRowHighlight(oldFlatIndex: nil, newFlatIndex: selectedFlatIndex)
        scrollView.contentView.scroll(to: NSPoint(x: 0, y: 0))
    }

    private func makeView(for row: LauncherListRows.Row) -> NSView {
        switch row.kind {
        case .sectionHeader(let title, let shortcutIndex):
            return LauncherSectionHeaderView(title: title, shortcutIndex: shortcutIndex)
        case .item(let item, let flatIndex):
            let listRow = LauncherListRow(
                item: item,
                moduleLabel: LauncherModuleLabel.shortName(for: item.id.module),
                isSelected: flatIndex == selectedFlatIndex,
                onRun: { [weak self] item in self?.onRun?(item) },
                onRightClick: { [weak self] item in self?.onRightClick?(item) }
            )
            return listRow
        case .placeholder(let text):
            return LauncherPlaceholderRow(text: text)
        }
    }

    private func isNestedChildRow(_ row: LauncherListRows.Row) -> Bool {
        guard case .item(let item, _) = row.kind else { return false }
        return item.listNest != .none
    }

    private func isAppParentRow(_ row: LauncherListRows.Row) -> Bool {
        guard case .item(let item, _) = row.kind else { return false }
        return item.id.module.rawValue == "luma.apps" && item.listNest == .none
    }

    private func updateRowHighlight(oldFlatIndex: Int?, newFlatIndex: Int) {
        for (rowIndex, row) in rows.enumerated() {
            guard case .item(_, let flatIndex) = row.kind,
                  let listRow = rowViews[rowIndex] as? LauncherListRow else { continue }
            if let oldFlatIndex, flatIndex == oldFlatIndex {
                listRow.setSelected(false)
            }
            if flatIndex == newFlatIndex {
                listRow.setSelected(true)
            }
        }
    }

    private func scrollToSelected() {
        guard let rowIndex = rows.firstIndex(where: {
            if case .item(_, let idx) = $0.kind { return idx == selectedFlatIndex }
            return false
        }), rowViews.indices.contains(rowIndex) else { return }
        rowViews[rowIndex].scrollToVisible(rowViews[rowIndex].bounds)
    }
}

private extension Array {
    subscript(safe index: Int) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
