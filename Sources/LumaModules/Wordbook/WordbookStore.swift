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

public struct SessionPlanningStats: Sendable, Equatable {
    public let dueLeft: Int
    public let newLeft: Int
    public let quotaLeft: Int
    public let wrongToday: Int
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
    private var schemaEnsured = false
    private var ftsEnsured = false

    public init(dbURL: URL = WordbookMigrator.defaultDestinationURL()) {
        self.dbURL = dbURL
    }

    public func databaseURL() -> URL { dbURL }

    // MARK: - Settings

    public func setting(_ key: String, default defaultValue: String) async throws -> String {
        try await ensureSchema()
        let db = try openReadConnection()
        var statement: OpaquePointer?
        let sql = "SELECT value FROM settings WHERE key = ? LIMIT 1"
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
            return defaultValue
        }
        defer { sqlite3_finalize(statement) }
        sqlite3_bind_text(statement, 1, key, -1, SQLITE_TRANSIENT)
        guard sqlite3_step(statement) == SQLITE_ROW else { return defaultValue }
        return text(statement, 0)
    }

    public func intSetting(_ key: String, default defaultValue: Int) async throws -> Int {
        let raw = try await setting(key, default: "")
        return Int(raw) ?? defaultValue
    }

    public func setSetting(_ key: String, value: String) async throws {
        try await ensureSchema()
        try await write { db in
            let sql = "INSERT INTO settings(key, value) VALUES(?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
            var statement: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else {
                throw WordbookStoreError.prepareFailed(message: String(cString: sqlite3_errmsg(db)))
            }
            defer { sqlite3_finalize(statement) }
            sqlite3_bind_text(statement, 1, key, -1, SQLITE_TRANSIENT)
            sqlite3_bind_text(statement, 2, value, -1, SQLITE_TRANSIENT)
            guard sqlite3_step(statement) == SQLITE_DONE else {
                throw WordbookStoreError.stepFailed(message: String(cString: sqlite3_errmsg(db)))
            }
        }
        invalidateReadConnection()
    }

    public func setIntSetting(_ key: String, value: Int) async throws {
        try await setSetting(key, value: String(value))
    }

    private static func todayKey(now: Date = Date()) -> String {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd"
        f.timeZone = .current
        return f.string(from: now)
    }

    public func resetDailyStatsIfNeeded(now: Date = Date()) async throws {
        let today = Self.todayKey(now: now)
        let stored = try await setting("daily_stats_date", default: "")
        guard stored != today else { return }
        try await setSetting("daily_stats_date", value: today)
        try await setIntSetting("daily_new_seen", value: 0)
        try await setIntSetting("daily_wrong_count", value: 0)
        try await setIntSetting("daily_reviewed", value: 0)
        try await setIntSetting("daily_mastered", value: 0)
    }

    public func dailyNewLimitForDueCount(_ dueCount: Int) async throws -> Int {
        try await resetDailyStatsIfNeeded()
        let wrongToday = try await intSetting("daily_wrong_count", default: 0)
        if wrongToday >= 10 { return 0 }
        if dueCount > 80 { return wrongToday >= 5 ? 8 : 15 }
        if dueCount > 30 { return wrongToday >= 5 ? 12 : 25 }
        if dueCount > 0 { return wrongToday >= 5 ? 18 : 35 }
        return wrongToday >= 5 ? 20 : 45
    }

    public func dailyNewRemaining(dueCount: Int) async throws -> Int {
        let limit = try await dailyNewLimitForDueCount(dueCount)
        let seen = try await intSetting("daily_new_seen", default: 0)
        return max(0, limit - seen)
    }

    public func recordNewWordShown() async throws {
        try await resetDailyStatsIfNeeded()
        let cur = try await intSetting("daily_new_seen", default: 0)
        try await setIntSetting("daily_new_seen", value: cur + 1)
    }

    public func recordWrongAnswer() async throws {
        try await resetDailyStatsIfNeeded()
        let cur = try await intSetting("daily_wrong_count", default: 0)
        try await setIntSetting("daily_wrong_count", value: cur + 1)
    }

    public func newWordCount() async throws -> Int {
        let db = try openReadConnection()
        var statement: OpaquePointer?
        let sql = "SELECT COUNT(*) FROM words WHERE (mastered_at IS NULL OR mastered_at = '') AND review_count = 0"
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else { return 0 }
        defer { sqlite3_finalize(statement) }
        return sqlite3_step(statement) == SQLITE_ROW ? Int(sqlite3_column_int(statement, 0)) : 0
    }

    public func nextNewWord() async throws -> WordEntry? {
        try query("""
        SELECT id, term, phonetic, meaning, example, category, familiarity,
               review_stage, review_count, wrong_count, next_review_at
        FROM words
        WHERE (mastered_at IS NULL OR mastered_at = '') AND review_count = 0
        ORDER BY id ASC
        LIMIT 1
        """, params: []).first
    }

    public func nextDueWord(before cutoff: String) async throws -> WordEntry? {
        try query("""
        SELECT id, term, phonetic, meaning, example, category, familiarity,
               review_stage, review_count, wrong_count, next_review_at
        FROM words
        WHERE next_review_at <= ?
        ORDER BY next_review_at ASC, wrong_count DESC, review_count ASC
        LIMIT 1
        """, params: [cutoff]).first
    }

    public func dueCount(before cutoff: String) async throws -> Int {
        let db = try openReadConnection()
        var statement: OpaquePointer?
        let sql = "SELECT COUNT(*) FROM words WHERE next_review_at <= ?"
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else { return 0 }
        defer { sqlite3_finalize(statement) }
        sqlite3_bind_text(statement, 1, cutoff, -1, SQLITE_TRANSIENT)
        return sqlite3_step(statement) == SQLITE_ROW ? Int(sqlite3_column_int(statement, 0)) : 0
    }

    /// Batched stats for `WordbookSessionPlanner.nextCard` (single actor round-trip).
    public func sessionPlanningStats(cutoff: String) async throws -> SessionPlanningStats {
        try await resetDailyStatsIfNeeded()
        let db = try openReadConnection()
        let dueLeft = try scalarInt(db, sql: "SELECT COUNT(*) FROM words WHERE next_review_at <= ?", bind: cutoff)
        let newLeft = try scalarInt(
            db,
            sql: "SELECT COUNT(*) FROM words WHERE (mastered_at IS NULL OR mastered_at = '') AND review_count = 0"
        )
        let wrongToday = try await intSetting("daily_wrong_count", default: 0)
        let seen = try await intSetting("daily_new_seen", default: 0)
        let limit = try await dailyNewLimitForDueCount(dueLeft)
        return SessionPlanningStats(
            dueLeft: dueLeft,
            newLeft: newLeft,
            quotaLeft: max(0, limit - seen),
            wrongToday: wrongToday
        )
    }

    public func wrongWordCount(atLeast minCount: Int = 3) async throws -> Int {
        let db = try openReadConnection()
        return try scalarInt(db, sql: "SELECT COUNT(*) FROM words WHERE wrong_count >= ?", bind: String(minCount))
    }

    public func wordsWithWrongCount(atLeast minCount: Int = 3, limit: Int = 200, offset: Int = 0) async throws -> [WordEntry] {
        try query("""
        SELECT id, term, phonetic, meaning, example, category, familiarity,
               review_stage, review_count, wrong_count, next_review_at
        FROM words
        WHERE wrong_count >= ?
        ORDER BY wrong_count DESC, review_count ASC, term COLLATE NOCASE ASC
        LIMIT ? OFFSET ?
        """, params: [String(minCount), String(limit), String(offset)])
    }

    public func progressSnapshot(now: Date = Date()) async throws -> WordbookProgressSnapshot {
        try await resetDailyStatsIfNeeded(now: now)
        let db = try openReadConnection()
        let total = try scalarInt(db, sql: "SELECT COUNT(*) FROM words")
        let mastered = try scalarInt(db, sql: "SELECT COUNT(*) FROM words WHERE mastered_at IS NOT NULL AND mastered_at != ''")
        let nowISO = Self.iso(now)
        let dueToday = try scalarInt(db, sql: "SELECT COUNT(*) FROM words WHERE next_review_at <= ?", bind: nowISO)
        let newAvailable = try await newWordCount()
        let todayReviewed = try await intSetting("daily_reviewed", default: 0)
        let todayNewLearned = try await intSetting("daily_new_seen", default: 0)
        let todayMastered = try await intSetting("daily_mastered", default: 0)
        let wrongToday = try await intSetting("daily_wrong_count", default: 0)
        let correctToday = max(0, todayReviewed - wrongToday)
        let accuracy = todayReviewed > 0 ? Double(correctToday) / Double(todayReviewed) : 1.0
        let streak = try await streakDays()
        let limit = try await dailyNewLimitForDueCount(dueToday)
        let seen = try await intSetting("daily_new_seen", default: 0)
        return WordbookProgressSnapshot(
            total: total,
            mastered: mastered,
            dueToday: dueToday,
            newAvailable: newAvailable,
            todayReviewed: todayReviewed,
            todayNewLearned: todayNewLearned,
            todayMastered: todayMastered,
            accuracyToday: accuracy,
            streakDays: streak,
            dailyNewLimit: limit,
            dailyNewSeen: seen
        )
    }

    public func allWords(limit: Int = 200, offset: Int = 0) async throws -> [WordEntry] {
        try query("""
        SELECT id, term, phonetic, meaning, example, category, familiarity,
               review_stage, review_count, wrong_count, next_review_at
        FROM words
        ORDER BY term COLLATE NOCASE ASC
        LIMIT ? OFFSET ?
        """, params: [String(limit), String(offset)])
    }

    public func upsertWords(_ entries: [WordEntry], replaceDuplicates: Bool = false) async throws -> (imported: Int, skipped: Int) {
        var imported = 0
        var skipped = 0
        try await write { db in
            for entry in entries {
                var existsStmt: OpaquePointer?
                guard sqlite3_prepare_v2(db, "SELECT id FROM words WHERE term = ? LIMIT 1", -1, &existsStmt, nil) == SQLITE_OK else { continue }
                sqlite3_bind_text(existsStmt, 1, entry.term, -1, SQLITE_TRANSIENT)
                let exists = sqlite3_step(existsStmt) == SQLITE_ROW
                sqlite3_finalize(existsStmt)
                if exists && !replaceDuplicates {
                    skipped += 1
                    continue
                }
                let nowISO = Self.iso(Date())
                let sql: String
                if exists {
                    sql = """
                    UPDATE words SET phonetic=?, meaning=?, example=?, category=?, updated_at=?
                    WHERE term=?
                    """
                } else {
                    sql = """
                    INSERT INTO words(term, phonetic, meaning, example, category, familiarity,
                        review_stage, review_count, wrong_count, next_review_at, created_at, updated_at)
                    VALUES(?, ?, ?, ?, ?, 'new', 0, 0, 0, ?, ?, ?)
                    """
                }
                var stmt: OpaquePointer?
                guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { continue }
                if exists {
                    sqlite3_bind_text(stmt, 1, entry.phonetic, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 2, entry.meaning, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 3, entry.example, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 4, entry.category, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 5, nowISO, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 6, entry.term, -1, SQLITE_TRANSIENT)
                } else {
                    sqlite3_bind_text(stmt, 1, entry.term, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 2, entry.phonetic, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 3, entry.meaning, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 4, entry.example, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 5, entry.category, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 6, nowISO, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 7, nowISO, -1, SQLITE_TRANSIENT)
                    sqlite3_bind_text(stmt, 8, nowISO, -1, SQLITE_TRANSIENT)
                }
                if sqlite3_step(stmt) == SQLITE_DONE { imported += 1 } else { skipped += 1 }
                sqlite3_finalize(stmt)
            }
        }
        invalidateReadConnection()
        WordbookStoreChangeHub.publishDataChanged()
        return (imported, skipped)
    }

    public func deleteWord(id: Int64) async throws {
        try await write { db in
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, "DELETE FROM words WHERE id = ?", -1, &stmt, nil) == SQLITE_OK else { return }
            sqlite3_bind_int64(stmt, 1, id)
            _ = sqlite3_step(stmt)
            sqlite3_finalize(stmt)
        }
        invalidateReadConnection()
        WordbookStoreChangeHub.publishDataChanged()
    }

    public func updateWord(_ entry: WordEntry) async throws {
        try await write { db in
            let sql = """
            UPDATE words SET term=?, phonetic=?, meaning=?, example=?, category=?, updated_at=?
            WHERE id=?
            """
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
            let nowISO = Self.iso(Date())
            sqlite3_bind_text(stmt, 1, entry.term, -1, SQLITE_TRANSIENT)
            sqlite3_bind_text(stmt, 2, entry.phonetic, -1, SQLITE_TRANSIENT)
            sqlite3_bind_text(stmt, 3, entry.meaning, -1, SQLITE_TRANSIENT)
            sqlite3_bind_text(stmt, 4, entry.example, -1, SQLITE_TRANSIENT)
            sqlite3_bind_text(stmt, 5, entry.category, -1, SQLITE_TRANSIENT)
            sqlite3_bind_text(stmt, 6, nowISO, -1, SQLITE_TRANSIENT)
            sqlite3_bind_int64(stmt, 7, entry.id)
            _ = sqlite3_step(stmt)
            sqlite3_finalize(stmt)
        }
        invalidateReadConnection()
        WordbookStoreChangeHub.publishDataChanged()
    }

    public func resetWordStage(id: Int64) async throws {
        let nowISO = Self.iso(Date())
        try await write { db in
            let sql = """
            UPDATE words SET review_stage=0, review_count=0, wrong_count=0, familiarity='new',
                next_review_at=?, mastered_at='', updated_at=?
            WHERE id=?
            """
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
            sqlite3_bind_text(stmt, 1, nowISO, -1, SQLITE_TRANSIENT)
            sqlite3_bind_text(stmt, 2, nowISO, -1, SQLITE_TRANSIENT)
            sqlite3_bind_int64(stmt, 3, id)
            _ = sqlite3_step(stmt)
            sqlite3_finalize(stmt)
        }
        invalidateReadConnection()
        WordbookStoreChangeHub.publishDataChanged()
    }

    public func voiceAccent() async throws -> String {
        try await setting("voice_accent", default: "uk")
    }

    public func setVoiceAccent(_ accent: String) async throws {
        try await setSetting("voice_accent", value: accent)
    }

    public func resetTodayProgress() async throws {
        try await setIntSetting("daily_new_seen", value: 0)
        try await setIntSetting("daily_wrong_count", value: 0)
        try await setIntSetting("daily_reviewed", value: 0)
        try await setIntSetting("daily_mastered", value: 0)
    }

    public func dailyTargetAckedDate() async throws -> String {
        try await setting("daily_target_acked", default: "")
    }

    public func setDailyTargetAcked(now: Date = Date()) async throws {
        try await setSetting("daily_target_acked", value: Self.todayKey(now: now))
    }

    // MARK: - Review queries

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

    public func search(_ text: String, limit: Int = 20) async throws -> [WordEntry] {
        try await ensureSchema()
        try await ensureFTSIndex()
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return [] }
        let tokens = trimmed.split(whereSeparator: \.isWhitespace)
            .map { String($0).replacingOccurrences(of: "\"", with: "\"\"") }
        guard !tokens.isEmpty else { return [] }
        let match = tokens.map { "\"\($0)\"*" }.joined(separator: " ")
        let results = try query("""
        SELECT w.id, w.term, w.phonetic, w.meaning, w.example, w.category, w.familiarity,
               w.review_stage, w.review_count, w.wrong_count, w.next_review_at
        FROM words_fts AS f
        JOIN words AS w ON w.id = f.rowid
        WHERE words_fts MATCH ?
        ORDER BY bm25(words_fts) ASC, w.wrong_count DESC, w.review_count ASC
        LIMIT ?
        """, params: [match, String(limit)])
        if !results.isEmpty { return results }
        let pattern = "%\(trimmed)%"
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
        sqlite3_exec(db, """
        CREATE TABLE IF NOT EXISTS daily_review_log (
            date TEXT PRIMARY KEY, reviewed INTEGER NOT NULL DEFAULT 0,
            learned INTEGER NOT NULL DEFAULT 0, wrong INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        """, nil, nil, nil)
        schemaEnsured = true

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
        let masteredISO: String
        switch familiarity {
        case .known:
            newWrongCount = current.wrongCount
            newFamiliarity = newStage >= ReviewScheduler.intervals.count ? "mastered" : "known"
            masteredISO = newStage >= ReviewScheduler.intervals.count ? lastISO : ""
        case .mastered:
            newWrongCount = current.wrongCount
            newFamiliarity = "mastered"
            masteredISO = lastISO
        case .fuzzy:
            newWrongCount = current.wrongCount
            newFamiliarity = "fuzzy"
            masteredISO = ""
        case .unknown:
            newWrongCount = current.wrongCount + 1
            newFamiliarity = "unknown"
            masteredISO = ""
        }

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

        try updateDailyLogOnReview(db: db!, familiarity: familiarity, now: now)
        WordbookStoreChangeHub.publishDataChanged()
        return try fetchSingle(db: db, id: wordID)
    }

    private func updateDailyLogOnReview(db: OpaquePointer, familiarity: WordFamiliarity, now: Date) throws {
        let today = Self.todayKey(now: now)
        let upsertReviewed = "INSERT INTO settings(key, value) VALUES(?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
        var statement: OpaquePointer?

        if familiarity == .mastered {
            var mastered = 0
            if sqlite3_prepare_v2(db, "SELECT value FROM settings WHERE key = 'daily_mastered'", -1, &statement, nil) == SQLITE_OK {
                if sqlite3_step(statement) == SQLITE_ROW {
                    mastered = Int(String(cString: sqlite3_column_text(statement, 0))) ?? 0
                }
                sqlite3_finalize(statement)
            }
            if sqlite3_prepare_v2(db, upsertReviewed, -1, &statement, nil) == SQLITE_OK {
                sqlite3_bind_text(statement, 1, "daily_mastered", -1, SQLITE_TRANSIENT)
                sqlite3_bind_text(statement, 2, String(mastered + 1), -1, SQLITE_TRANSIENT)
                _ = sqlite3_step(statement)
                sqlite3_finalize(statement)
            }
            return
        }

        let reviewedKey = "daily_reviewed"
        var reviewed = 0
        if sqlite3_prepare_v2(db, "SELECT value FROM settings WHERE key = ?", -1, &statement, nil) == SQLITE_OK {
            sqlite3_bind_text(statement, 1, reviewedKey, -1, SQLITE_TRANSIENT)
            if sqlite3_step(statement) == SQLITE_ROW {
                reviewed = Int(String(cString: sqlite3_column_text(statement, 0))) ?? 0
            }
            sqlite3_finalize(statement)
        }
        if sqlite3_prepare_v2(db, upsertReviewed, -1, &statement, nil) == SQLITE_OK {
            sqlite3_bind_text(statement, 1, reviewedKey, -1, SQLITE_TRANSIENT)
            sqlite3_bind_text(statement, 2, String(reviewed + 1), -1, SQLITE_TRANSIENT)
            _ = sqlite3_step(statement)
            sqlite3_finalize(statement)
        }
        if familiarity == .unknown {
            var wrong = 0
            if sqlite3_prepare_v2(db, "SELECT value FROM settings WHERE key = 'daily_wrong_count'", -1, &statement, nil) == SQLITE_OK {
                if sqlite3_step(statement) == SQLITE_ROW {
                    wrong = Int(String(cString: sqlite3_column_text(statement, 0))) ?? 0
                }
                sqlite3_finalize(statement)
            }
            if sqlite3_prepare_v2(db, upsertReviewed, -1, &statement, nil) == SQLITE_OK {
                sqlite3_bind_text(statement, 1, "daily_wrong_count", -1, SQLITE_TRANSIENT)
                sqlite3_bind_text(statement, 2, String(wrong + 1), -1, SQLITE_TRANSIENT)
                _ = sqlite3_step(statement)
                sqlite3_finalize(statement)
            }
        }
        let logSQL = """
        INSERT INTO daily_review_log(date, reviewed, learned, wrong)
        VALUES(?, 1, 0, ?)
        ON CONFLICT(date) DO UPDATE SET reviewed = reviewed + 1, wrong = wrong + excluded.wrong
        """
        if sqlite3_prepare_v2(db, logSQL, -1, &statement, nil) == SQLITE_OK {
            sqlite3_bind_text(statement, 1, today, -1, SQLITE_TRANSIENT)
            sqlite3_bind_int(statement, 2, familiarity == .unknown ? 1 : 0)
            _ = sqlite3_step(statement)
            sqlite3_finalize(statement)
        }
    }

    private func streakDays() async throws -> Int {
        let db = try openReadConnection()
        var statement: OpaquePointer?
        let sql = "SELECT date FROM daily_review_log WHERE reviewed > 0 ORDER BY date DESC LIMIT 60"
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else { return 0 }
        defer { sqlite3_finalize(statement) }
        var dates: [String] = []
        while sqlite3_step(statement) == SQLITE_ROW {
            dates.append(text(statement, 0))
        }
        guard !dates.isEmpty else { return 0 }
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        formatter.timeZone = .current
        var streak = 0
        var cursor = Calendar.current.startOfDay(for: Date())
        for dateStr in dates {
            guard let date = formatter.date(from: dateStr) else { break }
            let day = Calendar.current.startOfDay(for: date)
            if day == cursor {
                streak += 1
                cursor = Calendar.current.date(byAdding: .day, value: -1, to: cursor) ?? cursor
            } else if day < cursor {
                break
            }
        }
        return streak
    }

    private func ensureSchema() async throws {
        guard !schemaEnsured else { return }
        try await write { db in
            let sql = """
            CREATE TABLE IF NOT EXISTS daily_review_log (
                date TEXT PRIMARY KEY,
                reviewed INTEGER NOT NULL DEFAULT 0,
                learned INTEGER NOT NULL DEFAULT 0,
                wrong INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            """
            sqlite3_exec(db, sql, nil, nil, nil)
        }
        schemaEnsured = true
        try await ensureFTSIndex()
    }

    private func ensureFTSIndex() async throws {
        guard !ftsEnsured else { return }
        try await write { db in
            sqlite3_exec(
                db,
                """
                CREATE VIRTUAL TABLE IF NOT EXISTS words_fts USING fts5(
                    term, meaning, example, category,
                    content='words', content_rowid='id'
                );
                """,
                nil,
                nil,
                nil
            )
            let ddl = [
                "DROP TRIGGER IF EXISTS words_fts_ai",
                "DROP TRIGGER IF EXISTS words_fts_ad",
                "DROP TRIGGER IF EXISTS words_fts_au",
                """
                CREATE TRIGGER words_fts_ai AFTER INSERT ON words BEGIN
                  INSERT INTO words_fts(rowid, term, meaning, example, category)
                  VALUES (new.id, new.term, new.meaning, new.example, new.category);
                END
                """,
                """
                CREATE TRIGGER words_fts_ad AFTER DELETE ON words BEGIN
                  INSERT INTO words_fts(words_fts, rowid, term, meaning, example, category)
                  VALUES ('delete', old.id, old.term, old.meaning, old.example, old.category);
                END
                """,
                """
                CREATE TRIGGER words_fts_au AFTER UPDATE ON words BEGIN
                  INSERT INTO words_fts(words_fts, rowid, term, meaning, example, category)
                  VALUES ('delete', old.id, old.term, old.meaning, old.example, old.category);
                  INSERT INTO words_fts(rowid, term, meaning, example, category)
                  VALUES (new.id, new.term, new.meaning, new.example, new.category);
                END
                """
            ]
            for statement in ddl {
                sqlite3_exec(db, statement, nil, nil, nil)
            }
            sqlite3_exec(db, "INSERT INTO words_fts(words_fts) VALUES('rebuild')", nil, nil, nil)
        }
        ftsEnsured = true
        invalidateReadConnection()
    }

    private func write(_ block: (OpaquePointer) throws -> Void) async throws {
        invalidateReadConnection()
        var db: OpaquePointer?
        guard sqlite3_open_v2(dbURL.path, &db, SQLITE_OPEN_READWRITE, nil) == SQLITE_OK else {
            throw WordbookStoreError.openFailed
        }
        defer { sqlite3_close(db) }
        try block(db!)
    }

    private func scalarInt(_ db: OpaquePointer, sql: String, bind: String? = nil) throws -> Int {
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK else { return 0 }
        defer { sqlite3_finalize(statement) }
        if let bind {
            sqlite3_bind_text(statement, 1, bind, -1, SQLITE_TRANSIENT)
        }
        return sqlite3_step(statement) == SQLITE_ROW ? Int(sqlite3_column_int(statement, 0)) : 0
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
        WordbookDateFormat.iso(date)
    }
}
