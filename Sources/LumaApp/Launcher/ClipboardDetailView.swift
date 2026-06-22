import AppKit
import LumaCore
import LumaModules

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
    private var refreshTask: Task<Void, Never>?
    private var currentFilter: ClipboardListFilter = .all
    private var onOpenSettings: (() -> Void)?

    init(module: ClipboardModule, onOpenSettings: (() -> Void)? = nil) {
        self.module = module
        self.onOpenSettings = onOpenSettings
        let container = NSView()
        container.translatesAutoresizingMaskIntoConstraints = false
        self.detailView = container
        super.init()
        setup(container: container)
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

    private func setup(container: NSView) {
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
        tableView.onDelete = { [weak self] in
            guard let self else { return }
            self.deleteEntry(at: self.tableView.selectedRow)
        }
        tableView.onPin = { [weak self] in
            guard let self else { return }
            self.togglePin(at: self.tableView.selectedRow)
        }
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

        container.addSubview(toolbar)
        container.addSubview(tableScroll)
        container.addSubview(emptyStateLabel)

        NSLayoutConstraint.activate([
            toolbar.topAnchor.constraint(equalTo: container.topAnchor, constant: 4),
            toolbar.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 12),
            toolbar.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -12),
            toolbar.heightAnchor.constraint(equalToConstant: 32),

            tableScroll.topAnchor.constraint(equalTo: toolbar.bottomAnchor, constant: 8),
            tableScroll.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 8),
            tableScroll.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -8),
            tableScroll.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -8),

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

        filterControl.segmentCount = 4
        filterControl.setLabel("All", forSegment: 0)
        filterControl.setLabel("Text", forSegment: 1)
        filterControl.setLabel("Links", forSegment: 2)
        filterControl.setLabel("Pinned", forSegment: 3)
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
        case 1: currentFilter = .text
        case 2: currentFilter = .links
        case 3: currentFilter = .pinned
        default: currentFilter = .all
        }
        refresh()
    }

    @objc private func clearUnpinned() {
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
        let row = tableView.selectedRow
        guard entries.indices.contains(row) else { return }
        copyEntry(entries[row])
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
                self.tableView.reloadData()
                self.updateEmptyState(query: query)
                if !self.entries.isEmpty {
                    self.tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
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
        NSPasteboard.general.setString(entry.text, forType: .string)
    }

    private func togglePin(at row: Int) {
        guard entries.indices.contains(row) else { return }
        let id = entries[row].id
        Task { [weak self] in
            guard let self else { return }
            await self.module.togglePin(id)
            await MainActor.run { self.refresh() }
        }
    }

    private func deleteEntry(at row: Int) {
        guard entries.indices.contains(row) else { return }
        let id = entries[row].id
        Task { [weak self] in
            guard let self else { return }
            await self.module.remove(id)
            await MainActor.run { self.refresh() }
        }
    }

    private func pinnedSectionLabelRow(for row: Int) -> Bool {
        guard row < entries.count else { return false }
        let entry = entries[row]
        guard entry.isPinned else { return false }
        if row == 0 { return true }
        return !entries[row - 1].isPinned
    }
}

extension ClipboardDetailView: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int {
        entries.count
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard entries.indices.contains(row) else { return nil }
        let entry = entries[row]
        let cellID = NSUserInterfaceItemIdentifier("ClipboardRowCell")
        let cell = tableView.makeView(withIdentifier: cellID, owner: self) as? ClipboardRowCell
            ?? ClipboardRowCell()
        cell.identifier = cellID
        let showSection = pinnedSectionLabelRow(for: row)
        cell.configure(
            entry: entry,
            showsPinnedSection: showSection,
            onCopy: { [weak self] in self?.copyEntry(entry) },
            onPin: { [weak self] in self?.togglePin(at: row) },
            onDelete: { [weak self] in self?.deleteEntry(at: row) }
        )
        return cell
    }

    func tableView(_ tableView: NSTableView, heightOfRow row: Int) -> CGFloat {
        pinnedSectionLabelRow(for: row) ? 72 : 56
    }

    func tableViewSelectionDidChange(_ notification: Notification) {}
}

@MainActor
private final class ClipboardEntriesTableView: NSTableView {
    var onCopy: (() -> Void)?
    var onDelete: (() -> Void)?
    var onPin: (() -> Void)?

    override func keyDown(with event: NSEvent) {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        switch event.keyCode {
        case 36, 76:
            onCopy?()
        case 51:
            onDelete?()
        default:
            if flags.contains(.command), event.charactersIgnoringModifiers?.lowercased() == "p" {
                onPin?()
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
    private let previewLabel = NSTextField(wrappingLabelWithString: "")
    private let metaLabel = NSTextField(labelWithString: "")
    private let actionsStack = NSStackView()
    private let copyButton = NSButton()
    private let pinButton = NSButton()
    private let deleteButton = NSButton()
    private var onCopy: (() -> Void)?
    private var onPin: (() -> Void)?
    private var onDelete: (() -> Void)?

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
        onCopy: @escaping () -> Void,
        onPin: @escaping () -> Void,
        onDelete: @escaping () -> Void
    ) {
        self.onCopy = onCopy
        self.onPin = onPin
        self.onDelete = onDelete

        sectionLabel.isHidden = !showsPinnedSection
        iconView.image = NSImage(systemSymbolName: symbolName(for: entry.detectedKind), accessibilityDescription: nil)
        previewLabel.stringValue = previewText(for: entry.text)
        metaLabel.stringValue = metadata(for: entry)
        pinButton.image = NSImage(systemSymbolName: entry.isPinned ? "pin.fill" : "pin", accessibilityDescription: "Pin")
        pinButton.contentTintColor = entry.isPinned ? .systemOrange : .secondaryLabelColor
    }

    private func setup() {
        wantsLayer = true
        layer?.cornerRadius = 8

        sectionLabel.font = .systemFont(ofSize: 11, weight: .semibold)
        sectionLabel.textColor = .tertiaryLabelColor
        sectionLabel.translatesAutoresizingMaskIntoConstraints = false

        iconView.contentTintColor = .secondaryLabelColor
        iconView.translatesAutoresizingMaskIntoConstraints = false

        previewLabel.font = .systemFont(ofSize: 13, weight: .medium)
        previewLabel.maximumNumberOfLines = 2
        previewLabel.lineBreakMode = .byTruncatingTail
        previewLabel.translatesAutoresizingMaskIntoConstraints = false

        metaLabel.font = .systemFont(ofSize: 11)
        metaLabel.textColor = .secondaryLabelColor
        metaLabel.translatesAutoresizingMaskIntoConstraints = false

        for button in [copyButton, pinButton, deleteButton] {
            button.bezelStyle = .regularSquare
            button.isBordered = false
            button.alphaValue = 0.85
        }
        copyButton.image = NSImage(systemSymbolName: "doc.on.doc", accessibilityDescription: "Copy")
        deleteButton.image = NSImage(systemSymbolName: "trash", accessibilityDescription: "Delete")
        copyButton.target = self
        copyButton.action = #selector(copyTapped)
        pinButton.target = self
        pinButton.action = #selector(pinTapped)
        deleteButton.target = self
        deleteButton.action = #selector(deleteTapped)

        actionsStack.orientation = .horizontal
        actionsStack.spacing = 4
        actionsStack.translatesAutoresizingMaskIntoConstraints = false
        actionsStack.addArrangedSubview(copyButton)
        actionsStack.addArrangedSubview(pinButton)
        actionsStack.addArrangedSubview(deleteButton)

        addSubview(sectionLabel)
        addSubview(iconView)
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

            previewLabel.leadingAnchor.constraint(equalTo: iconView.trailingAnchor, constant: 10),
            previewLabel.topAnchor.constraint(equalTo: iconView.topAnchor, constant: -2),
            previewLabel.trailingAnchor.constraint(equalTo: actionsStack.leadingAnchor, constant: -8),

            metaLabel.leadingAnchor.constraint(equalTo: previewLabel.leadingAnchor),
            metaLabel.topAnchor.constraint(equalTo: previewLabel.bottomAnchor, constant: 2),
            metaLabel.trailingAnchor.constraint(equalTo: previewLabel.trailingAnchor),

            actionsStack.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            actionsStack.centerYAnchor.constraint(equalTo: centerYAnchor, constant: 6)
        ])
    }

    @objc private func copyTapped() { onCopy?() }
    @objc private func pinTapped() { onPin?() }
    @objc private func deleteTapped() { onDelete?() }

    private func previewText(for text: String) -> String {
        let collapsed = text.replacingOccurrences(of: "\n", with: " ")
        if collapsed.count <= 120 { return collapsed }
        return String(collapsed.prefix(120)) + "…"
    }

    private func metadata(for entry: ClipboardEntry) -> String {
        let ago = RelativeDateTimeFormatter().localizedString(for: entry.createdAt, relativeTo: Date())
        let chars = "\(entry.text.count) chars"
        let pin = entry.isPinned ? " · Pinned" : ""
        if let app = entry.sourceAppName {
            return "\(ago) · \(chars)\(pin) · \(app)"
        }
        return "\(ago) · \(chars)\(pin)"
    }

    private func symbolName(for kind: ClipboardEntryKind) -> String {
        switch kind {
        case .text: return "text.alignleft"
        case .link: return "link"
        case .email: return "envelope"
        case .code: return "chevron.left.forwardslash.chevron.right"
        }
    }
}
