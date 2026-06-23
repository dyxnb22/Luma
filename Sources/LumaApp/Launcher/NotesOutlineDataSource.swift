import AppKit
import LumaModules

@MainActor
enum NotesOutlineDisplayMode {
    case directory
    case tree
}

@MainActor
final class NotesOutlineItem: NSObject {
    let node: NotesNode
    let children: [NotesOutlineItem]
    let isRecentGroup: Bool
    let depth: Int
    let ancestorLastFlags: [Bool]
    let isLast: Bool

    init(
        node: NotesNode,
        children: [NotesOutlineItem]? = nil,
        isRecentGroup: Bool = false,
        depth: Int = 0,
        ancestorLastFlags: [Bool] = [],
        isLast: Bool = true
    ) {
        self.node = node
        self.isRecentGroup = isRecentGroup
        self.depth = depth
        self.ancestorLastFlags = ancestorLastFlags
        self.isLast = isLast
        if let children {
            self.children = children
        } else {
            self.children = node.children.enumerated().map { index, child in
                NotesOutlineItem(
                    node: child,
                    depth: depth + 1,
                    ancestorLastFlags: ancestorLastFlags + [isLast],
                    isLast: index == node.children.count - 1
                )
            }
        }
        super.init()
    }
}

@MainActor
final class NotesOutlineDataSource: NSObject, NSOutlineViewDataSource, NSOutlineViewDelegate {
    var rootItem: NotesOutlineItem?
    var recentGroupItem: NotesOutlineItem?
    var displayRoot: NotesOutlineItem?
    var displayMode: NotesOutlineDisplayMode = .directory
    var filterText = ""
    var onActivate: ((NotesNode) -> Void)?
    var onExpansionChanged: ((NotesNode, Bool) -> Void)?
    private var flatListTitle: String?
    private var typeByPath: [String: String] = [:]
    private var flatMatches: [NotesOutlineItem] = []
    private var matchIndex = 0

    func setTypeLabels(_ labels: [String: String]) {
        typeByPath = labels
    }

    func showTree(root: NotesNode?, recentNodes: [NotesNode] = []) {
        flatListTitle = nil
        reload(root: root, recentNodes: recentNodes)
    }

    func showFlatList(title: String, nodes: [NotesNode]) {
        flatListTitle = title
        let children = nodes.map { NotesNode(path: $0.path, name: $0.name, kind: .note, children: []) }
        let groupNode = NotesNode(path: "__flat__", name: title, kind: .folder, children: children)
        recentGroupItem = nil
        rootItem = nil
        displayRoot = NotesOutlineItem(node: groupNode)
        flatMatches = []
    }

    func showGroupedList(groups: [(String, [NotesNode])]) {
        flatListTitle = "Browse"
        let children = groups.map { title, nodes in
            let noteChildren = nodes.map { NotesNode(path: $0.path, name: $0.name, kind: .note, children: []) }
            return NotesOutlineItem(
                node: NotesNode(path: "__group__\(title)", name: title, kind: .folder, children: noteChildren)
            )
        }
        let virtualRoot = NotesNode(path: "__virtual_root__", name: "", kind: .folder, children: [])
        displayRoot = NotesOutlineItem(node: virtualRoot, children: children)
        rootItem = nil
        recentGroupItem = nil
        flatMatches = []
    }

    func reload(root: NotesNode?, recentNodes: [NotesNode] = []) {
        rootItem = root.map { NotesOutlineItem(node: $0) }
        if recentNodes.isEmpty {
            recentGroupItem = nil
        } else {
            let children = recentNodes.map {
                NotesNode(path: $0.path, name: $0.name, kind: .note, children: [])
            }
            let groupNode = NotesNode(path: "__recent__", name: "Recent", kind: .folder, children: children)
            recentGroupItem = NotesOutlineItem(node: groupNode, isRecentGroup: true)
        }
        applyFilter()
    }

    func setFilter(_ text: String) {
        filterText = text.trimmingCharacters(in: .whitespacesAndNewlines)
        applyFilter()
    }

    func selectFirstMatch(in outlineView: NSOutlineView) {
        guard !flatMatches.isEmpty else { return }
        matchIndex = 0
        selectMatch(at: matchIndex, in: outlineView)
    }

    func selectNextMatch(in outlineView: NSOutlineView) {
        guard !flatMatches.isEmpty else { return }
        matchIndex = (matchIndex + 1) % flatMatches.count
        selectMatch(at: matchIndex, in: outlineView)
    }

    private func selectMatch(at index: Int, in outlineView: NSOutlineView) {
        let item = flatMatches[index]
        let row = outlineView.row(forItem: item)
        if row >= 0 {
            outlineView.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
            outlineView.scrollRowToVisible(row)
        }
    }

    private func applyFilter() {
        guard let rootItem else {
            displayRoot = recentGroupItem
            rebuildMatches()
            return
        }

        if filterText.isEmpty {
            displayRoot = rootItem
            if let recentGroupItem {
                displayRoot = NotesOutlineItem(
                    node: NotesNode(path: "__virtual_root__", name: "", kind: .folder, children: []),
                    children: [recentGroupItem, rootItem]
                )
            }
            flatMatches = []
            return
        }

        let filtered = filterTree(node: rootItem.node, query: filterText)
        displayRoot = filtered.map { NotesOutlineItem(node: $0) }
        rebuildMatches()
    }

    private func rebuildMatches() {
        guard let displayRoot, !filterText.isEmpty else {
            flatMatches = []
            matchIndex = 0
            return
        }
        flatMatches = collectMatches(in: displayRoot)
        matchIndex = 0
    }

    private func collectMatches(in item: NotesOutlineItem) -> [NotesOutlineItem] {
        var results: [NotesOutlineItem] = []
        if item.node.kind == .note || (item.node.kind == .folder && !item.isRecentGroup && item.node.path != "__virtual_root__") {
            if item.node.name.localizedCaseInsensitiveContains(filterText) {
                results.append(item)
            }
        }
        for child in item.children {
            results.append(contentsOf: collectMatches(in: child))
        }
        return results
    }

    private func filterTree(node: NotesNode, query: String) -> NotesNode? {
        let lowered = query.lowercased()
        let nameMatches = node.name.lowercased().contains(lowered)
        let filteredChildren = node.children.compactMap { filterTree(node: $0, query: query) }

        if node.kind == .folder {
            if nameMatches || !filteredChildren.isEmpty {
                return NotesNode(path: node.path, name: node.name, kind: node.kind, children: filteredChildren)
            }
            return nil
        }

        return nameMatches ? node : nil
    }

    func outlineView(_ outlineView: NSOutlineView, numberOfChildrenOfItem item: Any?) -> Int {
        guard let item = item as? NotesOutlineItem else {
            guard let displayRoot else { return 0 }
            if displayRoot.node.path == "__virtual_root__" {
                return displayRoot.children.count
            }
            return 1
        }
        return item.children.count
    }

    func outlineView(_ outlineView: NSOutlineView, isItemExpandable item: Any) -> Bool {
        guard let item = item as? NotesOutlineItem else { return true }
        return item.node.kind == .folder
    }

    func outlineView(_ outlineView: NSOutlineView, child index: Int, ofItem item: Any?) -> Any {
        if let item = item as? NotesOutlineItem {
            return item.children[index]
        }
        if let displayRoot, displayRoot.node.path == "__virtual_root__" {
            return displayRoot.children[index]
        }
        return displayRoot!
    }

    func outlineView(_ outlineView: NSOutlineView, viewFor tableColumn: NSTableColumn?, item: Any) -> NSView? {
        guard let item = item as? NotesOutlineItem else { return nil }
        let identifier = NSUserInterfaceItemIdentifier("NotesOutlineCell")
        let cell: NSTableCellView
        if let reused = outlineView.makeView(withIdentifier: identifier, owner: self) as? NSTableCellView {
            cell = reused
        } else {
            cell = NSTableCellView()
            cell.identifier = identifier
            let imageView = NSImageView()
            imageView.translatesAutoresizingMaskIntoConstraints = false
            imageView.imageScaling = .scaleProportionallyDown
            let textField = NSTextField(labelWithString: "")
            textField.translatesAutoresizingMaskIntoConstraints = false
            textField.font = .systemFont(ofSize: 13, weight: .medium)
            textField.lineBreakMode = .byTruncatingTail
            textField.isEditable = false
            textField.isBordered = false
            textField.drawsBackground = false
            cell.imageView = imageView
            cell.textField = textField
            cell.addSubview(imageView)
            cell.addSubview(textField)
            NSLayoutConstraint.activate([
                imageView.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 4),
                imageView.centerYAnchor.constraint(equalTo: cell.centerYAnchor),
                imageView.widthAnchor.constraint(equalToConstant: 16),
                imageView.heightAnchor.constraint(equalToConstant: 16),
                textField.leadingAnchor.constraint(equalTo: imageView.trailingAnchor, constant: 6),
                textField.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -4),
                textField.centerYAnchor.constraint(equalTo: cell.centerYAnchor)
            ])
        }

        let symbol: String
        if item.isRecentGroup {
            symbol = "clock"
        } else {
            symbol = item.node.kind == .folder ? "folder" : "doc.text"
        }
        cell.imageView?.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)

        let prefix = displayMode == .tree ? treePrefix(for: item) : ""
        let displayName = prefix + item.node.name

        if !filterText.isEmpty, let range = item.node.name.range(of: filterText, options: .caseInsensitive) {
            let attributed = NSMutableAttributedString(string: displayName)
            let nsRange = NSRange(range, in: item.node.name)
            let adjustedRange = NSRange(location: prefix.count + nsRange.location, length: nsRange.length)
            attributed.addAttribute(.foregroundColor, value: NSColor.secondaryLabelColor, range: NSRange(location: 0, length: prefix.count))
            attributed.addAttribute(.foregroundColor, value: NSColor.controlAccentColor, range: adjustedRange)
            appendTypeBadge(to: attributed, path: item.node.path)
            cell.textField?.attributedStringValue = attributed
        } else {
            if let type = typeByPath[item.node.path], item.node.kind == .note {
                let attributed = NSMutableAttributedString(string: displayName)
                appendTypeBadge(to: attributed, path: item.node.path, explicitType: type)
                cell.textField?.attributedStringValue = attributed
            } else {
                cell.textField?.stringValue = displayName
            }
        }
        return cell
    }

    private func appendTypeBadge(to attributed: NSMutableAttributedString, path: String, explicitType: String? = nil) {
        guard let type = explicitType ?? typeByPath[path], !type.isEmpty else { return }
        let badge = NSAttributedString(
            string: "  [\(type)]",
            attributes: [.foregroundColor: NSColor.tertiaryLabelColor, .font: NSFont.systemFont(ofSize: 11)]
        )
        attributed.append(badge)
    }

    private func treePrefix(for item: NotesOutlineItem) -> String {
        guard item.depth > 0, item.node.path != "__virtual_root__" else { return "" }
        let ancestorPrefix = item.ancestorLastFlags.dropFirst().map { $0 ? "   " : "│  " }.joined()
        return ancestorPrefix + (item.isLast ? "└─ " : "├─ ")
    }

    func outlineView(_ outlineView: NSOutlineView, heightOfRowByItem item: Any) -> CGFloat { 24 }

    func outlineView(_ outlineView: NSOutlineView, rowViewForItem item: Any) -> NSTableRowView? {
        let row = NotesOutlineRowView()
        row.selectionHighlightStyle = .regular
        return row
    }

    func outlineViewItemDidExpand(_ notification: Notification) {
        guard let item = notification.userInfo?["NSObject"] as? NotesOutlineItem else { return }
        onExpansionChanged?(item.node, true)
        if !filterText.isEmpty {
            expandAncestors(of: item, in: notification.object as? NSOutlineView)
        }
    }

    func outlineViewItemDidCollapse(_ notification: Notification) {
        guard let item = notification.userInfo?["NSObject"] as? NotesOutlineItem else { return }
        onExpansionChanged?(item.node, false)
    }

    private func expandAncestors(of item: NotesOutlineItem, in outlineView: NSOutlineView?) {
        guard let outlineView else { return }
        outlineView.expandItem(item)
    }

    func expandAll(in outlineView: NSOutlineView) {
        func expand(_ item: NotesOutlineItem) {
            if item.node.kind == .folder {
                outlineView.expandItem(item)
            }
            item.children.forEach(expand)
        }
        if let displayRoot {
            expand(displayRoot)
        }
    }

    func collapseAll(in outlineView: NSOutlineView) {
        func collapse(_ item: NotesOutlineItem) {
            item.children.forEach(collapse)
            if item.node.kind == .folder {
                outlineView.collapseItem(item, collapseChildren: true)
            }
        }
        if let displayRoot {
            collapse(displayRoot)
        }
    }

    func mindMapRootNode() -> NotesNode? {
        guard let displayRoot else { return nil }
        if displayRoot.node.path == "__virtual_root__" {
            return displayRoot.children.first { $0.node.path != "__recent__" }?.node
        }
        return displayRoot.node
    }

    func handleDoubleClick(on outlineView: NSOutlineView) {
        let row = outlineView.clickedRow
        guard row >= 0, let item = outlineView.item(atRow: row) as? NotesOutlineItem else { return }
        guard item.node.kind == .note else { return }
        onActivate?(item.node)
    }

    func activateSelection(in outlineView: NSOutlineView) {
        let row = outlineView.selectedRow
        guard row >= 0, let item = outlineView.item(atRow: row) as? NotesOutlineItem else { return }
        if item.node.kind == .note {
            onActivate?(item.node)
        } else if !item.isRecentGroup {
            outlineView.isItemExpanded(item) ? outlineView.collapseItem(item) : outlineView.expandItem(item)
        }
    }

    func item(for path: String) -> NotesOutlineItem? {
        guard let displayRoot else { return nil }
        return findItem(withPath: path, in: displayRoot)
    }

    private func findItem(withPath path: String, in item: NotesOutlineItem) -> NotesOutlineItem? {
        if item.node.path == path { return item }
        for child in item.children {
            if let found = findItem(withPath: path, in: child) { return found }
        }
        return nil
    }
}

private final class NotesOutlineRowView: NSTableRowView {
    override func drawSelection(in dirtyRect: NSRect) {
        guard selectionHighlightStyle != .none, isSelected else { return }
        NSColor.selectedContentBackgroundColor.setFill()
        dirtyRect.fill()
    }
}
