import Foundation
import LumaCore

public struct ProjectSearchResult: Sendable, Hashable {
    public let record: ProjectRecord
    public let score: Double
}

public struct ProjectIndex: Sendable {
    private let records: [ProjectRecord]
    private let byPath: [String: ProjectRecord]

    public init(records: [ProjectRecord]) {
        var merged: [String: ProjectRecord] = [:]
        for rawRecord in records {
            let record = rawRecord.normalized()
            if let existing = merged[record.path] {
                merged[record.path] = Self.merge(existing, record)
            } else {
                merged[record.path] = record
            }
        }
        self.byPath = merged
        self.records = merged.values.sorted {
            $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending
        }
    }

    public var all: [ProjectRecord] { records }

    public func homeRecords(limit: Int = 8, recentPaths: [String]) -> [ProjectRecord] {
        var results: [ProjectRecord] = []
        var used = Set<String>()

        for record in records where record.pinned {
            guard used.insert(record.path).inserted else { continue }
            results.append(record)
            if results.count >= limit { return results }
        }

        for path in recentPaths {
            let normalizedPath = ProjectRecord.normalizePath(path)
            guard let record = byPath[normalizedPath], used.insert(normalizedPath).inserted else { continue }
            results.append(record)
            if results.count >= limit { return results }
        }

        for record in records.sorted(by: { ($0.lastOpened ?? .distantPast) > ($1.lastOpened ?? .distantPast) }) {
            guard used.insert(record.path).inserted else { continue }
            results.append(record)
            if results.count >= limit { return results }
        }

        return results
    }

    public func matchByLabel(_ label: String) -> ProjectRecord? {
        let normalized = label.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !normalized.isEmpty else { return nil }

        if let exact = records.first(where: { $0.name.lowercased() == normalized }) {
            return exact
        }

        if let aliasMatch = records.first(where: { record in
            record.aliases.contains { $0.lowercased() == normalized }
        }) {
            return aliasMatch
        }

        let basenameMatches = records.filter { record in
            URL(fileURLWithPath: record.path).lastPathComponent.lowercased() == normalized
        }
        if basenameMatches.count == 1 {
            return basenameMatches[0]
        }

        return nil
    }

    public func search(_ query: String, limit: Int = 8) -> [ProjectSearchResult] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return records.prefix(limit).map { ProjectSearchResult(record: $0, score: 1.0) }
        }

        let lowered = trimmed.lowercased()
        var results: [ProjectSearchResult] = []

        for record in records {
            let nameScore = FuzzyMatcher.score(query: lowered, target: record.name.lowercased())
            let pathScore = FuzzyMatcher.score(query: lowered, target: record.path.lowercased()) * 0.85
            var score = max(nameScore, pathScore)
            if record.aliases.contains(where: { alias in
                let aliasLower = alias.lowercased()
                return aliasLower == lowered || aliasLower.hasPrefix(lowered) || lowered.contains(aliasLower)
            }) {
                score = min(1.0, max(score, 0.95))
            }
            guard score > 0 else { continue }
            if record.pinned { score = min(1.0, score + 0.05) }
            results.append(ProjectSearchResult(record: record, score: score))
        }

        return results
            .sorted { lhs, rhs in
                if lhs.score != rhs.score { return lhs.score > rhs.score }
                return lhs.record.name.localizedCaseInsensitiveCompare(rhs.record.name) == .orderedAscending
            }
            .prefix(limit)
            .map { $0 }
    }

    private static func merge(_ lhs: ProjectRecord, _ rhs: ProjectRecord) -> ProjectRecord {
        ProjectRecord(
            name: lhs.name.isEmpty ? rhs.name : lhs.name,
            path: lhs.path,
            aliases: Array(Set(lhs.aliases + rhs.aliases)),
            preferredOpener: lhs.preferredOpener,
            pinned: lhs.pinned || rhs.pinned,
            lastOpened: [lhs.lastOpened, rhs.lastOpened].compactMap { $0 }.max()
        )
    }
}
