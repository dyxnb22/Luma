import Foundation

public actor QuicklinksStore {
    private let url: URL
    private let fileManager: FileManager
    private var quicklinks: [Quicklink] = []

    public init(url: URL = QuicklinksStore.defaultURL(), fileManager: FileManager = .default) {
        self.url = url
        self.fileManager = fileManager
        try? fileManager.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        quicklinks = Self.load(from: url, fileManager: fileManager)
        if quicklinks.isEmpty, !fileManager.fileExists(atPath: url.path) {
            quicklinks = Self.defaultQuicklinks()
            try? Self.persist(quicklinks, to: url, fileManager: fileManager)
        }
    }

    public func all() -> [Quicklink] {
        quicklinks
    }

    public func configFileURL() -> URL {
        url
    }

    @discardableResult
    public func add(_ quicklink: Quicklink) throws -> Quicklink {
        var normalized = Self.normalized(quicklink)
        if normalized.name.isEmpty { normalized.name = normalized.trigger.uppercased() }
        quicklinks.append(normalized)
        try Self.persist(quicklinks, to: url, fileManager: fileManager)
        return normalized
    }

    @discardableResult
    public func update(_ quicklink: Quicklink) throws -> Quicklink {
        let normalized = Self.normalized(quicklink)
        guard let index = quicklinks.firstIndex(where: { $0.id == quicklink.id }) else {
            quicklinks.append(normalized)
            try Self.persist(quicklinks, to: url, fileManager: fileManager)
            return normalized
        }
        quicklinks[index] = normalized
        try Self.persist(quicklinks, to: url, fileManager: fileManager)
        return normalized
    }

    public func delete(id: UUID) throws {
        quicklinks.removeAll { $0.id == id }
        try Self.persist(quicklinks, to: url, fileManager: fileManager)
    }

    public func conflictingQuicklink(trigger: String, excluding id: UUID? = nil) -> Quicklink? {
        let normalizedTrigger = Self.normalized(
            Quicklink(name: "", trigger: trigger, urlTemplate: "https://example.com")
        ).trigger
        guard !normalizedTrigger.isEmpty else { return nil }
        return quicklinks.first { link in
            link.trigger == normalizedTrigger && link.id != id
        }
    }

    public func duplicateQuicklink(urlTemplate: String, excluding id: UUID? = nil) -> Quicklink? {
        let normalized = Self.normalizedURLKey(urlTemplate)
        guard !normalized.isEmpty else { return nil }
        return quicklinks.first { link in
            link.id != id && Self.normalizedURLKey(link.urlTemplate) == normalized
        }
    }

    public static func normalizedURLKey(_ template: String) -> String {
        let trimmed = template.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard var components = URLComponents(string: trimmed.hasPrefix("http") ? trimmed : "https://\(trimmed)") else {
            return trimmed
        }
        components.fragment = nil
        if components.path == "/" { components.path = "" }
        return components.string?.lowercased() ?? trimmed
    }

    public static func validateURLTemplate(_ template: String) -> String? {
        let trimmed = template.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return "URL template is empty" }
        if !trimmed.contains("{{") {
            if let url = URL(string: trimmed), url.scheme?.isEmpty == false { return nil }
            if trimmed.hasPrefix("http://") || trimmed.hasPrefix("https://") {
                return "URL doesn't look valid"
            }
            return "URL needs http:// or https://"
        }
        let sample = QuicklinkTemplateRenderer.render(template: trimmed, query: "test")
        guard let url = URL(string: sample), let scheme = url.scheme, !scheme.isEmpty else {
            return "Rendered URL doesn't look valid"
        }
        return nil
    }

    public static func defaultURL() -> URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support/Luma/quicklinks.json")
    }

    public static func defaultQuicklinks() -> [Quicklink] {
        [
            Quicklink(name: "GitHub Search", trigger: "gh", urlTemplate: "https://github.com/search?q={{query}}&type=repositories"),
            Quicklink(name: "Google", trigger: "g", urlTemplate: "https://www.google.com/search?q={{query}}"),
            Quicklink(name: "Apple Developer", trigger: "swift", urlTemplate: "https://developer.apple.com/search/?q={{query}}")
        ]
    }

    public static func normalized(_ quicklink: Quicklink) -> Quicklink {
        var copy = quicklink
        copy.trigger = copy.trigger
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .split(whereSeparator: { $0.isWhitespace })
            .first
            .map(String.init) ?? ""
        copy.name = copy.name.trimmingCharacters(in: .whitespacesAndNewlines)
        copy.urlTemplate = copy.urlTemplate.trimmingCharacters(in: .whitespacesAndNewlines)
        copy.openWith = copy.openWith?.trimmingCharacters(in: .whitespacesAndNewlines)
        if copy.openWith?.isEmpty == true { copy.openWith = nil }
        copy.icon = copy.icon?.trimmingCharacters(in: .whitespacesAndNewlines)
        if copy.icon?.isEmpty == true { copy.icon = nil }
        return copy
    }

    private static func load(from url: URL, fileManager: FileManager) -> [Quicklink] {
        guard fileManager.fileExists(atPath: url.path),
              let data = try? Data(contentsOf: url) else { return [] }
        do {
            return try JSONDecoder().decode([Quicklink].self, from: data).map(normalized)
        } catch {
            quarantineCorruptFile(at: url, fileManager: fileManager)
            return []
        }
    }

    private static func persist(_ quicklinks: [Quicklink], to url: URL, fileManager: FileManager) throws {
        try fileManager.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(quicklinks)
        try data.write(to: url, options: .atomic)
    }

    private static func quarantineCorruptFile(at url: URL, fileManager: FileManager) {
        let quarantine = url.deletingPathExtension().appendingPathExtension("corrupt-\(Int(Date().timeIntervalSince1970)).json")
        try? fileManager.moveItem(at: url, to: quarantine)
    }
}
