import Foundation

public enum SnippetVariableExpander {
    public static func expand(_ content: String, clipboardText: String? = nil, now: Date = Date()) -> String {
        let dateFormatter = DateFormatter()
        dateFormatter.dateStyle = .medium
        dateFormatter.timeStyle = .none
        let dateString = dateFormatter.string(from: now)

        var result = content
        result = result.replacingOccurrences(of: "{{date}}", with: dateString)
        result = result.replacingOccurrences(of: "{{clipboard}}", with: clipboardText ?? "")
        result = result.replacingOccurrences(of: "{{cursor}}", with: "")
        return result
    }
}
