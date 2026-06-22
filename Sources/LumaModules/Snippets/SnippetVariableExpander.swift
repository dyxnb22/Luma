import AppKit
import Foundation

public enum SnippetVariableExpander {
    public static func expand(_ content: String, now: Date = Date()) -> String {
        let dateFormatter = DateFormatter()
        dateFormatter.dateStyle = .medium
        dateFormatter.timeStyle = .none
        let dateString = dateFormatter.string(from: now)

        var result = content
        result = result.replacingOccurrences(of: "{{date}}", with: dateString)
        result = result.replacingOccurrences(of: "{{clipboard}}", with: NSPasteboard.general.string(forType: .string) ?? "")
        result = result.replacingOccurrences(of: "{{cursor}}", with: "")
        return result
    }
}
