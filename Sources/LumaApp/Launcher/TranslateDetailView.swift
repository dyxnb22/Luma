import AppKit
import LumaCore
import LumaInfrastructure
import LumaServices

@MainActor
enum TranslateDashboardStatus {
    static var summary: String = "Ready"
    static var targetLanguageCode: String = "en"
}

@MainActor
final class TranslateDetailView: ModuleDetailView {
    let moduleTitle = "Translate"
    let detailView: NSView
    let usesSharedTopBar = false

    private let translation: any TranslationClient
    private let config: ConfigurationStore
    private var onBack: (() -> Void)?
    private var onContentChanged: ((String, String) -> Void)?

    private let sourceLabel = NSTextField(labelWithString: "Auto Detect")
    private let targetPopup = NSPopUpButton()
    private let swapButton = NSButton()
    private let statusLabel = NSTextField(labelWithString: "")
    private let errorBanner = NSView()
    private let errorBannerLabel = NSTextField(wrappingLabelWithString: "")
    private let inputTextView = TranslateInputTextView()
    private let outputTextView = TranslateOutputTextView()
    private let inputPanel = NSView()
    private let outputPanel = NSView()
    private let panelsStack = NSStackView()
    private let translateButton = NSButton()
    private let copyResultButton = NSButton()
    private let clearButton = NSButton()
    private let pasteButton = NSButton()
    private let copySourceButton = NSButton()

    private var sourceLanguageCode: String? = nil
    private var pendingTask: Task<Void, Never>?
    private var translationState: TranslationUIState = .idle
    private var languageChipButtons: [NSButton] = []

    private static let panelTextInset = NSSize(width: 12, height: 12)
    private static let panelFont = NSFont.systemFont(ofSize: 14)

    private static let languageOptions: [(code: String, name: String)] = [
        ("en", "English"),
        ("zh-Hans", "Chinese (Simplified)"),
        ("zh-Hant", "Chinese (Traditional)"),
        ("ja", "Japanese"),
        ("ko", "Korean"),
        ("es", "Spanish"),
        ("fr", "French"),
        ("de", "German"),
        ("it", "Italian"),
        ("pt", "Portuguese"),
        ("ru", "Russian"),
        ("ar", "Arabic")
    ]

    init(
        translation: any TranslationClient,
        config: ConfigurationStore,
        onBack: @escaping () -> Void,
        onContentChanged: ((String, String) -> Void)? = nil
    ) {
        self.translation = translation
        self.config = config
        self.onBack = onBack
        self.onContentChanged = onContentChanged
        let container = NSView()
        container.translatesAutoresizingMaskIntoConstraints = false
        self.detailView = container
        setup(container: container)
    }

    func activate() {
        Task { await loadTargetLanguage() }
        DispatchQueue.main.async { [weak self] in
            self?.detailView.window?.makeFirstResponder(self?.inputTextView)
        }
    }

    func deactivate() {
        pendingTask?.cancel()
        pendingTask = nil
    }

    func prefill(text: String, autoTranslate: Bool) {
        inputTextView.string = text
        inputTextView.refreshPlaceholder()
        updateTranslateButtonState()
        notifyContentChanged()
        if autoTranslate {
            performTranslation()
        }
    }

    func restore(sourceText: String, outputText: String) {
        inputTextView.string = sourceText
        outputTextView.string = outputText
        inputTextView.refreshPlaceholder()
        copyResultButton.isEnabled = !outputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        updateTranslateButtonState()
        setState(outputText.isEmpty ? .idle : .success)
        notifyContentChanged()
    }

    func currentContent() -> (source: String, output: String) {
        (inputTextView.string, outputTextView.string)
    }

    func handleKeyDown(_ event: NSEvent) -> Bool {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if event.keyCode == 53 {
            onBack?()
            return true
        }
        if flags.contains(.command), event.charactersIgnoringModifiers?.lowercased() == "\r" {
            performTranslation()
            return true
        }
        return false
    }

    private func loadTargetLanguage() async {
        let code = await config.translationTargetLanguage()
        await MainActor.run {
            selectTargetLanguage(code)
            TranslateDashboardStatus.targetLanguageCode = code
            updateDashboardSummary()
        }
    }

    private func setup(container: NSView) {
        let header = buildHeader()
        let toolbar = buildToolbar()
        setupPanels()
        setupActionButtons()

        inputTextView.placeholderString = "Type or paste text…"
        inputTextView.font = Self.panelFont
        inputTextView.textContainerInset = Self.panelTextInset
        inputTextView.textContainer?.lineFragmentPadding = 0

        panelsStack.orientation = .horizontal
        panelsStack.spacing = 12
        panelsStack.distribution = .fillEqually
        panelsStack.translatesAutoresizingMaskIntoConstraints = false
        panelsStack.addArrangedSubview(inputPanel)
        panelsStack.addArrangedSubview(outputPanel)

        statusLabel.font = .systemFont(ofSize: 12)
        statusLabel.textColor = .secondaryLabelColor
        statusLabel.lineBreakMode = .byWordWrapping
        statusLabel.maximumNumberOfLines = 2
        statusLabel.translatesAutoresizingMaskIntoConstraints = false

        errorBanner.wantsLayer = true
        errorBanner.layer?.cornerRadius = 8
        errorBanner.layer?.cornerCurve = .continuous
        errorBanner.layer?.backgroundColor = NSColor.systemRed.withAlphaComponent(0.12).cgColor
        errorBanner.isHidden = true
        errorBanner.translatesAutoresizingMaskIntoConstraints = false

        errorBannerLabel.font = .systemFont(ofSize: 12, weight: .medium)
        errorBannerLabel.textColor = .systemRed
        errorBannerLabel.isEditable = false
        errorBannerLabel.isSelectable = true
        errorBannerLabel.isBezeled = false
        errorBannerLabel.drawsBackground = false
        errorBannerLabel.translatesAutoresizingMaskIntoConstraints = false
        errorBanner.addSubview(errorBannerLabel)

        container.addSubview(header)
        container.addSubview(toolbar)
        container.addSubview(errorBanner)
        container.addSubview(panelsStack)
        container.addSubview(statusLabel)
        container.addSubview(copyResultButton)
        container.addSubview(clearButton)
        container.addSubview(pasteButton)
        container.addSubview(copySourceButton)

        NSLayoutConstraint.activate([
            header.topAnchor.constraint(equalTo: container.topAnchor, constant: 8),
            header.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 12),
            header.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -12),
            header.heightAnchor.constraint(equalToConstant: 36),

            toolbar.topAnchor.constraint(equalTo: header.bottomAnchor, constant: 8),
            toolbar.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 12),
            toolbar.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -12),
            toolbar.heightAnchor.constraint(equalToConstant: 64),

            errorBanner.topAnchor.constraint(equalTo: toolbar.bottomAnchor, constant: 8),
            errorBanner.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 12),
            errorBanner.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -12),

            errorBannerLabel.topAnchor.constraint(equalTo: errorBanner.topAnchor, constant: 8),
            errorBannerLabel.leadingAnchor.constraint(equalTo: errorBanner.leadingAnchor, constant: 10),
            errorBannerLabel.trailingAnchor.constraint(equalTo: errorBanner.trailingAnchor, constant: -10),
            errorBannerLabel.bottomAnchor.constraint(equalTo: errorBanner.bottomAnchor, constant: -8),

            panelsStack.topAnchor.constraint(equalTo: errorBanner.bottomAnchor, constant: 8),
            panelsStack.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 12),
            panelsStack.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -12),
            panelsStack.heightAnchor.constraint(greaterThanOrEqualToConstant: 180),

            statusLabel.topAnchor.constraint(equalTo: panelsStack.bottomAnchor, constant: 10),
            statusLabel.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 12),
            statusLabel.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -12),

            copyResultButton.topAnchor.constraint(equalTo: statusLabel.bottomAnchor, constant: 10),
            copyResultButton.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 12),
            copyResultButton.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -12),

            clearButton.centerYAnchor.constraint(equalTo: copyResultButton.centerYAnchor),
            clearButton.leadingAnchor.constraint(equalTo: copyResultButton.trailingAnchor, constant: 8),

            pasteButton.centerYAnchor.constraint(equalTo: copyResultButton.centerYAnchor),
            pasteButton.leadingAnchor.constraint(equalTo: clearButton.trailingAnchor, constant: 8),

            copySourceButton.centerYAnchor.constraint(equalTo: copyResultButton.centerYAnchor),
            copySourceButton.leadingAnchor.constraint(equalTo: pasteButton.trailingAnchor, constant: 8)
        ])

        inputTextView.onCommandReturn = { [weak self] in self?.performTranslation() }
        outputTextView.onCommandC = { [weak self] in self?.copyResult() }

        NotificationCenter.default.addObserver(
            self,
            selector: #selector(inputChanged),
            name: NSText.didChangeNotification,
            object: inputTextView
        )
    }

    deinit {
        NotificationCenter.default.removeObserver(self)
    }

    private func buildHeader() -> NSView {
        let header = NSView()
        header.translatesAutoresizingMaskIntoConstraints = false

        let backButton = NSButton(title: "Back", target: self, action: #selector(goBack))
        backButton.image = NSImage(systemSymbolName: "chevron.left", accessibilityDescription: nil)
        backButton.imagePosition = .imageLeading
        backButton.bezelStyle = .regularSquare
        backButton.isBordered = false
        backButton.font = .systemFont(ofSize: 13, weight: .medium)
        backButton.translatesAutoresizingMaskIntoConstraints = false

        let title = NSTextField(labelWithString: "Translate")
        title.font = .systemFont(ofSize: 15, weight: .semibold)
        title.translatesAutoresizingMaskIntoConstraints = false

        header.addSubview(backButton)
        header.addSubview(title)
        NSLayoutConstraint.activate([
            backButton.leadingAnchor.constraint(equalTo: header.leadingAnchor),
            backButton.centerYAnchor.constraint(equalTo: header.centerYAnchor),
            title.centerXAnchor.constraint(equalTo: header.centerXAnchor),
            title.centerYAnchor.constraint(equalTo: header.centerYAnchor)
        ])
        return header
    }

    private func buildToolbar() -> NSView {
        let toolbar = NSView()
        toolbar.translatesAutoresizingMaskIntoConstraints = false

        let chipRow = NSStackView()
        chipRow.orientation = .horizontal
        chipRow.spacing = 6
        chipRow.translatesAutoresizingMaskIntoConstraints = false
        languageChipButtons = []
        for (code, label) in [("zh-Hans", "中文"), ("en", "EN"), ("ja", "日本語"), ("ko", "한국어")] {
            let chip = NSButton(title: label, target: self, action: #selector(quickLanguageChip(_:)))
            chip.bezelStyle = .rounded
            chip.font = .systemFont(ofSize: 11, weight: .medium)
            chip.identifier = NSUserInterfaceItemIdentifier(code)
            chipRow.addArrangedSubview(chip)
            languageChipButtons.append(chip)
        }

        sourceLabel.font = .systemFont(ofSize: 12, weight: .medium)
        sourceLabel.textColor = .secondaryLabelColor
        sourceLabel.translatesAutoresizingMaskIntoConstraints = false

        targetPopup.removeAllItems()
        for option in Self.languageOptions {
            targetPopup.addItem(withTitle: option.name)
            targetPopup.lastItem?.representedObject = option.code
        }
        targetPopup.target = self
        targetPopup.action = #selector(targetLanguageChanged)
        targetPopup.translatesAutoresizingMaskIntoConstraints = false

        swapButton.image = NSImage(systemSymbolName: "arrow.left.arrow.right", accessibilityDescription: "Swap languages")
        swapButton.bezelStyle = .regularSquare
        swapButton.isBordered = false
        swapButton.target = self
        swapButton.action = #selector(swapLanguages)
        swapButton.isEnabled = false
        swapButton.toolTip = "Translate once to detect the source language before swapping"
        swapButton.translatesAutoresizingMaskIntoConstraints = false

        translateButton.title = "Translate"
        translateButton.bezelStyle = .rounded
        translateButton.font = .systemFont(ofSize: 13, weight: .semibold)
        translateButton.target = self
        translateButton.action = #selector(translateTapped)
        translateButton.isEnabled = false
        translateButton.translatesAutoresizingMaskIntoConstraints = false
        translateButton.setAccessibilityLabel("Translate")
        translateButton.toolTip = "翻译 ⌘↩"

        toolbar.addSubview(chipRow)
        toolbar.addSubview(sourceLabel)
        toolbar.addSubview(swapButton)
        toolbar.addSubview(targetPopup)
        toolbar.addSubview(translateButton)

        NSLayoutConstraint.activate([
            chipRow.leadingAnchor.constraint(equalTo: toolbar.leadingAnchor),
            chipRow.topAnchor.constraint(equalTo: toolbar.topAnchor),
            chipRow.trailingAnchor.constraint(lessThanOrEqualTo: toolbar.trailingAnchor),

            sourceLabel.leadingAnchor.constraint(equalTo: toolbar.leadingAnchor),
            sourceLabel.topAnchor.constraint(equalTo: chipRow.bottomAnchor, constant: 6),

            swapButton.leadingAnchor.constraint(equalTo: sourceLabel.trailingAnchor, constant: 10),
            swapButton.centerYAnchor.constraint(equalTo: sourceLabel.centerYAnchor),

            targetPopup.leadingAnchor.constraint(equalTo: swapButton.trailingAnchor, constant: 10),
            targetPopup.centerYAnchor.constraint(equalTo: sourceLabel.centerYAnchor),
            targetPopup.widthAnchor.constraint(greaterThanOrEqualToConstant: 140),

            translateButton.trailingAnchor.constraint(equalTo: toolbar.trailingAnchor),
            translateButton.centerYAnchor.constraint(equalTo: sourceLabel.centerYAnchor),

            sourceLabel.bottomAnchor.constraint(equalTo: toolbar.bottomAnchor)
        ])
        return toolbar
    }

    private func setupPanels() {
        configurePanel(inputPanel, textView: inputTextView)
        configurePanel(outputPanel, textView: outputTextView)
        outputTextView.isEditable = false
    }

    private func configurePanel(_ panel: NSView, textView: NSTextView) {
        panel.wantsLayer = true
        panel.layer?.cornerRadius = 12
        panel.layer?.cornerCurve = .continuous
        panel.layer?.borderWidth = 1
        panel.layer?.borderColor = NSColor.separatorColor.withAlphaComponent(0.35).cgColor
        panel.layer?.backgroundColor = NSColor.textBackgroundColor.withAlphaComponent(0.55).cgColor
        panel.translatesAutoresizingMaskIntoConstraints = false

        textView.isRichText = false
        textView.font = Self.panelFont
        textView.isVerticallyResizable = true
        textView.isHorizontallyResizable = false
        textView.textContainer?.widthTracksTextView = true
        textView.textContainerInset = Self.panelTextInset
        textView.textContainer?.lineFragmentPadding = 0
        textView.drawsBackground = false
        textView.autoresizingMask = [.width]
        textView.minSize = NSSize(width: 0, height: 120)
        textView.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)

        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        scroll.borderType = .noBorder
        scroll.translatesAutoresizingMaskIntoConstraints = false
        scroll.documentView = textView

        panel.addSubview(scroll)
        NSLayoutConstraint.activate([
            scroll.topAnchor.constraint(equalTo: panel.topAnchor),
            scroll.leadingAnchor.constraint(equalTo: panel.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: panel.trailingAnchor),
            scroll.bottomAnchor.constraint(equalTo: panel.bottomAnchor),
            panel.heightAnchor.constraint(equalToConstant: 180)
        ])
    }

    private func setupActionButtons() {
        copyResultButton.title = "Copy Result"
        copyResultButton.bezelStyle = .rounded
        copyResultButton.target = self
        copyResultButton.action = #selector(copyResult)
        copyResultButton.isEnabled = false
        copyResultButton.translatesAutoresizingMaskIntoConstraints = false

        clearButton.title = "Clear"
        clearButton.bezelStyle = .rounded
        clearButton.target = self
        clearButton.action = #selector(clearAll)
        clearButton.translatesAutoresizingMaskIntoConstraints = false

        pasteButton.title = "Paste from Clipboard"
        pasteButton.bezelStyle = .rounded
        pasteButton.target = self
        pasteButton.action = #selector(pasteFromClipboard)
        pasteButton.translatesAutoresizingMaskIntoConstraints = false

        copySourceButton.title = "Copy Source"
        copySourceButton.bezelStyle = .rounded
        copySourceButton.target = self
        copySourceButton.action = #selector(copySource)
        copySourceButton.translatesAutoresizingMaskIntoConstraints = false
    }

    @objc private func translateTapped() {
        performTranslation()
    }

    @objc private func goBack() {
        onBack?()
    }

    @objc private func quickLanguageChip(_ sender: NSButton) {
        guard let code = sender.identifier?.rawValue else { return }
        selectTargetLanguage(code)
        Task {
            await config.setTranslationTargetLanguage(code)
            await MainActor.run {
                TranslateDashboardStatus.targetLanguageCode = code
                updateDashboardSummary()
                if !inputTextView.string.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    performTranslation()
                }
            }
        }
    }

    @objc private func targetLanguageChanged() {
        guard let code = targetPopup.selectedItem?.representedObject as? String else { return }
        selectTargetLanguage(code)
        Task {
            await config.setTranslationTargetLanguage(code)
            await MainActor.run {
                TranslateDashboardStatus.targetLanguageCode = code
                updateDashboardSummary()
            }
        }
    }

    @objc private func swapLanguages() {
        guard let source = sourceLanguageCode else { return }
        guard let currentTarget = targetPopup.selectedItem?.representedObject as? String else { return }
        sourceLanguageCode = currentTarget
        sourceLabel.stringValue = displayName(for: currentTarget)
        selectTargetLanguage(source)
        Task { await config.setTranslationTargetLanguage(source) }
    }

    @objc private func inputChanged() {
        inputTextView.refreshPlaceholder()
        updateTranslateButtonState()
        notifyContentChanged()
        if translationState != .translating {
            hideErrorBanner()
            setState(.idle)
        }
    }

    private func updateTranslateButtonState() {
        let hasText = !inputTextView.string.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        translateButton.isEnabled = hasText && translationState != .translating
    }

    @objc private func copyResult() {
        let text = outputTextView.string.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }

    @objc private func clearAll() {
        inputTextView.string = ""
        outputTextView.string = ""
        inputTextView.refreshPlaceholder()
        updateTranslateButtonState()
        sourceLanguageCode = nil
        sourceLabel.stringValue = "Auto Detect"
        swapButton.isEnabled = false
        swapButton.toolTip = "Translate once to detect the source language before swapping"
        setState(.idle)
        hideErrorBanner()
        notifyContentChanged()
    }

    @objc private func pasteFromClipboard() {
        guard let text = NSPasteboard.general.string(forType: .string) else { return }
        inputTextView.string = text
        inputTextView.refreshPlaceholder()
        updateTranslateButtonState()
        notifyContentChanged()
        performTranslation()
    }

    @objc private func copySource() {
        let text = inputTextView.string.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }

    private func performTranslation() {
        let text = inputTextView.string.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else {
            setState(.idle)
            return
        }
        setState(.translating)
        hideErrorBanner()
        copyResultButton.isEnabled = false
        translateButton.isEnabled = false
        let svc = translation
        pendingTask?.cancel()
        pendingTask = Task { [weak self] in
            do {
                let outcome = try await svc.translate(text)
                await MainActor.run {
                    guard let self, !Task.isCancelled else { return }
                    self.outputTextView.string = outcome.text
                    self.applyDetectedSourceLanguage(outcome.detectedSourceLanguageCode)
                    self.hideErrorBanner()
                    self.setState(.success)
                    self.copyResultButton.isEnabled = true
                    self.updateTranslateButtonState()
                    self.notifyContentChanged()
                }
            } catch is CancellationError {
                return
            } catch {
                await MainActor.run {
                    guard let self, !Task.isCancelled else { return }
                    self.showErrorBanner(Self.userFacingError(error))
                    self.setState(.error(Self.userFacingError(error)))
                    self.copyResultButton.isEnabled = false
                    self.updateTranslateButtonState()
                    self.notifyContentChanged()
                }
            }
        }
    }

    private func notifyContentChanged() {
        onContentChanged?(inputTextView.string, outputTextView.string)
    }

    private func applyDetectedSourceLanguage(_ code: String?) {
        guard let code, !code.isEmpty else {
            sourceLanguageCode = nil
            sourceLabel.stringValue = "Auto Detect"
            swapButton.isEnabled = false
            swapButton.toolTip = "Translate once to detect the source language before swapping"
            return
        }
        sourceLanguageCode = code
        sourceLabel.stringValue = displayName(for: code)
        swapButton.isEnabled = true
        swapButton.toolTip = "Swap source and target languages"
    }

    private func setState(_ state: TranslationUIState) {
        translationState = state
        switch state {
        case .idle:
            statusLabel.stringValue = ""
            statusLabel.textColor = .secondaryLabelColor
            TranslateDashboardStatus.summary = "Ready"
        case .translating:
            statusLabel.stringValue = "Translating…"
            statusLabel.textColor = .secondaryLabelColor
            TranslateDashboardStatus.summary = "Translating…"
        case .success:
            statusLabel.stringValue = "Translation complete"
            statusLabel.textColor = .secondaryLabelColor
            TranslateDashboardStatus.summary = "Last: success"
        case .error:
            statusLabel.stringValue = ""
            statusLabel.textColor = .secondaryLabelColor
            TranslateDashboardStatus.summary = "Last: unavailable"
        }
        updateDashboardSummary()
        updateTranslateButtonState()
    }

    private func updateDashboardSummary() {
        let lang = displayName(for: TranslateDashboardStatus.targetLanguageCode)
        if TranslateDashboardStatus.summary == "Ready" {
            TranslateDashboardStatus.summary = "→ \(lang)"
        }
    }

    private func selectTargetLanguage(_ code: String) {
        if let index = Self.languageOptions.firstIndex(where: { $0.code == code }) {
            targetPopup.selectItem(at: index)
        } else {
            targetPopup.addItem(withTitle: code)
            targetPopup.lastItem?.representedObject = code
            targetPopup.selectItem(at: targetPopup.numberOfItems - 1)
        }
        TranslateDashboardStatus.targetLanguageCode = code
        updateChipHighlight(selectedCode: code)
    }

    private func updateChipHighlight(selectedCode: String) {
        for chip in languageChipButtons {
            let selected = chip.identifier?.rawValue == selectedCode
            if #available(macOS 13.0, *) {
                chip.bezelColor = selected ? .controlAccentColor : nil
            }
            chip.contentTintColor = selected ? .white : .labelColor
            chip.alphaValue = selected ? 1 : 0.82
        }
    }

    private func displayName(for code: String) -> String {
        Self.languageOptions.first(where: { $0.code == code })?.name ?? code
    }

    static func userFacingError(_ error: Error) -> String {
        TranslationUserMessages.message(for: error)
    }

    private func showErrorBanner(_ message: String) {
        errorBannerLabel.stringValue = message
        errorBanner.isHidden = false
    }

    private func hideErrorBanner() {
        errorBannerLabel.stringValue = ""
        errorBanner.isHidden = true
    }
}

private enum TranslationUIState: Equatable {
    case idle
    case translating
    case success
    case error(String)
}

private final class TranslateInputTextView: NSTextView {
    var onCommandReturn: (() -> Void)?
    var placeholderString = ""

    private var placeholderAttributes: [NSAttributedString.Key: Any] {
        [
            .font: font ?? NSFont.systemFont(ofSize: 14),
            .foregroundColor: NSColor.placeholderTextColor
        ]
    }

    func refreshPlaceholder() {
        needsDisplay = true
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard string.isEmpty, !placeholderString.isEmpty else { return }
        let padding = textContainer?.lineFragmentPadding ?? 0
        let origin = NSPoint(
            x: textContainerInset.width + padding,
            y: textContainerInset.height
        )
        let size = NSSize(
            width: bounds.width - origin.x - textContainerInset.width,
            height: bounds.height - origin.y - textContainerInset.height
        )
        let rect = NSRect(origin: origin, size: size)
        placeholderString.draw(with: rect, options: [.usesLineFragmentOrigin, .usesFontLeading], attributes: placeholderAttributes)
    }

    override var string: String {
        didSet { needsDisplay = true }
    }

    override func didChangeText() {
        super.didChangeText()
        needsDisplay = true
    }

    override func keyDown(with event: NSEvent) {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if flags.contains(.command), event.charactersIgnoringModifiers == "\r" {
            onCommandReturn?()
            return
        }
        if event.keyCode == 36 || event.keyCode == 76 {
            if flags.contains(.shift) {
                super.keyDown(with: event)
                return
            }
            onCommandReturn?()
            return
        }
        super.keyDown(with: event)
    }
}

private final class TranslateOutputTextView: NSTextView {
    var onCommandC: (() -> Void)?

    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if flags.contains(.command), event.charactersIgnoringModifiers?.lowercased() == "c" {
            onCommandC?()
            return true
        }
        return super.performKeyEquivalent(with: event)
    }
}
