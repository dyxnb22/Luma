import Foundation
import LumaCore

public actor SecretsModule: LumaModule {
    public static let manifest = ModuleManifest(
        identifier: .secrets,
        displayName: "Secrets",
        capabilities: [.queryable, .providesActions],
        defaultEnabled: true,
        priority: 1,
        queryTimeout: .milliseconds(30)
    )

    private let vault: SecretsVault
    private var autoClearSeconds = 10

    public init(vault: SecretsVault = SecretsVault()) {
        self.vault = vault
    }

    public func warmup(_ context: ModuleContext) async {
        autoClearSeconds = await context.config.secretsAutoClearSeconds()
        let relockSeconds = await context.config.secretsRelockTimeoutSeconds()
        await vault.configure(relockTimeoutSeconds: relockSeconds) { locked in
            await MainActor.run {
                LauncherBridge.onSecretsLockStateChanged?(locked)
            }
        }
        if !(await context.config.secretsRequireUnlockOnLaunch()) {
            await vault.unlock()
        } else {
            await MainActor.run {
                LauncherBridge.onSecretsLockStateChanged?(true)
            }
        }
    }

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        let normalized = query.normalized
        guard normalized.hasPrefix("secret ") || normalized == "secrets" || normalized.hasPrefix("secrets ") else {
            return ModuleResult(items: [])
        }

        if normalized == "secret unlock" || normalized == "secrets unlock" {
            return ModuleResult(items: [unlockResult()])
        }

        let searchText = normalized
            .replacingOccurrences(of: "secrets", with: "")
            .replacingOccurrences(of: "secret", with: "")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        if ModuleHelp.isHelpQuery(searchText) {
            return ModuleResult(items: ModuleHelp.results(for: Self.manifest.identifier))
        }

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
                return ModuleResult(items: [lockedResult(title: "Secrets Vault Unlocked", subtitle: "Type secret <label> to search")])
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
            await context.pasteboard.writeSecure(value, clearAfterSeconds: autoClearSeconds)
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
        await vault.configure(relockTimeoutSeconds: relockTimeoutSeconds) { locked in
            await MainActor.run {
                LauncherBridge.onSecretsLockStateChanged?(locked)
            }
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
}
