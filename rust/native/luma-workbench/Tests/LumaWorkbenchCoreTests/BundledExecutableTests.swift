import XCTest

@testable import LumaWorkbenchCore

final class BundledExecutableTests: XCTestCase {
    private let hostExecutable = URL(
        fileURLWithPath: "/Users/me/Applications/Luma.app/Contents/MacOS/LumaWorkbench"
    )

    func testResolvesSiblingOfHostExecutable() {
        let expected = "/Users/me/Applications/Luma.app/Contents/MacOS/luma"
        XCTAssertEqual(BundledExecutable.expectedURL(hostExecutableURL: hostExecutable).path, expected)
    }

    func testResolveSucceedsWhenPresentAndExecutable() {
        let result = BundledExecutable.resolve(
            hostExecutableURL: hostExecutable,
            fileExists: { _ in true },
            isExecutable: { _ in true }
        )
        guard case .success(let url) = result else {
            return XCTFail("expected success, got \(result)")
        }
        XCTAssertEqual(url.lastPathComponent, "luma")
    }

    func testResolveReportsMissingBinary() {
        let result = BundledExecutable.resolve(
            hostExecutableURL: hostExecutable,
            fileExists: { _ in false },
            isExecutable: { _ in true }
        )
        guard case .failure(let error) = result else {
            return XCTFail("expected failure, got \(result)")
        }
        XCTAssertEqual(
            error,
            .missing(expectedPath: "/Users/me/Applications/Luma.app/Contents/MacOS/luma")
        )
        XCTAssertTrue(error.description.contains("build_workbench_app.sh"))
    }

    func testResolveReportsNonExecutableBinary() {
        let result = BundledExecutable.resolve(
            hostExecutableURL: hostExecutable,
            fileExists: { _ in true },
            isExecutable: { _ in false }
        )
        guard case .failure(let error) = result else {
            return XCTFail("expected failure, got \(result)")
        }
        XCTAssertEqual(
            error,
            .notExecutable(path: "/Users/me/Applications/Luma.app/Contents/MacOS/luma")
        )
    }

    /// The host must never fall back to whatever `luma` happens to be on the developer's PATH.
    func testResolutionNeverLeavesTheBundleDirectory() {
        let candidate = BundledExecutable.expectedURL(hostExecutableURL: hostExecutable)
        XCTAssertEqual(
            candidate.deletingLastPathComponent().path,
            hostExecutable.deletingLastPathComponent().path
        )
    }
}
