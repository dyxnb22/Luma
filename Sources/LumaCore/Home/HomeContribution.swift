import Foundation

public struct HomeContribution: Sendable {
    public let item: ResultItem
    public let key: String
    public let kind: HomeSuggestionKind
    public let basePriority: Int

    public init(item: ResultItem, key: String, kind: HomeSuggestionKind, basePriority: Int) {
        self.item = item
        self.key = key
        self.kind = kind
        self.basePriority = basePriority
    }
}

public struct HomeContributionContext: Sendable {
  public let pinnedModuleIDs: Set<ModuleIdentifier>
  public let enabledModuleIDs: Set<ModuleIdentifier>
  public let workbench: WorkbenchContext?

  public init(
    pinnedModuleIDs: Set<ModuleIdentifier>,
    enabledModuleIDs: Set<ModuleIdentifier>,
    workbench: WorkbenchContext? = nil
  ) {
    self.pinnedModuleIDs = pinnedModuleIDs
    self.enabledModuleIDs = enabledModuleIDs
    self.workbench = workbench
  }

  public func isEnabled(_ id: ModuleIdentifier) -> Bool {
    workbench?.isEnabled(id) ?? enabledModuleIDs.contains(id)
  }

  /// Module is enabled and pinned for Home hot-path suggestions.
  public func isHot(_ id: ModuleIdentifier) -> Bool {
    workbench?.isHot(id) ?? (enabledModuleIDs.contains(id) && pinnedModuleIDs.contains(id))
  }
}

public protocol HomeContributor: Sendable {
    func contribute(context: HomeContributionContext) async -> [HomeContribution]
}
