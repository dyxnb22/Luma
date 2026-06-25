import Foundation
import LumaCore
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

@Test func secretsVaultUpdateAndDelete() async throws {
    let vault = makeIsolatedSecretsVault()
    await vault.unlock()
    let id = try await vault.save(label: "OpenAI", account: "api", value: "sk-old")
    try await vault.update(id: id, label: "OpenAI Prod", account: "prod", value: "sk-new")
    let records = try await vault.allRecords()
    #expect(records.count == 1)
    #expect(records.first?.label == "OpenAI Prod")
    #expect(try await vault.revealValue(id: id) == "sk-new")

    try await vault.update(id: id, label: "OpenAI Prod", account: "prod", value: nil)
    #expect(try await vault.revealValue(id: id) == "sk-new")

    try await vault.delete(id: id)
    #expect(try await vault.allRecords().isEmpty)
}

@Test func secretsVaultAutoRelocks() async throws {
    let vault = makeIsolatedSecretsVault()
    await vault.configure(relockTimeoutSeconds: 1)
    await vault.unlock()
    #expect(await vault.unlocked())
    try await Task.sleep(for: .milliseconds(1500))
    #expect(await vault.unlocked() == false)
}

@Test func secretsModuleAcceptsBareSecretTrigger() async {
    let vault = makeIsolatedSecretsVault()
    let module = SecretsModule(vault: vault)
    await module.unlock()
    let context = QueryContext(deadline: ContinuousClock().now.advanced(by: .milliseconds(30)))

    let bare = await module.handle(Query(raw: "secret", sequence: 1), context: context)
    #expect(!bare.items.isEmpty)

    let unrelated = await module.handle(Query(raw: "secretary", sequence: 2), context: context)
    #expect(unrelated.items.isEmpty)
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
