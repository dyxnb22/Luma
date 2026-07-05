import AppKit
import LumaCore
import LumaModules
import LumaServices

@MainActor
final class NotesDetailView: NSObject, ModuleDetailView {
    private enum ViewMode { case outline, mindMap }

    let moduleTitle = "Notes"
    let detailView: NSView
    let usesSharedTopBar = true

    private let module: NotesModule
    private let workspace = WorkspaceService()
    private let topStrip = NSView()
    private let chipBar = NotesDetailChipBar()
    private let filterStrip = NSView()
    private let filterField = NSTextField()
    private let rootPathLabel = NSTextField(labelWithString: "")
    private let expandAllButton = NSButton()
    private let collapseAllButton = NSButton()
    private let newNoteButton = NSButton()
    private let newFolderButton = NSButton()
    private let viewModeControl = NSSegmentedControl()
    private let gearButton = NSButton()
    private let emptyStateButton = NSButton()
    private let scrollView = NSScrollView()
    private let mindMapScroll = NSScrollView()
    private let mindMapView = NotesMindMapView()
    private let outlineView = NSOutlineView()
    private let dataSource = NotesOutlineDataSource()
    private var viewMode: ViewMode = .outline
    private var refreshTask: Task<Void, Never>?
    private var actions: NotesActions?
    private var savedExpansion = Set<String>()
    private var activeChip: NotesDetailChip?
    private var activePanel: NotesDetailPanel = .outline

    init(module: NotesModule) {
        self.module = module
        let container = NSView()
        container.translatesAutoresizingMaskIntoConstraints = false
        GeekUIKit.installDetailRootChrome(on: container)
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
            await MainActor.run { self.resizeOutlineColumn() }
        }
    }

    func prepareForLauncherHide() async {
        guard activePanel == .outline, activeChip == nil, dataSource.filterText.isEmpty else { return }
        await persistExpansionNow(captureOutlineExpansion())
    }

    func deactivate() {
        refreshTask?.cancel()
        refreshTask = nil
        if activePanel == .outline, activeChip == nil, dataSource.filterText.isEmpty {
            Task { await persistExpansionNow(captureOutlineExpansion()) }
        }
    }

    func handleKeyDown(_ event: NSEvent) -> Bool {
        if event.modifierFlags.contains(.command),
           let chars = event.charactersIgnoringModifiers,
           let digit = Int(chars), (1...3).contains(digit),
           let chip = NotesDetailChip(rawValue: digit - 1) {
            handleChipSelection(chip)
            return true
        }
        if event.modifierFlags.contains(.command),
           let chars = event.charactersIgnoringModifiers?.lowercased(),
           chars == "n",
           !topStrip.isHidden {
            let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
            if flags.contains(.shift) {
                createFolderFromToolbar()
            } else {
                createNoteFromToolbar()
            }
            return true
        }
        if event.modifierFlags.contains(.command),
           event.charactersIgnoringModifiers?.lowercased() == "l",
           detailView.window?.firstResponder === outlineView {
            findBacklinksForSelected()
            return true
        }
        if event.keyCode == 53 {
            if viewMode == .mindMap {
                viewModeControl.selectedSegment = 0
                viewModeChanged()
                return true
            }
            let filterText = filterField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
            if viewMode == .outline, !filterText.isEmpty {
                filterField.stringValue = ""
                applyFilter()
                return true
            }
            return false
        }
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
        GeekUIKit.installDetailRootChrome(on: topStrip)
        GeekUIKit.installDetailRootChrome(on: filterStrip)

        rootPathLabel.font = TypographyTokens.caption2()
        rootPathLabel.textColor = .secondaryLabelColor
        rootPathLabel.lineBreakMode = .byTruncatingMiddle
        rootPathLabel.translatesAutoresizingMaskIntoConstraints = false

        GeekUIKit.styleIconToolbarButton(expandAllButton, symbol: "arrow.up.left.and.arrow.down.right", tooltip: "Expand all")
        expandAllButton.translatesAutoresizingMaskIntoConstraints = false
        expandAllButton.target = self
        expandAllButton.action = #selector(expandAll)

        GeekUIKit.styleIconToolbarButton(collapseAllButton, symbol: "arrow.down.right.and.arrow.up.left", tooltip: "Collapse all")
        collapseAllButton.translatesAutoresizingMaskIntoConstraints = false
        collapseAllButton.target = self
        collapseAllButton.action = #selector(collapseAll)

        GeekUIKit.styleIconToolbarButton(newNoteButton, symbol: "square.and.pencil", tooltip: "New Note (⌘N)")
        newNoteButton.translatesAutoresizingMaskIntoConstraints = false
        newNoteButton.target = self
        newNoteButton.action = #selector(createNoteFromToolbar)

        GeekUIKit.styleIconToolbarButton(newFolderButton, symbol: "folder.badge.plus", tooltip: "New Folder (⌘⇧N)")
        newFolderButton.translatesAutoresizingMaskIntoConstraints = false
        newFolderButton.target = self
        newFolderButton.action = #selector(createFolderFromToolbar)

        viewModeControl.segmentCount = 2
        viewModeControl.setLabel(L10n.tr("notes.viewMode.tree"), forSegment: 0)
        viewModeControl.setLabel(L10n.tr("notes.viewMode.map"), forSegment: 1)
        viewModeControl.selectedSegment = 0
        viewModeControl.segmentStyle = .rounded
        viewModeControl.target = self
        viewModeControl.action = #selector(viewModeChanged)
        viewModeControl.isHidden = true
        viewModeControl.translatesAutoresizingMaskIntoConstraints = false

        GeekUIKit.styleIconToolbarButton(gearButton, symbol: "gearshape", tooltip: "Settings")
        gearButton.target = self
        gearButton.action = #selector(showGearMenu(_:))
        gearButton.translatesAutoresizingMaskIntoConstraints = false

        filterField.placeholderString = "Filter notes and folders…"
        filterField.font = TypographyTokens.body
        filterField.isBezeled = true
        filterField.bezelStyle = .roundedBezel
        filterField.target = self
        filterField.action = #selector(filterChanged)
        filterField.translatesAutoresizingMaskIntoConstraints = false
        NotificationCenter.default.addObserver(self, selector: #selector(filterTextDidChange(_:)), name: NSControl.textDidChangeNotification, object: filterField)

        emptyStateButton.title = "Set Notes Root…"
        GeekUIKit.styleSecondaryButton(emptyStateButton)
        emptyStateButton.target = self
        emptyStateButton.action = #selector(pickRoot)
        emptyStateButton.translatesAutoresizingMaskIntoConstraints = false

        outlineView.headerView = nil
        outlineView.indentationPerLevel = 8
        outlineView.rowSizeStyle = .custom
        outlineView.allowsEmptySelection = true
        outlineView.backgroundColor = .clear
        outlineView.delegate = dataSource
        outlineView.dataSource = dataSource
        outlineView.doubleAction = #selector(outlineDoubleClicked)
        outlineView.target = self
        outlineView.menu = buildContextMenu()
        outlineView.autoresizingMask = [.width]

        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("tree"))
        column.title = ""
        column.minWidth = 120
        column.width = 320
        column.resizingMask = [.autoresizingMask, .userResizingMask]
        outlineView.addTableColumn(column)
        outlineView.outlineTableColumn = column
        GeekUIKit.wireVerticalListScroll(
            scrollView,
            documentView: outlineView,
            observer: self,
            onClipViewResize: #selector(resizeOutlineColumn)
        )
        scrollView.translatesAutoresizingMaskIntoConstraints = false

        mindMapScroll.documentView = mindMapView
        GeekUIKit.configureVerticalListScroll(mindMapScroll)
        mindMapScroll.isHidden = true
        mindMapScroll.translatesAutoresizingMaskIntoConstraints = false
        mindMapView.onActivate = { [weak self] node in self?.openNote(node) }
        mindMapView.onLayoutChanged = { [weak self] in
            self?.syncMindMapDocumentFrame()
        }

        dataSource.onActivate = { [weak self] node in
            self?.openNote(node)
        }
        dataSource.onExpansionChanged = { [weak self] node, expanded in
            self?.persistExpansion(path: node.path, expanded: expanded)
        }
        dataSource.onVisibleRowsChanged = { [weak self] in
            self?.syncOutlineDocumentFrame()
        }

        chipBar.onChipChanged = { [weak self] chip in
            self?.handleChipSelection(chip)
        }
        chipBar.onPanelChanged = { [weak self] panel in
            guard let self else { return }
            self.activePanel = panel
            self.activeChip = nil
            self.chipBar.selectChip(nil)
            Task { await self.applyPanelView(panel) }
        }
        GeekUIKit.installDetailRootChrome(on: chipBar)

        container.addSubview(topStrip)
        topStrip.addSubview(rootPathLabel)
        topStrip.addSubview(newNoteButton)
        topStrip.addSubview(newFolderButton)
        topStrip.addSubview(viewModeControl)
        topStrip.addSubview(expandAllButton)
        topStrip.addSubview(collapseAllButton)
        topStrip.addSubview(gearButton)
        container.addSubview(chipBar)
        container.addSubview(filterStrip)
        filterStrip.addSubview(filterField)
        container.addSubview(emptyStateButton)
        container.addSubview(scrollView)
        container.addSubview(mindMapScroll)
        scrollView.documentView = outlineView

        NSLayoutConstraint.activate([
            topStrip.topAnchor.constraint(equalTo: container.topAnchor),
            topStrip.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            topStrip.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            topStrip.heightAnchor.constraint(equalToConstant: 28),

            rootPathLabel.leadingAnchor.constraint(equalTo: topStrip.leadingAnchor, constant: LauncherChromeTokens.detailTableRowPaddingH),
            rootPathLabel.centerYAnchor.constraint(equalTo: topStrip.centerYAnchor),
            rootPathLabel.trailingAnchor.constraint(lessThanOrEqualTo: newNoteButton.leadingAnchor, constant: -8),

            newNoteButton.trailingAnchor.constraint(equalTo: newFolderButton.leadingAnchor, constant: -4),
            newNoteButton.centerYAnchor.constraint(equalTo: topStrip.centerYAnchor),
            newNoteButton.widthAnchor.constraint(equalToConstant: 24),
            newNoteButton.heightAnchor.constraint(equalToConstant: 24),

            newFolderButton.trailingAnchor.constraint(equalTo: expandAllButton.leadingAnchor, constant: -6),
            newFolderButton.centerYAnchor.constraint(equalTo: topStrip.centerYAnchor),
            newFolderButton.widthAnchor.constraint(equalToConstant: 24),
            newFolderButton.heightAnchor.constraint(equalToConstant: 24),

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

            chipBar.topAnchor.constraint(equalTo: topStrip.bottomAnchor),
            chipBar.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            chipBar.trailingAnchor.constraint(equalTo: container.trailingAnchor),

            filterStrip.topAnchor.constraint(equalTo: chipBar.bottomAnchor),
            filterStrip.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            filterStrip.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            filterStrip.heightAnchor.constraint(equalToConstant: 28),

            filterField.leadingAnchor.constraint(equalTo: filterStrip.leadingAnchor, constant: LauncherChromeTokens.detailTableRowPaddingH),
            filterField.trailingAnchor.constraint(equalTo: filterStrip.trailingAnchor, constant: -LauncherChromeTokens.detailTableRowPaddingH),
            filterField.centerYAnchor.constraint(equalTo: filterStrip.centerYAnchor),

            emptyStateButton.centerXAnchor.constraint(equalTo: container.centerXAnchor),
            emptyStateButton.centerYAnchor.constraint(equalTo: container.centerYAnchor),

            scrollView.topAnchor.constraint(equalTo: filterStrip.bottomAnchor),
            scrollView.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            scrollView.bottomAnchor.constraint(equalTo: container.bottomAnchor),

            mindMapScroll.topAnchor.constraint(equalTo: filterStrip.bottomAnchor),
            mindMapScroll.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            mindMapScroll.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            mindMapScroll.bottomAnchor.constraint(equalTo: container.bottomAnchor)
        ])
    }

    @objc private func resizeOutlineColumn() {
        guard let column = outlineView.tableColumns.first else { return }
        let width = max(120, scrollView.contentSize.width - 8)
        if abs(column.width - width) > 1 {
            column.width = width
            outlineView.noteHeightOfRows(withIndexesChanged: IndexSet(integersIn: 0..<outlineView.numberOfRows))
        }
        syncOutlineDocumentFrame()
    }

    private func syncOutlineDocumentFrame() {
        guard scrollView.documentView === outlineView, !scrollView.isHidden else { return }
        GeekUIKit.syncVerticalListDocumentFrame(in: scrollView)
    }

    private func syncMindMapDocumentFrame() {
        guard mindMapScroll.documentView === mindMapView, !mindMapScroll.isHidden else { return }
        GeekUIKit.syncVerticalListDocumentFrame(in: mindMapScroll)
    }

    @objc private func viewModeChanged() {
        viewMode = viewModeControl.selectedSegment == 0 ? .outline : .mindMap
        let outline = viewMode == .outline
        scrollView.isHidden = !outline
        mindMapScroll.isHidden = outline
        filterStrip.isHidden = !outline
        expandAllButton.isHidden = !outline
        collapseAllButton.isHidden = !outline
        newNoteButton.isHidden = !outline
        newFolderButton.isHidden = !outline
        if outline {
            scrollView.window?.makeFirstResponder(outlineView)
        } else {
            mindMapView.reload(root: dataSource.mindMapRootNode())
            mindMapScroll.contentView.scroll(to: .zero)
            syncMindMapDocumentFrame()
            mindMapScroll.window?.makeFirstResponder(mindMapView)
        }
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
            chipBar.isHidden = false
            filterStrip.isHidden = (viewMode != .outline)
            scrollView.isHidden = (viewMode != .outline)
            mindMapScroll.isHidden = (viewMode != .mindMap)
            emptyStateButton.isHidden = true

            let inboxCount = await module.inboxCount()
            chipBar.setPanelInboxCount(inboxCount)
            let dailyMissing = await module.dailyNotePath() == nil
            chipBar.setTodayHint(missing: dailyMissing)
            await reloadTypeLabels()
        } else {
            rootPathLabel.stringValue = ""
            topStrip.isHidden = true
            chipBar.isHidden = true
            filterStrip.isHidden = true
            scrollView.isHidden = true
            emptyStateButton.isHidden = false
            dataSource.reload(root: nil)
            outlineView.reloadData()
            return
        }

        if let chip = activeChip {
            await applyChipView(chip)
        } else {
            await applyPanelView(activePanel)
        }

        if viewMode == .mindMap {
            mindMapView.reload(root: dataSource.mindMapRootNode())
            syncMindMapDocumentFrame()
        }
        detailView.window?.makeFirstResponder(viewMode == .outline ? outlineView : mindMapView)
        syncOutlineDocumentFrame()
    }

    private func reloadOutlineData(restoreExpansion: (() -> Void)? = nil) {
        dataSource.suppressExpansionCallbacks = true
        outlineView.reloadData()
        restoreExpansion?()
        syncOutlineDocumentFrame()
        DispatchQueue.main.async { [weak self] in
            self?.dataSource.suppressExpansionCallbacks = false
            self?.syncOutlineDocumentFrame()
        }
    }

    private func captureOutlineExpansion() -> Set<String> {
        guard let displayRoot = dataSource.displayRoot else { return [] }
        var paths = Set<String>()
        func collect(_ item: NotesOutlineItem) {
            if item.node.kind == .folder, outlineView.isItemExpanded(item) {
                paths.insert(item.node.path)
            }
            for child in item.children {
                collect(child)
            }
        }
        collect(displayRoot)
        return paths
    }

    private func persistExpansionNow(_ paths: Set<String>) async {
        let filtered = Set(paths.filter { !isVirtualOutlinePath($0) })
        let store = NotesRootConfigStore()
        var config = await store.load()
        config.expandedFolders = filtered
        try? await store.save(config)
        try? await module.saveConfig(config)
    }

    private func isVirtualOutlinePath(_ path: String) -> Bool {
        path.hasPrefix("__")
    }

    private func restoreExpandedFolders(from saved: Set<String>) {
        let paths = saved.filter { !isVirtualOutlinePath($0) }.sorted {
            $0.split(separator: "/").count < $1.split(separator: "/").count
        }
        if let displayRoot = dataSource.displayRoot {
            outlineView.expandItem(displayRoot)
        } else if let rootItem = dataSource.rootItem {
            outlineView.expandItem(rootItem)
        }
        for path in paths {
            if let item = dataSource.item(for: path) {
                outlineView.expandItem(item)
            }
        }
    }

    private func reloadTypeLabels() async {
        let entries = await module.notesMetaIndex().allEntries()
        var labels: [String: String] = [:]
        for entry in entries {
            if let type = entry.type, !type.isEmpty {
                labels[entry.path] = type
            }
        }
        dataSource.setTypeLabels(labels)
    }

    private func handleChipSelection(_ chip: NotesDetailChip?) {
        if chip == .today {
            chipBar.selectChip(nil)
            activeChip = nil
            Task { await openOrCreateTodayDaily() }
            return
        }
        if let chip {
            chipBar.selectChip(chip)
            activeChip = chip
            Task { await applyChipView(chip) }
            return
        }
        chipBar.selectChip(nil)
        activeChip = nil
        Task { await applyChipView(nil) }
    }

    private func openOrCreateTodayDaily() async {
        let config = await module.loadConfig()
        guard let root = config.root else { return }
        if actions == nil {
            actions = NotesActions(index: await module.treeIndex())
        }
        guard let actions else { return }
        do {
            let url = try await actions.openOrCreateDailyNote(
                root: root,
                dailyFolderName: config.dailyFolderName
            )
            await module.recordOpenedNote(path: url.path)
            await workspace.openURL(url)
            chipBar.setTodayHint(missing: false)
            await refreshTree()
        } catch {
            showError(error.localizedDescription)
        }
    }

    private func applyChipView(_ chip: NotesDetailChip?) async {
        let config = await module.loadConfig()
        guard config.root != nil else { return }
        activePanel = .outline
        chipBar.selectPanel(.outline)

        guard let chip else {
            await applyPanelView(.outline)
            return
        }

        switch chip {
        case .today:
            break
        case .recent:
            let paths = await module.recentNotePaths()
            let nodes = paths.compactMap { path -> NotesNode? in
                guard FileManager.default.fileExists(atPath: path) else { return nil }
                let url = URL(fileURLWithPath: path)
                return NotesNode(path: path, name: url.deletingPathExtension().lastPathComponent, kind: .note, children: [])
            }
            dataSource.showFlatList(title: "Recent", nodes: nodes)
        case .pinned:
            let pinned = await module.notesMetaIndex().pinnedNotes()
            let nodes = pinned.map { NotesNode(path: $0.path, name: $0.name, kind: .note, children: []) }
            dataSource.showFlatList(title: "Pinned", nodes: nodes)
        }

        reloadOutlineData { [self] in
            if let displayRoot = dataSource.displayRoot {
                outlineView.expandItem(displayRoot)
            }
        }
    }

    private func applyPanelView(_ panel: NotesDetailPanel) async {
        let config = await module.loadConfig()
        guard let snapshot = await module.snapshot() else { return }
        activePanel = panel

        switch panel {
        case .outline:
            dataSource.showTree(root: snapshot, recentNodes: [])
            reloadOutlineData { [self] in
                restoreExpandedFolders(from: config.expandedFolders)
            }
        case .browse:
            let meta = await module.notesMetaIndex()
            let weekStart = Calendar.current.date(byAdding: .day, value: -7, to: Date()) ?? Date()
            var groups: [(String, [NotesNode])] = []
            let modified = await meta.modifiedSince(weekStart).map {
                NotesNode(path: $0.path, name: $0.name, kind: .note, children: [])
            }
            if !modified.isEmpty {
                groups.append(("Modified this week", modified))
            }
            for type in await meta.distinctTypes() {
                let nodes = await meta.notes(withType: type).map {
                    NotesNode(path: $0.path, name: $0.name, kind: .note, children: [])
                }
                groups.append((type, nodes))
            }
            dataSource.showGroupedList(groups: groups)
            reloadOutlineData { [self] in
                if let displayRoot = dataSource.displayRoot {
                    outlineView.expandItem(displayRoot)
                    dataSource.expandAll(in: outlineView)
                }
            }
        case .inbox:
            guard let root = config.root else { return }
            let nodes = await actions?.notesInFolder(named: config.inboxFolderName, root: root) ?? []
            dataSource.showFlatList(title: "Inbox", nodes: nodes)
            reloadOutlineData { [self] in
                if let displayRoot = dataSource.displayRoot {
                    outlineView.expandItem(displayRoot)
                }
            }
        }
    }

    @objc private func expandAll() {
        dataSource.expandAll(in: outlineView)
        syncOutlineDocumentFrame()
    }

    @objc private func collapseAll() {
        dataSource.collapseAll(in: outlineView)
        syncOutlineDocumentFrame()
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
        menu.addItem(NSMenuItem.separator())
        menu.addItem(withTitle: "Image Tools…", action: #selector(openImageTools), keyEquivalent: "")
        menu.addItem(withTitle: "Experimental: Mind Map…", action: #selector(showExperimentalMindMap), keyEquivalent: "")
        menu.items.forEach { $0.target = self }
        menu.popUp(positioning: nil, at: NSPoint(x: 0, y: sender.bounds.height + 4), in: sender)
    }

    @objc private func openImageTools() {
        Task { @MainActor [weak self] in
            guard let self, let root = await module.loadConfig().root, let window = detailView.window else { return }
            await NotesDetailSheets.presentImageTools(on: window, root: root)
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
                config.expandedFolders = savedExpansion.filter { !isVirtualOutlinePath($0) }
                try? await module.saveConfig(config)
            }
        }
        dataSource.setFilter(text)
        if !text.isEmpty {
            reloadOutlineData { [self] in
                dataSource.expandAll(in: outlineView)
            }
        } else {
            Task { await refreshTree() }
        }
    }

    private func openNote(_ node: NotesNode) {
        let url = URL(fileURLWithPath: node.path)
        Task {
            await module.recordOpenedNote(path: node.path)
            await workspace.openURL(url)
        }
    }

    private func persistExpansion(path: String, expanded: Bool) {
        guard !isVirtualOutlinePath(path) else { return }
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

    @objc private func createNoteFromToolbar() {
        let preferred = selectedItem()
        contextItem = nil
        beginCreateNote(preferred: preferred)
    }

    @objc private func createNote() {
        beginCreateNote(preferred: nil)
    }

    private func beginCreateNote(preferred: NotesOutlineItem?) {
        Task { [weak self] in
            guard let self, let folder = await self.defaultCreateFolderURL(preferred: preferred) else { return }
            let templates = await self.availableTemplates()
            await MainActor.run {
                self.presentCreateNotePrompt(in: folder, templates: templates)
            }
        }
    }

    @objc private func createFolderFromToolbar() {
        let preferred = selectedItem()
        contextItem = nil
        beginCreateFolder(preferred: preferred)
    }

    @objc private func createFolder() {
        beginCreateFolder(preferred: nil)
    }

    private func beginCreateFolder(preferred: NotesOutlineItem?) {
        Task { [weak self] in
            guard let self, let folder = await self.defaultCreateFolderURL(preferred: preferred) else { return }
            await MainActor.run {
                self.showNamePrompt(title: "New Folder", defaultName: "") { name in
                    Task { await self.performCreateFolder(name: name, in: folder) }
                }
            }
        }
    }

    private func availableTemplates() async -> [NotesTemplateInfo] {
        let config = await module.loadConfig()
        guard let root = config.root else { return [] }
        return NotesTemplateStore.scanTemplates(root: root, folderName: config.templatesFolderName)
    }

    private func defaultCreateFolderURL(preferred: NotesOutlineItem? = nil) async -> URL? {
        let config = await module.loadConfig()
        guard let root = config.root else { return nil }
        if actions == nil {
            actions = NotesActions(index: await module.treeIndex())
        }
        guard let actions else { return root }
        let item = preferred ?? selectedActionItem()
        return await resolvedCreateFolderURL(config: config, root: root, actions: actions, item: item)
    }

    private func resolvedCreateFolderURL(
        config: NotesRootConfig,
        root: URL,
        actions: NotesActions,
        item: NotesOutlineItem?
    ) async -> URL {
        if let item, !isVirtualOutlinePath(item.node.path) {
            if item.node.kind == .folder {
                return URL(fileURLWithPath: item.node.path)
            }
            let parent = URL(fileURLWithPath: item.node.path).deletingLastPathComponent()
            if !isVirtualOutlinePath(parent.path) {
                return parent
            }
        }
        if let inbox = try? await actions.ensureFolder(named: config.inboxFolderName, under: root) {
            return inbox
        }
        return root
    }

    private func folderLabel(for folder: URL) async -> String {
        let config = await module.loadConfig()
        guard let root = config.root else { return folder.lastPathComponent }
        let rootPath = root.standardizedFileURL.path
        let folderPath = folder.standardizedFileURL.path
        if folderPath == rootPath { return "Notes root" }
        if folderPath.hasPrefix(rootPath + "/") {
            return String(folderPath.dropFirst(rootPath.count + 1))
        }
        return folder.lastPathComponent
    }

    private func presentCreateNotePrompt(in folder: URL, templates: [NotesTemplateInfo]) {
        guard let window = detailView.window else { return }
        let alert = NSAlert()
        alert.messageText = "New Note"
        Task { [weak self] in
            let label = await self?.folderLabel(for: folder) ?? folder.lastPathComponent
            await MainActor.run {
                self?.showCreateNoteAlert(alert, in: window, folder: folder, folderLabel: label, templates: templates)
            }
        }
    }

    private func showCreateNoteAlert(
        _ alert: NSAlert,
        in window: NSWindow,
        folder: URL,
        folderLabel: String,
        templates: [NotesTemplateInfo]
    ) {
        alert.informativeText = "In \(folderLabel)"

        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 240, height: 24))
        field.placeholderString = "Note title"
        field.font = TypographyTokens.body

        var accessory: NSView = field
        var templatePopup: NSPopUpButton?
        if !templates.isEmpty {
            let stack = NSStackView()
            stack.orientation = .vertical
            stack.spacing = 8
            stack.translatesAutoresizingMaskIntoConstraints = false
            stack.addArrangedSubview(field)

            let popup = NSPopUpButton(frame: .zero, pullsDown: false)
            popup.addItem(withTitle: "Empty note")
            for template in templates {
                popup.addItem(withTitle: template.name)
            }
            popup.selectItem(at: 0)
            templatePopup = popup
            stack.addArrangedSubview(popup)
            accessory = stack
        }

        alert.accessoryView = accessory
        alert.addButton(withTitle: "Create")
        alert.addButton(withTitle: "Cancel")
        alert.beginSheetModal(for: window) { [weak self] response in
            guard response == .alertFirstButtonReturn, let self else { return }
            let name = field.stringValue
            let template = templatePopup.flatMap { popup in
                let index = popup.indexOfSelectedItem
                guard index > 0, templates.indices.contains(index - 1) else { return nil as NotesTemplateInfo? }
                return templates[index - 1]
            }
            Task {
                if let template {
                    await self.performCreateNoteFromTemplate(name: name, template: template, in: folder)
                } else {
                    await self.performCreateNote(name: name, in: folder)
                }
            }
        }
    }

    private func ensureActions() async -> NotesActions? {
        if actions == nil {
            actions = NotesActions(index: await module.treeIndex())
        }
        return actions
    }

    private func performCreateNote(name: String, in folder: URL) async {
        guard let actions = await ensureActions() else { return }
        do {
            let url = try await actions.createNote(name: name, inFolder: folder)
            await refreshTree()
            selectPath(url.path, expandParent: true)
            openNote(NotesNode(path: url.path, name: url.deletingPathExtension().lastPathComponent, kind: .note, children: []))
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

    private func performCreateNoteFromTemplate(
        name: String,
        template: NotesTemplateInfo,
        in folder: URL
    ) async {
        guard let actions = await ensureActions() else { return }
        do {
            let url = try await actions.createNoteFromTemplate(template: template, title: name, inFolder: folder)
            await refreshTree()
            selectPath(url.path, expandParent: true)
            openNote(NotesNode(path: url.path, name: url.deletingPathExtension().lastPathComponent, kind: .note, children: []))
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
        guard let actions = await ensureActions() else { return }
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
            Task { await confirmFolderDelete(item: item) }
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

    private func confirmFolderDelete(item: NotesOutlineItem) async {
        guard let actions else { return }
        let url = URL(fileURLWithPath: item.node.path)
        let isEmpty = (try? await actions.isFolderEmpty(url)) ?? false
        await MainActor.run { [weak self] in
            guard let self, let window = detailView.window else { return }
            let alert = NSAlert()
            if isEmpty {
                alert.messageText = "Move folder to Trash?"
                alert.informativeText = item.node.name
                alert.addButton(withTitle: "Cancel")
                alert.addButton(withTitle: "Move to Trash")
                if let button = alert.buttons.last {
                    button.hasDestructiveAction = true
                }
                alert.beginSheetModal(for: window) { [weak self] response in
                    guard response == .alertSecondButtonReturn else { return }
                    Task { await self?.performTrash(item: item) }
                }
            } else {
                alert.messageText = "Cannot delete “\(item.node.name)”"
                alert.informativeText = "This folder is not empty. Move or delete its contents first."
                alert.addButton(withTitle: "OK")
                alert.beginSheetModal(for: window)
            }
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

    @objc private func moveSelectedToFolder() {
        guard let item = selectedActionItem(), item.node.kind == .note else { return }
        Task { [weak self] in
            guard let self, let root = await module.loadConfig().root else { return }
            await MainActor.run {
                let panel = NSOpenPanel()
                panel.canChooseDirectories = true
                panel.canChooseFiles = false
                panel.allowsMultipleSelection = false
                panel.directoryURL = root
                panel.prompt = "Move Here"
                panel.message = "Choose destination folder"
                guard let window = self.detailView.window else { return }
                panel.beginSheetModal(for: window) { response in
                    guard response == .OK, let folder = panel.url else { return }
                    Task { await self.performMove(item: item, to: folder) }
                }
            }
        }
    }

    private func performMove(item: NotesOutlineItem, to folder: URL) async {
        guard let actions else { return }
        do {
            let url = try await actions.move(URL(fileURLWithPath: item.node.path), toFolder: folder)
            await refreshTree()
            selectPath(url.path, expandParent: true)
        } catch NotesActionError.alreadyExists {
            showError("A file with that name already exists in the destination folder.")
        } catch {
            showError(error.localizedDescription)
        }
    }

    @objc private func showExperimentalMindMap() {
        viewModeControl.isHidden = false
        viewModeControl.selectedSegment = 1
        viewModeChanged()
    }

    @objc private func findBacklinksForSelected() {
        guard let item = selectedActionItem(), item.node.kind == .note else { return }
        let target = item.node.name
        Task { [weak self] in
            guard let self, let actions else { return }
            let links = await actions.findBacklinks(to: target)
            await MainActor.run {
                if links.isEmpty {
                    self.showError("No notes link to “\(target)” via [[\(target)]].")
                } else {
                    self.showNoteListPopover(title: "Backlinks to \(target)", links: links, anchoredTo: item)
                }
            }
        }
    }

    @objc private func openLinkedNotes() {
        guard let item = selectedActionItem(), item.node.kind == .note else { return }
        Task { [weak self] in
            guard let self, let actions else { return }
            let links = await actions.relatedNotes(in: URL(fileURLWithPath: item.node.path))
            guard !links.isEmpty else { return }
            await MainActor.run {
                self.showNoteListPopover(title: "Linked notes", links: links, anchoredTo: item)
            }
        }
    }

    private func showNoteListPopover(title: String, links: [URL], anchoredTo item: NotesOutlineItem) {
        let controller = NSViewController()
        let list = NSTableView()
        list.headerView = nil
        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("link"))
        column.title = title
        list.addTableColumn(column)
        let dataSource = LinkedNotesDataSource(urls: links)
        list.dataSource = dataSource
        list.delegate = dataSource
        controller.view = list
        let popover = NSPopover()
        popover.contentSize = NSSize(width: 260, height: min(links.count, 8) * 24 + 8)
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
                dataSource.suppressExpansionCallbacks = true
                outlineView.expandItem(parent)
                dataSource.suppressExpansionCallbacks = false
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
            menu.addItem(withTitle: "Open Linked Notes…", action: #selector(openLinkedNotes), keyEquivalent: "")
            let backlinksItem = menu.addItem(
                withTitle: "Find Backlinks…",
                action: #selector(findBacklinksForSelected),
                keyEquivalent: "l"
            )
            backlinksItem.keyEquivalentModifierMask = .command
            menu.addItem(withTitle: "Rename", action: #selector(showRenamePrompt), keyEquivalent: "")
            if activePanel == .inbox {
                menu.addItem(withTitle: "Move to Folder…", action: #selector(moveSelectedToFolder), keyEquivalent: "")
            }
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
    private let workspace = WorkspaceService()

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
        Task { await workspace.openURL(urls[row]) }
    }
}
