import Foundation
import LumaCore

public struct CurrentProjectClientAdapter: CurrentProjectClient {
    private let service: CurrentProjectService

    public init(service: CurrentProjectService = .shared) {
        self.service = service
    }

    public func snapshot() async -> CurrentProjectContext? {
        await service.snapshot()
    }
}

public struct SelectionSnapshotClientAdapter: SelectionSnapshotClient {
    private let service: SelectionSnapshotService

    public init(service: SelectionSnapshotService = .shared) {
        self.service = service
    }

    public func snapshot() async -> String? {
        await service.snapshot()
    }
}
