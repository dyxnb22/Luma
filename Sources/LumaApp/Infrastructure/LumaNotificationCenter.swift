import Foundation

/// Block-based NotificationCenter observer that hops to MainActor — safe for @MainActor detail controllers.
enum LumaNotificationCenter {
  static func observe(
    name: Notification.Name,
    object: Any? = nil,
    handler: @escaping @MainActor () -> Void
  ) -> NSObjectProtocol {
    NotificationCenter.default.addObserver(forName: name, object: object, queue: nil) { _ in
      Task { @MainActor in
        handler()
      }
    }
  }
}
