import Foundation

public protocol LauncherHomeProvider: Sendable {
    func items() async -> [ResultItem]
}
