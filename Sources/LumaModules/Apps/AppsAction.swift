import Foundation

public enum AppsAction: Codable, Sendable {
    case quit(bundleID: String)
}
