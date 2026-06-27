import Foundation
import Testing

extension Tag {
    @Tag static var integration: Tag
}

enum IntegrationTestSettings {
    static var enabled: Bool {
        ProcessInfo.processInfo.environment["LUMA_INTEGRATION_TESTS"] == "1"
    }
}
