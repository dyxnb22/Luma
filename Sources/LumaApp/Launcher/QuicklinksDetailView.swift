import AppKit
import LumaModules

@MainActor
final class QuicklinksDetailView: NSObject, ModuleDetailView {
    let moduleTitle = "Quicklinks"
    let detailView: NSView
    let usesSharedTopBar = true

    private let module: QuicklinksModule
    private let tableView = NSTableView()
    private let tableScroll = NSScrollView()
    private let nameField = NSTextField()
    private let triggerField = NSTextField()
    private let urlField = NSTextField()
    private let openWithField = NSTextField()
    private let previewLabel = NSTextField(labelWithString: "")
    private var quicklinks: [Quicklink] = []
    private var refreshTask: Task<Void, Never>?

    init(module: QuicklinksModule) {
        self.module = module
        let chrome = BaseDetailContainer()
        self.detailView = chrome
        super.init()
        setup(chrome: chrome)
    }

    func activate() {
        refresh()
    }

    func deactivate() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    private func setup(chrome: BaseDetailContainer) {
        let toolbar = NSStackView()
        toolbar.orientation = .horizontal
        toolbar.spacing = 8
        toolbar.alignment = .centerY
        toolbar.addArrangedSubview(GeekUIKit.makeToolbarButton("Add", target: self, action: #selector(addQuicklink)))
        toolbar.addArrangedSubview(GeekUIKit.makeToolbarButton("Delete", target: self, action: #selector(deleteQuicklink)))
        toolbar.addArrangedSubview(GeekUIKit.makeToolbarButton("Save", target: self, action: #selector(saveSelected)))
        chrome.setToolbar(toolbar)

        tableView.headerView = NSTableHeaderView()
        GeekUIKit.configureDetailTable(tableView, rowHeight: 34)
        tableView.delegate = self
        tableView.dataSource = self
        tableView.action = #selector(selectionChanged)
        tableView.target = self
        for (id, title, width) in [
            ("trigger", "Trigger", 70.0),
            ("name", "Name", 150.0),
            ("url", "Template", 320.0),
            ("openWith", "Open With", 120.0)
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

        let editor = NSGridView(views: [
            [label("Trigger"), triggerField],
            [label("Name"), nameField],
            [label("URL Template"), urlField],
            [label("Open With Bundle ID"), openWithField],
            [label("Sample"), previewLabel]
        ])
        editor.rowSpacing = 8
        editor.columnSpacing = 10
        editor.translatesAutoresizingMaskIntoConstraints = false
        previewLabel.lineBreakMode = .byTruncatingMiddle
        previewLabel.textColor = .secondaryLabelColor
        for field in [nameField, triggerField, urlField, openWithField] {
            field.target = self
            field.action = #selector(editorChanged)
            NotificationCenter.default.addObserver(self, selector: #selector(editorChanged), name: NSTextField.textDidChangeNotification, object: field)
        }

        let split = NSSplitView()
        split.isVertical = false
        split.dividerStyle = .thin
        split.addArrangedSubview(tableScroll)
        split.addArrangedSubview(editor)
        chrome.setContent(split, embedInScroll: false)
    }

    private func label(_ value: String) -> NSTextField {
        let label = NSTextField(labelWithString: value)
        label.textColor = .secondaryLabelColor
        label.alignment = .right
        return label
    }

    private func refresh(select id: UUID? = nil) {
        refreshTask?.cancel()
        refreshTask = Task { [weak self] in
            guard let self else { return }
            let loaded = await module.allQuicklinks()
            await MainActor.run {
                self.quicklinks = loaded
                self.tableView.reloadData()
                if let id, let idx = loaded.firstIndex(where: { $0.id == id }) {
                    self.tableView.selectRowIndexes(IndexSet(integer: idx), byExtendingSelection: false)
                } else if !loaded.isEmpty, self.tableView.selectedRow < 0 {
                    self.tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
                }
                self.loadSelectedIntoEditor()
            }
        }
    }

    @objc private func addQuicklink() {
        Task { [weak self] in
            guard let self else { return }
            let saved = try? await self.module.add(Quicklink(name: "New Quicklink", trigger: "new", urlTemplate: "https://example.com/search?q={{query}}"))
            await MainActor.run { self.refresh(select: saved?.id) }
        }
    }

    @objc private func deleteQuicklink() {
        let row = tableView.selectedRow
        guard quicklinks.indices.contains(row) else { return }
        let id = quicklinks[row].id
        Task { [weak self] in
            try? await self?.module.delete(id: id)
            await MainActor.run { self?.refresh() }
        }
    }

    @objc private func saveSelected() {
        let row = tableView.selectedRow
        guard quicklinks.indices.contains(row) else { return }
        var quicklink = quicklinks[row]
        quicklink.trigger = triggerField.stringValue
        quicklink.name = nameField.stringValue
        quicklink.urlTemplate = urlField.stringValue
        quicklink.openWith = openWithField.stringValue.isEmpty ? nil : openWithField.stringValue
        Task { [weak self] in
            guard let self else { return }
            let saved = try? await self.module.update(quicklink)
            await MainActor.run { self.refresh(select: saved?.id ?? quicklink.id) }
        }
    }

    @objc private func selectionChanged() {
        loadSelectedIntoEditor()
    }

    @objc private func editorChanged() {
        updatePreview()
    }

    private func loadSelectedIntoEditor() {
        let row = tableView.selectedRow
        guard quicklinks.indices.contains(row) else {
            for field in [nameField, triggerField, urlField, openWithField] { field.stringValue = "" }
            previewLabel.stringValue = ""
            return
        }
        let quicklink = quicklinks[row]
        triggerField.stringValue = quicklink.trigger
        nameField.stringValue = quicklink.name
        urlField.stringValue = quicklink.urlTemplate
        openWithField.stringValue = quicklink.openWith ?? ""
        updatePreview()
    }

    private func updatePreview() {
        previewLabel.stringValue = QuicklinkTemplateRenderer.render(
            template: urlField.stringValue,
            query: "swift package",
            clipboard: NSPasteboard.general.string(forType: .string),
            selection: nil,
            project: nil,
            projectPath: nil
        )
    }
}

extension QuicklinksDetailView: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int { quicklinks.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard quicklinks.indices.contains(row), let id = tableColumn?.identifier.rawValue else { return nil }
        let q = quicklinks[row]
        let value: String
        switch id {
        case "trigger": value = q.trigger
        case "name": value = q.name
        case "url": value = q.urlTemplate
        case "openWith": value = q.openWith ?? ""
        default: value = ""
        }
        let cell = NSTableCellView()
        let label = NSTextField(labelWithString: value)
        label.lineBreakMode = .byTruncatingMiddle
        label.translatesAutoresizingMaskIntoConstraints = false
        cell.addSubview(label)
        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 4),
            label.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -4),
            label.centerYAnchor.constraint(equalTo: cell.centerYAnchor)
        ])
        return cell
    }
}
