import AppKit
import LumaModules

@MainActor
final class SnippetEditorSheet: NSWindow {
  private let editingID: UUID?
  private let onSave: (String, String, String, [String]) -> Void
  private let titleField = NSTextField()
  private let triggerField = NSTextField()
  private let tagsField = NSTextField()
  private let contentHintLabel = NSTextField(labelWithString: "")
  private let conflictLabel = NSTextField(labelWithString: "")
  private let bodyTextView = NSTextView()
  private var conflictTask: Task<Void, Never>?

  init(
    snippet: Snippet?,
    draft: SnippetDraft? = nil,
    onSave: @escaping (String, String, String, [String]) -> Void
  ) {
    self.editingID = snippet?.id
    self.onSave = onSave
    super.init(
      contentRect: NSRect(x: 0, y: 0, width: 480, height: 440),
      styleMask: [.titled, .closable],
      backing: .buffered,
      defer: false
    )
    title = snippet == nil ? "Add Snippet" : "Edit Snippet"
    setup(snippet: snippet, draft: draft)
  }

  private func setup(snippet: Snippet?, draft: SnippetDraft?) {
    let container = NSView(frame: NSRect(x: 0, y: 0, width: 480, height: 440))

    titleField.stringValue = snippet?.title ?? draft?.title ?? ""
    titleField.placeholderString = "Title"
    titleField.translatesAutoresizingMaskIntoConstraints = false
    titleField.target = self
    titleField.action = #selector(fieldsChanged)

    let resolvedTrigger = snippet?.trigger
      ?? draft?.trigger
      ?? (draft.map { Snippet(title: $0.title, content: $0.content).displayTrigger } ?? "")
    triggerField.stringValue = resolvedTrigger
    triggerField.placeholderString = "Trigger (e.g. ;addr)"
    triggerField.translatesAutoresizingMaskIntoConstraints = false
    triggerField.target = self
    triggerField.action = #selector(fieldsChanged)
    NotificationCenter.default.addObserver(
      self, selector: #selector(fieldsChanged), name: NSTextField.textDidChangeNotification, object: triggerField
    )

    tagsField.stringValue = snippet?.tags.joined(separator: ", ") ?? draft?.tags.joined(separator: ", ") ?? ""
    tagsField.placeholderString = "Tags (comma-separated)"
    tagsField.translatesAutoresizingMaskIntoConstraints = false

    contentHintLabel.font = .systemFont(ofSize: 11)
    contentHintLabel.textColor = .secondaryLabelColor
    contentHintLabel.lineBreakMode = .byWordWrapping
    contentHintLabel.maximumNumberOfLines = 2
    contentHintLabel.translatesAutoresizingMaskIntoConstraints = false
    if let draft, draft.isLongClipboardClip {
      let lineCount = draft.content.split(separator: "\n", omittingEmptySubsequences: false).count
      contentHintLabel.stringValue =
        "Long clipboard clip (\(draft.content.count) chars, \(lineCount) lines). Line breaks are preserved below."
    } else {
      contentHintLabel.isHidden = true
    }

    conflictLabel.font = .systemFont(ofSize: 11)
    conflictLabel.textColor = .systemOrange
    conflictLabel.lineBreakMode = .byWordWrapping
    conflictLabel.maximumNumberOfLines = 3
    conflictLabel.isHidden = true
    conflictLabel.translatesAutoresizingMaskIntoConstraints = false

    bodyTextView.string = snippet?.content ?? draft?.content ?? ""
    bodyTextView.isRichText = false
    bodyTextView.font = .monospacedSystemFont(ofSize: 12, weight: .regular)
    bodyTextView.isAutomaticQuoteSubstitutionEnabled = false
    bodyTextView.translatesAutoresizingMaskIntoConstraints = false
    NotificationCenter.default.addObserver(
      self, selector: #selector(fieldsChanged), name: NSText.didChangeNotification, object: bodyTextView
    )

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
    container.addSubview(contentHintLabel)
    container.addSubview(conflictLabel)
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

      contentHintLabel.topAnchor.constraint(equalTo: tagsField.bottomAnchor, constant: 6),
      contentHintLabel.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
      contentHintLabel.trailingAnchor.constraint(equalTo: titleField.trailingAnchor),

      conflictLabel.topAnchor.constraint(equalTo: contentHintLabel.bottomAnchor, constant: 4),
      conflictLabel.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
      conflictLabel.trailingAnchor.constraint(equalTo: titleField.trailingAnchor),

      scroll.topAnchor.constraint(equalTo: conflictLabel.bottomAnchor, constant: 6),
      scroll.leadingAnchor.constraint(equalTo: titleField.leadingAnchor),
      scroll.trailingAnchor.constraint(equalTo: titleField.trailingAnchor),
      scroll.heightAnchor.constraint(equalToConstant: 200),

      cancelButton.topAnchor.constraint(equalTo: scroll.bottomAnchor, constant: 12),
      cancelButton.trailingAnchor.constraint(equalTo: saveButton.leadingAnchor, constant: -8),
      saveButton.topAnchor.constraint(equalTo: scroll.bottomAnchor, constant: 12),
      saveButton.trailingAnchor.constraint(equalTo: titleField.trailingAnchor)
    ])

    fieldsChanged()
  }

  @objc private func fieldsChanged() {
    conflictTask?.cancel()
    conflictTask = Task { [weak self] in
      guard let self else { return }
      guard let module = LauncherEnvironment.current?.snippetsModule else { return }
      let trigger = await MainActor.run { self.triggerField.stringValue }
      let content = await MainActor.run { self.bodyTextView.string }
      let editingID = self.editingID
      let triggerConflict = await module.conflictingSnippet(trigger: trigger, excluding: editingID)
      let contentConflict = await module.similarSnippet(content: content, excluding: editingID)
      await MainActor.run {
        if let triggerConflict {
          self.conflictLabel.stringValue =
            "Trigger “\(triggerConflict.displayTrigger)” is used by “\(triggerConflict.title)”."
          self.conflictLabel.isHidden = false
        } else if let contentConflict {
          self.conflictLabel.stringValue =
            "Similar content exists in “\(contentConflict.title)” — review before saving."
          self.conflictLabel.isHidden = false
        } else {
          self.conflictLabel.isHidden = true
          self.conflictLabel.stringValue = ""
        }
      }
    }
  }

  @objc private func save() {
    let title = titleField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
    let trigger = triggerField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
    let content = bodyTextView.string.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !title.isEmpty, !trigger.isEmpty, !content.isEmpty else {
      if title.isEmpty {
        titleField.becomeFirstResponder()
      } else if trigger.isEmpty {
        triggerField.becomeFirstResponder()
      } else {
        bodyTextView.window?.makeFirstResponder(bodyTextView)
      }
      return
    }
    if !conflictLabel.isHidden, conflictLabel.stringValue.contains("Trigger") {
      LauncherEnvironment.current?.showStatus(LauncherStatusMessages.snippetTriggerTaken)
      triggerField.becomeFirstResponder()
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
