import Foundation

public enum MarkdownContentScanner {
    public static func scanFiles(
        containing needle: String,
        in files: [URL],
        limit: Int,
        caseInsensitive: Bool = true,
        maxConcurrentScans: Int = 8
    ) async -> [URL] {
        guard limit > 0, !needle.isEmpty else { return [] }
        let searchNeedle = caseInsensitive ? needle.lowercased() : needle
        var hits: [URL] = []
        var iterator = files.makeIterator()
        let workerCount = max(1, maxConcurrentScans)

        await withTaskGroup(of: URL?.self) { group in
            func enqueue(_ file: URL) {
                group.addTask {
                    guard !Task.isCancelled else { return nil }
                    guard let data = try? String(contentsOf: file, encoding: .utf8) else { return nil }
                    guard !Task.isCancelled else { return nil }
                    let haystack = caseInsensitive ? data.lowercased() : data
                    return haystack.contains(searchNeedle) ? file : nil
                }
            }

            for _ in 0..<workerCount {
                guard let file = iterator.next() else { break }
                enqueue(file)
            }

            while let match = await group.next() {
                if let match {
                    hits.append(match)
                    if hits.count >= limit {
                        group.cancelAll()
                        break
                    }
                }
                if let file = iterator.next(), hits.count < limit {
                    enqueue(file)
                }
            }
        }

        return Array(hits.prefix(limit))
    }
}
