import Foundation

public enum MediaCategory: String, Codable, CaseIterable, Sendable {
    case movie
    case tv
    case anime
    case game
    case book

    public var displayName: String {
        switch self {
        case .movie: return "Movie"
        case .tv: return "TV"
        case .anime: return "Anime"
        case .game: return "Game"
        case .book: return "Book"
        }
    }

    public var symbolName: String {
        switch self {
        case .movie: return "film"
        case .tv: return "tv"
        case .anime: return "sparkles.tv"
        case .game: return "gamecontroller"
        case .book: return "book"
        }
    }
}

public enum MediaStatus: String, Codable, CaseIterable, Sendable {
    case planned
    case inProgress
    case done
    case abandoned

    public var displayName: String {
        switch self {
        case .planned: return "Planned"
        case .inProgress: return "In Progress"
        case .done: return "Done"
        case .abandoned: return "Abandoned"
        }
    }

    public func verb(for category: MediaCategory) -> String {
        switch (self, category) {
        case (.inProgress, .book): return "Reading"
        case (.inProgress, .game): return "Playing"
        case (.inProgress, _): return "Watching"
        default: return displayName
        }
    }
}

public struct MediaItem: Sendable, Codable, Hashable, Identifiable {
    public let id: UUID
    public var title: String
    public var category: MediaCategory
    public var status: MediaStatus
    public var rating: Int?
    public var startedAt: Date?
    public var completedAt: Date?
    public var notes: String
    public var tags: [String]
    public var createdAt: Date
    public var updatedAt: Date

    public init(
        id: UUID = UUID(),
        title: String,
        category: MediaCategory,
        status: MediaStatus = .done,
        rating: Int? = nil,
        startedAt: Date? = nil,
        completedAt: Date? = nil,
        notes: String = "",
        tags: [String] = [],
        createdAt: Date = Date(),
        updatedAt: Date = Date()
    ) {
        self.id = id
        self.title = title
        self.category = category
        self.status = status
        self.rating = rating
        self.startedAt = startedAt
        self.completedAt = completedAt
        self.notes = String(notes.prefix(2000))
        self.tags = tags.map { $0.lowercased().trimmingCharacters(in: .whitespacesAndNewlines) }.filter { !$0.isEmpty }
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }
}

public struct MediaEditorDraft: Sendable, Codable, Equatable {
    public var title: String
    public var category: MediaCategory?
    public var status: MediaStatus
    public var rating: Int?
    public var startedAt: Date?
    public var completedAt: Date?
    public var notes: String
    public var tags: [String]
    public var existingID: UUID?

    public init(
        title: String,
        category: MediaCategory? = nil,
        status: MediaStatus = .done,
        rating: Int? = nil,
        startedAt: Date? = nil,
        completedAt: Date? = nil,
        notes: String = "",
        tags: [String] = [],
        existingID: UUID? = nil
    ) {
        self.title = title
        self.category = category
        self.status = status
        self.rating = rating
        self.startedAt = startedAt
        self.completedAt = completedAt
        self.notes = notes
        self.tags = tags
        self.existingID = existingID
    }

    public init(item: MediaItem) {
        self.title = item.title
        self.category = item.category
        self.status = item.status
        self.rating = item.rating
        self.startedAt = item.startedAt
        self.completedAt = item.completedAt
        self.notes = item.notes
        self.tags = item.tags
        self.existingID = item.id
    }
}
