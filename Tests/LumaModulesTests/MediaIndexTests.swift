import Foundation
import Testing
@testable import LumaModules

@Test func mediaIndexSearchRanksRecency() {
    let now = Date(timeIntervalSince1970: 1_700_000_000)
    let recent = MediaItem(title: "Oppenheimer", category: .movie, status: .done, rating: 9, updatedAt: now)
    let old = MediaItem(title: "Oppenheimer II", category: .movie, status: .done, rating: 9, updatedAt: now.addingTimeInterval(-86400 * 60))
    let results = MediaIndex.search([old, recent], query: "oppen", limit: 8, now: now)
    #expect(results.first?.item.title == "Oppenheimer")
}

@Test func mediaIndexSearchByTag() {
    let tagged = MediaItem(title: "三体", category: .book, status: .done, tags: ["sci-fi"])
    let other = MediaItem(title: "Dune", category: .book, status: .done, tags: ["space"])
    let results = MediaIndex.search([other, tagged], query: "sci-fi", limit: 8)
    #expect(results.first?.item.title == "三体")
}

@Test func mediaIndexSearchByCategory() {
    let movie = MediaItem(title: "Arrival", category: .movie, status: .done)
    let book = MediaItem(title: "Story of Your Life", category: .book, status: .done)
    let results = MediaIndex.search([movie, book], query: "book", limit: 8)
    #expect(results.first?.item.title == "Story of Your Life")
}

@Test func mediaIndexSearchByStatus() {
    let watching = MediaItem(title: "Frieren", category: .anime, status: .inProgress)
    let done = MediaItem(title: "Barbie", category: .movie, status: .done)
    let results = MediaIndex.search([done, watching], query: "watching", limit: 8)
    #expect(results.first?.item.title == "Frieren")
}

@Test func mediaIndexFilterByCategoryAndStatus() {
    let movie = MediaItem(title: "A", category: .movie, status: .done)
    let book = MediaItem(title: "B", category: .book, status: .planned)
    let filtered = MediaIndex.filter([movie, book], category: .book, status: .planned, sort: .title)
    #expect(filtered.count == 1)
    #expect(filtered[0].title == "B")
}

@Test func mediaIndexStats() {
    let year = Calendar.current.component(.year, from: Date())
    let components = DateComponents(year: year, month: 6, day: 1)
    let completed = Calendar.current.date(from: components)!
    let items = [
        MediaItem(title: "A", category: .movie, status: .done, rating: 8, completedAt: completed),
        MediaItem(title: "B", category: .book, status: .done, rating: 6, completedAt: completed),
        MediaItem(title: "C", category: .game, status: .planned)
    ]
    let stats = MediaIndex.stats(for: items)
    #expect(stats.count == 3)
    #expect(stats.averageRating == 7.0)
    #expect(stats.doneThisYear == 2)
}

@Test func mediaIndexSearchPerformanceStaysUnderBudget() {
    var items: [MediaItem] = []
    items.reserveCapacity(1000)
    for index in 0..<1000 {
        items.append(MediaItem(
            title: "Record \(index)",
            category: index.isMultiple(of: 5) ? .book : .movie,
            status: index.isMultiple(of: 3) ? .inProgress : .done,
            rating: (index % 10) + 1,
            tags: index.isMultiple(of: 7) ? ["sci-fi"] : ["drama"],
            updatedAt: Date(timeIntervalSince1970: Double(index))
        ))
    }

    var samples: [Double] = []
    let clock = ContinuousClock()
    for query in ["record", "book", "sci-fi", "watching", "42"] {
        for _ in 0..<100 {
            let start = clock.now
            _ = MediaIndex.search(items, query: query, limit: 8)
            let elapsed = start.duration(to: clock.now)
            samples.append(Double(elapsed.components.seconds) * 1000 + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000)
        }
    }

    let sorted = samples.sorted()
    let p95 = sorted[Int(Double(sorted.count - 1) * 0.95)]
    #expect(p95 < 30)
}
