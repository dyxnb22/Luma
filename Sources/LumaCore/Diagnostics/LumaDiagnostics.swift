import Foundation

public struct LumaDiagnosticIssue: Sendable, Equatable {
  public enum Severity: String, Sendable {
    case info
    case warning
    case error
  }

  public let severity: Severity
  public let moduleID: ModuleIdentifier?
  public let message: String

  public init(severity: Severity, moduleID: ModuleIdentifier? = nil, message: String) {
    self.severity = severity
    self.moduleID = moduleID
    self.message = message
  }
}

public struct LumaDoctorContext: Sendable {
  public let accessibilityTrusted: Bool
  public let remindersAuthorization: RemindersAuthorization?
  public let notesRootConfigured: Bool
  public let enabledModuleCount: Int
  public let totalModuleCount: Int
  public let menuItemsCachedCount: Int

  public init(
    accessibilityTrusted: Bool,
    remindersAuthorization: RemindersAuthorization? = nil,
    notesRootConfigured: Bool,
    enabledModuleCount: Int,
    totalModuleCount: Int,
    menuItemsCachedCount: Int = 0
  ) {
    self.accessibilityTrusted = accessibilityTrusted
    self.remindersAuthorization = remindersAuthorization
    self.notesRootConfigured = notesRootConfigured
    self.enabledModuleCount = enabledModuleCount
    self.totalModuleCount = totalModuleCount
    self.menuItemsCachedCount = menuItemsCachedCount
  }
}

public struct LumaDiagnosticsSummary: Sendable, Equatable {
  public let issues: [LumaDiagnosticIssue]
  public let moduleCount: Int
  public let defaultEnabledCount: Int

  public var isHealthy: Bool {
    !issues.contains { $0.severity == .error }
  }
}

/// Lightweight cross-module diagnostics for settings and doctor-style queries.
public enum LumaDiagnostics {
  public static func summarize(manifests: [ModuleManifest], notesRootConfigured: Bool) -> LumaDiagnosticsSummary {
    summarize(
      manifests: manifests,
      context: LumaDoctorContext(
        accessibilityTrusted: true,
        notesRootConfigured: notesRootConfigured,
        enabledModuleCount: manifests.filter(\.defaultEnabled).count,
        totalModuleCount: manifests.count
      )
    )
  }

  public static func summarize(manifests: [ModuleManifest], context: LumaDoctorContext) -> LumaDiagnosticsSummary {
    var issues: [LumaDiagnosticIssue] = []

    if !context.notesRootConfigured {
      issues.append(LumaDiagnosticIssue(
        severity: .warning,
        moduleID: ModuleIdentifier(rawValue: "luma.notes"),
        message: "Notes root is not configured — daily note capture will redirect to Notes settings."
      ))
    }

    if !context.accessibilityTrusted {
      issues.append(LumaDiagnosticIssue(
        severity: .warning,
        moduleID: ModuleIdentifier(rawValue: "luma.window-layouts"),
        message: "Accessibility is not trusted — window layouts, snippets paste, and menu bar search need it."
      ))
    }

    if let reminders = context.remindersAuthorization, reminders != .authorized {
      let message = reminders == .denied
        ? "Reminders access denied — open System Settings to allow Luma."
        : "Reminders access not granted — run Todo and tap Allow Reminders Access."
      issues.append(LumaDiagnosticIssue(
        severity: .warning,
        moduleID: ModuleIdentifier(rawValue: "luma.todo"),
        message: message
      ))
    }

    if context.menuItemsCachedCount == 0 {
      issues.append(LumaDiagnosticIssue(
        severity: .info,
        moduleID: ModuleIdentifier(rawValue: "luma.menu-items"),
        message: "Menu bar search cache is empty for the frontmost app — results appear after the first scan."
      ))
    }

    let triggers = manifests.map(\.identifier.rawValue)
    if Set(triggers).count != triggers.count {
      issues.append(LumaDiagnosticIssue(
        severity: .error,
        message: "Duplicate module identifiers in manifest catalog."
      ))
    }

    let enabled = manifests.filter(\.defaultEnabled)
    if context.enabledModuleCount < context.totalModuleCount {
      issues.append(LumaDiagnosticIssue(
        severity: .info,
        message: "\(context.enabledModuleCount) of \(context.totalModuleCount) modules enabled."
      ))
    }

    return LumaDiagnosticsSummary(
      issues: issues,
      moduleCount: manifests.count,
      defaultEnabledCount: enabled.count
    )
  }

  public static func doctorRows(
    from summary: LumaDiagnosticsSummary,
    module: ModuleIdentifier,
    basePriority: Int
  ) -> [ResultItem] {
    var rows: [ResultItem] = [
      ResultItem(
        id: ResultID(module: module, key: "doctor.summary"),
        title: summary.isHealthy ? "Luma looks healthy" : "Luma needs attention",
        titleAttributed: AttributedString(summary.isHealthy ? "Luma looks healthy" : "Luma needs attention"),
        subtitle: "\(summary.moduleCount) modules · \(summary.issues.count) note(s)",
        icon: .symbol(summary.isHealthy ? "heart.text.square" : "stethoscope"),
        primaryAction: Action(
          id: ActionID(module: module, key: "doctor.summary"),
          title: "Summary",
          kind: .noop
        ),
        rankingHints: RankingHints(basePriority: basePriority),
        rowKind: .informational
      )
    ]
    rows.append(contentsOf: summary.issues.map { issue in
      ResultItem(
        id: ResultID(module: module, key: "doctor.\(issue.message.hashValue)"),
        title: issue.message,
        titleAttributed: AttributedString(issue.message),
        subtitle: issue.severity.rawValue.capitalized,
        icon: .symbol(issue.severity == .error ? "xmark.octagon" : (issue.severity == .warning ? "exclamationmark.triangle" : "info.circle")),
        primaryAction: Action(
          id: ActionID(module: module, key: "doctor.issue"),
          title: "Info",
          kind: .noop
        ),
        rankingHints: RankingHints(basePriority: basePriority),
        rowKind: .informational
      )
    })
    return rows
  }
}
