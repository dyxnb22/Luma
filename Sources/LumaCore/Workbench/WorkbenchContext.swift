import Foundation

/// Reference to a pending draft surfaced on Home or Resume.
public struct WorkbenchDraftRef: Sendable, Hashable, Codable {
    public let target: WorkbenchCaptureTarget
    public let moduleID: ModuleIdentifier
    public let preview: String

    public init(target: WorkbenchCaptureTarget, moduleID: ModuleIdentifier, preview: String) {
        self.target = target
        self.moduleID = moduleID
        self.preview = preview
    }
}

/// Snapshot of user context for Home, Capture, and Command flows.
public struct WorkbenchContext: Sendable {
    public let selectionText: String?
    public let clipboardPreview: String?
    public let clipboardURL: URL?
    public let frontmostAppName: String?
    public let currentProject: CurrentProjectContext?
    public let pendingDrafts: [WorkbenchDraftRef]
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
        pendingDrafts: [WorkbenchDraftRef] = [],
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
        self.pendingDrafts = pendingDrafts
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
