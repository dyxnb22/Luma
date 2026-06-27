import Foundation
import LumaCore

struct RecentActionsHomeProvider: LauncherHomeProvider {
  private let memory: RecentActionMemory

  init(memory: RecentActionMemory = .shared) {
    self.memory = memory
  }

  func items() async -> [ResultItem] {
    let records = await memory.recent(limit: 5)
    return records.map { RecentActionMemory.resultItem(from: $0) }
  }
}
