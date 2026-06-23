import AppKit
import LumaModules

@MainActor
final class MediaDetailView: NSObject, ModuleDetailView {
    let moduleTitle = "Media"
    let detailView: NSView
    let usesSharedTopBar = true

    private let module: MediaModule
    private let categoryControl = NSSegmentedControl()
    private let statusPopup = NSPopUpButton()
    private let sortPopup = NSPopUpButton()
    private let tableScroll = NSScrollView()
    private let tableView = NSTableView()
    private let footerLabel = NSTextField(labelWithString: "")
    private var items: [MediaItem] = []
    private var refreshTask: Task<Void, Never>?
    private var selectedCategory: MediaCategory?
    private var selectedStatus: MediaStatus?

    init(module: MediaModule) {
        self.module = module
        let chrome = BaseDetailContainer()
        self.detailView = chrome
        super.init()
        setup(chrome: chrome)
        ModuleDetailReloads.reloadMediaDetail = { [weak self] in self?.refresh() }
    }

    func activate() {
        refresh()
        if let draft = LauncherSharedState.pendingMediaEditorDraft {
            LauncherSharedState.pendingMediaEditorDraft = nil
            presentEditor(draft: draft)
        }
    }

    func deactivate() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    func presentEditor(draft: MediaEditorDraft) {
        let sheet = MediaItemEditorSheet(draft: draft) { [weak self] saved in
            guard let self else { return }
            Task {
                if saved.existingID != nil {
                    _ = try? await self.module.update(from: saved)
                } else {
                    _ = try? await self.module.add(from: saved)
                }
                await MainActor.run { self.refresh() }
            }
        }
        if let window = detailView.window {
            window.beginSheet(sheet) { _ in }
        }
    }

    func handleKeyDown(_ event: NSEvent) -> Bool {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if flags.contains(.command), event.charactersIgnoringModifiers?.lowercased() == "f" {
            return false
        }
        if event.keyCode == 51, flags.contains(.command) {
            deleteSelected()
            return true
        }
        return false
    }

    private func setup(chrome: BaseDetailContainer) {
        let toolbar = buildToolbar()
        let header = NSView()
        header.translatesAutoresizingMaskIntoConstraints = false

        categoryControl.segmentCount = 6
        let segments = ["All"] + MediaCategory.allCases.map(\.displayName)
        for (index, label) in segments.enumerated() {
            categoryControl.setLabel(label, forSegment: index)
        }
        categoryControl.selectedSegment = 0
        categoryControl.target = self
        categoryControl.action = #selector(categoryChanged)
        categoryControl.translatesAutoresizingMaskIntoConstraints = false

        statusPopup.addItem(withTitle: "All Statuses")
        for status in MediaStatus.allCases {
            statusPopup.addItem(withTitle: status.displayName)
        }
        statusPopup.target = self
        statusPopup.action = #selector(filterChanged)
        statusPopup.translatesAutoresizingMaskIntoConstraints = false

        sortPopup.addItem(withTitle: "Recently Completed")
        sortPopup.addItem(withTitle: "Recently Added")
        sortPopup.addItem(withTitle: "Rating Desc")
        sortPopup.addItem(withTitle: "Title")
        sortPopup.target = self
        sortPopup.action = #selector(filterChanged)
        sortPopup.translatesAutoresizingMaskIntoConstraints = false

        tableView.headerView = NSTableHeaderView()
        tableView.style = .plain
        tableView.rowHeight = 34
        tableView.delegate = self
        tableView.dataSource = self
        tableView.target = self
        tableView.doubleAction = #selector(editSelected)
        tableView.translatesAutoresizingMaskIntoConstraints = false

        for (id, title, width) in [
            ("title", "Title", 200.0),
            ("category", "Cat", 70.0),
            ("status", "Status", 90.0),
            ("rating", "Rating", 60.0),
            ("completed", "Completed", 100.0)
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

        footerLabel.font = .systemFont(ofSize: 11)
        footerLabel.textColor = .secondaryLabelColor
        footerLabel.translatesAutoresizingMaskIntoConstraints = false

        header.addSubview(toolbar)
        header.addSubview(categoryControl)
        header.addSubview(statusPopup)
        header.addSubview(sortPopup)

        chrome.setToolbar(header, height: 80)
        chrome.setFooter(footerLabel, height: 20)
        chrome.setContent(tableScroll, embedInScroll: false)

        NSLayoutConstraint.activate([
            toolbar.topAnchor.constraint(equalTo: header.topAnchor),
            toolbar.leadingAnchor.constraint(equalTo: header.leadingAnchor),
            toolbar.trailingAnchor.constraint(equalTo: header.trailingAnchor),
            toolbar.heightAnchor.constraint(equalToConstant: 32),

            categoryControl.topAnchor.constraint(equalTo: toolbar.bottomAnchor, constant: 8),
            categoryControl.leadingAnchor.constraint(equalTo: header.leadingAnchor),

            statusPopup.centerYAnchor.constraint(equalTo: categoryControl.centerYAnchor),
            statusPopup.leadingAnchor.constraint(equalTo: categoryControl.trailingAnchor, constant: 12),

            sortPopup.centerYAnchor.constraint(equalTo: categoryControl.centerYAnchor),
            sortPopup.leadingAnchor.constraint(equalTo: statusPopup.trailingAnchor, constant: 12),
            sortPopup.bottomAnchor.constraint(equalTo: header.bottomAnchor)
        ])
    }

    private func buildToolbar() -> NSView {
        let toolbar = NSView()
        toolbar.translatesAutoresizingMaskIntoConstraints = false
        let addButton = NSButton(title: "Add", target: self, action: #selector(addItem))
        addButton.bezelStyle = .rounded
        let exportButton = NSButton(title: "Export", target: self, action: #selector(exportCSV))
        exportButton.bezelStyle = .rounded
        addButton.translatesAutoresizingMaskIntoConstraints = false
        exportButton.translatesAutoresizingMaskIntoConstraints = false
        toolbar.addSubview(addButton)
        toolbar.addSubview(exportButton)
        NSLayoutConstraint.activate([
            addButton.leadingAnchor.constraint(equalTo: toolbar.leadingAnchor),
            addButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            exportButton.leadingAnchor.constraint(equalTo: addButton.trailingAnchor, constant: 8),
            exportButton.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor)
        ])
        return toolbar
    }

    @objc private func categoryChanged() {
        let index = categoryControl.selectedSegment
        selectedCategory = index == 0 ? nil : MediaCategory.allCases[index - 1]
        refresh()
    }

    @objc private func filterChanged() {
        let statusIndex = statusPopup.indexOfSelectedItem
        selectedStatus = statusIndex == 0 ? nil : MediaStatus.allCases[statusIndex - 1]
        refresh()
    }

    @objc private func addItem() {
        presentEditor(draft: MediaEditorDraft(title: ""))
    }

    @objc private func exportCSV() {
        Task { [weak self] in
            guard let self else { return }
            if let url = try? await self.module.exportCSV() {
                await MainActor.run {
                    NSWorkspace.shared.activateFileViewerSelecting([url])
                }
            }
        }
    }

    @objc private func editSelected() {
        let row = tableView.selectedRow
        guard items.indices.contains(row) else { return }
        presentEditor(draft: MediaEditorDraft(item: items[row]))
    }

    private func deleteSelected() {
        let row = tableView.selectedRow
        guard items.indices.contains(row) else { return }
        let item = items[row]
        Task { [weak self] in
            guard let self else { return }
            try? await self.module.delete(id: item.id)
            await MainActor.run { self.refresh() }
        }
    }

    private func refresh() {
        refreshTask?.cancel()
        let category = selectedCategory
        let status = selectedStatus
        let sort: MediaSort
        switch sortPopup.indexOfSelectedItem {
        case 1: sort = .recentlyAdded
        case 2: sort = .ratingDesc
        case 3: sort = .title
        default: sort = .recentlyCompleted
        }
        refreshTask = Task { [weak self] in
            guard let self else { return }
            let all = await self.module.allItems()
            let filtered = MediaIndex.filter(all, category: category, status: status, sort: sort)
            let stats = MediaIndex.stats(for: filtered)
            await MainActor.run {
                self.items = filtered
                self.tableView.reloadData()
                let avgText: String
                if let avg = stats.averageRating {
                    avgText = String(format: "%.1f", avg)
                } else {
                    avgText = "—"
                }
                self.footerLabel.stringValue = "\(stats.count) items · avg rating \(avgText) · \(stats.doneThisYear) done this year"
                if !filtered.isEmpty {
                    self.tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
                }
            }
        }
    }
}

extension MediaDetailView: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int { items.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard items.indices.contains(row), let id = tableColumn?.identifier.rawValue else { return nil }
        let item = items[row]
        let cell = NSTextField(labelWithString: "")
        cell.font = .systemFont(ofSize: 12)
        switch id {
        case "title":
            cell.stringValue = item.title
            cell.font = .systemFont(ofSize: 13, weight: .medium)
        case "category":
            cell.stringValue = item.category.displayName
            cell.textColor = .secondaryLabelColor
        case "status":
            cell.stringValue = item.status.verb(for: item.category)
            cell.textColor = .secondaryLabelColor
        case "rating":
            cell.stringValue = item.rating.map { "★\($0)" } ?? "—"
            cell.textColor = .secondaryLabelColor
        case "completed":
            if let date = item.completedAt {
                cell.stringValue = Self.dateFormatter.string(from: date)
            } else {
                cell.stringValue = "—"
            }
            cell.textColor = .secondaryLabelColor
        default:
            break
        }
        return cell
    }

    private static let dateFormatter: DateFormatter = {
        let f = DateFormatter()
        f.dateStyle = .medium
        f.timeStyle = .none
        return f
    }()
}

@MainActor
private final class MediaItemEditorSheet: NSWindow {
    private let onSave: (MediaEditorDraft) -> Void
    private let titleField = NSTextField()
    private let categoryPopup = NSPopUpButton()
    private let statusPopup = NSPopUpButton()
    private let ratingSlider = NSSlider()
    private let ratingLabel = NSTextField(labelWithString: "No rating")
    private let noRatingToggle = NSButton()
    private let notesView = NSTextView()
    private var draft: MediaEditorDraft

    init(draft: MediaEditorDraft, onSave: @escaping (MediaEditorDraft) -> Void) {
        self.draft = draft
        self.onSave = onSave
        super.init(contentRect: NSRect(x: 0, y: 0, width: 480, height: 380), styleMask: [.titled, .closable], backing: .buffered, defer: false)
        title = draft.existingID == nil ? "Add Media" : "Edit Media"
        setup()
    }

    private func setup() {
        let container = NSView(frame: NSRect(x: 0, y: 0, width: 480, height: 380))
        titleField.stringValue = draft.title
        titleField.placeholderString = "Title"
        titleField.translatesAutoresizingMaskIntoConstraints = false

        for category in MediaCategory.allCases {
            categoryPopup.addItem(withTitle: category.displayName)
        }
        if let category = draft.category, let index = MediaCategory.allCases.firstIndex(of: category) {
            categoryPopup.selectItem(at: index)
        }
        categoryPopup.translatesAutoresizingMaskIntoConstraints = false

        for status in MediaStatus.allCases {
            statusPopup.addItem(withTitle: status.displayName)
        }
        if let index = MediaStatus.allCases.firstIndex(of: draft.status) {
            statusPopup.selectItem(at: index)
        }
        statusPopup.translatesAutoresizingMaskIntoConstraints = false

        ratingSlider.minValue = 1
        ratingSlider.maxValue = 10
        ratingSlider.integerValue = draft.rating ?? 8
        ratingSlider.target = self
        ratingSlider.action = #selector(ratingChanged)
        ratingSlider.translatesAutoresizingMaskIntoConstraints = false

        noRatingToggle.setButtonType(.switch)
        noRatingToggle.title = "No rating"
        noRatingToggle.state = draft.rating == nil ? .on : .off
        noRatingToggle.target = self
        noRatingToggle.action = #selector(ratingChanged)
        noRatingToggle.translatesAutoresizingMaskIntoConstraints = false
        ratingChanged()

        notesView.string = draft.notes
        notesView.isRichText = false
        notesView.translatesAutoresizingMaskIntoConstraints = false
        let scroll = NSScrollView()
        scroll.documentView = notesView
        scroll.hasVerticalScroller = true
        scroll.borderType = .bezelBorder
        scroll.translatesAutoresizingMaskIntoConstraints = false

        let saveButton = NSButton(title: "Save", target: self, action: #selector(save))
        saveButton.bezelStyle = .rounded
        saveButton.keyEquivalent = "\r"
        saveButton.translatesAutoresizingMaskIntoConstraints = false
        let cancelButton = NSButton(title: "Cancel", target: self, action: #selector(cancel))
        cancelButton.bezelStyle = .rounded
        cancelButton.translatesAutoresizingMaskIntoConstraints = false

        container.addSubview(titleField)
        container.addSubview(categoryPopup)
        container.addSubview(statusPopup)
        container.addSubview(ratingSlider)
        container.addSubview(ratingLabel)
        container.addSubview(noRatingToggle)
        container.addSubview(scroll)
        container.addSubview(saveButton)
        container.addSubview(cancelButton)
        contentView = container

        NSLayoutConstraint.activate([
            titleField.topAnchor.constraint(equalTo: container.topAnchor, constant: 16),
            titleField.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 16),
            titleField.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -16),
            categoryPopup.topAnchor.constraint(equalTo: titleField.bottomAnchor, constant: 8),
            categoryPopup.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            statusPopup.centerYAnchor.constraint(equalTo: categoryPopup.centerYAnchor),
            statusPopup.leadingAnchor.constraint(equalTo: categoryPopup.trailingAnchor, constant: 12),
            ratingSlider.topAnchor.constraint(equalTo: categoryPopup.bottomAnchor, constant: 10),
            ratingSlider.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            ratingSlider.widthAnchor.constraint(equalToConstant: 200),
            ratingLabel.centerYAnchor.constraint(equalTo: ratingSlider.centerYAnchor),
            ratingLabel.leadingAnchor.constraint(equalTo: ratingSlider.trailingAnchor, constant: 8),
            noRatingToggle.centerYAnchor.constraint(equalTo: ratingSlider.centerYAnchor),
            noRatingToggle.leadingAnchor.constraint(equalTo: ratingLabel.trailingAnchor, constant: 12),
            scroll.topAnchor.constraint(equalTo: ratingSlider.bottomAnchor, constant: 10),
            scroll.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: titleField.trailingAnchor),
            scroll.heightAnchor.constraint(equalToConstant: 160),
            cancelButton.topAnchor.constraint(equalTo: scroll.bottomAnchor, constant: 12),
            cancelButton.trailingAnchor.constraint(equalTo: saveButton.leadingAnchor, constant: -8),
            saveButton.topAnchor.constraint(equalTo: scroll.bottomAnchor, constant: 12),
            saveButton.trailingAnchor.constraint(equalTo: titleField.trailingAnchor)
        ])
    }

    @objc private func ratingChanged() {
        let disabled = noRatingToggle.state == .on
        ratingSlider.isEnabled = !disabled
        if disabled {
            ratingLabel.stringValue = "No rating"
        } else {
            ratingLabel.stringValue = "\(ratingSlider.integerValue)/10"
        }
    }

    @objc private func save() {
        let title = titleField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !title.isEmpty else { return }
        guard categoryPopup.indexOfSelectedItem >= 0 else { return }
        draft.title = title
        draft.category = MediaCategory.allCases[categoryPopup.indexOfSelectedItem]
        draft.status = MediaStatus.allCases[statusPopup.indexOfSelectedItem]
        draft.rating = noRatingToggle.state == .on ? nil : ratingSlider.integerValue
        draft.notes = notesView.string
        if draft.status == .done, draft.completedAt == nil {
            draft.completedAt = Date()
        }
        onSave(draft)
        close()
    }

    @objc private func cancel() {
        close()
    }
}
