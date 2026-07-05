import Foundation
import SQLite3
@testable import LumaModules

enum WordbookTestFixtures {
    static func makeStore() throws -> (store: WordbookStore, url: URL) {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("wordbook-test-\(UUID().uuidString).sqlite3")
        try createSchema(at: url)
        return (WordbookStore(dbURL: url), url)
    }

    static func createSchema(at url: URL) throws {
        var db: OpaquePointer?
        guard sqlite3_open(url.path, &db) == SQLITE_OK else {
            throw WordbookStoreError.openFailed
        }
        defer { sqlite3_close(db) }
        let sql = """
        CREATE TABLE words (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            term TEXT NOT NULL UNIQUE,
            phonetic TEXT DEFAULT '',
            meaning TEXT DEFAULT '',
            example TEXT DEFAULT '',
            category TEXT DEFAULT '',
            familiarity TEXT DEFAULT 'new',
            review_stage INTEGER DEFAULT 0,
            review_count INTEGER DEFAULT 0,
            wrong_count INTEGER DEFAULT 0,
            next_review_at TEXT DEFAULT '',
            created_at TEXT DEFAULT '',
            updated_at TEXT DEFAULT '',
            last_review_at TEXT DEFAULT '',
            mastered_at TEXT DEFAULT ''
        );
        """
        guard sqlite3_exec(db, sql, nil, nil, nil) == SQLITE_OK else {
            throw WordbookStoreError.prepareFailed(message: "schema creation failed")
        }
    }

    static func newWord(term: String, meaning: String = "") -> WordEntry {
        WordEntry(
            id: 0,
            term: term,
            phonetic: "",
            meaning: meaning,
            example: "",
            category: "",
            familiarity: "new",
            reviewStage: 0,
            reviewCount: 0,
            wrongCount: 0,
            nextReviewAt: ""
        )
    }

    static func insertDueWord(at url: URL, term: String, nextReviewAt: String) throws {
        var db: OpaquePointer?
        guard sqlite3_open(url.path, &db) == SQLITE_OK else {
            throw WordbookStoreError.openFailed
        }
        defer { sqlite3_close(db) }
        let now = WordbookDateFormat.iso(Date())
        let sql = """
        INSERT INTO words(term, phonetic, meaning, example, category, familiarity,
            review_stage, review_count, wrong_count, next_review_at, created_at, updated_at)
        VALUES(?, '', '', '', '', 'known', 1, 1, 0, ?, ?, ?)
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            throw WordbookStoreError.prepareFailed(message: "insert due word failed")
        }
        defer { sqlite3_finalize(stmt) }
        sqlite3_bind_text(stmt, 1, term, -1, unsafeBitCast(-1, to: sqlite3_destructor_type.self))
        sqlite3_bind_text(stmt, 2, nextReviewAt, -1, unsafeBitCast(-1, to: sqlite3_destructor_type.self))
        sqlite3_bind_text(stmt, 3, now, -1, unsafeBitCast(-1, to: sqlite3_destructor_type.self))
        sqlite3_bind_text(stmt, 4, now, -1, unsafeBitCast(-1, to: sqlite3_destructor_type.self))
        guard sqlite3_step(stmt) == SQLITE_DONE else {
            throw WordbookStoreError.stepFailed(message: "insert due word step failed")
        }
    }

    static func bulkInsertWords(at url: URL, count: Int, prefix: String = "word") throws {
        var db: OpaquePointer?
        guard sqlite3_open(url.path, &db) == SQLITE_OK else {
            throw WordbookStoreError.openFailed
        }
        defer { sqlite3_close(db) }
        let now = WordbookDateFormat.iso(Date())
        let sql = """
        INSERT INTO words(term, phonetic, meaning, example, category, familiarity,
            review_stage, review_count, wrong_count, next_review_at, created_at, updated_at)
        VALUES(?, '', ?, '', '', 'new', 0, 0, 0, '', ?, ?)
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            throw WordbookStoreError.prepareFailed(message: "bulk insert prepare failed")
        }
        defer { sqlite3_finalize(stmt) }
        for index in 0..<count {
            let term = "\(prefix)\(index)"
            sqlite3_bind_text(stmt, 1, term, -1, unsafeBitCast(-1, to: sqlite3_destructor_type.self))
            sqlite3_bind_text(stmt, 2, "meaning \(index)", -1, unsafeBitCast(-1, to: sqlite3_destructor_type.self))
            sqlite3_bind_text(stmt, 3, now, -1, unsafeBitCast(-1, to: sqlite3_destructor_type.self))
            sqlite3_bind_text(stmt, 4, now, -1, unsafeBitCast(-1, to: sqlite3_destructor_type.self))
            guard sqlite3_step(stmt) == SQLITE_DONE else {
                throw WordbookStoreError.stepFailed(message: "bulk insert step failed")
            }
            sqlite3_reset(stmt)
        }
    }
}
