import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
final class AutoworkflowDetailView: ModuleDetailView {
    let moduleTitle = "Auto Workflow"
    let detailView: NSView

    // MARK: - Dependencies (captured at init to avoid async access)
    private let module: AutoworkflowModule
    private let service: any AutoworkflowServiceProtocol
    private var config: AutoworkflowConfig
    private let onOpenSettings: (() -> Void)?

    // MARK: - State
    private enum UIState {
        case checking
        case unavailable(reason: String)
        case idle
        case running(taskID: String, pid: Int32)
        case stopped(detail: String? = nil)
        case completed
        case failed_terminal(reason: String)
        case error(message: String)
    }
    private var state: UIState = .checking
    private var pollTask: Task<Void, Never>?
    private var lastSnapshot: AutoworkflowTaskSnapshot?
    private var tasks: [AutoworkflowTaskItem] = []
    private var selectedTaskID: String?
    private var isStarting = false

    // MARK: - UI Elements
    private let statusBadge = NSView()
    private let statusLabel = NSTextField(labelWithString: "Checking...")
    private let detailLabel = NSTextField(wrappingLabelWithString: "")
    private let openSettingsButton = NSButton()
    private let goalField = NSTextField()
    private let repoField = NSTextField()
    private let taskIDField = NSTextField()
    private let testCommandField = NSTextField()
    private let startButton = NSButton()
    private let stopButton = NSButton()
    private let resumeButton = NSButton()
    private let refreshButton = NSButton()
    private let logTextView = NSTextView()
    private let logScrollView = NSScrollView()
    private let taskListStack = NSStackView()
    private let taskListScroll = NSScrollView()
    private let contentStack = NSStackView()

    // MARK: - Font & sizing
    private static let titleFont = NSFont.systemFont(ofSize: 13, weight: .semibold)
    private static let bodyFont = NSFont.systemFont(ofSize: 12)
    private static let monoFont = NSFont.monospacedSystemFont(ofSize: 11, weight: .regular)
    private static let sectionFont = NSFont.systemFont(ofSize: 11, weight: .semibold)

    init(module: AutoworkflowModule,
         service: any AutoworkflowServiceProtocol,
         config: AutoworkflowConfig,
         onOpenSettings: (() -> Void)? = nil) {
        self.module = module
        self.service = service
        self.config = config
        self.onOpenSettings = onOpenSettings

        let container = NSView()
        container.translatesAutoresizingMaskIntoConstraints = false
        self.detailView = container
        setupUI(container: container)
    }

    func activate() {
        Task {
            config = await module.getConfig()
            await checkHealthAndLoad()
        }
    }

    func deactivate() {
        stopPolling()
    }

    func prepareForLauncherHide() async {
        stopPolling()
    }

    func handleKeyDown(_ event: NSEvent) -> Bool {
        guard event.keyCode == 36 else { return false }
        guard let responder = detailView.window?.firstResponder,
              responder === goalField || responder === repoField else { return false }
        let goal = goalField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let repo = repoField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !goal.isEmpty, !repo.isEmpty else { return false }
        startTapped()
        return true
    }

    // MARK: - Setup

    private func setupUI(container: NSView) {
        setupStatusHeader()
        setupConfigForm()
        setupControls()
        setupTaskList()
        setupLogViewer()

        contentStack.orientation = .vertical
        contentStack.spacing = 12
        contentStack.distribution = .fill
        contentStack.translatesAutoresizingMaskIntoConstraints = false
        contentStack.edgeInsets = NSEdgeInsets(top: 16, left: 20, bottom: 16, right: 20)

        container.addSubview(contentStack)
        NSLayoutConstraint.activate([
            contentStack.topAnchor.constraint(equalTo: container.topAnchor),
            contentStack.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            contentStack.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            contentStack.bottomAnchor.constraint(lessThanOrEqualTo: container.bottomAnchor)
        ])
    }

    private func setupStatusHeader() {
        statusBadge.wantsLayer = true
        statusBadge.layer?.cornerRadius = 5
        statusBadge.translatesAutoresizingMaskIntoConstraints = false
        statusBadge.widthAnchor.constraint(equalToConstant: 10).isActive = true
        statusBadge.heightAnchor.constraint(equalToConstant: 10).isActive = true

        statusLabel.font = Self.titleFont
        statusLabel.lineBreakMode = .byTruncatingTail

        detailLabel.font = Self.bodyFont
        detailLabel.textColor = .secondaryLabelColor
        detailLabel.lineBreakMode = .byWordWrapping
        detailLabel.preferredMaxLayoutWidth = 500

        let headerStack = NSStackView(views: [statusBadge, statusLabel])
        headerStack.spacing = 8
        headerStack.alignment = .centerY

        openSettingsButton.title = "Configure in Settings..."
        openSettingsButton.bezelStyle = .rounded
        openSettingsButton.font = Self.bodyFont
        openSettingsButton.target = self
        openSettingsButton.action = #selector(openSettingsTapped)
        openSettingsButton.isHidden = true

        let headerGroup = sectionCard([headerStack, detailLabel, openSettingsButton], header: "STATUS")
        contentStack.addArrangedSubview(headerGroup)
    }

    private func setupConfigForm() {
        goalField.placeholderString = "Task goal (e.g. Add error handling to API)"
        goalField.font = Self.bodyFont
        goalField.lineBreakMode = .byWordWrapping

        repoField.placeholderString = "Target repo path (e.g. /Users/you/MyProject)"
        repoField.font = Self.bodyFont

        taskIDField.placeholderString = "Task ID (optional — auto-generated)"
        taskIDField.font = Self.bodyFont

        testCommandField.placeholderString = "Test command (optional, e.g. swift test)"
        testCommandField.font = Self.bodyFont

        let fields = labeledField("Goal", goalField)
        let repoFields = labeledField("Repo", repoField)
        let taskFields = labeledField("Task ID", taskIDField)
        let testFields = labeledField("Tests", testCommandField)

        let formGroup = sectionCard([fields, repoFields, taskFields, testFields], header: "NEW WORKFLOW")
        contentStack.addArrangedSubview(formGroup)
    }

    private func setupControls() {
        startButton.title = "Start Workflow"
        startButton.bezelStyle = .rounded
        startButton.font = Self.bodyFont
        startButton.target = self
        startButton.action = #selector(startTapped)

        stopButton.title = "Stop"
        stopButton.bezelStyle = .rounded
        stopButton.font = Self.bodyFont
        stopButton.target = self
        stopButton.action = #selector(stopTapped)
        stopButton.isHidden = true

        resumeButton.title = "Resume"
        resumeButton.bezelStyle = .rounded
        resumeButton.font = Self.bodyFont
        resumeButton.target = self
        resumeButton.action = #selector(resumeTapped)
        resumeButton.isHidden = true

        refreshButton.title = "Refresh"
        refreshButton.bezelStyle = .rounded
        refreshButton.font = Self.bodyFont
        refreshButton.target = self
        refreshButton.action = #selector(refreshTapped)

        let btnStack = NSStackView(views: [startButton, stopButton, resumeButton, refreshButton])
        btnStack.spacing = 8

        let ctrlGroup = sectionCard([btnStack], header: "CONTROLS")
        contentStack.addArrangedSubview(ctrlGroup)
    }

    private func setupLogViewer() {
        logTextView.isEditable = false
        logTextView.isSelectable = true
        logTextView.font = Self.monoFont
        logTextView.textColor = .textColor
        logTextView.backgroundColor = .textBackgroundColor
        logTextView.string = "Log output will appear here..."

        logScrollView.documentView = logTextView
        logScrollView.hasVerticalScroller = true
        logScrollView.translatesAutoresizingMaskIntoConstraints = false
        logScrollView.heightAnchor.constraint(greaterThanOrEqualToConstant: 120).isActive = true

        let logGroup = sectionCard([logScrollView], header: "LOG")
        contentStack.addArrangedSubview(logGroup)
    }

    private func setupTaskList() {
        taskListStack.orientation = .vertical
        taskListStack.spacing = 4
        taskListStack.distribution = .fillEqually

        taskListScroll.documentView = taskListStack
        taskListScroll.hasVerticalScroller = true
        taskListScroll.translatesAutoresizingMaskIntoConstraints = false
        taskListScroll.heightAnchor.constraint(lessThanOrEqualToConstant: 150).isActive = true

        let tasksGroup = sectionCard([taskListScroll], header: "TASKS")
        contentStack.addArrangedSubview(tasksGroup)
    }

    // MARK: - Helpers

    private func sectionCard(_ views: [NSView], header: String) -> NSView {
        let headerLabel = NSTextField(labelWithString: header.uppercased())
        headerLabel.font = Self.sectionFont
        headerLabel.textColor = .secondaryLabelColor

        let stack = NSStackView()
        stack.orientation = .vertical
        stack.spacing = 6
        stack.setContentHuggingPriority(.defaultHigh, for: .vertical)

        stack.addArrangedSubview(headerLabel)
        for v in views { stack.addArrangedSubview(v) }

        let card = NSView()
        card.wantsLayer = true
        card.layer?.backgroundColor = NSColor.controlBackgroundColor.cgColor
        card.layer?.cornerRadius = 8
        card.layer?.borderWidth = 0.5
        card.layer?.borderColor = NSColor.separatorColor.cgColor

        stack.translatesAutoresizingMaskIntoConstraints = false
        card.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: card.topAnchor, constant: 10),
            stack.leadingAnchor.constraint(equalTo: card.leadingAnchor, constant: 12),
            stack.trailingAnchor.constraint(equalTo: card.trailingAnchor, constant: -12),
            stack.bottomAnchor.constraint(equalTo: card.bottomAnchor, constant: -10)
        ])
        return card
    }

    private func labeledField(_ label: String, _ field: NSControl) -> NSView {
        let lbl = NSTextField(labelWithString: label)
        lbl.font = NSFont.systemFont(ofSize: 11, weight: .medium)
        lbl.textColor = .secondaryLabelColor
        lbl.widthAnchor.constraint(equalToConstant: 70).isActive = true

        let stack = NSStackView(views: [lbl, field])
        stack.spacing = 8
        stack.alignment = .centerY
        return stack
    }

    // MARK: - Actions

    @objc private func startTapped() {
        let goal = goalField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let repo = repoField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)

        guard !goal.isEmpty, !repo.isEmpty else {
            updateUI(state: .error(message: "Goal and Repo are required"))
            return
        }
        if isStarting { return }
        isStarting = true
        startButton.isEnabled = false
        startButton.title = "Starting..."

        let taskID = taskIDField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let testCmd = testCommandField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let testArgs: [String]? = testCmd.isEmpty ? nil : autoworkflowShellSplitArguments(testCmd)

        Task {
            // Step 1: Run doctor
            let doctorResult = await service.runDoctor(repo: repo, config: config)
            if case .failure(let err) = doctorResult {
                await MainActor.run {
                    updateUI(state: .error(message: "Preflight failed: \(err.localizedDescription)"))
                    finishStarting()
                }
                return
            }

            // Step 2: Init
            let initResult = await service.initializeTask(
                goal: goal,
                repo: repo,
                taskID: taskID.isEmpty ? nil : taskID,
                testCommand: testArgs,
                config: config
            )
            guard case .success(let resolvedID) = initResult else {
                if case .failure(let err) = initResult {
                    await MainActor.run {
                        updateUI(state: .error(message: "Init failed: \(err.localizedDescription)"))
                        finishStarting()
                    }
                }
                return
            }

            // Step 3: Start
            let startResult = await service.startTask(taskID: resolvedID, config: config)
            guard case .success(let pid) = startResult else {
                if case .failure(let err) = startResult {
                    await MainActor.run {
                        updateUI(state: .error(message: "Start failed: \(err.localizedDescription)"))
                        finishStarting()
                    }
                }
                return
            }

            await MainActor.run {
                selectedTaskID = resolvedID
                updateTaskList()
                updateUI(state: .running(taskID: resolvedID, pid: pid))
                finishStarting()
                startPolling(taskID: resolvedID)
                updateLog(taskID: resolvedID)
            }
        }
    }

    @objc private func stopTapped() {
        let taskID: String?
        if case .running(let runningTaskID, _) = state {
            taskID = runningTaskID
        } else if lastSnapshot?.isRunning == true {
            taskID = lastSnapshot?.taskID
        } else {
            taskID = nil
        }

        guard let taskID else { return }

        Task {
            let result = await service.stopTask(taskID: taskID, config: config)
            await MainActor.run {
                switch result {
                case .success:
                    updateUI(state: .stopped())
                    stopPolling()
                case .failure(let err):
                    updateUI(state: .error(message: "Stop failed: \(err.localizedDescription)"))
                }
            }
        }
    }

    @objc private func resumeTapped() {
        guard let taskID = lastSnapshot?.taskID else { return }

        resumeButton.isEnabled = false
        Task {
            let result = await service.resumeTask(taskID: taskID, config: config)
            await MainActor.run {
                resumeButton.isEnabled = true
                switch result {
                case .success(let pid):
                    updateUI(state: .running(taskID: taskID, pid: pid))
                    startPolling(taskID: taskID)
                    updateLog(taskID: taskID)
                case .failure(let err):
                    updateUI(state: .error(message: "Resume failed: \(err.localizedDescription)"))
                }
            }
        }
    }

    @objc private func refreshTapped() {
        Task { await checkHealthAndLoad() }
    }

    @objc private func openSettingsTapped() {
        onOpenSettings?()
    }

    @objc private func taskRowTapped(_ sender: NSButton) {
        guard let taskID = sender.identifier?.rawValue else { return }
        selectedTaskID = taskID
        updateTaskList()
        selectTask(taskID: taskID)
    }

    private func selectTask(taskID: String) {
        Task {
            let result = await service.taskStatus(taskID: taskID, config: config)
            await MainActor.run {
                guard case .success(let snapshot) = result else { return }
                displaySelectedTask(snapshot)
            }
        }
    }

    private func displaySelectedTask(_ snapshot: AutoworkflowTaskSnapshot) {
        lastSnapshot = snapshot

        if snapshot.isRunning || snapshot.status == .running {
            updateUI(state: .running(
                taskID: snapshot.taskID,
                pid: snapshot.runnerPID.map(Int32.init) ?? 0
            ))
            updateSnapshotDisplay(snapshot)
            startPolling(taskID: snapshot.taskID)
        } else {
            stopPolling()
            switch snapshot.status {
            case .done:
                updateUI(state: .completed)
            case .failed:
                updateUI(state: .failed_terminal(reason: failureReason(from: snapshot)))
            case .interrupted, .waitingManualReview:
                updateUI(state: .stopped(detail: snapshot.status.displayName))
            case .stopped:
                updateUI(state: .stopped())
            default:
                setStatus(color: .systemGray, text: snapshot.status.displayName)
                startButton.isEnabled = true
                stopButton.isHidden = true
                resumeButton.isHidden = snapshot.nextAction != "resume"
            }
            updateSnapshotDisplay(snapshot)
        }
        updateLog(taskID: snapshot.taskID)
    }

    // MARK: - Health check & load

    private func checkHealthAndLoad() async {
        updateUI(state: .checking)

        let sourceExists = await service.sourceExists(at: config.autoworkflowPath)
        if !sourceExists {
            updateUI(state: .unavailable(reason: "autoworkflow source not found at \(config.autoworkflowPath)"))
            return
        }

        let health = await service.healthCheck()
        switch health {
        case .success(true):
            break
        case .success(false):
            updateUI(state: .unavailable(reason: "cc-loop not found on PATH. Run: pip install -e \(config.autoworkflowPath)"))
            return
        case .failure(let err):
            updateUI(state: .unavailable(reason: err.localizedDescription))
            return
        }

        // Load tasks
        let listResult = await service.listTasks(config: config)
        await MainActor.run {
            if case .success(let loaded) = listResult {
                tasks = loaded
                updateTaskList()
            }

            // If a task is running, poll it
            if let runningTask = tasks.first(where: { $0.status == "running" }) {
                selectedTaskID = runningTask.taskID
                updateTaskList()
                selectTask(taskID: runningTask.taskID)
            } else {
                updateUI(state: .idle)
            }
        }
    }

    // MARK: - Polling

    private func startPolling(taskID: String) {
        stopPolling()
        pollTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self else { break }
                let result = await service.taskStatus(taskID: taskID, config: config)
                await MainActor.run {
                    if case .success(let snapshot) = result {
                        self.lastSnapshot = snapshot
                        self.updateSnapshotDisplay(snapshot)
                        self.updateLog(taskID: taskID)
                        if snapshot.isRunning || snapshot.status == .running {
                            self.updateUI(state: .running(
                                taskID: snapshot.taskID,
                                pid: snapshot.runnerPID.map(Int32.init) ?? 0
                            ))
                            self.updateSnapshotDisplay(snapshot)
                        } else if snapshot.status.isTerminal
                            || (!snapshot.status.isTerminal && !snapshot.isRunning) {
                            self.stopPolling()
                            if snapshot.status.isTerminal {
                                switch snapshot.status {
                                case .done:
                                    self.updateUI(state: .completed)
                                case .failed:
                                    self.updateUI(state: .failed_terminal(
                                        reason: self.failureReason(from: snapshot)
                                    ))
                                default:
                                    self.updateUI(state: .stopped())
                                }
                            } else if !snapshot.isRunning {
                                switch snapshot.status {
                                case .interrupted:
                                    self.updateUI(state: .stopped(detail: snapshot.status.displayName))
                                case .waitingManualReview:
                                    self.updateUI(state: .stopped(detail: "Review needed"))
                                default:
                                    self.updateUI(state: .stopped(detail: snapshot.status.displayName))
                                }
                            }
                        }
                    }
                }
                try? await Task.sleep(for: .seconds(2))
            }
        }
    }

    private func stopPolling() {
        pollTask?.cancel()
        pollTask = nil
    }

    private func finishStarting() {
        isStarting = false
        startButton.title = "Start Workflow"
    }

    private func updateLog(taskID: String) {
        Task {
            let logResult = await service.readLog(taskID: taskID, config: config, maxLines: 200)
            await MainActor.run {
                if case .success(let log) = logResult {
                    logTextView.string = log.isEmpty ? "(no output yet)" : log
                    // Scroll to bottom
                    logTextView.scrollToEndOfDocument(nil)
                }
            }
        }
    }

    // MARK: - UI updates

    private func updateUI(state newState: UIState) {
        state = newState
        switch newState {
        case .checking:
            setStatus(color: .systemGray, text: "Checking...")
            detailLabel.stringValue = "Verifying autoworkflow availability"
            openSettingsButton.isHidden = true
            startButton.isEnabled = false
            stopButton.isHidden = true
            resumeButton.isHidden = true
        case .unavailable(let reason):
            setStatus(color: .systemRed, text: "Unavailable")
            detailLabel.stringValue = reason
            openSettingsButton.isHidden = onOpenSettings == nil
            startButton.isEnabled = false
            stopButton.isHidden = true
            resumeButton.isHidden = true
        case .idle:
            setStatus(color: .systemGreen, text: "Ready")
            detailLabel.stringValue = "Configure and start a new workflow"
            openSettingsButton.isHidden = true
            startButton.isEnabled = true
            stopButton.isHidden = true
            resumeButton.isHidden = true
        case .running(let taskID, let pid):
            setStatus(color: .systemBlue, text: "Running")
            detailLabel.stringValue = "Task: \(taskID) · PID: \(pid)"
            openSettingsButton.isHidden = true
            startButton.isEnabled = false
            stopButton.isHidden = false
            resumeButton.isHidden = true
        case .stopped(let detail):
            setStatus(color: .systemGray, text: "Stopped")
            detailLabel.stringValue = detail ?? "Workflow has stopped"
            openSettingsButton.isHidden = true
            startButton.isEnabled = true
            stopButton.isHidden = true
            resumeButton.isHidden = lastSnapshot?.nextAction != "resume"
        case .completed:
            setStatus(color: .systemGreen, text: "Completed")
            openSettingsButton.isHidden = true
            startButton.isEnabled = true
            stopButton.isHidden = true
            resumeButton.isHidden = true
        case .failed_terminal:
            setStatus(color: .systemRed, text: "Failed")
            openSettingsButton.isHidden = true
            startButton.isEnabled = true
            stopButton.isHidden = true
            resumeButton.isHidden = true
        case .error(let message):
            setStatus(color: .systemRed, text: "Error")
            detailLabel.stringValue = errorDetailLabel(message: message)
            openSettingsButton.isHidden = true
            startButton.isEnabled = true
            stopButton.isHidden = true
            resumeButton.isHidden = true
        }
    }

    private func setStatus(color: NSColor, text: String) {
        statusBadge.layer?.backgroundColor = color.cgColor
        statusLabel.stringValue = text
    }

    private func updateSnapshotDisplay(_ snapshot: AutoworkflowTaskSnapshot) {
        var parts: [String] = [
            "Task: \(snapshot.taskID)",
            "Status: \(snapshot.status.displayName)",
            "Iteration: \(snapshot.iteration)",
            "Phase: \(snapshot.attempt.phase.isEmpty ? "—" : snapshot.attempt.phase)",
            "Next: \(snapshot.nextAction)"
        ]
        if let runnerPID = snapshot.runnerPID {
            parts.append("PID: \(runnerPID)")
        }
        if snapshot.attempt.decision.isEmpty == false {
            parts.append("Decision: \(snapshot.attempt.decision)")
        }
        if snapshot.attempt.testStatus.isEmpty == false {
            parts.append("Tests: \(snapshot.attempt.testStatus)")
        }
        if snapshot.failure.failureType.isEmpty == false {
            parts.append("Failure: \(snapshot.failure.failureType)")
        }
        detailLabel.stringValue = parts.joined(separator: " · ")
        resumeButton.isHidden = snapshot.nextAction != "resume"
    }

    private func failureReason(from snapshot: AutoworkflowTaskSnapshot) -> String {
        if snapshot.failure.stopReason.isEmpty == false { return snapshot.failure.stopReason }
        if snapshot.failure.failureType.isEmpty == false { return snapshot.failure.failureType }
        return "Workflow failed"
    }

    private func errorDetailLabel(message: String) -> String {
        guard let snapshot = lastSnapshot else { return message }
        return "Task: \(snapshot.taskID) · Iteration: \(snapshot.iteration) · \(message)"
    }

    private func updateTaskList() {
        taskListStack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        if tasks.isEmpty {
            let emptyLabel = NSTextField(labelWithString: "No tasks yet")
            emptyLabel.font = Self.bodyFont
            emptyLabel.textColor = .tertiaryLabelColor
            taskListStack.addArrangedSubview(emptyLabel)
            return
        }
        for task in tasks.prefix(10) {
            let icon = statusIcon(task.status)
            let button = NSButton()
            button.title = "\(icon) [\(task.status)] \(task.goal.prefix(40)) — \(task.taskID)"
            button.bezelStyle = .inline
            button.isBordered = false
            button.alignment = .left
            button.font = NSFont.systemFont(ofSize: 11)
            button.target = self
            button.action = #selector(taskRowTapped(_:))
            button.identifier = NSUserInterfaceItemIdentifier(task.taskID)
            if task.taskID == selectedTaskID {
                button.wantsLayer = true
                button.layer?.backgroundColor = NSColor.selectedContentBackgroundColor
                    .withAlphaComponent(0.25).cgColor
                button.layer?.cornerRadius = 4
            }
            taskListStack.addArrangedSubview(button)
        }
    }

    private func statusIcon(_ status: String) -> String {
        switch status {
        case "running": "●"
        case "done": "✓"
        case "failed": "✗"
        case "stopped": "■"
        case "initialized": "○"
        default: "○"
        }
    }

}
