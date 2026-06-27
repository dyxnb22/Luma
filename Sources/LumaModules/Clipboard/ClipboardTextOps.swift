import Foundation

public enum ClipboardTextKind: String, Sendable {
  case plain
  case url
  case json
  case markdown
  case code
}

public enum ClipboardTextOps {
  public static func classify(_ text: String) -> ClipboardTextKind {
    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty else { return .plain }
    if URLTextParser.firstHTTPURL(in: trimmed) != nil, trimmed.split(whereSeparator: \.isWhitespace).count == 1 {
      return .url
    }
    if detectJSON(trimmed) != nil { return .json }
    if trimmed.hasPrefix("#") || trimmed.contains("```") || trimmed.contains("](") {
      return .markdown
    }
    if trimmed.contains("{") && trimmed.contains("}") && (trimmed.contains("func ") || trimmed.contains("def ") || trimmed.contains("class ")) {
      return .code
    }
    return .plain
  }

  public static func trimWhitespace(_ text: String) -> String {
    text.trimmingCharacters(in: .whitespacesAndNewlines)
  }

  public static func collapseLines(_ text: String) -> String {
    text
      .split(whereSeparator: \.isNewline)
      .map { $0.trimmingCharacters(in: .whitespaces) }
      .filter { !$0.isEmpty }
      .joined(separator: " ")
  }

  public static func quoteLines(_ text: String) -> String {
    text
      .split(separator: "\n", omittingEmptySubsequences: false)
      .map { line in
        let s = String(line)
        return s.hasPrefix("> ") ? s : "> \(s)"
      }
      .joined(separator: "\n")
  }

  public static func unquoteLines(_ text: String) -> String {
    text
      .split(separator: "\n", omittingEmptySubsequences: false)
      .map { line in
        var s = String(line)
        if s.hasPrefix("> ") { s.removeFirst(2) }
        else if s == ">" { s = "" }
        return s
      }
      .joined(separator: "\n")
  }

  public static func copyAsOneLine(_ text: String) -> String {
    collapseLines(text)
  }

  public static func detectJSON(_ text: String) -> String? {
    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
    guard trimmed.hasPrefix("{") || trimmed.hasPrefix("[") else { return nil }
    guard let data = trimmed.data(using: .utf8),
          let object = try? JSONSerialization.jsonObject(with: data),
          let pretty = try? JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted, .sortedKeys]),
          let output = String(data: pretty, encoding: .utf8) else { return nil }
    return output
  }

  public static func decodeBase64(_ text: String) -> String? {
    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
    guard trimmed.count >= 8, trimmed.count % 4 == 0,
          let data = Data(base64Encoded: trimmed),
          let output = String(data: data, encoding: .utf8),
          !output.isEmpty else { return nil }
    return output
  }
}

/// Backward-compatible alias used by home suggestions.
public enum ClipboardTransform {
  public static func detectJSON(_ text: String) -> String? {
    ClipboardTextOps.detectJSON(text)
  }

  public static func decodeBase64(_ text: String) -> String? {
    ClipboardTextOps.decodeBase64(text)
  }
}
