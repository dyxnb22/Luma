import Foundation

public enum ActionExecutionResult: Sendable, Equatable {
    case success
    case failure(message: String?, recoverable: Bool)

    public var succeeded: Bool {
        if case .success = self { return true }
        return false
    }

    public var userFacingMessage: String? {
        guard case .failure(let message, _) = self else { return nil }
        return message
    }
}
