import Foundation
import LumaCore

public enum MediaStoreError: Error {
    case notFound
    case writeFailed
    case invalidInput
}

public actor MediaStore {
    private let backing: JSONFileStore<MediaItem>

    public init(fileManager: FileManager = .default, persistenceURL: URL? = nil) {
        let url: URL
        if let persistenceURL {
            url = persistenceURL
        } else {
            let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
                ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
            url = base.appendingPathComponent("Luma/Media/media.json")
        }
        self.backing = JSONFileStore(url: url)
    }

    public func all() async -> [MediaItem] {
        await backing.items
    }

    public func add(from draft: MediaEditorDraft, now: Date = Date()) async throws -> MediaItem {
        guard let category = draft.category, !draft.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            throw MediaStoreError.invalidInput
        }
        let item = MediaItem(
            title: draft.title.trimmingCharacters(in: .whitespacesAndNewlines),
            category: category,
            status: draft.status,
            rating: draft.rating,
            startedAt: draft.startedAt,
            completedAt: draft.completedAt ?? (draft.status == .done ? now : nil),
            notes: draft.notes,
            tags: StringNormalization.tags(draft.tags),
            createdAt: now,
            updatedAt: now
        )
        do {
            try await backing.mutate { items in
                items.append(item)
            }
        } catch JSONFileStoreError.writeFailed {
            throw MediaStoreError.writeFailed
        }
        return item
    }

    public func update(from draft: MediaEditorDraft, now: Date = Date()) async throws -> MediaItem {
        guard let id = draft.existingID else { throw MediaStoreError.notFound }
        guard let category = draft.category else { throw MediaStoreError.invalidInput }
        var updated: MediaItem?
        do {
            try await backing.mutate { items in
                guard let index = items.firstIndex(where: { $0.id == id }) else {
                    throw MediaStoreError.notFound
                }
                var item = items[index]
                item.title = draft.title.trimmingCharacters(in: .whitespacesAndNewlines)
                item.category = category
                item.status = draft.status
                item.rating = draft.rating
                item.startedAt = draft.startedAt
                if draft.status == .done, item.completedAt == nil {
                    item.completedAt = draft.completedAt ?? now
                } else {
                    item.completedAt = draft.completedAt
                }
                item.notes = String(draft.notes.prefix(2000))
                item.tags = StringNormalization.tags(draft.tags)
                item.updatedAt = now
                items[index] = item
                updated = item
            }
        } catch let error as MediaStoreError {
            throw error
        } catch JSONFileStoreError.writeFailed {
            throw MediaStoreError.writeFailed
        }
        guard let updated else { throw MediaStoreError.notFound }
        return updated
    }

    public func delete(id: UUID) async throws {
        do {
            try await backing.mutate { items in
                guard items.contains(where: { $0.id == id }) else { throw MediaStoreError.notFound }
                items.removeAll { $0.id == id }
            }
        } catch let error as MediaStoreError {
            throw error
        } catch JSONFileStoreError.writeFailed {
            throw MediaStoreError.writeFailed
        }
    }

    public func save(_ item: MediaItem) async throws {
        do {
            try await backing.mutate { items in
                if let index = items.firstIndex(where: { $0.id == item.id }) {
                    items[index] = item
                } else {
                    items.append(item)
                }
            }
        } catch JSONFileStoreError.writeFailed {
            throw MediaStoreError.writeFailed
        }
    }

    public func persistencePath() async -> URL {
        await backing.persistencePath()
    }
}
