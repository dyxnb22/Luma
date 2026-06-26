import Foundation

public struct CurrentProjectContext: Sendable, Codable, Hashable {
    public let frontAppName: String
    public let bundleID: String
    public let windowTitle: String
    public let projectLabel: String
    public let filename: String?
    public let matchedProjectPath: String?
    public let matchedProjectName: String?

    public init(
        frontAppName: String,
        bundleID: String,
        windowTitle: String,
        projectLabel: String,
        filename: String?,
        matchedProjectPath: String? = nil,
        matchedProjectName: String? = nil
    ) {
        self.frontAppName = frontAppName
        self.bundleID = bundleID
        self.windowTitle = windowTitle
        self.projectLabel = projectLabel
        self.filename = filename
        self.matchedProjectPath = matchedProjectPath
        self.matchedProjectName = matchedProjectName
    }

    public var projectPath: String? { matchedProjectPath }
    public var projectName: String? { matchedProjectName ?? (projectLabel.isEmpty ? nil : projectLabel) }
}
