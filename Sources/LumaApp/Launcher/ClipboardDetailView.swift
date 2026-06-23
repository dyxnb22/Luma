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

    init(module: ClipboardModule, onOpenSettings: (() -> Void)? = nil) {
        self.module = module
        self.onOpenSettings = onOpenSettings
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
        tableView.style = .plain
        tableView.rowHeight = 56
        tableView.intercellSpacing = NSSize(width: 0, height: 4)
        tableView.backgroundColor = .clear
        tableView.selectionHighlightStyle = .regular
        tableView.usesAlternatingRowBackgroundColors = false
        tableView.delegate = self
        tableView.dataSource = self
        tableView.onCopy = { [weak self] in self?.copySelected() }
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

        emptyStateLabel.font = .systemFont(ofSize: 13)
        emptyStateLabel.textColor = .secondaryLabelColor
        emptyStateLabel.alignment = .center
        emptyStateLabel.maximumNumberOfLines = 3
        emptyStateLabel.isHidden = true
        emptyStateLabel.translatesAutoresizingMaskIntoConstraints = false

        chrome.setToolbar(toolbar, height: 32)
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
        clearUnpinnedButton.bezelStyle = .rounded
        clearUnpinnedButton.target = self
        clearUnpinnedButton.action = #selector(clearUnpinned)
        clearUnpinnedButton.translatesAutoresizingMaskIntoConstraints = false

        let settingsButton = NSButton(title: "Settings", target: self, action: #selector(openSettings))
        settingsButton.bezelStyle = .rounded
        settingsButton.translatesAutoresizingMaskIntoConstraints = false

        toolbar.addSubview(searchField)
        toolbar.addSubview(filterControl)
        toolbar.addSubview(clearUnpinnedButton)
        toolbar.addSubview(settingsButton)

        NSLayoutConstraint.activate([
            searchField.leadingAnchor.constraint(equalTo: toolbar.leadingAnchor),
            searchField.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            searchField.widthAnchor.constraint(equalToConstant: 180),

            filterControl.leadingAnchor.constraint(equalTo: searchField.trailingAnchor, constant: 10),
            filterControl.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),

            clearUnpinnedButton.leadingAnchor.constraint(equalTo: filterControl.trailingAnchor, constant: 10),
            clearUnpinnedButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),

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

    @objc private func openSettings() {
        onOpenSettings?()
    }

    @objc private func copySelected() {
        guard let entry = selectedEntry() else { return }
        copyEntry(entry)
    }

    @objc private func doubleClickRow() {
        guard let entry = selectedEntry() else { return }
        pasteEntry(entry)
    }

    @objc private func pasteSelected() {
        guard let entry = selectedEntry() else { return }
        pasteEntry(entry)
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

    private func copyEntry(_ entry: ClipboardEntry) {
        NSPasteboard.general.clearContents()
        if let data = entry.imageData, let type = entry.imagePasteboardType {
            NSPasteboard.general.setData(data, forType: NSPasteboard.PasteboardType(type))
        } else {
            NSPasteboard.general.setString(entry.text, forType: .string)
        }
    }

    private func pasteEntry(_ entry: ClipboardEntry) {
        copyEntry(entry)
        guard entry.imageData == nil else { return }
        LauncherCallbackRegistry.current?.onHideLauncher()
        let ax = AXService()
        let text = entry.text
        Task {
            try? await Task.sleep(for: .milliseconds(80))
            if AXService.isProcessTrusted() {
                await ax.insert(text: text)
            }
        }
    }

    private func togglePin(at row: Int) {
        guard case .entry(let entry) = displayRows[row] else { return }
        Task { [weak self] in
            guard let self else { return }
            await self.module.togglePin(entry.id)
            await MainActor.run { self.refresh() }
        }
    }

    private func deleteEntry(at row: Int) {
        guard case .entry(let entry) = displayRows[row] else { return }
        Task { [weak self] in
            guard let self else { return }
            await self.module.remove(entry.id)
            await MainActor.run { self.refresh() }
        }
    }

    private func saveAsSnippet(_ entry: ClipboardEntry) {
        let draftTitle = entry.text
            .split(separator: "\n", maxSplits: 1, omittingEmptySubsequences: true)
            .first
            .map(String.init)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? "Clipboard clip"
        let title = String(draftTitle.prefix(60))
        let draft = Snippet(title: title, content: entry.text, tags: ["clipboard"])
        let sheet = SnippetEditorSheet(snippet: draft) { [weak self] savedTitle, trigger, content, tags in
            guard let mod = ModuleDetailRegistry.snippetsModule else { return }
            Task {
                _ = try? await mod.add(title: savedTitle, content: content, tags: tags, trigger: trigger)
                await MainActor.run {
                    ModuleDetailReloads.reloadSnippetsDetail?()
                    self?.detailView.window?.makeFirstResponder(self?.tableView)
                }
            }
        }
        if let window = detailView.window {
            window.beginSheet(sheet) { _ in }
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
            let label = NSTextField(labelWithString: title)
            label.font = .systemFont(ofSize: 11, weight: .semibold)
            label.textColor = .tertiaryLabelColor
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
                onCopy: { [weak self] in self?.copyEntry(entry) },
                onPin: { [weak self] in self?.togglePin(at: entryRow) },
                onDelete: { [weak self] in self?.deleteEntry(at: entryRow) },
                onSaveAsSnippet: { [weak self] in self?.saveAsSnippet(entry) }
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
    private let pinButton = NSButton()
    private let deleteButton = NSButton()
    private var onCopy: (() -> Void)?
    private var onSaveAsSnippet: (() -> Void)?
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
        onSaveAsSnippet: @escaping () -> Void
    ) {
        self.onCopy = onCopy
        self.onPin = onPin
        self.onDelete = onDelete
        self.onSaveAsSnippet = onSaveAsSnippet

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
            iconView.image = NSImage(systemSymbolName: symbolName(for: entry.detectedKind), accessibilityDescription: nil)
            previewLabel.stringValue = previewText(for: entry.text)
        }
        metaLabel.stringValue = metadata(for: entry)
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
        wantsLayer = true
        layer?.cornerRadius = 8

        sectionLabel.font = .systemFont(ofSize: 11, weight: .semibold)
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

        previewLabel.font = .systemFont(ofSize: 13, weight: .medium)
        previewLabel.maximumNumberOfLines = 2
        previewLabel.lineBreakMode = .byTruncatingTail
        previewLabel.translatesAutoresizingMaskIntoConstraints = false

        metaLabel.font = .systemFont(ofSize: 11)
        metaLabel.textColor = .secondaryLabelColor
        metaLabel.translatesAutoresizingMaskIntoConstraints = false

        for button in [copyButton, snippetButton, pinButton, deleteButton] {
            button.bezelStyle = .regularSquare
            button.isBordered = false
            button.alphaValue = 0.85
        }
        copyButton.image = NSImage(systemSymbolName: "doc.on.doc", accessibilityDescription: "Copy")
        snippetButton.image = NSImage(systemSymbolName: "text.badge.plus", accessibilityDescription: "Save as Snippet")
        deleteButton.image = NSImage(systemSymbolName: "trash", accessibilityDescription: "Delete")
        copyButton.target = self
        copyButton.action = #selector(copyTapped)
        snippetButton.target = self
        snippetButton.action = #selector(snippetTapped)
        pinButton.target = self
        pinButton.action = #selector(pinTapped)
        deleteButton.target = self
        deleteButton.action = #selector(deleteTapped)

        actionsStack.orientation = .horizontal
        actionsStack.spacing = 4
        actionsStack.translatesAutoresizingMaskIntoConstraints = false
        actionsStack.addArrangedSubview(copyButton)
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
            sectionLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),

            iconView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            iconView.topAnchor.constraint(equalTo: sectionLabel.bottomAnchor, constant: 4),
            iconView.widthAnchor.constraint(equalToConstant: 18),
            iconView.heightAnchor.constraint(equalToConstant: 18),

            thumbnailView.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
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
    @objc private func pinTapped() { onPin?() }
    @objc private func deleteTapped() { onDelete?() }

    private func previewText(for text: String) -> String {
        let collapsed = text.replacingOccurrences(of: "\n", with: " ")
        if collapsed.count <= 120 { return collapsed }
        return String(collapsed.prefix(120)) + "…"
    }

    private func metadata(for entry: ClipboardEntry) -> String {
        let ago = RelativeDateTimeFormatter().localizedString(for: entry.createdAt, relativeTo: Date())
        let detail: String
        if entry.imageData != nil {
            let bytes = entry.imageData?.count ?? 0
            detail = bytes >= 1024 ? "\(bytes / 1024) KB image" : "\(bytes) B image"
        } else {
            detail = "\(entry.text.count) chars"
        }
        let pin = entry.isPinned ? " · Pinned" : ""
        if let app = entry.sourceAppName {
            return "\(ago) · \(detail)\(pin) · \(app)"
        }
        return "\(ago) · \(detail)\(pin)"
    }

    private func symbolName(for kind: ClipboardEntryKind) -> String {
        switch kind {
        case .text: return "text.alignleft"
        case .link: return "link"
        case .email: return "envelope"
        case .code: return "chevron.left.forwardslash.chevron.right"
        case .image: return "photo"
        }
    }
}
