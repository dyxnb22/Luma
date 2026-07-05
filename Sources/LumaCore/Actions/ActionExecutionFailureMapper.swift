import Foundation

public enum ActionExecutionFailureMapper {
    public static func message(for error: Error) -> (message: String?, recoverable: Bool) {
        if let moduleError = error as? ModuleError {
            switch moduleError {
            case .unsupportedAction:
                return ("This action is not supported.", true)
            case .dataUnavailable:
                return ("Required data is unavailable.", true)
            case .actionTimedOut:
                return ("The action took too long and was cancelled.", true)
            case .permissionRequired(let permission):
                switch permission {
                case .accessibility:
                    return ("Accessibility permission is required.", true)
                case .automation:
                    return ("Automation permission is required.", true)
                }
            }
        }
        return (nil, true)
    }
}
