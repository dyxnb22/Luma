import Foundation

public enum EventsAction: Codable, Sendable {
    case create(title: String, start: TimeInterval, end: TimeInterval)
    case grant
}
