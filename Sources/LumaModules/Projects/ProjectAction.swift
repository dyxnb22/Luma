import Foundation

public enum ProjectAction: Codable, Sendable, Hashable {
    case open(path: String, opener: ProjectOpener)
    case copyPath(String)
    case reveal(String)
    case revealConfig
}
