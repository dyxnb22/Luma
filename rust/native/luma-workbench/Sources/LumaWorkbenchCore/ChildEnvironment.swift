import Foundation

/// Environment handed to the `luma tui` child process.
///
/// A GUI-launched app inherits a minimal launchd environment, so the TUI would otherwise fail to
/// find `ssh`, `git`, or anything installed by Homebrew or cargo when it runs a Command Recipe.
public enum ChildEnvironment {
    /// Variables copied through from the host process when present.
    ///
    /// `SSH_AUTH_SOCK` is included so `/ssh` can use the running agent instead of prompting for a
    /// passphrase on every connection.
    public static let preservedNames: Set<String> = [
        "HOME", "USER", "LOGNAME", "SHELL", "LANG", "TMPDIR", "SSH_AUTH_SOCK"
    ]

    /// Locations appended to the inherited `PATH`, in priority order after it.
    public static func standardPathEntries(homeDirectory: String) -> [String] {
        [
            URL(fileURLWithPath: homeDirectory).appendingPathComponent(".cargo/bin").path,
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin"
        ]
    }

    /// Inherited entries keep their position and priority; the standard locations fill the gaps.
    /// Empty and duplicate entries are dropped, first occurrence wins.
    public static func makePath(inherited: String?, homeDirectory: String) -> String {
        let inheritedEntries = (inherited ?? "").split(separator: ":", omittingEmptySubsequences: true)
            .map(String.init)
        var seen = Set<String>()
        var ordered: [String] = []
        for entry in inheritedEntries + standardPathEntries(homeDirectory: homeDirectory) {
            let trimmed = entry.trimmingCharacters(in: .whitespaces)
            guard !trimmed.isEmpty, seen.insert(trimmed).inserted else { continue }
            ordered.append(trimmed)
        }
        return ordered.joined(separator: ":")
    }

    /// Builds the `KEY=VALUE` list SwiftTerm passes to `execve`.
    ///
    /// Sorted by key so the result is deterministic and diffable in tests.
    public static func make(inherited: [String: String], homeDirectory: String) -> [String] {
        var result: [String: String] = [
            "TERM": "xterm-256color",
            "COLORTERM": "truecolor",
            "HOME": homeDirectory,
            "PATH": makePath(inherited: inherited["PATH"], homeDirectory: homeDirectory)
        ]
        for (key, value) in inherited {
            // HOME stays authoritative: it must agree with the working directory the child starts in.
            guard key != "HOME" else { continue }
            guard preservedNames.contains(key) || key.hasPrefix("LC_") else { continue }
            result[key] = value
        }
        return result
            .sorted { $0.key < $1.key }
            .map { "\($0.key)=\($0.value)" }
    }
}
