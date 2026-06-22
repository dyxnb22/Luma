import Foundation

public enum NotesAction: Codable, Sendable {
    case open(path: String)
}
