import AppKit
import LumaCore
import LumaModules

@MainActor
final class SnippetsDetailView: NSObject, ModuleDetailView {
    let moduleTitle = "Snippets"
    let detailView: NSView
    let usesSharedTopBar = true

    private let module: SnippetsModule
    private let detailReloadRouter: ModuleDetailReloadRouter
    private let searchField = NSSearchField()
    private let tableScroll = NSScrollView()
    private let tableView = NSTableView()
    private let emptyStateLabel = NSTextField(labelWithString: "")
    private let copiedFeedbackLabel = NSTextField(labelWithString: "")
    private var snippets: [Snippet] = []
    private var refreshTask: Task<Void, Never>?
    private var copiedFeedbackTask: Task<Void, Never>?

    init(module: SnippetsModule, detailReloadRouter: ModuleDetailReloadRouter) {
        self.module = module
        self.detailReloadRouter = detailReloadRouter
        let chrome = BaseDetailContainer()
        self.detailView = chrome
        super.init()
        setup(chrome: chrome)
    }

    func activate() {
        detailReloadRouter.register(.snippets) { [weak self] in self?.refresh() }
        refresh()
        if let draft = LauncherSharedState.pendingSnippetDraft {
            LauncherSharedState.pendingSnippetDraft = nil
            DispatchQueue.main.async { [weak self] in
                self?.presentEditor(snippet: nil, draft: draft)
            }
        }
        DispatchQueue.main.async { [weak self] in
            self?.tableView.window?.makeFirstResponder(self?.tableView)
        }
    }

    func deactivate() {
        detailReloadRouter.unregister(.snippets)
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
        GeekUIKit.configureDetailTable(tableView, rowHeight: 36)
        tableView.delegate = self
        tableView.dataSource = self
        tableView.target = self
        tableView.doubleAction = #selector(useSelected)
        tableView.translatesAutoresizingMaskIntoConstraints = false

        for (id, title, width) in [
            ("title", "Title", 140.0),
            ("trigger", "Trigger", 88.0),
            ("tags", "Tags", 88.0),
            ("lastUsed", "Last Used", 88.0),
            ("preview", "Preview", 180.0)
        ] {
            let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(id))
            column.title = title
            column.width = width
            tableView.addTableColumn(column)
        }
        GeekUIKit.styleDetailTableColumns(tableView)
        if let previewColumn = tableView.tableColumn(withIdentifier: NSUserInterfaceItemIdentifier("preview")) {
            GeekUIKit.configureDetailTableColumn(previewColumn, minWidth: 120)
        }
        if let lastUsedColumn = tableView.tableColumn(withIdentifier: NSUserInterfaceItemIdentifier("lastUsed")) {
            GeekUIKit.configureDetailTableColumn(lastUsedColumn, minWidth: 72, maxWidth: 110, resizingMask: .userResizingMask)
        }
        tableView.columnAutoresizingStyle = .lastColumnOnlyAutoresizingStyle

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

        copiedFeedbackLabel.font = TypographyTokens.caption(weight: .medium)
        copiedFeedbackLabel.textColor = .secondaryLabelColor
        copiedFeedbackLabel.alignment = .center
        copiedFeedbackLabel.isHidden = true
        copiedFeedbackLabel.translatesAutoresizingMaskIntoConstraints = false

        chrome.setToolbar(toolbar, height: LauncherChromeTokens.detailToolbarHeight)
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
        GeekUIKit.styleDetailSearchField(searchField)
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
        GeekUIKit.makeToolbarButton(title, target: self, action: action)
    }

    @objc private func searchChanged() {
        refresh()
    }

    @objc private func addSnippet() {
        presentEditor(snippet: nil, draft: nil)
    }

    @objc private func useSelected() {
        let row = tableView.selectedRow
        guard snippets.indices.contains(row) else { return }
        let snippet = snippets[row]
        onHideLauncherIfNeeded()
        Task { [weak self] in
            guard let self else { return }
            try? await self.module.insertSnippet(id: snippet.id)
            await MainActor.run { self.refresh() }
        }
    }

    private func onHideLauncherIfNeeded() {
        LauncherEnvironment.current?.onHideLauncher()
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
        presentEditor(snippet: snippets[row], draft: nil)
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

    private func presentEditor(snippet: Snippet?, draft: SnippetDraft?) {
        let sheet = SnippetEditorSheet(snippet: snippet, draft: draft) { [weak self] title, trigger, content, tags in
            guard let self else { return }
            Task {
                if var existing = snippet {
                    existing.title = title
                    existing.trigger = trigger
                    existing.content = content
                    existing.tags = tags
                    let saved = try? await self.module.update(existing)
                    await MainActor.run {
                        if saved == nil {
                            LauncherEnvironment.current?.showStatus(LauncherStatusMessages.snippetSaveFailed)
                        }
                        self.refresh()
                    }
                } else {
                    let saved = try? await self.module.add(title: title, content: content, tags: tags, trigger: trigger)
                    await MainActor.run {
                        LauncherEnvironment.current?.showStatus(
                            saved == nil ? LauncherStatusMessages.snippetSaveFailed : LauncherStatusMessages.snippetCreated
                        )
                        self.refresh()
                    }
                }
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
        let toolTip = SnippetDisplay.rowToolTip(snippet)
        switch id {
        case "title":
            let title = SnippetDisplay.disambiguatedTitle(snippet, among: snippets)
            return GeekUIKit.makeDetailTableCell(
                text: title,
                font: .systemFont(ofSize: 13, weight: .medium),
                toolTip: toolTip
            )
        case "trigger":
            return GeekUIKit.makeDetailTableCell(
                text: snippet.displayTrigger,
                font: .monospacedSystemFont(ofSize: 12, weight: .regular),
                color: .secondaryLabelColor,
                toolTip: toolTip
            )
        case "preview":
            return GeekUIKit.makeDetailTableCell(
                text: SnippetDisplay.contentPreview(snippet),
                color: .secondaryLabelColor,
                lineBreak: .byTruncatingMiddle,
                toolTip: toolTip
            )
        case "tags":
            return GeekUIKit.makeDetailTableCell(
                text: snippet.tags.joined(separator: ", "),
                color: .secondaryLabelColor,
                toolTip: toolTip
            )
        case "lastUsed":
            let value: String
            if snippet.usageCount == 0 {
                value = "—"
            } else {
                value = RelativeDateTimeFormatter().localizedString(for: snippet.lastUsedAt, relativeTo: Date())
            }
            return GeekUIKit.makeDetailTableCell(text: value, color: .secondaryLabelColor, toolTip: toolTip)
        default:
            return nil
        }
    }
}
