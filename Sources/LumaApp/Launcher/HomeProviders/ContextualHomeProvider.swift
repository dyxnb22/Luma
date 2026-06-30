import Foundation
import LumaCore
import LumaModules

struct ContextualHomeProvider: LauncherHomeProvider, ContextualHomeSectionProvider {
  private let suggestionMemory: HomeSuggestionMemory
  private let contributors: [any HomeContributor]
  private var pinnedModuleIDs: Set<ModuleIdentifier>
  private var enabledModuleIDs: Set<ModuleIdentifier>

  init(
    notesModule: NotesModule? = nil,
    todoModule: TodoModule? = nil,
    mediaModule: MediaModule? = nil,
    wordbookModule: WordbookModule? = nil,
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
      SelectionHomeContributor(),
      ClipboardHomeContributor(),
      ContinueHomeContributor(
        notesModule: notesModule,
        todoModule: todoModule,
        mediaModule: mediaModule,
        wordbookModule: wordbookModule
      )
    ]
  }

  mutating func updatePinnedModuleIDs(_ ids: Set<ModuleIdentifier>) {
    pinnedModuleIDs = ids
  }

  mutating func updateEnabledModuleIDs(_ ids: Set<ModuleIdentifier>) {
    enabledModuleIDs = ids
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
      enabledModuleIDs: enabledModuleIDs
    )
    let contributions = await collectContributions(context: contributionContext)
    let memory = suggestionMemory
    if contributions.contains(where: { $0.key == "contextual.current" }) {
      await memory.boostSessionContext(key: "contextual.current")
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
      .prefix(2)
      .map(\.item)
    let hasProjectCreateRows = contributions.contains { $0.key.hasPrefix("contextual.project-") }
    let createLimit = hasProjectCreateRows ? 2 : 1
    let create = ranked
      .filter { $0.kind != .continueFlow }
      .prefix(createLimit)
      .map(\.item)
    return (continue: Array(continueFlow), create: Array(create))
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
