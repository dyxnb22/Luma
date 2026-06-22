import Foundation

public enum SecretsAction: Codable, Sendable {
    case unlockVault
    case copySecret(id: UUID)
}
