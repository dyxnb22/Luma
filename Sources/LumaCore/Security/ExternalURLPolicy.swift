import Foundation

public enum ExternalURLPolicyError: Error, Sendable, Equatable {
    case schemeNotAllowed(String)
}

public enum ExternalURLPolicy {
    private static let allowedSchemes: Set<String> = ["http", "https", "mailto", "x-apple.systempreferences"]

    /// Validates URLs opened via NSWorkspace. File URLs require explicit opt-in.
    public static func validateOpenURL(_ url: URL, allowFileURLs: Bool = false) throws {
        guard let scheme = url.scheme?.lowercased(), !scheme.isEmpty else {
            throw ExternalURLPolicyError.schemeNotAllowed("(missing)")
        }
        if scheme == "file" {
            guard allowFileURLs else {
                throw ExternalURLPolicyError.schemeNotAllowed("file")
            }
            return
        }
        guard allowedSchemes.contains(scheme) else {
            throw ExternalURLPolicyError.schemeNotAllowed(scheme)
        }
    }
}
