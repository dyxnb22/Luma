import Foundation

public enum FuzzyMatcher {
    public static func score(query: String, target: String) -> Double {
        guard !query.isEmpty else { return 0.2 }
        guard let first = query.first, target.contains(first) else { return 0 }

        let queryChars = Array(query)
        let targetChars = Array(target)
        var queryIndex = 0
        var rawScore = 0.0
        var consecutive = 0.0

        for targetIndex in targetChars.indices {
            guard queryIndex < queryChars.count else { break }
            guard targetChars[targetIndex] == queryChars[queryIndex] else {
                consecutive = 0
                continue
            }

            rawScore += 0.35 + consecutive * 0.15
            if targetIndex == 0 {
                rawScore += 0.4
            } else if isBoundary(previous: targetChars[targetIndex - 1], current: targetChars[targetIndex]) {
                rawScore += 0.25
            }

            consecutive += 1
            queryIndex += 1
        }

        guard queryIndex == queryChars.count else { return 0 }

        let normalized = rawScore / max(1.0, Double(queryChars.count) * 1.1)
        return min(1.0, normalized)
    }

    private static func isBoundary(previous: Character, current: Character) -> Bool {
        if " /-_.".contains(previous) { return true }
        return previous.isLowercase && current.isUppercase
    }
}
