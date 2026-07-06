import Foundation
import LumaCore
import LumaServices

enum SystemSettingsOpener {
    @MainActor
    static func open(
        _ url: URL,
        workspace: any WorkspaceClient = WorkspaceService(),
        onFailure: ((String) -> Void)? = nil
    ) async {
        do {
            try await workspace.openURL(url)
        } catch {
            onFailure?(LauncherStatusMessages.operationFailed)
        }
    }
}
