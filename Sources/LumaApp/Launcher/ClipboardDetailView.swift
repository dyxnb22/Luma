import AppKit
import LumaCore
import LumaModules
import LumaServices

@MainActor
final class ClipboardDetailView: NSObject, ModuleDetailView {
    let moduleTitle = "Clipboard"
    let detailView: NSView
    let usesSharedTopBar = true

    private let module: ClipboardModule
    private let searchField = NSSearchField()
    private let filterControl = NSSegmentedControl()
    private let clearUnpinnedButton = NSButton()
    private let tableScroll = NSScrollView()
    private let tableView = ClipboardEntriesTableView()
    private let emptyStateLabel = NSTextField(labelWithString: "")
    private var entries: [ClipboardEntry] = []
    private var displayRows: [ClipboardDisplayRow] = []
    private var refreshTask: Task<Void, Never>?
    private var currentFilter: ClipboardListFilter = .all
    private var onOpenSettings: (() -> Void)?
    private var onHideLauncher: (() -> Void)?

    init(module: ClipboardModule, onOpenSettings: (() -> Void)? = nil, onHideLauncher: (() -> Void)? = nil) {
        self.module = module
        self.onOpenSettings = onOpenSettings
        self.onHideLauncher = onHideLauncher
        let chrome = BaseDetailContainer()
        self.detailView = chrome
        super.init()
        setup(chrome: chrome)
    }

    func activate() {
        refresh()
        DispatchQueue.main.async { [weak self] in
            self?.tableView.window?.makeFirstResponder(self?.tableView)
        }
    }

    func deactivate() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    func handleKeyDown(_ event: NSEvent) -> Bool {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if flags.contains(.command), event.charactersIgnoringModifiers?.lowercased() == "f" {
            detailView.window?.makeFirstResponder(searchField)
            return true
        }
        return false
    }

    private func setup(chrome: BaseDetailContainer) {
        let toolbar = buildToolbar()

        tableView.headerView = nil
        GeekUIKit.configureDetailTable(tableView, rowHeight: 56)
        tableView.selectionHighlightStyle = .regular
        tableView.delegate = self
        tableView.dataSource = self
        tableView.onCopy = { [weak self] in self?.copySelected() }
        tableView.onCopyPlainText = { [weak self] in self?.copySelectedPlainText() }
        tableView.onPaste = { [weak self] in self?.pasteSelected() }
        tableView.onDelete = { [weak self] in
            guard let self else { return }
            self.deleteEntry(at: self.tableView.selectedRow)
        }
        tableView.onPin = { [weak self] in
            guard let self else { return }
            self.togglePin(at: self.tableView.selectedRow)
        }
        tableView.target = self
        tableView.doubleAction = #selector(doubleClickRow)
        tableView.columnAutoresizingStyle = .uniformColumnAutoresizingStyle

        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("main"))
        column.resizingMask = .autoresizingMask
        tableView.addTableColumn(column)

        tableScroll.documentView = tableView
        tableScroll.hasVerticalScroller = true
        tableScroll.drawsBackground = false
        tableScroll.borderType = .noBorder
        tableScroll.translatesAutoresizingMaskIntoConstraints = false

        emptyStateLabel.font = TypographyTokens.body
        emptyStateLabel.textColor = .secondaryLabelColor
        emptyStateLabel.alignment = .center
        emptyStateLabel.maximumNumberOfLines = 3
        emptyStateLabel.isHidden = true
        emptyStateLabel.translatesAutoresizingMaskIntoConstraints = false

        chrome.setToolbar(toolbar, height: LauncherChromeTokens.detailToolbarHeight)
        chrome.setContent(tableScroll, embedInScroll: false)
        chrome.addSubview(emptyStateLabel)

        NSLayoutConstraint.activate([
            emptyStateLabel.centerXAnchor.constraint(equalTo: tableScroll.centerXAnchor),
            emptyStateLabel.centerYAnchor.constraint(equalTo: tableScroll.centerYAnchor),
            emptyStateLabel.leadingAnchor.constraint(greaterThanOrEqualTo: tableScroll.leadingAnchor, constant: 24),
            emptyStateLabel.trailingAnchor.constraint(lessThanOrEqualTo: tableScroll.trailingAnchor, constant: -24)
        ])

        searchField.target = self
        searchField.action = #selector(searchChanged)
        NotificationCenter.default.addObserver(self, selector: #selector(searchChanged), name: NSSearchField.textDidChangeNotification, object: searchField)
    }

    private func buildToolbar() -> NSView {
        let toolbar = NSView()
        toolbar.translatesAutoresizingMaskIntoConstraints = false

        searchField.placeholderString = "Search clipboard…"
        GeekUIKit.styleDetailSearchField(searchField)
        searchField.translatesAutoresizingMaskIntoConstraints = false

        filterControl.segmentCount = 3
        filterControl.setLabel("All", forSegment: 0)
        filterControl.setLabel("Pinned", forSegment: 1)
        filterControl.setLabel("Image", forSegment: 2)
        filterControl.selectedSegment = 0
        filterControl.target = self
        filterControl.action = #selector(filterChanged)
        filterControl.translatesAutoresizingMaskIntoConstraints = false

        clearUnpinnedButton.title = "Clear Unpinned"
        GeekUIKit.styleToolbarButton(clearUnpinnedButton)
        clearUnpinnedButton.target = self
        clearUnpinnedButton.action = #selector(clearUnpinned)
        clearUnpinnedButton.translatesAutoresizingMaskIntoConstraints = false

        let clearRecentButton = NSButton(title: "Clear Recent…", target: self, action: #selector(clearRecent))
        GeekUIKit.styleToolbarButton(clearRecentButton)
        clearRecentButton.translatesAutoresizingMaskIntoConstraints = false

        let settingsButton = NSButton(title: "Settings", target: self, action: #selector(openSettings))
        GeekUIKit.styleToolbarButton(settingsButton)
        settingsButton.translatesAutoresizingMaskIntoConstraints = false

        toolbar.addSubview(searchField)
        toolbar.addSubview(filterControl)
        toolbar.addSubview(clearUnpinnedButton)
        toolbar.addSubview(clearRecentButton)
        toolbar.addSubview(settingsButton)

        NSLayoutConstraint.activate([
            searchField.leadingAnchor.constraint(equalTo: toolbar.leadingAnchor),
            searchField.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            searchField.widthAnchor.constraint(equalToConstant: 180),

            filterControl.leadingAnchor.constraint(equalTo: searchField.trailingAnchor, constant: 10),
            filterControl.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),

            clearUnpinnedButton.leadingAnchor.constraint(equalTo: filterControl.trailingAnchor, constant: 10),
            clearUnpinnedButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),

            clearRecentButton.leadingAnchor.constraint(equalTo: clearUnpinnedButton.trailingAnchor, constant: 8),
            clearRecentButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),

            settingsButton.trailingAnchor.constraint(equalTo: toolbar.trailingAnchor),
            settingsButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor)
        ])
        return toolbar
    }

    @objc private func searchChanged() {
        refresh()
    }

    @objc private func filterChanged() {
        switch filterControl.selectedSegment {
        case 1: currentFilter = .pinned
        case 2: currentFilter = .image
        default: currentFilter = .all
        }
        refresh()
    }

    @objc private func clearUnpinned() {
        let alert = NSAlert()
        alert.messageText = "Clear unpinned clipboard history?"
        alert.informativeText = "Pinned items will be kept."
        alert.addButton(withTitle: "Clear")
        alert.addButton(withTitle: "Cancel")
        alert.alertStyle = .warning
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        Task { [weak self] in
            guard let self else { return }
            await self.module.clearUnpinned()
            await MainActor.run { self.refresh() }
        }
    }

    @objc private func clearRecent() {
        let alert = NSAlert()
        alert.messageText = "Clear recent clipboard items?"
        alert.informativeText = "Pinned items are kept. Choose a time window."
        alert.addButton(withTitle: "Last 5 minutes")
        alert.addButton(withTitle: "Last hour")
        alert.addButton(withTitle: "Today")
        alert.addButton(withTitle: "Cancel")
        alert.alertStyle = .warning
        let response = alert.runModal()
        let window: ClipboardRecentClearWindow?
        switch response {
        case .alertFirstButtonReturn: window = .last5Minutes
        case .alertSecondButtonReturn: window = .lastHour
        case .alertThirdButtonReturn: window = .today
        default: window = nil
        }
        guard let window else { return }
        Task { [weak self] in
            guard let self else { return }
            await self.module.clearRecent(window)
            await MainActor.run { self.refresh() }
        }
    }

    @objc private func openSettings() {
        onOpenSettings?()
    }

    @objc private func copySelected() {
        guard let entry = selectedEntry() else { return }
        Task { try? await module.copyEntry(id: entry.id) }
    }

    @objc private func copySelectedPlainText() {
        guard let entry = selectedEntry() else { return }
        guard entry.imageData == nil, entry.fileURLs?.isEmpty != false else { return }
        Task { try? await module.copyEntry(id: entry.id, plainTextOnly: true) }
    }

    @objc private func doubleClickRow() {
        guard let entry = selectedEntry() else { return }
        pasteEntry(entry)
    }

    @objc private func pasteSelected() {
        guard let entry = selectedEntry() else { return }
        pasteEntry(entry)
    }

    private func pasteEntry(_ entry: ClipboardEntry) {
        onHideLauncher?()
        Task { try? await module.pasteEntry(id: entry.id) }
    }

    private func selectedEntry() -> ClipboardEntry? {
        let row = tableView.selectedRow
        guard displayRows.indices.contains(row) else { return nil }
        if case .entry(let entry) = displayRows[row] { return entry }
        return nil
    }

    private func refresh() {
        refreshTask?.cancel()
        let query = searchField.stringValue
        let filter = currentFilter
        refreshTask = Task { [weak self] in
            guard let self else { return }
            let loaded = await self.module.filteredEntries(filter: filter, query: query, limit: 200)
            await MainActor.run {
                self.entries = loaded
                self.displayRows = ClipboardTimeGrouping.displayRows(for: loaded)
                self.tableView.reloadData()
                self.updateEmptyState(query: query)
                if let firstEntry = self.displayRows.firstIndex(where: {
                    if case .entry = $0 { return true }
                    return false
                }) {
                    self.tableView.selectRowIndexes(IndexSet(integer: firstEntry), byExtendingSelection: false)
                }
            }
        }
    }

    private func updateEmptyState(query: String) {
        if entries.isEmpty {
            emptyStateLabel.isHidden = false
            if query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                emptyStateLabel.stringValue = "Clipboard history is empty.\nCopied text will appear here."
            } else {
                emptyStateLabel.stringValue = "No results for “\(query)”."
            }
        } else {
            emptyStateLabel.isHidden = true
        }
    }

    private func togglePin(at row: Int) {
        guard displayRows.indices.contains(row) else { return }
        guard case .entry(let entry) = displayRows[row] else { return }
        Task { [weak self] in
            guard let self else { return }
            await self.module.togglePin(entry.id)
            await MainActor.run { self.refresh() }
        }
    }

    private func deleteEntry(at row: Int) {
        guard displayRows.indices.contains(row) else { return }
        guard case .entry(let entry) = displayRows[row] else { return }
        Task { [weak self] in
            guard let self else { return }
            await self.module.remove(entry.id)
            await MainActor.run { self.refresh() }
        }
    }

    private func saveAsSnippet(_ entry: ClipboardEntry) {
        let draft = SnippetDraft.fromClipboard(entry.plainTextForCopy)
        let sheet = SnippetEditorSheet(snippet: nil, draft: draft) { [weak self] savedTitle, trigger, content, tags in
            guard let mod = LauncherEnvironment.current?.snippetsModule else { return }
            Task {
                let saved = try? await mod.add(title: savedTitle, content: content, tags: tags, trigger: trigger)
                await MainActor.run {
                    LauncherEnvironment.current?.showStatus?(
                        saved == nil ? LauncherStatusMessages.snippetSaveFailed : LauncherStatusMessages.snippetCreated
                    )
                    LauncherEnvironment.current?.detailReloadRouter.reload(.snippets)
                    self?.detailView.window?.makeFirstResponder(self?.tableView)
                }
            }
        }
        if let window = detailView.window {
            window.beginSheet(sheet) { _ in }
        }
    }

    private func saveAsNote(_ entry: ClipboardEntry) {
        let text = entry.plainTextForCopy.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        Task {
            _ = await NotesCaptureHelper.appendToDailyNote(text)
        }
    }

    private func pinnedSectionLabelRow(for row: Int) -> Bool { false }
}

extension ClipboardDetailView: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int {
        displayRows.count
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard displayRows.indices.contains(row) else { return nil }
        switch displayRows[row] {
        case .header(let title):
            let label = NSTextField(labelWithString: "")
            GeekUIKit.styleDetailSectionHeaderLabel(label, title: title)
            return label
        case .entry(let entry):
            let cellID = NSUserInterfaceItemIdentifier("ClipboardRowCell")
            let cell = tableView.makeView(withIdentifier: cellID, owner: self) as? ClipboardRowCell
                ?? ClipboardRowCell()
            cell.identifier = cellID
            let entryRow = row
            cell.configure(
                entry: entry,
                showsPinnedSection: false,
                imageLayout: currentFilter == .image,
                onCopy: { [weak self] in
                    guard let self else { return }
                    Task { try? await self.module.copyEntry(id: entry.id) }
                },
                onPin: { [weak self] in self?.togglePin(at: entryRow) },
                onDelete: { [weak self] in self?.deleteEntry(at: entryRow) },
                onSaveAsSnippet: { [weak self] in self?.saveAsSnippet(entry) },
                onSaveAsNote: entry.imageData == nil ? { [weak self] in self?.saveAsNote(entry) } : nil
            )
            return cell
        }
    }

    func tableView(_ tableView: NSTableView, heightOfRow row: Int) -> CGFloat {
        guard displayRows.indices.contains(row) else { return 56 }
        if case .header = displayRows[row] { return 24 }
        if currentFilter == .image { return 72 }
        return 56
    }

    func tableView(_ tableView: NSTableView, isGroupRow row: Int) -> Bool {
        guard displayRows.indices.contains(row) else { return false }
        if case .header = displayRows[row] { return true }
        return false
    }

    func tableView(_ tableView: NSTableView, shouldSelectRow row: Int) -> Bool {
        guard displayRows.indices.contains(row) else { return false }
        if case .header = displayRows[row] { return false }
        return true
    }

    func tableViewSelectionDidChange(_ notification: Notification) {}
}

@MainActor
private final class ClipboardEntriesTableView: NSTableView {
    var onCopy: (() -> Void)?
    var onCopyPlainText: (() -> Void)?
    var onDelete: (() -> Void)?
    var onPin: (() -> Void)?
    var onPaste: (() -> Void)?

    override func keyDown(with event: NSEvent) {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        switch event.keyCode {
        case 36, 76:
            onPaste?()
        case 51:
            onDelete?()
        default:
            if flags.contains(.command), event.charactersIgnoringModifiers?.lowercased() == "p" {
                onPin?()
            } else if flags.contains(.command), flags.contains(.shift), event.charactersIgnoringModifiers?.lowercased() == "c" {
                onCopyPlainText?()
            } else if flags.contains(.command), event.charactersIgnoringModifiers?.lowercased() == "c" {
                onCopy?()
            } else {
                super.keyDown(with: event)
            }
        }
    }
}

@MainActor
private final class ClipboardRowCell: NSTableCellView {
    private let sectionLabel = NSTextField(labelWithString: "Pinned")
    private let iconView = NSImageView()
    private let thumbnailView = NSImageView()
    private let previewLabel = NSTextField(wrappingLabelWithString: "")
    private let metaLabel = NSTextField(labelWithString: "")
    private let actionsStack = NSStackView()
    private let copyButton = NSButton()
    private let snippetButton = NSButton()
    private let noteButton = NSButton()
    private let pinButton = NSButton()
    private let deleteButton = NSButton()
    private var onCopy: (() -> Void)?
    private var onSaveAsSnippet: (() -> Void)?
    private var onSaveAsNote: (() -> Void)?
    private var onPin: (() -> Void)?
    private var onDelete: (() -> Void)?
    private var metaLeadingToPreview: NSLayoutConstraint!
    private var metaTopToPreview: NSLayoutConstraint!
    private var metaLeadingToThumbnail: NSLayoutConstraint!
    private var metaCenterYToThumbnail: NSLayoutConstraint!

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        setup()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func configure(
        entry: ClipboardEntry,
        showsPinnedSection: Bool,
        imageLayout: Bool = false,
        onCopy: @escaping () -> Void,
        onPin: @escaping () -> Void,
        onDelete: @escaping () -> Void,
        onSaveAsSnippet: @escaping () -> Void,
        onSaveAsNote: (() -> Void)? = nil
    ) {
        self.onCopy = onCopy
        self.onPin = onPin
        self.onDelete = onDelete
        self.onSaveAsSnippet = onSaveAsSnippet
        self.onSaveAsNote = onSaveAsNote
        noteButton.isHidden = onSaveAsNote == nil

        sectionLabel.isHidden = !showsPinnedSection
        let useThumbnail = imageLayout && entry.imageData != nil
        if useThumbnail, let data = entry.imageData, let image = NSImage(data: data) {
            thumbnailView.image = image
            thumbnailView.isHidden = false
            iconView.isHidden = true
            previewLabel.isHidden = true
        } else {
            thumbnailView.isHidden = true
            iconView.isHidden = false
            previewLabel.isHidden = false
            iconView.image = NSImage(systemSymbolName: entry.symbolName, accessibilityDescription: nil)
            previewLabel.stringValue = previewText(for: entry.displayText)
        }
        metaLabel.stringValue = entry.metadataLine
        pinButton.image = NSImage(systemSymbolName: entry.isPinned ? "pin.fill" : "pin", accessibilityDescription: "Pin")
        pinButton.contentTintColor = entry.isPinned ? .systemOrange : .secondaryLabelColor

        if useThumbnail {
            metaLeadingToPreview.isActive = false
            metaTopToPreview.isActive = false
            metaLeadingToThumbnail.isActive = true
            metaCenterYToThumbnail.isActive = true
        } else {
            metaLeadingToThumbnail.isActive = false
            metaCenterYToThumbnail.isActive = false
            metaLeadingToPreview.isActive = true
            metaTopToPreview.isActive = true
        }
    }

    private func setup() {
        GeekUIKit.configureDetailTableRowSurface(self)

        sectionLabel.font = TypographyTokens.caption2(weight: .semibold)
        sectionLabel.textColor = .tertiaryLabelColor
        sectionLabel.translatesAutoresizingMaskIntoConstraints = false

        iconView.contentTintColor = .secondaryLabelColor
        iconView.translatesAutoresizingMaskIntoConstraints = false

        thumbnailView.imageScaling = .scaleProportionallyUpOrDown
        thumbnailView.wantsLayer = true
        thumbnailView.layer?.cornerRadius = 6
        thumbnailView.layer?.masksToBounds = true
        thumbnailView.isHidden = true
        thumbnailView.translatesAutoresizingMaskIntoConstraints = false

        previewLabel.font = TypographyTokens.body
        previewLabel.maximumNumberOfLines = 2
        previewLabel.lineBreakMode = .byTruncatingTail
        previewLabel.translatesAutoresizingMaskIntoConstraints = false

        metaLabel.font = TypographyTokens.caption2()
        metaLabel.textColor = .secondaryLabelColor
        metaLabel.translatesAutoresizingMaskIntoConstraints = false

        for button in [copyButton, snippetButton, noteButton, pinButton, deleteButton] {
            button.bezelStyle = .regularSquare
            button.isBordered = false
            button.alphaValue = 0.85
        }
        copyButton.image = NSImage(systemSymbolName: "doc.on.doc", accessibilityDescription: "Copy")
        snippetButton.image = NSImage(systemSymbolName: "text.badge.plus", accessibilityDescription: "Create Snippet")
        noteButton.image = NSImage(systemSymbolName: "square.and.pencil", accessibilityDescription: "Append to Note")
        deleteButton.image = NSImage(systemSymbolName: "trash", accessibilityDescription: "Delete")
        copyButton.target = self
        copyButton.action = #selector(copyTapped)
        snippetButton.target = self
        snippetButton.action = #selector(snippetTapped)
        noteButton.target = self
        noteButton.action = #selector(noteTapped)
        pinButton.target = self
        pinButton.action = #selector(pinTapped)
        deleteButton.target = self
        deleteButton.action = #selector(deleteTapped)

        actionsStack.orientation = .horizontal
        actionsStack.spacing = 4
        actionsStack.translatesAutoresizingMaskIntoConstraints = false
        actionsStack.addArrangedSubview(copyButton)
        actionsStack.addArrangedSubview(noteButton)
        actionsStack.addArrangedSubview(snippetButton)
        actionsStack.addArrangedSubview(pinButton)
        actionsStack.addArrangedSubview(deleteButton)

        addSubview(sectionLabel)
        addSubview(iconView)
        addSubview(thumbnailView)
        addSubview(previewLabel)
        addSubview(metaLabel)
        addSubview(actionsStack)

        NSLayoutConstraint.activate([
            sectionLabel.topAnchor.constraint(equalTo: topAnchor, constant: 2),
            sectionLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: LauncherChromeTokens.detailTableRowPaddingH),

            iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: LauncherChromeTokens.detailTableRowPaddingH),
            iconView.topAnchor.constraint(equalTo: sectionLabel.bottomAnchor, constant: 4),
            iconView.widthAnchor.constraint(equalToConstant: 18),
            iconView.heightAnchor.constraint(equalToConstant: 18),

            thumbnailView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: LauncherChromeTokens.detailTableRowPaddingH),
            thumbnailView.centerYAnchor.constraint(equalTo: centerYAnchor, constant: 4),
            thumbnailView.widthAnchor.constraint(equalToConstant: 52),
            thumbnailView.heightAnchor.constraint(equalToConstant: 52),

            previewLabel.leadingAnchor.constraint(equalTo: iconView.trailingAnchor, constant: 10),
            previewLabel.topAnchor.constraint(equalTo: iconView.topAnchor, constant: -2),
            previewLabel.trailingAnchor.constraint(equalTo: actionsStack.leadingAnchor, constant: -8)
        ])

        metaLeadingToPreview = metaLabel.leadingAnchor.constraint(equalTo: previewLabel.leadingAnchor)
        metaTopToPreview = metaLabel.topAnchor.constraint(equalTo: previewLabel.bottomAnchor, constant: 2)
        metaLeadingToThumbnail = metaLabel.leadingAnchor.constraint(equalTo: thumbnailView.trailingAnchor, constant: 12)
        metaCenterYToThumbnail = metaLabel.centerYAnchor.constraint(equalTo: thumbnailView.centerYAnchor)
        metaLeadingToPreview.isActive = true
        metaTopToPreview.isActive = true
        NSLayoutConstraint.activate([
            metaLabel.trailingAnchor.constraint(equalTo: previewLabel.trailingAnchor),

            actionsStack.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            actionsStack.centerYAnchor.constraint(equalTo: centerYAnchor, constant: 6)
        ])
    }

    @objc private func copyTapped() { onCopy?() }
    @objc private func snippetTapped() { onSaveAsSnippet?() }
    @objc private func noteTapped() { onSaveAsNote?() }
    @objc private func pinTapped() { onPin?() }
    @objc private func deleteTapped() { onDelete?() }

    private func previewText(for text: String) -> String {
        let collapsed = text.replacingOccurrences(of: "\n", with: " ")
        if collapsed.count <= 120 { return collapsed }
        return String(collapsed.prefix(120)) + "…"
    }
}
