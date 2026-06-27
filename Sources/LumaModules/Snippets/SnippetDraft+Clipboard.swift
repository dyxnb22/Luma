import Foundation

public extension SnippetDraft {
    /// Builds a snippet editor draft from clipboard text with a sensible title and trigger.
    static func fromClipboard(_ text: String) -> SnippetDraft {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        let firstLine = trimmed
            .split(separator: "\n", maxSplits: 1, omittingEmptySubsequences: true)
            .first
            .map(String.init)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let title = firstLine.isEmpty ? "Clipboard clip" : String(firstLine.prefix(40))
        let trigger = Snippet(title: title, content: trimmed).displayTrigger
        return SnippetDraft(title: title, trigger: trigger, content: text, tags: ["clipboard"])
    }

    static let clipboardContentPreviewLimit = 2_000

    var isLongClipboardClip: Bool {
        content.count > Self.clipboardContentPreviewLimit
    }
}
