import AppKit
import LumaCore
import LumaModules
import LumaServices

@MainActor
protocol ModuleDetailView: AnyObject {
    var detailView: NSView { get }
    var moduleTitle: String { get }
    func activate()
    func deactivate()
}

@MainActor
final class TranslateDetailView: ModuleDetailView {
    let moduleTitle = "Translate"
    let detailView: NSView
    private let translation: any TranslationClient
    private let input = NSTextView()
    private let output = NSTextField(wrappingLabelWithString: "")
    private let copyButton = NSButton(title: "Copy", target: nil, action: nil)
    private let statusLabel = NSTextField(labelWithString: "")
    private var pendingTask: Task<Void, Never>?

    init(translation: any TranslationClient) {
        self.translation = translation
        let container = NSView()
        container.translatesAutoresizingMaskIntoConstraints = false
        self.detailView = container
        setup(container: container)
    }

    func activate() {
        DispatchQueue.main.async { [weak self] in
            self?.detailView.window?.makeFirstResponder(self?.input)
        }
    }

    func deactivate() {
        pendingTask?.cancel()
        pendingTask = nil
    }

    private func setup(container: NSView) {
        let inputScroll = NSScrollView()
        inputScroll.hasVerticalScroller = true
        inputScroll.borderType = .lineBorder
        inputScroll.translatesAutoresizingMaskIntoConstraints = false
        input.font = .systemFont(ofSize: 14)
        input.isEditable = true
        input.delegate = nil
        input.autoresizingMask = [.width]
        inputScroll.documentView = input
        NotificationCenter.default.addObserver(self, selector: #selector(inputChanged), name: NSText.didChangeNotification, object: input)

        output.font = .systemFont(ofSize: 14)
        output.textColor = .labelColor
        output.translatesAutoresizingMaskIntoConstraints = false

        copyButton.target = self
        copyButton.action = #selector(copyOutput)
        copyButton.isEnabled = false
        copyButton.bezelStyle = .rounded
        copyButton.translatesAutoresizingMaskIntoConstraints = false

        statusLabel.font = .systemFont(ofSize: 12)
        statusLabel.textColor = .secondaryLabelColor
        statusLabel.translatesAutoresizingMaskIntoConstraints = false

        container.addSubview(inputScroll)
        container.addSubview(output)
        container.addSubview(copyButton)
        container.addSubview(statusLabel)

        NSLayoutConstraint.activate([
            inputScroll.topAnchor.constraint(equalTo: container.topAnchor, constant: 12),
            inputScroll.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 12),
            inputScroll.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -12),
            inputScroll.heightAnchor.constraint(equalToConstant: 100),

            output.topAnchor.constraint(equalTo: inputScroll.bottomAnchor, constant: 14),
            output.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 12),
            output.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -12),

            copyButton.topAnchor.constraint(equalTo: output.bottomAnchor, constant: 12),
            copyButton.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 12),

            statusLabel.centerYAnchor.constraint(equalTo: copyButton.centerYAnchor),
            statusLabel.leadingAnchor.constraint(equalTo: copyButton.trailingAnchor, constant: 14),
            statusLabel.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -12)
        ])
    }

    @objc private func inputChanged() {
        let text = input.string.trimmingCharacters(in: .whitespacesAndNewlines)
        pendingTask?.cancel()
        guard !text.isEmpty else {
            output.stringValue = ""
            statusLabel.stringValue = ""
            copyButton.isEnabled = false
            return
        }
        statusLabel.stringValue = "Translating…"
        copyButton.isEnabled = false
        let svc = translation
        pendingTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(280))
            guard !Task.isCancelled else { return }
            do {
                let result = try await svc.translate(text)
                await MainActor.run {
                    guard let self else { return }
                    self.output.stringValue = result
                    self.statusLabel.stringValue = ""
                    self.copyButton.isEnabled = true
                }
            } catch {
                await MainActor.run {
                    guard let self else { return }
                    self.output.stringValue = ""
                    self.statusLabel.stringValue = "Translation unavailable. Create a Shortcut named \"Luma Translate\"."
                    self.copyButton.isEnabled = false
                }
            }
        }
    }

    @objc private func copyOutput() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(output.stringValue, forType: .string)
    }
}

@MainActor
final class ClipboardDetailView: ModuleDetailView {
    let moduleTitle = "Clipboard"
    let detailView: NSView
    private let module: ClipboardModule
    private let stack = NSStackView()
    private let scroll = NSScrollView()
    private var refreshTask: Task<Void, Never>?

    init(module: ClipboardModule) {
        self.module = module
        let container = NSView()
        container.translatesAutoresizingMaskIntoConstraints = false
        self.detailView = container
        setup(container: container)
    }

    func activate() {
        refresh()
    }

    func deactivate() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    private func setup(container: NSView) {
        stack.orientation = .vertical
        stack.spacing = 6
        stack.alignment = .leading
        stack.edgeInsets = NSEdgeInsets(top: 8, left: 8, bottom: 8, right: 8)
        stack.translatesAutoresizingMaskIntoConstraints = false

        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        scroll.borderType = .noBorder
        scroll.translatesAutoresizingMaskIntoConstraints = false
        scroll.documentView = stack

        container.addSubview(scroll)
        NSLayoutConstraint.activate([
            scroll.topAnchor.constraint(equalTo: container.topAnchor, constant: 8),
            scroll.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 8),
            scroll.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -8),
            scroll.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -8),
            stack.widthAnchor.constraint(equalTo: scroll.contentView.widthAnchor, constant: -16)
        ])
    }

    private func refresh() {
        refreshTask?.cancel()
        refreshTask = Task { [weak self] in
            guard let self else { return }
            let entries = await self.module.recentEntries(limit: 50)
            await MainActor.run {
                self.render(entries: entries)
            }
        }
    }

    private func render(entries: [ClipboardEntry]) {
        stack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        guard !entries.isEmpty else {
            let empty = NSTextField(labelWithString: "Clipboard history is empty.")
            empty.textColor = .secondaryLabelColor
            empty.font = .systemFont(ofSize: 13)
            stack.addArrangedSubview(empty)
            return
        }
        for entry in entries {
            stack.addArrangedSubview(makeRow(entry: entry))
        }
    }

    private func makeRow(entry: ClipboardEntry) -> NSView {
        let row = NSStackView()
        row.orientation = .horizontal
        row.spacing = 8
        row.alignment = .centerY
        row.translatesAutoresizingMaskIntoConstraints = false

        let preview = NSTextField(labelWithString: entry.text.prefix(80) + (entry.text.count > 80 ? "…" : ""))
        preview.font = .systemFont(ofSize: 13)
        preview.lineBreakMode = .byTruncatingTail
        preview.setContentHuggingPriority(.defaultLow, for: .horizontal)
        preview.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        let pinButton = NSButton(title: entry.isPinned ? "★" : "☆", target: self, action: #selector(togglePin(_:)))
        pinButton.identifier = NSUserInterfaceItemIdentifier(entry.id.uuidString)
        pinButton.bezelStyle = .regularSquare
        pinButton.isBordered = false
        pinButton.font = .systemFont(ofSize: 14)

        let copyButton = NSButton(title: "Copy", target: self, action: #selector(copy(_:)))
        copyButton.identifier = NSUserInterfaceItemIdentifier(entry.id.uuidString)
        copyButton.bezelStyle = .rounded
        copyButton.font = .systemFont(ofSize: 12)

        let deleteButton = NSButton(title: "Delete", target: self, action: #selector(deleteRow(_:)))
        deleteButton.identifier = NSUserInterfaceItemIdentifier(entry.id.uuidString)
        deleteButton.bezelStyle = .rounded
        deleteButton.font = .systemFont(ofSize: 12)

        row.addArrangedSubview(preview)
        row.addArrangedSubview(pinButton)
        row.addArrangedSubview(copyButton)
        row.addArrangedSubview(deleteButton)
        return row
    }

    @objc private func togglePin(_ sender: NSButton) {
        guard let raw = sender.identifier?.rawValue, let id = UUID(uuidString: raw) else { return }
        Task { [weak self] in
            guard let self else { return }
            await self.module.togglePin(id)
            await MainActor.run { self.refresh() }
        }
    }

    @objc private func copy(_ sender: NSButton) {
        guard let raw = sender.identifier?.rawValue, let id = UUID(uuidString: raw) else { return }
        Task { [weak self] in
            guard let self else { return }
            let entries = await self.module.recentEntries(limit: 500)
            guard let entry = entries.first(where: { $0.id == id }) else { return }
            await MainActor.run {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(entry.text, forType: .string)
            }
        }
    }

    @objc private func deleteRow(_ sender: NSButton) {
        guard let raw = sender.identifier?.rawValue, let id = UUID(uuidString: raw) else { return }
        Task { [weak self] in
            guard let self else { return }
            await self.module.remove(id)
            await MainActor.run { self.refresh() }
        }
    }
}

@MainActor
final class CalculatorDetailView: ModuleDetailView {
    let moduleTitle = "Calculator"
    let detailView: NSView
    private let input = NSTextField()
    private let resultLabel = NSTextField(labelWithString: "")
    private let copyButton = NSButton(title: "Copy Result", target: nil, action: nil)

    init() {
        let container = NSView()
        container.translatesAutoresizingMaskIntoConstraints = false
        self.detailView = container
        setup(container: container)
    }

    func activate() {
        DispatchQueue.main.async { [weak self] in
            self?.detailView.window?.makeFirstResponder(self?.input)
        }
    }

    func deactivate() {}

    private func setup(container: NSView) {
        input.placeholderString = "Enter expression, e.g. 3*7+1"
        input.font = .systemFont(ofSize: 18)
        input.isBezeled = true
        input.bezelStyle = .roundedBezel
        input.target = self
        input.action = #selector(evaluate)
        input.delegate = nil
        input.translatesAutoresizingMaskIntoConstraints = false
        NotificationCenter.default.addObserver(self, selector: #selector(textChanged(_:)), name: NSControl.textDidChangeNotification, object: input)

        resultLabel.font = .systemFont(ofSize: 28, weight: .semibold)
        resultLabel.alignment = .left
        resultLabel.translatesAutoresizingMaskIntoConstraints = false

        copyButton.target = self
        copyButton.action = #selector(copyResult)
        copyButton.bezelStyle = .rounded
        copyButton.isEnabled = false
        copyButton.translatesAutoresizingMaskIntoConstraints = false

        container.addSubview(input)
        container.addSubview(resultLabel)
        container.addSubview(copyButton)
        NSLayoutConstraint.activate([
            input.topAnchor.constraint(equalTo: container.topAnchor, constant: 16),
            input.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 16),
            input.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -16),
            resultLabel.topAnchor.constraint(equalTo: input.bottomAnchor, constant: 18),
            resultLabel.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 16),
            resultLabel.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -16),
            copyButton.topAnchor.constraint(equalTo: resultLabel.bottomAnchor, constant: 14),
            copyButton.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 16)
        ])
    }

    @objc private func textChanged(_ note: Notification) {
        evaluate()
    }

    @objc private func evaluate() {
        let raw = input.stringValue.trimmingCharacters(in: .whitespaces)
        guard !raw.isEmpty else {
            resultLabel.stringValue = ""
            copyButton.isEnabled = false
            return
        }
        guard let value = SafeExpressionEvaluator.evaluate(raw) else {
            resultLabel.stringValue = "—"
            copyButton.isEnabled = false
            return
        }
        let formatted: String
        if value == floor(value) {
            formatted = "\(Int(value))"
        } else {
            formatted = String(value)
        }
        resultLabel.stringValue = formatted
        copyButton.isEnabled = true
    }

    @objc private func copyResult() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(resultLabel.stringValue, forType: .string)
    }
}

private struct SafeExpressionEvaluator {
    private let characters: [Character]
    private var index = 0

    static func evaluate(_ expression: String) -> Double? {
        var parser = SafeExpressionEvaluator(expression)
        guard let value = parser.parseExpression() else { return nil }
        parser.skipSpaces()
        guard parser.index == parser.characters.count, value.isFinite else { return nil }
        return value
    }

    private init(_ expression: String) {
        self.characters = Array(expression)
    }

    private mutating func parseExpression() -> Double? {
        guard var value = parseTerm() else { return nil }
        while true {
            skipSpaces()
            if consume("+") {
                guard let rhs = parseTerm() else { return nil }
                value += rhs
            } else if consume("-") {
                guard let rhs = parseTerm() else { return nil }
                value -= rhs
            } else {
                return value
            }
        }
    }

    private mutating func parseTerm() -> Double? {
        guard var value = parseFactor() else { return nil }
        while true {
            skipSpaces()
            if consume("*") {
                guard let rhs = parseFactor() else { return nil }
                value *= rhs
            } else if consume("/") {
                guard let rhs = parseFactor(), rhs != 0 else { return nil }
                value /= rhs
            } else {
                return value
            }
        }
    }

    private mutating func parseFactor() -> Double? {
        skipSpaces()
        if consume("+") { return parseFactor() }
        if consume("-") { return parseFactor().map { -$0 } }
        if consume("(") {
            guard let value = parseExpression() else { return nil }
            skipSpaces()
            return consume(")") ? value : nil
        }
        return parseNumber()
    }

    private mutating func parseNumber() -> Double? {
        skipSpaces()
        let start = index
        var hasDecimal = false
        while index < characters.count {
            let ch = characters[index]
            if ch == "." {
                guard !hasDecimal else { return nil }
                hasDecimal = true
                index += 1
            } else if ch.isNumber {
                index += 1
            } else {
                break
            }
        }
        guard start != index else { return nil }
        return Double(String(characters[start..<index]))
    }

    private mutating func consume(_ expected: Character) -> Bool {
        skipSpaces()
        guard index < characters.count, characters[index] == expected else { return false }
        index += 1
        return true
    }

    private mutating func skipSpaces() {
        while index < characters.count, characters[index].isWhitespace {
            index += 1
        }
    }
}

@MainActor
final class WindowsDetailView: ModuleDetailView {
    let moduleTitle = "Windows"
    let detailView: NSView
    private let accessibility: any AccessibilityClient
    private let stack = NSStackView()
    private let scroll = NSScrollView()
    private var refreshTask: Task<Void, Never>?

    init(accessibility: any AccessibilityClient) {
        self.accessibility = accessibility
        let container = NSView()
        container.translatesAutoresizingMaskIntoConstraints = false
        self.detailView = container
        setup(container: container)
    }

    func activate() {
        refresh()
    }

    func deactivate() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    private func setup(container: NSView) {
        stack.orientation = .vertical
        stack.spacing = 6
        stack.alignment = .leading
        stack.edgeInsets = NSEdgeInsets(top: 8, left: 8, bottom: 8, right: 8)
        stack.translatesAutoresizingMaskIntoConstraints = false

        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        scroll.borderType = .noBorder
        scroll.translatesAutoresizingMaskIntoConstraints = false
        scroll.documentView = stack

        container.addSubview(scroll)
        NSLayoutConstraint.activate([
            scroll.topAnchor.constraint(equalTo: container.topAnchor, constant: 8),
            scroll.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 8),
            scroll.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -8),
            scroll.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -8),
            stack.widthAnchor.constraint(equalTo: scroll.contentView.widthAnchor, constant: -16)
        ])
    }

    private func refresh() {
        let records = WindowEnumerator.windows()
        stack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        guard !records.isEmpty else {
            let empty = NSTextField(labelWithString: "No focusable windows found.")
            empty.textColor = .secondaryLabelColor
            empty.font = .systemFont(ofSize: 13)
            stack.addArrangedSubview(empty)
            return
        }
        for record in records.prefix(40) {
            stack.addArrangedSubview(makeRow(record: record))
        }
    }

    private func makeRow(record: WindowRecord) -> NSView {
        let row = NSStackView()
        row.orientation = .horizontal
        row.spacing = 8
        row.alignment = .centerY
        row.translatesAutoresizingMaskIntoConstraints = false

        let title = NSTextField(labelWithString: record.title.isEmpty ? record.appName : record.title)
        title.font = .systemFont(ofSize: 13)
        title.lineBreakMode = .byTruncatingTail
        title.setContentHuggingPriority(.defaultLow, for: .horizontal)
        title.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        let appName = NSTextField(labelWithString: record.appName)
        appName.font = .systemFont(ofSize: 11)
        appName.textColor = .secondaryLabelColor
        appName.lineBreakMode = .byTruncatingTail

        let focusButton = NSButton(title: "Focus", target: self, action: #selector(focus(_:)))
        focusButton.identifier = NSUserInterfaceItemIdentifier("\(record.windowID)|\(record.pid)|\(record.title)")
        focusButton.bezelStyle = .rounded
        focusButton.font = .systemFont(ofSize: 12)

        row.addArrangedSubview(title)
        row.addArrangedSubview(appName)
        row.addArrangedSubview(focusButton)
        return row
    }

    @objc private func focus(_ sender: NSButton) {
        guard let raw = sender.identifier?.rawValue else { return }
        let parts = raw.split(separator: "|", maxSplits: 2)
        guard parts.count == 3,
              let windowID = UInt32(parts[0]),
              let pid = Int32(parts[1]) else { return }
        let title = String(parts[2])
        let svc = accessibility
        Task { await svc.focus(windowID: windowID, pid: pid, title: title) }
    }
}

@MainActor
enum ModuleDetailRegistry {
    nonisolated(unsafe) static var clipboardModule: ClipboardModule?
    nonisolated(unsafe) static var translation: (any TranslationClient)?
    nonisolated(unsafe) static var accessibility: (any AccessibilityClient)?

    static func make(for id: ModuleIdentifier) -> (any ModuleDetailView)? {
        switch id {
        case .translate:
            guard let svc = translation else { return nil }
            return TranslateDetailView(translation: svc)
        case .clipboard:
            guard let mod = clipboardModule else { return nil }
            return ClipboardDetailView(module: mod)
        case .calculator:
            return CalculatorDetailView()
        case .windows:
            guard let ax = accessibility else { return nil }
            return WindowsDetailView(accessibility: ax)
        default:
            return nil
        }
    }
}
