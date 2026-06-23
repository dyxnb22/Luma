import AppKit
import LumaModules

@MainActor
final class SnippetsDetailView: NSObject, ModuleDetailView {
    let moduleTitle = "Snippets"
    let detailView: NSView
    let usesSharedTopBar = true

    private let module: SnippetsModule
    private let searchField = NSSearchField()
    private let tableScroll = NSScrollView()
    private let tableView = NSTableView()
    private let emptyStateLabel = NSTextField(labelWithString: "")
    private let copiedFeedbackLabel = NSTextField(labelWithString: "")
    private var snippets: [Snippet] = []
    private var refreshTask: Task<Void, Never>?
    private var copiedFeedbackTask: Task<Void, Never>?

    init(module: SnippetsModule) {
        self.module = module
        let chrome = BaseDetailContainer()
        self.detailView = chrome
        super.init()
        setup(chrome: chrome)
        ModuleDetailReloads.reloadSnippetsDetail = { [weak self] in self?.refresh() }
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
        copiedFeedbackTask?.cancel()
        copiedFeedbackTask = nil
    }

    func handleKeyDown(_ event: NSEvent) -> Bool {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if flags.contains(.command), event.charactersIgnoringModifiers?.lowercased() == "f" {
            detailView.window?.makeFirstResponder(searchField)
            return true
        }
        if flags.contains(.command), event.charactersIgnoringModifiers?.lowercased() == "e" {
            editSelected()
            return true
        }
        return false
    }

    private func setup(chrome: BaseDetailContainer) {
        let toolbar = buildToolbar()

        tableView.headerView = NSTableHeaderView()
        tableView.style = .plain
        tableView.rowHeight = 36
        tableView.delegate = self
        tableView.dataSource = self
        tableView.target = self
        tableView.doubleAction = #selector(useSelected)
        tableView.translatesAutoresizingMaskIntoConstraints = false

        for (id, title, width) in [
            ("title", "Title", 160.0),
            ("trigger", "Trigger", 100.0),
            ("tags", "Tags", 100.0),
            ("lastUsed", "Last Used", 100.0)
        ] {
            let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(id))
            column.title = title
            column.width = width
            tableView.addTableColumn(column)
        }

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

        copiedFeedbackLabel.font = .systemFont(ofSize: 12, weight: .medium)
        copiedFeedbackLabel.textColor = .secondaryLabelColor
        copiedFeedbackLabel.alignment = .center
        copiedFeedbackLabel.isHidden = true
        copiedFeedbackLabel.translatesAutoresizingMaskIntoConstraints = false

        chrome.setToolbar(toolbar, height: 32)
        chrome.setContent(tableScroll, embedInScroll: false)
        chrome.addSubview(emptyStateLabel)
        chrome.addSubview(copiedFeedbackLabel)

        NSLayoutConstraint.activate([
            emptyStateLabel.centerXAnchor.constraint(equalTo: tableScroll.centerXAnchor),
            emptyStateLabel.centerYAnchor.constraint(equalTo: tableScroll.centerYAnchor),
            copiedFeedbackLabel.centerXAnchor.constraint(equalTo: tableScroll.centerXAnchor),
            copiedFeedbackLabel.topAnchor.constraint(equalTo: tableScroll.topAnchor, constant: 8)
        ])

        searchField.target = self
        searchField.action = #selector(searchChanged)
        NotificationCenter.default.addObserver(self, selector: #selector(searchChanged), name: NSSearchField.textDidChangeNotification, object: searchField)
    }

    private func buildToolbar() -> NSView {
        let toolbar = NSView()
        toolbar.translatesAutoresizingMaskIntoConstraints = false

        searchField.placeholderString = "Search snippets…"
        searchField.translatesAutoresizingMaskIntoConstraints = false

        let addButton = makeToolbarButton("Add", action: #selector(addSnippet))
        let editButton = makeToolbarButton("Edit", action: #selector(editSelected))
        let deleteButton = makeToolbarButton("Delete", action: #selector(deleteSelected))
        let duplicateButton = makeToolbarButton("Duplicate", action: #selector(duplicateSelected))

        let buttonStack = NSStackView(views: [addButton, editButton, deleteButton, duplicateButton])
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

    private func makeToolbarButton(_ title: String, action: Selector) -> NSButton {
        let button = NSButton(title: title, target: self, action: action)
        button.bezelStyle = .rounded
        return button
    }

    @objc private func searchChanged() {
        refresh()
    }

    @objc private func addSnippet() {
        presentEditor(snippet: nil)
    }

    @objc private func useSelected() {
        let row = tableView.selectedRow
        guard snippets.indices.contains(row) else { return }
        let snippet = snippets[row]
        let expanded = SnippetVariableExpander.expand(snippet.content)
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(expanded, forType: .string)
        showCopiedFeedback()
        Task { [weak self] in
            guard let self else { return }
            _ = try? await self.module.markUsed(id: snippet.id)
            await MainActor.run { self.refresh() }
        }
    }

    private func showCopiedFeedback() {
        copiedFeedbackTask?.cancel()
        copiedFeedbackLabel.stringValue = "Copied to clipboard"
        copiedFeedbackLabel.isHidden = false
        copiedFeedbackTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(1.5))
            guard !Task.isCancelled else { return }
            await MainActor.run {
                self?.copiedFeedbackLabel.isHidden = true
            }
        }
    }

    @objc private func editSelected() {
        let row = tableView.selectedRow
        guard snippets.indices.contains(row) else { return }
        presentEditor(snippet: snippets[row])
    }

    @objc private func deleteSelected() {
        let row = tableView.selectedRow
        guard snippets.indices.contains(row) else { return }
        let snippet = snippets[row]
        let alert = NSAlert()
        alert.messageText = "Delete “\(snippet.title)”?"
        alert.informativeText = "This cannot be undone."
        alert.addButton(withTitle: "Delete")
        alert.addButton(withTitle: "Cancel")
        alert.alertStyle = .warning
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        Task { [weak self] in
            guard let self else { return }
            try? await self.module.delete(id: snippet.id)
            await MainActor.run { self.refresh() }
        }
    }

    @objc private func duplicateSelected() {
        let row = tableView.selectedRow
        guard snippets.indices.contains(row) else { return }
        let id = snippets[row].id
        Task { [weak self] in
            guard let self else { return }
            _ = try? await self.module.duplicate(id: id)
            await MainActor.run { self.refresh() }
        }
    }

    private func presentEditor(snippet: Snippet?) {
        let sheet = SnippetEditorSheet(snippet: snippet) { [weak self] title, trigger, content, tags in
            guard let self else { return }
            Task {
                if var existing = snippet {
                    existing.title = title
                    existing.trigger = trigger
                    existing.content = content
                    existing.tags = tags
                    _ = try? await self.module.update(existing)
                } else {
                    _ = try? await self.module.add(title: title, content: content, tags: tags, trigger: trigger)
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
            let all = await self.module.allSnippets()
            let filtered: [Snippet]
            if query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                filtered = all.sorted { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
            } else {
                filtered = SnippetIndex.search(all, query: query, limit: 500).map(\.snippet)
            }
            await MainActor.run {
                self.snippets = filtered
                self.tableView.reloadData()
                self.emptyStateLabel.isHidden = !filtered.isEmpty
                self.emptyStateLabel.stringValue = filtered.isEmpty
                    ? (query.isEmpty ? "No snippets yet.\nClick Add to create your first one." : "No results for “\(query)”.")
                    : ""
                if !filtered.isEmpty {
                    self.tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
                }
            }
        }
    }
}

extension SnippetsDetailView: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int {
        snippets.count
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard snippets.indices.contains(row), let id = tableColumn?.identifier.rawValue else { return nil }
        let snippet = snippets[row]
        let cell = NSTextField(labelWithString: "")
        cell.font = .systemFont(ofSize: 12)
        cell.lineBreakMode = .byTruncatingTail
        switch id {
        case "title":
            cell.stringValue = snippet.title
            cell.font = .systemFont(ofSize: 13, weight: .medium)
        case "trigger":
            cell.stringValue = snippet.displayTrigger
            cell.textColor = .secondaryLabelColor
            cell.font = .monospacedSystemFont(ofSize: 12, weight: .regular)
        case "tags":
            cell.stringValue = snippet.tags.joined(separator: ", ")
            cell.textColor = .secondaryLabelColor
        case "lastUsed":
            if snippet.usageCount == 0 {
                cell.stringValue = "—"
            } else {
                cell.stringValue = RelativeDateTimeFormatter().localizedString(for: snippet.lastUsedAt, relativeTo: Date())
            }
            cell.textColor = .secondaryLabelColor
        default:
            break
        }
        return cell
    }
}

