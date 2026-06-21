import Foundation
import LumaCore

public actor ApplicationSupportPaths: DatabaseClient {
    public let applicationSupportURL: URL

    public init(fileManager: FileManager = .default) {
        let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        self.applicationSupportURL = base.appendingPathComponent("Luma", isDirectory: true)
        try? fileManager.createDirectory(at: applicationSupportURL, withIntermediateDirectories: true)
    }
}
