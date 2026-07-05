import Foundation

public protocol RunningApplicationsClient: Sendable {
    func runningBundleIDs() async -> Set<String>
    func startMonitoring() async
    func stopMonitoring() async
}

public struct NoopRunningApplicationsClient: RunningApplicationsClient {
    public init() {}

    public func runningBundleIDs() async -> Set<String> { [] }

    public func startMonitoring() async {}

    public func stopMonitoring() async {}
}
