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
    private var relockTimeout: TimeInterval = 300
    private var relockTask: Task<Void, Never>?
    private var onLockStateChanged: (@Sendable (Bool) async -> Void)?

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

    public func configure(relockTimeoutSeconds: Int, onLockStateChanged: (@Sendable (Bool) async -> Void)? = nil) {
        relockTimeout = TimeInterval(max(1, relockTimeoutSeconds))
        self.onLockStateChanged = onLockStateChanged
    }

    public func unlocked() -> Bool {
        isUnlocked
    }

    public func unlock() {
        isUnlocked = true
        scheduleRelock()
        notifyLockState()
    }

    public func lock() {
        relockTask?.cancel()
        relockTask = nil
        isUnlocked = false
        notifyLockState()
    }

    public func touchActivity() {
        guard isUnlocked else { return }
        scheduleRelock()
    }

    public func save(label: String, account: String = "", value: String) throws -> UUID {
        try requireUnlocked()
        let record = SecretMetadata(id: UUID(), label: label, account: account, updatedAt: Date())
        try keychain.save(value: value, account: record.id.uuidString)
        records[record.id] = record
        persistMetadata()
        touchActivity()
        return record.id
    }

    public func update(id: UUID, label: String, account: String, value: String?) throws {
        try requireUnlocked()
        guard records[id] != nil else { throw SecretsVaultError.notFound }
        records[id] = SecretMetadata(id: id, label: label, account: account, updatedAt: Date())
        if let value {
            try keychain.save(value: value, account: id.uuidString)
        }
        persistMetadata()
        touchActivity()
    }

    public func delete(id: UUID) throws {
        try requireUnlocked()
        guard records[id] != nil else { throw SecretsVaultError.notFound }
        records.removeValue(forKey: id)
        try keychain.delete(account: id.uuidString)
        persistMetadata()
        touchActivity()
    }

    public func allRecords() throws -> [SecretRecord] {
        try requireUnlocked()
        return records.values
            .sorted { $0.label.localizedStandardCompare($1.label) == .orderedAscending }
            .map { SecretRecord(id: $0.id, label: $0.label, account: $0.account, value: "", updatedAt: $0.updatedAt) }
    }

    public func searchLabels(_ query: String) throws -> [SecretRecord] {
        try requireUnlocked()
        touchActivity()
        let normalized = query.lowercased()
        return records.values
            .filter { normalized.isEmpty || $0.label.lowercased().contains(normalized) || $0.account.lowercased().contains(normalized) }
            .sorted { $0.label < $1.label }
            .map { SecretRecord(id: $0.id, label: $0.label, account: $0.account, value: "", updatedAt: $0.updatedAt) }
    }

    public func revealValue(id: UUID) throws -> String {
        try requireUnlocked()
        guard records[id] != nil else { throw SecretsVaultError.notFound }
        touchActivity()
        return try keychain.read(account: id.uuidString)
    }

    private func requireUnlocked() throws {
        guard isUnlocked else { throw SecretsVaultError.locked }
    }

    private func scheduleRelock() {
        relockTask?.cancel()
        relockTask = Task { [relockTimeout] in
            try? await Task.sleep(for: .seconds(relockTimeout))
            guard !Task.isCancelled else { return }
            await expireLock()
        }
    }

    private func expireLock() {
        guard isUnlocked else { return }
        isUnlocked = false
        relockTask = nil
        notifyLockState()
    }

    private func notifyLockState() {
        let locked = !isUnlocked
        if let onLockStateChanged {
            Task { await onLockStateChanged(locked) }
        }
    }

    private func persistMetadata() {
        try? FileManager.default.createDirectory(at: metadataURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        let encoded = Array(records.values)
        if let data = try? JSONEncoder().encode(encoded) {
            try? data.write(to: metadataURL, options: .atomic)
        }
    }
}
