import Foundation

public struct QuicklinksIndex: Sendable {
    private let byTrigger: [String: Quicklink]

    public init(quicklinks: [Quicklink]) {
        var map: [String: Quicklink] = [:]
        for quicklink in quicklinks {
            let normalized = QuicklinksStore.normalized(quicklink)
            guard !normalized.trigger.isEmpty, map[normalized.trigger] == nil else { continue }
            map[normalized.trigger] = normalized
        }
        self.byTrigger = map
    }

    public func match(raw: String) -> (quicklink: Quicklink, query: String)? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }
        let parts = trimmed.split(separator: " ", maxSplits: 1, omittingEmptySubsequences: false)
        guard let first = parts.first else { return nil }
        let trigger = String(first).lowercased()
        guard let quicklink = byTrigger[trigger] else { return nil }
        let query = parts.count > 1 ? String(parts[1]).trimmingCharacters(in: .whitespacesAndNewlines) : ""
        return (quicklink, query)
    }
}

public enum QuicklinkTemplateRenderer {
    private static let variablePattern = #"\{\{[^}]+\}\}"#

    public static func render(
        template: String,
        query: String,
        clipboard: String? = nil,
        selection: String? = nil,
        project: String? = nil,
        projectPath: String? = nil,
        filename: String? = nil,
        now: Date = Date()
    ) -> String {
        let context = SnippetExpansionContext(
            queryText: query,
            clipboardText: clipboard,
            selectionText: selection,
            projectName: project,
            projectPath: projectPath,
            filename: filename,
            now: now
        )
        guard let regex = try? NSRegularExpression(pattern: variablePattern) else { return template }
        var result = template
        let range = NSRange(result.startIndex..<result.endIndex, in: result)
        for match in regex.matches(in: result, range: range).reversed() {
            guard let fullRange = Range(match.range, in: result) else { continue }
            let token = String(result[fullRange])
            let expanded = SnippetVariableExpander.expand(token, context: context)
            let encoded = expanded.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? expanded
            result.replaceSubrange(fullRange, with: encoded)
        }
        return result
    }
}
