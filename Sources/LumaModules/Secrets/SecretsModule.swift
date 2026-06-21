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

    private let vault = SecretsVault()

    public init() {}

    public func handle(_ query: Query, context: QueryContext) async -> ModuleResult {
        let normalized = query.normalized
        guard normalized.hasPrefix("secret ") || normalized == "secrets" || normalized.hasPrefix("secrets ") else {
            return ModuleResult(items: [])
        }

        if normalized == "secret unlock" || normalized == "secrets unlock" {
            return ModuleResult(items: [unlockResult()])
        }

        guard await vault.unlocked() else {
            return ModuleResult(items: [lockedResult()])
        }

        let searchText = normalized
            .replacingOccurrences(of: "secrets", with: "")
            .replacingOccurrences(of: "secret", with: "")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        do {
            let records = try await vault.searchLabels(searchText)
            var items: [ResultItem] = []
            for record in records {
                items.append(try await secretResult(record))
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
        switch action.id.key {
        case "unlock-vault":
            await vault.unlock()
        default:
            throw ModuleError.unsupportedAction(action.id)
        }
    }

    private func lockedResult(title: String = "Unlock Secrets Vault", subtitle: String = "Run secret unlock first") -> ResultItem {
        let id = ResultID(module: Self.manifest.identifier, key: "unlock-vault")
        return ResultItem(
            id: id,
            title: title,
            titleAttributed: AttributedString(title),
            subtitle: subtitle,
            icon: .symbol("lock.shield"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "unlock-vault"),
                title: "Unlock Vault",
                kind: .custom(payload: Data(), handler: Self.manifest.identifier)
            ),
            rankingHints: RankingHints(basePriority: Self.manifest.priority)
        )
    }

    private func unlockResult() -> ResultItem {
        lockedResult(title: "Unlock Secrets Vault", subtitle: "Unlock to search saved secrets")
    }

    private func secretResult(_ record: SecretRecord) async throws -> ResultItem {
        let value = try await vault.revealValue(id: record.id)
        let id = ResultID(module: Self.manifest.identifier, key: record.id.uuidString)
        let subtitle = record.account.isEmpty ? "Secret" : record.account
        return ResultItem(
            id: id,
            title: record.label,
            titleAttributed: AttributedString(record.label),
            subtitle: subtitle,
            icon: .symbol("lock.shield"),
            primaryAction: Action(
                id: ActionID(module: Self.manifest.identifier, key: "copy.\(record.id.uuidString)"),
                title: "Copy Secret",
                kind: .copyToPasteboard(value)
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
