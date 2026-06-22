import Foundation
import LumaCore

public struct Snippet: Sendable, Codable, Hashable, Identifiable {
    public let id: UUID
    public var title: String
    public var content: String
    public var tags: [String]
    public var usageCount: Int
    public var lastUsedAt: Date
    public var createdAt: Date

    public init(
        id: UUID = UUID(),
        title: String,
        content: String,
        tags: [String] = [],
        usageCount: Int = 0,
        lastUsedAt: Date = Date(),
        createdAt: Date = Date()
    ) {
        self.id = id
        self.title = title
        self.content = content
        self.tags = tags.map { $0.lowercased().trimmingCharacters(in: .whitespacesAndNewlines) }.filter { !$0.isEmpty }
        self.usageCount = usageCount
        self.lastUsedAt = lastUsedAt
        self.createdAt = createdAt
    }
}

public enum SnippetsStoreError: Error {
    case notFound
    case writeFailed
}

public actor SnippetsStore {
    private let backing: JSONFileStore<Snippet>

    public init(fileManager: FileManager = .default, persistenceURL: URL? = nil) {
        let url: URL
        if let persistenceURL {
            url = persistenceURL
        } else {
            let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
                ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
            url = base.appendingPathComponent("Luma/snippets.json")
        }
        self.backing = JSONFileStore(url: url)
    }

    public func all() async -> [Snippet] {
        await backing.items
    }

    public func add(title: String, content: String, tags: [String]) async throws -> Snippet {
        let snippet = Snippet(title: title, content: content, tags: StringNormalization.tags(tags))
        do {
            try await backing.mutate { items in
                items.append(snippet)
            }
        } catch JSONFileStoreError.writeFailed {
            throw SnippetsStoreError.writeFailed
        }
        return snippet
    }

    public func update(_ snippet: Snippet) async throws -> Snippet {
        var updated = snippet
        updated.tags = StringNormalization.tags(snippet.tags)
        do {
            try await backing.mutate { items in
                guard let index = items.firstIndex(where: { $0.id == snippet.id }) else {
                    throw SnippetsStoreError.notFound
                }
                items[index] = updated
            }
        } catch let error as SnippetsStoreError {
            throw error
        } catch JSONFileStoreError.writeFailed {
            throw SnippetsStoreError.writeFailed
        }
        return updated
    }

    public func delete(id: UUID) async throws {
        do {
            try await backing.mutate { items in
                guard items.contains(where: { $0.id == id }) else { throw SnippetsStoreError.notFound }
                items.removeAll { $0.id == id }
            }
        } catch let error as SnippetsStoreError {
            throw error
        } catch JSONFileStoreError.writeFailed {
            throw SnippetsStoreError.writeFailed
        }
    }

    public func duplicate(id: UUID) async throws -> Snippet {
        guard let source = await backing.items.first(where: { $0.id == id }) else {
            throw SnippetsStoreError.notFound
        }
        let copy = Snippet(title: source.title + " Copy", content: source.content, tags: source.tags)
        do {
            try await backing.mutate { items in
                items.append(copy)
            }
        } catch JSONFileStoreError.writeFailed {
            throw SnippetsStoreError.writeFailed
        }
        return copy
    }

    public func recordUsage(id: UUID, at: Date = Date()) async throws -> Snippet {
        var result: Snippet?
        do {
            try await backing.mutateBuffered { items in
                guard let index = items.firstIndex(where: { $0.id == id }) else {
                    throw SnippetsStoreError.notFound
                }
                items[index].usageCount += 1
                items[index].lastUsedAt = at
                result = items[index]
            }
        } catch let error as SnippetsStoreError {
            throw error
        } catch JSONFileStoreError.writeFailed {
            throw SnippetsStoreError.writeFailed
        }
        guard let result else { throw SnippetsStoreError.notFound }
        return result
    }

    public func flush() async throws {
        do {
            try await backing.flushIfNeeded()
        } catch JSONFileStoreError.writeFailed {
            throw SnippetsStoreError.writeFailed
        }
    }

    public func persistencePath() async -> URL {
        await backing.persistencePath()
    }
}
