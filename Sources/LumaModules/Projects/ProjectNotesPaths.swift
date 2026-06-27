import Foundation

public enum ProjectNotesPaths {
  public static func candidates(projectPath: String, projectName: String) -> [URL] {
    [
      URL(fileURLWithPath: projectPath).appendingPathComponent("NOTES.md"),
      URL(fileURLWithPath: projectPath).appendingPathComponent("notes/README.md"),
      URL(fileURLWithPath: projectPath).appendingPathComponent("README.md"),
      FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Documents/Notes/\(projectName).md")
    ]
  }

  public static func existingNotePath(projectPath: String, projectName: String) -> String? {
    candidates(projectPath: projectPath, projectName: projectName)
      .first { FileManager.default.fileExists(atPath: $0.path) }?
      .path
  }
}
