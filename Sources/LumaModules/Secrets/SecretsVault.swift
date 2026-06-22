import Foundation

public struct SecretRecord: Identifiable, Sendable, Hashable {
    public let id: UUID
    public var label: String
    public var account: String
    fileprivate var value: String
    public var updatedAt: Date

    public init(id: UUID = UUID(), label: String, account: String = "", value: String, updatedAt: Date = Date()) {
        self.id = id
        self.label = label
        self.account = account
        self.value = value
        self.updatedAt = updatedAt
    }
}

private struct SecretMetadata: Codable, Sendable {
    let id: UUID
    var label: String
    var account: String
    var updatedAt: Date
}

public enum SecretsVaultError: Error, Equatable {
    case locked
    case notFound
}

public actor SecretsVault {
    private let keychain: KeychainSecretsStore
    private let metadataURL: URL
    private var isUnlocked = false
    private var records: [UUID: SecretMetadata] = [:]

    public init(
        keychain: KeychainSecretsStore = KeychainSecretsStore(),
        fileManager: FileManager = .default,
        metadataURL: URL? = nil
    ) {
        self.keychain = keychain
        if let metadataURL {
            self.metadataURL = metadataURL
        } else {
            let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
                ?? fileManager.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
            self.metadataURL = base.appendingPathComponent("Luma/secrets-metadata.json")
        }
        if let data = try? Data(contentsOf: self.metadataURL),
           let decoded = try? JSONDecoder().decode([SecretMetadata].self, from: data) {
            records = Dictionary(uniqueKeysWithValues: decoded.map { ($0.id, $0) })
        }
    }

    public func unlocked() -> Bool {
        isUnlocked
    }

    public func unlock() {
        isUnlocked = true
    }

    public func lock() {
        isUnlocked = false
    }

    public func save(label: String, account: String = "", value: String) throws -> UUID {
        guard isUnlocked else { throw SecretsVaultError.locked }
        let record = SecretMetadata(id: UUID(), label: label, account: account, updatedAt: Date())
        try keychain.save(value: value, account: record.id.uuidString)
        records[record.id] = record
        persistMetadata()
        return record.id
    }

    public func searchLabels(_ query: String) throws -> [SecretRecord] {
        guard isUnlocked else { throw SecretsVaultError.locked }
        let normalized = query.lowercased()
        return records.values
            .filter { normalized.isEmpty || $0.label.lowercased().contains(normalized) || $0.account.lowercased().contains(normalized) }
            .sorted { $0.label < $1.label }
            .map { SecretRecord(id: $0.id, label: $0.label, account: $0.account, value: "", updatedAt: $0.updatedAt) }
    }

    public func revealValue(id: UUID) throws -> String {
        guard isUnlocked else { throw SecretsVaultError.locked }
        guard records[id] != nil else { throw SecretsVaultError.notFound }
        return try keychain.read(account: id.uuidString)
    }

    private func persistMetadata() {
        try? FileManager.default.createDirectory(at: metadataURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        let encoded = Array(records.values)
        if let data = try? JSONEncoder().encode(encoded) {
            try? data.write(to: metadataURL, options: .atomic)
        }
    }
}
