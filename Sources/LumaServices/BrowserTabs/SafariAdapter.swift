import Foundation

public struct SafariAdapter: BrowserAdapter {
    public let bundleID = "com.apple.Safari"
    public let applicationName = "Safari"

    public init() {}

    public func fetchTabs(runner: AppleScriptRunner) async throws -> [TabRecord] {
        let script = """
        tell application "Safari"
          set delim to ASCII character 9
          set out to ""
          repeat with w from 1 to count of windows
            set tabsOfW to tabs of window w
            repeat with t from 1 to count of tabsOfW
              set theTab to item t of tabsOfW
              set out to out & w & delim & t & delim & (name of theTab) & delim & (URL of theTab) & linefeed
            end repeat
          end repeat
          return out
        end tell
        """
        return BrowserTabParser.parseTSV(try await runner.run(script), bundleID: bundleID, browserName: applicationName)
    }

    public func activate(record: TabRecord, runner: AppleScriptRunner) async throws {
        _ = try await runner.run(BrowserTabActivationScripts.safari(record: record))
    }
}
