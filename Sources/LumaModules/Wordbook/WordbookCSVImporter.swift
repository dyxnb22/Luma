import Foundation

public enum WordbookCSVImporter {
    public static func parse(_ text: String) -> [WordEntry] {
        let lines = text.split(whereSeparator: \.isNewline).map(String.init)
        guard !lines.isEmpty else { return [] }
        let delimiter: Character = lines[0].contains("\t") ? "\t" : ","
        var start = 0
        let header = splitLine(lines[0], delimiter: delimiter).map { $0.lowercased() }
        if header.contains("term") || header.contains("word") { start = 1 }
        var results: [WordEntry] = []
        for line in lines.dropFirst(start) {
            let cols = splitLine(line, delimiter: delimiter)
            guard let term = cols.first?.trimmingCharacters(in: .whitespacesAndNewlines), !term.isEmpty else { continue }
            let phonetic = cols.count > 1 ? cols[1] : ""
            let meaning = cols.count > 2 ? cols[2] : ""
            let example = cols.count > 3 ? cols[3] : ""
            let category = cols.count > 4 ? cols[4] : ""
            results.append(WordEntry(
                id: 0,
                term: term,
                phonetic: phonetic,
                meaning: meaning,
                example: example,
                category: category,
                familiarity: "new",
                reviewStage: 0,
                reviewCount: 0,
                wrongCount: 0,
                nextReviewAt: ""
            ))
        }
        return results
    }

    private static func splitLine(_ line: String, delimiter: Character) -> [String] {
        var fields: [String] = []
        var current = ""
        var inQuotes = false
        var index = line.startIndex
        while index < line.endIndex {
            let ch = line[index]
            if ch == "\"" {
                if inQuotes {
                    let next = line.index(after: index)
                    if next < line.endIndex, line[next] == "\"" {
                        current.append("\"")
                        index = line.index(after: next)
                        continue
                    }
                    inQuotes = false
                } else {
                    inQuotes = true
                }
            } else if ch == delimiter && !inQuotes {
                fields.append(current)
                current = ""
            } else {
                current.append(ch)
            }
            index = line.index(after: index)
        }
        fields.append(current)
        return fields
    }

    public static func export(_ entries: [WordEntry]) -> String {
        var lines = ["term,phonetic,meaning,example,category"]
        for entry in entries {
            let cols = [entry.term, entry.phonetic, entry.meaning, entry.example, entry.category].map(escapeField)
            lines.append(cols.joined(separator: ","))
        }
        return lines.joined(separator: "\n")
    }

    private static func escapeField(_ value: String) -> String {
        if value.contains(",") || value.contains("\"") || value.contains("\n") {
            return "\"\(value.replacingOccurrences(of: "\"", with: "\"\""))\""
        }
        return value
    }
}
