import Foundation
import XCTest

@testable import LumaWorkbenchCore

final class SingleInstanceLockTests: XCTestCase {
    private func temporaryLockURL() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString)
            .appendingPathExtension("lock")
    }

    func testOnlyOneOwnerCanHoldTheSessionLock() throws {
        let url = temporaryLockURL()
        defer { try? FileManager.default.removeItem(at: url) }

        let first = try XCTUnwrap(SingleInstanceLock.acquire(at: url))
        XCTAssertNil(try SingleInstanceLock.acquire(at: url))
        withExtendedLifetime(first) {}
    }

    func testLockCanBeReacquiredAfterRelease() throws {
        let url = temporaryLockURL()
        defer { try? FileManager.default.removeItem(at: url) }

        let first = try XCTUnwrap(SingleInstanceLock.acquire(at: url))
        first.release()

        XCTAssertNotNil(try SingleInstanceLock.acquire(at: url))
    }

    func testLockPathIsStableAndScopedToTheTemporaryDirectory() {
        let directory = URL(fileURLWithPath: "/private/tmp/luma-tests", isDirectory: true)

        XCTAssertEqual(
            HostIdentity.singleInstanceLockURL(temporaryDirectory: directory).path,
            "/private/tmp/luma-tests/com.luma.next.workbench.session.lock"
        )
    }
}
