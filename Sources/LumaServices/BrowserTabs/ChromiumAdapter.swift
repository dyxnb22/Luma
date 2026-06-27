import Foundation

public struct ChromiumAdapter: BrowserAdapter {
    public let bundleID: String
    public let applicationName: String

    public init(bundleID: String, applicationName: String) {
        self.bundleID = bundleID
        self.applicationName = applicationName
    }

    public func fetchTabs(runner: AppleScriptRunner) async throws -> [TabRecord] {
        let appName = applicationName
        let script = """
        tell application "\(appName)"
          set delim to ASCII character 9
          set out to ""
          repeat with w from 1 to count of windows
            set tabsOfW to tabs of window w
            repeat with t from 1 to count of tabsOfW
              set theTab to item t of tabsOfW
              set out to out & w & delim & t & delim & (title of theTab) & delim & (URL of theTab) & linefeed
            end repeat
          end repeat
          return out
        end tell
        """
        return BrowserTabParser.parseTSV(try await runner.run(script), bundleID: bundleID, browserName: appName)
    }

    public func activate(record: TabRecord, runner: AppleScriptRunner) async throws {
        let appName = applicationName
        let script = """
        tell application "\(appName)"
          set index of window \(record.windowIndex) to 1
          tell window \(record.windowIndex) to set active tab index to \(record.tabIndex)
          activate
        end tell
        """
        _ = try await runner.run(script)
    }
}
