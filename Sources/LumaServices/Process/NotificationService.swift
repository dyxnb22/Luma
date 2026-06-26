import Foundation
import LumaCore
import UserNotifications

public actor NotificationService: NotificationClient {
    private var authorized = false

    public init() {}

    public func post(title: String, body: String) async {
        await ensureAuthorized()
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = String(body.prefix(256))
        let request = UNNotificationRequest(
            identifier: UUID().uuidString,
            content: content,
            trigger: nil
        )
        try? await UNUserNotificationCenter.current().add(request)
    }

    private func ensureAuthorized() async {
        guard !authorized else { return }
        let center = UNUserNotificationCenter.current()
        let granted = try? await center.requestAuthorization(options: [.alert, .sound])
        authorized = granted == true
    }
}
