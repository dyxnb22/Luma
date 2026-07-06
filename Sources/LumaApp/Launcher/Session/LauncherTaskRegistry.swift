import Foundation

/// Registers unstructured launcher tasks and cancels them when the surface hides.
@MainActor
final class LauncherTaskRegistry {
    private var tasks: [String: Task<Void, Never>] = [:]

    func register(key: String, task: Task<Void, Never>) {
        tasks[key]?.cancel()
        tasks[key] = task
    }

    func cancel(key: String) {
        tasks[key]?.cancel()
        tasks.removeValue(forKey: key)
    }

    func cancelAll() {
        for task in tasks.values {
            task.cancel()
        }
        tasks.removeAll(keepingCapacity: true)
    }

    func contains(key: String) -> Bool {
        tasks[key] != nil
    }
}
