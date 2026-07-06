import Foundation

public protocol LauncherHomeProvider: Sendable {
    func items() async -> [ResultItem]
    func isWarming() async -> Bool
}
