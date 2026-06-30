import Foundation
import LumaCore
import LumaModules

actor ContextualHomeProvider: LauncherHomeProvider, ContextualHomeSectionProvider {
  private let suggestionMemory: HomeSuggestionMemory
  private let contributors: [any HomeContributor]
  private var pinnedModuleIDs: Set<ModuleIdentifier>
  private var enabledModuleIDs: Set<ModuleIdentifier>
  private var workbench: WorkbenchContext?

  init(
    notes: (any NotesContinueClient)? = nil,
    todo: (any TodoContinueClient)? = nil,
    media: (any MediaContinueClient)? = nil,
    wordbook: (any WordbookContinueClient)? = nil,
    pinnedModuleIDs: Set<ModuleIdentifier> = ModuleWarmupDefaults.defaultPinnedModuleIDs,
    enabledModuleIDs: Set<ModuleIdentifier>? = nil,
    suggestionMemory: HomeSuggestionMemory = .shared,
    contributors: [any HomeContributor]? = nil
  ) {
    self.pinnedModuleIDs = pinnedModuleIDs
    self.enabledModuleIDs = enabledModuleIDs ?? Set(ModuleRegistry.allBundles.map { $0.identifier })
    self.suggestionMemory = suggestionMemory
    self.contributors = contributors ?? [
      ProjectHomeContributor(),
      ProjectActivityHomeContributor(),
      SelectionHomeContributor(),
      ClipboardHomeContributor(),
      ContinueHomeContributor(
        notes: notes,
        todo: todo,
        media: media,
        wordbook: wordbook
      )
    ]
  }

  func updatePinnedModuleIDs(_ ids: Set<ModuleIdentifier>) {
    pinnedModuleIDs = ids
  }

  func updateEnabledModuleIDs(_ ids: Set<ModuleIdentifier>) {
    enabledModuleIDs = ids
  }

  func updateWorkbench(_ context: WorkbenchContext?) {
    workbench = context
  }

  func items() async -> [ResultItem] {
    let sections = await rankedSectionItems()
    return Array((sections.continue + sections.create).prefix(4))
  }

  func sectionedItems() async -> (continue: [ResultItem], create: [ResultItem]) {
    await rankedSectionItems()
  }

  private func rankedSectionItems() async -> (continue: [ResultItem], create: [ResultItem]) {
    let contributionContext = HomeContributionContext(
      pinnedModuleIDs: pinnedModuleIDs,
      enabledModuleIDs: enabledModuleIDs,
      workbench: workbench
    )
    let contributions = await collectContributions(context: contributionContext)
    let memory = suggestionMemory
    if contributions.contains(where: { $0.key == "contextual.current" }) {
      await memory.boostSessionContext(key: "contextual.current")
    }
    for contribution in contributions where contribution.key.hasPrefix("contextual.project-") {
      await memory.boostSessionContext(key: contribution.key)
    }
    for contribution in contributions where contribution.key.hasPrefix("contextual.project-activity") {
      await memory.boostSessionContext(key: contribution.key)
    }

    var ranked: [(item: ResultItem, kind: HomeSuggestionKind, priority: Int)] = []
    for contribution in contributions {
      guard await memory.isEligible(key: contribution.key, kind: contribution.kind) else { continue }
      let adjusted = await memory.adjustedPriority(
        base: contribution.basePriority,
        key: contribution.key,
        kind: contribution.kind
      )
      ranked.append((contribution.item, contribution.kind, adjusted))
    }

    ranked.sort { $0.priority > $1.priority }
    let continueFlow = ranked
      .filter { $0.kind == .continueFlow }
      .prefix(HomeSuggestionPolicy.maxContinueRows)
      .map(\.item)
    let hasProjectCreateRows = contributions.contains { $0.key.hasPrefix("contextual.project-") }
    let createLimit = hasProjectCreateRows
      ? HomeSuggestionPolicy.maxCreateRowsWithProject
      : HomeSuggestionPolicy.maxCreateRowsDefault
    let utilityRows = ranked
      .filter { $0.kind == .utility || $0.kind == .transform }
      .prefix(HomeSuggestionPolicy.maxUtilityCreateRows)
      .map(\.item)
    let createRows = ranked
      .filter { $0.kind == .create }
      .prefix(createLimit)
      .map(\.item)
    let create = utilityRows + createRows
    return (continue: Array(continueFlow), create: create)
  }

  private func collectContributions(context: HomeContributionContext) async -> [HomeContribution] {
    await withTaskGroup(of: [HomeContribution].self, returning: [HomeContribution].self) { group in
      for contributor in contributors {
        group.addTask {
          await contributor.contribute(context: context)
        }
      }

      var output: [HomeContribution] = []
      for await items in group {
        output.append(contentsOf: items)
      }
      return output
    }
  }
}
