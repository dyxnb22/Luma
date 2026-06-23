import AppKit
import LumaModules

@MainActor
final class WordbookWordEditorSheet: NSWindow {
    private let onSave: (WordEntry) -> Void
    private let termField = NSTextField()
    private let phoneticField = NSTextField()
    private let meaningField = NSTextField()
    private let exampleField = NSTextField()
    private let categoryField = NSTextField()
    private let ipaButton = NSButton(title: "Suggest IPA", target: nil, action: nil)
    private let existing: WordEntry?
    private var lookupTask: Task<Void, Never>?

    init(entry: WordEntry?, onSave: @escaping (WordEntry) -> Void) {
        self.existing = entry
        self.onSave = onSave
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 420, height: 360),
            styleMask: [.titled, .closable],
            backing: .buffered,
            defer: false
        )
        title = entry == nil ? "Add Word" : "Edit Word"
        setup(entry: entry)
    }

    private func setup(entry: WordEntry?) {
        let container = NSView(frame: NSRect(x: 0, y: 0, width: 420, height: 360))
        termField.stringValue = entry?.term ?? ""
        termField.placeholderString = "Term"
        termField.translatesAutoresizingMaskIntoConstraints = false

        phoneticField.stringValue = entry?.phonetic ?? ""
        phoneticField.placeholderString = "Phonetic (IPA)"
        phoneticField.translatesAutoresizingMaskIntoConstraints = false

        ipaButton.bezelStyle = .rounded
        ipaButton.target = self
        ipaButton.action = #selector(suggestIPA)
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

        container.addSubview(termField)
        container.addSubview(phoneticField)
        container.addSubview(ipaButton)
        container.addSubview(meaningField)
        container.addSubview(exampleField)
        container.addSubview(categoryField)

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
            ipaButton.widthAnchor.constraint(equalToConstant: 96),

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
            categoryField.heightAnchor.constraint(equalToConstant: 24)
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

    @objc private func save() {
        let term = termField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !term.isEmpty else { return }
        let updated = WordEntry(
            id: existing?.id ?? 0,
            term: term,
            phonetic: phoneticField.stringValue,
            meaning: meaningField.stringValue,
            example: exampleField.stringValue,
            category: categoryField.stringValue,
            familiarity: existing?.familiarity ?? "new",
            reviewStage: existing?.reviewStage ?? 0,
            reviewCount: existing?.reviewCount ?? 0,
            wrongCount: existing?.wrongCount ?? 0,
            nextReviewAt: existing?.nextReviewAt ?? WordbookDateFormat.iso(Date())
        )
        onSave(updated)
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
            let (data, response) = try await URLSession.shared.data(from: url)
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
