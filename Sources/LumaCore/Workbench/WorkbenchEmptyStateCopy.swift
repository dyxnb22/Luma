import Foundation

/// Shared empty-state and status copy for workbench surfaces.
public enum WorkbenchEmptyStateCopy {
    public static let noIDEProject = "No IDE project detected."
    public static let noProjectContext = "No active project context"
    public static let noLinkedItems = "No linked items yet — capture to attach to this project"
    public static let noRecentActivity = "No project activity yet"
    public static let moduleDisabled = "Module disabled in Settings"
    public static let captureModulesDisabled = "Enable Snippets, Quicklinks, Todo, or Notes in Settings"
    public static let openProjectWorkspace = "Open project workspace"
    public static let continueProject = "Continue project"
}
