import Foundation

/// Product guardrails for contextual home suggestions.
public enum HomeSuggestionPolicy {
    public static let maxContinueRows = 2
    public static let maxCreateRowsDefault = 1
    public static let maxCreateRowsWithProject = 2
    public static let maxUtilityCreateRows = 1
    public static let maxSetupRows = 2
    public static let maxResumeRows = 3
    public static let maxMergedContinueRows = 4
}
