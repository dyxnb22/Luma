import Foundation

/// Shared TTL for in-memory due-list caches (Todo, Wordbook).
public enum CacheTTL {
    public static let dueListSeconds: TimeInterval = 30
}
