import Foundation
import LumaServices

public enum BrowserTabsAction: Codable, Sendable, Hashable {
    case activate(record: CodableTabRecord)
}

public struct CodableTabRecord: Codable, Sendable, Hashable {
    public let bundleID: String
    public let browserName: String
    public let windowIndex: Int
    public let tabIndex: Int
    public let title: String
    public let url: String

    public init(_ record: TabRecord) {
        self.bundleID = record.bundleID
        self.browserName = record.browserName
        self.windowIndex = record.windowIndex
        self.tabIndex = record.tabIndex
        self.title = record.title
        self.url = record.url
    }

    public var tabRecord: TabRecord {
        TabRecord(bundleID: bundleID, browserName: browserName, windowIndex: windowIndex, tabIndex: tabIndex, title: title, url: url)
    }
}
