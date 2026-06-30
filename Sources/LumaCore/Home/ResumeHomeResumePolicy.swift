import Foundation

/// Eligibility rules for module-level resume rows on the home screen.
public enum ResumeHomeResumePolicy {
    /// Module resume is eligible only when no draft resume row was actually shown.
    public static func allowsModuleResume(
        snippetDraftRowShown: Bool,
        quicklinkDraftRowShown: Bool
    ) -> Bool {
        !snippetDraftRowShown && !quicklinkDraftRowShown
    }
}
