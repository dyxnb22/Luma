import Foundation
import LumaCore

enum LauncherStatusMessages {
    static var savedToDailyNote: String { L10n.tr("status.savedToDailyNote") }
    static var draftLoadedInQuicklinks: String { L10n.tr("status.draftQuicklinks") }
    static var draftLoadedInSnippets: String { L10n.tr("status.draftSnippets") }
    static var snippetCreated: String { L10n.tr("status.snippetCreated") }
    static var quicklinkSaved: String { L10n.tr("status.quicklinkSaved") }
    static var copiedToClipboard: String { L10n.tr("status.copiedToClipboard") }
    static var snippetExpanded: String { L10n.tr("status.snippetExpanded") }
    static var notesRootNotConfigured: String { L10n.tr("status.notesRootNotConfigured") }
    static var quicklinkTriggerTaken: String { L10n.tr("status.quicklinkTriggerTaken") }
    static var quicklinkURLDuplicate: String { L10n.tr("status.quicklinkURLDuplicate") }
    static var quicklinkMissingProtocol: String { L10n.tr("status.quicklinkMissingProtocol") }
    static var snippetTriggerTaken: String { L10n.tr("status.snippetTriggerTaken") }
    static var snippetDuplicateContent: String { L10n.tr("status.snippetDuplicateContent") }
    static var snippetSaveFailed: String { L10n.tr("status.snippetSaveFailed") }
    static var quicklinkSaveFailed: String { L10n.tr("status.quicklinkSaveFailed") }
    static var noteSaveFailed: String { L10n.tr("status.noteSaveFailed") }
    static var operationFailed: String { L10n.tr("status.operationFailed") }
    static var nothingToCapture: String { L10n.tr("status.nothingToCapture") }
    static var modulesLoading: String { L10n.tr("status.modulesLoading") }
    static var noAlternateActions: String { L10n.tr("status.noAlternateActions") }
    static var noActionPanelActions: String { L10n.tr("status.noActionPanelActions") }
    static var noResultsYet: String { L10n.tr("status.noResultsYet") }
    static var clipboardEntryUpdated: String { L10n.tr("status.clipboardEntryUpdated") }
    static var replaceSelectionDone: String { L10n.tr("status.replaceSelectionDone") }
    static var replaceSelectionFailed: String { L10n.tr("status.replaceSelectionFailed") }
    static var replaceSelectionEmpty: String { L10n.tr("status.replaceSelectionEmpty") }
}

enum LauncherFeedbackKind {
    case success
    case warning
    case error
    case noop
}

struct LauncherFeedback {
    let kind: LauncherFeedbackKind
    let message: String
    let delayDismiss: Bool

    init(kind: LauncherFeedbackKind, message: String, delayDismiss: Bool? = nil) {
        self.kind = kind
        self.message = message
        switch kind {
        case .success:
            self.delayDismiss = delayDismiss ?? true
        case .warning:
            self.delayDismiss = delayDismiss ?? true
        case .error:
            self.delayDismiss = delayDismiss ?? true
        case .noop:
            self.delayDismiss = delayDismiss ?? false
        }
    }
}
