import Foundation

public struct Query: Sendable, Hashable {
    public let raw: String
    public let normalized: String
    public let tokens: [Substring]
    public let issuedAt: ContinuousClock.Instant
    public let sequence: UInt64
    public let command: ParsedCommand?

    public init(
        raw: String,
        sequence: UInt64,
        command: ParsedCommand? = nil,
        clock: ContinuousClock = .init()
    ) {
        self.raw = raw
        self.normalized = raw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        self.tokens = Self.tokenize(raw)
        self.issuedAt = clock.now
        self.sequence = sequence
        self.command = command
    }

    public var commandPayload: String? {
        command?.payload
    }

    public static func tokenize(_ raw: String) -> [Substring] {
        raw.lowercased().split(whereSeparator: { $0.isWhitespace })
    }
}
