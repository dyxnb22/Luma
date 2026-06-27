import Foundation

public enum SnippetVariableExpander {
    private static let dateFormatPattern = #"\{\{date:([^}]+)\}\}"#

    public static func expand(_ content: String, context: SnippetExpansionContext) -> String {
        var result = content

        result = result.replacingOccurrences(of: "{{uuid}}", with: UUID().uuidString)
        result = result.replacingOccurrences(of: "{{timestamp}}", with: ISO8601DateFormatter().string(from: context.now))
        result = result.replacingOccurrences(of: "{{query}}", with: context.queryText ?? "")
        result = result.replacingOccurrences(of: "{{clipboard}}", with: context.clipboardText ?? "")
        result = result.replacingOccurrences(of: "{{selection}}", with: context.selectionText ?? "")
        result = result.replacingOccurrences(of: "{{project}}", with: context.projectName ?? "")
        result = result.replacingOccurrences(of: "{{project_path}}", with: context.projectPath ?? "")
        result = result.replacingOccurrences(of: "{{file}}", with: context.filename ?? "")
        result = result.replacingOccurrences(of: "{{filename}}", with: context.filename ?? "")
        result = result.replacingOccurrences(of: "{{caret}}", with: "")
        result = result.replacingOccurrences(of: "{{cursor}}", with: "")

        let defaultFormatter = DateFormatter()
        defaultFormatter.dateStyle = .medium
        defaultFormatter.timeStyle = .none
        result = result.replacingOccurrences(of: "{{date}}", with: defaultFormatter.string(from: context.now))

        if let regex = try? NSRegularExpression(pattern: dateFormatPattern) {
            let range = NSRange(result.startIndex..<result.endIndex, in: result)
            let matches = regex.matches(in: result, range: range).reversed()
            for match in matches {
                guard let fullRange = Range(match.range, in: result),
                      let formatRange = Range(match.range(at: 1), in: result) else { continue }
                let format = String(result[formatRange])
                let formatter = DateFormatter()
                formatter.dateFormat = format
                let replacement = formatter.string(from: context.now)
                result.replaceSubrange(fullRange, with: replacement)
            }
        }

        return result
    }

    public static func expand(_ content: String, clipboardText: String? = nil, now: Date = Date()) -> String {
        expand(content, context: SnippetExpansionContext(clipboardText: clipboardText, now: now))
    }

    public static let documentedVariables: [String] = [
        "{{uuid}} — random UUID",
        "{{timestamp}} — ISO8601 timestamp",
        "{{query}} — command payload text",
        "{{date}} — medium-style date",
        "{{date:FORMAT}} — custom DateFormatter format",
        "{{clipboard}} — pasteboard text",
        "{{selection}} — selected text in frontmost app",
        "{{project}} — current IDE project name",
        "{{project_path}} — matched project path",
        "{{file}} / {{filename}} — active file in IDE title",
        "{{caret}} — leaves cursor at end after paste (no positioning)"
    ]
}
