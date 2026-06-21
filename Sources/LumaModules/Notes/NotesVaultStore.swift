import AppKit
import Foundation

public struct NoteFile: Sendable, Hashable {
    public let url: URL
    public let title: String
    public let body: String
}

public actor NotesVaultStore {
    private let fileManager: FileManager
    private var vaultURL: URL

    public init(vaultURL: URL, fileManager: FileManager = .default) {
        self.vaultURL = vaultURL
        self.fileManager = fileManager
    }

    public func setVaultURL(_ url: URL) {
        vaultURL = url
    }

    public func scan() -> [NoteFile] {
        guard let enumerator = fileManager.enumerator(
            at: vaultURL,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        var notes: [NoteFile] = []
        for case let url as URL in enumerator where url.pathExtension.lowercased() == "md" {
            guard let body = try? String(contentsOf: url, encoding: .utf8) else { continue }
            notes.append(NoteFile(url: url, title: url.deletingPathExtension().lastPathComponent, body: body))
        }
        return notes.sorted { $0.title < $1.title }
    }

    public func graph() -> NotesGraph {
        let files = Dictionary(uniqueKeysWithValues: scan().map { ($0.url.path, $0.body) })
        return NotesGraphIndexer.index(files: files)
    }

    public func create(title: String, body: String = "") throws -> URL {
        try fileManager.createDirectory(at: vaultURL, withIntermediateDirectories: true)
        let sanitized = title.replacingOccurrences(of: "/", with: "-")
        let url = vaultURL.appendingPathComponent(sanitized).appendingPathExtension("md")
        if !fileManager.fileExists(atPath: url.path) {
            try body.write(to: url, atomically: true, encoding: .utf8)
        }
        return url
    }

    public func openInTypora(_ url: URL) {
        let typoraURL = URL(fileURLWithPath: "/Applications/Typora.app")
        let configuration = NSWorkspace.OpenConfiguration()
        if FileManager.default.fileExists(atPath: typoraURL.path) {
            NSWorkspace.shared.open([url], withApplicationAt: typoraURL, configuration: configuration)
        } else {
            NSWorkspace.shared.open(url)
        }
    }
}
