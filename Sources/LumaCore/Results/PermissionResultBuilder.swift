import Foundation

/// Shared inline permission card for module search results.
public struct PermissionCardSpec: Sendable {
  public let module: ModuleIdentifier
  public let title: String
  public let explanation: String
  public let icon: IconRef
  public let requestAction: Action?
  public let settingsAction: Action
  public let accessDenied: Bool

  public init(
    module: ModuleIdentifier,
    title: String,
    explanation: String,
    icon: IconRef,
    requestAction: Action?,
    settingsAction: Action,
    accessDenied: Bool
  ) {
    self.module = module
    self.title = title
    self.explanation = explanation
    self.icon = icon
    self.requestAction = requestAction
    self.settingsAction = settingsAction
    self.accessDenied = accessDenied
  }
}

public enum PermissionResultBuilder {
  public static func row(spec: PermissionCardSpec, resultKey: String = "grant") -> ResultItem {
    let primary: Action
    let secondaries: [Action]
    if spec.accessDenied {
      primary = spec.settingsAction
      secondaries = []
    } else {
      primary = spec.requestAction ?? spec.settingsAction
      secondaries = spec.requestAction == nil ? [] : [spec.settingsAction]
    }
    return ResultItem(
      id: ResultID(module: spec.module, key: resultKey),
      title: spec.title,
      titleAttributed: AttributedString(spec.title),
      subtitle: spec.explanation,
      icon: spec.icon,
      primaryAction: primary,
      secondaryActions: secondaries,
      rankingHints: RankingHints(basePriority: 100),
      rowKind: .informational
    )
  }
}
