import Foundation

public struct FSChangeEvent: Sendable, Hashable {
    public enum Kind: Sendable { case created, removed, renamed, modified, overflow, unknown }
    public let path: String
    public let kind: Kind

    public init(path: String, kind: Kind) {
        self.path = path
        self.kind = kind
    }
}
