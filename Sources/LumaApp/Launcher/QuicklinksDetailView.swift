import AppKit
import LumaCore
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
    private let conflictLabel = NSTextField(labelWithString: "")
    private var quicklinks: [Quicklink] = []
    private var refreshTask: Task<Void, Never>?
    private var pendingDraft: URLQuicklinkDraft?
    nonisolated(unsafe) private var editorObservers: [NSObjectProtocol] = []

    init(module: QuicklinksModule) {
        self.module = module
        let chrome = BaseDetailContainer()
        self.detailView = chrome
        super.init()
        setup(chrome: chrome)
    }

    deinit {
        for token in editorObservers {
            NotificationCenter.default.removeObserver(token)
        }
    }

    func activate() {
        if let draft = LauncherSharedState.pendingQuicklinkDraft {
            LauncherSharedState.pendingQuicklinkDraft = nil
            pendingDraft = draft
        }
        refresh()
        DispatchQueue.main.async { [weak self] in
            self?.syncListScrollDocumentFrame()
        }
    }

    @objc private func syncListScrollDocumentFrame() {
        GeekUIKit.syncVerticalListDocumentFrame(in: tableScroll)
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
            ("name", "Name", 140.0),
            ("openWith", "Open With", 110.0),
            ("url", "Template", 240.0)
        ] {
            let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(id))
            column.title = title
            column.width = width
            tableView.addTableColumn(column)
        }
        GeekUIKit.styleDetailTableColumns(tableView)
        if let triggerColumn = tableView.tableColumn(withIdentifier: NSUserInterfaceItemIdentifier("trigger")) {
            GeekUIKit.configureDetailTableColumn(triggerColumn, minWidth: 64, maxWidth: 96, resizingMask: .userResizingMask)
        }
        if let nameColumn = tableView.tableColumn(withIdentifier: NSUserInterfaceItemIdentifier("name")) {
            GeekUIKit.configureDetailTableColumn(nameColumn, minWidth: 100, maxWidth: 220, resizingMask: [.autoresizingMask, .userResizingMask])
        }
        if let openWithColumn = tableView.tableColumn(withIdentifier: NSUserInterfaceItemIdentifier("openWith")) {
            GeekUIKit.configureDetailTableColumn(openWithColumn, minWidth: 90, maxWidth: 160, resizingMask: .userResizingMask)
        }
        if let urlColumn = tableView.tableColumn(withIdentifier: NSUserInterfaceItemIdentifier("url")) {
            GeekUIKit.configureDetailTableColumn(urlColumn, minWidth: 160, resizingMask: [.autoresizingMask, .userResizingMask])
        }
        tableView.columnAutoresizingStyle = .lastColumnOnlyAutoresizingStyle
        tableScroll.documentView = tableView
        GeekUIKit.wireVerticalListScroll(
            tableScroll,
            documentView: tableView,
            onClipViewResize: { [weak self] in
                self?.syncListScrollDocumentFrame()
            }
        )

        let editor = NSGridView(views: [
            [label("Trigger"), triggerField],
            [label("Name"), nameField],
            [label("URL Template"), urlField],
            [label("Open With Bundle ID"), openWithField],
            [label("Sample"), previewLabel],
            [label(""), conflictLabel]
        ])
        editor.rowSpacing = 8
        editor.columnSpacing = 10
        editor.translatesAutoresizingMaskIntoConstraints = false
        previewLabel.lineBreakMode = .byWordWrapping
        previewLabel.maximumNumberOfLines = 3
        previewLabel.textColor = .secondaryLabelColor
        urlField.lineBreakMode = .byWordWrapping
        urlField.cell?.wraps = true
        urlField.cell?.isScrollable = false
        urlField.maximumNumberOfLines = 3
        urlField.preferredMaxLayoutWidth = 480
        conflictLabel.lineBreakMode = .byWordWrapping
        conflictLabel.maximumNumberOfLines = 2
        conflictLabel.textColor = .systemOrange
        conflictLabel.isHidden = true
        for field in [nameField, triggerField, urlField, openWithField] {
            field.target = self
            field.action = #selector(editorChanged)
            let token = LumaNotificationCenter.observe(
                name: NSTextField.textDidChangeNotification,
                object: field
            ) { [weak self] in
                self?.editorChanged()
            }
            editorObservers.append(token)
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
                GeekUIKit.syncVerticalListDocumentFrame(in: self.tableScroll)
                if let draft = self.pendingDraft {
                    self.tableView.deselectAll(nil)
                    self.loadDraftIntoEditor(draft)
                    self.pendingDraft = nil
                } else if let id, let idx = loaded.firstIndex(where: { $0.id == id }) {
                    self.tableView.selectRowIndexes(IndexSet(integer: idx), byExtendingSelection: false)
                } else if !loaded.isEmpty, self.tableView.selectedRow < 0 {
                    self.tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
                }
                if self.tableView.selectedRow >= 0 {
                    self.loadSelectedIntoEditor()
                }
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
        let quicklink = quicklinks[row]
        let alert = NSAlert()
        alert.messageText = L10n.tr("quicklinks.detail.delete.title", quicklink.name)
        alert.informativeText = L10n.tr("detail.delete.cannotUndo")
        alert.addButton(withTitle: L10n.tr("detail.delete.confirm"))
        alert.addButton(withTitle: L10n.tr("detail.delete.cancel"))
        alert.alertStyle = .warning
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        let id = quicklink.id
        Task { [weak self] in
            guard let self else { return }
            do {
                try await self.module.delete(id: id)
                await MainActor.run { self.refresh() }
            } catch {
                await MainActor.run {
                    LauncherEnvironment.current?.showStatus(LauncherStatusMessages.deleteFailed)
                }
            }
        }
    }

    @objc private func saveSelected() {
        let row = tableView.selectedRow
        Task { [weak self] in
            guard let self else { return }
            let trigger = self.triggerField.stringValue
            let name = self.nameField.stringValue
            let urlTemplate = self.urlField.stringValue
            let openWith = self.openWithField.stringValue.isEmpty ? nil : self.openWithField.stringValue
            let editingID = self.quicklinks.indices.contains(row) ? self.quicklinks[row].id : nil
            if let validation = await self.module.validateURLTemplate(urlTemplate) {
                await MainActor.run {
                    self.conflictLabel.stringValue = validation
                    self.conflictLabel.isHidden = false
                    if validation.contains("http") {
                        LauncherEnvironment.current?.showStatus(LauncherStatusMessages.quicklinkMissingProtocol)
                    }
                }
                return
            }
            if let conflict = await self.module.conflictingQuicklink(trigger: trigger, excluding: editingID) {
                await MainActor.run {
                    self.conflictLabel.stringValue =
                        "Trigger “\(conflict.trigger)” is used by “\(conflict.name)”. Pick another trigger."
                    self.conflictLabel.isHidden = false
                    LauncherEnvironment.current?.showStatus(LauncherStatusMessages.quicklinkTriggerTaken)
                }
                return
            }
            let duplicate = await self.module.duplicateQuicklink(urlTemplate: urlTemplate, excluding: editingID)
            if let duplicate {
                await MainActor.run {
                    self.conflictLabel.stringValue =
                        "URL matches “\(duplicate.name)” (\(duplicate.trigger)). Review before saving another quicklink."
                    self.conflictLabel.isHidden = false
                    LauncherEnvironment.current?.showStatus(LauncherStatusMessages.quicklinkURLDuplicate)
                }
            }
            if self.quicklinks.indices.contains(row) {
                var quicklink = self.quicklinks[row]
                quicklink.trigger = trigger
                quicklink.name = name
                quicklink.urlTemplate = urlTemplate
                quicklink.openWith = openWith
                let saved = try? await self.module.update(quicklink)
                await MainActor.run {
                    self.conflictLabel.isHidden = true
                    LauncherEnvironment.current?.showStatus(
                        saved == nil ? LauncherStatusMessages.quicklinkSaveFailed : LauncherStatusMessages.quicklinkSaved
                    )
                    self.refresh(select: saved?.id ?? quicklink.id)
                }
            } else {
                let saved = try? await self.module.add(
                    Quicklink(name: name, trigger: trigger, urlTemplate: urlTemplate, openWith: openWith, icon: "link")
                )
                await MainActor.run {
                    self.conflictLabel.isHidden = true
                    LauncherEnvironment.current?.showStatus(
                        saved == nil ? LauncherStatusMessages.quicklinkSaveFailed : LauncherStatusMessages.quicklinkSaved
                    )
                    self.refresh(select: saved?.id)
                }
            }
        }
    }

    @objc private func selectionChanged() {
        loadSelectedIntoEditor()
    }

    @objc private func editorChanged() {
        updatePreview()
        updateConflictHint()
    }

    private func loadSelectedIntoEditor() {
        let row = tableView.selectedRow
        guard quicklinks.indices.contains(row) else {
            for field in [nameField, triggerField, urlField, openWithField] { field.stringValue = "" }
            previewLabel.stringValue = ""
            conflictLabel.isHidden = true
            conflictLabel.stringValue = ""
            return
        }
        let quicklink = quicklinks[row]
        triggerField.stringValue = quicklink.trigger
        nameField.stringValue = quicklink.name
        urlField.stringValue = quicklink.urlTemplate
        openWithField.stringValue = quicklink.openWith ?? ""
        updatePreview()
        updateConflictHint()
    }

    private func loadDraftIntoEditor(_ draft: URLQuicklinkDraft) {
        triggerField.stringValue = draft.trigger
        nameField.stringValue = draft.name.isEmpty ? draft.trigger.uppercased() : draft.name
        urlField.stringValue = draft.urlTemplate
        openWithField.stringValue = ""
        updatePreview()
        updateConflictHint()
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

    private func updateConflictHint() {
        let row = tableView.selectedRow
        let editingID = quicklinks.indices.contains(row) ? quicklinks[row].id : nil
        Task { [weak self] in
            guard let self else { return }
            let trigger = await MainActor.run { self.triggerField.stringValue }
            let urlTemplate = await MainActor.run { self.urlField.stringValue }
            if let validation = await self.module.validateURLTemplate(urlTemplate) {
                await MainActor.run {
                    self.conflictLabel.stringValue = validation
                    self.conflictLabel.isHidden = false
                }
                return
            }
            if let conflict = await self.module.conflictingQuicklink(trigger: trigger, excluding: editingID) {
                await MainActor.run {
                    self.conflictLabel.stringValue =
                        "Trigger “\(conflict.trigger)” is used by “\(conflict.name)”."
                    self.conflictLabel.isHidden = false
                }
                return
            }
            if let duplicate = await self.module.duplicateQuicklink(urlTemplate: urlTemplate, excluding: editingID) {
                await MainActor.run {
                    self.conflictLabel.stringValue =
                        "URL matches existing quicklink “\(duplicate.name)”."
                    self.conflictLabel.isHidden = false
                }
                return
            }
            await MainActor.run {
                self.conflictLabel.isHidden = true
                self.conflictLabel.stringValue = ""
            }
        }
    }
}

extension QuicklinksDetailView: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int { quicklinks.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard quicklinks.indices.contains(row), let id = tableColumn?.identifier.rawValue else { return nil }
        let q = quicklinks[row]
        let value: String
        let toolTip: String?
        switch id {
        case "trigger":
            value = q.trigger
            toolTip = "Trigger: \(q.trigger)"
        case "name":
            value = q.name
            toolTip = q.name
        case "url":
            value = q.urlTemplate
            toolTip = q.urlTemplate
        case "openWith":
            value = q.openWith ?? ""
            toolTip = q.openWith
        default:
            value = ""
            toolTip = nil
        }
        return GeekUIKit.makeDetailTableCell(
            text: value,
            lineBreak: id == "url" ? .byTruncatingMiddle : .byTruncatingTail,
            toolTip: toolTip
        )
    }
}
