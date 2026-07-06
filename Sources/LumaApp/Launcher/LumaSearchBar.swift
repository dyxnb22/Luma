@preconcurrency import AppKit
import LumaCore
import LumaModules

final class LumaSearchBar: NSView {
    enum KeyCommand {
        case up
        case down
        case tab
        case backtab
        case actionPanel
        case commandReturn
        case commandNumber(Int)
    }

    private var detailModeState = LauncherSearchDetailModeState()

    /// True while module detail has disabled the query field (suspended query may still be held).
    var isDetailModeActive: Bool {
        detailModeState.suspendedQuery != nil || !detailModeState.isEditable
    }

    /// Query to persist while detail mode suspends the visible field.
    var persistedQuery: String {
        detailModeState.suspendedQuery ?? stringValue
    }

    /// Suspended query snapshot for `LauncherDetailExitPlanner` (chrome Esc/back/close only).
    var detailSuspendedQueryForPlanner: String? {
        detailModeState.suspendedQuery
    }

    /// True while IME composition is in progress (marked text active).
    var isComposing: Bool {
        guard let editor = textField.currentEditor() as? NSTextView else { return false }
        return editor.hasMarkedText()
    }

    private let surfaceView = NSView()
    private let iconView = NSImageView()
    private let textField = LumaSearchTextField()
    private let settingsButton = SettingsGearButton()
    private let hintsButton = NSButton()
    private let clearButton = NSButton()
    private let hintsPopover = NSPopover()
    private var currentPlaceholderText: String?
    var onTextChange: ((String) -> Void)?
    /// Called when IME composition is active and the visible text changed (starts query-sync polling).
    var onCompositionActive: (() -> Void)?
    var onEscape: (() -> Void)?
    var onReturn: (() -> Void)?
    var onKeyCommand: ((KeyCommand) -> Bool)?
    var onDetailKey: ((NSEvent) -> Bool)?
    var onInterceptKeyDown: ((NSEvent) -> Bool)?
    var onOpenSettings: (() -> Void)?

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        translatesAutoresizingMaskIntoConstraints = false
        GeekUIKit.installSearchSurface(on: surfaceView)
        surfaceView.translatesAutoresizingMaskIntoConstraints = false

        let symbolConfig = NSImage.SymbolConfiguration(
            pointSize: LauncherChromeTokens.searchBarIconSize,
            weight: .medium
        )
        iconView.image = NSImage(systemSymbolName: "magnifyingglass", accessibilityDescription: nil)?
            .withSymbolConfiguration(symbolConfig)
        iconView.contentTintColor = .secondaryLabelColor.withAlphaComponent(0.82)
        iconView.imageScaling = .scaleProportionallyDown
        iconView.translatesAutoresizingMaskIntoConstraints = false

        textField.isBordered = false
        textField.isBezeled = false
        textField.drawsBackground = false
        textField.font = .systemFont(ofSize: LauncherChromeTokens.searchFontSize, weight: .regular)
        applyPlaceholder(ModuleSearchHints.default)
        textField.textColor = .labelColor.withAlphaComponent(0.92)
        textField.focusRingType = .none
        textField.delegate = self
        textField.target = self
        textField.action = #selector(submit)
        textField.translatesAutoresizingMaskIntoConstraints = false
        textField.onEscape = { [weak self] in self?.onEscape?() }
        textField.onKeyCommand = { [weak self] command in self?.onKeyCommand?(command) ?? false }
        textField.onDetailKey = { [weak self] event in self?.onDetailKey?(event) ?? false }
        textField.onInterceptKeyDown = { [weak self] event in self?.onInterceptKeyDown?(event) ?? false }

        hintsButton.isBordered = false
        hintsButton.bezelStyle = .inline
        hintsButton.image = NSImage(systemSymbolName: "info.circle", accessibilityDescription: "Keyboard shortcuts")
        hintsButton.imagePosition = .imageOnly
        hintsButton.contentTintColor = .secondaryLabelColor.withAlphaComponent(0.58)
        hintsButton.target = self
        hintsButton.action = #selector(hintsTapped)
        hintsButton.toolTip = "Keyboard shortcuts"
        hintsButton.refusesFirstResponder = true
        hintsButton.translatesAutoresizingMaskIntoConstraints = false

        settingsButton.isBordered = false
        settingsButton.bezelStyle = .inline
        settingsButton.image = NSImage(systemSymbolName: "gearshape", accessibilityDescription: "Settings")
        settingsButton.imagePosition = .imageOnly
        settingsButton.contentTintColor = .secondaryLabelColor.withAlphaComponent(0.58)
        settingsButton.target = self
        settingsButton.action = #selector(settingsTapped)
        settingsButton.toolTip = "Settings"
        settingsButton.refusesFirstResponder = true
        settingsButton.translatesAutoresizingMaskIntoConstraints = false

        clearButton.isBordered = false
        clearButton.bezelStyle = .inline
        clearButton.image = NSImage(systemSymbolName: "xmark.circle.fill", accessibilityDescription: "Clear search")
        clearButton.imagePosition = .imageOnly
        clearButton.contentTintColor = .secondaryLabelColor.withAlphaComponent(0.58)
        clearButton.target = self
        clearButton.action = #selector(clearTapped)
        clearButton.isHidden = true
        clearButton.refusesFirstResponder = true
        clearButton.translatesAutoresizingMaskIntoConstraints = false

        addSubview(surfaceView)
        surfaceView.addSubview(iconView)
        surfaceView.addSubview(textField)
        surfaceView.addSubview(hintsButton)
        surfaceView.addSubview(settingsButton)
        surfaceView.addSubview(clearButton)

        let inset = LauncherChromeTokens.searchBarInsetH
        NSLayoutConstraint.activate([
            surfaceView.topAnchor.constraint(equalTo: topAnchor),
            surfaceView.leadingAnchor.constraint(equalTo: leadingAnchor),
            surfaceView.trailingAnchor.constraint(equalTo: trailingAnchor),
            surfaceView.bottomAnchor.constraint(equalTo: bottomAnchor),

            iconView.leadingAnchor.constraint(equalTo: surfaceView.leadingAnchor, constant: inset),
            iconView.centerYAnchor.constraint(equalTo: surfaceView.centerYAnchor),
            iconView.widthAnchor.constraint(equalToConstant: LauncherChromeTokens.searchBarIconSize),
            iconView.heightAnchor.constraint(equalToConstant: LauncherChromeTokens.searchBarIconSize),
            textField.leadingAnchor.constraint(equalTo: iconView.trailingAnchor, constant: 10),
            textField.trailingAnchor.constraint(equalTo: clearButton.leadingAnchor, constant: -8),
            textField.centerYAnchor.constraint(equalTo: surfaceView.centerYAnchor),
            settingsButton.trailingAnchor.constraint(equalTo: surfaceView.trailingAnchor, constant: -inset + 2),
            settingsButton.centerYAnchor.constraint(equalTo: surfaceView.centerYAnchor),
            settingsButton.widthAnchor.constraint(equalToConstant: 24),
            settingsButton.heightAnchor.constraint(equalToConstant: 24),
            hintsButton.trailingAnchor.constraint(equalTo: settingsButton.leadingAnchor, constant: -6),
            hintsButton.centerYAnchor.constraint(equalTo: surfaceView.centerYAnchor),
            hintsButton.widthAnchor.constraint(equalToConstant: 24),
            hintsButton.heightAnchor.constraint(equalToConstant: 24),
            clearButton.trailingAnchor.constraint(equalTo: hintsButton.leadingAnchor, constant: -6),
            clearButton.centerYAnchor.constraint(equalTo: surfaceView.centerYAnchor),
            clearButton.widthAnchor.constraint(equalToConstant: 20),
            clearButton.heightAnchor.constraint(equalToConstant: 20)
        ])

        textField.nextKeyView = textField
        hintsButton.nextKeyView = textField
        settingsButton.nextKeyView = textField
        clearButton.nextKeyView = textField
        updateClearButtonVisibility()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    var stringValue: String {
        get { textField.stringValue }
        set {
            let limited = Self.limitedQuery(newValue)
            textField.stringValue = limited
            updateClearButtonVisibility()
        }
    }

    private static func limitedQuery(_ text: String) -> String {
        guard text.count > LauncherQueryLimits.maxCharacters else { return text }
        return String(text.prefix(LauncherQueryLimits.maxCharacters))
    }

    var queryText: String {
        if let editor = textField.currentEditor() {
            return editor.string
        }
        return textField.stringValue
    }

    var isActivelyEditing: Bool {
        guard let window = textField.window else { return false }
        let responder = window.firstResponder
        return responder === textField || responder === textField.currentEditor()
    }

    func commitEditingIfNeeded() {
        guard let editor = textField.currentEditor() else { return }
        let text = editor.string
        guard textField.stringValue != text else { return }
        textField.stringValue = text
        updateClearButtonVisibility()
    }

    func setPlaceholder(_ text: String) {
        applyPlaceholder(text)
    }

    func appendText(_ text: String) {
        guard detailModeState.suspendedQuery == nil, textField.isEditable else { return }
        stringValue = stringValue + text
    }

    func focus() {
        window?.makeFirstResponder(textField)
    }

    /// Clears the query and keeps the field editor in sync when Esc/home resets search.
    func resetQueryText() {
        if let editor = textField.currentEditor() as? NSTextView {
            editor.string = ""
        }
        textField.stringValue = ""
        updateClearButtonVisibility()
    }

    /// Clears the visible query while module detail is open; restores on `endDetailMode()`.
    func beginDetailMode(moduleTitle: String) {
        detailModeState.visibleQuery = stringValue
        detailModeState = LauncherSearchDetailMode.beginDetailMode(detailModeState, moduleTitle: moduleTitle)
        resetQueryText()
        textField.isEditable = detailModeState.isEditable
        if let window, window.firstResponder === textField || window.firstResponder === textField.currentEditor() {
            window.makeFirstResponder(nil)
        }
        setPlaceholder(L10n.tr("launcher.detail.placeholder", moduleTitle))
    }

    func endDetailMode() -> String? {
        let (next, restored) = LauncherSearchDetailMode.endDetailMode(detailModeState)
        detailModeState = next
        textField.isEditable = detailModeState.isEditable
        setPlaceholder(ModuleSearchHints.cheatSheet)
        return restored
    }

    func cancelDetailMode() {
        detailModeState = LauncherSearchDetailMode.cancelDetailMode(detailModeState)
        textField.isEditable = detailModeState.isEditable
        setPlaceholder(ModuleSearchHints.cheatSheet)
    }

    /// Re-enables clicking and typing when detail was torn down without `endDetailMode()` / `cancelDetailMode()`.
    func reEnableSearchFieldIfNeeded() {
        let next = LauncherSearchDetailMode.reEnableSearchFieldIfNeeded(detailModeState)
        guard next != detailModeState else { return }
        detailModeState = next
        textField.isEditable = detailModeState.isEditable
    }

    func clearStuckDetailModeState() {
        detailModeState = LauncherSearchDetailMode.clearStuckDetailModeState(detailModeState)
        textField.isEditable = detailModeState.isEditable
        setPlaceholder(ModuleSearchHints.cheatSheet)
    }

    @objc private func submit() {
        onReturn?()
    }

    @objc private func clearTapped() {
        textField.stringValue = ""
        updateClearButtonVisibility()
        onTextChange?("")
        focus()
    }

    @objc private func settingsTapped() {
        onOpenSettings?()
    }

    @objc private func hintsTapped() {
        if hintsPopover.isShown {
            hintsPopover.performClose(self)
            return
        }
        let label = NSTextField(wrappingLabelWithString: """
        ⌘1…9 — jump to row
        ↑ ↓ — move selection
        Tab / ⌘K — action panel
        Shift+Tab — close action panel
        ⌘↩ — first secondary action
        help clip — module help (IME-friendly)
        Return — run selected item
        Esc — back / close panel
        In detail: Esc back · ⌘W close detail
        Click list and type — forwards to search
        """)
        label.font = .systemFont(ofSize: 12)
        label.textColor = .labelColor
        label.preferredMaxLayoutWidth = 220
        let host = NSView(frame: NSRect(x: 0, y: 0, width: 240, height: 120))
        label.translatesAutoresizingMaskIntoConstraints = false
        host.addSubview(label)
        NSLayoutConstraint.activate([
            label.topAnchor.constraint(equalTo: host.topAnchor, constant: 12),
            label.leadingAnchor.constraint(equalTo: host.leadingAnchor, constant: 12),
            label.trailingAnchor.constraint(equalTo: host.trailingAnchor, constant: -12),
            label.bottomAnchor.constraint(equalTo: host.bottomAnchor, constant: -12)
        ])
        hintsPopover.contentSize = NSSize(width: 240, height: 120)
        hintsPopover.contentViewController = NSViewController()
        hintsPopover.contentViewController?.view = host
        hintsPopover.behavior = .transient
        hintsPopover.show(relativeTo: hintsButton.bounds, of: hintsButton, preferredEdge: .maxY)
    }

    private func updateClearButtonVisibility() {
        clearButton.isHidden = textField.stringValue.isEmpty
    }

    private func applyPlaceholder(_ text: String) {
        guard currentPlaceholderText != text else { return }
        currentPlaceholderText = text
        textField.placeholderString = nil
        textField.placeholderAttributedString = NSAttributedString(
            string: text,
            attributes: [
                .foregroundColor: NSColor.secondaryLabelColor.withAlphaComponent(0.66),
                .font: textField.font ?? .systemFont(ofSize: LauncherChromeTokens.searchFontSize)
            ]
        )
    }
}

extension LumaSearchBar: NSTextFieldDelegate {
    func controlTextDidChange(_ obj: Notification) {
        let now = CFAbsoluteTimeGetCurrent()
        LatencyTracker.shared.markKeystroke(at: now)
        if let editor = textField.currentEditor(), editor.string.count > LauncherQueryLimits.maxCharacters {
            editor.string = Self.limitedQuery(editor.string)
        }
        updateClearButtonVisibility()
        guard LauncherQueryDispatchPolicy.shouldDispatchQuery(isComposing: isComposing) else {
            onCompositionActive?()
            return
        }
        onTextChange?(queryText)
    }

    func controlTextDidEndEditing(_ obj: Notification) {
        commitEditingIfNeeded()
        guard LauncherQueryDispatchPolicy.shouldDispatchQuery(isComposing: isComposing) else {
            onCompositionActive?()
            return
        }
        onTextChange?(queryText)
    }

    func control(_ control: NSControl, textView: NSTextView, doCommandBy commandSelector: Selector) -> Bool {
        if commandSelector == #selector(NSResponder.insertBacktab(_:)) {
            _ = onKeyCommand?(.backtab)
            return true
        }
        if commandSelector == #selector(NSResponder.insertTab(_:)) {
            _ = onKeyCommand?(.tab)
            return true
        }
        return false
    }
}

@MainActor
final class LatencyTracker {
    static let shared = LatencyTracker()
    private var lastKeystroke: CFAbsoluteTime?

    func markKeystroke(at time: CFAbsoluteTime) {
        lastKeystroke = time
    }

    func markFirstPaint() -> Double? {
        guard let last = lastKeystroke else { return nil }
        lastKeystroke = nil
        return (CFAbsoluteTimeGetCurrent() - last) * 1000
    }
}

private final class LumaSearchTextField: NSTextField {
    nonisolated(unsafe) var onEscape: (() -> Void)?
    nonisolated(unsafe) var onKeyCommand: ((LumaSearchBar.KeyCommand) -> Bool)?
    nonisolated(unsafe) var onDetailKey: ((NSEvent) -> Bool)?
    nonisolated(unsafe) var onInterceptKeyDown: ((NSEvent) -> Bool)?

    nonisolated override var stringValue: String {
        get { super.stringValue }
        set {
            let before = super.stringValue
            if let editor = currentEditor() as? NSTextView {
                editor.string = newValue
            }
            super.stringValue = newValue
            guard newValue != before else { return }
            notifyTextDidChange()
        }
    }

    nonisolated private func notifyTextDidChange() {
        NotificationCenter.default.post(name: NSControl.textDidChangeNotification, object: self)
    }

    nonisolated override func performKeyEquivalent(with event: NSEvent) -> Bool {
        if let window, LumaStandardEditShortcuts.performKeyEquivalent(event, in: window) {
            return true
        }
        return super.performKeyEquivalent(with: event)
    }

    nonisolated override func keyDown(with event: NSEvent) {
        if onInterceptKeyDown?(event) == true { return }
        if LumaStandardEditShortcuts.handleKeyDown(event, in: window) { return }
        if !isEditable {
            if onDetailKey?(event) == true { return }
            if event.keyCode == 53 {
                onEscape?()
                return
            }
            if event.keyCode == 125 || event.keyCode == 126 { return }
            if event.modifierFlags.contains(.command),
               let chars = event.charactersIgnoringModifiers,
               let number = Int(chars),
               (1...9).contains(number) { return }
            return
        }
        if stringValue.isEmpty, onDetailKey?(event) == true { return }
        if event.keyCode == 125, onKeyCommand?(.down) == true { return }
        if event.keyCode == 126, onKeyCommand?(.up) == true { return }
        if event.keyCode == 48 {
            _ = onKeyCommand?(.tab)
            return
        }
        if event.modifierFlags.contains(.command),
           event.charactersIgnoringModifiers?.lowercased() == "k",
           onKeyCommand?(.actionPanel) == true { return }
        if event.modifierFlags.contains(.command),
           event.keyCode == 36,
           onKeyCommand?(.commandReturn) == true { return }
        if event.modifierFlags.contains(.command),
           let chars = event.charactersIgnoringModifiers,
           let number = Int(chars),
           (1...9).contains(number),
           onKeyCommand?(.commandNumber(number)) == true {
            return
        }
        super.keyDown(with: event)
    }

    nonisolated override func cancelOperation(_ sender: Any?) {
        Task { @MainActor in self.onEscape?() }
    }

    nonisolated override func insertTab(_ sender: Any?) {
        _ = onKeyCommand?(.tab)
    }

    nonisolated override func insertBacktab(_ sender: Any?) {
        _ = onKeyCommand?(.backtab)
    }
}

private final class SettingsGearButton: NSButton {
    private var tracking: NSTrackingArea?

    nonisolated override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let tracking { removeTrackingArea(tracking) }
        let area = NSTrackingArea(
            rect: bounds,
            options: [.mouseEnteredAndExited, .activeAlways, .inVisibleRect],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(area)
        tracking = area
    }

    nonisolated override func mouseEntered(with event: NSEvent) {
        contentTintColor = .secondaryLabelColor
    }

    nonisolated override func mouseExited(with event: NSEvent) {
        contentTintColor = .tertiaryLabelColor
    }
}
