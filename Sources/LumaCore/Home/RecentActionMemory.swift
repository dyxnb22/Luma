import Foundation

/// Persists recently completed launcher actions for Home replay.
public actor RecentActionMemory {
  public static let shared = RecentActionMemory()

  public struct Record: Codable, Sendable, Equatable, Identifiable {
    public let itemID: ResultID
    public let title: String
    public let subtitle: String?
    public let action: Action
    public let icon: IconRef
    public let completedAt: Date

    public var id: String { "\(itemID.module.rawValue).\(itemID.key).\(action.id.key)" }
  }

  private let maxEntries: Int
  private let persistenceURL: URL
  private var records: [Record] = []

  public init(maxEntries: Int = 8, persistenceURL: URL? = nil, fileManager: FileManager = .default) {
    self.maxEntries = maxEntries
    if let persistenceURL {
      self.persistenceURL = persistenceURL
    } else {
      let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
        ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
      self.persistenceURL = base.appendingPathComponent("Luma/recent-actions.json")
    }
    if let data = try? Data(contentsOf: self.persistenceURL),
       let decoded = try? JSONDecoder().decode([Record].self, from: data) {
      records = decoded
    }
  }

  public func record(action: Action, item: ResultItem) {
    guard Self.shouldRecord(action: action, item: item) else { return }
    let entry = Record(
      itemID: item.id,
      title: item.title,
      subtitle: item.subtitle,
      action: action,
      icon: item.icon,
      completedAt: Date()
    )
    records.removeAll { $0.id == entry.id }
    records.insert(entry, at: 0)
    if records.count > maxEntries {
      records = Array(records.prefix(maxEntries))
    }
    persist()
  }

  public func recent(limit: Int = 5) -> [Record] {
    Array(records.prefix(limit))
  }

  public static func resultItem(from record: Record, modulePriority: Int = 0) -> ResultItem {
    ResultItem(
      id: ResultID(module: record.itemID.module, key: "recent.\(record.itemID.key)"),
      title: record.title,
      titleAttributed: AttributedString(record.title),
      subtitle: record.subtitle ?? record.action.title,
      icon: record.icon,
      primaryAction: record.action,
      secondaryActions: [],
      rankingHints: RankingHints(basePriority: modulePriority),
      rowKind: .starter
    )
  }

  public static func shouldRecord(action: Action, item: ResultItem) -> Bool {
    if item.id.key.hasPrefix("recent.") { return false }
    switch action.kind {
    case .launchApp, .focusWindow, .noop, .replaceQuery:
      return false
    default:
      return true
    }
  }

  private func persist() {
    let dir = persistenceURL.deletingLastPathComponent()
    try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    guard let data = try? JSONEncoder().encode(records) else { return }
    try? data.write(to: persistenceURL, options: .atomic)
  }
}
