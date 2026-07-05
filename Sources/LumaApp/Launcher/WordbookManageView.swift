import AppKit
import LumaCore
import LumaModules

@MainActor
final class WordbookManageView: NSView {
    enum ScopeFilter: Int { case all = 0, wrongWords = 1 }

    private let store: WordbookStore
    private let onBack: () -> Void
    private let searchField = NSSearchField()
    private let categoryFilter = NSPopUpButton()
    private let scopeFilter = NSPopUpButton()
    private let tableScroll = NSScrollView()
    private let tableView = NSTableView()
    private var allWords: [WordEntry] = []
    private var filtered: [WordEntry] = []
    private var loadOffset = 0
    private let pageSize = 200
    private var loadTask: Task<Void, Never>?
    private var isLoadingMore = false

    init(store: WordbookStore, wrongWordsOnly: Bool = false, onBack: @escaping () -> Void) {
        self.store = store
        self.onBack = onBack
        super.init(frame: .zero)
        setup()
        if wrongWordsOnly {
            scopeFilter.selectItem(at: ScopeFilter.wrongWords.rawValue)
        }
        loadMore()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    private func setup() {
        translatesAutoresizingMaskIntoConstraints = false
        let toolbar = NSView()
        toolbar.translatesAutoresizingMaskIntoConstraints = false

        let backButton = NSButton(title: "← Back", target: self, action: #selector(backTapped))
        backButton.bezelStyle = .rounded
        backButton.translatesAutoresizingMaskIntoConstraints = false

        let title = NSTextField(labelWithString: "Manage Words")
        title.font = .systemFont(ofSize: 14, weight: .semibold)
        title.translatesAutoresizingMaskIntoConstraints = false

        let addButton = NSButton(title: "+ Add", target: self, action: #selector(addWord))
        addButton.bezelStyle = .rounded
        addButton.translatesAutoresizingMaskIntoConstraints = false

        let importButton = NSButton(title: "Import CSV", target: self, action: #selector(importCSV))
        importButton.bezelStyle = .rounded
        importButton.translatesAutoresizingMaskIntoConstraints = false

        let exportButton = NSButton(title: "Export CSV", target: self, action: #selector(exportCSV))
        exportButton.bezelStyle = .rounded
        exportButton.translatesAutoresizingMaskIntoConstraints = false

        searchField.placeholderString = "Search words…"
        searchField.target = self
        searchField.action = #selector(filterChanged)
        searchField.translatesAutoresizingMaskIntoConstraints = false
        NotificationCenter.default.addObserver(self, selector: #selector(filterChanged), name: NSSearchField.textDidChangeNotification, object: searchField)

        categoryFilter.addItem(withTitle: "All categories")
        categoryFilter.target = self
        categoryFilter.action = #selector(filterChanged)
        categoryFilter.translatesAutoresizingMaskIntoConstraints = false

        scopeFilter.addItem(withTitle: "All words")
        scopeFilter.addItem(withTitle: "Wrong (≥3)")
        scopeFilter.target = self
        scopeFilter.action = #selector(filterChanged)
        scopeFilter.translatesAutoresizingMaskIntoConstraints = false

        toolbar.addSubview(backButton)
        toolbar.addSubview(title)
        toolbar.addSubview(searchField)
        toolbar.addSubview(scopeFilter)
        toolbar.addSubview(categoryFilter)
        toolbar.addSubview(addButton)
        toolbar.addSubview(importButton)
        toolbar.addSubview(exportButton)

        for (id, title, width) in [
            ("term", "Term", 120.0),
            ("phonetic", "Phonetic", 90.0),
            ("meaning", "Meaning", 160.0),
            ("stage", "Stage", 72.0),
            ("wrong", "Wrong", 50.0),
            ("category", "Category", 90.0)
        ] {
            let col = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(id))
            col.title = title
            col.width = width
            tableView.addTableColumn(col)
        }
        tableView.headerView = NSTableHeaderView()
        tableView.delegate = self
        tableView.dataSource = self
        tableView.target = self
        tableView.doubleAction = #selector(editSelected)
        tableView.menu = contextMenu()
        tableScroll.documentView = tableView
        GeekUIKit.wireVerticalListScroll(
            tableScroll,
            documentView: tableView,
            observer: self,
            onClipViewResize: #selector(syncListScrollDocumentFrame)
        )
        tableScroll.translatesAutoresizingMaskIntoConstraints = false

        addSubview(toolbar)
        addSubview(tableScroll)

        NSLayoutConstraint.activate([
            toolbar.topAnchor.constraint(equalTo: topAnchor, constant: 8),
            toolbar.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            toolbar.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            toolbar.heightAnchor.constraint(equalToConstant: 32),
            backButton.leadingAnchor.constraint(equalTo: toolbar.leadingAnchor),
            backButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            title.leadingAnchor.constraint(equalTo: backButton.trailingAnchor, constant: 8),
            title.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            searchField.leadingAnchor.constraint(equalTo: title.trailingAnchor, constant: 16),
            searchField.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            searchField.widthAnchor.constraint(equalToConstant: 160),
            scopeFilter.leadingAnchor.constraint(equalTo: searchField.trailingAnchor, constant: 8),
            scopeFilter.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            categoryFilter.leadingAnchor.constraint(equalTo: scopeFilter.trailingAnchor, constant: 8),
            categoryFilter.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            importButton.trailingAnchor.constraint(equalTo: toolbar.trailingAnchor),
            importButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            exportButton.trailingAnchor.constraint(equalTo: importButton.leadingAnchor, constant: -8),
            exportButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            addButton.trailingAnchor.constraint(equalTo: exportButton.leadingAnchor, constant: -8),
            addButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            tableScroll.topAnchor.constraint(equalTo: toolbar.bottomAnchor, constant: 8),
            tableScroll.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 8),
            tableScroll.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -8),
            tableScroll.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -8)
        ])

        NotificationCenter.default.addObserver(
            self,
            selector: #selector(scrollDidLive),
            name: NSScrollView.didLiveScrollNotification,
            object: tableScroll
        )
        tableScroll.contentView.postsBoundsChangedNotifications = true
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(scrollDidLive),
            name: NSView.boundsDidChangeNotification,
            object: tableScroll.contentView
        )
    }

    @objc private func syncListScrollDocumentFrame() {
        GeekUIKit.syncVerticalListDocumentFrame(in: tableScroll)
    }

    @objc private func scrollDidLive() {
        let visibleRect = tableScroll.documentVisibleRect
        let contentHeight = tableScroll.documentView?.frame.height ?? 0
        guard contentHeight > 0, visibleRect.maxY >= contentHeight - 80 else { return }
        loadMore()
    }

    private func contextMenu() -> NSMenu {
        let menu = NSMenu()
        menu.addItem(withTitle: "Edit", action: #selector(editSelected), keyEquivalent: "")
        menu.addItem(withTitle: "Delete", action: #selector(deleteSelected), keyEquivalent: "")
        menu.addItem(.separator())
        menu.addItem(withTitle: "Reset Stage", action: #selector(resetStage), keyEquivalent: "")
        menu.items.forEach { $0.target = self }
        return menu
    }

    private func loadMore() {
        guard !isLoadingMore else { return }
        isLoadingMore = true
        loadTask?.cancel()
        loadTask = Task { [weak self] in
            defer { Task { @MainActor in self?.isLoadingMore = false } }
            guard let self else { return }
            do {
                let batch = try await store.allWords(limit: pageSize, offset: loadOffset)
                await MainActor.run {
                    self.allWords.append(contentsOf: batch)
                    self.loadOffset += batch.count
                    self.rebuildCategoryFilter()
                    self.applyFilter()
                }
            } catch {}
        }
    }

    private func rebuildCategoryFilter() {
        let cats = Set(allWords.map(\.category).filter { !$0.isEmpty }).sorted()
        let selected = categoryFilter.titleOfSelectedItem
        while categoryFilter.numberOfItems > 1 { categoryFilter.removeItem(at: 1) }
        for cat in cats { categoryFilter.addItem(withTitle: cat) }
        if let selected, categoryFilter.itemTitles.contains(selected) {
            categoryFilter.selectItem(withTitle: selected)
        }
    }

    @objc private func filterChanged() {
        applyFilter()
    }

    private func applyFilter() {
        let q = searchField.stringValue.lowercased()
        let cat = categoryFilter.indexOfSelectedItem > 0 ? categoryFilter.titleOfSelectedItem : nil
        filtered = allWords.filter { word in
            let matchesCat = cat == nil || word.category == cat
            let matchesScope = scopeFilter.indexOfSelectedItem != ScopeFilter.wrongWords.rawValue || word.wrongCount >= 3
            let matchesQ = q.isEmpty
                || word.term.lowercased().contains(q)
                || word.meaning.lowercased().contains(q)
            return matchesCat && matchesScope && matchesQ
        }
        tableView.reloadData()
        GeekUIKit.syncVerticalListDocumentFrame(in: tableScroll)
    }

    @objc private func backTapped() { onBack() }

    @objc private func addWord() {
        presentEditor(entry: nil)
    }

    func triggerAdd() {
        presentEditor(entry: nil)
    }

    func triggerImport() {
        importCSV()
    }

    @objc private func editSelected() {
        let row = tableView.selectedRow
        guard filtered.indices.contains(row) else { return }
        presentEditor(entry: filtered[row])
    }

    @objc private func deleteSelected() {
        let row = tableView.selectedRow
        guard filtered.indices.contains(row) else { return }
        let entry = filtered[row]
        let alert = NSAlert()
        alert.messageText = L10n.tr("wordbook.manage.delete.title", entry.term)
        alert.informativeText = L10n.tr("detail.delete.cannotUndo")
        alert.addButton(withTitle: L10n.tr("detail.delete.confirm"))
        alert.addButton(withTitle: L10n.tr("detail.delete.cancel"))
        alert.alertStyle = .warning
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        Task {
            do {
                try await store.deleteWord(id: entry.id)
                await MainActor.run {
                    allWords.removeAll { $0.id == entry.id }
                    applyFilter()
                }
            } catch {
                await MainActor.run {
                    LauncherEnvironment.current?.showStatus(LauncherStatusMessages.deleteFailed)
                }
            }
        }
    }

    @objc private func resetStage() {
        let row = tableView.selectedRow
        guard filtered.indices.contains(row) else { return }
        let entry = filtered[row]
        let alert = NSAlert()
        alert.messageText = "Reset review progress for “\(entry.term)”?"
        alert.informativeText = "Stage, wrong count, and next review date will be cleared."
        alert.addButton(withTitle: "Reset")
        alert.addButton(withTitle: "Cancel")
        alert.alertStyle = .warning
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        let id = entry.id
        Task {
            try? await store.resetWordStage(id: id)
            await MainActor.run { loadOffset = 0; allWords = []; loadMore() }
        }
    }

    @objc private func importCSV() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.commaSeparatedText, .tabSeparatedText, .plainText]
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        Task {
            guard let text = try? String(contentsOf: url, encoding: .utf8) else { return }
            let entries = WordbookCSVImporter.parse(text)
            let result = try? await store.upsertWords(entries)
            await MainActor.run {
                if let result {
                    let alert = NSAlert()
                    alert.messageText = "Imported \(result.imported), skipped \(result.skipped) duplicates"
                    alert.runModal()
                }
                loadOffset = 0
                allWords = []
                loadMore()
            }
        }
    }

    @objc private func exportCSV() {
        loadTask?.cancel()
        loadTask = Task { [weak self] in
            guard let self else { return }
            var all: [WordEntry] = []
            var offset = 0
            while true {
                let batch = (try? await store.allWords(limit: pageSize, offset: offset)) ?? []
                if batch.isEmpty { break }
                all.append(contentsOf: batch)
                offset += batch.count
                if batch.count < pageSize { break }
            }
            let csv = WordbookCSVImporter.export(all)
            await MainActor.run {
                let panel = NSSavePanel()
                panel.allowedContentTypes = [.commaSeparatedText]
                panel.nameFieldStringValue = "wordbook-export.csv"
                guard panel.runModal() == .OK, let url = panel.url else { return }
                try? csv.write(to: url, atomically: true, encoding: .utf8)
            }
        }
    }

    private func presentEditor(entry: WordEntry?) {
        let sheet = WordbookWordEditorSheet(
            entry: entry,
            onSave: { [weak self] updated, markMastered in
            guard let self else { return }
            Task {
                if entry == nil {
                    _ = try? await store.upsertWords([updated])
                    if let inserted = try? await store.search(updated.term, limit: 1).first, markMastered {
                        _ = try? await store.recordReview(wordID: inserted.id, familiarity: .mastered)
                    }
                } else {
                    try? await store.updateWord(updated)
                    if markMastered, updated.familiarity != "mastered" {
                        _ = try? await store.recordReview(wordID: updated.id, familiarity: .mastered)
                    }
                }
                await MainActor.run { loadOffset = 0; allWords = []; loadMore() }
            }
        },
            onResetStage: { [weak self] in
                guard let self, let entry else { return }
                Task {
                    try? await store.resetWordStage(id: entry.id)
                    await MainActor.run { loadOffset = 0; allWords = []; loadMore() }
                }
            }
        )
        if let window = window {
            window.beginSheet(sheet) { _ in }
        }
    }
}

extension WordbookManageView: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int { filtered.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard filtered.indices.contains(row), let id = tableColumn?.identifier.rawValue else { return nil }
        let word = filtered[row]
        let cell = NSTextField(labelWithString: "")
        cell.font = .systemFont(ofSize: 12)
        cell.lineBreakMode = .byTruncatingTail
        switch id {
        case "term": cell.stringValue = word.term
        case "phonetic": cell.stringValue = word.phonetic
        case "meaning": cell.stringValue = word.meaning
        case "stage":
            cell.stringValue = stageLabel(for: word)
            cell.toolTip = "复习阶段 1–9；new = 未学，mastered = 已掌握"
        case "wrong": cell.stringValue = "\(word.wrongCount)"
        case "category": cell.stringValue = word.category
        default: break
        }
        return cell
    }

    func tableView(_ tableView: NSTableView, heightOfRow row: Int) -> CGFloat { 28 }

    func tableView(_ tableView: NSTableView, shouldSelectRow row: Int) -> Bool { true }

    private func stageLabel(for word: WordEntry) -> String {
        if word.familiarity == "mastered" || word.reviewStage >= ReviewScheduler.intervals.count {
            return "9 (mastered)"
        }
        if word.reviewCount == 0 {
            return "0 (new)"
        }
        return "\(word.reviewStage + 1)/9"
    }
}
