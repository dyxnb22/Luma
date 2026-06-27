import Foundation

/// Classifies contextual home suggestions for memory and ranking rules.
public enum HomeSuggestionKind: Sendable {
  /// Resume an in-progress flow (daily note, todo, records, current project).
  case continueFlow
  /// Create new content from clipboard or selection (note, snippet, quicklink).
  case create
  /// One-shot clipboard transform (format JSON, decode Base64).
  case transform
  /// Utility action (translate selection).
  case utility
}
