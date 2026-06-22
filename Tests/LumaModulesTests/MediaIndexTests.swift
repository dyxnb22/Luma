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
