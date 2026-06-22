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
    case prepareFailed(message: String)
    case stepFailed(message: String)
}

/// Cross-instance notifications when any `WordbookStore` mutates review data.
public enum WordbookStoreChangeHub {
    private static let lock = NSLock()
    private nonisolated(unsafe) static var continuations: [UUID: AsyncStream<Void>.Continuation] = [:]

    public static func dataChanges() -> AsyncStream<Void> {
        AsyncStream { continuation in
            let id = UUID()
            lock.lock()
            continuations[id] = continuation
            lock.unlock()
            continuation.onTermination = { _ in
                lock.lock()
                continuations.removeValue(forKey: id)
                lock.unlock()
            }
        }
    }

    static func publishDataChanged() {
        lock.lock()
        let snapshot = continuations.values
        lock.unlock()
        for continuation in snapshot {
            continuation.yield(())
        }
    }
}

public actor WordbookStore {
    private let dbURL: URL
    private var readDB: OpaquePointer?

    public init(dbURL: URL = WordbookMigrator.defaultDestinationURL()) {
        self.dbURL = dbURL
    }

    public func databaseURL() -> URL { dbURL }

    public func dueWords(limit: Int = 20, now: Date = Date()) throws -> [WordEntry] {
        let nowISO = Self.iso(now)
        return try query("""
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
        let db = try openReadConnection()
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(db, "SELECT COUNT(*) FROM words", -1, &statement, nil) == SQLITE_OK else { return 0 }
        defer { sqlite3_finalize(statement) }
        return sqlite3_step(statement) == SQLITE_ROW ? Int(sqlite3_column_int(statement, 0)) : 0
    }

    public func dueTodayCount(now: Date = Date()) throws -> Int {
        let nowISO = Self.iso(now)
        let db = try openReadConnection()
        var statement: OpaquePointer?
        let sql = "SELECT COUNT(*) FROM words WHERE next_review_at <= ?"
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else { return 0 }
        defer { sqlite3_finalize(statement) }
        sqlite3_bind_text(statement, 1, nowISO, -1, SQLITE_TRANSIENT)
        return sqlite3_step(statement) == SQLITE_ROW ? Int(sqlite3_column_int(statement, 0)) : 0
    }

    /// Apply a review outcome: updates stage, next_review_at, review_count, wrong_count, last_review_at.
    /// Stage 9 + .known additionally sets mastered_at.
    @discardableResult
    public func recordReview(
        wordID: Int64,
        familiarity: WordFamiliarity,
        now: Date = Date()
    ) throws -> WordEntry? {
        invalidateReadConnection()
        var db: OpaquePointer?
        guard sqlite3_open_v2(dbURL.path, &db, SQLITE_OPEN_READWRITE, nil) == SQLITE_OK else {
            throw WordbookStoreError.openFailed
        }
        defer { sqlite3_close(db) }

        let current = try fetchSingle(db: db, id: wordID)
        guard let current else { return nil }

        let (newStage, delay) = ReviewScheduler.schedule(
            familiarity: familiarity,
            currentStage: current.reviewStage,
            wrongCount: current.wrongCount
        )
        let nextDate = now.addingTimeInterval(TimeInterval(delay.components.seconds))
        let nextISO = Self.iso(nextDate)
        let lastISO = Self.iso(now)

        let newReviewCount = current.reviewCount + 1
        let newWrongCount: Int
        let newFamiliarity: String
        switch familiarity {
        case .known:
            newWrongCount = current.wrongCount
            newFamiliarity = newStage >= ReviewScheduler.intervals.count ? "mastered" : "known"
        case .fuzzy:
            newWrongCount = current.wrongCount
            newFamiliarity = "fuzzy"
        case .unknown:
            newWrongCount = current.wrongCount + 1
            newFamiliarity = "unknown"
        }
        let masteredISO = (familiarity == .known && newStage >= ReviewScheduler.intervals.count) ? lastISO : ""

        let updateSQL = """
        UPDATE words
        SET review_stage = ?, next_review_at = ?, review_count = ?, wrong_count = ?,
            familiarity = ?, last_review_at = ?, updated_at = ?, mastered_at = CASE WHEN ? = '' THEN mastered_at ELSE ? END
        WHERE id = ?
        """
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(db, updateSQL, -1, &statement, nil) == SQLITE_OK else {
            throw WordbookStoreError.prepareFailed(message: String(cString: sqlite3_errmsg(db)))
        }
        defer { sqlite3_finalize(statement) }

        sqlite3_bind_int(statement, 1, Int32(newStage))
        sqlite3_bind_text(statement, 2, nextISO, -1, SQLITE_TRANSIENT)
        sqlite3_bind_int(statement, 3, Int32(newReviewCount))
        sqlite3_bind_int(statement, 4, Int32(newWrongCount))
        sqlite3_bind_text(statement, 5, newFamiliarity, -1, SQLITE_TRANSIENT)
        sqlite3_bind_text(statement, 6, lastISO, -1, SQLITE_TRANSIENT)
        sqlite3_bind_text(statement, 7, lastISO, -1, SQLITE_TRANSIENT)
        sqlite3_bind_text(statement, 8, masteredISO, -1, SQLITE_TRANSIENT)
        sqlite3_bind_text(statement, 9, masteredISO, -1, SQLITE_TRANSIENT)
        sqlite3_bind_int64(statement, 10, wordID)

        guard sqlite3_step(statement) == SQLITE_DONE else {
            throw WordbookStoreError.stepFailed(message: String(cString: sqlite3_errmsg(db)))
        }

        WordbookStoreChangeHub.publishDataChanged()
        return try fetchSingle(db: db, id: wordID)
    }

    private func openReadConnection() throws -> OpaquePointer {
        if let readDB { return readDB }
        var db: OpaquePointer?
        guard sqlite3_open_v2(dbURL.path, &db, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
            throw WordbookStoreError.openFailed
        }
        readDB = db
        return db!
    }

    private func invalidateReadConnection() {
        if let readDB {
            sqlite3_close(readDB)
            self.readDB = nil
        }
    }

    private func fetchSingle(db: OpaquePointer?, id: Int64) throws -> WordEntry? {
        let sql = """
        SELECT id, term, phonetic, meaning, example, category, familiarity,
               review_stage, review_count, wrong_count, next_review_at
        FROM words
        WHERE id = ?
        LIMIT 1
        """
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
            throw WordbookStoreError.prepareFailed(message: String(cString: sqlite3_errmsg(db)))
        }
        defer { sqlite3_finalize(statement) }
        sqlite3_bind_int64(statement, 1, id)
        guard sqlite3_step(statement) == SQLITE_ROW else { return nil }
        return WordEntry(
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
        )
    }

    private func query(_ sql: String, params: [String]) throws -> [WordEntry] {
        let db = try openReadConnection()
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

    private static func iso(_ date: Date) -> String {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        return formatter.string(from: date)
    }
}
