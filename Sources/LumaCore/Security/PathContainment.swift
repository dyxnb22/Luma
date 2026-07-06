import Foundation

public enum PathContainmentError: Error, Sendable, Equatable {
    case pathOutsideRoot
}

public enum PathContainment {
    public static func validateContained(path: String, in root: URL) throws {
        let rootResolved = root.standardizedFileURL.resolvingSymlinksInPath().standardizedFileURL
        let target = URL(fileURLWithPath: path).standardizedFileURL.resolvingSymlinksInPath().standardizedFileURL
        let rootPath = rootResolved.path
        let targetPath = target.path
        guard targetPath == rootPath || targetPath.hasPrefix(rootPath + "/") else {
            throw PathContainmentError.pathOutsideRoot
        }
    }
}
