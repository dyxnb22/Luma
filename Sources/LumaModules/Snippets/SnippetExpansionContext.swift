import Foundation
import LumaCore

public struct SnippetExpansionContext: Sendable {
    public var queryText: String?
    public var clipboardText: String?
    public var selectionText: String?
    public var projectName: String?
    public var projectPath: String?
    public var filename: String?
    public var now: Date

    public init(
        queryText: String? = nil,
        clipboardText: String? = nil,
        selectionText: String? = nil,
        projectName: String? = nil,
        projectPath: String? = nil,
        filename: String? = nil,
        now: Date = Date()
    ) {
        self.queryText = queryText
        self.clipboardText = clipboardText
        self.selectionText = selectionText
        self.projectName = projectName
        self.projectPath = projectPath
        self.filename = filename
        self.now = now
    }

    public static func from(project: CurrentProjectContext?, clipboardText: String?, selectionText: String?) -> SnippetExpansionContext {
        SnippetExpansionContext(
            clipboardText: clipboardText,
            selectionText: selectionText,
            projectName: project?.projectName,
            projectPath: project?.projectPath,
            filename: project?.filename
        )
    }
}
