import Foundation

public enum QuicklinksAction: Codable, Sendable, Hashable {
    case open(url: String, bundleID: String?)
    case copy(url: String)
    case revealConfig
}
