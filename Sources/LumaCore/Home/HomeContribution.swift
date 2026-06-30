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

  public init(pinnedModuleIDs: Set<ModuleIdentifier>, enabledModuleIDs: Set<ModuleIdentifier>) {
    self.pinnedModuleIDs = pinnedModuleIDs
    self.enabledModuleIDs = enabledModuleIDs
  }
}

public protocol HomeContributor: Sendable {
    func contribute(context: HomeContributionContext) async -> [HomeContribution]
}
