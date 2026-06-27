import Foundation

public struct TabRecord: Sendable, Hashable {
    public let bundleID: String
    public let browserName: String
    public let windowIndex: Int
    public let tabIndex: Int
    public let title: String
    public let url: String

    public init(bundleID: String, browserName: String, windowIndex: Int, tabIndex: Int, title: String, url: String) {
        self.bundleID = bundleID
        self.browserName = browserName
        self.windowIndex = windowIndex
        self.tabIndex = tabIndex
        self.title = title
        self.url = url
    }
}

public protocol BrowserAdapter: Sendable {
    var bundleID: String { get }
    var applicationName: String { get }
    func fetchTabs(runner: AppleScriptRunner) async throws -> [TabRecord]
    func activate(record: TabRecord, runner: AppleScriptRunner) async throws
}

public enum BrowserTabParser {
    public static func parseTSV(_ output: String, bundleID: String, browserName: String) -> [TabRecord] {
        output.split(separator: "\n").compactMap { line in
            let parts = line.split(separator: "\t", omittingEmptySubsequences: false)
            guard parts.count >= 4,
                  let windowIndex = Int(parts[0]),
                  let tabIndex = Int(parts[1]) else { return nil }
            return TabRecord(
                bundleID: bundleID,
                browserName: browserName,
                windowIndex: windowIndex,
                tabIndex: tabIndex,
                title: String(parts[2]),
                url: String(parts[3])
            )
        }
    }
}
