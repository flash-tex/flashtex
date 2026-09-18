import CryptoKit
import Foundation

/// Pairing-code derivation for the nearby transport (proposal nearby-v1).
///
/// The Mac shows a short code; both sides derive the same bootstrap PSK and
/// `pair_id` from (code, Mac salt) with HKDF-SHA256. The bootstrap PSK only
/// authenticates the first connection: the Mac then mints a random 32-byte
/// long-term PSK and hands it over inside that TLS session (`hello_ack.pair_psk`).
enum Pairing {
    static let codeLength = 6
    static let codeLifetime: TimeInterval = 120
    /// How many times a shown code that expired unused is replaced in place
    /// (fresh code, next generation, same window) before the attempt fails:
    /// 5 rolls = 5 extra codes = 10 more minutes of an open, unanswered sheet
    /// (12 in all). Each code still lives `codeLifetime`; rolling never
    /// lengthens one.
    static let maxCodeRolls = 5
    static let saltLength = 16
    static let pskLength = 32
    static let pskInfo = Data("flashtex-nearby-v1 psk".utf8)
    static let pairIDInfo = Data("flashtex-nearby-v1 pair-id".utf8)
    static let fingerprintInfo = Data("flashtex-nearby-v1 mac-id".utf8)

    struct Derived: Equatable {
        let pairId: String
        let psk: Data
    }

    /// Six decimal digits from the system CSPRNG (~20 bits; see the proposal's
    /// threat model for why this is only a bootstrap secret).
    static func generateCode() -> String {
        var g = SystemRandomNumberGenerator()
        return (0..<codeLength).map { _ in String(Int.random(in: 0...9, using: &g)) }.joined()
    }

    static func generateSalt() -> Data { randomBytes(saltLength) }
    static func mintLongTermPSK() -> Data { randomBytes(pskLength) }

    static func randomBytes(_ count: Int) -> Data {
        SymmetricKey(size: .init(bitCount: count * 8)).withUnsafeBytes { Data($0) }
    }

    /// Deterministic: the companion runs the same derivation from the code it
    /// typed and the `salt=` TXT value.
    static func derive(code: String, salt: Data) -> Derived {
        let ikm = SymmetricKey(data: Data(code.utf8))
        let psk = HKDF<SHA256>.deriveKey(inputKeyMaterial: ikm, salt: salt, info: pskInfo, outputByteCount: pskLength)
        let id = HKDF<SHA256>.deriveKey(inputKeyMaterial: ikm, salt: salt, info: pairIDInfo, outputByteCount: 8)
        return Derived(pairId: hex(id.withUnsafeBytes { Data($0) }), psk: psk.withUnsafeBytes { Data($0) })
    }

    /// Stable per-Mac identifier advertised as TXT `fp=` so a companion can
    /// match a discovered service to a stored pairing without connecting.
    static func fingerprint(salt: Data) -> String {
        var h = SHA256()
        h.update(data: fingerprintInfo)
        h.update(data: salt)
        return String(hex(Data(h.finalize())).prefix(16))
    }

    /// Test vector: code "123456", salt 000102…0f (see docs/nearby-v1-proposal.md).
    static let vectorPairID = "3917d7c3e5eef7ce"
    static let vectorPSKHex = "b127a48a782dbece3edbed18d027d658890624ecf21da77d19f47d5e604d1221"
    static let vectorFingerprint = "0e712816d64b7c47"
    /// helloProof(psk: vector PSK, nonce: "n-1")
    static let vectorHelloProof = "rIBrMvRrNjs2eQueTGVsBNF6K130BW6fqV93Rw//WgM="

    static let helloProofInfo = Data("flashtex-nearby-v1 hello".utf8)

    /// `hello.proof`: HMAC-SHA256(psk, "flashtex-nearby-v1 hello" || nonce), base64.
    /// Network.framework does not expose which table PSK a TLS session used, so
    /// the companion proves possession of the key for the `pair_id` it claims.
    static func helloProof(psk: Data, nonce: String) -> String {
        Data(HMAC<SHA256>.authenticationCode(for: helloProofInfo + Data(nonce.utf8), using: SymmetricKey(data: psk)))
            .base64EncodedString()
    }

    static func verifyHelloProof(_ proof: String, psk: Data, nonce: String) -> Bool {
        guard let given = Data(base64Encoded: proof) else { return false }
        return HMAC<SHA256>.isValidAuthenticationCode(given, authenticating: helloProofInfo + Data(nonce.utf8),
                                                      using: SymmetricKey(data: psk))
    }

    static func hex(_ data: Data) -> String { data.map { String(format: "%02x", $0) }.joined() }

    static func data(hex: String) -> Data? {
        guard hex.count % 2 == 0 else { return nil }
        var out = Data(capacity: hex.count / 2)
        var i = hex.startIndex
        while i < hex.endIndex {
            let j = hex.index(i, offsetBy: 2)
            guard let b = UInt8(hex[i..<j], radix: 16) else { return nil }
            out.append(b)
            i = j
        }
        return out
    }
}

/// Per-companion permission, persisted with the pairing record and checked by
/// the listener on every `capture_submit`. A `viewOnly` companion still pairs,
/// reconnects and reads the destination; its captures are refused with
/// `capture_not_permitted` (session kept open) until the Mac changes it.
enum CompanionPermission: String, Codable, CaseIterable, Identifiable {
    case captures
    case viewOnly = "view_only"
    var id: String { rawValue }
    var allowsCaptures: Bool { self == .captures }
    /// Pop-up title in the Nearby Companion window.
    var title: String {
        switch self {
        case .captures: return "Captures allowed"
        case .viewOnly: return "View only"
        }
    }
    /// Spoken with the device row.
    var spoken: String {
        switch self {
        case .captures: return "captures allowed"
        case .viewOnly: return "view only, captures refused"
        }
    }
    static let refusalCode = "capture_not_permitted"
    static func refusalMessage(pairId: String) -> String {
        "companion \(pairId) is view-only on this Mac; change its permission in Nearby Companion to accept captures"
    }
}

/// One paired companion. `psk` is the long-term key (base64, 32 bytes).
/// String conversion never includes the key: interpolating a record into a
/// log line or an error message yields `pairId`, name and timestamps only.
struct PairRecord: Codable, Equatable, Identifiable, CustomStringConvertible, CustomDebugStringConvertible {
    var pairId: String
    var psk: String
    var companionName: String
    var createdAt: Date
    var lastSeenAt: Date?
    /// Pairing attempt (`PairingCoordinator.Pending.generation`, journaled by
    /// `PairingJournal`) that confirmed this record; nil for records written
    /// by schema v1 files.
    var generation: Int? = nil
    /// Bounded activity summary (mac-nearby-transport): captures accepted from
    /// this pairing, and the last one. Optional so v1/v2 files without them decode.
    var captureCount: Int? = nil
    var lastCaptureAt: Date? = nil
    var lastCaptureId: String? = nil
    /// What this companion may do (pairs.json v3, key `permission`). Records
    /// from v1/v2 files decode with nil and are upgraded to the explicit
    /// `captures` value they behaved as; `effectivePermission` is what the
    /// listener enforces.
    var permission: CompanionPermission? = nil
    /// Acknowledged captures (mac-nearby-transport-3), oldest first, at most
    /// `PairStore.maxRememberedCaptures`: seeds the listener's acknowledgement
    /// memory after `stopAdvertising()` or a relaunch so a companion's retry
    /// is acknowledged again, never re-delivered. Optional so older files decode.
    var rememberedCaptures: [NearbyAckMemory.Persisted]? = nil
    var id: String { pairId }
    enum CodingKeys: String, CodingKey {
        case pairId = "pair_id", psk, companionName = "companion_name"
        case createdAt = "created_at", lastSeenAt = "last_seen_at", generation
        case captureCount = "capture_count", lastCaptureAt = "last_capture_at", lastCaptureId = "last_capture_id"
        case permission
        case rememberedCaptures = "remembered_captures"
    }
    var pskData: Data? { Data(base64Encoded: psk) }
    var effectivePermission: CompanionPermission { permission ?? .captures }

    var description: String {
        "PairRecord(\(pairId) “\(companionName)” created \(Pairing.stamp(createdAt))"
            + (lastSeenAt.map { " seen \(Pairing.stamp($0))" } ?? "") + ", psk: <redacted>)"
    }
    var debugDescription: String { description }
}

/// Plain-file store for pairings: `~/Library/Application Support/FlashTeX/pairs.json`,
/// mode 0600, written atomically. Not the Keychain — see the proposal's
/// "Not provided" list; moving the keys there is a follow-up.
final class PairStore {
    /// On-disk schema of `pairs.json`. Version 1: `{version, salt, pairs[]}` as
    /// documented in `apps/mac/docs/nearby-v1-proposal.md` §2. A file written by
    /// a newer build (`version > schemaVersion`) is left untouched and the store
    /// starts empty with `loadError` set; an older version is upgraded in
    /// memory by `upgrade(_:)` and rewritten on the next persist.
    static let schemaVersion = 3 // v2: per-record optional `generation`; v3: per-record `permission` (v1/v2 records decode with nil)

    struct File: Codable {
        var version: Int
        var salt: String
        var pairs: [PairRecord]
    }

    enum LoadOutcome: Equatable { case created, loaded(version: Int), upgraded(from: Int), refused(reason: String) }
    struct DecodeError: Error, Equatable { let reason: String }

    let url: URL
    private let lock = NSLock()
    private var file: File
    private(set) var loadError: String?
    /// What the initializer found at `url` (evidence for recovery reports).
    private(set) var loadOutcome: LoadOutcome = .created

    /// `FLASHTEX_PAIR_STORE=<path>` overrides the location (automation and
    /// evidence runs must not touch the user's real pairings).
    static func defaultURL() -> URL {
        if let p = ProcessInfo.processInfo.environment["FLASHTEX_PAIR_STORE"], !p.isEmpty { return URL(fileURLWithPath: p) }
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("FlashTeX/pairs.json")
    }

    init(url: URL) {
        self.url = url
        let fresh = File(version: Self.schemaVersion, salt: Pairing.hex(Pairing.generateSalt()), pairs: [])
        if let data = try? Data(contentsOf: url) {
            switch Self.decode(data) {
            case .success(let (f, outcome)):
                file = f
                loadOutcome = outcome
                if case .upgraded = outcome { _ = try? persist() }
            case .failure(let e):
                // Keep a corrupt or too-new file rather than silently overwriting it.
                file = fresh
                loadOutcome = .refused(reason: e.reason)
                loadError = "\(url.path): \(e.reason); starting empty without overwriting it"
            }
        } else {
            file = fresh
            loadOutcome = .created
            do { try persist() } catch { loadError = "cannot create \(url.path): \(error.localizedDescription)" }
        }
    }

    /// Decodes a pair-store file, upgrading older schema versions in memory.
    static func decode(_ data: Data) -> Result<(File, LoadOutcome), DecodeError> {
        struct Header: Decodable { var version: Int }
        guard let header = try? JSONDecoder().decode(Header.self, from: data) else {
            return .failure(DecodeError(reason: "not a pair store (no integer `version`)"))
        }
        guard header.version <= schemaVersion else {
            return .failure(DecodeError(reason: "pair store version \(header.version) is newer than this build's \(schemaVersion)"))
        }
        guard header.version >= 1 else { return .failure(DecodeError(reason: "pair store version \(header.version) is not supported")) }
        let dec = JSONDecoder()
        dec.dateDecodingStrategy = .iso8601
        guard var f = try? dec.decode(File.self, from: data) else {
            return .failure(DecodeError(reason: "version-\(header.version) pair store is undecodable"))
        }
        guard let salt = Pairing.data(hex: f.salt), salt.count == Pairing.saltLength else {
            return .failure(DecodeError(reason: "pair store salt is not \(Pairing.saltLength) hex bytes"))
        }
        // Every record must carry a usable long-term key; drop nothing silently.
        guard f.pairs.allSatisfy({ $0.pskData?.count == Pairing.pskLength && !$0.pairId.isEmpty }) else {
            return .failure(DecodeError(reason: "pair store holds a record without a \(Pairing.pskLength)-byte key"))
        }
        if f.version < schemaVersion {
            f = upgrade(f)
            return .success((f, .upgraded(from: header.version)))
        }
        return .success((f, .loaded(version: header.version)))
    }

    /// Schema upgrades, oldest first. v1→v2 added the optional per-record
    /// `generation` (nothing to rewrite); v2→v3 writes the explicit
    /// `permission` every older record behaved as (`captures`).
    static func upgrade(_ old: File) -> File {
        var f = old
        if f.version < 3 {
            for i in f.pairs.indices where f.pairs[i].permission == nil { f.pairs[i].permission = .captures }
        }
        f.version = schemaVersion
        return f
    }

    var salt: Data { lock.withLock { Pairing.data(hex: file.salt) ?? Data() } }
    var pairs: [PairRecord] { lock.withLock { file.pairs } }
    func pair(id: String) -> PairRecord? { lock.withLock { file.pairs.first { $0.pairId == id } } }

    @discardableResult
    func upsert(_ record: PairRecord) -> Bool {
        lock.withLock {
            guard loadError == nil else { return false }
            file.pairs.removeAll { $0.pairId == record.pairId }
            file.pairs.append(record)
            return (try? persist()) != nil
        }
    }

    @discardableResult
    func remove(pairId: String) -> Bool {
        lock.withLock {
            file.pairs.removeAll { $0.pairId == pairId }
            return (try? persist()) != nil
        }
    }

    func touch(pairId: String) {
        lock.withLock {
            guard let i = file.pairs.firstIndex(where: { $0.pairId == pairId }) else { return }
            file.pairs[i].lastSeenAt = Date()
            _ = try? persist()
        }
    }

    /// Changes what a companion may do; persisted at once (pairs.json v3).
    @discardableResult
    func setPermission(pairId: String, _ permission: CompanionPermission) -> Bool {
        lock.withLock {
            guard loadError == nil, let i = file.pairs.firstIndex(where: { $0.pairId == pairId }) else { return false }
            file.pairs[i].permission = permission
            return (try? persist()) != nil
        }
    }

    /// The listener's check for one `capture_submit`. A pairing the store does
    /// not hold could not have authenticated with a long-term key, so it is
    /// not refused here (bare-listener tests, bootstrap sessions mid-confirm).
    func capturesPermitted(pairId: String) -> Bool {
        lock.withLock { file.pairs.first { $0.pairId == pairId }?.effectivePermission.allowsCaptures ?? true }
    }
    /// Bound on `PairRecord.rememberedCaptures` (each entry is a few hundred
    /// bytes: ids, revision, digest, ack flags).
    static let maxRememberedCaptures = 64

    /// Records one accepted capture on the pairing's bounded summary
    /// (count, last id ≤ 128 bytes, timestamp), marks it seen, and appends
    /// `remembered` (replacing an entry with the same id) under the bound.
    func recordCapture(pairId: String, captureId: String, at date: Date = Date(),
                       remembered: NearbyAckMemory.Persisted? = nil) {
        lock.withLock {
            guard let i = file.pairs.firstIndex(where: { $0.pairId == pairId }) else { return }
            file.pairs[i].captureCount = (file.pairs[i].captureCount ?? 0) + 1
            file.pairs[i].lastCaptureAt = date
            file.pairs[i].lastCaptureId = String(captureId.prefix(128))
            file.pairs[i].lastSeenAt = date
            if let remembered {
                var list = file.pairs[i].rememberedCaptures ?? []
                list.removeAll { $0.captureId == remembered.captureId }
                list.append(remembered)
                if list.count > Self.maxRememberedCaptures { list.removeFirst(list.count - Self.maxRememberedCaptures) }
                file.pairs[i].rememberedCaptures = list
            }
            _ = try? persist()
        }
    }

    /// Caller holds `lock` (or is the initializer).
    private func persist() throws {
        let fm = FileManager.default
        let dir = url.deletingLastPathComponent()
        try fm.createDirectory(at: dir, withIntermediateDirectories: true,
                               attributes: [.posixPermissions: 0o700])
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        enc.dateEncodingStrategy = .iso8601
        let data = try enc.encode(file)
        try data.write(to: url, options: [.atomic])
        try fm.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
    }
}

extension Pairing {
    private static let iso: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime]
        return f
    }()
    /// ISO-8601 UTC seconds, for logs and journal evidence.
    static func stamp(_ date: Date) -> String { iso.string(from: date) }

    /// Digits separated by spaces ("123 456"), what the window displays.
    static func displayCode(_ code: String) -> String {
        stride(from: 0, to: code.count, by: 3).map { i -> String in
            let s = code.index(code.startIndex, offsetBy: i)
            let e = code.index(s, offsetBy: 3, limitedBy: code.endIndex) ?? code.endIndex
            return String(code[s..<e])
        }.joined(separator: " ")
    }

    /// One digit per word, grouped in pairs with a pause between the groups
    /// ("1 2, 3 4, 5 6") so VoiceOver spells the code the way it is typed.
    static func spokenCode(_ code: String) -> String {
        stride(from: 0, to: code.count, by: 2).map { i -> String in
            let s = code.index(code.startIndex, offsetBy: i)
            let e = code.index(s, offsetBy: 2, limitedBy: code.endIndex) ?? code.endIndex
            return code[s..<e].map(String.init).joined(separator: " ")
        }.joined(separator: ", ")
    }

    /// What "Copy code" and ⌘C on the code put on the pasteboard: the digits
    /// without the display spacing, ready to paste into the companion.
    static func clipboardCode(_ code: String) -> String { code.filter(\.isNumber) }
}

// MARK: - pairing flow state machine

/// The pairing window's state, modelled explicitly so that every transition is
/// testable and a stale event (from an attempt that was cancelled, expired, or
/// replaced by a newer code) can never overwrite the current pairing.
///
/// Each "Show Pairing Code" is an `Attempt` with a monotonically increasing
/// `generation` (persisted by `PairingJournal`, so it survives relaunch).
/// Inputs that concern one attempt carry its generation or its `pairId`; the
/// machine ignores any that do not match the attempt it is showing.
///
/// The machine is pure: `Machine.apply` returns the effects the owner must
/// perform against the transport (`NearbyState`) and the journal.
enum PairingFlow {
    /// One pairing code and everything needed to show or resume it.
    struct Attempt: Codable, Equatable {
        var generation: Int
        var code: String
        var pairId: String
        var startedAt: Date
        var expiresAt: Date
        /// How many earlier codes of this same sitting expired unused before
        /// this one was minted (`Pairing.maxCodeRolls` bounds it). 0 for a
        /// code the user asked for; journals written before rolling existed
        /// decode as 0.
        var rolls: Int = 0
        enum CodingKeys: String, CodingKey {
            case generation, code, pairId = "pair_id", startedAt = "started_at", expiresAt = "expires_at", rolls
        }
        func remaining(at now: Date) -> TimeInterval { max(0, expiresAt.timeIntervalSince(now)) }
        func isExpired(at now: Date) -> Bool { now >= expiresAt }
        var canRoll: Bool { rolls < Pairing.maxCodeRolls }
    }

    /// Why a code that was being shown is no longer being served.
    enum Interruption: String, Codable, Equatable {
        /// The app was quit or crashed while the code was valid.
        case relaunch
        /// A bootstrap session opened but ended before the pairing completed.
        case peerGone
        /// The transport stopped serving the code: listener failure, advertising
        /// turned off, or the code withdrawn by another caller.
        case transportStopped
    }

    struct Paired: Equatable {
        var pairId: String
        var companionName: String
        var generation: Int
    }

    struct Receiving: Equatable {
        var pairId: String
        var companionName: String?
        var captureId: String?
        var bytes: Int
        /// Unknown until the transport reports a length (JSON lines do not announce one).
        var total: Int?
    }

    enum Phase: Equatable {
        /// Listener off; nothing can pair.
        case off
        /// Listener up and advertising; paired companions may connect.
        case advertising
        /// A code is displayed and its bootstrap key is accepted.
        case codeShown(Attempt)
        /// A bootstrap session opened; waiting for a valid `hello`.
        case verifying(Attempt)
        /// A pairing was stored (banner until dismissed or the next event).
        case paired(Paired)
        /// A paired companion is sending a capture.
        case receiving(Receiving)
        /// A code that may still be valid stopped being served; user must resume or cancel.
        case interrupted(Attempt, Interruption, detail: String)
        /// Something ended with a reason the user must see and dismiss.
        case failed(reason: String, generation: Int)
    }

    enum Input: Equatable {
        case advertising(Bool)
        /// The transport issued a new code (always a new generation).
        case codeIssued(Attempt)
        /// A pending attempt was found in the journal at launch.
        case restored(Attempt)
        case bootstrapSessionOpened(generation: Int)
        case confirmed(pairId: String, companionName: String, generation: Int)
        /// The attempt's code ran out. `replacement` (a fresh code the owner
        /// minted, next generation, same salt) is taken only while the code is
        /// still shown with no companion connected and the attempt has rolls
        /// left; otherwise the attempt fails as if none were offered.
        case codeExpired(generation: Int, replacement: Attempt? = nil)
        case peerGone(pairId: String?, reason: String, generation: Int)
        case listenerFailed(String)
        /// The transport stopped serving the attempt's code without the user asking.
        case withdrawn(generation: Int, reason: String)
        /// A session opened while the code was shown turned out to be an already
        /// paired companion, not the one being paired.
        case otherCompanionConnected(generation: Int)
        case receiving(pairId: String, companionName: String?, captureId: String?, bytes: Int, total: Int?)
        case captureReceived(pairId: String?, captureId: String)
        /// The transport refused a capture with an `error` reply (or the Mac
        /// closed the session while receiving it).
        case captureRefused(pairId: String?, captureId: String?, code: String)
        case forgotten(pairId: String)
        /// A paired companion (long-term key) said hello, i.e. it reconnected;
        /// not the one being paired.
        case companionConnected(pairId: String, companionName: String)
        /// A paired companion's session closed while no receive from it was
        /// in progress (the controller sends this only for known pair ids).
        case companionDisconnected(pairId: String, companionName: String, reason: String)
        /// User actions.
        case cancel
        case resume
        case dismiss
    }

    enum Effect: Equatable {
        /// Write the attempt to the journal (it is now the pending pairing).
        case persist(Attempt)
        /// Remove the pending attempt from the journal.
        case clearJournal
        /// Tell the transport to stop accepting the current attempt's bootstrap key.
        case cancelTransport(Attempt)
        /// Tell the transport to accept this attempt's bootstrap key (again
        /// after an interruption, or for the first time after a code rolled).
        case resumeTransport(Attempt)
        /// Close every live session of one pairing (cancels a receive).
        case closeSession(pairId: String)
        /// Post an accessibility announcement.
        case announce(String)
    }

    struct Outcome: Equatable {
        var effects: [Effect] = []
        /// The input belonged to an older attempt (or another pairing) and was ignored.
        var stale = false
        /// The input made no sense in the current phase and was ignored (not stale).
        var ignored = false
        static let none = Outcome()
        static let staleInput = Outcome(stale: true)
        static let ignoredInput = Outcome(ignored: true)
    }

    struct Machine: Equatable {
        private(set) var phase: Phase
        /// Generation of the newest attempt seen; anything older is stale.
        private(set) var generation: Int
        private(set) var isAdvertising: Bool

        init(phase: Phase = .off, generation: Int = 0, isAdvertising: Bool = false) {
            self.phase = phase
            self.generation = generation
            self.isAdvertising = isAdvertising
        }

        /// The attempt currently shown, verified, or interrupted.
        var attempt: Attempt? {
            switch phase {
            case .codeShown(let a), .verifying(let a), .interrupted(let a, _, _): return a
            default: return nil
            }
        }

        private var rest: Phase { isAdvertising ? .advertising : .off }

        @discardableResult
        mutating func apply(_ input: Input, now: Date = Date()) -> Outcome {
            switch input {
            case .advertising(let on):
                isAdvertising = on
                switch phase {
                case .off where on: phase = .advertising
                case .advertising where !on: phase = .off
                case .codeShown(let a) where !on, .verifying(let a) where !on:
                    // The transport drops the code with the listener; keep the
                    // attempt so the user can resume once advertising again.
                    phase = .interrupted(a, .transportStopped, detail: "advertising was turned off")
                    return Outcome(effects: [.announce("Pairing interrupted: advertising was turned off.")])
                case .paired where !on, .receiving where !on: phase = .off
                default: break
                }
                return .none

            case .codeIssued(let a):
                guard a.generation > generation else { return .staleInput }
                generation = a.generation
                isAdvertising = true
                phase = .codeShown(a)
                return Outcome(effects: [.persist(a), .announce("Pairing code \(Pairing.spokenCode(a.code)), valid for \(Int(a.remaining(at: now).rounded(.up))) seconds.")])

            case .restored(let a):
                guard a.generation >= generation else { return .staleInput }
                switch phase {
                case .off, .advertising:
                    generation = a.generation
                    let expired = a.isExpired(at: now)
                    phase = .interrupted(a, .relaunch, detail: expired
                        ? "the code expired while FlashTeX was not running"
                        : "FlashTeX was quit while the code was valid")
                    return Outcome(effects: [.announce(expired
                        ? "A pairing from a previous launch expired. Dismiss it or show a new code."
                        : "A pairing from a previous launch is waiting. Resume it or cancel.")])
                default:
                    return .ignoredInput
                }

            case .bootstrapSessionOpened(let g):
                guard g >= generation else { return .staleInput }
                guard case .codeShown(let a) = phase, a.generation == g else { return .ignoredInput }
                phase = .verifying(a)
                return Outcome(effects: [.announce("A companion connected; verifying the code.")])

            case .confirmed(let pairId, let name, let g):
                guard g >= generation else { return .staleInput }
                switch phase {
                case .codeShown(let a), .verifying(let a), .interrupted(let a, _, _):
                    guard a.generation == g, a.pairId == pairId else { return .staleInput }
                    phase = .paired(Paired(pairId: pairId, companionName: name, generation: g))
                    return Outcome(effects: [.clearJournal, .announce("Paired with \(name).")])
                case .paired(let p) where p.pairId == pairId:
                    return .ignoredInput // idempotent
                default:
                    return .ignoredInput
                }

            case .codeExpired(let g, let replacement):
                guard g >= generation else { return .staleInput }
                switch phase {
                case .codeShown(let a) where a.canRoll && replacement != nil:
                    // Rolling code: the sheet is open and nobody connected, so
                    // the expired code is replaced in place rather than failing
                    // the attempt (#355: a paste that lands a second late).
                    guard a.generation == g else { return .staleInput }
                    guard var next = replacement, next.generation > a.generation, next.code != a.code else { return .ignoredInput }
                    next.rolls = a.rolls + 1
                    generation = next.generation
                    phase = .codeShown(next)
                    return Outcome(effects: [.persist(next), .resumeTransport(next),
                                             .announce("The pairing code expired unused. New pairing code \(Pairing.spokenCode(next.code)), valid for \(Int(next.remaining(at: now).rounded(.up))) seconds.")])
                case .codeShown(let a), .verifying(let a):
                    guard a.generation == g else { return .staleInput }
                    phase = .failed(reason: "The pairing code expired before a companion paired.", generation: g)
                    return Outcome(effects: [.clearJournal, .announce("The pairing code expired. Show a new code to try again.")])
                case .interrupted(let a, _, _):
                    guard a.generation == g else { return .staleInput }
                    phase = .failed(reason: "The pairing code expired before it could be resumed.", generation: g)
                    return Outcome(effects: [.clearJournal, .announce("The interrupted pairing code expired.")])
                default:
                    return .ignoredInput
                }

            case .peerGone(let pairId, let reason, let g):
                guard g >= generation else { return .staleInput }
                switch phase {
                case .verifying(let a):
                    guard a.generation == g, pairId == nil || pairId == a.pairId else { return .staleInput }
                    phase = .interrupted(a, .peerGone, detail: reason)
                    return Outcome(effects: [.announce("The companion disconnected before pairing finished: \(reason). Resume or cancel.")])
                case .receiving(let r):
                    guard pairId == r.pairId else { return .staleInput }
                    phase = .failed(reason: "Connection to \(r.companionName ?? r.pairId) closed while receiving: \(reason)", generation: g)
                    return Outcome(effects: [.announce("Capture interrupted: \(reason).")])
                case .paired(let p):
                    guard pairId == p.pairId else { return .staleInput }
                    phase = rest
                    return .none
                default:
                    return .ignoredInput
                }

            case .listenerFailed(let reason):
                isAdvertising = false
                switch phase {
                case .codeShown(let a), .verifying(let a):
                    phase = .interrupted(a, .transportStopped, detail: reason)
                    return Outcome(effects: [.announce("Pairing interrupted: \(reason). Resume or cancel.")])
                case .interrupted:
                    return .ignoredInput
                default:
                    phase = .failed(reason: reason, generation: generation)
                    return Outcome(effects: [.announce("Nearby error: \(reason)")])
                }

            case .withdrawn(let g, let reason):
                guard g >= generation else { return .staleInput }
                switch phase {
                case .codeShown(let a), .verifying(let a):
                    guard a.generation == g else { return .staleInput }
                    phase = .interrupted(a, .transportStopped, detail: reason)
                    return Outcome(effects: [.announce("Pairing interrupted: \(reason). Resume or cancel.")])
                default:
                    return .ignoredInput
                }

            case .otherCompanionConnected(let g):
                guard g >= generation else { return .staleInput }
                guard case .verifying(let a) = phase, a.generation == g else { return .ignoredInput }
                phase = .codeShown(a)
                return .none

            case .receiving(let pairId, let name, let captureId, let bytes, let total):
                switch phase {
                case .advertising, .paired, .receiving, .failed:
                    phase = .receiving(Receiving(pairId: pairId, companionName: name, captureId: captureId, bytes: bytes, total: total))
                    return .none
                default:
                    return .ignoredInput
                }

            case .captureReceived(let pairId, let captureId):
                switch phase {
                case .receiving(let r):
                    guard pairId == nil || pairId == r.pairId else { return .staleInput }
                    phase = rest
                    return Outcome(effects: [.announce("Received capture \(captureId) from \(r.companionName ?? r.pairId).")])
                case .paired, .advertising:
                    return Outcome(effects: [.announce("Received capture \(captureId).")])
                default:
                    return .ignoredInput
                }

            case .captureRefused(let pairId, let captureId, let code):
                switch phase {
                case .receiving(let r):
                    guard pairId == nil || pairId == r.pairId else { return .staleInput }
                    phase = rest
                    return Outcome(effects: [.announce("Capture \(captureId ?? "?") from \(r.companionName ?? r.pairId) refused: \(code).")])
                case .paired, .advertising:
                    return Outcome(effects: [.announce("Capture \(captureId ?? "?") refused: \(code).")])
                default:
                    return .ignoredInput
                }

            case .forgotten(let pairId):
                switch phase {
                case .paired(let p) where p.pairId == pairId:
                    phase = rest
                    return Outcome(effects: [.announce("Forgot pairing \(pairId).")])
                case .receiving(let r) where r.pairId == pairId:
                    phase = rest
                    return Outcome(effects: [.announce("Forgot pairing \(pairId).")])
                default:
                    return .ignoredInput
                }

            case .companionConnected(_, let name):
                // Reconnects never change the pairing phase; they are announced
                // so a keyboard/VoiceOver user knows the companion is back.
                return Outcome(effects: [.announce("\(name) reconnected.")])

            case .companionDisconnected(let pairId, let name, let reason):
                switch phase {
                case .receiving(let r) where r.pairId == pairId:
                    return .ignoredInput // `.peerGone` already reported the failed receive
                case .failed:
                    return .ignoredInput
                default:
                    return Outcome(effects: [.announce("\(name) disconnected: \(reason).")])
                }

            case .cancel:
                switch phase {
                case .codeShown(let a), .verifying(let a):
                    phase = rest
                    return Outcome(effects: [.cancelTransport(a), .clearJournal, .announce("Pairing cancelled.")])
                case .interrupted(let a, let why, _):
                    phase = rest
                    // After a relaunch the transport never held this key; nothing to cancel there.
                    let effects: [Effect] = why == .relaunch ? [.clearJournal] : [.cancelTransport(a), .clearJournal]
                    return Outcome(effects: effects + [.announce("Pairing cancelled.")])
                case .failed:
                    phase = rest
                    return .none
                case .receiving(let r):
                    // The Mac closes the session; the companion may resend with the same capture_id.
                    phase = .failed(reason: "Receive from \(r.companionName ?? r.pairId) cancelled after \(r.bytes) bytes; the companion can resend it with the same capture_id.",
                                    generation: generation)
                    return Outcome(effects: [.closeSession(pairId: r.pairId), .announce("Receive cancelled.")])
                default:
                    return .ignoredInput
                }

            case .resume:
                guard case .interrupted(let a, let why, _) = phase else { return .ignoredInput }
                if a.isExpired(at: now) {
                    phase = .failed(reason: "The pairing code expired before it could be resumed.", generation: a.generation)
                    return Outcome(effects: [.clearJournal, .announce("The pairing code expired; show a new code.")])
                }
                phase = .codeShown(a)
                isAdvertising = true
                // The transport still holds the key after a peer dropped; it must
                // be told again after a relaunch or a listener failure.
                let effects: [Effect] = why == .peerGone ? [] : [.resumeTransport(a)]
                return Outcome(effects: effects + [.announce("Resumed pairing code \(Pairing.spokenCode(a.code)), \(Int(a.remaining(at: now).rounded(.up))) seconds left.")])

            case .dismiss:
                switch phase {
                case .failed, .paired:
                    phase = rest
                    return .none
                case .interrupted(let a, _, _) where a.isExpired(at: now):
                    phase = rest
                    return Outcome(effects: [.clearJournal])
                default:
                    return .ignoredInput
                }
            }
        }
    }
}

extension PairingFlow.Attempt {
    /// Journals written before rolling codes carry no `rolls`; they decode as 0
    /// (in an extension so the memberwise initialiser keeps its default).
    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        generation = try c.decode(Int.self, forKey: .generation)
        code = try c.decode(String.self, forKey: .code)
        pairId = try c.decode(String.self, forKey: .pairId)
        startedAt = try c.decode(Date.self, forKey: .startedAt)
        expiresAt = try c.decode(Date.self, forKey: .expiresAt)
        rolls = try c.decodeIfPresent(Int.self, forKey: .rolls) ?? 0
    }
}

// MARK: - user-visible text and accessibility

extension PairingFlow.Phase {
    /// Which user actions apply. `cancel` ends an attempt; `dismiss` clears a
    /// terminal banner; `resume` re-serves an interrupted code.
    func canCancel(now: Date = Date()) -> Bool {
        switch self {
        case .codeShown, .verifying, .receiving: return true
        case .interrupted(let a, _, _): return !a.isExpired(at: now)
        default: return false
        }
    }
    func canResume(now: Date = Date()) -> Bool {
        if case .interrupted(let a, _, _) = self { return !a.isExpired(at: now) }
        return false
    }
    func canDismiss(now: Date = Date()) -> Bool {
        switch self {
        case .failed, .paired: return true
        case .interrupted(let a, _, _): return a.isExpired(at: now)
        default: return false
        }
    }
    func canShowCode(now: Date = Date()) -> Bool {
        switch self {
        case .off, .advertising, .paired, .failed: return true
        case .interrupted(let a, _, _): return a.isExpired(at: now)
        default: return false
        }
    }

    /// Short state name (also the accessibility label of the status element).
    var title: String {
        switch self {
        case .off: return "Off"
        case .advertising: return "Advertising"
        case .codeShown: return "Pairing code shown"
        case .verifying: return "Verifying companion"
        case .paired: return "Paired"
        case .receiving: return "Receiving capture"
        case .interrupted: return "Pairing interrupted"
        case .failed: return "Error"
        }
    }

    /// One precise sentence for the current state, evaluated at `now`.
    func detail(now: Date = Date()) -> String {
        switch self {
        case .off: return "Not advertising. Turn on Advertise or show a pairing code."
        case .advertising: return "Paired companions can connect. No pairing in progress."
        case .codeShown(let a):
            let lead = a.rolls > 0 ? "The previous code expired unused; enter the new code on the companion." : "Enter the code on the companion."
            return "\(lead) Expires in \(Int(a.remaining(at: now).rounded(.up))) s."
        case .verifying(let a):
            return "A companion connected with the code; waiting for its hello. Expires in \(Int(a.remaining(at: now).rounded(.up))) s."
        case .paired(let p): return "Paired with \(p.companionName) (\(p.pairId))."
        case .receiving(let r):
            let who = r.companionName ?? r.pairId
            if let total = r.total { return "Receiving \(r.bytes) of \(total) bytes from \(who)." }
            return "Receiving \(r.bytes) bytes from \(who); total unknown until the line ends."
        case .interrupted(let a, let why, let detail):
            let base: String
            switch why {
            case .relaunch: base = "Interrupted by a relaunch"
            case .peerGone: base = "The companion disconnected"
            case .transportStopped: base = "The listener stopped serving the code"
            }
            if a.isExpired(at: now) { return "\(base) (\(detail)); the code has expired. Dismiss it or show a new code." }
            return "\(base) (\(detail)). Code \(Pairing.displayCode(a.code)) is still valid for \(Int(a.remaining(at: now).rounded(.up))) s: resume or cancel."
        case .failed(let reason, _): return reason
        }
    }

    /// Accessibility value for the status element: the detail without the
    /// countdown churn, so VoiceOver does not re-announce every second.
    func accessibilityValue(now: Date = Date()) -> String {
        switch self {
        case .codeShown(let a): return "Code \(Pairing.spokenCode(a.code)). Enter it on the companion, or scan the QR code."
        case .verifying: return "A companion connected; waiting for its hello."
        case .interrupted(let a, _, let detail):
            return a.isExpired(at: now) ? "\(detail). The code has expired." : "\(detail). Code \(Pairing.spokenCode(a.code)) is still valid."
        default: return detail(now: now)
        }
    }
}

extension PairingFlow {
    /// Where a phase sits in the pairing sequence, for the window's step
    /// indicator. Pure: derived from the phase only, so what the indicator
    /// shows is exactly the listener state the machine holds.
    struct Step: Equatable {
        enum Status: String, Equatable { case pending, active, done, interrupted }
        static let names = ["Show a code", "Companion enters the code", "Verify the companion", "Paired"]
        /// Capsule captions at the window's minimum width; `names` are spoken.
        static let shortNames = ["Show code", "Enter code", "Verify", "Paired"]
        static var count: Int { names.count }
        /// 1-based index of the step the phase is at.
        var index: Int
        var status: Status

        var name: String { Self.names[index - 1] }

        /// One label for the whole indicator, e.g. "Pairing step 2 of 4:
        /// Companion enters the code, in progress. Done: Show a code."
        var accessibilityLabel: String {
            let word: String
            switch status {
            case .pending: word = "not started"
            case .active: word = "in progress"
            case .done: word = "done"
            case .interrupted: word = "interrupted"
            }
            var text = "Pairing step \(index) of \(Self.count): \(name), \(word)."
            let done = Self.names.prefix(status == .done ? index : index - 1)
            if !done.isEmpty { text += " Done: \(done.joined(separator: ", "))." }
            return text
        }

        /// Status of step `k` (1-based) relative to this step.
        func status(of k: Int) -> Status {
            if k < index { return .done }
            if k > index { return .pending }
            return status
        }
    }
}

extension PairingFlow.Phase {
    /// nil while nothing about a pairing is shown (receiving, or an error
    /// banner, whose reason the status row already names).
    var step: PairingFlow.Step? {
        switch self {
        case .off, .advertising: return .init(index: 1, status: .pending)
        case .codeShown: return .init(index: 2, status: .active)
        case .verifying: return .init(index: 3, status: .active)
        case .paired: return .init(index: 4, status: .done)
        case .interrupted(_, let why, _):
            // A peer that opened the bootstrap session got to verification;
            // a relaunch or a stopped transport interrupted the code itself.
            return .init(index: why == .peerGone ? 3 : 2, status: .interrupted)
        case .receiving, .failed: return nil
        }
    }
}

/// Accessibility text for the window's rows (pure, testable).
enum PairingAccessibility {
    static func deviceRow(_ r: PairRecord, connected: Bool) -> (label: String, value: String) {
        var value = connected ? "Connected." : "Not connected."
        value += " Permission: \(r.effectivePermission.spoken)."
        value += " Paired \(r.createdAt.formatted(date: .abbreviated, time: .shortened))."
        if let seen = r.lastSeenAt { value += " Last seen \(seen.formatted(date: .abbreviated, time: .shortened))." }
        return ("Companion \(r.companionName), pair id \(r.pairId)", value)
    }

    struct CaptureRow: Equatable, Identifiable {
        enum Source: String { case inbox, bridge }
        var captureId: String
        var source: Source
        var state: String
        /// nil: durability is whatever the bridge acknowledged (see its note).
        var durable: Bool?
        var note: String
        var id: String { "\(source.rawValue):\(captureId)" }
        var durabilityText: String {
            switch durable {
            case .some(true): return "durable"
            case .some(false): return "not durable (in memory only)"
            case .none: return "via bridge"
            }
        }
        var accessibilityLabel: String { "Capture \(captureId)" }
        var accessibilityValue: String { "\(state), \(durabilityText)" + (note.isEmpty ? "" : ". \(note)") }
    }
}

// MARK: - pairing journal (durable pending attempt)

/// `~/Library/Application Support/FlashTeX/pairing-session.json` (mode 0600):
/// the attempt generation counter and the one pending attempt, so a code that
/// was valid when the app quit can be resumed or explicitly cancelled after
/// relaunch. The code is a bootstrap secret (≤ 120 s, one pairing); it is
/// stored only while pending and never together with a long-term PSK. A code
/// rolled in place (`Pairing.maxCodeRolls`) replaces the pending attempt with
/// its `rolls` count, so the cap survives a relaunch too.
final class PairingJournal {
    static let schemaVersion = 1

    struct File: Codable, Equatable {
        var version: Int
        var generation: Int
        var pending: PairingFlow.Attempt?
    }

    let url: URL
    private let lock = NSLock()
    private var file: File
    private(set) var loadError: String?

    /// Next to the pair store; `FLASHTEX_PAIRING_JOURNAL=<path>` overrides.
    static func defaultURL() -> URL {
        if let p = ProcessInfo.processInfo.environment["FLASHTEX_PAIRING_JOURNAL"], !p.isEmpty { return URL(fileURLWithPath: p) }
        return PairStore.defaultURL().deletingLastPathComponent().appendingPathComponent("pairing-session.json")
    }

    init(url: URL) {
        self.url = url
        let dec = JSONDecoder()
        dec.dateDecodingStrategy = .iso8601
        if let data = try? Data(contentsOf: url) {
            if let f = try? dec.decode(File.self, from: data), f.version == Self.schemaVersion {
                file = f
            } else {
                file = File(version: Self.schemaVersion, generation: 0, pending: nil)
                loadError = "\(url.path) is not a version-\(Self.schemaVersion) pairing journal; starting from generation 0 without overwriting it"
            }
        } else {
            file = File(version: Self.schemaVersion, generation: 0, pending: nil)
        }
    }

    var generation: Int { lock.withLock { file.generation } }
    var pending: PairingFlow.Attempt? { lock.withLock { file.pending } }

    /// Reserves the next attempt generation (persisted before use, so a crash
    /// between reserve and show still leaves older events stale).
    func nextGeneration() -> Int {
        lock.withLock {
            file.generation += 1
            _ = try? persist()
            return file.generation
        }
    }

    @discardableResult
    func setPending(_ attempt: PairingFlow.Attempt?) -> Bool {
        lock.withLock {
            guard loadError == nil else { return false }
            file.pending = attempt
            if let a = attempt, a.generation > file.generation { file.generation = a.generation }
            return (try? persist()) != nil
        }
    }

    private func persist() throws {
        let fm = FileManager.default
        try fm.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true,
                               attributes: [.posixPermissions: 0o700])
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        enc.dateEncodingStrategy = .iso8601
        try enc.encode(file).write(to: url, options: [.atomic])
        try fm.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
    }
}
