import Foundation
import LumaCore

public actor SecretsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .secrets,
        displayName: "Secrets",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: false,
        priority: 1,
        queryTimeout: .milliseconds(30)
    )

    private let vault: SecretsVault
    private var autoClearSeconds = 10
    private var launcherUI: any LauncherUIClient = NoopLauncherUIClient()

    public init(vault: SecretsVault = SecretsVault()) {
        self.vault = vault
    }
    public func warmup(_ context: ModuleContext) async {
        launcherUI = context.launcherUI
        autoClearSeconds = await context.runtime.config.secretsAutoClearSeconds()
        let relockSeconds = await context.runtime.config.secretsRelockTimeoutSeconds()
        await vault.configure(relockTimeoutSeconds: relockSeconds) { locked in
            await context.launcherUI.notifySecretsLockStateChanged(locked)
        }
        if !(await context.runtime.config.secretsRequireUnlockOnLaunch()) {
            await vault.unlock()
        } else {
            await context.launcherUI.notifySecretsLockStateChanged(true)
        }
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        guard let payload = query.command?.payload ?? Self.extractPayload(raw: query.raw) else {
            return ModuleResult(items: [])
        }

        if payload.lowercased() == "unlock" {
            return ModuleResult(items: [unlockResult()])
        }

        if ModuleHelp.isHelpQuery(payload) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

        let searchText = payload.trimmingCharacters(in: .whitespacesAndNewlines)

        guard await vault.unlocked() else {
            return ModuleResult(items: [lockedResult()])
        }

        do {
            let records = try await vault.searchLabels(searchText)
            var items: [ResultItem] = []
            for record in records {
                items.append(secretResult(record))
            }
            if items.isEmpty, searchText.isEmpty {
                return ModuleResult(items: [unlockedEmptyResult()])
            }
            return ModuleResult(items: items)
        } catch SecretsVaultError.locked {
            return ModuleResult(items: [lockedResult()])
        } catch {
            return ModuleResult(items: [])
        }
    }

    public func perform(_ action: Action, context: ActionContext) async throws {
        guard case .custom(let payload, let handler) = action.kind, handler == Self.manifest.identifier else {
            throw ModuleError.unsupportedAction(action.id)
        }
        let decoded = try ModuleActionCoding.decode(SecretsAction.self, from: payload)
        switch decoded {
        case .unlockVault:
            await vault.unlock()
        case .copySecret(let id):
            let value = try await vault.revealValue(id: id)
            try await context.platform.pasteboard.writeSecure(value, clearAfterSeconds: autoClearSeconds)
        }
    }

    public func isUnlocked() async -> Bool {
        await vault.unlocked()
    }

    public func unlock() async {
        await vault.unlock()
    }

    public func lock() async {
        await vault.lock()
    }

    public func allRecords() async throws -> [SecretRecord] {
        try await vault.allRecords()
    }

    public func save(label: String, account: String, value: String) async throws -> UUID {
        try await vault.save(label: label, account: account, value: value)
    }

    public func update(id: UUID, label: String, account: String, value: String?) async throws {
        try await vault.update(id: id, label: label, account: account, value: value)
    }

    public func delete(id: UUID) async throws {
        try await vault.delete(id: id)
    }

    public func revealValue(id: UUID) async throws -> String {
        try await vault.revealValue(id: id)
    }

    public func applySettings(autoClearSeconds: Int, relockTimeoutSeconds: Int) async {
        self.autoClearSeconds = max(1, autoClearSeconds)
        await vault.configure(relockTimeoutSeconds: relockTimeoutSeconds) { [launcherUI] locked in
            await launcherUI.notifySecretsLockStateChanged(locked)
        }
    }

    func autoClearDuration() -> Int { autoClearSeconds }

    private func lockedResult(title: String = "Unlock Secrets Vault", subtitle: String = "Run secret unlock first") -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: "unlock-vault")
        let payload = (try? ModuleActionCoding.encode(SecretsAction.unlockVault)) ?? Data()
        return ResultItem(
            id: id,
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: subtitle,
            icon: .symbol("lock.shield"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "unlock-vault"),
                title: "Unlock Vault",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func unlockedEmptyResult() -> ResultItem {
        ResultItem(
            id: ResultID(module: Self.manifest.identifier, key: "open-vault"),
            title: "Secrets Vault",
            titleAttributed: AttributedString("Secrets Vault"),
            subtitle: "Open vault to manage secrets",
            icon: .symbol("lock.shield"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "open-detail"),
                title: "Open Vault",
                kind: .openModuleDetail(Self.manifest.identifier, payload: nil)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority),
        )
    }

    private func unlockResult() -> ResultItem {
        lockedResult(title: "Unlock Secrets Vault", subtitle: "Unlock to search saved secrets")
    }

    private func secretResult(_ record: SecretRecord) -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: record.id.uuidString)
        let subtitle = record.account.isEmpty ? "Secret" : record.account
        let payload = (try? ModuleActionCoding.encode(SecretsAction.copySecret(id: record.id))) ?? Data()
        return ResultItem(
            id: id,
            title: record.label,
            titleAttributed: AttributedString(record.label),
            subtitle: subtitle,
            icon: .symbol("lock.shield"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "copy.\(record.id.uuidString)"),
                title: "Copy Secret",
                kind: .custom(payload: payload, handler: Self.manifest.identifier)
            ),
            secondaryActions: [
                Action(
                    id: ActionID(module: Self.manifest.identifier, key: "copy-account.\(record.id.uuidString)"),
                    title: "Copy Account",
                    kind: .copyToPasteboard(record.account)
                )
            ],
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    public static func extractPayload(raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let lower = trimmed.lowercased()
        if lower == "sec" || lower == "secret" || lower == "secrets" {
            return ""
        }
        if lower.hasPrefix("sec ") {
            return String(trimmed.dropFirst(4)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("secret ") {
            return String(trimmed.dropFirst(7)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        if lower.hasPrefix("secrets ") {
            return String(trimmed.dropFirst(8)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }
}
