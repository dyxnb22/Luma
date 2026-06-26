import AppKit
import LumaCore
import LumaModules

@MainActor
final class SecretsDetailView: NSObject, ModuleDetailView {
    let moduleTitle = "Secrets"
    let detailView: NSView
    let usesSharedTopBar = true

    private let module: SecretsModule
    private let detailReloadRouter: ModuleDetailReloadRouter
    private let searchField = NSSearchField()
    private let toolbarContainer = NSView()
    private let tableScroll = NSScrollView()
    private let tableView = NSTableView()
    private let emptyStateLabel = NSTextField(labelWithString: "")
    private let lockedContainer = NSView()
    private let unlockButton = NSButton()
    private var records: [SecretRecord] = []
    private var refreshTask: Task<Void, Never>?
    private var isUnlocked = false

    init(module: SecretsModule, detailReloadRouter: ModuleDetailReloadRouter) {
        self.module = module
        self.detailReloadRouter = detailReloadRouter
        let chrome = BaseDetailContainer()
        self.detailView = chrome
        super.init()
        setup(chrome: chrome)
    }

    func activate() {
        detailReloadRouter.register(.secrets) { [weak self] in self?.refresh() }
        refresh()
    }

    func deactivate() {
        detailReloadRouter.unregister(.secrets)
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
        lockedContainer.translatesAutoresizingMaskIntoConstraints = false
        unlockButton.title = "Unlock Vault"
        GeekUIKit.stylePrimaryButton(unlockButton)
        unlockButton.target = self
        unlockButton.action = #selector(unlockVault)
        unlockButton.translatesAutoresizingMaskIntoConstraints = false
        lockedContainer.addSubview(unlockButton)

        let toolbar = buildToolbar()
        toolbarContainer.addSubview(toolbar)
        toolbar.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            toolbar.topAnchor.constraint(equalTo: toolbarContainer.topAnchor),
            toolbar.leadingAnchor.constraint(equalTo: toolbarContainer.leadingAnchor),
            toolbar.trailingAnchor.constraint(equalTo: toolbarContainer.trailingAnchor),
            toolbar.bottomAnchor.constraint(equalTo: toolbarContainer.bottomAnchor),
            toolbar.heightAnchor.constraint(equalToConstant: LauncherChromeTokens.detailToolbarHeight)
        ])
        toolbarContainer.translatesAutoresizingMaskIntoConstraints = false

        tableView.headerView = NSTableHeaderView()
        GeekUIKit.configureDetailTable(tableView)
        tableView.delegate = self
        tableView.dataSource = self
        tableView.target = self
        tableView.doubleAction = #selector(editSelected)
        tableView.translatesAutoresizingMaskIntoConstraints = false

        for (id, title, width) in [
            ("label", "Label", 180.0),
            ("account", "Account", 140.0),
            ("updated", "Updated", 120.0)
        ] {
            let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(id))
            column.title = title
            column.width = width
            tableView.addTableColumn(column)
        }
        GeekUIKit.styleDetailTableColumns(tableView)

        tableScroll.documentView = tableView
        tableScroll.hasVerticalScroller = true
        tableScroll.drawsBackground = false
        tableScroll.borderType = .noBorder
        tableScroll.translatesAutoresizingMaskIntoConstraints = false

        GeekUIKit.configureEmptyStateLabel(emptyStateLabel, text: "")
        emptyStateLabel.isHidden = true
        emptyStateLabel.translatesAutoresizingMaskIntoConstraints = false

        chrome.setToolbar(toolbarContainer, height: LauncherChromeTokens.detailToolbarHeight)
        chrome.setContent(tableScroll, embedInScroll: false)
        chrome.addSubview(lockedContainer)
        chrome.addSubview(emptyStateLabel)

        NSLayoutConstraint.activate([
            lockedContainer.topAnchor.constraint(equalTo: chrome.topAnchor),
            lockedContainer.leadingAnchor.constraint(equalTo: chrome.leadingAnchor),
            lockedContainer.trailingAnchor.constraint(equalTo: chrome.trailingAnchor),
            lockedContainer.bottomAnchor.constraint(equalTo: chrome.bottomAnchor),
            unlockButton.centerXAnchor.constraint(equalTo: lockedContainer.centerXAnchor),
            unlockButton.centerYAnchor.constraint(equalTo: lockedContainer.centerYAnchor),

            emptyStateLabel.centerXAnchor.constraint(equalTo: tableScroll.centerXAnchor),
            emptyStateLabel.centerYAnchor.constraint(equalTo: tableScroll.centerYAnchor)
        ])

        searchField.target = self
        searchField.action = #selector(searchChanged)
        NotificationCenter.default.addObserver(self, selector: #selector(searchChanged), name: NSSearchField.textDidChangeNotification, object: searchField)
    }

    private func buildToolbar() -> NSView {
        let toolbar = NSView()
        toolbar.translatesAutoresizingMaskIntoConstraints = false

        searchField.placeholderString = "Search secrets…"
        GeekUIKit.styleDetailSearchField(searchField)
        searchField.translatesAutoresizingMaskIntoConstraints = false

        let addButton = GeekUIKit.makeToolbarButton("Add", target: self, action: #selector(addSecret))
        let editButton = GeekUIKit.makeToolbarButton("Edit", target: self, action: #selector(editSelected))
        let deleteButton = GeekUIKit.makeToolbarButton("Delete", target: self, action: #selector(deleteSelected))
        let revealButton = GeekUIKit.makeToolbarButton("Reveal", target: self, action: #selector(revealSelected))
        let lockButton = GeekUIKit.makeToolbarButton("Lock Vault", target: self, action: #selector(lockVault))

        let buttonStack = NSStackView(views: [addButton, editButton, deleteButton, revealButton, lockButton])
        buttonStack.orientation = .horizontal
        buttonStack.spacing = 8
        buttonStack.translatesAutoresizingMaskIntoConstraints = false

        toolbar.addSubview(searchField)
        toolbar.addSubview(buttonStack)

        NSLayoutConstraint.activate([
            searchField.leadingAnchor.constraint(equalTo: toolbar.leadingAnchor),
            searchField.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            searchField.widthAnchor.constraint(equalToConstant: 200),
            buttonStack.trailingAnchor.constraint(equalTo: toolbar.trailingAnchor),
            buttonStack.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor)
        ])
        return toolbar
    }

    @objc private func searchChanged() {
        refresh()
    }

    @objc private func unlockVault() {
        Task { [weak self] in
            guard let self else { return }
            await self.module.unlock()
            await MainActor.run { self.refresh() }
        }
    }

    @objc private func lockVault() {
        Task { [weak self] in
            guard let self else { return }
            await self.module.lock()
            await MainActor.run { self.refresh() }
        }
    }

    @objc private func addSecret() {
        presentEditor(record: nil)
    }

    @objc private func editSelected() {
        let row = tableView.selectedRow
        guard records.indices.contains(row) else { return }
        presentEditor(record: records[row])
    }

    @objc private func deleteSelected() {
        let row = tableView.selectedRow
        guard records.indices.contains(row) else { return }
        let record = records[row]
        let alert = NSAlert()
        alert.messageText = "Delete “\(record.label)”?"
        alert.informativeText = "This cannot be undone."
        alert.addButton(withTitle: "Delete")
        alert.addButton(withTitle: "Cancel")
        alert.alertStyle = .warning
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        Task { [weak self] in
            guard let self else { return }
            try? await self.module.delete(id: record.id)
            await MainActor.run { self.refresh() }
        }
    }

    @objc private func revealSelected() {
        let row = tableView.selectedRow
        guard records.indices.contains(row) else { return }
        let record = records[row]
        Task { [weak self] in
            guard let self else { return }
            guard let value = try? await self.module.revealValue(id: record.id) else { return }
            await MainActor.run {
                let alert = NSAlert()
                alert.messageText = record.label
                alert.informativeText = value
                alert.alertStyle = .informational
                alert.addButton(withTitle: "OK")
                alert.runModal()
            }
        }
    }

    private func presentEditor(record: SecretRecord?) {
        let sheet = SecretEditorSheet(record: record) { [weak self] label, account, value in
            guard let self else { return }
            Task {
                if let record {
                    try? await self.module.update(id: record.id, label: label, account: account, value: value)
                } else if let value {
                    _ = try? await self.module.save(label: label, account: account, value: value)
                }
                await MainActor.run { self.refresh() }
            }
        }
        if let window = detailView.window {
            window.beginSheet(sheet) { _ in }
        }
    }

    private func refresh() {
        refreshTask?.cancel()
        let query = searchField.stringValue
        refreshTask = Task { [weak self] in
            guard let self else { return }
            let unlocked = await self.module.isUnlocked()
            let loaded: [SecretRecord]
            if unlocked {
                if query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    loaded = (try? await self.module.allRecords()) ?? []
                } else {
                    loaded = (try? await self.module.allRecords())?
                        .filter {
                            $0.label.localizedCaseInsensitiveContains(query)
                                || $0.account.localizedCaseInsensitiveContains(query)
                        } ?? []
                }
            } else {
                loaded = []
            }
            await MainActor.run {
                self.isUnlocked = unlocked
                self.records = loaded
                self.lockedContainer.isHidden = unlocked
                self.toolbarContainer.isHidden = !unlocked
                self.tableScroll.isHidden = !unlocked
                self.searchField.isEnabled = unlocked
                self.searchField.placeholderString = unlocked ? "Search secrets…" : "Unlock vault to search"
                self.tableView.reloadData()
                self.emptyStateLabel.isHidden = !unlocked || !loaded.isEmpty
                self.emptyStateLabel.stringValue = loaded.isEmpty && unlocked
                    ? (query.isEmpty ? "No secrets yet.\nClick Add to store your first credential." : "No results for “\(query)”.")
                    : ""
                if unlocked, !loaded.isEmpty {
                    self.tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
                }
            }
        }
    }
}

extension SecretsDetailView: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int {
        records.count
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard records.indices.contains(row), let id = tableColumn?.identifier.rawValue else { return nil }
        let record = records[row]
        let cell = NSTextField(labelWithString: "")
        cell.font = .systemFont(ofSize: 12)
        cell.lineBreakMode = .byTruncatingTail
        switch id {
        case "label":
            cell.stringValue = record.label
            cell.font = .systemFont(ofSize: 13, weight: .medium)
        case "account":
            cell.stringValue = record.account
            cell.textColor = .secondaryLabelColor
        case "updated":
            cell.stringValue = RelativeDateTimeFormatter().localizedString(for: record.updatedAt, relativeTo: Date())
            cell.textColor = .secondaryLabelColor
        default:
            break
        }
        return cell
    }
}

@MainActor
private final class SecretEditorSheet: NSWindow {
    private let onSave: (String, String, String?) -> Void
    private let labelField = NSTextField()
    private let accountField = NSTextField()
    private let valueField = NSSecureTextField()
    private let isEditing: Bool

    init(record: SecretRecord?, onSave: @escaping (String, String, String?) -> Void) {
        self.onSave = onSave
        self.isEditing = record != nil
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 440, height: 220),
            styleMask: [.titled, .closable],
            backing: .buffered,
            defer: false
        )
        title = record == nil ? "Add Secret" : "Edit Secret"
        setup(record: record)
    }

    private func setup(record: SecretRecord?) {
        let container = NSView(frame: NSRect(x: 0, y: 0, width: 440, height: 220))

        labelField.stringValue = record?.label ?? ""
        labelField.placeholderString = "Label"
        labelField.translatesAutoresizingMaskIntoConstraints = false

        accountField.stringValue = record?.account ?? ""
        accountField.placeholderString = "Account (optional)"
        accountField.translatesAutoresizingMaskIntoConstraints = false

        valueField.placeholderString = isEditing ? "New value (leave blank to keep)" : "Secret value"
        valueField.translatesAutoresizingMaskIntoConstraints = false

        let saveButton = NSButton(title: "Save", target: self, action: #selector(save))
        saveButton.bezelStyle = .rounded
        saveButton.keyEquivalent = "\r"
        saveButton.translatesAutoresizingMaskIntoConstraints = false

        let cancelButton = NSButton(title: "Cancel", target: self, action: #selector(cancel))
        cancelButton.bezelStyle = .rounded
        cancelButton.translatesAutoresizingMaskIntoConstraints = false

        container.addSubview(labelField)
        container.addSubview(accountField)
        container.addSubview(valueField)
        container.addSubview(saveButton)
        container.addSubview(cancelButton)
        contentView = container

        NSLayoutConstraint.activate([
            labelField.topAnchor.constraint(equalTo: container.topAnchor, constant: 16),
            labelField.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 16),
            labelField.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -16),

            accountField.topAnchor.constraint(equalTo: labelField.bottomAnchor, constant: 8),
            accountField.leadingAnchor.constraint(equalTo: labelField.leadingAnchor),
            accountField.trailingAnchor.constraint(equalTo: labelField.trailingAnchor),

            valueField.topAnchor.constraint(equalTo: accountField.bottomAnchor, constant: 8),
            valueField.leadingAnchor.constraint(equalTo: labelField.leadingAnchor),
            valueField.trailingAnchor.constraint(equalTo: labelField.trailingAnchor),

            cancelButton.topAnchor.constraint(equalTo: valueField.bottomAnchor, constant: 16),
            cancelButton.trailingAnchor.constraint(equalTo: saveButton.leadingAnchor, constant: -8),
            saveButton.topAnchor.constraint(equalTo: valueField.bottomAnchor, constant: 16),
            saveButton.trailingAnchor.constraint(equalTo: labelField.trailingAnchor)
        ])
    }

    @objc private func save() {
        let label = labelField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let account = accountField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let rawValue = valueField.stringValue
        guard !label.isEmpty else { return }
        if isEditing {
            let value = rawValue.isEmpty ? nil : rawValue
            onSave(label, account, value)
        } else {
            guard !rawValue.isEmpty else { return }
            onSave(label, account, rawValue)
        }
        close()
    }

    @objc private func cancel() {
        close()
    }
}
