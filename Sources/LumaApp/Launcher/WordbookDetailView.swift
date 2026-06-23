import AppKit
import LumaModules
import LumaServices

@MainActor
final class WordbookDetailView: ModuleDetailView {
    let moduleTitle = "Wordbook"
    let detailView: NSView
    var usesSharedTopBar: Bool { true }

    private enum SubState {
        case home
        case session
        case done
        case manage
    }

    private let store: WordbookStore
    private let planner: WordbookSessionPlanner
    private var speech: SpeechService
    private var subState: SubState = .home
    private var currentWord: WordEntry?
    private var revealed = false
    private var speechEnabled = true
    private var prefetchTask: Task<WordbookSessionPlanner.Card?, Never>?
    private var prefetchedCard: WordbookSessionPlanner.Card?
    private var loadTask: Task<Void, Never>?

    private let container = BaseDetailContainer()
    private let homeView = NSView()
    private let sessionView = NSView()
    private let doneView = NSView()
    private var manageView: WordbookManageView?
    private var manageWrongWordsOnly = false

    private let progressCard = WordbookProgressCardView()
    private let startButton = NSButton(title: "Start Session", target: nil, action: nil)
    private let newOnlyButton = NSButton(title: "New Words Only", target: nil, action: nil)
    private let manageButton = NSButton(title: "Manage", target: nil, action: nil)
    private let wrongWordsButton = NSButton(title: "Wrong Words", target: nil, action: nil)
    private let settingsGear = NSButton()

    private let termLabel = NSTextField(labelWithString: "")
    private let phoneticLabel = NSTextField(labelWithString: "")
    private let meaningLabel = NSTextField(wrappingLabelWithString: "")
    private let exampleLabel = NSTextField(wrappingLabelWithString: "")
    private let sessionProgressLabel = NSTextField(labelWithString: "")
    private let knownButton = NSButton(title: "认识", target: nil, action: nil)
    private let unknownButton = NSButton(title: "不认识", target: nil, action: nil)
    private let masteredButton = NSButton(title: "已学过", target: nil, action: nil)
    private let nextButton = NSButton(title: "Next →", target: nil, action: nil)
    private let speakButton = NSButton(title: "🔊", target: nil, action: nil)

    private let doneTitle = NSTextField(labelWithString: "Done for today ✓")
    private let doneStats = NSTextField(wrappingLabelWithString: "")
    private let continueButton = NSButton(title: "Continue", target: nil, action: nil)
    private let backHomeButton = NSButton(title: "Back Home", target: nil, action: nil)

    init(store: WordbookStore, speech: SpeechService = .shared) {
        self.store = store
        self.planner = WordbookSessionPlanner(store: store)
        self.speech = speech
        self.detailView = container
        installLayouts()
        wireActions()
        showSubview(.home)
    }

    func activate() {
        switch subState {
        case .home:
            refreshHome()
        case .session:
            break
        case .done:
            break
        case .manage:
            break
        }
    }

    func deactivate() {
        loadTask?.cancel()
        prefetchTask?.cancel()
        speech.stop()
    }

    func handleKeyDown(_ event: NSEvent) -> Bool {
        guard case .session = subState else {
            if case .done = subState, event.charactersIgnoringModifiers?.lowercased() == "n" {
                startSession(newWordsOnly: true)
                return true
            }
            return false
        }
        switch event.keyCode {
        case 18: gradeCurrent(.unknown); return true
        case 19: gradeCurrent(.known); return true
        case 20: gradeCurrent(.mastered); return true
        case 49:
            if revealed { advance() } else { revealAnswer() }
            return true
        case 1 where event.charactersIgnoringModifiers?.lowercased() == "s":
            speechEnabled.toggle()
            speakButton.alphaValue = speechEnabled ? 1 : 0.4
            return true
        default:
            return false
        }
    }

    private func installLayouts() {
        installHome()
        installSession()
        installDone()
        container.setContent(homeView, embedInScroll: false)
    }

    private func installHome() {
        homeView.translatesAutoresizingMaskIntoConstraints = false
        progressCard.translatesAutoresizingMaskIntoConstraints = false
        [startButton, newOnlyButton, wrongWordsButton, manageButton].forEach {
            $0.bezelStyle = .rounded
            $0.controlSize = .large
            $0.translatesAutoresizingMaskIntoConstraints = false
        }
        settingsGear.bezelStyle = .inline
        settingsGear.isBordered = false
        settingsGear.image = NSImage(systemSymbolName: "gearshape", accessibilityDescription: "TTS Settings")
        settingsGear.translatesAutoresizingMaskIntoConstraints = false

        let buttons = NSStackView(views: [startButton, newOnlyButton, manageButton])
        buttons.orientation = .horizontal
        buttons.spacing = 12
        buttons.translatesAutoresizingMaskIntoConstraints = false

        homeView.addSubview(progressCard)
        homeView.addSubview(settingsGear)
        homeView.addSubview(buttons)
        NSLayoutConstraint.activate([
            progressCard.topAnchor.constraint(equalTo: homeView.topAnchor, constant: 16),
            progressCard.leadingAnchor.constraint(equalTo: homeView.leadingAnchor, constant: 24),
            progressCard.trailingAnchor.constraint(equalTo: homeView.trailingAnchor, constant: -24),
            settingsGear.topAnchor.constraint(equalTo: homeView.topAnchor, constant: 12),
            settingsGear.trailingAnchor.constraint(equalTo: homeView.trailingAnchor, constant: -24),
            buttons.topAnchor.constraint(equalTo: progressCard.bottomAnchor, constant: 24),
            buttons.centerXAnchor.constraint(equalTo: homeView.centerXAnchor),
            buttons.bottomAnchor.constraint(lessThanOrEqualTo: homeView.bottomAnchor, constant: -24)
        ])
    }

    private func installSession() {
        sessionView.translatesAutoresizingMaskIntoConstraints = false
        termLabel.font = .systemFont(ofSize: 32, weight: .semibold)
        termLabel.alignment = .center
        phoneticLabel.font = .systemFont(ofSize: 15)
        phoneticLabel.textColor = .secondaryLabelColor
        phoneticLabel.alignment = .center
        meaningLabel.font = .systemFont(ofSize: 16, weight: .medium)
        meaningLabel.alignment = .center
        exampleLabel.font = .systemFont(ofSize: 13)
        exampleLabel.textColor = .secondaryLabelColor
        exampleLabel.alignment = .center
        sessionProgressLabel.font = .systemFont(ofSize: 11, weight: .medium)
        sessionProgressLabel.textColor = .tertiaryLabelColor
        [knownButton, unknownButton, masteredButton, nextButton].forEach { $0.bezelStyle = .rounded; $0.controlSize = .large }
        if #available(macOS 13.0, *) {
            unknownButton.bezelColor = .systemRed.withAlphaComponent(0.85)
            knownButton.bezelColor = .systemGreen.withAlphaComponent(0.85)
            masteredButton.bezelColor = .systemYellow.withAlphaComponent(0.85)
        }
        unknownButton.setAccessibilityLabel("不认识，进度归零，快捷键 1")
        knownButton.setAccessibilityLabel("认识，进度加一，快捷键 2")
        masteredButton.setAccessibilityLabel("已学过，永久标记，快捷键 3")
        speakButton.bezelStyle = .accessoryBarAction
        speakButton.isBordered = false
        speakButton.font = .systemFont(ofSize: 18)

        let gradeRow = NSStackView(views: [unknownButton, knownButton, masteredButton])
        gradeRow.orientation = .horizontal
        gradeRow.spacing = 16
        gradeRow.distribution = .fillEqually
        let textColumn = NSStackView(views: [termLabel, phoneticLabel, meaningLabel, exampleLabel])
        textColumn.orientation = .vertical
        textColumn.spacing = 10
        textColumn.alignment = .centerX
        let topBar = NSStackView(views: [sessionProgressLabel, NSView(), speakButton])
        topBar.orientation = .horizontal
        let stack = NSStackView(views: [topBar, textColumn, gradeRow, nextButton])
        stack.orientation = .vertical
        stack.spacing = 18
        stack.alignment = .centerX
        stack.translatesAutoresizingMaskIntoConstraints = false
        sessionView.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.centerXAnchor.constraint(equalTo: sessionView.centerXAnchor),
            stack.topAnchor.constraint(equalTo: sessionView.topAnchor, constant: 12),
            stack.widthAnchor.constraint(lessThanOrEqualToConstant: 480)
        ])
    }

    private func installDone() {
        doneView.translatesAutoresizingMaskIntoConstraints = false
        doneTitle.font = .systemFont(ofSize: 22, weight: .semibold)
        doneTitle.alignment = .center
        doneStats.alignment = .center
        doneStats.font = .systemFont(ofSize: 14)
        doneStats.textColor = .secondaryLabelColor
        [continueButton, backHomeButton].forEach { $0.bezelStyle = .rounded; $0.controlSize = .large }
        let row = NSStackView(views: [continueButton, backHomeButton])
        row.orientation = .horizontal
        row.spacing = 12
        row.translatesAutoresizingMaskIntoConstraints = false
        let stack = NSStackView(views: [doneTitle, doneStats, row])
        stack.orientation = .vertical
        stack.spacing = 16
        stack.alignment = .centerX
        stack.translatesAutoresizingMaskIntoConstraints = false
        doneView.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.centerXAnchor.constraint(equalTo: doneView.centerXAnchor),
            stack.centerYAnchor.constraint(equalTo: doneView.centerYAnchor)
        ])
    }

    private func wireActions() {
        startButton.target = self; startButton.action = #selector(handleStart)
        newOnlyButton.target = self; newOnlyButton.action = #selector(handleNewOnly)
        manageButton.target = self; manageButton.action = #selector(handleManage)
        wrongWordsButton.target = self; wrongWordsButton.action = #selector(handleWrongWords)
        settingsGear.target = self; settingsGear.action = #selector(showTTSPopover(_:))
        unknownButton.target = self; unknownButton.action = #selector(handleUnknown)
        knownButton.target = self; knownButton.action = #selector(handleKnown)
        masteredButton.target = self; masteredButton.action = #selector(handleMastered)
        nextButton.target = self; nextButton.action = #selector(handleNext)
        speakButton.target = self; speakButton.action = #selector(handleSpeak)
        continueButton.target = self; continueButton.action = #selector(handleContinue)
        backHomeButton.target = self; backHomeButton.action = #selector(handleBackHome)
    }

    private func refreshHome() {
        loadTask?.cancel()
        progressCard.showLoading()
        loadTask = Task {
            let snapshot = try? await store.progressSnapshot()
            let dailyNewRemaining = try? await store.dailyNewRemaining(dueCount: snapshot?.dueToday ?? 0)
            guard let snapshot, !Task.isCancelled else { return }
            await MainActor.run {
                progressCard.apply(snapshot)
                let canLearnNew = (dailyNewRemaining ?? 0) > 0 && snapshot.newAvailable > 0
                newOnlyButton.isEnabled = canLearnNew
                newOnlyButton.toolTip = canLearnNew ? nil : "今日新词额度已用尽"
                Task {
                    let wrongCount = try? await store.wrongWordCount(atLeast: 3)
                    await MainActor.run {
                        wrongWordsButton.isEnabled = (wrongCount ?? 0) > 0
                        wrongWordsButton.toolTip = (wrongCount ?? 0) > 0
                            ? "\(wrongCount!) words missed 3+ times"
                            : "No frequent wrong words"
                    }
                }
            }
        }
    }

    private func startSession(newWordsOnly: Bool = false) {
        loadTask?.cancel()
        loadTask = Task {
            await planner.startNewSession(newWordsOnly: newWordsOnly)
            await MainActor.run {
                self.subState = .session
                self.prefetchedCard = nil
                self.showSubview(.session)
                self.advance()
            }
        }
    }

    private func showSubview(_ which: SubState) {
        let view: NSView
        switch which {
        case .home: view = homeView
        case .session: view = sessionView
        case .done: view = doneView
        case .manage:
            let manage = WordbookManageView(store: store, wrongWordsOnly: manageWrongWordsOnly) { [weak self] in
                self?.subState = .home
                self?.manageWrongWordsOnly = false
                self?.manageView = nil
                self?.showSubview(.home)
                self?.refreshHome()
            }
            manageView = manage
            view = manage
        }
        container.setContent(view, embedInScroll: false)
    }

    private func renderWord(_ word: WordEntry, isFresh: Bool) {
        currentWord = word
        revealed = false
        termLabel.stringValue = word.term
        phoneticLabel.stringValue = word.phonetic
        meaningLabel.stringValue = ""
        exampleLabel.stringValue = ""
        let tag = isFresh ? "New" : "Review"
        sessionProgressLabel.stringValue = "\(tag) · Stage \(word.reviewStage + 1)/9"
        nextButton.isHidden = true
        if speechEnabled { speech.speak(word.term) }
    }

    private func revealAnswer() {
        guard let word = currentWord else { return }
        revealed = true
        meaningLabel.stringValue = word.meaning
        exampleLabel.stringValue = word.example
        nextButton.isHidden = false
        if speechEnabled, !word.example.isEmpty { speech.speak(word.example) }
    }

    private func gradeCurrent(_ familiarity: WordFamiliarity) {
        guard let word = currentWord else { return }
        Task {
            _ = try? await store.recordReview(wordID: word.id, familiarity: familiarity)
            await MainActor.run {
                if familiarity == .mastered {
                    self.advance()
                } else {
                    self.revealAnswer()
                    self.prefetchNext()
                }
            }
        }
    }

    private func prefetchNext() {
        prefetchTask?.cancel()
        prefetchTask = Task {
            let card = try? await planner.nextCard()
            return card
        }
    }

    private func advance() {
        loadTask?.cancel()
        loadTask = Task {
            let card: WordbookSessionPlanner.Card?
            if let prefetched = prefetchedCard {
                card = prefetched
                prefetchedCard = nil
            } else if let result = await prefetchTask?.value {
                card = result
            } else {
                card = try? await planner.nextCard()
            }
            await MainActor.run { self.applyCard(card) }
        }
    }

    private func applyCard(_ card: WordbookSessionPlanner.Card?) {
        guard let card else {
            showError("Wordbook unavailable")
            return
        }
        switch card {
        case .review(let word):
            renderWord(word, isFresh: false)
            prefetchNext()
        case .fresh(let word):
            renderWord(word, isFresh: true)
            prefetchNext()
        case .done(let reviewed, let learned):
            showDone(reviewed: reviewed, learned: learned)
        }
    }

    private func showDone(reviewed: Int, learned: Int) {
        subState = .done
        Task {
            let snapshot = try? await store.progressSnapshot()
            let accuracy = Int((snapshot?.accuracyToday ?? 1) * 100)
            let dailyNewRemaining = max(0, (snapshot?.dailyNewLimit ?? 0) - (snapshot?.dailyNewSeen ?? 0))
            let continueCount = (snapshot?.dueToday ?? 0) + min(snapshot?.newAvailable ?? 0, dailyNewRemaining)
            await MainActor.run {
                let streakNote = (snapshot?.streakDays ?? 0) > 0 ? " · 今日 +1 🔥" : ""
                doneStats.stringValue = "复习 \(reviewed) · 新学 \(learned) · 正确率 \(accuracy)%\(streakNote)"
                continueButton.title = continueCount > 0 ? "Continue · \(continueCount) more" : "Back Home"
                showSubview(.done)
            }
        }
    }

    private func showError(_ message: String) {
        subState = .home
        termLabel.stringValue = "Wordbook unavailable"
        meaningLabel.stringValue = message
        exampleLabel.stringValue = ""
        showSubview(.home)
    }

    @objc private func handleStart() { startSession() }
    @objc private func handleNewOnly() { startSession(newWordsOnly: true) }
    @objc private func handleManage() {
        manageWrongWordsOnly = false
        subState = .manage
        showSubview(.manage)
    }

    @objc private func handleWrongWords() {
        manageWrongWordsOnly = true
        subState = .manage
        showSubview(.manage)
    }
    @objc private func handleKnown() { gradeCurrent(.known) }
    @objc private func handleMastered() { gradeCurrent(.mastered) }
    @objc private func handleUnknown() { gradeCurrent(.unknown) }
    @objc private func handleNext() { advance() }
    @objc private func handleSpeak() {
        guard let word = currentWord else { return }
        speech.speak(word.term)
    }
    @objc private func handleContinue() { startSession() }
    @objc private func handleBackHome() {
        subState = .home
        showSubview(.home)
        refreshHome()
    }

    @objc private func showTTSPopover(_ sender: NSButton) {
        let popover = NSPopover()
        let form = NSView(frame: NSRect(x: 0, y: 0, width: 180, height: 100))
        let popup = NSPopUpButton(frame: NSRect(x: 12, y: 40, width: 156, height: 24))
        for (code, label) in [("uk", "UK English"), ("us", "US English"), ("zh-CN", "Chinese")] {
            popup.addItem(withTitle: label)
            popup.lastItem?.representedObject = code
        }
        Task {
            let accent = (try? await store.voiceAccent()) ?? "uk"
            await MainActor.run {
                let idx = ["uk", "us", "zh-CN"].firstIndex(of: accent) ?? 0
                popup.selectItem(at: idx)
            }
        }
        popup.target = self
        popup.action = #selector(accentChanged(_:))
        form.addSubview(popup)
        popover.contentSize = form.bounds.size
        popover.contentViewController = NSViewController()
        popover.contentViewController?.view = form
        popover.behavior = .transient
        popover.show(relativeTo: sender.bounds, of: sender, preferredEdge: .maxY)
    }

    @objc private func accentChanged(_ sender: NSPopUpButton) {
        guard let code = sender.selectedItem?.representedObject as? String else { return }
        let lang = code == "uk" ? "en-GB" : (code == "us" ? "en-US" : "zh-CN")
        speech = SpeechService(languageCode: lang)
        Task { try? await store.setVoiceAccent(code) }
    }
}
