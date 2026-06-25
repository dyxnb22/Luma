import AppKit
import LumaCore
import LumaModules
import LumaServices

@MainActor
final class TodoDetailView: NSObject, ModuleDetailView {
    private enum Tab: Int, CaseIterable { case today = 0, inbox = 1, upcoming = 2, completed = 3 }

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
    private var statusResetTask: Task<Void, Never>?
    private var editSheetReminder: ReminderSnapshot?
    private var pendingTransientStatus: String?

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
        statusResetTask?.cancel()
        statusResetTask = nil
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
            case "2": selectTab(.inbox); return true
            case "3": selectTab(.upcoming); return true
            case "4": selectTab(.completed); return true
            default: break
            }
        }
        return false
    }

    private func setup(chrome: BaseDetailContainer) {
        let topBar = NSView()
        topBar.translatesAutoresizingMaskIntoConstraints = false

        inputField.placeholderString = "添加待办，可附 9:00 / +30m / 明天 9点"
        inputField.font = TypographyTokens.body
        inputField.target = self
        inputField.action = #selector(addTodo)
        inputField.translatesAutoresizingMaskIntoConstraints = false

        addButton.title = "Add"
        GeekUIKit.stylePrimaryButton(addButton)
        addButton.target = self
        addButton.action = #selector(addTodo)
        addButton.translatesAutoresizingMaskIntoConstraints = false

        GeekUIKit.styleIconToolbarButton(refreshButton, symbol: "arrow.clockwise", tooltip: "Refresh")
        refreshButton.target = self
        refreshButton.action = #selector(refreshTapped)

        GeekUIKit.styleIconToolbarButton(openRemindersButton, symbol: "arrow.up.forward.app", tooltip: "Open Reminders")
        openRemindersButton.target = self
        openRemindersButton.action = #selector(openReminders)
        openRemindersButton.translatesAutoresizingMaskIntoConstraints = false
        refreshButton.translatesAutoresizingMaskIntoConstraints = false

        tabControl.segmentCount = 4
        tabControl.setLabel("Today", forSegment: 0)
        tabControl.setLabel("Inbox", forSegment: 1)
        tabControl.setLabel("Upcoming", forSegment: 2)
        tabControl.setLabel("Done", forSegment: 3)
        tabControl.selectedSegment = 0
        tabControl.target = self
        tabControl.action = #selector(tabChanged)
        tabControl.translatesAutoresizingMaskIntoConstraints = false

        GeekUIKit.configureStatusLabel(statusLabel)
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

        chrome.setToolbar(topBar, height: LauncherChromeTokens.detailToolbarTallHeight)
        chrome.setFooter(statusLabel, height: LauncherChromeTokens.detailFooterHeight)
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

    private func listKind(for tab: Tab) -> TodoListKind {
        switch tab {
        case .today: return .today
        case .inbox: return .inbox
        case .upcoming: return .upcoming
        case .completed: return .completed
        }
    }

    private func emptyMessage(for tab: Tab) -> String {
        switch tab {
        case .today: return "Press ⌘+T to add a reminder…"
        case .inbox: return "No inbox reminders. Capture without a date."
        case .upcoming: return "No upcoming reminders."
        case .completed: return "No completed reminders this week."
        }
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


    @objc private func addTodo() {
        let raw = inputField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !raw.isEmpty else {
            detailView.window?.makeFirstResponder(inputField)
            return
        }
        let parsed = TodoTimeParser.parse(raw)
        setControlsEnabled(false)
        statusLabel.stringValue = "Adding..."
        Task { [weak self] in
            guard let self else { return }
            do {
                _ = try await module.createReminder(from: raw)
                let message = TodoModule.captureStatusMessage(for: parsed, now: Date(), calendar: .current)
                await MainActor.run {
                    inputField.stringValue = ""
                    setControlsEnabled(true)
                    pendingTransientStatus = message
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

    private func showTransientStatus(_ message: String, revertAfter seconds: TimeInterval = 2.5) {
        statusResetTask?.cancel()
        statusLabel.stringValue = message
        statusResetTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(seconds))
            guard !Task.isCancelled, let self else { return }
            await MainActor.run {
                if statusLabel.stringValue == message {
                    statusLabel.stringValue = ""
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
        let kind = listKind(for: tab)
        refreshTask = Task { [weak self] in
            guard let self else { return }
            do {
                let authorization = await module.authorizationStatus()
                if authorization == .denied {
                    await MainActor.run { showPermissionState() }
                    return
                }
                let items = try await module.reminders(kind: kind, limit: 30)
                guard !Task.isCancelled else { return }
                await MainActor.run {
                    render(items: items, tab: tab)
                }
            } catch RemindersServiceError.accessDenied {
                await MainActor.run { showPermissionState() }
            } catch {
                guard !Task.isCancelled else { return }
                await MainActor.run {
                    clearRows()
                    statusLabel.stringValue = userMessage(for: error)
                }
            }
        }
    }

    private func updateTabLabel(tab: Tab, count: Int) {
        let base: String
        switch tab {
        case .today: base = "Today"
        case .inbox: base = "Inbox"
        case .upcoming: base = "Upcoming"
        case .completed: base = "Done"
        }
        tabControl.setLabel(count > 0 ? "\(base) (\(count))" : base, forSegment: tab.rawValue)
    }

    private func render(items: [ReminderSnapshot], tab: Tab) {
        clearRows()
        updateTabLabel(tab: tab, count: items.count)
        if let pending = pendingTransientStatus {
            pendingTransientStatus = nil
            showTransientStatus(pending)
        } else if tab == .completed {
            statusLabel.stringValue = "\(items.count) completed"
        } else {
            statusLabel.stringValue = "\(items.count) reminder\(items.count == 1 ? "" : "s")"
        }
        if items.isEmpty {
            addPlaceholderRow(emptyMessage(for: tab))
            return
        }
        let isCompleted = tab == .completed
        for item in items {
            let row = TodoReminderRow(
                reminder: item,
                isCompleted: isCompleted,
                onPrimary: { [weak self] in
                    if isCompleted {
                        self?.uncomplete(item)
                    } else {
                        self?.complete(item)
                    }
                },
                onEdit: { [weak self] in self?.showEditPrompt(for: item) },
                onScheduleToday: { [weak self] in self?.scheduleToday(item) },
                onScheduleTomorrow: { [weak self] in self?.scheduleTomorrow(item) },
                onClearDate: { [weak self] in self?.clearDate(item) }
            )
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

    private func uncomplete(_ reminder: ReminderSnapshot) {
        Task { [weak self] in
            guard let self else { return }
            do {
                try await module.uncompleteReminder(id: reminder.id)
                await MainActor.run { refresh() }
            } catch {
                await MainActor.run { statusLabel.stringValue = userMessage(for: error) }
            }
        }
    }

    private func scheduleToday(_ reminder: ReminderSnapshot) {
        guard let due = defaultDueDate(dayOffset: 0) else { return }
        applySchedule(reminder, dueDate: due, message: "Scheduled for today")
    }

    private func scheduleTomorrow(_ reminder: ReminderSnapshot) {
        guard let due = defaultDueDate(dayOffset: 1) else { return }
        applySchedule(reminder, dueDate: due, message: "Scheduled for tomorrow")
    }

    private func clearDate(_ reminder: ReminderSnapshot) {
        Task { [weak self] in
            guard let self else { return }
            do {
                _ = try await module.clearDueDate(id: reminder.id, title: reminder.title)
                await MainActor.run {
                    pendingTransientStatus = "Date cleared"
                    refresh()
                }
            } catch {
                await MainActor.run { statusLabel.stringValue = userMessage(for: error) }
            }
        }
    }

    private func applySchedule(_ reminder: ReminderSnapshot, dueDate: Date, message: String) {
        Task { [weak self] in
            guard let self else { return }
            do {
                _ = try await module.scheduleReminder(id: reminder.id, title: reminder.title, dueDate: dueDate)
                await MainActor.run {
                    pendingTransientStatus = message
                    refresh()
                }
            } catch {
                await MainActor.run { statusLabel.stringValue = userMessage(for: error) }
            }
        }
    }

    private func defaultDueDate(dayOffset: Int) -> Date? {
        let calendar = Calendar.current
        guard let baseDay = calendar.date(byAdding: .day, value: dayOffset, to: calendar.startOfDay(for: Date())) else {
            return nil
        }
        var components = calendar.dateComponents([.year, .month, .day], from: baseDay)
        components.hour = 9
        components.minute = 0
        return calendar.date(from: components)
    }

    private func showEditPrompt(for reminder: ReminderSnapshot) {
        guard let window = detailView.window else { return }
        editSheetReminder = reminder
        let alert = NSAlert()
        alert.messageText = "Edit Reminder"
        alert.informativeText = "Use the same time syntax as Add, e.g. tomorrow 9:00 or +30m."

        let container = NSView(frame: NSRect(x: 0, y: 0, width: 360, height: 56))
        let field = NSTextField(frame: NSRect(x: 0, y: 28, width: 360, height: 24))
        field.stringValue = editString(for: reminder)
        container.addSubview(field)

        let quickActions = NSStackView(frame: NSRect(x: 0, y: 0, width: 360, height: 24))
        quickActions.orientation = .horizontal
        quickActions.spacing = 8
        let todayButton = NSButton(title: "Today", target: self, action: #selector(editSheetToday))
        let tomorrowButton = NSButton(title: "Tomorrow", target: self, action: #selector(editSheetTomorrow))
        let clearButton = NSButton(title: "Clear Date", target: self, action: #selector(editSheetClearDate))
        clearButton.isEnabled = reminder.dueDate != nil
        for button in [todayButton, tomorrowButton, clearButton] {
            button.bezelStyle = .rounded
            button.setContentHuggingPriority(.defaultHigh, for: .horizontal)
            quickActions.addArrangedSubview(button)
        }
        container.addSubview(quickActions)

        alert.accessoryView = container
        alert.addButton(withTitle: "Save")
        alert.addButton(withTitle: "Cancel")

        alert.beginSheetModal(for: window) { [weak self] response in
            guard let self else { return }
            defer { editSheetReminder = nil }
            if response == .alertFirstButtonReturn {
                let raw = field.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
                guard !raw.isEmpty else { return }
                Task { await self.update(reminder, rawTitle: raw) }
            }
        }
    }

    @objc private func editSheetToday() {
        dismissEditSheet { [weak self] reminder in
            self?.scheduleToday(reminder)
        }
    }

    @objc private func editSheetTomorrow() {
        dismissEditSheet { [weak self] reminder in
            self?.scheduleTomorrow(reminder)
        }
    }

    @objc private func editSheetClearDate() {
        dismissEditSheet { [weak self] reminder in
            self?.clearDate(reminder)
        }
    }

    private func dismissEditSheet(action: (ReminderSnapshot) -> Void) {
        guard let reminder = editSheetReminder, let window = detailView.window, let sheet = window.attachedSheet else {
            return
        }
        window.endSheet(sheet)
        editSheetReminder = nil
        action(reminder)
    }

    private func update(_ reminder: ReminderSnapshot, rawTitle: String) async {
        do {
            _ = try await module.updateReminder(
                id: reminder.id,
                rawTitle: rawTitle,
                existingDueDate: reminder.dueDate
            )
            await MainActor.run {
                pendingTransientStatus = "Updated"
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
    private let isCompleted: Bool
    private let onPrimary: () -> Void
    private let onEdit: () -> Void
    private let onScheduleToday: () -> Void
    private let onScheduleTomorrow: () -> Void
    private let onClearDate: () -> Void

    init(
        reminder: ReminderSnapshot,
        isCompleted: Bool,
        onPrimary: @escaping () -> Void,
        onEdit: @escaping () -> Void,
        onScheduleToday: @escaping () -> Void,
        onScheduleTomorrow: @escaping () -> Void,
        onClearDate: @escaping () -> Void
    ) {
        self.reminder = reminder
        self.isCompleted = isCompleted
        self.onPrimary = onPrimary
        self.onEdit = onEdit
        self.onScheduleToday = onScheduleToday
        self.onScheduleTomorrow = onScheduleTomorrow
        self.onClearDate = onClearDate
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
        layer?.backgroundColor = NSColor.windowBackgroundColor
            .withAlphaComponent(isCompleted ? 0.45 : 0.72).cgColor
        translatesAutoresizingMaskIntoConstraints = false
        heightAnchor.constraint(equalToConstant: 54).isActive = true

        let primaryButton = NSButton()
        primaryButton.bezelStyle = .regularSquare
        primaryButton.isBordered = false
        let primarySymbol = isCompleted ? "arrow.uturn.backward.circle" : "circle"
        let primaryLabel = isCompleted ? "Undo" : "Complete"
        primaryButton.image = NSImage(systemSymbolName: primarySymbol, accessibilityDescription: primaryLabel)
        primaryButton.toolTip = primaryLabel
        primaryButton.target = self
        primaryButton.action = #selector(primaryTapped)
        primaryButton.translatesAutoresizingMaskIntoConstraints = false

        let editButton = NSButton()
        editButton.bezelStyle = .regularSquare
        editButton.isBordered = false
        editButton.image = NSImage(systemSymbolName: "pencil", accessibilityDescription: "Edit")
        editButton.target = self
        editButton.action = #selector(editTapped)
        editButton.translatesAutoresizingMaskIntoConstraints = false

        let scheduleButton = NSButton()
        scheduleButton.bezelStyle = .regularSquare
        scheduleButton.isBordered = false
        scheduleButton.image = NSImage(systemSymbolName: "calendar", accessibilityDescription: "Schedule")
        scheduleButton.target = self
        scheduleButton.action = #selector(scheduleTapped(_:))
        scheduleButton.translatesAutoresizingMaskIntoConstraints = false
        scheduleButton.isHidden = isCompleted

        let title = NSTextField(labelWithString: reminder.title)
        title.font = .systemFont(ofSize: 13, weight: .medium)
        title.textColor = isCompleted ? .secondaryLabelColor : .labelColor
        title.lineBreakMode = .byTruncatingTail
        title.translatesAutoresizingMaskIntoConstraints = false

        let meta = NSTextField(labelWithString: metadata())
        meta.font = .systemFont(ofSize: 11)
        meta.textColor = .secondaryLabelColor
        meta.lineBreakMode = .byTruncatingTail
        meta.translatesAutoresizingMaskIntoConstraints = false

        addSubview(primaryButton)
        addSubview(scheduleButton)
        addSubview(editButton)
        addSubview(title)
        addSubview(meta)

        NSLayoutConstraint.activate([
            primaryButton.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            primaryButton.centerYAnchor.constraint(equalTo: centerYAnchor),
            primaryButton.widthAnchor.constraint(equalToConstant: 24),
            primaryButton.heightAnchor.constraint(equalToConstant: 24),

            title.leadingAnchor.constraint(equalTo: primaryButton.trailingAnchor, constant: 8),
            title.topAnchor.constraint(equalTo: topAnchor, constant: 9),
            title.trailingAnchor.constraint(equalTo: scheduleButton.leadingAnchor, constant: -8),

            editButton.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            editButton.centerYAnchor.constraint(equalTo: centerYAnchor),
            editButton.widthAnchor.constraint(equalToConstant: 24),
            editButton.heightAnchor.constraint(equalToConstant: 24),

            scheduleButton.trailingAnchor.constraint(equalTo: editButton.leadingAnchor, constant: -4),
            scheduleButton.centerYAnchor.constraint(equalTo: centerYAnchor),
            scheduleButton.widthAnchor.constraint(equalToConstant: 24),
            scheduleButton.heightAnchor.constraint(equalToConstant: 24),

            meta.leadingAnchor.constraint(equalTo: title.leadingAnchor),
            meta.topAnchor.constraint(equalTo: title.bottomAnchor, constant: 2),
            meta.trailingAnchor.constraint(equalTo: title.trailingAnchor)
        ])
    }

    @objc private func primaryTapped() {
        onPrimary()
    }

    @objc private func editTapped() {
        onEdit()
    }

    @objc private func scheduleTapped(_ sender: NSButton) {
        let menu = NSMenu()
        menu.addItem(menuItem(title: "Today", action: #selector(scheduleTodayTapped)))
        menu.addItem(menuItem(title: "Tomorrow", action: #selector(scheduleTomorrowTapped)))
        if reminder.dueDate != nil {
            menu.addItem(.separator())
            menu.addItem(menuItem(title: "Clear Date", action: #selector(clearDateTapped)))
        }
        let point = NSPoint(x: 0, y: sender.bounds.height)
        menu.popUp(positioning: nil, at: point, in: sender)
    }

    private func menuItem(title: String, action: Selector) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
        item.target = self
        return item
    }

    @objc private func scheduleTodayTapped() { onScheduleToday() }
    @objc private func scheduleTomorrowTapped() { onScheduleTomorrow() }
    @objc private func clearDateTapped() { onClearDate() }

    private func metadata() -> String {
        if isCompleted {
            if let completed = reminder.completionDate {
                return "Completed \(TodoModule.formatDue(completed))"
            }
            return "Completed"
        }
        return reminder.dueDate.map { TodoModule.formatDue($0) } ?? "No due date"
    }
}
