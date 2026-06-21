import Foundation
import SQLite3

private let SQLITE_TRANSIENT = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

public struct WordEntry: Sendable, Hashable {
    public let id: Int64
    public let term: String
    public let phonetic: String
    public let meaning: String
    public let example: String
    public let category: String
    public let familiarity: String
    public let reviewStage: Int
    public let reviewCount: Int
    public let wrongCount: Int
    public let nextReviewAt: String

    public init(id: Int64, term: String, phonetic: String, meaning: String, example: String, category: String, familiarity: String, reviewStage: Int, reviewCount: Int, wrongCount: Int, nextReviewAt: String) {
        self.id = id
        self.term = term
        self.phonetic = phonetic
        self.meaning = meaning
        self.example = example
        self.category = category
        self.familiarity = familiarity
        self.reviewStage = reviewStage
        self.reviewCount = reviewCount
        self.wrongCount = wrongCount
        self.nextReviewAt = nextReviewAt
    }
}

public enum WordbookStoreError: Error {
    case openFailed
}

public actor WordbookStore {
    private let dbURL: URL

    public init(dbURL: URL = URL(fileURLWithPath: "/Users/diaoyuxuan/wordbot/data/wordpet.sqlite3")) {
        self.dbURL = dbURL
    }

    public func dueWords(limit: Int = 20, nowISO: String = ISO8601DateFormatter().string(from: Date())) throws -> [WordEntry] {
        try query("""
        SELECT id, term, phonetic, meaning, example, category, familiarity,
               review_stage, review_count, wrong_count, next_review_at
        FROM words
        WHERE next_review_at <= ?
        ORDER BY next_review_at ASC, wrong_count DESC, review_count ASC
        LIMIT ?
        """, params: [nowISO, String(limit)])
    }

    public func search(_ text: String, limit: Int = 20) throws -> [WordEntry] {
        let pattern = "%\(text)%"
        return try query("""
        SELECT id, term, phonetic, meaning, example, category, familiarity,
               review_stage, review_count, wrong_count, next_review_at
        FROM words
        WHERE term LIKE ? OR meaning LIKE ? OR example LIKE ? OR category LIKE ?
        ORDER BY wrong_count DESC, review_count ASC
        LIMIT ?
        """, params: [pattern, pattern, pattern, pattern, String(limit)])
    }

    public func count() throws -> Int {
        var db: OpaquePointer?
        guard sqlite3_open_v2(dbURL.path, &db, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
            throw WordbookStoreError.openFailed
        }
        defer { sqlite3_close(db) }
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(db, "SELECT COUNT(*) FROM words", -1, &statement, nil) == SQLITE_OK else { return 0 }
        defer { sqlite3_finalize(statement) }
        return sqlite3_step(statement) == SQLITE_ROW ? Int(sqlite3_column_int(statement, 0)) : 0
    }

    private func query(_ sql: String, params: [String]) throws -> [WordEntry] {
        var db: OpaquePointer?
        guard sqlite3_open_v2(dbURL.path, &db, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
            throw WordbookStoreError.openFailed
        }
        defer { sqlite3_close(db) }

        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(statement) }

        for (index, param) in params.enumerated() {
            sqlite3_bind_text(statement, Int32(index + 1), param, -1, SQLITE_TRANSIENT)
        }

        var rows: [WordEntry] = []
        while sqlite3_step(statement) == SQLITE_ROW {
            rows.append(WordEntry(
                id: sqlite3_column_int64(statement, 0),
                term: text(statement, 1),
                phonetic: text(statement, 2),
                meaning: text(statement, 3),
                example: text(statement, 4),
                category: text(statement, 5),
                familiarity: text(statement, 6),
                reviewStage: Int(sqlite3_column_int(statement, 7)),
                reviewCount: Int(sqlite3_column_int(statement, 8)),
                wrongCount: Int(sqlite3_column_int(statement, 9)),
                nextReviewAt: text(statement, 10)
            ))
        }
        return rows
    }

    private func text(_ statement: OpaquePointer?, _ index: Int32) -> String {
        guard let cString = sqlite3_column_text(statement, index) else { return "" }
        return String(cString: cString)
    }
}
