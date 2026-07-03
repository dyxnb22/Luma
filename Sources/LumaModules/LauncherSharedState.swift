import Foundation
import LumaCore

/// Cross-module mutable launcher state consumed by detail views in LumaApp.
@MainActor
public enum LauncherSharedState {
    public static var pendingMediaEditorDraft: MediaEditorDraft?
    public static var pendingWordbookAutoStartReview = false
    public static var pendingCurrentProjectContext: CurrentProjectContext?
    public static var pendingProjectsManage = false
    public static var pendingSnippetDraft: SnippetDraft?
    public static var pendingQuicklinkDraft: URLQuicklinkDraft?
}
