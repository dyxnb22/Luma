import Foundation

/// Validates script executables and trims environment for subprocess launches.
public enum ScriptRunnerSecurityPolicy {
    public static let allowedEnvironmentKeys: Set<String> = [
        "PATH", "HOME", "USER", "SHELL", "LANG", "LC_ALL", "TMPDIR", "LOGNAME"
    ]

    public enum ValidationError: Error, Equatable, Sendable {
        case emptyExecutable
        case pathNotAllowed(String)
    }

    /// Returns allowed absolute paths: `~/.luma/commands` and Application Support commands dir.
    public static func defaultAllowedDirectories() -> [URL] {
        var dirs: [URL] = []
        if let home = FileManager.default.homeDirectoryForCurrentUser as URL? {
            dirs.append(home.appendingPathComponent(".luma/commands", isDirectory: true))
        }
        if let appSupport = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        ).first {
            dirs.append(appSupport.appendingPathComponent("Luma/commands", isDirectory: true))
        }
        return dirs
    }

    public static func validateExecutable(_ path: String, allowedDirectories: [URL] = defaultAllowedDirectories()) throws {
        let trimmed = path.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { throw ValidationError.emptyExecutable }
        let expanded = (trimmed as NSString).expandingTildeInPath
        let url = URL(fileURLWithPath: expanded).standardizedFileURL
        let resolved = url.resolvingSymlinksInPath().standardizedFileURL
        guard resolved.isFileURL else { throw ValidationError.pathNotAllowed(expanded) }
        let allowed = allowedDirectories.map {
            $0.standardizedFileURL.resolvingSymlinksInPath().standardizedFileURL
        }
        let isAllowed = allowed.contains { dir in
            resolved.path == dir.path || resolved.path.hasPrefix(dir.path + "/")
        }
        guard isAllowed else { throw ValidationError.pathNotAllowed(expanded) }
    }

    public static func sanitizedEnvironment(from source: [String: String] = ProcessInfo.processInfo.environment) -> [String: String] {
        var result: [String: String] = [:]
        for key in allowedEnvironmentKeys {
            if let value = source[key] {
                result[key] = value
            }
        }
        return result
    }

    public static func redactedRunMetadata(executable: String, exitCode: Int32) -> String {
        let name = URL(fileURLWithPath: executable).lastPathComponent
        return "script.run executable=\(name) exit=\(exitCode)"
    }
}
