import Foundation

public enum QuicklinksAction: Codable, Sendable, Hashable {
    case open(id: UUID, query: String, bundleID: String?)
    case copy(id: UUID, query: String)
    case revealConfig
    case prepareDraft(URLQuicklinkDraft)
}
