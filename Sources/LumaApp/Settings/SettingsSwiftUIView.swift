import AppKit
import SwiftUI
import LumaCore
import LumaInfrastructure
import LumaModules
import LumaServices

// MARK: - Navigation sections

enum SettingsSection: String, CaseIterable, Identifiable {
    case general       = "General"
    case modules       = "Modules"
    case clipboard     = "Clipboard"
    case translation   = "Translation"
    case wordbook      = "Wordbook"
    case secrets       = "Secrets"
    case accessibility = "Accessibility"
    case activity      = "Activity"
    case developer     = "Developer"

    var id: String { rawValue }

    var icon: String {
        switch self {
        case .general:       return "gearshape"
        case .modules:       return "square.grid.2x2"
        case .clipboard:     return "doc.on.clipboard"
        case .translation:   return "character.bubble"
        case .wordbook:      return "text.book.closed"
        case .secrets:       return "lock.shield"
        case .accessibility: return "hand.raised"
        case .activity:      return "chart.bar"
        case .developer:     return "hammer"
        }
    }

    var tint: Color {
        switch self {
        case .general:       return .gray
        case .modules:       return .blue
        case .clipboard:     return .orange
        case .translation:   return .cyan
        case .wordbook:      return .purple
        case .secrets:       return .yellow
        case .accessibility: return .green
        case .activity:      return .indigo
        case .developer:     return .brown
        }
    }
}

// MARK: - Root view

struct SettingsSwiftUIView: View {
    let snapshot: SettingsSnapshot
    let config: ConfigurationStore
    let usage: PersistentUsageTracker
    let onModulesChanged: @MainActor (Set<ModuleIdentifier>) -> Void
    let onPinnedChanged: @MainActor (Set<ModuleIdentifier>) -> Void
    let onClipboardSettingsChanged: @MainActor (SettingsSnapshot) -> Void
    let onSecretsSettingsChanged: @MainActor (Int, Int) -> Void
    let onLatencyHUDChanged: @MainActor (Bool) -> Void
    private let initialSection: SettingsSection

    @State private var selected: SettingsSection?

    init(
        snapshot: SettingsSnapshot,
        config: ConfigurationStore,
        usage: PersistentUsageTracker,
        onModulesChanged: @escaping @MainActor (Set<ModuleIdentifier>) -> Void,
        onPinnedChanged: @escaping @MainActor (Set<ModuleIdentifier>) -> Void,
        onClipboardSettingsChanged: @escaping @MainActor (SettingsSnapshot) -> Void,
        onSecretsSettingsChanged: @escaping @MainActor (Int, Int) -> Void,
        onLatencyHUDChanged: @escaping @MainActor (Bool) -> Void,
        initialSection: SettingsSection = .general
    ) {
        self.snapshot = snapshot
        self.config = config
        self.usage = usage
        self.onModulesChanged = onModulesChanged
        self.onPinnedChanged = onPinnedChanged
        self.onClipboardSettingsChanged = onClipboardSettingsChanged
        self.onSecretsSettingsChanged = onSecretsSettingsChanged
        self.onLatencyHUDChanged = onLatencyHUDChanged
        self.initialSection = initialSection
        _selected = State(initialValue: initialSection)
    }

    var body: some View {
        NavigationSplitView(columnVisibility: .constant(.all)) {
            List(SettingsSection.allCases, selection: $selected) { section in
                Label {
                    Text(section.rawValue)
                        .font(.system(size: 13, weight: .medium))
                } icon: {
                    Image(systemName: section.icon)
                        .foregroundStyle(section.tint)
                        .frame(width: 20, height: 20)
                        .background(section.tint.opacity(0.12))
                        .clipShape(RoundedRectangle(cornerRadius: 5, style: .continuous))
                }
                .tag(section)
            }
            .listStyle(.sidebar)
            .navigationTitle("Settings")
            .navigationSplitViewColumnWidth(min: 160, ideal: 180)
        } detail: {
            detailContent
        }
        .frame(minWidth: 680, minHeight: 480)
        .background(SettingsKeyboardSupport())
    }

    @ViewBuilder
    private var detailContent: some View {
        switch selected ?? .general {
        case .general:
            GeneralSettingsView()
        case .modules:
            ModulesSettingsView(
                snapshot: snapshot,
                config: config,
                onModulesChanged: onModulesChanged,
                onPinnedChanged: onPinnedChanged
            )
        case .clipboard:
            ClipboardSettingsView(
                snapshot: snapshot,
                config: config,
                onClipboardSettingsChanged: onClipboardSettingsChanged
            )
        case .translation:
            TranslationSettingsView(snapshot: snapshot, config: config)
        case .wordbook:
            WordbookSettingsView()
        case .secrets:
            SecretsSettingsView(
                snapshot: snapshot,
                config: config,
                onSecretsSettingsChanged: onSecretsSettingsChanged
            )
        case .accessibility:
            AccessibilitySettingsView()
        case .activity:
            ActivitySettingsPage(usage: usage)
        case .developer:
            DeveloperSettingsView(
                snapshot: snapshot,
                config: config,
                onLatencyHUDChanged: onLatencyHUDChanged
            )
        }
    }
}

// MARK: - Section pages

struct GeneralSettingsView: View {
    @State private var language = LumaLocale.choice

    var body: some View {
        SettingsPage("General") {
            SettingsCard(L10n.tr("settings.language.title")) {
                Picker(L10n.tr("settings.language.title"), selection: $language) {
                    ForEach(LumaLocale.Choice.allCases, id: \.self) { choice in
                        Text(choice.displayName).tag(choice)
                    }
                }
                .pickerStyle(.menu)
                .onChange(of: language) { _, newValue in
                    LumaLocale.choice = newValue
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 9)
            }

            SettingsCard("Hotkey") {
                SettingsRow("Launcher", icon: "keyboard") {
                    Text("⌘ Space")
                        .font(.system(size: 13, weight: .semibold, design: .monospaced))
                        .foregroundStyle(.secondary)
                }
                Divider()
                HStack {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Disable Spotlight's ⌘Space shortcut if Luma can't register the launcher hotkey.")
                            .font(.caption).foregroundStyle(.tertiary)
                        Text("Menu bar shows a warning when registration fails.")
                            .font(.caption2).foregroundStyle(.quaternary)
                    }
                    Spacer()
                    Button("Open Keyboard Settings…") {
                        NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.keyboard")!)
                    }
                    .buttonStyle(.link).font(.caption)
                }
                .padding(.vertical, 4).padding(.horizontal, 12)
            }

            SettingsCard("About") {
                SettingsRow("Version", icon: "info.circle") {
                    let v = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "0.1"
                    Text("Luma \(v)").foregroundStyle(.secondary).font(.system(size: 13))
                }
                Divider()
                dataLocationRow()
            }
        }
    }

    @ViewBuilder
    private func dataLocationRow() -> some View {
        let path = (try? FileManager.default
            .url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: false)
            .appendingPathComponent("Luma").path) ?? "~/Library/Application Support/Luma"
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 12) {
                Image(systemName: "folder")
                    .foregroundStyle(.secondary)
                    .frame(width: 18)
                Text("Data location")
                    .font(.system(size: 13))
                Spacer()
            }
            Text(path)
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.leading, 30)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 9)
    }
}

struct ModulesSettingsView: View {
    let snapshot: SettingsSnapshot
    let config: ConfigurationStore
    let onModulesChanged: @MainActor (Set<ModuleIdentifier>) -> Void
    let onPinnedChanged: @MainActor (Set<ModuleIdentifier>) -> Void

    @State private var enabledModules: Set<ModuleIdentifier>
    @State private var pinnedModules: Set<ModuleIdentifier>
    @State private var debounceTask: Task<Void, Never>?

    init(snapshot: SettingsSnapshot,
         config: ConfigurationStore,
         onModulesChanged: @escaping @MainActor (Set<ModuleIdentifier>) -> Void,
         onPinnedChanged: @escaping @MainActor (Set<ModuleIdentifier>) -> Void) {
        self.snapshot = snapshot; self.config = config
        self.onModulesChanged = onModulesChanged
        self.onPinnedChanged = onPinnedChanged
        _enabledModules = State(initialValue: snapshot.enabledModules)
        _pinnedModules = State(initialValue: snapshot.pinnedModuleIDs)
    }

    var body: some View {
        SettingsPage("Modules") {
            SettingsCard("Pin to hot path") {
                Text("Pinned modules warm at startup and stay ready for search.")
                    .font(.caption).foregroundStyle(.secondary)
                    .padding(.horizontal, 12).padding(.top, 8)
                Text("Default-off modules must be enabled before pinning takes effect.")
                    .font(.caption).foregroundStyle(.tertiary)
                    .padding(.horizontal, 12).padding(.bottom, 4)
                ForEach(Array(snapshot.modules.enumerated()), id: \.element.id) { index, module in
                    if index > 0 { Divider() }
                    HStack {
                        Toggle(isOn: Binding(
                            get: { pinnedModules.contains(module.id) },
                            set: { on in
                                if on { pinnedModules.insert(module.id) } else { pinnedModules.remove(module.id) }
                                let snap = pinnedModules
                                Task {
                                    await config.setPinnedModuleIDs(snap)
                                    await MainActor.run { onPinnedChanged(snap) }
                                }
                            }
                        )) {
                            HStack(spacing: 8) {
                                Label(module.name, systemImage: moduleIcon(module.id))
                                    .font(.system(size: 13))
                                if ModuleRegistry.bundle(for: module.id)?.manifest.defaultEnabled == false {
                                    Text("Default off")
                                        .font(.caption2.weight(.semibold))
                                        .foregroundStyle(.orange)
                                        .padding(.horizontal, 6)
                                        .padding(.vertical, 2)
                                        .background(Color.orange.opacity(0.12))
                                        .clipShape(Capsule())
                                }
                            }
                        }
                        .disabled(!enabledModules.contains(module.id))
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 12).padding(.vertical, 8)
                }
            }

            SettingsCard("Enabled modules") {
                ForEach(Array(snapshot.modules.enumerated()), id: \.element.id) { index, module in
                    if index > 0 { Divider() }
                    VStack(alignment: .leading, spacing: 4) {
                        Toggle(isOn: Binding(
                            get: { enabledModules.contains(module.id) },
                            set: { on in
                                if on { enabledModules.insert(module.id) } else {
                                    enabledModules.remove(module.id)
                                    pinnedModules.remove(module.id)
                                }
                                debounceTask?.cancel()
                                let snap = enabledModules
                                let pins = pinnedModules
                                debounceTask = Task {
                                    try? await Task.sleep(for: .milliseconds(200))
                                    guard !Task.isCancelled else { return }
                                    await config.setEnabledModules(snap)
                                    await config.setPinnedModuleIDs(pins)
                                    await MainActor.run {
                                        onModulesChanged(snap)
                                        onPinnedChanged(pins)
                                    }
                                }
                            }
                        )) {
                            HStack(spacing: 8) {
                                Label(module.name, systemImage: moduleIcon(module.id))
                                    .font(.system(size: 13))
                                if ModuleRegistry.bundle(for: module.id)?.manifest.defaultEnabled == false {
                                    Text("Default off")
                                        .font(.caption2.weight(.semibold))
                                        .foregroundStyle(.orange)
                                        .padding(.horizontal, 6)
                                        .padding(.vertical, 2)
                                        .background(Color.orange.opacity(0.12))
                                        .clipShape(Capsule())
                                }
                            }
                        }
                        if ModuleRegistry.bundle(for: module.id)?.manifest.defaultEnabled == false,
                           let note = ModuleRegistry.defaultOffNote(for: module.id) {
                            Text(note)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .padding(.leading, 28)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 12).padding(.vertical, 8)
                }
            }

            Text("Disabled modules are paused from warmup and query dispatching; their data is preserved.")
                .font(.caption).foregroundStyle(.secondary).padding(.horizontal, 2)
        }
    }

    private func moduleIcon(_ id: ModuleIdentifier) -> String {
        ModuleRegistry.presentation(for: id)?.settingsSymbol ?? "puzzlepiece"
    }
}

struct ClipboardSettingsView: View {
    let snapshot: SettingsSnapshot
    let config: ConfigurationStore
    let onClipboardSettingsChanged: @MainActor (SettingsSnapshot) -> Void

    @State private var maxEntries: String
    @State private var maxDays: String
    @State private var maxKB: String
    @State private var historyEnabled: Bool
    @State private var ignoredBundleIDs: String
    @State private var pasteBehavior: ClipboardPasteBehavior
    @State private var savedMessage: String?

    init(snapshot: SettingsSnapshot, config: ConfigurationStore,
         onClipboardSettingsChanged: @escaping @MainActor (SettingsSnapshot) -> Void) {
        self.snapshot = snapshot; self.config = config; self.onClipboardSettingsChanged = onClipboardSettingsChanged
        _maxEntries = State(initialValue: "\(snapshot.clipboardMaxEntries)")
        _maxDays = State(initialValue: "\(snapshot.clipboardMaxAgeDays)")
        _maxKB = State(initialValue: "\(snapshot.clipboardMaxEntrySizeKB)")
        _historyEnabled = State(initialValue: snapshot.clipboardHistoryEnabled)
        _ignoredBundleIDs = State(initialValue: snapshot.clipboardIgnoredBundleIDs.joined(separator: ", "))
        _pasteBehavior = State(initialValue: ClipboardPasteBehavior(rawValue: snapshot.clipboardPasteBehavior) ?? .pasteDirectly)
    }

    var body: some View {
        SettingsPage("Clipboard") {
            SettingsCard("Capture") {
                SettingsRow("Enable clipboard history", icon: "doc.on.clipboard") {
                    Toggle("", isOn: $historyEnabled).labelsHidden()
                }
                Divider()
                SettingsRow("Paste behavior", icon: "arrow.turn.down.right") {
                    Picker("", selection: $pasteBehavior) {
                        ForEach(ClipboardPasteBehavior.allCases, id: \.self) { mode in
                            Text(mode.displayName).tag(mode)
                        }
                    }
                    .labelsHidden()
                    .frame(width: 160)
                }
                Divider()
                VStack(alignment: .leading, spacing: 4) {
                    SettingsRow("Ignore apps", icon: "nosign") {
                        TextField("com.example.app", text: $ignoredBundleIDs)
                            .textFieldStyle(.roundedBorder)
                    }
                    Text("Comma-separated bundle IDs. Built-in password managers are always ignored.")
                        .font(.caption).foregroundStyle(.tertiary)
                        .padding(.horizontal, 12).padding(.bottom, 4)
                }
            }

            SettingsCard("Retention") {
                SettingsRow("Max entries", icon: "list.number") {
                    TextField("500", text: $maxEntries)
                        .textFieldStyle(.roundedBorder).frame(width: 80).multilineTextAlignment(.trailing)
                }
                Divider()
                SettingsRow("Keep for (days)", icon: "calendar") {
                    TextField("30", text: $maxDays)
                        .textFieldStyle(.roundedBorder).frame(width: 80).multilineTextAlignment(.trailing)
                }
                Divider()
                SettingsRow("Max entry size (KB)", icon: "doc.text") {
                    TextField("1024", text: $maxKB)
                        .textFieldStyle(.roundedBorder).frame(width: 80).multilineTextAlignment(.trailing)
                }
            }

            HStack {
                if let savedMessage {
                    Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
                    Text(savedMessage).foregroundStyle(.secondary).font(.subheadline)
                }
                Spacer()
                Button("Apply") { apply() }
                    .buttonStyle(.borderedProminent)
                    .disabled(!isValid)
            }
        }
    }

    private var isValid: Bool {
        (Int(maxEntries).map { $0 > 0 } ?? false) &&
        (Int(maxDays).map { $0 > 0 } ?? false) &&
        (Int(maxKB).map { $0 > 0 } ?? false)
    }

    private func apply() {
        guard let e = Int(maxEntries), let d = Int(maxDays), let k = Int(maxKB),
              e > 0, d > 0, k > 0 else { return }
        let ignored = ignoredBundleIDs
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        Task {
            await config.setClipboardMaxEntries(e)
            await config.setClipboardMaxAgeDays(d)
            await config.setClipboardMaxEntrySizeKB(k)
            await config.setClipboardHistoryEnabled(historyEnabled)
            await config.setClipboardIgnoredBundleIDs(ignored)
            await config.setClipboardPasteBehavior(pasteBehavior.rawValue)
            var updated = snapshot
            updated.clipboardMaxEntries = e
            updated.clipboardMaxAgeDays = d
            updated.clipboardMaxEntrySizeKB = k
            updated.clipboardHistoryEnabled = historyEnabled
            updated.clipboardIgnoredBundleIDs = ignored
            updated.clipboardPasteBehavior = pasteBehavior.rawValue
            await MainActor.run { onClipboardSettingsChanged(updated); savedMessage = "Saved" }
            try? await Task.sleep(for: .seconds(2))
            await MainActor.run { savedMessage = nil }
        }
    }
}

struct TranslationSettingsView: View {
    let snapshot: SettingsSnapshot
    let config: ConfigurationStore

    @State private var target: String
    @State private var saved = false

    private let languages: [(code: String, name: String)] = [
        ("en", "English"), ("zh-Hans", "Chinese (Simplified)"), ("zh-Hant", "Chinese (Traditional)"),
        ("ja", "Japanese"), ("ko", "Korean"), ("fr", "French"), ("de", "German"),
        ("es", "Spanish"), ("it", "Italian"), ("pt", "Portuguese"), ("ru", "Russian"), ("ar", "Arabic")
    ]

    init(snapshot: SettingsSnapshot, config: ConfigurationStore) {
        self.snapshot = snapshot; self.config = config
        _target = State(initialValue: snapshot.translationTargetLanguage)
    }

    var body: some View {
        SettingsPage("Translation") {
            SettingsCard("Default language") {
                SettingsRow("Target language", icon: "globe") {
                    Picker("", selection: $target) {
                        ForEach(languages, id: \.code) { Text($0.name).tag($0.code) }
                    }
                    .labelsHidden()
                    .frame(width: 200)
                    .onChange(of: target) { _, value in
                        Task {
                            await config.setTranslationTargetLanguage(value)
                            await MainActor.run { saved = true }
                            try? await Task.sleep(for: .seconds(2))
                            await MainActor.run { saved = false }
                        }
                    }
                }
            }

            if saved {
                HStack(spacing: 6) {
                    Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
                    Text("Saved").foregroundStyle(.secondary).font(.subheadline)
                }
            }

            Text("This sets the default target in Translate detail. You can also switch per-session using the language chips in the Translate panel.")
                .font(.caption).foregroundStyle(.secondary).padding(.horizontal, 2)
        }
    }
}

struct WordbookSettingsView: View {
    @State private var resetConfirm = false
    @State private var resetDone = false
    @State private var dbPath = ""

    var body: some View {
        SettingsPage("Wordbook") {
            SettingsCard("Learning plan") {
                SettingsRow("Daily quota", icon: "calendar.badge.clock") {
                    VStack(alignment: .trailing, spacing: 2) {
                        Text("Auto-adaptive").foregroundStyle(.secondary).font(.system(size: 13))
                        Text("Scales with due count & errors").font(.caption).foregroundStyle(.tertiary)
                    }
                }
                Divider()
                SettingsRow("Review algorithm", icon: "brain.head.profile") {
                    Text("Ebbinghaus 9-stage").foregroundStyle(.secondary).font(.system(size: 13))
                }
            }

            SettingsCard("Data") {
                VStack(alignment: .leading, spacing: 6) {
                    HStack(spacing: 12) {
                        Image(systemName: "internaldrive")
                            .foregroundStyle(.secondary)
                            .frame(width: 18)
                        Text("Database path")
                            .font(.system(size: 13))
                        Spacer()
                    }
                    Text(dbPath.isEmpty ? "Loading…" : dbPath)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .fixedSize(horizontal: false, vertical: true)
                        .padding(.leading, 30)
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 9)
                .onAppear {
                    if let store = LauncherEnvironment.current?.wordbookStore {
                        Task { let url = await store.databaseURL(); dbPath = url.path }
                    }
                }
                Divider()
                SettingsRow("Today's progress", icon: "arrow.counterclockwise") {
                    if resetDone {
                        Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
                    } else {
                        Button("Reset") { resetConfirm = true }
                            .buttonStyle(.bordered)
                            .confirmationDialog("Reset today's progress?", isPresented: $resetConfirm) {
                                Button("Reset", role: .destructive) {
                                    Task {
                                        guard let store = LauncherEnvironment.current?.wordbookStore else { return }
                                        try? await store.resetTodayProgress()
                                        await MainActor.run { resetDone = true }
                                        try? await Task.sleep(for: .seconds(2))
                                        await MainActor.run { resetDone = false }
                                    }
                                }
                                Button("Cancel", role: .cancel) {}
                            } message: {
                                Text("Clears daily new-seen count and wrong-answer count.")
                            }
                    }
                }
            }
        }
    }
}

struct SecretsSettingsView: View {
    let snapshot: SettingsSnapshot
    let config: ConfigurationStore
    let onSecretsSettingsChanged: @MainActor (Int, Int) -> Void

    @State private var autoClear: String
    @State private var relock: String
    @State private var requireUnlock: Bool
    @State private var savedMessage: String?

    init(snapshot: SettingsSnapshot, config: ConfigurationStore,
         onSecretsSettingsChanged: @escaping @MainActor (Int, Int) -> Void) {
        self.snapshot = snapshot; self.config = config; self.onSecretsSettingsChanged = onSecretsSettingsChanged
        _autoClear = State(initialValue: "\(snapshot.secretsAutoClearSeconds)")
        _relock = State(initialValue: "\(snapshot.secretsRelockTimeoutSeconds)")
        _requireUnlock = State(initialValue: snapshot.secretsRequireUnlockOnLaunch)
    }

    var body: some View {
        SettingsPage("Secrets") {
            SettingsCard("Vault") {
                Toggle(isOn: $requireUnlock) {
                    Label("Require unlock on launch", systemImage: "lock.fill")
                        .font(.system(size: 13))
                }
                .onChange(of: requireUnlock) { _, v in Task { await config.setSecretsRequireUnlockOnLaunch(v) } }
                .padding(.horizontal, 12).padding(.vertical, 9)
            }

            SettingsCard("Timeouts") {
                SettingsRow("Auto-clear pasteboard after", icon: "clock") {
                    HStack(spacing: 4) {
                        TextField("30", text: $autoClear)
                            .textFieldStyle(.roundedBorder).frame(width: 60).multilineTextAlignment(.trailing)
                        Text("s").foregroundStyle(.secondary)
                    }
                }
                Divider()
                SettingsRow("Re-lock vault after", icon: "timer") {
                    HStack(spacing: 4) {
                        TextField("300", text: $relock)
                            .textFieldStyle(.roundedBorder).frame(width: 60).multilineTextAlignment(.trailing)
                        Text("s").foregroundStyle(.secondary)
                    }
                }
            }

            HStack {
                if let savedMessage {
                    Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
                    Text(savedMessage).foregroundStyle(.secondary).font(.subheadline)
                }
                Spacer()
                Button("Apply") { applySecrets() }
                    .buttonStyle(.borderedProminent)
                    .disabled(!isValid)
            }
        }
    }

    private var isValid: Bool {
        (Int(autoClear).map { $0 > 0 } ?? false) && (Int(relock).map { $0 >= 30 } ?? false)
    }

    private func applySecrets() {
        guard let a = Int(autoClear), let r = Int(relock), a > 0, r >= 30 else { return }
        Task {
            await config.setSecretsAutoClearSeconds(a)
            await config.setSecretsRelockTimeoutSeconds(r)
            await MainActor.run { onSecretsSettingsChanged(a, r); savedMessage = "Saved" }
            try? await Task.sleep(for: .seconds(2))
            await MainActor.run { savedMessage = nil }
        }
    }
}

struct AccessibilitySettingsView: View {
    @State private var trusted = AXService.isProcessTrusted()

    var body: some View {
        SettingsPage("Accessibility") {
            SettingsCard("Permission") {
                SettingsRow("Status", icon: "hand.raised") {
                    HStack(spacing: 6) {
                        Circle().fill(trusted ? Color.green : Color.orange).frame(width: 8, height: 8)
                        Text(trusted ? "Granted" : "Not granted")
                            .foregroundStyle(trusted ? .primary : .secondary)
                    }
                }
                Divider()
                if trusted {
                    HStack {
                        Text("Accessibility is enabled for Luma.")
                            .font(.caption).foregroundStyle(.secondary)
                        Spacer()
                        Button("Open System Settings…") {
                            if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
                                NSWorkspace.shared.open(url)
                            }
                        }
                        .buttonStyle(.bordered)
                    }
                    .padding(.horizontal, 12).padding(.vertical, 6)
                } else {
                    HStack {
                        Text("Required for window focus and snippet auto-paste features.")
                            .font(.caption).foregroundStyle(.secondary)
                        Spacer()
                        Button("Grant Access…") {
                            AXService.requestPermission()
                            if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
                                NSWorkspace.shared.open(url)
                            }
                            DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
                                trusted = AXService.isProcessTrusted()
                            }
                        }
                        .buttonStyle(.borderedProminent)
                    }
                    .padding(.horizontal, 12).padding(.vertical, 6)
                }
            }
        }
        .onAppear { refreshTrustedStatus() }
        .onReceive(NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)) { _ in
            refreshTrustedStatus()
        }
    }

    private func refreshTrustedStatus() {
        trusted = AXService.isProcessTrusted()
    }
}

struct ActivitySettingsPage: View {
    let usage: PersistentUsageTracker

    var body: some View {
        SettingsPage("Activity") {
            ActivitySettingsRepresentable(usage: usage)
                .frame(minHeight: 300)
        }
    }
}

struct DeveloperSettingsView: View {
    let snapshot: SettingsSnapshot
    let config: ConfigurationStore
    let onLatencyHUDChanged: @MainActor (Bool) -> Void

    @State private var latencyHUD: Bool

    init(snapshot: SettingsSnapshot, config: ConfigurationStore,
         onLatencyHUDChanged: @escaping @MainActor (Bool) -> Void) {
        self.snapshot = snapshot; self.config = config; self.onLatencyHUDChanged = onLatencyHUDChanged
        _latencyHUD = State(initialValue: snapshot.latencyHUDEnabled)
    }

    var body: some View {
        SettingsPage("Developer") {
            SettingsCard("Performance") {
                Toggle(isOn: $latencyHUD) {
                    Label("Show latency HUD overlay", systemImage: "speedometer")
                        .font(.system(size: 13))
                }
                .onChange(of: latencyHUD) { _, value in
                    Task {
                        await config.setLatencyHUDEnabled(value)
                        await MainActor.run { onLatencyHUDChanged(value) }
                    }
                }
                .padding(.horizontal, 12).padding(.vertical, 9)
            }

            Text("The latency HUD displays keystroke-to-first-paint timing in the launcher panel. For internal profiling only.")
                .font(.caption).foregroundStyle(.secondary).padding(.horizontal, 2)
        }
    }
}

// MARK: - Reusable layout primitives

/// Scrollable page with a large title and VStack content.
struct SettingsPage<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    init(_ title: String, @ViewBuilder content: () -> Content) {
        self.title = title
        self.content = content()
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                Text(title)
                    .font(.largeTitle.bold())
                    .padding(.bottom, 4)
                content
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .topLeading)
        }
        .background(Color(NSColor.windowBackgroundColor))
        .focusable()
    }
}

/// Rounded card group with a small uppercase header label.
struct SettingsCard<Content: View>: View {
    let header: String
    @ViewBuilder let content: Content

    init(_ header: String, @ViewBuilder content: () -> Content) {
        self.header = header
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(header.uppercased())
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(.secondary)

            VStack(spacing: 0) {
                content
            }
            .background(Color(NSColor.controlBackgroundColor))
            .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .strokeBorder(Color(NSColor.separatorColor).opacity(0.6), lineWidth: 0.5)
            )
        }
    }
}

/// Single row: SF symbol + label on left, custom control on right.
struct SettingsRow<Control: View>: View {
    let label: String
    let icon: String
    @ViewBuilder let control: Control

    init(_ label: String, icon: String, @ViewBuilder control: () -> Control) {
        self.label = label
        self.icon = icon
        self.control = control()
    }

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .foregroundStyle(.secondary)
                .frame(width: 18)
            Text(label).font(.system(size: 13))
            Spacer()
            control
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 9)
    }
}

// MARK: - NSViewRepresentable bridge

private struct ActivitySettingsRepresentable: NSViewRepresentable {
    let usage: PersistentUsageTracker
    func makeNSView(context: Context) -> ActivitySettingsView { ActivitySettingsView(usage: usage) }
    func updateNSView(_ nsView: ActivitySettingsView, context: Context) { nsView.refresh() }
}

// MARK: - Keyboard support

private struct SettingsKeyboardSupport: NSViewRepresentable {
    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeNSView(context: Context) -> SettingsKeyHandlerView {
        let view = SettingsKeyHandlerView()
        view.onWindowChanged = { [weak view] in
            guard let view else { return }
            context.coordinator.syncMonitor(for: view.window)
        }
        return view
    }

    func updateNSView(_ nsView: SettingsKeyHandlerView, context: Context) {
        nsView.onWindowChanged = { [weak nsView] in
            guard let nsView else { return }
            context.coordinator.syncMonitor(for: nsView.window)
        }
        context.coordinator.syncMonitor(for: nsView.window)
    }

    static func dismantleNSView(_ nsView: SettingsKeyHandlerView, coordinator: Coordinator) {
        coordinator.tearDown()
    }

    @MainActor
    final class Coordinator {
        private var monitor: Any?

        func syncMonitor(for window: NSWindow?) {
            tearDown()
            guard let window else { return }
            monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
                guard event.window === window else { return event }
                return SettingsKeyboardActions.handle(event, in: window)
            }
        }

        func tearDown() {
            if let monitor {
                NSEvent.removeMonitor(monitor)
                self.monitor = nil
            }
        }
    }
}

@MainActor
private enum SettingsKeyboardActions {
    static func handle(_ event: NSEvent, in window: NSWindow) -> NSEvent? {
        if LumaStandardEditShortcuts.handleKeyDown(event, in: window) {
            return nil
        }
        if event.modifierFlags.contains(.command),
           event.charactersIgnoringModifiers?.lowercased() == "w" {
            window.close()
            return nil
        }
        if event.keyCode == 121, shouldHandlePageScroll(for: window) {
            scrollPage(.down, in: window)
            return nil
        }
        if event.keyCode == 116, shouldHandlePageScroll(for: window) {
            scrollPage(.up, in: window)
            return nil
        }
        return event
    }

    private enum PageDirection { case up, down }

    private static func shouldHandlePageScroll(for window: NSWindow) -> Bool {
        guard let responder = window.firstResponder else { return true }
        if responder is NSTextView { return false }
        if let field = responder as? NSTextField, field.isEditable {
            let wraps = field.cell?.wraps == true
            if wraps || field.maximumNumberOfLines != 1 {
                return false
            }
        }
        return true
    }

    private static func scrollPage(_ direction: PageDirection, in window: NSWindow) {
        guard let scrollView = targetScrollView(in: window) else { return }
        switch direction {
        case .down:
            scrollView.pageDown(nil)
        case .up:
            scrollView.pageUp(nil)
        }
    }

    private static func targetScrollView(in window: NSWindow) -> NSScrollView? {
        if let scrollView = (window.firstResponder as? NSView)?.enclosingScrollView {
            return scrollView
        }
        guard let root = window.contentView else { return nil }
        var candidates: [NSScrollView] = []
        collectScrollViews(in: root, into: &candidates)
        let scrollable = candidates.filter { scrollView in
            guard let document = scrollView.documentView else { return false }
            return document.frame.height > scrollView.contentView.bounds.height + 1
        }
        return scrollable.max(by: { $0.contentView.bounds.height < $1.contentView.bounds.height })
            ?? candidates.max(by: { $0.contentView.bounds.height < $1.contentView.bounds.height })
    }

    private static func collectScrollViews(in view: NSView, into result: inout [NSScrollView]) {
        if let scrollView = view as? NSScrollView {
            result.append(scrollView)
        }
        for subview in view.subviews {
            collectScrollViews(in: subview, into: &result)
        }
    }
}

@MainActor
private final class SettingsKeyHandlerView: NSView {
    var onWindowChanged: (() -> Void)?

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        onWindowChanged?()
    }

    override func viewWillMove(toWindow newWindow: NSWindow?) {
        if newWindow == nil {
            onWindowChanged?()
        }
        super.viewWillMove(toWindow: newWindow)
    }
}
