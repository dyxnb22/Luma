import AppKit
import LumaCore
import LumaModules

@MainActor
final class ProjectsDetailView: NSObject, ModuleDetailView {
    let moduleTitle = "Projects"
    let detailView: NSView
    let usesSharedTopBar = true

    private let module: ProjectsModule
    private let onRunProjectAction: (ProjectAction, @escaping () -> Void) -> Void
    private let tableScroll = NSScrollView()
    private let tableView = NSTableView()
    private var records: [ProjectRecord] = []
    private var refreshTask: Task<Void, Never>?

    init(module: ProjectsModule, onRunProjectAction: @escaping (ProjectAction, @escaping () -> Void) -> Void) {
        self.module = module
        self.onRunProjectAction = onRunProjectAction
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
        let addRootButton = NSButton(title: "Add Root…", target: self, action: #selector(addRoot))
        addRootButton.bezelStyle = .rounded
        chrome.setToolbar(addRootButton)

        tableView.headerView = NSTableHeaderView()
        GeekUIKit.configureDetailTable(tableView, rowHeight: 40)
        tableView.delegate = self
        tableView.dataSource = self
        tableView.target = self
        tableView.doubleAction = #selector(openSelected)

        for (id, title, width) in [
            ("name", "Project", 160.0),
            ("path", "Path", 220.0),
            ("pinned", "Pin", 50.0),
            ("opener", "Opener", 90.0)
        ] {
            let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(id))
            column.title = title
            column.width = width
            tableView.addTableColumn(column)
        }

        tableScroll.documentView = tableView
        tableScroll.hasVerticalScroller = true
        tableScroll.translatesAutoresizingMaskIntoConstraints = false
        chrome.setContent(tableScroll, embedInScroll: false)

        let footer = NSTextField(labelWithString: "Double-click to open · proj manage for roots and pins")
        GeekUIKit.configureStatusLabel(footer)
        chrome.setFooter(footer, height: LauncherChromeTokens.detailFooterHeight)
    }

    private func refresh() {
        refreshTask?.cancel()
        refreshTask = Task { [weak self] in
            guard let self else { return }
            let loaded = await module.allRecords()
            await MainActor.run {
                self.records = loaded
                self.tableView.reloadData()
            }
        }
    }

    @objc private func addRoot() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "Add Root"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        onRunProjectAction(.addRoot(url.path)) { [weak self] in
            self?.refresh()
        }
    }

    @objc private func openSelected() {
        let row = tableView.selectedRow
        guard records.indices.contains(row) else { return }
        let record = records[row]
        onRunProjectAction(.open(path: record.path, opener: record.preferredOpener)) {}
    }
}

extension ProjectsDetailView: NSTableViewDataSource, NSTableViewDelegate {
    func numberOfRows(in tableView: NSTableView) -> Int { records.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard records.indices.contains(row), let id = tableColumn?.identifier.rawValue else { return nil }
        let record = records[row]
        switch id {
        case "name":
            return textCell(record.name)
        case "path":
            return textCell(shortPath(record.path), toolTip: record.path)
        case "pinned":
            let button = NSButton(checkboxWithTitle: "", target: self, action: #selector(togglePin(_:)))
            button.state = record.pinned ? .on : .off
            button.tag = row
            return button
        case "opener":
            let popup = NSPopUpButton(frame: .zero, pullsDown: false)
            for opener in ProjectOpener.allCases {
                popup.addItem(withTitle: opener.rawValue)
            }
            popup.selectItem(withTitle: record.preferredOpener.rawValue)
            popup.tag = row
            popup.target = self
            popup.action = #selector(changeOpener(_:))
            return popup
        default:
            return nil
        }
    }

    private func textCell(_ value: String, toolTip: String? = nil) -> NSTableCellView {
        let cell = NSTableCellView()
        let label = NSTextField(labelWithString: value)
        label.lineBreakMode = .byTruncatingMiddle
        label.translatesAutoresizingMaskIntoConstraints = false
        label.toolTip = toolTip ?? value
        cell.addSubview(label)
        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: cell.leadingAnchor, constant: 4),
            label.trailingAnchor.constraint(equalTo: cell.trailingAnchor, constant: -4),
            label.centerYAnchor.constraint(equalTo: cell.centerYAnchor)
        ])
        cell.textField = label
        return cell
    }

    private func shortPath(_ path: String) -> String {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        if path.hasPrefix(home) { return "~" + path.dropFirst(home.count) }
        return path
    }

    @objc private func togglePin(_ sender: NSButton) {
        let row = sender.tag
        guard records.indices.contains(row) else { return }
        onRunProjectAction(.togglePin(path: records[row].path)) { [weak self] in
            self?.refresh()
        }
    }

    @objc private func changeOpener(_ sender: NSPopUpButton) {
        let row = sender.tag
        guard records.indices.contains(row),
              let title = sender.titleOfSelectedItem,
              let opener = ProjectOpener(rawValue: title) else { return }
        onRunProjectAction(.updateOpener(path: records[row].path, opener: opener)) { [weak self] in
            self?.refresh()
        }
    }
}
