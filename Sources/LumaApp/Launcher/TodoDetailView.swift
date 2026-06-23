import AppKit
import LumaModules
import LumaServices

@MainActor
final class TodoDetailView: NSObject, ModuleDetailView {
    private enum Tab: Int { case today = 0, upcoming = 1, completed = 2 }

    let moduleTitle = "Todo"
    let detailView: NSView
    let usesSharedTopBar = true

    private let module: TodoModule
    private let inputField = NSTextField()
    private let addButton = NSButton()
    private let refreshButton = NSButton()
    private let openRemindersButton = NSButton()
    private let tabControl = NSSegmentedControl()
    private let statusLabel = NSTextField(labelWithString: "")
    private let scrollView = NSScrollView()
    private let stackView = NSStackView()
    private var refreshTask: Task<Void, Never>?
    private var currentTab: Tab = .today

    init(module: TodoModule) {
        self.module = module
        let chrome = BaseDetailContainer()
        self.detailView = chrome
        super.init()
        setup(chrome: chrome)
    }

    func activate() {
        refresh()
        DispatchQueue.main.async { [weak self] in
            self?.detailView.window?.makeFirstResponder(self?.inputField)
        }
    }

    func deactivate() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    func handleKeyDown(_ event: NSEvent) -> Bool {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if flags.contains(.command), event.charactersIgnoringModifiers?.lowercased() == "t" {
            detailView.window?.makeFirstResponder(inputField)
            return true
        }
        if event.keyCode == 36, detailView.window?.firstResponder === inputField {
            addTodo()
            return true
        }
        if flags.contains(.command) {
            switch event.charactersIgnoringModifiers {
            case "1": selectTab(.today); return true
            case "2": selectTab(.upcoming); return true
            case "3": selectTab(.completed); return true
            default: break
            }
        }
        return false
    }

    private func setup(chrome: BaseDetailContainer) {
        let topBar = NSView()
        topBar.translatesAutoresizingMaskIntoConstraints = false

        inputField.placeholderString = "添加待办，可附 9:00 / +30m"
        inputField.font = .systemFont(ofSize: 14)
        inputField.target = self
        inputField.action = #selector(addTodo)
        inputField.translatesAutoresizingMaskIntoConstraints = false

        addButton.title = "Add"
        addButton.bezelStyle = .rounded
        addButton.target = self
        addButton.action = #selector(addTodo)
        addButton.translatesAutoresizingMaskIntoConstraints = false

        configureIconButton(refreshButton, symbol: "arrow.clockwise", tooltip: "Refresh")
        refreshButton.target = self
        refreshButton.action = #selector(refreshTapped)

        configureIconButton(openRemindersButton, symbol: "arrow.up.forward.app", tooltip: "Open Reminders")
        openRemindersButton.target = self
        openRemindersButton.action = #selector(openReminders)

        tabControl.segmentCount = 3
        tabControl.setLabel("Today", forSegment: 0)
        tabControl.setLabel("Upcoming", forSegment: 1)
        tabControl.setLabel("Completed", forSegment: 2)
        tabControl.selectedSegment = 0
        tabControl.target = self
        tabControl.action = #selector(tabChanged)
        tabControl.translatesAutoresizingMaskIntoConstraints = false

        statusLabel.font = .systemFont(ofSize: 12)
        statusLabel.textColor = .secondaryLabelColor
        statusLabel.lineBreakMode = .byTruncatingTail
        statusLabel.translatesAutoresizingMaskIntoConstraints = false

        topBar.addSubview(inputField)
        topBar.addSubview(addButton)
        topBar.addSubview(refreshButton)
        topBar.addSubview(openRemindersButton)
        topBar.addSubview(tabControl)

        stackView.orientation = .vertical
        stackView.alignment = .leading
        stackView.spacing = 8
        stackView.edgeInsets = NSEdgeInsets(top: 4, left: 8, bottom: 12, right: 8)
        stackView.translatesAutoresizingMaskIntoConstraints = false

        scrollView.documentView = stackView
        scrollView.hasVerticalScroller = true
        scrollView.drawsBackground = false
        scrollView.borderType = .noBorder
        scrollView.translatesAutoresizingMaskIntoConstraints = false

        chrome.setToolbar(topBar, height: 58)
        chrome.setFooter(statusLabel, height: 20)
        chrome.setContent(scrollView, embedInScroll: false)

        NSLayoutConstraint.activate([
            inputField.leadingAnchor.constraint(equalTo: topBar.leadingAnchor),
            inputField.topAnchor.constraint(equalTo: topBar.topAnchor, constant: 2),
            inputField.trailingAnchor.constraint(equalTo: addButton.leadingAnchor, constant: -8),

            addButton.trailingAnchor.constraint(equalTo: refreshButton.leadingAnchor, constant: -6),
            addButton.centerYAnchor.constraint(equalTo: inputField.centerYAnchor),
            addButton.widthAnchor.constraint(equalToConstant: 64),

            refreshButton.trailingAnchor.constraint(equalTo: openRemindersButton.leadingAnchor, constant: -4),
            refreshButton.centerYAnchor.constraint(equalTo: inputField.centerYAnchor),
            refreshButton.widthAnchor.constraint(equalToConstant: 26),
            refreshButton.heightAnchor.constraint(equalToConstant: 26),

            openRemindersButton.trailingAnchor.constraint(equalTo: topBar.trailingAnchor),
            openRemindersButton.centerYAnchor.constraint(equalTo: inputField.centerYAnchor),
            openRemindersButton.widthAnchor.constraint(equalToConstant: 26),
            openRemindersButton.heightAnchor.constraint(equalToConstant: 26),

            tabControl.leadingAnchor.constraint(equalTo: topBar.leadingAnchor),
            tabControl.bottomAnchor.constraint(equalTo: topBar.bottomAnchor, constant: -2),

            stackView.widthAnchor.constraint(equalTo: scrollView.contentView.widthAnchor, constant: -16)
        ])
    }

    private func selectTab(_ tab: Tab) {
        currentTab = tab
        tabControl.selectedSegment = tab.rawValue
        refresh()
    }

    @objc private func tabChanged() {
        currentTab = Tab(rawValue: tabControl.selectedSegment) ?? .today
        refresh()
    }

    private func configureIconButton(_ button: NSButton, symbol: String, tooltip: String) {
        button.bezelStyle = .texturedRounded
        button.isBordered = false
        button.image = NSImage(systemSymbolName: symbol, accessibilityDescription: tooltip)
        button.toolTip = tooltip
        button.translatesAutoresizingMaskIntoConstraints = false
    }

    @objc private func addTodo() {
        let raw = inputField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !raw.isEmpty else { return }
        setControlsEnabled(false)
        statusLabel.stringValue = "Adding..."
        Task { [weak self] in
            guard let self else { return }
            do {
                _ = try await module.createReminder(from: raw)
                await MainActor.run {
                    inputField.stringValue = ""
                    setControlsEnabled(true)
                    refresh()
                }
            } catch {
                await MainActor.run {
                    statusLabel.stringValue = userMessage(for: error)
                    setControlsEnabled(true)
                }
            }
        }
    }

    @objc private func refreshTapped() { refresh() }

    @objc private func openReminders() {
        NSWorkspace.shared.open(URL(fileURLWithPath: "/System/Applications/Reminders.app"))
    }

    private func refresh() {
        refreshTask?.cancel()
        statusLabel.stringValue = "Loading reminders..."
        let tab = currentTab
        refreshTask = Task { [weak self] in
            guard let self else { return }
            do {
                let authorization = await module.authorizationStatus()
                if authorization == .denied {
                    await MainActor.run { showPermissionState() }
                    return
                }
                switch tab {
                case .today:
                    let today = try await module.todayDue(limit: 30)
                    await MainActor.run { render(items: today, empty: "Press ⌘+T to add a reminder…") }
                case .upcoming:
                    let future = try await module.futureDue(limit: 30)
                    await MainActor.run { render(items: future, empty: "No upcoming reminders.") }
                case .completed:
                    let done = try await module.completedReminders(limit: 30)
                    await MainActor.run { renderCompleted(items: done, empty: "No completed reminders this week.") }
                }
            } catch RemindersServiceError.accessDenied {
                await MainActor.run { showPermissionState() }
            } catch {
                await MainActor.run {
                    clearRows()
                    statusLabel.stringValue = userMessage(for: error)
                }
            }
        }
    }

    private func updateTabLabel(tab: Tab, count: Int) {
        switch tab {
        case .today:
            tabControl.setLabel(count > 0 ? "Today (\(count))" : "Today", forSegment: 0)
        case .upcoming:
            tabControl.setLabel(count > 0 ? "Upcoming (\(count))" : "Upcoming", forSegment: 1)
        case .completed:
            tabControl.setLabel(count > 0 ? "Completed (\(count))" : "Completed", forSegment: 2)
        }
    }

    private func render(items: [ReminderSnapshot], empty: String) {
        clearRows()
        updateTabLabel(tab: currentTab, count: items.count)
        statusLabel.stringValue = "\(items.count) reminder\(items.count == 1 ? "" : "s")"
        if items.isEmpty {
            addPlaceholderRow(empty)
            return
        }
        for item in items {
            let row = TodoReminderRow(
                reminder: item,
                onComplete: { [weak self] in self?.complete(item) },
                onEdit: { [weak self] in self?.showEditPrompt(for: item) }
            )
            stackView.addArrangedSubview(row)
            row.widthAnchor.constraint(equalTo: stackView.widthAnchor).isActive = true
        }
    }

    private func renderCompleted(items: [ReminderSnapshot], empty: String) {
        clearRows()
        updateTabLabel(tab: .completed, count: items.count)
        statusLabel.stringValue = "\(items.count) completed"
        if items.isEmpty {
            addPlaceholderRow(empty)
            return
        }
        for item in items {
            let row = TodoCompletedRow(reminder: item) { [weak self] in
                self?.showEditPrompt(for: item)
            }
            stackView.addArrangedSubview(row)
            row.widthAnchor.constraint(equalTo: stackView.widthAnchor).isActive = true
        }
    }

    private func addPlaceholderRow(_ text: String) {
        let label = NSTextField(labelWithString: text)
        label.font = .systemFont(ofSize: 13)
        label.textColor = .tertiaryLabelColor
        stackView.addArrangedSubview(label)
        label.widthAnchor.constraint(equalTo: stackView.widthAnchor).isActive = true
    }

    private func clearRows() {
        stackView.arrangedSubviews.forEach { view in
            stackView.removeArrangedSubview(view)
            view.removeFromSuperview()
        }
    }

    private func complete(_ reminder: ReminderSnapshot) {
        Task { [weak self] in
            guard let self else { return }
            do {
                try await module.completeReminder(id: reminder.id)
                await MainActor.run { refresh() }
            } catch {
                await MainActor.run { statusLabel.stringValue = userMessage(for: error) }
            }
        }
    }

    private func showEditPrompt(for reminder: ReminderSnapshot) {
        guard let window = detailView.window else { return }
        let alert = NSAlert()
        alert.messageText = "Edit Reminder"
        alert.informativeText = "Use the same time syntax as Add, e.g. tomorrow 9:00 or +30m."
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 320, height: 24))
        field.stringValue = editString(for: reminder)
        alert.accessoryView = field
        alert.addButton(withTitle: "Save")
        alert.addButton(withTitle: "Cancel")
        alert.beginSheetModal(for: window) { [weak self] response in
            guard response == .alertFirstButtonReturn else { return }
            let raw = field.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !raw.isEmpty else { return }
            Task { await self?.update(reminder, rawTitle: raw) }
        }
    }

    private func update(_ reminder: ReminderSnapshot, rawTitle: String) async {
        do {
            _ = try await module.updateReminder(
                id: reminder.id,
                rawTitle: rawTitle,
                existingDueDate: reminder.dueDate
            )
            await MainActor.run {
                statusLabel.stringValue = "Updated"
                refresh()
            }
        } catch {
            await MainActor.run { statusLabel.stringValue = userMessage(for: error) }
        }
    }

    private func editString(for reminder: ReminderSnapshot) -> String {
        guard let dueDate = reminder.dueDate else { return reminder.title }
        let calendar = Calendar.current
        guard calendar.isDateInToday(dueDate) || calendar.isDateInTomorrow(dueDate) else {
            return reminder.title
        }
        return "\(reminder.title) \(TodoModule.formatDue(dueDate))"
    }

    private func showPermissionState() {
        clearRows()
        statusLabel.stringValue = "Reminders access is required."
        let grantButton = NSButton(title: "Grant Reminders Access", target: self, action: #selector(grantAccess))
        grantButton.bezelStyle = .rounded
        stackView.addArrangedSubview(grantButton)
    }

    @objc private func grantAccess() {
        Task { [weak self] in
            guard let self else { return }
            let result = await module.requestRemindersAccess()
            await MainActor.run {
                if result == .authorized {
                    refresh()
                } else {
                    statusLabel.stringValue = "Access denied. Open System Settings > Privacy & Security > Reminders."
                }
            }
        }
    }

    private func setControlsEnabled(_ enabled: Bool) {
        inputField.isEnabled = enabled
        addButton.isEnabled = enabled
        refreshButton.isEnabled = enabled
    }

    private func userMessage(for error: Error) -> String {
        if let reminderError = error as? RemindersServiceError {
            switch reminderError {
            case .accessDenied: return "Reminders access is required."
            case .noDefaultCalendar: return "No default Reminders list is configured."
            case .notFound: return "Reminder was not found."
            case .saveFailed(let message): return "Could not save reminder: \(message)"
            }
        }
        return error.localizedDescription
    }
}

@MainActor
private final class TodoReminderRow: NSView {
    private let reminder: ReminderSnapshot
    private let onComplete: () -> Void
    private let onEdit: () -> Void

    init(reminder: ReminderSnapshot, onComplete: @escaping () -> Void, onEdit: @escaping () -> Void) {
        self.reminder = reminder
        self.onComplete = onComplete
        self.onEdit = onEdit
        super.init(frame: .zero)
        setup()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    private func setup() {
        wantsLayer = true
        layer?.cornerRadius = 8
        layer?.backgroundColor = NSColor.windowBackgroundColor.withAlphaComponent(0.72).cgColor
        translatesAutoresizingMaskIntoConstraints = false
        heightAnchor.constraint(equalToConstant: 54).isActive = true

        let completeButton = NSButton()
        completeButton.bezelStyle = .regularSquare
        completeButton.isBordered = false
        completeButton.image = NSImage(systemSymbolName: "circle", accessibilityDescription: "Complete")
        completeButton.target = self
        completeButton.action = #selector(completeTapped)
        completeButton.translatesAutoresizingMaskIntoConstraints = false

        let editButton = NSButton()
        editButton.bezelStyle = .regularSquare
        editButton.isBordered = false
        editButton.image = NSImage(systemSymbolName: "pencil", accessibilityDescription: "Edit")
        editButton.target = self
        editButton.action = #selector(editTapped)
        editButton.translatesAutoresizingMaskIntoConstraints = false

        let title = NSTextField(labelWithString: reminder.title)
        title.font = .systemFont(ofSize: 13, weight: .medium)
        title.lineBreakMode = .byTruncatingTail
        title.translatesAutoresizingMaskIntoConstraints = false

        let meta = NSTextField(labelWithString: metadata())
        meta.font = .systemFont(ofSize: 11)
        meta.textColor = .secondaryLabelColor
        meta.lineBreakMode = .byTruncatingTail
        meta.translatesAutoresizingMaskIntoConstraints = false

        addSubview(completeButton)
        addSubview(editButton)
        addSubview(title)
        addSubview(meta)

        NSLayoutConstraint.activate([
            completeButton.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            completeButton.centerYAnchor.constraint(equalTo: centerYAnchor),
            completeButton.widthAnchor.constraint(equalToConstant: 24),
            completeButton.heightAnchor.constraint(equalToConstant: 24),

            title.leadingAnchor.constraint(equalTo: completeButton.trailingAnchor, constant: 8),
            title.topAnchor.constraint(equalTo: topAnchor, constant: 9),
            title.trailingAnchor.constraint(equalTo: editButton.leadingAnchor, constant: -8),

            editButton.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            editButton.centerYAnchor.constraint(equalTo: centerYAnchor),
            editButton.widthAnchor.constraint(equalToConstant: 24),
            editButton.heightAnchor.constraint(equalToConstant: 24),

            meta.leadingAnchor.constraint(equalTo: title.leadingAnchor),
            meta.topAnchor.constraint(equalTo: title.bottomAnchor, constant: 2),
            meta.trailingAnchor.constraint(equalTo: title.trailingAnchor)
        ])
    }

    @objc private func completeTapped() {
        onComplete()
    }

    @objc private func editTapped() {
        onEdit()
    }

    private func metadata() -> String {
        reminder.dueDate.map { TodoModule.formatDue($0) } ?? "No due date"
    }
}

@MainActor
private final class TodoCompletedRow: NSView {
    private let onEdit: () -> Void

    init(reminder: ReminderSnapshot, onEdit: @escaping () -> Void) {
        self.onEdit = onEdit
        super.init(frame: .zero)
        setup(reminder: reminder)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    private func setup(reminder: ReminderSnapshot) {
        wantsLayer = true
        layer?.cornerRadius = 8
        layer?.backgroundColor = NSColor.windowBackgroundColor.withAlphaComponent(0.45).cgColor
        translatesAutoresizingMaskIntoConstraints = false
        heightAnchor.constraint(equalToConstant: 48).isActive = true

        let checkmark = NSImageView()
        checkmark.image = NSImage(systemSymbolName: "checkmark.circle.fill", accessibilityDescription: "Completed")
        checkmark.contentTintColor = .tertiaryLabelColor
        checkmark.translatesAutoresizingMaskIntoConstraints = false

        let title = NSTextField(labelWithString: reminder.title)
        title.font = .systemFont(ofSize: 13, weight: .medium)
        title.textColor = .secondaryLabelColor
        title.lineBreakMode = .byTruncatingTail
        title.translatesAutoresizingMaskIntoConstraints = false

        let bodyButton = NSButton(title: "", target: self, action: #selector(editTapped))
        bodyButton.bezelStyle = .regularSquare
        bodyButton.isBordered = false
        bodyButton.translatesAutoresizingMaskIntoConstraints = false

        addSubview(checkmark)
        addSubview(title)
        addSubview(bodyButton)

        NSLayoutConstraint.activate([
            checkmark.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            checkmark.centerYAnchor.constraint(equalTo: centerYAnchor),
            checkmark.widthAnchor.constraint(equalToConstant: 22),
            checkmark.heightAnchor.constraint(equalToConstant: 22),

            title.leadingAnchor.constraint(equalTo: checkmark.trailingAnchor, constant: 10),
            title.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            title.centerYAnchor.constraint(equalTo: centerYAnchor),

            bodyButton.leadingAnchor.constraint(equalTo: title.leadingAnchor),
            bodyButton.trailingAnchor.constraint(equalTo: title.trailingAnchor),
            bodyButton.topAnchor.constraint(equalTo: topAnchor),
            bodyButton.bottomAnchor.constraint(equalTo: bottomAnchor)
        ])
    }

    @objc private func editTapped() {
        onEdit()
    }
}
