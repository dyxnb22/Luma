import Foundation

/// Persisted in-progress launcher and workbench workflows.
public struct LauncherResumeState: Codable, Sendable, Equatable {
  public var moduleRaw: String?
  public var query: String
  public var translateSource: String
  public var translateOutput: String
  public var snippetDraftJSON: Data?
  public var quicklinkDraftJSON: Data?
  public var todoCaptureText: String?

  public init(
    moduleRaw: String? = nil,
    query: String = "",
    translateSource: String = "",
    translateOutput: String = "",
    snippetDraftJSON: Data? = nil,
    quicklinkDraftJSON: Data? = nil,
    todoCaptureText: String? = nil
  ) {
    self.moduleRaw = moduleRaw
    self.query = query
    self.translateSource = translateSource
    self.translateOutput = translateOutput
    self.snippetDraftJSON = snippetDraftJSON
    self.quicklinkDraftJSON = quicklinkDraftJSON
    self.todoCaptureText = todoCaptureText
  }
}

public enum LauncherResumeStore {
  private static let fileName = "launcher-resume.json"

  public static func load(from url: URL = defaultURL()) -> LauncherResumeState {
    guard let data = try? Data(contentsOf: url),
          let state = try? JSONDecoder().decode(LauncherResumeState.self, from: data) else {
      return LauncherResumeState()
    }
    return state
  }

  public static func save(_ state: LauncherResumeState, to url: URL = defaultURL()) {
    let dir = url.deletingLastPathComponent()
    try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    guard let data = try? JSONEncoder().encode(state) else { return }
    try? data.write(to: url, options: .atomic)
  }

  public static func defaultURL(fileManager: FileManager = .default) -> URL {
    let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
    return base.appendingPathComponent("Luma/\(fileName)")
  }
}
