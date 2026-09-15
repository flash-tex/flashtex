import Foundation
import Network

/// One stored pairing, keyed by the Mac's TXT `fp` (proposal §2, §7).
/// The companion app should keep `pair_psk` in the Keychain; the CLI keeps it
/// in a 0600 JSON file (`PairFile`).
public struct PairedMac: Codable, Equatable, Sendable {
    public var fingerprint: String
    public var macName: String
    public var pairId: String
    /// Long-term 32-byte PSK from `hello_ack.pair_psk`, base64.
    public var pairPsk: String
    public var companionName: String
    public var pairedAt: Date
    public var lastSeenAt: Date?
    /// Last `destination` the Mac reported (for display; always refreshed by hello).
    public var lastDestination: NearbyWire.Destination?
    enum CodingKeys: String, CodingKey {
        case fingerprint = "fp", macName = "mac_name", pairId = "pair_id", pairPsk = "pair_psk"
        case companionName = "companion_name", pairedAt = "paired_at", lastSeenAt = "last_seen_at"
        case lastDestination = "last_destination"
    }
    public init(fingerprint: String, macName: String, pairId: String, pairPsk: String, companionName: String,
                pairedAt: Date = Date(), lastSeenAt: Date? = nil, lastDestination: NearbyWire.Destination? = nil) {
        self.fingerprint = fingerprint; self.macName = macName; self.pairId = pairId; self.pairPsk = pairPsk
        self.companionName = companionName; self.pairedAt = pairedAt; self.lastSeenAt = lastSeenAt
        self.lastDestination = lastDestination
    }
    public var psk: Data? { Data(base64Encoded: pairPsk) }
}

/// An authenticated, hello-completed connection.
public final class NearbySession: @unchecked Sendable {
    public let connection: NearbyConnection
    public let ack: NearbyWire.HelloAck
    public var macName: String { ack.macName }
    /// Destination as of `hello_ack`; use `destinationQuery()` for a fresh one.
    public var destination: NearbyWire.Destination? { ack.destination }

    init(connection: NearbyConnection, ack: NearbyWire.HelloAck) { self.connection = connection; self.ack = ack }

    public func destinationQuery() async throws -> NearbyWire.Destination? { try await connection.destinationQuery() }

    public func submitCapture(_ c: NearbyWire.CaptureSubmit, requestID: String? = nil) async throws -> NearbyWire.CaptureReceived {
        try await connection.submitCapture(c, requestID: requestID)
    }

    /// Outcome of a submitted capture (additive `capture_status`).
    public func captureStatus(captureId: String, requestID: String? = nil) async throws -> NearbyWire.CaptureStatus {
        try await connection.captureStatus(captureId: captureId, requestID: requestID)
    }

    /// Builds a `capture_submit` for the given destination (or the hello_ack
    /// one) with the image bytes base64-encoded.
    public func makeCapture(captureId: String = "cap-" + UUID().uuidString.lowercased(), image: Data, mimeType: String,
                            instructions: String, destination: NearbyWire.Destination? = nil) throws -> NearbyWire.CaptureSubmit {
        guard let dest = destination ?? self.destination else {
            throw NearbyError.invalidInput("the Mac has no pinned insertion point (Edit > Pin Insertion Point) and no destination was given")
        }
        return NearbyWire.CaptureSubmit(captureId: captureId, destinationId: dest.destinationId, baseRevision: dest.baseRevision,
                                        image: .init(mimeType: mimeType, dataBase64: image.base64EncodedString()),
                                        instructions: instructions)
    }

    public var isOpen: Bool { connection.isOpen }
    public func close() { connection.close() }
}

/// Pairing and reconnecting, exactly as proposal §7 describes.
public enum NearbyClient {
    /// Bootstrap connection with the code the Mac displays: derive
    /// `pair_id`/`psk_boot` from (code, TXT salt), TLS-PSK with that key,
    /// `hello`, and take the long-term `pair_psk` from `hello_ack`.
    /// The returned session stays open and already uses the new pairing's identity.
    public static func pair(endpoint: NWEndpoint, salt: Data, fingerprint: String, macName: String,
                            code: String, companionName: String, connectTimeout: TimeInterval = 10,
                            onLine: ((NearbyConnection.Direction, Data) -> Void)? = nil) async throws -> (PairedMac, NearbySession) {
        guard salt.count == NearbyCrypto.saltLength else { throw NearbyError.unsupportedService("salt is not 16 bytes") }
        guard !code.isEmpty else { throw NearbyError.invalidInput("empty pairing code") }
        let derived = NearbyCrypto.derive(code: code, salt: salt)
        let conn = NearbyConnection(endpoint: endpoint, pairId: derived.pairId, psk: derived.psk)
        conn.onLine = onLine
        try await conn.connect(timeout: connectTimeout)
        let ack: NearbyWire.HelloAck
        do { ack = try await conn.hello(companionName: companionName, expectPairPsk: true) } catch { conn.close(); throw error }
        let pair = PairedMac(fingerprint: fingerprint, macName: ack.macName.isEmpty ? macName : ack.macName,
                             pairId: derived.pairId, pairPsk: ack.pairPsk!, companionName: companionName,
                             pairedAt: Date(), lastSeenAt: Date(), lastDestination: ack.destination)
        return (pair, NearbySession(connection: conn, ack: ack))
    }

    public static func pair(mac: DiscoveredMac, code: String, companionName: String, connectTimeout: TimeInterval = 10,
                            onLine: ((NearbyConnection.Direction, Data) -> Void)? = nil) async throws -> (PairedMac, NearbySession) {
        if let why = mac.unsupportedReason { throw NearbyError.unsupportedService("\(mac.name): \(why)") }
        return try await pair(endpoint: mac.endpoint, salt: mac.salt!, fingerprint: mac.fingerprint!, macName: mac.macName,
                              code: code, companionName: companionName, connectTimeout: connectTimeout, onLine: onLine)
    }

    /// Every later connection: TLS-PSK with the stored long-term key, then `hello`.
    public static func connect(endpoint: NWEndpoint, pair: PairedMac, connectTimeout: TimeInterval = 10,
                               helloTimeout: TimeInterval = 30,
                               onLine: ((NearbyConnection.Direction, Data) -> Void)? = nil) async throws -> NearbySession {
        guard let psk = pair.psk, psk.count == NearbyCrypto.pskLength else {
            throw NearbyError.invalidInput("stored pair_psk for \(pair.pairId) is not a 32-byte base64 key")
        }
        let conn = NearbyConnection(endpoint: endpoint, pairId: pair.pairId, psk: psk)
        conn.onLine = onLine
        try await conn.connect(timeout: connectTimeout)
        do {
            let ack = try await conn.hello(companionName: pair.companionName, timeout: helloTimeout)
            return NearbySession(connection: conn, ack: ack)
        } catch { conn.close(); throw error }
    }

    /// A reconnecting session for `pair` that re-browses Bonjour by `fp`
    /// before every attempt (so a Mac back on a new port is found), or dials
    /// `endpoint` directly when given.
    public static func reconnector(pair: PairedMac, endpoint: NWEndpoint? = nil, browseSeconds: TimeInterval = 5,
                                   policy: ReconnectPolicy = ReconnectPolicy(),
                                   onEvent: (@Sendable (NearbyReconnector.Event) -> Void)? = nil,
                                   onLine: ((NearbyConnection.Direction, Data) -> Void)? = nil) -> NearbyReconnector {
        if let endpoint { return NearbyReconnector(pair: pair, endpoint: endpoint, policy: policy, onEvent: onEvent, onLine: onLine) }
        let p = pair
        return NearbyReconnector(pair: pair, policy: policy, endpoints: { try await find(pair: p, seconds: browseSeconds).endpoint },
                                 onEvent: onEvent, onLine: onLine)
    }

    /// Browses until a service with the pairing's `fp` shows up.
    public static func find(pair: PairedMac, seconds: TimeInterval = 5) async throws -> DiscoveredMac {
        let results = try await NearbyBrowser.discover(seconds: seconds) { $0.fingerprint == pair.fingerprint }
        guard let mac = results.first(where: { $0.fingerprint == pair.fingerprint }) else {
            let seen = results.map { "\($0.name) fp=\($0.fingerprint ?? "?")" }.joined(separator: ", ")
            throw NearbyError.noMatchingMac("no \(NearbyWire.serviceType) service with fp \(pair.fingerprint) within \(seconds)s (seen: \(seen.isEmpty ? "none" : seen))")
        }
        if let why = mac.unsupportedReason { throw NearbyError.unsupportedService("\(mac.name): \(why)") }
        return mac
    }
}

/// Plain JSON file of pairings for the CLI and tests (mode 0600). An iOS
/// companion should use the Keychain instead; the record shape is the same.
public final class PairFile: @unchecked Sendable {
    public struct Contents: Codable, Sendable {
        public var version: Int
        public var pairs: [PairedMac]
    }
    public let url: URL
    /// Snapshot of the list. Mutations go through `upsert`/`remove` under `lock`
    /// because one instance is shared across threads and concurrency domains
    /// (the CLI, reconnect helpers, and tests call `upsert`/`remove`/`pair`
    /// off whatever queue they run on), so an unlocked read-modify-write could
    /// tear the array or lose a concurrent update.
    public var pairs: [PairedMac] { lock.withLock { _pairs } }
    private var _pairs: [PairedMac]
    private let lock = NSLock()

    public static func defaultURL() -> URL {
        if let env = ProcessInfo.processInfo.environment["NEARBY_CLIENT_STORE"], !env.isEmpty { return URL(fileURLWithPath: env) }
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("FlashTeX/nearby-client-pairs.json")
    }

    public init(url: URL) throws {
        self.url = url
        if let data = try? Data(contentsOf: url) {
            let dec = JSONDecoder()
            dec.dateDecodingStrategy = .iso8601
            let c = try dec.decode(Contents.self, from: data)
            guard c.version == 1 else { throw NearbyError.invalidInput("\(url.path): unsupported store version \(c.version)") }
            _pairs = c.pairs
        } else {
            _pairs = []
        }
    }

    public func pair(fingerprint: String) -> PairedMac? { lock.withLock { _pairs.first { $0.fingerprint == fingerprint } } }

    /// Matches a stored pairing by fp, pair_id or Mac name (case-insensitive).
    public func pair(matching key: String) -> PairedMac? {
        lock.withLock { _pairs.first { $0.fingerprint == key || $0.pairId == key || $0.macName.caseInsensitiveCompare(key) == .orderedSame } }
    }

    public func upsert(_ p: PairedMac) throws {
        try lock.withLock {
            _pairs.removeAll { $0.fingerprint == p.fingerprint }
            _pairs.append(p)
            try saveLocked()
        }
    }

    @discardableResult
    public func remove(fingerprint: String) throws -> Bool {
        try lock.withLock {
            let before = _pairs.count
            _pairs.removeAll { $0.fingerprint == fingerprint }
            try saveLocked()
            return _pairs.count != before
        }
    }

    /// Caller must hold `lock`. `saveLocked` runs under that lock so a
    /// concurrent `pairs` read cannot observe a torn list; the file always
    /// holds one complete snapshot.
    private func saveLocked() throws {
        let fm = FileManager.default
        try fm.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true,
                               attributes: [.posixPermissions: 0o700])
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        enc.dateEncodingStrategy = .iso8601
        try enc.encode(Contents(version: 1, pairs: _pairs)).write(to: url, options: [.atomic])
        try fm.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
    }
}
