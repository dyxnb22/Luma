import Foundation
import LumaServices

public struct NotesTemplateInfo: Sendable, Equatable {
    public let name: String
    public let url: URL

    public init(name: String, url: URL) {
        self.name = name
        self.url = url
    }
}

public enum NotesTemplateStore {
    public static let dailyFallbackBody = "# {{date}}\n\n"

    public static func scanTemplates(root: URL, folderName: String, fileManager: FileManager = .default) -> [NotesTemplateInfo] {
        let folder = root.appendingPathComponent(folderName, isDirectory: true)
        guard let entries = try? fileManager.contentsOfDirectory(
            at: folder,
            includingPropertiesForKeys: nil,
            options: [.skipsHiddenFiles]
        ) else {
            return []
        }

        return entries
            .filter { $0.pathExtension.compare("md", options: .caseInsensitive) == .orderedSame }
            .map { url in
                NotesTemplateInfo(
                    name: url.deletingPathExtension().lastPathComponent.lowercased(),
                    url: url
                )
            }
            .sorted { $0.name < $1.name }
    }

    public static func templateNames(from templates: [NotesTemplateInfo]) -> Set<String> {
        Set(templates.map(\.name))
    }

    public static func template(named name: String, in templates: [NotesTemplateInfo]) -> NotesTemplateInfo? {
        let key = name.lowercased()
        return templates.first { $0.name == key }
    }

    public static func renderTemplate(
        at url: URL,
        title: String,
        now: Date = Date()
    ) throws -> String {
        let raw = try String(contentsOf: url, encoding: .utf8)
        return TemplateRenderer.render(raw, title: title, now: now)
    }
}
