import Foundation

public struct PlatformClients: Sendable {
    public let pasteboard: any PasteboardClient
    public let accessibility: any AccessibilityClient
    public let fileSystem: any FileSystemClient
    public let translation: any TranslationClient
    public let workspace: any WorkspaceClient
    public let clipboardSnapshot: any ClipboardSnapshotClient
    public let processMemory: any ProcessMemoryClient
    public let reminders: any RemindersClient
    public let scriptRunner: any ScriptRunnerClient
    public let notifications: any NotificationClient
    public let currentProject: any CurrentProjectClient
    public let selectionSnapshot: any SelectionSnapshotClient
    public let menuBarTree: any MenuBarTreeClient
    public let runningApplications: any RunningApplicationsClient

    public init(
        pasteboard: any PasteboardClient,
        accessibility: any AccessibilityClient,
        fileSystem: any FileSystemClient,
        translation: any TranslationClient,
        workspace: any WorkspaceClient = NoopWorkspaceClient(),
        clipboardSnapshot: any ClipboardSnapshotClient = NoopClipboardSnapshotClient(),
        processMemory: any ProcessMemoryClient = NoopProcessMemoryClient(),
        reminders: any RemindersClient = NoopRemindersClient(),
        scriptRunner: any ScriptRunnerClient = NoopScriptRunnerClient(),
        notifications: any NotificationClient = NoopNotificationClient(),
        currentProject: any CurrentProjectClient = NoopCurrentProjectClient(),
        selectionSnapshot: any SelectionSnapshotClient = NoopSelectionSnapshotClient(),
        menuBarTree: any MenuBarTreeClient = NoopMenuBarTreeClient(),
        runningApplications: any RunningApplicationsClient = NoopRunningApplicationsClient()
    ) {
        self.pasteboard = pasteboard
        self.accessibility = accessibility
        self.fileSystem = fileSystem
        self.translation = translation
        self.workspace = workspace
        self.clipboardSnapshot = clipboardSnapshot
        self.processMemory = processMemory
        self.reminders = reminders
        self.scriptRunner = scriptRunner
        self.notifications = notifications
        self.currentProject = currentProject
        self.selectionSnapshot = selectionSnapshot
        self.menuBarTree = menuBarTree
        self.runningApplications = runningApplications
    }
}

public struct QueryPlatformClients: Sendable {
    public let pasteboard: any PasteboardClient
    public let accessibility: any AccessibilityClient
    public let processMemory: any ProcessMemoryClient
    public let currentProject: any CurrentProjectClient
    public let selectionSnapshot: any SelectionSnapshotClient
    public let runningApplications: any RunningApplicationsClient

    public init(
        pasteboard: any PasteboardClient = NoopPasteboardClient(),
        accessibility: any AccessibilityClient = NoopAccessibilityClient(),
        processMemory: any ProcessMemoryClient = NoopProcessMemoryClient(),
        currentProject: any CurrentProjectClient = NoopCurrentProjectClient(),
        selectionSnapshot: any SelectionSnapshotClient = NoopSelectionSnapshotClient(),
        runningApplications: any RunningApplicationsClient = NoopRunningApplicationsClient()
    ) {
        self.pasteboard = pasteboard
        self.accessibility = accessibility
        self.processMemory = processMemory
        self.currentProject = currentProject
        self.selectionSnapshot = selectionSnapshot
        self.runningApplications = runningApplications
    }
}

public struct ActionPlatformClients: Sendable {
    public let pasteboard: any PasteboardClient
    public let accessibility: any AccessibilityClient
    public let translation: any TranslationClient
    public let workspace: any WorkspaceClient
    public let scriptRunner: any ScriptRunnerClient
    public let currentProject: any CurrentProjectClient
    public let selectionSnapshot: any SelectionSnapshotClient

    public init(
        pasteboard: any PasteboardClient,
        accessibility: any AccessibilityClient,
        translation: any TranslationClient = NoopTranslationClient(),
        workspace: any WorkspaceClient = NoopWorkspaceClient(),
        scriptRunner: any ScriptRunnerClient = NoopScriptRunnerClient(),
        currentProject: any CurrentProjectClient = NoopCurrentProjectClient(),
        selectionSnapshot: any SelectionSnapshotClient = NoopSelectionSnapshotClient()
    ) {
        self.pasteboard = pasteboard
        self.accessibility = accessibility
        self.translation = translation
        self.workspace = workspace
        self.scriptRunner = scriptRunner
        self.currentProject = currentProject
        self.selectionSnapshot = selectionSnapshot
    }
}
