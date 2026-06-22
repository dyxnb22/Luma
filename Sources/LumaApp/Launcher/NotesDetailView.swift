import AppKit
import LumaCore
import LumaModules

@MainActor
final class NotesDetailView: NSObject, ModuleDetailView {
    let moduleTitle = "Notes"
    let detailView: NSView
    let usesSharedTopBar = true

    private let module: NotesModule
    private let topStrip = NSView()
    private let filterStrip = NSView()
    private let filterField = NSTextField()
    private let rootPathLabel = NSTextField(labelWithString: "")
    private let modeControl = NSSegmentedControl(labels: ["目录", "导图"], trackingMode: .selectOne, target: nil, action: nil)
    private let expandAllButton = NSButton()
    private let collapseAllButton = NSButton()
    private let gearButton = NSButton()
    private let emptyStateButton = NSButton()
    private let scrollView = NSScrollView()
    private let outlineView = NSOutlineView()
    private let mindMapView = NotesMindMapView()
    private let dataSource = NotesOutlineDataSource()
    private var refreshTask: Task<Void, Never>?
    private var actions: NotesActions?
    private var savedExpansion = Set<String>()

    init(module: NotesModule) {
        self.module = module
        let container = NSView()
        container.translatesAutoresizingMaskIntoConstraints = false
        self.detailView = container
        super.init()
        setup(container: container)
    }

    func activate() {
        refreshTask?.cancel()
        refreshTask = Task { [weak self] in
            guard let self else { return }
            await module.reloadFromConfig()
            await refreshTree()
        }
    }

    func deactivate() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    func handleKeyDown(_ event: NSEvent) -> Bool {
        if detailView.window?.firstResponder === filterField {
            if event.keyCode == 36 {
                dataSource.selectFirstMatch(in: outlineView)
                return true
            }
            if event.keyCode == 125 {
                dataSource.selectNextMatch(in: outlineView)
                return true
            }
            return false
        }

        guard detailView.window?.firstResponder === outlineView else { return false }
        switch event.keyCode {
        case 36:
            dataSource.activateSelection(in: outlineView)
            return true
        case 51:
            showDeleteConfirmation()
            return true
        case 120:
            showRenamePrompt()
            return true
        default:
            return false
        }
    }

    private func setup(container: NSView) {
        topStrip.translatesAutoresizingMaskIntoConstraints = false
        filterStrip.translatesAutoresizingMaskIntoConstraints = false

        rootPathLabel.font = .systemFont(ofSize: 11)
        rootPathLabel.textColor = .secondaryLabelColor
        rootPathLabel.lineBreakMode = .byTruncatingMiddle
        rootPathLabel.translatesAutoresizingMaskIntoConstraints = false

        modeControl.selectedSegment = 0
        modeControl.segmentStyle = .rounded
        modeControl.target = self
        modeControl.action = #selector(modeChanged(_:))
        modeControl.translatesAutoresizingMaskIntoConstraints = false

        configureIconButton(expandAllButton, symbol: "arrow.up.left.and.arrow.down.right", tooltip: "Expand all")
        expandAllButton.target = self
        expandAllButton.action = #selector(expandAll)

        configureIconButton(collapseAllButton, symbol: "arrow.down.right.and.arrow.up.left", tooltip: "Collapse all")
        collapseAllButton.target = self
        collapseAllButton.action = #selector(collapseAll)

        gearButton.bezelStyle = .texturedRounded
        gearButton.image = NSImage(systemSymbolName: "gearshape", accessibilityDescription: "Settings")
        gearButton.isBordered = false
        gearButton.target = self
        gearButton.action = #selector(showGearMenu(_:))
        gearButton.translatesAutoresizingMaskIntoConstraints = false

        filterField.placeholderString = "Filter notes and folders…"
        filterField.font = .systemFont(ofSize: 13)
        filterField.isBezeled = true
        filterField.bezelStyle = .roundedBezel
        filterField.target = self
        filterField.action = #selector(filterChanged)
        filterField.translatesAutoresizingMaskIntoConstraints = false
        NotificationCenter.default.addObserver(self, selector: #selector(filterTextDidChange(_:)), name: NSControl.textDidChangeNotification, object: filterField)

        emptyStateButton.title = "Set Notes Root…"
        emptyStateButton.bezelStyle = .rounded
        emptyStateButton.target = self
        emptyStateButton.action = #selector(pickRoot)
        emptyStateButton.translatesAutoresizingMaskIntoConstraints = false

        outlineView.headerView = nil
        outlineView.indentationPerLevel = 8
        outlineView.rowSizeStyle = .custom
        outlineView.allowsEmptySelection = true
        outlineView.delegate = dataSource
        outlineView.dataSource = dataSource
        outlineView.doubleAction = #selector(outlineDoubleClicked)
        outlineView.target = self
        outlineView.menu = buildContextMenu()
        outlineView.translatesAutoresizingMaskIntoConstraints = false

        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("tree"))
        column.title = ""
        column.minWidth = 240
        column.width = 640
        column.resizingMask = .autoresizingMask
        outlineView.addTableColumn(column)
        outlineView.outlineTableColumn = column

        scrollView.documentView = outlineView
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = true
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
        scrollView.translatesAutoresizingMaskIntoConstraints = false

        dataSource.onActivate = { [weak self] node in
            self?.openNote(node)
        }
        dataSource.onExpansionChanged = { [weak self] node, expanded in
            self?.persistExpansion(path: node.path, expanded: expanded)
        }

        container.addSubview(topStrip)
        topStrip.addSubview(rootPathLabel)
        topStrip.addSubview(modeControl)
        topStrip.addSubview(expandAllButton)
        topStrip.addSubview(collapseAllButton)
        topStrip.addSubview(gearButton)
        container.addSubview(filterStrip)
        filterStrip.addSubview(filterField)
        container.addSubview(emptyStateButton)
        container.addSubview(scrollView)

        NSLayoutConstraint.activate([
            topStrip.topAnchor.constraint(equalTo: container.topAnchor),
            topStrip.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            topStrip.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            topStrip.heightAnchor.constraint(equalToConstant: 28),

            rootPathLabel.leadingAnchor.constraint(equalTo: topStrip.leadingAnchor, constant: 12),
            rootPathLabel.centerYAnchor.constraint(equalTo: topStrip.centerYAnchor),
            rootPathLabel.trailingAnchor.constraint(lessThanOrEqualTo: modeControl.leadingAnchor, constant: -8),

            modeControl.trailingAnchor.constraint(equalTo: expandAllButton.leadingAnchor, constant: -8),
            modeControl.centerYAnchor.constraint(equalTo: topStrip.centerYAnchor),
            modeControl.widthAnchor.constraint(equalToConstant: 104),
            modeControl.heightAnchor.constraint(equalToConstant: 24),

            expandAllButton.trailingAnchor.constraint(equalTo: collapseAllButton.leadingAnchor, constant: -4),
            expandAllButton.centerYAnchor.constraint(equalTo: topStrip.centerYAnchor),
            expandAllButton.widthAnchor.constraint(equalToConstant: 24),
            expandAllButton.heightAnchor.constraint(equalToConstant: 24),

            collapseAllButton.trailingAnchor.constraint(equalTo: gearButton.leadingAnchor, constant: -6),
            collapseAllButton.centerYAnchor.constraint(equalTo: topStrip.centerYAnchor),
            collapseAllButton.widthAnchor.constraint(equalToConstant: 24),
            collapseAllButton.heightAnchor.constraint(equalToConstant: 24),

            gearButton.trailingAnchor.constraint(equalTo: topStrip.trailingAnchor, constant: -8),
            gearButton.centerYAnchor.constraint(equalTo: topStrip.centerYAnchor),
            gearButton.widthAnchor.constraint(equalToConstant: 24),
            gearButton.heightAnchor.constraint(equalToConstant: 24),

            filterStrip.topAnchor.constraint(equalTo: topStrip.bottomAnchor),
            filterStrip.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            filterStrip.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            filterStrip.heightAnchor.constraint(equalToConstant: 28),

            filterField.leadingAnchor.constraint(equalTo: filterStrip.leadingAnchor, constant: 12),
            filterField.trailingAnchor.constraint(equalTo: filterStrip.trailingAnchor, constant: -12),
            filterField.centerYAnchor.constraint(equalTo: filterStrip.centerYAnchor),

            emptyStateButton.centerXAnchor.constraint(equalTo: container.centerXAnchor),
            emptyStateButton.centerYAnchor.constraint(equalTo: container.centerYAnchor),

            scrollView.topAnchor.constraint(equalTo: filterStrip.bottomAnchor),
            scrollView.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            scrollView.bottomAnchor.constraint(equalTo: container.bottomAnchor)
        ])
    }

    private func configureIconButton(_ button: NSButton, symbol: String, tooltip: String) {
        button.bezelStyle = .texturedRounded
        button.isBordered = false
        button.image = NSImage(systemSymbolName: symbol, accessibilityDescription: tooltip)
        button.toolTip = tooltip
        button.translatesAutoresizingMaskIntoConstraints = false
    }

    private func refreshTree() async {
        let config = await module.loadConfig()
        var snapshot = await module.snapshot()
        actions = NotesActions(index: await module.treeIndex())

        if let root = config.root {
            if snapshot == nil {
                await module.reloadFromConfig()
                snapshot = await module.snapshot()
            }
            rootPathLabel.stringValue = root.path
            topStrip.isHidden = false
            filterStrip.isHidden = false
            scrollView.isHidden = false
            emptyStateButton.isHidden = true
        } else {
            rootPathLabel.stringValue = ""
            topStrip.isHidden = true
            filterStrip.isHidden = true
            scrollView.isHidden = true
            emptyStateButton.isHidden = false
            dataSource.reload(root: nil)
            outlineView.reloadData()
            return
        }

        var recentNodes: [NotesNode] = []
        if ModuleDetailRegistry.isLauncherQueryEmpty {
            let paths = await module.recentNotePaths()
            recentNodes = paths.compactMap { path in
                let url = URL(fileURLWithPath: path)
                guard FileManager.default.fileExists(atPath: path) else { return nil }
                return NotesNode(path: path, name: url.deletingPathExtension().lastPathComponent, kind: .note, children: [])
            }
        }

        dataSource.reload(root: snapshot, recentNodes: recentNodes)
        mindMapView.reload(root: dataSource.mindMapRootNode())
        updateDocumentViewForCurrentMode()
        outlineView.reloadData()

        guard let rootItem = dataSource.rootItem else { return }
        outlineView.expandItem(dataSource.displayRoot ?? rootItem)

        for path in config.expandedFolders where path != config.root?.path {
            if let item = dataSource.item(for: path) {
                outlineView.expandItem(item)
            }
        }

        if !dataSource.filterText.isEmpty {
            dataSource.expandAll(in: outlineView)
        }
        detailView.window?.makeFirstResponder(outlineView)
    }

    @objc private func modeChanged(_ sender: NSSegmentedControl) {
        dataSource.displayMode = .directory
        updateDocumentViewForCurrentMode()
        outlineView.reloadData()
        if sender.selectedSegment == 1 {
            mindMapView.expandAll()
        }
    }

    @objc private func expandAll() {
        if modeControl.selectedSegment == 1 {
            mindMapView.expandAll()
        } else {
            dataSource.expandAll(in: outlineView)
        }
    }

    @objc private func collapseAll() {
        if modeControl.selectedSegment == 1 {
            mindMapView.collapseAll()
        } else {
            dataSource.collapseAll(in: outlineView)
        }
    }

    private func updateDocumentViewForCurrentMode() {
        scrollView.documentView = modeControl.selectedSegment == 1 ? mindMapView : outlineView
    }

    @objc private func pickRoot() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "Choose"
        beginRootPanel(panel)
    }

    private func beginRootPanel(_ panel: NSOpenPanel) {
        NSApp.activate()
        if let window = detailView.window {
            panel.beginSheetModal(for: window) { [weak self] response in
                guard response == .OK, let url = panel.url else { return }
                Task { await self?.setRoot(url) }
            }
        } else {
            let response = panel.runModal()
            guard response == .OK, let url = panel.url else { return }
            Task { await setRoot(url) }
        }
    }

    private func setRoot(_ url: URL) async {
        var config = await module.loadConfig()
        config.root = url.standardizedFileURL
        if !config.expandedFolders.contains(url.path) {
            config.expandedFolders.insert(url.path)
        }
        try? await module.saveConfig(config)
        await module.reloadFromConfig()
        await refreshTree()
    }

    @objc private func showGearMenu(_ sender: NSButton) {
        let menu = NSMenu()
        menu.addItem(withTitle: "Change Root…", action: #selector(pickRoot), keyEquivalent: "")
        menu.addItem(withTitle: "Reveal Root in Finder", action: #selector(revealRoot), keyEquivalent: "")
        menu.addItem(withTitle: "Image Tools…", action: #selector(openImageTools), keyEquivalent: "")
        menu.items.forEach { $0.target = self }
        menu.popUp(positioning: nil, at: NSPoint(x: 0, y: sender.bounds.height + 4), in: sender)
    }

    @objc private func openImageTools() {
        Task { @MainActor [weak self] in
            guard let self, let root = await module.loadConfig().root, let window = detailView.window else { return }
            let panel = NotesImageToolsPanel(root: root)
            let sheet = NSWindow(contentViewController: panel)
            await window.beginSheet(sheet)
        }
    }

    @objc private func revealRoot() {
        Task { [weak self] in
            guard let self, let root = await module.loadConfig().root else { return }
            NSWorkspace.shared.selectFile(nil, inFileViewerRootedAtPath: root.path)
        }
    }

    @objc private func outlineDoubleClicked() {
        dataSource.handleDoubleClick(on: outlineView)
    }

    @objc private func filterChanged() {
        applyFilter()
    }

    @objc private func filterTextDidChange(_ note: Notification) {
        applyFilter()
    }

    private func applyFilter() {
        let text = filterField.stringValue
        if dataSource.filterText.isEmpty && !text.isEmpty {
            Task {
                let config = await module.loadConfig()
                savedExpansion = config.expandedFolders
            }
        }
        if !dataSource.filterText.isEmpty && text.isEmpty {
            Task { [weak self] in
                guard let self else { return }
                var config = await module.loadConfig()
                config.expandedFolders = savedExpansion
                try? await module.saveConfig(config)
            }
        }
        dataSource.setFilter(text)
        mindMapView.reload(root: dataSource.mindMapRootNode())
        outlineView.reloadData()
        if !text.isEmpty {
            dataSource.expandAll(in: outlineView)
        } else {
            Task { await refreshTree() }
        }
    }

    private func openNote(_ node: NotesNode) {
        let url = URL(fileURLWithPath: node.path)
        Task {
            await module.recordOpenedNote(path: node.path)
            await MainActor.run { NotesTypora.open(url) }
        }
    }

    private func persistExpansion(path: String, expanded: Bool) {
        Task { [weak self] in
            guard let self else { return }
            var config = await module.loadConfig()
            if expanded {
                config.expandedFolders.insert(path)
            } else {
                config.expandedFolders.remove(path)
            }
            try? await module.saveConfig(config)
        }
    }

    private func selectedItem() -> NotesOutlineItem? {
        let row = outlineView.selectedRow
        guard row >= 0 else { return nil }
        return outlineView.item(atRow: row) as? NotesOutlineItem
    }

    private func buildContextMenu() -> NSMenu {
        let menu = NSMenu()
        menu.delegate = self
        return menu
    }

    private func showNamePrompt(title: String, defaultName: String, onOK: @escaping (String) -> Void) {
        guard let window = detailView.window else { return }
        let alert = NSAlert()
        alert.messageText = title
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 240, height: 24))
        field.stringValue = defaultName
        alert.accessoryView = field
        alert.addButton(withTitle: "OK")
        alert.addButton(withTitle: "Cancel")
        alert.beginSheetModal(for: window) { response in
            guard response == .alertFirstButtonReturn else { return }
            onOK(field.stringValue)
        }
    }

    private func showError(_ message: String) {
        guard let window = detailView.window else { return }
        let alert = NSAlert()
        alert.messageText = message
        alert.addButton(withTitle: "OK")
        alert.beginSheetModal(for: window)
    }

    @objc private func createNote() {
        Task { [weak self] in
            guard let self, let folder = await parentFolderURL() else { return }
            await MainActor.run {
                self.showNamePrompt(title: "New Note", defaultName: "") { name in
                    Task { await self.performCreateNote(name: name, in: folder) }
                }
            }
        }
    }

    @objc private func createFolder() {
        Task { [weak self] in
            guard let self, let folder = await parentFolderURL() else { return }
            await MainActor.run {
                self.showNamePrompt(title: "New Folder", defaultName: "") { name in
                    Task { await self.performCreateFolder(name: name, in: folder) }
                }
            }
        }
    }

    private func performCreateNote(name: String, in folder: URL) async {
        guard let actions else { return }
        do {
            let url = try await actions.createNote(name: name, inFolder: folder)
            await refreshTree()
            selectPath(url.path, expandParent: true)
        } catch NotesActionError.alreadyExists {
            showError("A note with that name already exists.")
        } catch NotesActionError.emptyName {
            showError("Name cannot be empty.")
        } catch NotesActionError.nameContainsSlash {
            showError("Name cannot contain '/'.")
        } catch {
            showError(error.localizedDescription)
        }
    }

    private func performCreateFolder(name: String, in folder: URL) async {
        guard let actions else { return }
        do {
            let url = try await actions.createFolder(name: name, inFolder: folder)
            await refreshTree()
            selectPath(url.path, expandParent: true)
        } catch NotesActionError.alreadyExists {
            showError("A folder with that name already exists.")
        } catch NotesActionError.emptyName {
            showError("Name cannot be empty.")
        } catch NotesActionError.nameContainsSlash {
            showError("Name cannot contain '/'.")
        } catch {
            showError(error.localizedDescription)
        }
    }

    @objc private func showRenamePrompt() {
        guard let item = selectedActionItem(), !isRootItem(item) else { return }
        let defaultName = item.node.kind == .note ? item.node.name : item.node.name
        showNamePrompt(title: "Rename", defaultName: defaultName) { [weak self] name in
            Task { await self?.performRename(item: item, to: name) }
        }
    }

    private func performRename(item: NotesOutlineItem, to name: String) async {
        guard let actions else { return }
        do {
            let url = try await actions.rename(URL(fileURLWithPath: item.node.path), to: name)
            await refreshTree()
            selectPath(url.path, expandParent: true)
        } catch NotesActionError.alreadyExists {
            showError("That name already exists.")
        } catch {
            showError(error.localizedDescription)
        }
    }

    @objc private func showDeleteConfirmation() {
        guard let item = selectedActionItem(), !isRootItem(item) else { return }
        if item.node.kind == .folder {
            Task { [weak self] in
                guard let self, let actions else { return }
                do {
                    try await actions.trash(URL(fileURLWithPath: item.node.path))
                    await refreshTree()
                } catch NotesDeleteError.folderNotEmpty {
                    await MainActor.run { [weak self] in
                        guard let self else { return }
                        guard let window = detailView.window else { return }
                        let alert = NSAlert()
                        alert.messageText = "This folder is not empty. Deleting non-empty folders is not supported in this version."
                        alert.addButton(withTitle: "OK")
                        alert.beginSheetModal(for: window)
                    }
                } catch {
                    showError(error.localizedDescription)
                }
            }
            return
        }

        let alert = NSAlert()
        alert.messageText = "Move to Trash?"
        alert.informativeText = item.node.name
        alert.addButton(withTitle: "Cancel")
        alert.addButton(withTitle: "Move to Trash")
        if let button = alert.buttons.last {
            button.hasDestructiveAction = true
        }
        guard let window = detailView.window else { return }
        alert.beginSheetModal(for: window) { [weak self] response in
            guard response == .alertSecondButtonReturn else { return }
            Task { await self?.performTrash(item: item) }
        }
    }

    private func performTrash(item: NotesOutlineItem) async {
        guard let actions else { return }
        do {
            try await actions.trash(URL(fileURLWithPath: item.node.path))
            await refreshTree()
        } catch {
            showError(error.localizedDescription)
        }
    }

    @objc private func revealSelectedInFinder() {
        if let item = selectedActionItem() {
            NSWorkspace.shared.selectFile(item.node.path, inFileViewerRootedAtPath: "")
            return
        }
        Task { [weak self] in
            guard let self, let root = await module.loadConfig().root else { return }
            NSWorkspace.shared.selectFile(nil, inFileViewerRootedAtPath: root.path)
        }
    }

    @objc private func copySelectedPath() {
        guard let item = selectedActionItem() else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(item.node.path, forType: .string)
    }

    @objc private func openSelectedInTypora() {
        guard let item = selectedActionItem(), item.node.kind == .note else { return }
        openNote(item.node)
    }

    @objc private func openLinkedNotes() {
        guard let item = selectedActionItem(), item.node.kind == .note else { return }
        Task { [weak self] in
            guard let self, let actions else { return }
            let links = await actions.relatedNotes(in: URL(fileURLWithPath: item.node.path))
            guard !links.isEmpty else { return }
            await MainActor.run {
                self.showLinkedNotesPopover(links: links, anchoredTo: item)
            }
        }
    }

    private func showLinkedNotesPopover(links: [URL], anchoredTo item: NotesOutlineItem) {
        let controller = NSViewController()
        let list = NSTableView()
        list.headerView = nil
        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("link"))
        list.addTableColumn(column)
        let dataSource = LinkedNotesDataSource(urls: links)
        list.dataSource = dataSource
        list.delegate = dataSource
        controller.view = list
        let popover = NSPopover()
        popover.contentSize = NSSize(width: 240, height: min(links.count, 6) * 24 + 8)
        popover.contentViewController = controller
        popover.behavior = .transient
        let row = outlineView.row(forItem: item)
        if row >= 0 {
            let rect = outlineView.rect(ofRow: row)
            popover.show(relativeTo: rect, of: outlineView, preferredEdge: .maxY)
        }
    }

    private var contextItem: NotesOutlineItem?

    private func selectedActionItem() -> NotesOutlineItem? {
        contextItem ?? selectedItem()
    }

    private func parentFolderURL(for item: NotesOutlineItem?) -> URL? {
        if let item {
            if item.node.kind == .folder {
                return URL(fileURLWithPath: item.node.path)
            }
            return URL(fileURLWithPath: item.node.path).deletingLastPathComponent()
        }
        return nil
    }

    private func parentFolderURL() async -> URL? {
        if let item = contextItem ?? selectedItem() {
            return parentFolderURL(for: item)
        }
        return await module.loadConfig().root
    }

    private func selectPath(_ path: String, expandParent: Bool) {
        if let item = dataSource.item(for: path) {
            if expandParent, let parent = outlineView.parent(forItem: item) {
                outlineView.expandItem(parent)
            }
            let row = outlineView.row(forItem: item)
            if row >= 0 {
                outlineView.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
                outlineView.scrollRowToVisible(row)
            }
        }
    }

    private func isRootItem(_ item: NotesOutlineItem) -> Bool {
        guard let root = dataSource.rootItem else { return false }
        return item.node.path == root.node.path
    }
}

extension NotesDetailView: NSMenuDelegate {
    func menuNeedsUpdate(_ menu: NSMenu) {
        menu.removeAllItems()
        let clickedRow = outlineView.clickedRow
        if clickedRow >= 0 {
            contextItem = outlineView.item(atRow: clickedRow) as? NotesOutlineItem
        } else {
            contextItem = selectedItem()
        }
        guard let item = contextItem else {
            if dataSource.rootItem != nil {
                menu.addItem(withTitle: "New Note", action: #selector(createNote), keyEquivalent: "")
                menu.addItem(withTitle: "New Folder", action: #selector(createFolder), keyEquivalent: "")
                menu.addItem(withTitle: "Reveal Root in Finder", action: #selector(revealSelectedInFinder), keyEquivalent: "")
                menu.items.forEach { $0.target = self }
            }
            return
        }

        if item.node.kind == .folder {
            menu.addItem(withTitle: "New Note", action: #selector(createNote), keyEquivalent: "")
            menu.addItem(withTitle: "New Folder", action: #selector(createFolder), keyEquivalent: "")
            if !isRootItem(item) {
                menu.addItem(withTitle: "Rename", action: #selector(showRenamePrompt), keyEquivalent: "")
                menu.addItem(withTitle: "Delete", action: #selector(showDeleteConfirmation), keyEquivalent: "")
            }
            menu.addItem(withTitle: "Reveal in Finder", action: #selector(revealSelectedInFinder), keyEquivalent: "")
        } else {
            menu.addItem(withTitle: "Open in Typora", action: #selector(openSelectedInTypora), keyEquivalent: "")
            menu.addItem(withTitle: "Open Linked Notes…", action: #selector(openLinkedNotes), keyEquivalent: "l")
            menu.addItem(withTitle: "Rename", action: #selector(showRenamePrompt), keyEquivalent: "")
            menu.addItem(withTitle: "Delete", action: #selector(showDeleteConfirmation), keyEquivalent: "")
            menu.addItem(withTitle: "Reveal in Finder", action: #selector(revealSelectedInFinder), keyEquivalent: "")
            menu.addItem(withTitle: "Copy Path", action: #selector(copySelectedPath), keyEquivalent: "")
        }
        menu.items.forEach { $0.target = self }
    }
}

@MainActor
private final class LinkedNotesDataSource: NSObject, NSTableViewDataSource, NSTableViewDelegate {
    private let urls: [URL]

    init(urls: [URL]) {
        self.urls = urls
    }

    func numberOfRows(in tableView: NSTableView) -> Int { urls.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let cell = NSTextField(labelWithString: urls[row].deletingPathExtension().lastPathComponent)
        cell.font = .systemFont(ofSize: 13)
        return cell
    }

    func tableViewSelectionDidChange(_ notification: Notification) {
        guard let tableView = notification.object as? NSTableView else { return }
        let row = tableView.selectedRow
        guard row >= 0 else { return }
        NotesTypora.open(urls[row])
    }
}
