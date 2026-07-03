import Foundation
import LumaCore

/// Ephemeral detail-open payloads staged between launcher action dispatch and detail `activate()`.
/// Lives in `LumaModules` so capture helpers can set drafts without importing `LumaApp`.
@MainActor
public enum LauncherSharedState {
    public static var pendingMediaEditorDraft: MediaEditorDraft?
    public static var pendingWordbookAutoStartReview = false
    public static var pendingCurrentProjectContext: CurrentProjectContext?
    public static var pendingProjectsManage = false
    public static var pendingSnippetDraft: SnippetDraft?
    public static var pendingQuicklinkDraft: URLQuicklinkDraft?
}
