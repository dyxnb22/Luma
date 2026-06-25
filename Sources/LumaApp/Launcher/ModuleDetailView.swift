import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

@MainActor
protocol ModuleDetailView: AnyObject {
    var detailView: NSView { get }
    var moduleTitle: String { get }
    var usesSharedTopBar: Bool { get }
    func activate()
    func deactivate()
    func handleKeyDown(_ event: NSEvent) -> Bool
    func prepareForLauncherHide() async
}

extension ModuleDetailView {
    var usesSharedTopBar: Bool { true }
    func handleKeyDown(_ event: NSEvent) -> Bool { false }
    func prepareForLauncherHide() async {}
}

// Deferred module detail views — not reachable from active dashboard.

@MainActor
final class WindowsDetailView: ModuleDetailView {
    let moduleTitle = "Windows"
    let detailView: NSView
    private let accessibility: any AccessibilityClient
    private let stack = NSStackView()
    private let scroll = NSScrollView()

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

    func deactivate() {}

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
        Task { await svc.focus(windowID: windowID, pid: pid, title: title, axTitle: nil, bounds: nil) }
    }
}

@MainActor
enum ModuleDetailRegistry {
    nonisolated(unsafe) static var clipboardModule: ClipboardModule?
    nonisolated(unsafe) static var notesModule: NotesModule?
    nonisolated(unsafe) static var snippetsModule: SnippetsModule?
    nonisolated(unsafe) static var secretsModule: SecretsModule?
    nonisolated(unsafe) static var mediaModule: MediaModule?
    nonisolated(unsafe) static var todoModule: TodoModule?
    nonisolated(unsafe) static var wordbookStore: WordbookStore?
    nonisolated(unsafe) static var translation: (any TranslationClient)?
    nonisolated(unsafe) static var config: ConfigurationStore?
    nonisolated(unsafe) static var isLauncherQueryEmpty = true

    static func make(for id: ModuleIdentifier) -> (any ModuleDetailView)? {
        switch id {
        case .translate:
            guard let svc = translation, let config else { return nil }
            return TranslateDetailView(translation: svc, config: config) { source, output in
                LauncherCallbackRegistry.current?.onTranslateContentChanged(source, output)
            }
        case .clipboard:
            guard let mod = clipboardModule else { return nil }
            return ClipboardDetailView(module: mod, onOpenSettings: { LauncherCallbackRegistry.current?.onOpenSettings() })
        case .notes:
            guard let mod = notesModule else { return nil }
            return NotesDetailView(module: mod)
        case .snippets:
            guard let mod = snippetsModule else { return nil }
            return SnippetsDetailView(module: mod)
        case .secrets:
            guard let mod = secretsModule else { return nil }
            return SecretsDetailView(module: mod)
        case .media:
            guard let mod = mediaModule else { return nil }
            return MediaDetailView(module: mod)
        case .todo:
            guard let mod = todoModule else { return nil }
            return TodoDetailView(module: mod)
        case .wordbook:
            guard let store = wordbookStore else { return nil }
            return WordbookDetailView(store: store)
        default:
            return nil
        }
    }
}
