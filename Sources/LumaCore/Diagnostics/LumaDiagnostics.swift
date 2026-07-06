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
  public let notesRootReadable: Bool
  public let enabledModuleCount: Int
  public let totalModuleCount: Int
  public let menuItemsCachedCount: Int
  public let hotkeyRegistered: Bool
  public let commandsConfigValid: Bool
  public let corruptConfigFiles: [String]
  public let secretsMetadataCorrupt: Bool
  public let secretsLocked: Bool
  public let clipboardEntryCount: Int?
  public let warmupTimeoutCount: Int
  public let latencyP95Milliseconds: Double?
  public let enabledPinnedConsistent: Bool

  public init(
    accessibilityTrusted: Bool,
    remindersAuthorization: RemindersAuthorization? = nil,
    notesRootConfigured: Bool,
    notesRootReadable: Bool = true,
    enabledModuleCount: Int,
    totalModuleCount: Int,
    menuItemsCachedCount: Int = 0,
    hotkeyRegistered: Bool = true,
    commandsConfigValid: Bool = true,
    corruptConfigFiles: [String] = [],
    secretsMetadataCorrupt: Bool = false,
    secretsLocked: Bool = true,
    clipboardEntryCount: Int? = nil,
    warmupTimeoutCount: Int = 0,
    latencyP95Milliseconds: Double? = nil,
    enabledPinnedConsistent: Bool = true
  ) {
    self.accessibilityTrusted = accessibilityTrusted
    self.remindersAuthorization = remindersAuthorization
    self.notesRootConfigured = notesRootConfigured
    self.notesRootReadable = notesRootReadable
    self.enabledModuleCount = enabledModuleCount
    self.totalModuleCount = totalModuleCount
    self.menuItemsCachedCount = menuItemsCachedCount
    self.hotkeyRegistered = hotkeyRegistered
    self.commandsConfigValid = commandsConfigValid
    self.corruptConfigFiles = corruptConfigFiles
    self.secretsMetadataCorrupt = secretsMetadataCorrupt
    self.secretsLocked = secretsLocked
    self.clipboardEntryCount = clipboardEntryCount
    self.warmupTimeoutCount = warmupTimeoutCount
    self.latencyP95Milliseconds = latencyP95Milliseconds
    self.enabledPinnedConsistent = enabledPinnedConsistent
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

    if !context.hotkeyRegistered {
      issues.append(LumaDiagnosticIssue(
        severity: .error,
        message: "Global hotkey is not registered — open Settings → General to review the shortcut."
      ))
    } else {
      issues.append(LumaDiagnosticIssue(
        severity: .info,
        message: "Global hotkey registered."
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

    if !context.notesRootConfigured {
      issues.append(LumaDiagnosticIssue(
        severity: .warning,
        moduleID: ModuleIdentifier(rawValue: "luma.notes"),
        message: "Notes root is not configured — choose a folder in Settings → Notes."
      ))
    } else if !context.notesRootReadable {
      issues.append(LumaDiagnosticIssue(
        severity: .error,
        moduleID: ModuleIdentifier(rawValue: "luma.notes"),
        message: "Notes root path is not readable — re-select the folder in Settings → Notes."
      ))
    }

    if !context.commandsConfigValid {
      issues.append(LumaDiagnosticIssue(
        severity: .warning,
        moduleID: ModuleIdentifier(rawValue: "luma.commands"),
        message: "commands.json is invalid — check Application Support/Luma/commands.json."
      ))
    }

    for file in context.corruptConfigFiles {
      issues.append(LumaDiagnosticIssue(
        severity: .warning,
        message: "Config corrupt: \(file) — quarantined; defaults restored."
      ))
    }

    if context.secretsMetadataCorrupt {
      issues.append(LumaDiagnosticIssue(
        severity: .warning,
        moduleID: ModuleIdentifier(rawValue: "luma.secrets"),
        message: "Secrets metadata was corrupt — Keychain entries may be orphaned until re-saved."
      ))
    }

    if context.secretsLocked {
      issues.append(LumaDiagnosticIssue(
        severity: .info,
        moduleID: ModuleIdentifier(rawValue: "luma.secrets"),
        message: "Secrets vault is locked."
      ))
    }

    if context.menuItemsCachedCount == 0 {
      issues.append(LumaDiagnosticIssue(
        severity: .info,
        moduleID: ModuleIdentifier(rawValue: "luma.menu-items"),
        message: "Menu bar search cache is empty for the frontmost app — results appear after the first scan."
      ))
    }

    if let count = context.clipboardEntryCount {
      issues.append(LumaDiagnosticIssue(
        severity: .info,
        moduleID: ModuleIdentifier(rawValue: "luma.clipboard"),
        message: "Clipboard history entries: \(count)."
      ))
    }

    if context.warmupTimeoutCount > 0 {
      issues.append(LumaDiagnosticIssue(
        severity: .warning,
        message: "Module warmup timeouts this session: \(context.warmupTimeoutCount)."
      ))
    }

    if !context.enabledPinnedConsistent {
      issues.append(LumaDiagnosticIssue(
        severity: .warning,
        message: "Pinned modules include disabled modules — review Settings → Modules."
      ))
    }

    if let p95 = context.latencyP95Milliseconds, p95 > 120 {
      issues.append(LumaDiagnosticIssue(
        severity: .warning,
        message: "Query latency p95 is \(Int(p95)) ms — above 120 ms budget."
      ))
    } else if context.latencyP95Milliseconds != nil {
      issues.append(LumaDiagnosticIssue(
        severity: .info,
        message: "Query latency p95 within budget."
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
      let key = "doctor.\(abs(issue.message.hashValue))"
      return ResultItem(
        id: ResultID(module: module, key: key),
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
