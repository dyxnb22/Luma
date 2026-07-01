import Foundation

public enum SnippetDisplay {
    public static func disambiguatedTitle(_ snippet: Snippet, among snippets: [Snippet]) -> String {
        let normalized = snippet.title.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !normalized.isEmpty else { return snippet.title }
        let duplicates = snippets.filter {
            $0.title.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() == normalized
        }.count
        guard duplicates > 1 else { return snippet.title }
        return "\(snippet.title) · \(snippet.displayTrigger)"
    }

    public static func contentPreview(_ snippet: Snippet, maxLength: Int = 56) -> String {
        let oneLine = snippet.content
            .replacingOccurrences(of: "\n", with: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard !oneLine.isEmpty else { return "Empty snippet" }
        if oneLine.count <= maxLength { return oneLine }
        return String(oneLine.prefix(maxLength)) + "…"
    }

    public static func rowToolTip(_ snippet: Snippet) -> String {
        var parts = [snippet.title, "Trigger: \(snippet.displayTrigger)"]
        if !snippet.tags.isEmpty {
            parts.append("Tags: \(snippet.tags.joined(separator: ", "))")
        }
        parts.append(SnippetDisplay.contentPreview(snippet, maxLength: 240))
        return parts.joined(separator: "\n")
    }
}
