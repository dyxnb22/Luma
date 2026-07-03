import Foundation

/// Snapshot of user context for workbench capture and command flows.
public struct WorkbenchContext: Sendable {
    public let selectionText: String?
    public let clipboardPreview: String?
    public let clipboardURL: URL?
    public let frontmostAppName: String?
    public let currentProject: CurrentProjectContext?
    public let enabledModuleIDs: Set<ModuleIdentifier>
    public let pinnedModuleIDs: Set<ModuleIdentifier>
    public let activitySnapshot: WorkbenchActivitySnapshot
    public let linkSnapshot: WorkbenchLinkSnapshot
    public let projectIndexCounts: WorkbenchProjectIndexCounts?

    public init(
        selectionText: String? = nil,
        clipboardPreview: String? = nil,
        clipboardURL: URL? = nil,
        frontmostAppName: String? = nil,
        currentProject: CurrentProjectContext? = nil,
        enabledModuleIDs: Set<ModuleIdentifier>,
        pinnedModuleIDs: Set<ModuleIdentifier>,
        activitySnapshot: WorkbenchActivitySnapshot = WorkbenchActivitySnapshot(),
        linkSnapshot: WorkbenchLinkSnapshot = WorkbenchLinkSnapshot(),
        projectIndexCounts: WorkbenchProjectIndexCounts? = nil
    ) {
        self.selectionText = selectionText
        self.clipboardPreview = clipboardPreview
        self.clipboardURL = clipboardURL
        self.frontmostAppName = frontmostAppName
        self.currentProject = currentProject
        self.enabledModuleIDs = enabledModuleIDs
        self.pinnedModuleIDs = pinnedModuleIDs
        self.activitySnapshot = activitySnapshot
        self.linkSnapshot = linkSnapshot
        self.projectIndexCounts = projectIndexCounts
    }

    /// Global recent activities (top N across all projects).
    public var recentActivities: [WorkbenchActivityEntry] {
        activitySnapshot.globalRecent
    }

    public func isEnabled(_ id: ModuleIdentifier) -> Bool {
        enabledModuleIDs.contains(id)
    }

    public func isHot(_ id: ModuleIdentifier) -> Bool {
        enabledModuleIDs.contains(id) && pinnedModuleIDs.contains(id)
    }
}
