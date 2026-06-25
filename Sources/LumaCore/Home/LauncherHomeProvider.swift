import Foundation

public protocol LauncherHomeProvider: Sendable {
    func items() async -> [ResultItem]
    func warmup() async
}

public extension LauncherHomeProvider {
    func warmup() async {}
}
