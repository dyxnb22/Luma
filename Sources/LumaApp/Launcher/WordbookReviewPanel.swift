import AppKit
import LumaModules
import LumaServices

/// Dedicated review window for the Wordbook module (ADR-009).
///
/// Operationally independent from the 860x540 launcher panel: closing it does not affect launcher
/// state, and the launcher panel never holds review state for the duration of a session.
@MainActor
final class WordbookReviewPanel: NSPanel {
    private let store: WordbookStore
    private let speech: SpeechService
    private var queue: [WordEntry] = []
    private var currentIndex: Int = 0
    private var revealed: Bool = false
    private var completedCount: Int = 0

    private let termLabel = NSTextField(labelWithString: "")
    private let phoneticLabel = NSTextField(labelWithString: "")
    private let meaningLabel = NSTextField(wrappingLabelWithString: "")
    private let exampleLabel = NSTextField(wrappingLabelWithString: "")
    private let progressLabel = NSTextField(labelWithString: "")
    private let knownButton = NSButton(title: "Known", target: nil, action: nil)
    private let fuzzyButton = NSButton(title: "Fuzzy", target: nil, action: nil)
    private let unknownButton = NSButton(title: "Unknown", target: nil, action: nil)
    private let nextButton = NSButton(title: "Next →", target: nil, action: nil)
    private let speakButton = NSButton(title: "🔊", target: nil, action: nil)
    private let closeButton = NSButton(title: "✕", target: nil, action: nil)

    init(store: WordbookStore = WordbookStore(), speech: SpeechService = .shared) {
        self.store = store
        self.speech = speech
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 480, height: 360),
            styleMask: [.titled, .closable, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        title = "Wordbook Review"
        titleVisibility = .hidden
        titlebarAppearsTransparent = true
        isMovableByWindowBackground = true
        isReleasedWhenClosed = false
        level = .floating
        hidesOnDeactivate = false
        backgroundColor = .clear
        isOpaque = false
        installContentView()
        wireActions()
    }

    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }

    override func cancelOperation(_ sender: Any?) {
        speech.stop()
        orderOut(nil)
    }

    override func keyDown(with event: NSEvent) {
        // Esc handled by cancelOperation. 1/2/3 grade. Space reveals/advances.
        switch event.keyCode {
        case 18: gradeCurrent(.known)        // 1
        case 19: gradeCurrent(.fuzzy)        // 2
        case 20: gradeCurrent(.unknown)      // 3
        case 49:                              // Space
            if revealed { advance() } else { revealAnswer() }
        default:
            super.keyDown(with: event)
        }
    }

    // MARK: - Public entry

    /// Loads the next batch of due words and shows the panel. Call from the dashboard card action.
    func startSession(limit: Int = 20, near origin: NSPoint? = nil) {
        Task { @MainActor in
            do {
                let due = try await store.dueWords(limit: limit)
                self.queue = due
                self.currentIndex = 0
                self.completedCount = 0
                self.revealed = false
                if self.queue.isEmpty {
                    self.showDoneState()
                } else {
                    self.renderCurrent()
                }
                self.center()
                if let origin {
                    var frame = self.frame
                    frame.origin = origin
                    self.setFrame(frame, display: true)
                }
                self.makeKeyAndOrderFront(nil)
            } catch {
                self.showError(error.localizedDescription)
                self.makeKeyAndOrderFront(nil)
            }
        }
    }

    // MARK: - Layout

    private func installContentView() {
        let root = NSVisualEffectView(frame: .zero)
        root.material = .underWindowBackground
        root.blendingMode = .behindWindow
        root.state = .active
        root.wantsLayer = true
        root.layer?.cornerRadius = 16
        root.layer?.borderWidth = 1
        root.layer?.borderColor = NSColor.white.withAlphaComponent(0.18).cgColor
        root.translatesAutoresizingMaskIntoConstraints = false
        contentView = root

        termLabel.font = .systemFont(ofSize: 32, weight: .semibold)
        termLabel.alignment = .center
        phoneticLabel.font = .systemFont(ofSize: 15, weight: .regular)
        phoneticLabel.textColor = .secondaryLabelColor
        phoneticLabel.alignment = .center
        meaningLabel.font = .systemFont(ofSize: 16, weight: .medium)
        meaningLabel.alignment = .center
        meaningLabel.maximumNumberOfLines = 3
        exampleLabel.font = .systemFont(ofSize: 13, weight: .regular)
        exampleLabel.textColor = .secondaryLabelColor
        exampleLabel.alignment = .center
        exampleLabel.maximumNumberOfLines = 3
        progressLabel.font = .systemFont(ofSize: 11, weight: .medium)
        progressLabel.textColor = .tertiaryLabelColor

        [knownButton, fuzzyButton, unknownButton, nextButton].forEach { button in
            button.bezelStyle = .rounded
            button.controlSize = .large
        }
        speakButton.bezelStyle = .accessoryBarAction
        speakButton.isBordered = false
        speakButton.font = .systemFont(ofSize: 18)
        closeButton.bezelStyle = .accessoryBarAction
        closeButton.isBordered = false
        closeButton.font = .systemFont(ofSize: 14)

        let gradeRow = NSStackView(views: [unknownButton, fuzzyButton, knownButton])
        gradeRow.orientation = .horizontal
        gradeRow.spacing = 16
        gradeRow.distribution = .fillEqually

        let textColumn = NSStackView(views: [termLabel, phoneticLabel, meaningLabel, exampleLabel])
        textColumn.orientation = .vertical
        textColumn.spacing = 10
        textColumn.alignment = .centerX

        let topBar = NSStackView(views: [progressLabel, NSView(), speakButton, closeButton])
        topBar.orientation = .horizontal
        topBar.spacing = 8
        topBar.distribution = .fill

        let stack = NSStackView(views: [topBar, textColumn, gradeRow, nextButton])
        stack.orientation = .vertical
        stack.spacing = 18
        stack.alignment = .centerX
        stack.edgeInsets = NSEdgeInsets(top: 20, left: 24, bottom: 20, right: 24)
        stack.translatesAutoresizingMaskIntoConstraints = false
        root.addSubview(stack)

        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            stack.topAnchor.constraint(equalTo: root.topAnchor),
            stack.bottomAnchor.constraint(equalTo: root.bottomAnchor),
            topBar.leadingAnchor.constraint(equalTo: stack.leadingAnchor, constant: 16),
            topBar.trailingAnchor.constraint(equalTo: stack.trailingAnchor, constant: -16)
        ])
    }

    private func wireActions() {
        knownButton.target = self
        knownButton.action = #selector(handleKnown)
        fuzzyButton.target = self
        fuzzyButton.action = #selector(handleFuzzy)
        unknownButton.target = self
        unknownButton.action = #selector(handleUnknown)
        nextButton.target = self
        nextButton.action = #selector(handleNext)
        speakButton.target = self
        speakButton.action = #selector(handleSpeak)
        closeButton.target = self
        closeButton.action = #selector(handleDismiss)
    }

    // MARK: - State transitions

    private func renderCurrent() {
        guard currentIndex < queue.count else {
            showDoneState()
            return
        }
        let word = queue[currentIndex]
        revealed = false
        termLabel.stringValue = word.term
        phoneticLabel.stringValue = word.phonetic
        meaningLabel.stringValue = ""
        exampleLabel.stringValue = ""
        progressLabel.stringValue = "Reviewing \(currentIndex + 1) / \(queue.count)"
        nextButton.isHidden = true
        knownButton.isHidden = false
        fuzzyButton.isHidden = false
        unknownButton.isHidden = false
        speech.speak(word.term)
    }

    private func revealAnswer() {
        guard currentIndex < queue.count else { return }
        let word = queue[currentIndex]
        revealed = true
        meaningLabel.stringValue = word.meaning
        exampleLabel.stringValue = word.example
        nextButton.isHidden = false
        if !word.example.isEmpty {
            speech.speak(word.example)
        }
    }

    private func gradeCurrent(_ familiarity: WordFamiliarity) {
        guard currentIndex < queue.count else { return }
        let word = queue[currentIndex]
        Task { @MainActor in
            _ = try? await self.store.recordReview(wordID: word.id, familiarity: familiarity)
            self.completedCount += 1
            self.revealAnswer()
        }
    }

    private func advance() {
        currentIndex += 1
        if currentIndex < queue.count {
            renderCurrent()
        } else {
            showDoneState()
        }
    }

    private func showDoneState() {
        revealed = false
        termLabel.stringValue = "Done for today"
        phoneticLabel.stringValue = ""
        meaningLabel.stringValue = "Reviewed \(completedCount) word\(completedCount == 1 ? "" : "s")."
        exampleLabel.stringValue = "Press Esc to close."
        progressLabel.stringValue = ""
        knownButton.isHidden = true
        fuzzyButton.isHidden = true
        unknownButton.isHidden = true
        nextButton.isHidden = true
    }

    private func showError(_ message: String) {
        termLabel.stringValue = "Wordbook unavailable"
        phoneticLabel.stringValue = ""
        meaningLabel.stringValue = message
        exampleLabel.stringValue = ""
        progressLabel.stringValue = ""
        knownButton.isHidden = true
        fuzzyButton.isHidden = true
        unknownButton.isHidden = true
        nextButton.isHidden = true
    }

    // MARK: - Actions

    @objc private func handleKnown() { gradeCurrent(.known) }
    @objc private func handleFuzzy() { gradeCurrent(.fuzzy) }
    @objc private func handleUnknown() { gradeCurrent(.unknown) }
    @objc private func handleNext() { advance() }
    @objc private func handleSpeak() {
        guard currentIndex < queue.count else { return }
        speech.speak(queue[currentIndex].term)
    }
    @objc private func handleDismiss() {
        speech.stop()
        orderOut(nil)
    }
}
