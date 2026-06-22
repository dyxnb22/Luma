import Foundation
import Testing
@testable import LumaModules

@Test func snippetIndexRanksByFrecencyWhenFuzzyMatchEqual() {
    let now = Date(timeIntervalSince1970: 1_700_000_000)
    let recent = Snippet(
        title: "Git Rebase",
        content: "git rebase -i HEAD~3",
        tags: ["git"],
        usageCount: 10,
        lastUsedAt: now,
        createdAt: now.addingTimeInterval(-86400 * 30)
    )
    let stale = Snippet(
        title: "Git Reset",
        content: "git reset --hard",
        tags: ["git"],
        usageCount: 1,
        lastUsedAt: now.addingTimeInterval(-86400 * 14),
        createdAt: now.addingTimeInterval(-86400 * 60)
    )
    let results = SnippetIndex.search([stale, recent], query: "git", limit: 8, now: now)
    #expect(results.first?.snippet.id == recent.id)
}

@Test func snippetIndexBoostsTagMatch() {
    let now = Date()
    let tagged = Snippet(title: "Rebase", content: "interactive rebase workflow", tags: ["git"], usageCount: 0, lastUsedAt: now)
    let unrelated = Snippet(title: "Docker", content: "docker compose up -d", tags: [], usageCount: 0, lastUsedAt: now)
    let results = SnippetIndex.search([unrelated, tagged], query: "git", limit: 8, now: now)
    #expect(results.count == 1)
    #expect(results.first?.snippet.id == tagged.id)
}

@Test func snippetIndexTopByFrecencyWithoutQuery() {
    let now = Date()
    let hot = Snippet(title: "A", content: "a", usageCount: 20, lastUsedAt: now)
    let cold = Snippet(title: "B", content: "b", usageCount: 0, lastUsedAt: now.addingTimeInterval(-86400 * 30))
    let top = SnippetIndex.topByFrecency([cold, hot], limit: 1, now: now)
    #expect(top.first?.id == hot.id)
}
