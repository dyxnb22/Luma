import Darwin
import Foundation
import Testing
@testable import LumaModules
import LumaServices

@Test func killProcessIndexFiltersSelfPID() {
    let selfPID = pid_t(42)
    let records = [
        RunningProcessRecord(pid: 42, bundleID: "luma", name: "Luma", launchDate: nil, residentBytes: nil),
        RunningProcessRecord(pid: 43, bundleID: "preview", name: "Preview", launchDate: nil, residentBytes: nil)
    ]
    #expect(KillProcessIndex.filtered(records, selfPID: selfPID).map(\.pid) == [43])
}

@Test func killProcessIndexSearchesLocalizedNameAndBundleID() {
    let records = [
        RunningProcessRecord(pid: 10, bundleID: "com.apple.Preview", name: "预览", launchDate: nil, residentBytes: nil)
    ]
    #expect(KillProcessIndex.search(records, query: "preview").first?.record.pid == 10)
    #expect(KillProcessIndex.search(records, query: "预览").first?.record.pid == 10)
}

@Test func killProcessIndexSearchesNameAndBundleID() {
    let records = [
        RunningProcessRecord(pid: 10, bundleID: "com.apple.Preview", name: "Preview", launchDate: nil, residentBytes: nil),
        RunningProcessRecord(pid: 11, bundleID: "com.apple.Safari", name: "Safari", launchDate: nil, residentBytes: nil)
    ]
    #expect(KillProcessIndex.search(records, query: "preview").first?.record.pid == 10)
    #expect(KillProcessIndex.search(records, query: "safari").first?.record.pid == 11)
}

@Test func killProcessIndexFormatsMemory() {
    #expect(KillProcessIndex.memoryDisplay(bytes: 512 * 1_048_576) == "512 MB")
    #expect(KillProcessIndex.memoryDisplay(bytes: 1536 * 1_048_576) == "1.5 GB")
    #expect(KillProcessIndex.memoryDisplay(bytes: nil) == "memory unknown")
}
