import Foundation

public enum ProjectScanner {
    public static let defaultBudget: Duration = .milliseconds(400)

    public static func scan(
        roots: [String],
        fileManager: FileManager = .default,
        budget: Duration = defaultBudget,
        clock: ContinuousClock = .init()
    ) -> [ProjectRecord] {
        let deadline = clock.now.advanced(by: budget)
        let home = fileManager.homeDirectoryForCurrentUser
        var seenPaths = Set<String>()
        var records: [ProjectRecord] = []

        for root in roots {
            if clock.now >= deadline { break }
            let expanded = expandPath(root, home: home)
            let rootURL = URL(fileURLWithPath: expanded, isDirectory: true)
            guard fileManager.fileExists(atPath: rootURL.path) else { continue }

            if let record = projectRecord(at: rootURL, fileManager: fileManager), seenPaths.insert(record.path).inserted {
                records.append(record)
            }

            guard let children = try? fileManager.contentsOfDirectory(
                at: rootURL,
                includingPropertiesForKeys: [.isDirectoryKey],
                options: [.skipsHiddenFiles]
            ) else { continue }

            for child in children {
                if clock.now >= deadline { break }
                var isDirectory: ObjCBool = false
                guard fileManager.fileExists(atPath: child.path, isDirectory: &isDirectory), isDirectory.boolValue else { continue }

                if let record = projectRecord(at: child, fileManager: fileManager), seenPaths.insert(record.path).inserted {
                    records.append(record)
                }

                guard let grandchildren = try? fileManager.contentsOfDirectory(
                    at: child,
                    includingPropertiesForKeys: [.isDirectoryKey],
                    options: [.skipsHiddenFiles]
                ) else { continue }

                for grandchild in grandchildren {
                    if clock.now >= deadline { break }
                    var grandchildIsDirectory: ObjCBool = false
                    guard fileManager.fileExists(atPath: grandchild.path, isDirectory: &grandchildIsDirectory),
                          grandchildIsDirectory.boolValue else { continue }
                    if let record = projectRecord(at: grandchild, fileManager: fileManager), seenPaths.insert(record.path).inserted {
                        records.append(record)
                    }
                }
            }
        }

        return records.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    static func projectRecord(at url: URL, fileManager: FileManager) -> ProjectRecord? {
        guard isProjectDirectory(url, fileManager: fileManager) else { return nil }
        return ProjectRecord(name: url.lastPathComponent, path: url.path)
    }

    static func isProjectDirectory(_ url: URL, fileManager: FileManager) -> Bool {
        let markers = [
            ".git",
            "Package.swift",
            "package.json",
            "pyproject.toml",
            "Cargo.toml"
        ]
        for marker in markers {
            if fileManager.fileExists(atPath: url.appendingPathComponent(marker).path) {
                return true
            }
        }
        if let children = try? fileManager.contentsOfDirectory(at: url, includingPropertiesForKeys: nil, options: [.skipsHiddenFiles]) {
            if children.contains(where: { $0.pathExtension == "xcodeproj" || $0.pathExtension == "xcworkspace" }) {
                return true
            }
        }
        return false
    }

    static func expandPath(_ path: String, home: URL) -> String {
        if path.hasPrefix("~/") {
            return home.appendingPathComponent(String(path.dropFirst(2))).path
        }
        if path == "~" {
            return home.path
        }
        return (path as NSString).expandingTildeInPath
    }
}
