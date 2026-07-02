import AppKit
import LumaCore
import LumaModules

@MainActor
final class MediaDetailView: NSObject, ModuleDetailView {
    private enum RecordFilter: Equatable {
        case all
        case category(MediaCategory)
        case status(MediaStatus)

        static let options: [(title: String, filter: RecordFilter)] = [
            ("All", .all),
            ("Books", .category(.book)),
            ("Movies", .category(.movie)),
            ("TV", .category(.tv)),
            ("Anime", .category(.anime)),
            ("Games", .category(.game)),
            ("In Progress", .status(.inProgress)),
            ("Planned", .status(.planned)),
            ("Done", .status(.done)),
            ("Dropped", .status(.abandoned))
        ]
    }

    let moduleTitle = "Records"
    let detailView: NSView
    let usesSharedTopBar = true

    private let module: MediaModule
    private let detailReloadRouter: ModuleDetailReloadRouter
    private let searchField = NSSearchField()
    private let filterPopup = NSPopUpButton()
    private let sortPopup = NSPopUpButton()
    private let tableScroll = NSScrollView()
    private let tableView = NSTableView()
    private let footerLabel = NSTextField(labelWithString: "")
    private var items: [MediaItem] = []
    private var refreshTask: Task<Void, Never>?
    private var selectedFilter: RecordFilter = .all

    init(module: MediaModule, detailReloadRouter: ModuleDetailReloadRouter) {
        self.module = module
        self.detailReloadRouter = detailReloadRouter
        let chrome = BaseDetailContainer()
        self.detailView = chrome
        super.init()
        setup(chrome: chrome)
    }

    func activate() {
        detailReloadRouter.register(.media) { [weak self] in self?.refresh() }
        refresh()
        if let draft = LauncherSharedState.pendingMediaEditorDraft {
            LauncherSharedState.pendingMediaEditorDraft = nil
            presentEditor(draft: draft)
        }
    }

    func deactivate() {
        detailReloadRouter.unregister(.media)
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
        } onDelete: { [weak self] id in
            guard let self else { return }
            Task {
                try? await self.module.delete(id: id)
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
            detailView.window?.makeFirstResponder(searchField)
            return true
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

        for (index, option) in RecordFilter.options.enumerated() {
            filterPopup.addItem(withTitle: option.title)
            if index == 0 { filterPopup.selectItem(at: 0) }
        }
        filterPopup.target = self
        filterPopup.action = #selector(filterChanged)
        filterPopup.translatesAutoresizingMaskIntoConstraints = false

        sortPopup.addItem(withTitle: "Recently Updated")
        sortPopup.addItem(withTitle: "Recently Added")
        sortPopup.addItem(withTitle: "Rating")
        sortPopup.addItem(withTitle: "Title")
        sortPopup.target = self
        sortPopup.action = #selector(filterChanged)
        sortPopup.translatesAutoresizingMaskIntoConstraints = false

        tableView.headerView = NSTableHeaderView()
        GeekUIKit.configureDetailTable(tableView, rowHeight: 36)
        tableView.delegate = self
        tableView.dataSource = self
        tableView.target = self
        tableView.doubleAction = #selector(editSelected)
        tableView.translatesAutoresizingMaskIntoConstraints = false

        for (id, title, width) in [
            ("title", "Title", 160.0),
            ("category", "Category", 72.0),
            ("status", "Status", 88.0),
            ("rating", "Rating", 56.0),
            ("tags", "Tags", 100.0),
            ("updated", "Updated", 96.0)
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

        GeekUIKit.configureStatusLabel(footerLabel)
        footerLabel.translatesAutoresizingMaskIntoConstraints = false

        header.addSubview(toolbar)
        header.addSubview(filterPopup)
        header.addSubview(sortPopup)

        chrome.setToolbar(header, height: LauncherChromeTokens.detailToolbarTallHeight + 8)
        chrome.setFooter(footerLabel, height: LauncherChromeTokens.detailFooterHeight)
        chrome.setContent(tableScroll, embedInScroll: false)

        NSLayoutConstraint.activate([
            toolbar.topAnchor.constraint(equalTo: header.topAnchor),
            toolbar.leadingAnchor.constraint(equalTo: header.leadingAnchor),
            toolbar.trailingAnchor.constraint(equalTo: header.trailingAnchor),
            toolbar.heightAnchor.constraint(equalToConstant: LauncherChromeTokens.detailToolbarHeight),

            filterPopup.topAnchor.constraint(equalTo: toolbar.bottomAnchor, constant: 8),
            filterPopup.leadingAnchor.constraint(equalTo: header.leadingAnchor),

            sortPopup.centerYAnchor.constraint(equalTo: filterPopup.centerYAnchor),
            sortPopup.leadingAnchor.constraint(equalTo: filterPopup.trailingAnchor, constant: 12),
            sortPopup.bottomAnchor.constraint(equalTo: header.bottomAnchor)
        ])

        searchField.target = self
        searchField.action = #selector(searchChanged)
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(searchChanged),
            name: NSSearchField.textDidChangeNotification,
            object: searchField
        )
    }

    private func buildToolbar() -> NSView {
        let toolbar = NSView()
        toolbar.translatesAutoresizingMaskIntoConstraints = false

        searchField.placeholderString = "Search records…"
        GeekUIKit.styleDetailSearchField(searchField)
        searchField.translatesAutoresizingMaskIntoConstraints = false

        let addButton = GeekUIKit.makeToolbarButton("New", target: self, action: #selector(addItem))
        let exportButton = GeekUIKit.makeToolbarButton("Export CSV", target: self, action: #selector(exportCSV))
        let buttonStack = NSStackView(views: [addButton, exportButton])

        toolbar.addSubview(searchField)
        GeekUIKit.constrainDetailToolbarTrailingActions(buttonStack, in: toolbar, after: searchField)

        NSLayoutConstraint.activate([
            searchField.leadingAnchor.constraint(equalTo: toolbar.leadingAnchor),
            searchField.centerYAnchor.constraint(equalTo: toolbar.centerYAnchor),
            searchField.widthAnchor.constraint(greaterThanOrEqualToConstant: 120)
        ])
        return toolbar
    }

    @objc private func filterChanged() {
        let index = filterPopup.indexOfSelectedItem
        selectedFilter = RecordFilter.options.indices.contains(index)
            ? RecordFilter.options[index].filter
            : .all
        refresh()
    }

    @objc private func searchChanged() {
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
        let filter = selectedFilter
        let searchText = searchField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
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
            let category: MediaCategory?
            let status: MediaStatus?
            switch filter {
            case .all:
                category = nil
                status = nil
            case .category(let value):
                category = value
                status = nil
            case .status(let value):
                category = nil
                status = value
            }
            var filtered = MediaIndex.filter(all, category: category, status: status, sort: sort)
            if !searchText.isEmpty {
                let limit = max(filtered.count, 8)
                filtered = MediaIndex.search(filtered, query: searchText, limit: limit).map(\.item)
            }
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
        case "tags":
            cell.stringValue = item.tags.isEmpty ? "—" : item.tags.map { "#\($0)" }.joined(separator: " ")
            cell.textColor = .secondaryLabelColor
        case "updated":
            let date = item.completedAt ?? item.updatedAt
            cell.stringValue = Self.dateFormatter.string(from: date)
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
private final class MediaItemEditorSheet: LumaWindow {
    private let onSave: (MediaEditorDraft) -> Void
    private let onDelete: ((UUID) -> Void)?
    private let titleField = NSTextField()
    private let categoryPopup = NSPopUpButton()
    private let statusPopup = NSPopUpButton()
    private let ratingSlider = NSSlider()
    private let ratingLabel = NSTextField(labelWithString: "No rating")
    private let noRatingToggle = NSButton()
    private let startedPicker = NSDatePicker()
    private let completedPicker = NSDatePicker()
    private let clearStartedButton = NSButton()
    private let clearCompletedButton = NSButton()
    private let tagsField = NSTextField()
    private let notesView = NSTextView()
    private var draft: MediaEditorDraft

    init(
        draft: MediaEditorDraft,
        onSave: @escaping (MediaEditorDraft) -> Void,
        onDelete: ((UUID) -> Void)? = nil
    ) {
        self.draft = draft
        self.onSave = onSave
        self.onDelete = onDelete
        super.init(contentRect: NSRect(x: 0, y: 0, width: 500, height: 520), styleMask: [.titled, .closable], backing: .buffered, defer: false)
        title = draft.existingID == nil ? "Add Record" : "Edit Record"
        setup()
    }

    private func setup() {
        let container = NSView(frame: NSRect(x: 0, y: 0, width: 500, height: 520))
        titleField.stringValue = draft.title
        titleField.placeholderString = "Title"
        titleField.translatesAutoresizingMaskIntoConstraints = false

        categoryPopup.addItem(withTitle: "Choose Category")
        for category in MediaCategory.allCases {
            categoryPopup.addItem(withTitle: category.displayName)
        }
        if let category = draft.category, let index = MediaCategory.allCases.firstIndex(of: category) {
            categoryPopup.selectItem(at: index + 1)
        } else {
            categoryPopup.selectItem(at: 0)
        }
        categoryPopup.target = self
        categoryPopup.action = #selector(categoryChanged)
        categoryPopup.translatesAutoresizingMaskIntoConstraints = false

        rebuildStatusMenu()
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

        if let date = draft.startedAt {
            configureDatePicker(startedPicker, date: date, enabled: true)
            clearStartedButton.title = "Clear"
        } else {
            configureDatePicker(startedPicker, date: Date(), enabled: false)
            clearStartedButton.title = "Set"
        }
        if let date = draft.completedAt {
            configureDatePicker(completedPicker, date: date, enabled: true)
            clearCompletedButton.title = "Clear"
        } else {
            configureDatePicker(completedPicker, date: Date(), enabled: false)
            clearCompletedButton.title = "Set"
        }

        clearStartedButton.bezelStyle = .rounded
        clearStartedButton.target = self
        clearStartedButton.action = #selector(toggleStarted)
        clearStartedButton.translatesAutoresizingMaskIntoConstraints = false

        clearCompletedButton.bezelStyle = .rounded
        clearCompletedButton.target = self
        clearCompletedButton.action = #selector(toggleCompleted)
        clearCompletedButton.translatesAutoresizingMaskIntoConstraints = false

        tagsField.stringValue = draft.tags.joined(separator: ", ")
        tagsField.placeholderString = "Tags (comma-separated)"
        tagsField.translatesAutoresizingMaskIntoConstraints = false

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

        let deleteButton = NSButton(title: "Delete", target: self, action: #selector(deleteItem))
        deleteButton.bezelStyle = .rounded
        deleteButton.isHidden = draft.existingID == nil
        deleteButton.translatesAutoresizingMaskIntoConstraints = false

        let startedLabel = label("Started")
        let completedLabel = label("Completed")
        let tagsLabel = label("Tags")
        let notesLabel = label("Notes")

        container.addSubview(titleField)
        container.addSubview(categoryPopup)
        container.addSubview(statusPopup)
        container.addSubview(ratingSlider)
        container.addSubview(ratingLabel)
        container.addSubview(noRatingToggle)
        container.addSubview(startedLabel)
        container.addSubview(startedPicker)
        container.addSubview(clearStartedButton)
        container.addSubview(completedLabel)
        container.addSubview(completedPicker)
        container.addSubview(clearCompletedButton)
        container.addSubview(tagsLabel)
        container.addSubview(tagsField)
        container.addSubview(notesLabel)
        container.addSubview(scroll)
        container.addSubview(saveButton)
        container.addSubview(cancelButton)
        container.addSubview(deleteButton)
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

            startedLabel.topAnchor.constraint(equalTo: ratingSlider.bottomAnchor, constant: 10),
            startedLabel.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            startedPicker.centerYAnchor.constraint(equalTo: startedLabel.centerYAnchor),
            startedPicker.leadingAnchor.constraint(equalTo: startedLabel.trailingAnchor, constant: 8),
            clearStartedButton.centerYAnchor.constraint(equalTo: startedLabel.centerYAnchor),
            clearStartedButton.leadingAnchor.constraint(equalTo: startedPicker.trailingAnchor, constant: 8),

            completedLabel.topAnchor.constraint(equalTo: startedLabel.bottomAnchor, constant: 10),
            completedLabel.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            completedPicker.centerYAnchor.constraint(equalTo: completedLabel.centerYAnchor),
            completedPicker.leadingAnchor.constraint(equalTo: completedLabel.trailingAnchor, constant: 8),
            clearCompletedButton.centerYAnchor.constraint(equalTo: completedLabel.centerYAnchor),
            clearCompletedButton.leadingAnchor.constraint(equalTo: completedPicker.trailingAnchor, constant: 8),

            tagsLabel.topAnchor.constraint(equalTo: completedLabel.bottomAnchor, constant: 10),
            tagsLabel.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            tagsField.centerYAnchor.constraint(equalTo: tagsLabel.centerYAnchor),
            tagsField.leadingAnchor.constraint(equalTo: tagsLabel.trailingAnchor, constant: 8),
            tagsField.trailingAnchor.constraint(equalTo: titleField.trailingAnchor),

            notesLabel.topAnchor.constraint(equalTo: tagsLabel.bottomAnchor, constant: 10),
            notesLabel.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            scroll.topAnchor.constraint(equalTo: notesLabel.bottomAnchor, constant: 4),
            scroll.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: titleField.trailingAnchor),
            scroll.heightAnchor.constraint(equalToConstant: 120),

            deleteButton.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            deleteButton.topAnchor.constraint(equalTo: scroll.bottomAnchor, constant: 12),
            cancelButton.trailingAnchor.constraint(equalTo: saveButton.leadingAnchor, constant: -8),
            cancelButton.centerYAnchor.constraint(equalTo: saveButton.centerYAnchor),
            saveButton.trailingAnchor.constraint(equalTo: titleField.trailingAnchor),
            saveButton.topAnchor.constraint(equalTo: scroll.bottomAnchor, constant: 12)
        ])
    }

    private func label(_ text: String) -> NSTextField {
        let field = NSTextField(labelWithString: text)
        field.font = .systemFont(ofSize: 12)
        field.textColor = .secondaryLabelColor
        field.translatesAutoresizingMaskIntoConstraints = false
        return field
    }

    private func configureDatePicker(_ picker: NSDatePicker, date: Date, enabled: Bool) {
        picker.datePickerStyle = .textFieldAndStepper
        picker.datePickerElements = .yearMonthDay
        picker.dateValue = date
        picker.isEnabled = enabled
        picker.translatesAutoresizingMaskIntoConstraints = false
    }

    private func selectedCategory() -> MediaCategory? {
        let index = categoryPopup.indexOfSelectedItem
        let categoryIndex = index - 1
        guard MediaCategory.allCases.indices.contains(categoryIndex) else { return nil }
        return MediaCategory.allCases[categoryIndex]
    }

    private func statusTitle(for status: MediaStatus, category: MediaCategory?) -> String {
        guard let category else { return status.displayName }
        return status.verb(for: category)
    }

    private func rebuildStatusMenu() {
        let category = selectedCategory() ?? draft.category
        let current = draft.status
        statusPopup.removeAllItems()
        for status in MediaStatus.allCases {
            statusPopup.addItem(withTitle: statusTitle(for: status, category: category))
        }
        if let index = MediaStatus.allCases.firstIndex(of: current) {
            statusPopup.selectItem(at: index)
        }
    }

    @objc private func categoryChanged() {
        if MediaStatus.allCases.indices.contains(statusPopup.indexOfSelectedItem) {
            draft.status = MediaStatus.allCases[statusPopup.indexOfSelectedItem]
        }
        rebuildStatusMenu()
    }

    @objc private func toggleStarted() {
        if startedPicker.isEnabled {
            startedPicker.isEnabled = false
            clearStartedButton.title = "Set"
        } else {
            startedPicker.isEnabled = true
            startedPicker.dateValue = Date()
            clearStartedButton.title = "Clear"
        }
    }

    @objc private func toggleCompleted() {
        if completedPicker.isEnabled {
            completedPicker.isEnabled = false
            clearCompletedButton.title = "Set"
        } else {
            completedPicker.isEnabled = true
            completedPicker.dateValue = Date()
            clearCompletedButton.title = "Clear"
        }
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
        guard let category = selectedCategory() else { return }
        draft.title = title
        draft.category = category
        draft.status = MediaStatus.allCases[statusPopup.indexOfSelectedItem]
        draft.rating = noRatingToggle.state == .on ? nil : ratingSlider.integerValue
        draft.startedAt = startedPicker.isEnabled ? startedPicker.dateValue : nil
        draft.completedAt = completedPicker.isEnabled ? completedPicker.dateValue : nil
        draft.tags = tagsField.stringValue
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        draft.notes = notesView.string
        if draft.status == .done, draft.completedAt == nil {
            draft.completedAt = Date()
        }
        onSave(draft)
        close()
    }

    @objc private func deleteItem() {
        guard let id = draft.existingID else { return }
        onDelete?(id)
        close()
    }

    @objc private func cancel() {
        close()
    }
}
