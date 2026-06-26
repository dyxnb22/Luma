import Foundation

public enum NotesAction: Codable, Sendable {
    case open(path: String)
    case createInInbox(title: String)
    case createFromTemplate(template: String, title: String)
    case openOrCreateDaily
    case createWeeklyReview
    case captureToDaily(text: String)
}
