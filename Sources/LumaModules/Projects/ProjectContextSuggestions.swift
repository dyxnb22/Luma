import Foundation
import LumaCore

/// Builds cross-module create rows from the current project context.
public enum ProjectContextSuggestions {
    public static func projectSlug(for context: CurrentProjectContext) -> String {
        let name = context.projectName ?? context.projectLabel
        let filtered = name.lowercased().filter { $0.isLetter || $0.isNumber }
        let slug = String(filtered.prefix(20))
        return slug.isEmpty ? "project" : slug
    }

    public static func snippetDraft(for context: CurrentProjectContext) -> SnippetDraft {
        let name = context.projectName ?? context.projectLabel
        let slug = projectSlug(for: context)
        return SnippetDraft(
            title: "\(name) snippet",
            trigger: ";\(slug)",
            content: "Project: {{project}}\nPath: {{projectPath}}\n",
            tags: [slug, "project"]
        )
    }

    public static func quicklinkDraft(for context: CurrentProjectContext) -> URLQuicklinkDraft? {
        ProjectQuicklinkDraftSource(context: context).quicklinkDraft()
    }

    public static func todoDraft(for context: CurrentProjectContext, text: String = "") -> String {
        let slug = projectSlug(for: context)
        let name = context.projectName ?? context.projectLabel
        let prefix = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if prefix.isEmpty {
            return "\(name) task"
        }
        return prefix.hasPrefix("#\(slug)") ? prefix : "#\(slug) \(prefix)"
    }

    public static func noteCaptureText(for context: CurrentProjectContext, text: String = "") -> String {
        let name = context.projectName ?? context.projectLabel
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            return "Project: \(name)"
        }
        return "Project: \(name)\n\(trimmed)"
    }
}
