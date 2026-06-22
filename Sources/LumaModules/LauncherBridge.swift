import Foundation

/// Unified main-actor callbacks from actor-isolated modules and detail views back to `LumaApp`.
///
/// See `docs/ARCHITECTURE_BRIDGES.md` for the split with `ModuleDetailRegistry`.
@MainActor
public enum LauncherBridge {
    // MARK: Module actor → UI

    nonisolated(unsafe) public static var onSecretsLockStateChanged: ((Bool) -> Void)?
    nonisolated(unsafe) public static var reloadSecretsDetail: (() -> Void)?
    nonisolated(unsafe) public static var reloadSnippetsDetail: (() -> Void)?
    nonisolated(unsafe) public static var openWordbookReview: (() -> Void)?
    nonisolated(unsafe) public static var openMediaDetail: (() -> Void)?
    nonisolated(unsafe) public static var reloadMediaDetail: (() -> Void)?
    nonisolated(unsafe) public static var pendingMediaEditorDraft: MediaEditorDraft?

    // MARK: Detail view navigation (injected at startup)

    nonisolated(unsafe) public static var onBackFromDetail: (() -> Void)?
    nonisolated(unsafe) public static var onOpenSettings: (() -> Void)?
    nonisolated(unsafe) public static var onTranslateContentChanged: ((String, String) -> Void)?
}
