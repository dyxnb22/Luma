import AppKit
import LumaCore

@MainActor
final class LauncherListView: NSView {
    private let scrollView = NSScrollView()
    private let stack = FlippedStackView()
    private var rowViews: [NSView] = []
    private(set) var rows: [LauncherListRows.Row] = []
    private var lastRenderedHomeSnapshot: LauncherHomeSnapshot?
    private(set) var selectedFlatIndex = 0
    private(set) var currentLayout: ResultListLayout = .flat

    var onRun: ((ResultItem) -> Void)?
    var onRightClick: ((ResultItem) -> Void)?
    var onSelectionChanged: ((Int) -> Void)?
    var onKeyCommand: ((LumaSearchBar.KeyCommand) -> Bool)?
    var onInterceptKeyDown: ((NSEvent) -> Bool)?
    var onActivate: (() -> Void)?
    var onEscape: (() -> Void)?
    var onTypeToSearch: ((String) -> Void)?

    /// When false, mouse events pass through (used during detail cross-fade).
    var passesHitTests = true

    private(set) var compactHomeColumn = false {
        didSet {
            guard oldValue != compactHomeColumn else { return }
            refreshCompactRowChrome()
        }
    }

    func setCompactHomeColumn(_ compact: Bool) {
        compactHomeColumn = compact
    }

    override var acceptsFirstResponder: Bool { !currentItems.isEmpty }

    override func hitTest(_ point: NSPoint) -> NSView? {
        guard passesHitTests, alphaValue > 0.01, !isHidden else { return nil }
        return super.hitTest(point)
    }

    func focusList() {
        window?.makeFirstResponder(self)
    }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false
        clipsToBounds = true
        setContentHuggingPriority(.defaultLow, for: .horizontal)
        setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        stack.orientation = .vertical
        stack.spacing = LauncherChromeTokens.listRowSpacing
        stack.alignment = .leading
        stack.translatesAutoresizingMaskIntoConstraints = false

        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = false
        scrollView.autohidesScrollers = true
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
        scrollView.clipsToBounds = true
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

    func renderHome(_ snapshot: LauncherHomeSnapshot, preserveSelection: Bool = false) {
        currentLayout = .flat
        if !preserveSelection, snapshot == lastRenderedHomeSnapshot { return }
        lastRenderedHomeSnapshot = snapshot
        apply(rows: LauncherListRows.rows(for: snapshot), preserveSelection: preserveSelection)
    }

    func renderResults(_ items: [ResultItem], layout: ResultListLayout = .flat, preserveSelectionID: ResultID? = nil) {
        let limited = Array(items.prefix(8))
        let previouslySelected = preserveSelectionID
        currentLayout = layout
        apply(rows: LauncherListRows.rows(for: limited, layout: layout), preserveSelection: previouslySelected != nil)
        if let previouslySelected,
           let index = currentItems.firstIndex(where: { $0.id == previouslySelected }) {
            updateSelection(to: index)
        }
    }

    func clear() {
        currentLayout = .flat
        lastRenderedHomeSnapshot = nil
        apply(rows: [], preserveSelection: false)
    }

    func updateSelection(to flatIndex: Int) {
        guard !currentItems.isEmpty else { return }
        let clamped = min(max(0, flatIndex), currentItems.count - 1)
        selectedFlatIndex = clamped
        updateRowHighlight(newFlatIndex: clamped)
        scrollToSelected()
        onSelectionChanged?(clamped)
    }

    override func mouseDown(with event: NSEvent) {
        focusList()
        super.mouseDown(with: event)
    }

    override func keyDown(with event: NSEvent) {
        if onInterceptKeyDown?(event) == true { return }
        if shouldForwardTyping(event), let text = event.characters, !text.isEmpty {
            onTypeToSearch?(text)
            return
        }
        if let command = mappedKeyCommand(event), onKeyCommand?(command) == true { return }
        if event.keyCode == 36 {
            onActivate?()
            return
        }
        super.keyDown(with: event)
    }

    override func insertText(_ insertString: Any) {
        let text: String
        if let string = insertString as? String {
            text = string
        } else if let attributed = insertString as? NSAttributedString {
            text = attributed.string
        } else {
            super.insertText(insertString)
            return
        }
        guard !text.isEmpty else { return }
        onTypeToSearch?(text)
    }

    private func shouldForwardTyping(_ event: NSEvent) -> Bool {
        guard !event.modifierFlags.contains(.command),
              !event.modifierFlags.contains(.control),
              !event.modifierFlags.contains(.option) else { return false }
        guard let chars = event.characters, chars.count == 1 else { return false }
        let scalar = chars.unicodeScalars.first!
        return CharacterSet.alphanumerics.contains(scalar)
            || CharacterSet.punctuationCharacters.contains(scalar)
            || CharacterSet.symbols.contains(scalar)
            || CharacterSet.whitespaces.contains(scalar)
    }

    override func cancelOperation(_ sender: Any?) {
        onEscape?()
    }

    private func mappedKeyCommand(_ event: NSEvent) -> LumaSearchBar.KeyCommand? {
        switch event.keyCode {
        case 125: return .down
        case 126: return .up
        case 48:
            return event.modifierFlags.contains(.shift) ? .backtab : .tab
        default:
            break
        }
        if event.modifierFlags.contains(.command),
           event.keyCode == 36 {
            return .commandReturn
        }
        if event.modifierFlags.contains(.command),
           event.charactersIgnoringModifiers?.lowercased() == "k" {
            return .actionPanel
        }
        if event.modifierFlags.contains(.command),
           let chars = event.charactersIgnoringModifiers,
           let number = Int(chars),
           (1...9).contains(number) {
            return .commandNumber(number)
        }
        return nil
    }

    private func apply(rows newRows: [LauncherListRows.Row], preserveSelection: Bool) {
        let previousID = preserveSelection ? currentItems[safe: selectedFlatIndex]?.id : nil
        let selectable = LauncherListRows.selectableItems(from: newRows)
        let nextSelectedFlatIndex: Int
        if let previousID, let restored = selectable.firstIndex(where: { $0.id == previousID }) {
            nextSelectedFlatIndex = restored
        } else {
            nextSelectedFlatIndex = 0
        }
        let clampedSelected = selectable.isEmpty
            ? 0
            : min(max(0, nextSelectedFlatIndex), selectable.count - 1)

        if LauncherListRowReuse.canReuseRows(rows, newRows) {
            rows = newRows
            for (rowIndex, newRow) in newRows.enumerated() {
                guard case .item(let item, let flatIndex) = newRow.kind,
                      let listRow = rowViews[rowIndex] as? LauncherListRow else { continue }
                listRow.update(
                    item: item,
                    isSelected: flatIndex == clampedSelected,
                    compactColumn: compactHomeColumn,
                    hidesTrailingModuleLabel: compactHomeColumn
                        && item.id.module.rawValue == "luma.apps"
                        && item.listNest == .none,
                    onRun: { [weak self] item in self?.onRun?(item) },
                    onRightClick: { [weak self] item in self?.onRightClick?(item) },
                    onHover: { [weak self] in self?.updateSelection(to: flatIndex) }
                )
            }
            selectedFlatIndex = clampedSelected
            GeekUIKit.syncVerticalListDocumentFrame(in: scrollView)
            return
        }

        if LauncherListRowReuse.canReorderRows(rows, newRows) {
            reorderRowViews(toMatch: newRows, selectedFlatIndex: clampedSelected)
            rows = newRows
            selectedFlatIndex = clampedSelected
            updateRowHighlight(newFlatIndex: clampedSelected)
            GeekUIKit.syncVerticalListDocumentFrame(in: scrollView)
            return
        }

        rows = newRows
        stack.arrangedSubviews.forEach { view in
            stack.removeArrangedSubview(view)
            view.removeFromSuperview()
        }
        rowViews.removeAll()

        for (index, row) in newRows.enumerated() {
            let view = makeView(for: row, selectedFlatIndex: clampedSelected)
            rowViews.append(view)
            stack.addArrangedSubview(view)
            view.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
            if index > 0, isNestedChildRow(row), isNestedChildRow(newRows[index - 1]) {
                stack.setCustomSpacing(1, after: rowViews[index - 1])
            } else if index > 0, isNestedChildRow(row), isAppParentRow(newRows[index - 1]) {
                stack.setCustomSpacing(2, after: rowViews[index - 1])
            }
        }

        selectedFlatIndex = clampedSelected
        updateRowHighlight(newFlatIndex: clampedSelected)
        scrollView.contentView.scroll(to: NSPoint(x: 0, y: 0))
        GeekUIKit.syncVerticalListDocumentFrame(in: scrollView)
    }

    private func reorderRowViews(toMatch newRows: [LauncherListRows.Row], selectedFlatIndex: Int) {
        var viewsByKey: [String: NSView] = [:]
        for (index, row) in rows.enumerated() {
            viewsByKey[LauncherListRowReuse.identityKey(for: row)] = rowViews[index]
        }

        stack.arrangedSubviews.forEach { view in
            stack.removeArrangedSubview(view)
            view.removeFromSuperview()
        }
        rowViews.removeAll()

        for (index, row) in newRows.enumerated() {
            let key = LauncherListRowReuse.identityKey(for: row)
            let reusedView = viewsByKey[key]
            let view = reusedView ?? makeView(for: row, selectedFlatIndex: selectedFlatIndex)
            rowViews.append(view)
            stack.addArrangedSubview(view)
            if reusedView == nil {
                view.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
            }
            if index > 0, isNestedChildRow(row), isNestedChildRow(newRows[index - 1]) {
                stack.setCustomSpacing(1, after: rowViews[index - 1])
            } else if index > 0, isNestedChildRow(row), isAppParentRow(newRows[index - 1]) {
                stack.setCustomSpacing(2, after: rowViews[index - 1])
            }
            if case .item(let item, let flatIndex) = row.kind, let listRow = view as? LauncherListRow {
                listRow.update(
                    item: item,
                    isSelected: flatIndex == selectedFlatIndex,
                    compactColumn: compactHomeColumn,
                    hidesTrailingModuleLabel: compactHomeColumn
                        && item.id.module.rawValue == "luma.apps"
                        && item.listNest == .none,
                    onRun: { [weak self] item in self?.onRun?(item) },
                    onRightClick: { [weak self] item in self?.onRightClick?(item) },
                    onHover: { [weak self] in self?.updateSelection(to: flatIndex) }
                )
            }
        }
    }

    private func makeView(for row: LauncherListRows.Row, selectedFlatIndex: Int) -> NSView {
        switch row.kind {
        case .sectionHeader(let title, let shortcutIndex):
            return LauncherSectionHeaderView(title: title, shortcutIndex: shortcutIndex)
        case .item(let item, let flatIndex):
            let listRow = LauncherListRow(
                item: item,
                moduleLabel: LauncherModuleLabel.shortName(for: item.id.module),
                isSelected: flatIndex == selectedFlatIndex,
                compactColumn: compactHomeColumn,
                hidesTrailingModuleLabel: compactHomeColumn && item.id.module.rawValue == "luma.apps" && item.listNest == .none,
                onRun: { [weak self] item in self?.onRun?(item) },
                onRightClick: { [weak self] item in self?.onRightClick?(item) },
                onHover: { [weak self] in self?.updateSelection(to: flatIndex) }
            )
            return listRow
        case .placeholder(let text):
            return LauncherPlaceholderRow(text: text)
        }
    }

    private func refreshCompactRowChrome() {
        for (rowIndex, row) in rows.enumerated() {
            guard case .item(let item, _) = row.kind,
                  let listRow = rowViews[rowIndex] as? LauncherListRow else { continue }
            let hide = compactHomeColumn && item.id.module.rawValue == "luma.apps" && item.listNest == .none
            listRow.setCompactColumn(compactHomeColumn)
            listRow.setHidesTrailingModuleLabel(hide)
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

    private func updateRowHighlight(newFlatIndex: Int) {
        for (rowIndex, row) in rows.enumerated() {
            guard case .item(_, let flatIndex) = row.kind,
                  let listRow = rowViews[rowIndex] as? LauncherListRow else { continue }
            listRow.setSelected(flatIndex == newFlatIndex)
        }
    }

    func selectedRowAnchorView() -> NSView? {
        guard let rowIndex = rows.firstIndex(where: {
            if case .item(_, let idx) = $0.kind { return idx == selectedFlatIndex }
            return false
        }), rowViews.indices.contains(rowIndex) else { return nil }
        return rowViews[rowIndex]
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
