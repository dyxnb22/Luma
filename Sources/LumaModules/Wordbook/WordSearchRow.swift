import Foundation

struct WordSearchRow: Sendable, Hashable {
    let id: Int64
    let term: String
    let phonetic: String
    let meaning: String
    let example: String
    let category: String
    let haystack: String

    init(id: Int64, term: String, phonetic: String, meaning: String, example: String, category: String) {
        self.id = id
        self.term = term
        self.phonetic = phonetic
        self.meaning = meaning
        self.example = example
        self.category = category
        self.haystack = [term, meaning, example, category, phonetic]
            .joined(separator: " ")
            .lowercased()
    }
}
