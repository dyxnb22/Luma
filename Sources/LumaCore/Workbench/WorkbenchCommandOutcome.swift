import Foundation

/// Follow-up from executing a workbench command route (Return on bare command or capture).
public enum WorkbenchCommandOutcome: Sendable {
    case notHandled
    case status(String)
    case openDetail(ModuleIdentifier, payload: Data?)
    case replaceQuery(String)
    case runAction(ActionKind)
    case resumeActivity(UUID)
    case openLinked(UUID)
    case openActivityEntry(UUID)
}
