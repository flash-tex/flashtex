import Foundation
import NearbyClient
import Security

/// Where the iPad keeps a pairing (`pair_psk` is a long-term key, so the
/// Keychain — nearby-v1 §6 names the Keychain as the companion's store).
public protocol PairingStore: AnyObject, Sendable {
    var pairs: [PairedMac] { get }
    func upsert(_ p: PairedMac) throws
    @discardableResult func remove(fingerprint: String) throws -> Bool
}

extension PairFile: PairingStore {}

/// One generic-password item per Mac fingerprint (`kSecAttrAccount` = fp,
/// `kSecAttrService` = `service`), value = the `PairedMac` JSON — the same
/// record shape as the reference client's `PairFile`. `ThisDeviceOnly`
/// accessibility: the key never leaves this iPad (no iCloud Keychain sync).
public final class KeychainPairStore: PairingStore, @unchecked Sendable {
    public struct KeychainError: Error, CustomStringConvertible {
        public let status: OSStatus
        public let what: String
        public var description: String { "\(what): OSStatus \(status) (\(SecCopyErrorMessageString(status, nil).map { $0 as String } ?? "?"))" }
    }

    public static let defaultService = "tech.jay3332.flashtex.pad.nearby-pairs"
    public let service: String
    private let lock = NSLock()
    private var cache: [PairedMac]

    public init(service: String = KeychainPairStore.defaultService) {
        self.service = service
        cache = Self.readAll(service: service)
    }

    public var pairs: [PairedMac] { lock.withLock { cache } }

    public func pair(fingerprint: String) -> PairedMac? { pairs.first { $0.fingerprint == fingerprint } }

    private static func encoder() -> JSONEncoder { let e = JSONEncoder(); e.dateEncodingStrategy = .iso8601; e.outputFormatting = [.sortedKeys]; return e }
    private static func decoder() -> JSONDecoder { let d = JSONDecoder(); d.dateDecodingStrategy = .iso8601; return d }

    private static func baseQuery(service: String, account: String?) -> [String: Any] {
        var q: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service]
        if let account { q[kSecAttrAccount as String] = account }
        return q
    }

    private static func readAll(service: String) -> [PairedMac] {
        var q = baseQuery(service: service, account: nil)
        q[kSecReturnData as String] = true
        q[kSecReturnAttributes as String] = true
        q[kSecMatchLimit as String] = kSecMatchLimitAll
        var out: CFTypeRef?
        let status = SecItemCopyMatching(q as CFDictionary, &out)
        guard status == errSecSuccess, let items = out as? [[String: Any]] else { return [] }
        let dec = decoder()
        return items.compactMap { item in
            guard let data = item[kSecValueData as String] as? Data else { return nil }
            return try? dec.decode(PairedMac.self, from: data)
        }.sorted { $0.pairedAt < $1.pairedAt }
    }

    public func upsert(_ p: PairedMac) throws {
        let data = try Self.encoder().encode(p)
        let query = Self.baseQuery(service: service, account: p.fingerprint)
        let attrs: [String: Any] = [kSecValueData as String: data,
                                    kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
                                    kSecAttrLabel as String: "FlashTeX nearby pairing: \(p.macName)"]
        var status = SecItemUpdate(query as CFDictionary, [kSecValueData as String: data] as CFDictionary)
        if status == errSecItemNotFound {
            status = SecItemAdd(query.merging(attrs) { $1 } as CFDictionary, nil)
        }
        guard status == errSecSuccess else { throw KeychainError(status: status, what: "storing the pairing for \(p.fingerprint)") }
        lock.withLock { cache.removeAll { $0.fingerprint == p.fingerprint }; cache.append(p) }
    }

    @discardableResult
    public func remove(fingerprint: String) throws -> Bool {
        let status = SecItemDelete(Self.baseQuery(service: service, account: fingerprint) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else { throw KeychainError(status: status, what: "removing the pairing for \(fingerprint)") }
        return lock.withLock {
            let before = cache.count
            cache.removeAll { $0.fingerprint == fingerprint }
            return cache.count != before
        }
    }

    /// Everything under `service` (tests).
    public func removeAll() throws {
        let status = SecItemDelete(Self.baseQuery(service: service, account: nil) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else { throw KeychainError(status: status, what: "clearing \(service)") }
        lock.withLock { cache.removeAll() }
    }
}

/// Drafts, receipts and outcomes on disk: `captures.json` (one entry per
/// capture: status, receipt, last `capture_status_ack`, instruction,
/// destination) beside `captures/<capture_id>.png` (the strokes or photo as
/// sent). Restored on relaunch: an entry that was `sending` when the app died
/// comes back retryable with the same `capture_id` (the Mac de-duplicates).
public final class CaptureStore: @unchecked Sendable {
    public struct Entry: Codable, Equatable {
        public var id: String
        public var source: String
        public var instructions: String
        public var destinationId: String?
        public var baseRevision: Int?
        public var pairId: String?
        public var macFingerprint: String?
        public var status: CaptureRecord.Status
        public var outcome: NearbyWire.CaptureStatus?
        public var outcomeProblem: String?
        public var createdAt: Date
        public var width: Int?
        public var height: Int?
    }
    struct File: Codable { var version: Int; var entries: [Entry] }

    public let directory: URL
    private let lock = NSLock()

    public static func defaultDirectory() -> URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("FlashTeXPad", isDirectory: true)
    }

    public init(directory: URL = CaptureStore.defaultDirectory()) {
        self.directory = directory
    }

    var indexURL: URL { directory.appendingPathComponent("captures.json") }
    public func pngURL(_ id: String) -> URL { directory.appendingPathComponent("captures", isDirectory: true).appendingPathComponent("\(id).png") }

    /// Records as last saved, newest first; a missing or undecodable index is
    /// an empty list (never a crash at launch). PNGs that vanished drop the entry.
    public func load() -> [CaptureRecord] {
        lock.withLock {
            guard let data = try? Data(contentsOf: indexURL) else { return [] }
            let dec = JSONDecoder(); dec.dateDecodingStrategy = .iso8601
            guard let file = try? dec.decode(File.self, from: data), file.version == 1 else { return [] }
            return file.entries.compactMap { e in
                guard let png = try? Data(contentsOf: pngURL(e.id)), let source = CaptureRecord.Source(rawValue: e.source) else { return nil }
                var r = CaptureRecord(id: e.id, source: source, png: png, instructions: e.instructions,
                                      pixelSize: e.width.flatMap { w in e.height.map { (width: w, height: $0) } })
                r.destinationId = e.destinationId; r.baseRevision = e.baseRevision; r.createdAt = e.createdAt
                r.pairId = e.pairId; r.macFingerprint = e.macFingerprint
                r.outcome = e.outcome; r.outcomeProblem = e.outcomeProblem
                if case .sending(let n) = e.status { r.status = .disconnected(reason: "app relaunched before the receipt", attempt: n) }
                else { r.status = e.status }
                return r
            }
        }
    }

    public func save(_ records: [CaptureRecord]) throws {
        try lock.withLock {
            let fm = FileManager.default
            try fm.createDirectory(at: directory.appendingPathComponent("captures", isDirectory: true), withIntermediateDirectories: true)
            var entries: [Entry] = []
            for r in records {
                let url = pngURL(r.id)
                if !fm.fileExists(atPath: url.path) { try r.png.write(to: url, options: [.atomic]) }
                entries.append(Entry(id: r.id, source: r.source.rawValue, instructions: r.instructions, destinationId: r.destinationId,
                                     baseRevision: r.baseRevision, pairId: r.pairId, macFingerprint: r.macFingerprint, status: r.status, outcome: r.outcome, outcomeProblem: r.outcomeProblem,
                                     createdAt: r.createdAt, width: r.pixelSize?.width, height: r.pixelSize?.height))
            }
            let enc = JSONEncoder(); enc.dateEncodingStrategy = .iso8601; enc.outputFormatting = [.prettyPrinted, .sortedKeys]
            try enc.encode(File(version: 1, entries: entries)).write(to: indexURL, options: [.atomic])
            // PNGs of records no longer listed (discarded and pruned) go too.
            let keep = Set(records.map { "\($0.id).png" })
            for f in (try? fm.contentsOfDirectory(atPath: directory.appendingPathComponent("captures").path)) ?? [] where !keep.contains(f) {
                try? fm.removeItem(at: directory.appendingPathComponent("captures").appendingPathComponent(f))
            }
        }
    }

    public func wipe() {
        lock.withLock { try? FileManager.default.removeItem(at: directory) }
    }
}
