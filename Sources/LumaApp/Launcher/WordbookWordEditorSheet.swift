import AppKit
import LumaModules

@MainActor
final class WordbookWordEditorSheet: LumaWindow {
    private let onSave: (WordEntry, Bool) -> Void
    private let onResetStage: (() -> Void)?
    private let termField = NSTextField()
    private let phoneticField = NSTextField()
    private let meaningField = NSTextField()
    private let exampleField = NSTextField()
    private let categoryField = NSTextField()
    private let ipaButton = NSButton(title: "Suggest IPA · online", target: nil, action: nil)
    private let masteredCheckbox = NSButton(checkboxWithTitle: "Mark as mastered (已学过)", target: nil, action: nil)
    private let resetStageButton = NSButton(title: "Reset Stage", target: nil, action: nil)
    private let existing: WordEntry?
    private var lookupTask: Task<Void, Never>?

    init(
        entry: WordEntry?,
        onSave: @escaping (WordEntry, Bool) -> Void,
        onResetStage: (() -> Void)? = nil
    ) {
        self.existing = entry
        self.onSave = onSave
        self.onResetStage = onResetStage
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 420, height: 400),
            styleMask: [.titled, .closable],
            backing: .buffered,
            defer: false
        )
        title = entry == nil ? "Add Word" : "Edit Word"
        setup(entry: entry)
    }

    private func setup(entry: WordEntry?) {
        let container = NSView(frame: NSRect(x: 0, y: 0, width: 420, height: 400))
        termField.stringValue = entry?.term ?? ""
        termField.placeholderString = "Term"
        termField.translatesAutoresizingMaskIntoConstraints = false

        phoneticField.stringValue = entry?.phonetic ?? ""
        phoneticField.placeholderString = "Phonetic (IPA)"
        phoneticField.translatesAutoresizingMaskIntoConstraints = false

        ipaButton.bezelStyle = .rounded
        ipaButton.target = self
        ipaButton.action = #selector(suggestIPA)
        ipaButton.toolTip = "Fetches IPA from dictionaryapi.dev (requires network)"
        ipaButton.translatesAutoresizingMaskIntoConstraints = false

        meaningField.stringValue = entry?.meaning ?? ""
        meaningField.placeholderString = "Meaning"
        meaningField.translatesAutoresizingMaskIntoConstraints = false

        exampleField.stringValue = entry?.example ?? ""
        exampleField.placeholderString = "Example"
        exampleField.translatesAutoresizingMaskIntoConstraints = false

        categoryField.stringValue = entry?.category ?? ""
        categoryField.placeholderString = "Category"
        categoryField.translatesAutoresizingMaskIntoConstraints = false

        masteredCheckbox.state = (entry?.familiarity == "mastered") ? .on : .off
        masteredCheckbox.translatesAutoresizingMaskIntoConstraints = false

        resetStageButton.bezelStyle = .rounded
        resetStageButton.target = self
        resetStageButton.action = #selector(resetStageTapped)
        resetStageButton.isHidden = entry == nil
        resetStageButton.translatesAutoresizingMaskIntoConstraints = false

        container.addSubview(termField)
        container.addSubview(phoneticField)
        container.addSubview(ipaButton)
        container.addSubview(meaningField)
        container.addSubview(exampleField)
        container.addSubview(categoryField)
        container.addSubview(masteredCheckbox)
        container.addSubview(resetStageButton)

        NSLayoutConstraint.activate([
            termField.topAnchor.constraint(equalTo: container.topAnchor, constant: 16),
            termField.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 16),
            termField.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -16),
            termField.heightAnchor.constraint(equalToConstant: 24),

            phoneticField.topAnchor.constraint(equalTo: termField.bottomAnchor, constant: 8),
            phoneticField.leadingAnchor.constraint(equalTo: termField.leadingAnchor),
            phoneticField.trailingAnchor.constraint(equalTo: ipaButton.leadingAnchor, constant: -8),
            phoneticField.heightAnchor.constraint(equalToConstant: 24),

            ipaButton.centerYAnchor.constraint(equalTo: phoneticField.centerYAnchor),
            ipaButton.trailingAnchor.constraint(equalTo: termField.trailingAnchor),
            ipaButton.widthAnchor.constraint(equalToConstant: 140),

            meaningField.topAnchor.constraint(equalTo: phoneticField.bottomAnchor, constant: 8),
            meaningField.leadingAnchor.constraint(equalTo: termField.leadingAnchor),
            meaningField.trailingAnchor.constraint(equalTo: termField.trailingAnchor),
            meaningField.heightAnchor.constraint(equalToConstant: 24),

            exampleField.topAnchor.constraint(equalTo: meaningField.bottomAnchor, constant: 8),
            exampleField.leadingAnchor.constraint(equalTo: termField.leadingAnchor),
            exampleField.trailingAnchor.constraint(equalTo: termField.trailingAnchor),
            exampleField.heightAnchor.constraint(equalToConstant: 24),

            categoryField.topAnchor.constraint(equalTo: exampleField.bottomAnchor, constant: 8),
            categoryField.leadingAnchor.constraint(equalTo: termField.leadingAnchor),
            categoryField.trailingAnchor.constraint(equalTo: termField.trailingAnchor),
            categoryField.heightAnchor.constraint(equalToConstant: 24),

            masteredCheckbox.topAnchor.constraint(equalTo: categoryField.bottomAnchor, constant: 12),
            masteredCheckbox.leadingAnchor.constraint(equalTo: termField.leadingAnchor),

            resetStageButton.centerYAnchor.constraint(equalTo: masteredCheckbox.centerYAnchor),
            resetStageButton.trailingAnchor.constraint(equalTo: termField.trailingAnchor)
        ])

        let saveButton = NSButton(title: "Save", target: self, action: #selector(save))
        saveButton.bezelStyle = .rounded
        saveButton.keyEquivalent = "\r"
        saveButton.translatesAutoresizingMaskIntoConstraints = false
        let cancelButton = NSButton(title: "Cancel", target: self, action: #selector(cancel))
        cancelButton.bezelStyle = .rounded
        cancelButton.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(saveButton)
        container.addSubview(cancelButton)
        NSLayoutConstraint.activate([
            cancelButton.trailingAnchor.constraint(equalTo: saveButton.leadingAnchor, constant: -8),
            cancelButton.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -16),
            saveButton.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -16),
            saveButton.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -16)
        ])
        contentView = container
    }

    override func becomeKey() {
        super.becomeKey()
        makeFirstResponder(termField)
    }

    @objc private func suggestIPA() {
        let term = termField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !term.isEmpty else { return }
        ipaButton.isEnabled = false
        lookupTask?.cancel()
        lookupTask = Task { [weak self] in
            let ipa = await WordbookIPALookup.suggest(for: term)
            await MainActor.run {
                guard let self else { return }
                self.ipaButton.isEnabled = true
                if let ipa, !ipa.isEmpty {
                    self.phoneticField.stringValue = ipa
                }
            }
        }
    }

    @objc private func resetStageTapped() {
        guard existing != nil else { return }
        let alert = NSAlert()
        alert.messageText = "Reset review progress?"
        alert.informativeText = "Stage, wrong count, and next review date will be cleared."
        alert.addButton(withTitle: "Reset")
        alert.addButton(withTitle: "Cancel")
        alert.alertStyle = .warning
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        onResetStage?()
        masteredCheckbox.state = .off
        dismiss()
    }

    @objc private func save() {
        let term = termField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !term.isEmpty else { return }
        let markMastered = masteredCheckbox.state == .on
        let updated = WordEntry(
            id: existing?.id ?? 0,
            term: term,
            phonetic: phoneticField.stringValue,
            meaning: meaningField.stringValue,
            example: exampleField.stringValue,
            category: categoryField.stringValue,
            familiarity: markMastered ? "mastered" : (existing?.familiarity ?? "new"),
            reviewStage: existing?.reviewStage ?? 0,
            reviewCount: existing?.reviewCount ?? 0,
            wrongCount: existing?.wrongCount ?? 0,
            nextReviewAt: existing?.nextReviewAt ?? WordbookDateFormat.iso(Date())
        )
        onSave(updated, markMastered && existing?.familiarity != "mastered")
        dismiss()
    }

    @objc private func cancel() {
        dismiss()
    }

    private func dismiss() {
        if let parent = sheetParent {
            parent.endSheet(self)
        } else {
            close()
        }
    }
}

enum WordbookIPALookup {
    static func suggest(for term: String) async -> String? {
        guard let encoded = term.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed),
              let url = URL(string: "https://api.dictionaryapi.dev/api/v2/entries/en/\(encoded)") else {
            return nil
        }
        do {
            var request = URLRequest(url: url)
            request.timeoutInterval = 5
            let (data, response) = try await URLSession.shared.data(for: request)
            guard let http = response as? HTTPURLResponse, (200...299).contains(http.statusCode) else {
                return nil
            }
            guard let json = try JSONSerialization.jsonObject(with: data) as? [[String: Any]],
                  let first = json.first,
                  let phonetics = first["phonetics"] as? [[String: Any]] else {
                return nil
            }
            for item in phonetics {
                if let text = item["text"] as? String {
                    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
                    if !trimmed.isEmpty { return trimmed }
                }
            }
            return nil
        } catch {
            return nil
        }
    }
}
