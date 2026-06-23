import Foundation

public enum PinyinIndex {
    public static func full(from inputs: [String]) -> String {
        let combined = inputs.joined(separator: " ")
        guard !combined.isEmpty else { return "" }
        let mutable = NSMutableString(string: combined) as CFMutableString
        guard CFStringTransform(mutable, nil, kCFStringTransformMandarinLatin, false) else {
            return combined.lowercased()
        }
        CFStringTransform(mutable, nil, kCFStringTransformStripDiacritics, false)
        return (mutable as String).lowercased()
    }

    public static func initials(from inputs: [String]) -> String {
        full(from: inputs)
            .split(separator: " ")
            .compactMap(\.first)
            .map(String.init)
            .joined()
    }
}
