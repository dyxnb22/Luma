import Foundation
import LumaCore

public enum ProjectAction: Codable, Sendable, Hashable {
    case open(path: String, opener: ProjectOpener)
    case copyPath(String)
    case reveal(String)
    case revealConfig
    case openCurrentDetail(CurrentProjectContext)
    case openManage
    case openTerminal(path: String)
    case openNotes(path: String, projectName: String)
    case togglePin(path: String)
    case updateAliases(path: String, aliases: [String])
    case updateOpener(path: String, opener: ProjectOpener)
    case addRoot(String)
    case addManualProject(name: String, path: String)
}

public extension ProjectAction {
  var hidesLauncher: Bool {
    switch self {
    case .togglePin, .updateAliases, .updateOpener, .addRoot, .addManualProject,
         .openManage, .openCurrentDetail, .revealConfig:
      return false
    case .open, .copyPath, .reveal, .openTerminal, .openNotes:
      return true
    }
  }
}
