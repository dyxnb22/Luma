import Foundation
import Testing
@testable import LumaModules

@Test func secretsVaultRequiresUnlockAndDoesNotExposeValuesInSearch() async throws {
    let vault = makeIsolatedSecretsVault()
    do {
        _ = try await vault.save(label: "OpenAI", value: "sk-secret")
        Issue.record("Save should fail while locked")
    } catch SecretsVaultError.locked {}

    await vault.unlock()
    let id = try await vault.save(label: "OpenAI", account: "api", value: "sk-secret")
    let results = try await vault.searchLabels("open")
    #expect(results.count == 1)
    #expect(results.first?.label == "OpenAI")
    #expect(try await vault.revealValue(id: id) == "sk-secret")
    #expect(results.first?.account == "api")
}

@Test func secretsVaultLocksAgain() async throws {
    let vault = makeIsolatedSecretsVault()
    await vault.unlock()
    _ = try await vault.save(label: "GitHub", value: "ghp-secret")
    await vault.lock()
    do {
        _ = try await vault.searchLabels("")
        Issue.record("Search should fail while locked")
    } catch SecretsVaultError.locked {}
}

@Test func secretsVaultReportsMissingRecord() async throws {
    let vault = makeIsolatedSecretsVault()
    await vault.unlock()
    do {
        _ = try await vault.revealValue(id: UUID())
        Issue.record("Missing secret should throw")
    } catch SecretsVaultError.notFound {}
}

private func makeIsolatedSecretsVault() -> SecretsVault {
    let metadataURL = FileManager.default.temporaryDirectory
        .appendingPathComponent(UUID().uuidString)
        .appendingPathComponent("secrets-metadata.json")
    return SecretsVault(
        keychain: KeychainSecretsStore(service: "app.luma.tests.\(UUID().uuidString)"),
        metadataURL: metadataURL
    )
}
