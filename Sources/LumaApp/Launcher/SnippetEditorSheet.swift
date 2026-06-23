import AppKit
import LumaModules

@MainActor
final class SnippetEditorSheet: NSWindow {
    private let onSave: (String, String, String, [String]) -> Void
    private let titleField = NSTextField()
    private let triggerField = NSTextField()
    private let tagsField = NSTextField()
    private let bodyTextView = NSTextView()

    init(snippet: Snippet?, onSave: @escaping (String, String, String, [String]) -> Void) {
        self.onSave = onSave
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 480, height: 400),
            styleMask: [.titled, .closable],
            backing: .buffered,
            defer: false
        )
        title = snippet == nil ? "Add Snippet" : "Edit Snippet"
        setup(snippet: snippet)
    }

    private func setup(snippet: Snippet?) {
        let container = NSView(frame: NSRect(x: 0, y: 0, width: 480, height: 400))

        titleField.stringValue = snippet?.title ?? ""
        titleField.placeholderString = "Title"
        titleField.translatesAutoresizingMaskIntoConstraints = false

        triggerField.stringValue = snippet?.trigger ?? ""
        triggerField.placeholderString = "Trigger (e.g. ;addr)"
        triggerField.translatesAutoresizingMaskIntoConstraints = false

        tagsField.stringValue = snippet?.tags.joined(separator: ", ") ?? ""
        tagsField.placeholderString = "Tags (comma-separated)"
        tagsField.translatesAutoresizingMaskIntoConstraints = false

        bodyTextView.string = snippet?.content ?? ""
        bodyTextView.isRichText = false
        bodyTextView.font = .monospacedSystemFont(ofSize: 12, weight: .regular)
        bodyTextView.translatesAutoresizingMaskIntoConstraints = false

        let scroll = NSScrollView()
        scroll.documentView = bodyTextView
        scroll.hasVerticalScroller = true
        scroll.borderType = .bezelBorder
        scroll.translatesAutoresizingMaskIntoConstraints = false

        let saveButton = NSButton(title: "Save", target: self, action: #selector(save))
        saveButton.bezelStyle = .rounded
        saveButton.keyEquivalent = "\r"
        saveButton.translatesAutoresizingMaskIntoConstraints = false

        let cancelButton = NSButton(title: "Cancel", target: self, action: #selector(cancel))
        cancelButton.bezelStyle = .rounded
        cancelButton.translatesAutoresizingMaskIntoConstraints = false

        container.addSubview(titleField)
        container.addSubview(triggerField)
        container.addSubview(tagsField)
        container.addSubview(scroll)
        container.addSubview(saveButton)
        container.addSubview(cancelButton)
        contentView = container

        NSLayoutConstraint.activate([
            titleField.topAnchor.constraint(equalTo: container.topAnchor, constant: 16),
            titleField.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 16),
            titleField.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -16),

            triggerField.topAnchor.constraint(equalTo: titleField.bottomAnchor, constant: 8),
            triggerField.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            triggerField.trailingAnchor.constraint(equalTo: titleField.trailingAnchor),

            tagsField.topAnchor.constraint(equalTo: triggerField.bottomAnchor, constant: 8),
            tagsField.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            tagsField.trailingAnchor.constraint(equalTo: titleField.trailingAnchor),

            scroll.topAnchor.constraint(equalTo: tagsField.bottomAnchor, constant: 8),
            scroll.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: titleField.trailingAnchor),
            scroll.heightAnchor.constraint(equalToConstant: 200),

            cancelButton.topAnchor.constraint(equalTo: scroll.bottomAnchor, constant: 12),
            cancelButton.trailingAnchor.constraint(equalTo: saveButton.leadingAnchor, constant: -8),
            saveButton.topAnchor.constraint(equalTo: scroll.bottomAnchor, constant: 12),
            saveButton.trailingAnchor.constraint(equalTo: titleField.trailingAnchor)
        ])
    }

    @objc private func save() {
        let title = titleField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let trigger = triggerField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let content = bodyTextView.string.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !title.isEmpty, !trigger.isEmpty, !content.isEmpty else {
            if trigger.isEmpty {
                triggerField.becomeFirstResponder()
            }
            return
        }
        let tags = tagsField.stringValue.split(separator: ",").map { String($0).trimmingCharacters(in: .whitespacesAndNewlines) }
        onSave(title, trigger, content, tags)
        close()
    }

    @objc private func cancel() {
        close()
    }
}
