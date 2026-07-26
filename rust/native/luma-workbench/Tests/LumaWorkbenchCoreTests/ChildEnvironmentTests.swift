import XCTest

@testable import LumaWorkbenchCore

final class ChildEnvironmentTests: XCTestCase {
    private let home = "/Users/me"

    func testInheritedPathKeepsPriorityAndStandardEntriesAreAppended() {
        let path = ChildEnvironment.makePath(inherited: "/custom/bin:/usr/bin", homeDirectory: home)
        let entries = path.split(separator: ":").map(String.init)

        XCTAssertEqual(entries.first, "/custom/bin")
        XCTAssertEqual(entries[1], "/usr/bin")
        XCTAssertTrue(entries.contains("/opt/homebrew/bin"))
        XCTAssertTrue(entries.contains("/Users/me/.cargo/bin"))
        XCTAssertTrue(entries.contains("/sbin"))
    }

    func testPathEntriesAreDeduplicated() {
        let path = ChildEnvironment.makePath(
            inherited: "/usr/bin:/opt/homebrew/bin:/usr/bin::/opt/homebrew/bin",
            homeDirectory: home
        )
        let entries = path.split(separator: ":").map(String.init)

        XCTAssertEqual(entries.count, Set(entries).count, "duplicate PATH entries: \(entries)")
        XCTAssertEqual(entries.filter { $0 == "/usr/bin" }.count, 1)
        XCTAssertFalse(entries.contains(""))
    }

    /// A GUI launch can arrive with no PATH at all; the child still needs a usable one.
    func testEmptyInheritedPathStillYieldsStandardEntries() {
        let path = ChildEnvironment.makePath(inherited: nil, homeDirectory: home)
        XCTAssertEqual(path, ChildEnvironment.standardPathEntries(homeDirectory: home).joined(separator: ":"))
    }

    func testTerminalVariablesAreAlwaysSet() {
        let environment = ChildEnvironment.make(inherited: [:], homeDirectory: home)
        XCTAssertTrue(environment.contains("TERM=xterm-256color"))
        XCTAssertTrue(environment.contains("COLORTERM=truecolor"))
        XCTAssertTrue(environment.contains("HOME=/Users/me"))
    }

    func testPreservesSafeVariablesAndDropsEverythingElse() {
        let inherited = [
            "USER": "me",
            "LOGNAME": "me",
            "SHELL": "/bin/zsh",
            "LANG": "en_US.UTF-8",
            "LC_ALL": "en_US.UTF-8",
            "LC_CTYPE": "en_US.UTF-8",
            "TMPDIR": "/var/folders/tmp/",
            "SSH_AUTH_SOCK": "/private/tmp/agent.sock",
            "SOME_SECRET_TOKEN": "nope",
            "XPC_SERVICE_NAME": "com.luma.next.workbench"
        ]
        let environment = ChildEnvironment.make(inherited: inherited, homeDirectory: home)
        let names = environment.compactMap { $0.split(separator: "=", maxSplits: 1).first.map(String.init) }

        XCTAssertTrue(names.contains("USER"))
        XCTAssertTrue(names.contains("LOGNAME"))
        XCTAssertTrue(names.contains("SHELL"))
        XCTAssertTrue(names.contains("LANG"))
        XCTAssertTrue(names.contains("LC_ALL"))
        XCTAssertTrue(names.contains("LC_CTYPE"))
        XCTAssertTrue(names.contains("TMPDIR"))
        XCTAssertTrue(names.contains("SSH_AUTH_SOCK"))
        XCTAssertFalse(names.contains("SOME_SECRET_TOKEN"))
        XCTAssertFalse(names.contains("XPC_SERVICE_NAME"))
    }

    func testOutputIsSortedAndHasNoDuplicateKeys() {
        let environment = ChildEnvironment.make(
            inherited: ["USER": "me", "LANG": "en_US.UTF-8", "HOME": "/Users/someone-else"],
            homeDirectory: home
        )
        let names = environment.compactMap { $0.split(separator: "=", maxSplits: 1).first.map(String.init) }

        XCTAssertEqual(names, names.sorted())
        XCTAssertEqual(names.count, Set(names).count)
        // HOME must match the directory the child is started in, not an inherited stale value.
        XCTAssertTrue(environment.contains("HOME=/Users/me"))
    }
}
