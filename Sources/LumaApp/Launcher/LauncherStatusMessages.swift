import Foundation

enum LauncherStatusMessages {
  // Success
  static let savedToDailyNote = "Saved to daily note"
  static let draftLoadedInQuicklinks = "Draft loaded in Quicklinks"
  static let draftLoadedInSnippets = "Draft loaded in Snippets"
  static let snippetCreated = "Snippet created"
  static let quicklinkSaved = "Quicklink saved"
  static let copiedToClipboard = "Copied to clipboard"
  static let snippetExpanded = "Snippet expanded"

  // Warnings
  static let notesRootNotConfigured = "Set a Notes root first"
  static let quicklinkTriggerTaken = "Trigger already in use — pick another"
  static let quicklinkURLDuplicate = "This URL is already saved"
  static let quicklinkMissingProtocol = "URL needs http:// or https://"
  static let snippetTriggerTaken = "Trigger already in use — pick another"
  static let snippetDuplicateContent = "Similar snippet already exists"

  // Errors
  static let snippetSaveFailed = "Couldn't create snippet"
  static let quicklinkSaveFailed = "Couldn't save quicklink"
  static let noteSaveFailed = "Couldn't save to daily note"
  static let operationFailed = "Action failed"

  // No-op / informational
  static let nothingToCapture = "Nothing to capture"
  static let modulesLoading = "Modules loading…"
  static let noAlternateActions = "No alternate actions for this item"
  static let noActionPanelActions = "No extra actions — press Return to open"
  static let noResultsYet = "No results yet"
  static let clipboardEntryUpdated = "Clipboard entry updated"
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
