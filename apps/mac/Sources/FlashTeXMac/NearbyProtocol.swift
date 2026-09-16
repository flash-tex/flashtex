import Compression
import CryptoKit
import Foundation
import FlashTeXProtocol

/// Messages of the proposed nearby companion transport (`docs/nearby-v1-proposal.md`).
/// Everything travels as runtime-v1 envelopes over an authenticated TLS-PSK
/// stream, one JSON object per line. `capture_submit` keeps its runtime-v1
/// payload unchanged; the types here are the nearby-only additions.
enum NearbyV1 {
    static let version = 1
    static let serviceType = "_flashtex._tcp"
    /// Same bound as the bridge (transfer-v1): a line including its newline.
    static let maxLineBytes = 12 * 1024 * 1024
    static let maxIDBytes = 128

    /// First line on every connection. `protocol_version` is the nearby
    /// protocol version (1), distinct from the envelope's runtime version.
    /// `proof` = `Pairing.helloProof(psk:nonce:)` binds the claimed `pair_id`
    /// to the PSK that opened the TLS session.
    struct Hello: Codable, Equatable {
        var pairId: String
        var companionName: String
        var protocolVersion: Int
        var nonce: String
        var proof: String
        enum CodingKeys: String, CodingKey {
            case pairId = "pair_id", companionName = "companion_name"
            case protocolVersion = "protocol_version", nonce, proof
        }
        init(pairId: String, companionName: String, protocolVersion: Int = NearbyV1.version, nonce: String, proof: String) {
            self.pairId = pairId; self.companionName = companionName
            self.protocolVersion = protocolVersion; self.nonce = nonce; self.proof = proof
        }
    }

    /// The Mac's current insertion destination, so the companion never types IDs.
    struct Destination: Codable, Equatable {
        var destinationId: String
        var projectId: String
        var path: String
        var baseRevision: Int
        /// What kind of place the caret is in — text, inline/display math, a
        /// tabular cell, verbatim, a comment — and the wrapping a capture
        /// landing there needs. Additive and optional: a Mac that predates it
        /// omits the key and a companion that predates it ignores it.
        /// See protocol/proposals/transfer-v1-caret-context.md.
        var caretContext: CaretContext?
        enum CodingKeys: String, CodingKey {
            case destinationId = "destination_id", projectId = "project_id", path
            case baseRevision = "base_revision", caretContext = "caret_context"
        }
        init(destinationId: String, projectId: String, path: String, baseRevision: Int,
             caretContext: CaretContext? = nil) {
            self.destinationId = destinationId; self.projectId = projectId
            self.path = path; self.baseRevision = baseRevision
            self.caretContext = caretContext
        }
    }

    /// `destination` is always present (explicit `null` when nothing is pinned).
    /// `pair_psk` (base64, 32 bytes) appears only on a bootstrap connection: the
    /// companion must persist it and use it for every later connection.
    struct HelloAck: Codable, Equatable {
        var macName: String
        var nonce: String
        var destination: Destination?
        var pairPsk: String?
        enum CodingKeys: String, CodingKey {
            case macName = "mac_name", nonce, destination, pairPsk = "pair_psk"
        }
        init(macName: String, nonce: String, destination: Destination?, pairPsk: String?) {
            self.macName = macName; self.nonce = nonce; self.destination = destination; self.pairPsk = pairPsk
        }
        func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encode(macName, forKey: .macName)
            try c.encode(nonce, forKey: .nonce)
            try c.encode(destination, forKey: .destination) // explicit null
            try c.encodeIfPresent(pairPsk, forKey: .pairPsk)
        }
    }

    struct DestinationReply: Codable, Equatable {
        var destination: Destination?
        init(destination: Destination?) { self.destination = destination }
        func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encode(destination, forKey: .destination)
        }
        enum CodingKeys: String, CodingKey { case destination }
    }

    /// transfer-v1 acknowledgement shape. `durable` is true only when the local
    /// bridge journaled the capture; the in-memory inbox always answers false.
    struct CaptureReceived: Codable, Equatable {
        var captureId: String
        var durable: Bool
        var hasProposal: Bool
        var applied: Bool
        enum CodingKeys: String, CodingKey {
            case captureId = "capture_id", durable, hasProposal = "has_proposal", applied
        }
        init(captureId: String, durable: Bool, hasProposal: Bool, applied: Bool) {
            self.captureId = captureId; self.durable = durable; self.hasProposal = hasProposal; self.applied = applied
        }
    }

    struct ErrorPayload: Codable, Equatable {
        var code: String
        var message: String
        init(code: String, message: String) { self.code = code; self.message = message }
    }

    /// Additive since nearby-v1 §4 (proposal §6 is closed for this item): a
    /// companion asks what became of a capture it submitted on this pairing.
    /// The request is only honoured for a `capture_id` this pairing's
    /// acknowledgement memory knows (`unknown_capture` otherwise), so a
    /// companion learns nothing about another pairing's captures.
    struct CaptureStatusRequest: Codable, Equatable {
        var captureId: String
        enum CodingKeys: String, CodingKey { case captureId = "capture_id" }
        init(captureId: String) { self.captureId = captureId }
    }

    /// `capture_status_ack`. `state` is one of `CaptureStatusState`; a
    /// companion shows an unknown string verbatim (additive vocabulary).
    /// `latex` is the proposal text (read-only on the companion) once a
    /// proposal exists — `proposal_ready`, `inserted`, `rejected`; `note` is
    /// the Mac's plain-text detail; `new_revision` the revision after an
    /// insertion. `durable` mirrors the receipt (false: in-memory inbox).
    struct CaptureStatusAck: Codable, Equatable {
        var captureId: String
        var state: String
        var durable: Bool
        var latex: String?
        var note: String?
        var newRevision: Int?
        enum CodingKeys: String, CodingKey {
            case captureId = "capture_id", state, durable, latex, note, newRevision = "new_revision"
        }
        init(captureId: String, state: CaptureStatusState, durable: Bool, latex: String? = nil, note: String? = nil, newRevision: Int? = nil) {
            self.captureId = captureId; self.state = state.rawValue; self.durable = durable
            self.latex = latex; self.note = note; self.newRevision = newRevision
        }
    }

    /// Additive `capture_insert` (nearby-v1; proposal
    /// `protocol/proposals/nearby-v1-companion-insert.md`): the companion
    /// approves the proposal **it was shown** by `capture_status_ack.latex`
    /// and asks the Mac to apply it.
    ///
    /// `approved_latex_sha256` is the lowercase hex SHA-256 of the UTF-8
    /// bytes of exactly that text. It is not a transport checksum: it is what
    /// makes this an approval of a *displayed* proposal rather than a blank
    /// cheque. The Mac refuses (`proposal_changed`) when its current proposal
    /// hashes differently, so a proposal that was re-converted between the
    /// companion reading it and tapping Insert is never inserted unreviewed.
    /// transfer-v1 "Never inserted automatically" is preserved: a human still
    /// reads the LaTeX and approves it; only the *surface* moved to the iPad.
    struct CaptureInsertRequest: Codable, Equatable {
        var captureId: String
        var approvedLatexSha256: String
        enum CodingKeys: String, CodingKey {
            case captureId = "capture_id", approvedLatexSha256 = "approved_latex_sha256"
        }
        init(captureId: String, approvedLatexSha256: String) {
            self.captureId = captureId; self.approvedLatexSha256 = approvedLatexSha256
        }
    }

    /// `capture_insert_ack`. `state` is a `CaptureStatusState` — `inserted`
    /// on success, otherwise whatever the capture actually is now, so the
    /// companion's row converges without a second round trip. `note` is the
    /// Mac's plain-text detail; `new_revision` the revision after the edit.
    struct CaptureInsertAck: Codable, Equatable {
        var captureId: String
        var state: String
        var newRevision: Int?
        var note: String?
        enum CodingKeys: String, CodingKey {
            case captureId = "capture_id", state, newRevision = "new_revision", note
        }
        init(captureId: String, state: CaptureStatusState, newRevision: Int? = nil, note: String? = nil) {
            self.captureId = captureId; self.state = state.rawValue
            self.newRevision = newRevision; self.note = note
        }
    }

    /// Lowercase hex SHA-256 of a proposal's UTF-8 bytes. Both ends derive the
    /// approval token the same way; see `CaptureInsertRequest`.
    static func proposalDigest(_ latex: String) -> String {
        SHA256.hash(data: Data(latex.utf8)).map { String(format: "%02x", $0) }.joined()
    }

    enum CaptureStatusState: String {
        /// In the Mac's in-memory inbox (no bridge attached): nothing converts it yet.
        case received
        /// Journaled by the bridge; no conversion started.
        case journaled
        case converting
        /// A proposal exists (possibly already prepared for review on the Mac).
        case proposalReady = "proposal_ready"
        case inserted, rejected, failed
        /// The bridge relaunched while the capture was in flight: journaling unknown.
        case uncertain
    }

    struct Empty: Codable, Equatable { init() {} }

    // MARK: encoding helpers

    static func envelope<P: Codable>(id: String, type: String, _ payload: P) -> RuntimeV1.Envelope<P> {
        .init(protocolVersion: RuntimeV1.protocolVersion, id: id, type: type, payload: payload)
    }

    static func line<P: Codable>(id: String, type: String, _ payload: P) -> Data {
        // Encoding our own Codable structs cannot fail in practice; an empty line
        // would be a protocol violation, so fall back to a hand-written error.
        (try? RuntimeV1.encodeLine(envelope(id: id, type: type, payload)))
            ?? Data("{\"protocol_version\":1,\"id\":null,\"type\":\"error\",\"payload\":{\"code\":\"internal\",\"message\":\"encoding failed\"}}\n".utf8)
    }

    static func errorLine(id: String?, code: String, message: String) -> Data {
        // `id` is null when the request could not be identified (transfer-v1).
        let payload = ErrorPayload(code: code, message: message)
        if let id { return line(id: id, type: "error", payload) }
        struct NullID: Encodable {
            var protocolVersion: Int, type: String, payload: ErrorPayload
            enum CodingKeys: String, CodingKey { case protocolVersion = "protocol_version", id, type, payload }
            func encode(to encoder: Encoder) throws {
                var c = encoder.container(keyedBy: CodingKeys.self)
                try c.encode(protocolVersion, forKey: .protocolVersion)
                try c.encodeNil(forKey: .id)
                try c.encode(type, forKey: .type)
                try c.encode(payload, forKey: .payload)
            }
        }
        var data = (try? JSONEncoder().encode(NullID(protocolVersion: 1, type: "error", payload: payload))) ?? Data()
        data.append(0x0A)
        return data
    }

    static func isValidID(_ id: String) -> Bool {
        !id.isEmpty && id.utf8.count <= maxIDBytes
            && id.unicodeScalars.allSatisfy { $0.isASCII && (CharacterSet.alphanumerics.contains($0) || $0 == "-" || $0 == "_") }
    }
}

// MARK: - host-side hooks

/// Receives authenticated captures. `reply` gets one encoded line: a
/// `capture_received` or an `error` envelope carrying the request's id.
protocol CaptureSink: AnyObject {
    func submit(_ envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>, reply: @escaping (Data) -> Void)
    /// nearby-v1 `capture_status` (additive). `reply` gets one encoded line:
    /// `capture_status_ack` or an `error` carrying the request's id. The
    /// session has already checked that this pairing submitted `capture_id`
    /// and that its receipt was sent.
    func captureStatus(_ envelope: RuntimeV1.Envelope<NearbyV1.CaptureStatusRequest>, reply: @escaping (Data) -> Void)
    /// nearby-v1 `capture_insert` (additive). `reply` gets one encoded line:
    /// `capture_insert_ack` or an `error` carrying the request's id. As for
    /// `captureStatus`, the session has already checked this pairing owns
    /// `capture_id`; the sink still checks that the approved digest matches
    /// the proposal it holds before anything reaches the document.
    func captureInsert(_ envelope: RuntimeV1.Envelope<NearbyV1.CaptureInsertRequest>, reply: @escaping (Data) -> Void)
}

extension CaptureSink {
    func captureStatus(_ envelope: RuntimeV1.Envelope<NearbyV1.CaptureStatusRequest>, reply: @escaping (Data) -> Void) {
        reply(NearbyV1.errorLine(id: envelope.id, code: "unavailable", message: "capture_status is not answered by this sink"))
    }

    func captureInsert(_ envelope: RuntimeV1.Envelope<NearbyV1.CaptureInsertRequest>, reply: @escaping (Data) -> Void) {
        reply(NearbyV1.errorLine(id: envelope.id, code: "unavailable", message: "capture_insert is not answered by this sink"))
    }
}

/// Answers `hello`/`destination_query` with the Mac's current anchor.
protocol DestinationProvider: AnyObject {
    func currentDestination(_ reply: @escaping (NearbyV1.Destination?) -> Void)
}

/// Turns a bootstrap (code-derived) connection into a long-term pairing.
/// Returns the freshly minted 32-byte PSK the companion must switch to.
/// `generation` is the pairing attempt the bootstrap key was issued for
/// (`PSKEntry.generation`); a confirmer refuses any other attempt, so a
/// session keyed by a replaced code cannot confirm the current one.
protocol PairingConfirmer: AnyObject {
    func confirmPairing(pairId: String, companionName: String, generation: Int?) -> Data?
    func notePairSeen(pairId: String)
    /// An accepted (acknowledged) capture from a paired session. `remembered`
    /// is the bounded dedup entry (`NearbyAckMemory.Persisted`) the confirmer
    /// may persist with the pairing so a later listener answers the retry.
    func noteCapture(pairId: String, captureId: String, remembered: NearbyAckMemory.Persisted?)
    /// Per-companion permission (pairs.json v3): false refuses every
    /// `capture_submit` of the pairing with `capture_not_permitted`, session
    /// kept open. Consulted per capture so a change in the window applies
    /// without restarting the listener.
    func capturesPermitted(pairId: String) -> Bool
}

extension PairingConfirmer {
    func capturesPermitted(pairId: String) -> Bool { true }
}

// MARK: - receive limits and accounting

/// Hard caps on what one listener accepts. Every cap is answered with an
/// explicit `error` envelope (or, for unauthenticated peers, a closed
/// connection and a listener event), never a silent drop. Image caps mirror
/// transfer-v1 so the bridge and the nearby adapter refuse the same inputs.
struct NearbyReceiveLimits: Equatable {
    /// One frame (a JSON line including its newline).
    var maxLineBytes = NearbyV1.maxLineBytes
    /// Encoded PNG/JPEG bytes after base64 decoding (transfer-v1: 8 MiB).
    var maxImageBytes = 8 * 1024 * 1024
    /// Width and height, each (transfer-v1: 8192).
    var maxImageSide = 8192
    /// Pixel bytes a full decode would allocate (transfer-v1: 64 MiB).
    var maxDecodedImageBytes = 64 * 1024 * 1024
    /// Frame bytes of one session's captures accepted but not yet acknowledged.
    var maxSessionBytesInFlight = 24 * 1024 * 1024
    /// Same, summed over every session of the listener ("inbox" = accepted, undelivered).
    var maxInboxBytes = 64 * 1024 * 1024
    /// Authenticated sessions one pairing may hold at once (reconnects overlap briefly).
    var maxSessionsPerPeer = 4
    /// Accepted TCP connections, authenticated or not.
    var maxConnections = 16
    /// Per-pairing `(capture_id, base_revision, digest)` memory for duplicate
    /// detection, shared by every session of that pairing on one listener.
    var maxRememberedCaptures = 256
    /// Seconds an accepted TCP connection may take to complete the TLS
    /// handshake; a silent peer is closed (no application bytes are owed).
    var handshakeTimeout: TimeInterval = 10
    /// Seconds an authenticated connection may stay silent before `hello`.
    var helloTimeout: TimeInterval = 10
    /// Seconds one frame may take from its first byte to its newline (a
    /// slow-loris partial line is refused with `frame_timeout`). The default
    /// admits a full 12 MiB frame at 200 KiB/s.
    var frameTimeout: TimeInterval = 60
    /// TCP keepalive probes so a peer that vanished without a FIN (sleep,
    /// Wi-Fi drop) releases its session in about idle + interval × count.
    var keepaliveIdleSeconds = 15
    var keepaliveIntervalSeconds = 5
    var keepaliveCount = 4

    static let base64Overhead = 4.0 / 3.0
    /// Longest base64 string that can encode `maxImageBytes`.
    var maxImageBase64Bytes: Int { (maxImageBytes + 2) / 3 * 4 }
}

/// Listener-wide counters shared by every session (and carried across a
/// listener restart by `adoptConnections`). Thread-safe; sessions call it
/// from the listener queue and reply callbacks from anywhere.
final class NearbyReceiveBudget {
    /// One accepted frame's share of the inbox; released exactly once, at the
    /// latest when its session ends.
    final class Reservation {
        let bytes: Int
        private weak var budget: NearbyReceiveBudget?
        private var released = false
        fileprivate init(bytes: Int, budget: NearbyReceiveBudget) { self.bytes = bytes; self.budget = budget }
        func release() {
            guard !released else { return }
            released = true
            budget?.release(bytes)
        }
        deinit { release() }
    }

    private let lock = NSLock()
    private var _limits: NearbyReceiveLimits
    private var inFlight = 0
    private var sessionsByPeer: [String: Int] = [:]

    init(limits: NearbyReceiveLimits) { _limits = limits }

    var limits: NearbyReceiveLimits {
        get { lock.withLock { _limits } }
        set { lock.withLock { _limits = newValue } }
    }
    var inboxBytesInFlight: Int { lock.withLock { inFlight } }
    func sessionCount(pairId: String) -> Int { lock.withLock { sessionsByPeer[pairId] ?? 0 } }

    func reserve(_ bytes: Int) -> Reservation? {
        lock.withLock {
            guard inFlight + bytes <= _limits.maxInboxBytes else { return nil }
            inFlight += bytes
            return Reservation(bytes: bytes, budget: self)
        }
    }
    private func release(_ bytes: Int) { lock.withLock { inFlight = max(0, inFlight - bytes) } }

    func admitSession(pairId: String) -> Bool {
        lock.withLock {
            let n = sessionsByPeer[pairId] ?? 0
            guard n < _limits.maxSessionsPerPeer else { return false }
            sessionsByPeer[pairId] = n + 1
            return true
        }
    }
    func leaveSession(pairId: String) {
        lock.withLock {
            let n = (sessionsByPeer[pairId] ?? 1) - 1
            if n <= 0 { sessionsByPeer.removeValue(forKey: pairId) } else { sessionsByPeer[pairId] = n }
        }
    }
}

/// Refusals decided from framing alone, before any line is decoded and, for
/// the size cases, before the offending bytes are buffered. `code` is what
/// the companion sees in the `error` envelope; the handshake case has none
/// because an unauthenticated peer is owed no application bytes.
enum NearbyFrameError: Error, Equatable {
    /// A terminated line longer than the limit.
    case lineTooLong(bytes: Int, limit: Int)
    /// An unterminated line that cannot end inside the limit.
    case unterminatedLineTooLong(limit: Int)
    /// A frame that did not reach its newline within `frameTimeout`.
    case frameTimeout(seconds: TimeInterval, pendingBytes: Int)
    /// An authenticated connection that sent no `hello` within `helloTimeout`.
    case helloTimeout(seconds: TimeInterval)
    /// A TCP peer that did not finish TLS within `handshakeTimeout`.
    case handshakeTimeout(seconds: TimeInterval)

    var code: String? {
        switch self {
        case .lineTooLong, .unterminatedLineTooLong: return "line_too_long"
        case .frameTimeout: return "frame_timeout"
        case .helloTimeout: return "hello_timeout"
        case .handshakeTimeout: return nil
        }
    }

    var message: String {
        switch self {
        case .lineTooLong(let bytes, let limit): return "line of \(bytes) bytes exceeds \(limit) bytes"
        case .unterminatedLineTooLong(let limit): return "unterminated line exceeds \(limit) bytes"
        case .frameTimeout(let s, let pending): return "frame not completed within \(Self.seconds(s)) (\(pending) bytes pending)"
        case .helloTimeout(let s): return "no hello within \(Self.seconds(s))"
        case .handshakeTimeout(let s): return "TLS handshake not completed within \(Self.seconds(s))"
        }
    }

    /// `connectionClosed` reason.
    var reason: String {
        switch self {
        case .lineTooLong: return "line too long"
        case .unterminatedLineTooLong: return "unterminated line too long"
        case .frameTimeout(let s, _): return "frame timed out after \(Self.seconds(s))"
        case .helloTimeout(let s): return "hello timed out after \(Self.seconds(s))"
        case .handshakeTimeout(let s): return "handshake timed out after \(Self.seconds(s))"
        }
    }

    static func seconds(_ s: TimeInterval) -> String {
        s == s.rounded() ? "\(Int(s))s" : String(format: "%.2fs", s)
    }
}

/// Acknowledgement memory per pairing, shared by every session of that
/// pairing on one listener (and carried across a restart by
/// `adoptConnections`), so a companion that reconnects and re-sends a capture
/// the Mac already accepted is acknowledged again and never re-delivered.
/// A retry that arrives while the first delivery is still pending — on any
/// session — waits for that one answer. Thread-safe: sink replies may land
/// after the delivering session is gone.
final class NearbyAckMemory {
    struct Remembered {
        var baseRevision: Int
        var digest: Data
        var ack: NearbyV1.CaptureReceived?
        var waiting: [(id: String, emit: (Data) -> Void)] = []
    }
    /// One acknowledged capture as persisted with its pairing record
    /// (`PairRecord.rememberedCaptures`): enough to answer a retry with the
    /// same `capture_received` and to name a revision/payload mismatch.
    /// Never carries the image, the key, or a pending (unanswered) entry.
    struct Persisted: Codable, Equatable {
        var captureId: String
        var baseRevision: Int
        /// Hex SHA-256 of the submit payload (`NearbySession.digest(of:)`).
        var digestHex: String
        var ack: NearbyV1.CaptureReceived
        enum CodingKeys: String, CodingKey {
            case captureId = "capture_id", baseRevision = "base_revision", digestHex = "digest_sha256", ack
        }
    }

    private let lock = NSLock()
    private var byPair: [String: [String: Remembered]] = [:]
    private var order: [String: [String]] = [:]
    private var maxPerPair: Int

    init(maxPerPair: Int) { self.maxPerPair = maxPerPair }

    /// Acknowledged entries of one pairing, oldest first (pending ones are
    /// not persistable: their answer is still owed by the sink).
    func snapshot(pairId: String) -> [Persisted] {
        lock.withLock {
            (order[pairId] ?? []).compactMap { id in
                guard let r = byPair[pairId]?[id], let ack = r.ack else { return nil }
                return Persisted(captureId: id, baseRevision: r.baseRevision, digestHex: Pairing.hex(r.digest), ack: ack)
            }
        }
    }

    /// Seeds a pairing's memory from its persisted entries. An entry already
    /// known live (pending or acknowledged) is left alone; the per-pair bound
    /// still applies, oldest acknowledged first.
    func restore(pairId: String, entries: [Persisted]) {
        lock.withLock {
            for e in entries where byPair[pairId]?[e.captureId] == nil {
                guard let digest = Pairing.data(hex: e.digestHex) else { continue }
                byPair[pairId, default: [:]][e.captureId] = Remembered(baseRevision: e.baseRevision, digest: digest, ack: e.ack)
                order[pairId, default: []].append(e.captureId)
            }
            evictLocked(pairId: pairId)
        }
    }

    /// Caller holds `lock`: drops the oldest acknowledged entries beyond the bound.
    private func evictLocked(pairId: String) {
        var i = 0
        while (byPair[pairId]?.count ?? 0) > maxPerPair, i < (order[pairId]?.count ?? 0) {
            let old = order[pairId]![i]
            if byPair[pairId]?[old]?.ack != nil {
                byPair[pairId]?.removeValue(forKey: old)
                order[pairId]?.remove(at: i)
            } else { i += 1 }
        }
    }

    var limit: Int {
        get { lock.withLock { maxPerPair } }
        set { lock.withLock { maxPerPair = newValue } }
    }

    func lookup(pairId: String, captureId: String) -> Remembered? {
        lock.withLock { byPair[pairId]?[captureId] }
    }

    func count(pairId: String) -> Int { lock.withLock { byPair[pairId]?.count ?? 0 } }
    var pairIds: [String] { lock.withLock { Array(byPair.keys) } }

    /// Records a capture as pending; evicts the oldest acknowledged entries
    /// beyond the per-pair bound (pending ones are never evicted).
    func remember(pairId: String, captureId: String, baseRevision: Int, digest: Data) {
        lock.withLock {
            byPair[pairId, default: [:]][captureId] = Remembered(baseRevision: baseRevision, digest: digest, ack: nil)
            order[pairId, default: []].append(captureId)
            evictLocked(pairId: pairId)
        }
    }

    /// Queues a retry behind a pending delivery. False when nothing is pending.
    func wait(pairId: String, captureId: String, id: String, emit: @escaping (Data) -> Void) -> Bool {
        lock.withLock {
            guard byPair[pairId]?[captureId] != nil, byPair[pairId]?[captureId]?.ack == nil else { return false }
            byPair[pairId]?[captureId]?.waiting.append((id, emit))
            return true
        }
    }

    /// Stores the sink's acknowledgement and hands back every queued retry.
    func acknowledge(pairId: String, captureId: String, ack: NearbyV1.CaptureReceived) -> [(id: String, emit: (Data) -> Void)] {
        lock.withLock {
            guard byPair[pairId]?[captureId] != nil else { return [] }
            let w = byPair[pairId]?[captureId]?.waiting ?? []
            byPair[pairId]?[captureId]?.ack = ack
            byPair[pairId]?[captureId]?.waiting = []
            return w
        }
    }

    /// Drops a refused capture (so a retry is delivered again) and hands back
    /// every queued retry so it can be refused too.
    func forget(pairId: String, captureId: String) -> [(id: String, emit: (Data) -> Void)] {
        lock.withLock {
            let w = byPair[pairId]?[captureId]?.waiting ?? []
            byPair[pairId]?.removeValue(forKey: captureId)
            if let i = order[pairId]?.firstIndex(of: captureId) { order[pairId]?.remove(at: i) }
            return w
        }
    }

    /// A forgotten pairing takes its memory with it.
    func forget(pairId: String) {
        lock.withLock { byPair.removeValue(forKey: pairId); order.removeValue(forKey: pairId) }
    }
}

// MARK: - image validation

/// Structural validation of a capture image before it reaches the main
/// thread. Deterministic and bounded: PNG chunks are walked with CRC checks
/// and the IDAT stream is inflated (streaming, fixed scratch buffer) so a
/// truncated or corrupt pixel stream is refused; JPEG markers are walked to
/// the frame header and EOI so a truncated file is refused. Neither step
/// allocates the decoded image. ImageIO is deliberately not used: it accepts
/// truncated PNG/JPEG data and only reports the problem while drawing.
enum NearbyImageCheck {
    struct Info: Equatable {
        var format: String
        var width: Int
        var height: Int
        var encodedBytes: Int
        var decodedBytes: Int
    }

    enum Failure: Error, Equatable {
        case tooLarge(String)
        case invalid(String)
        /// Same codes the bridge uses (transfer-v1).
        var code: String {
            switch self { case .tooLarge: return "image_too_large"; case .invalid: return "invalid_image" }
        }
        var message: String {
            switch self { case .tooLarge(let m), .invalid(let m): return m }
        }
        var payload: NearbyV1.ErrorPayload { .init(code: code, message: message) }
    }

    static func validate(base64: String, mimeType: String, limits: NearbyReceiveLimits) -> Result<Info, Failure> {
        guard base64.utf8.count <= limits.maxImageBase64Bytes else {
            return .failure(.tooLarge("encoded image exceeds \(limits.maxImageBytes) bytes"))
        }
        guard let bytes = Data(base64Encoded: base64) else {
            return .failure(.invalid("data_base64 is not valid base64"))
        }
        return validate(bytes: bytes, mimeType: mimeType, limits: limits)
    }

    static func validate(bytes: Data, mimeType: String, limits: NearbyReceiveLimits) -> Result<Info, Failure> {
        guard !bytes.isEmpty else { return .failure(.invalid("image is empty")) }
        guard bytes.count <= limits.maxImageBytes else {
            return .failure(.tooLarge("image is \(bytes.count) bytes, limit \(limits.maxImageBytes)"))
        }
        do {
            let info: Info
            switch mimeType {
            case "image/png": info = try png(bytes)
            case "image/jpeg": info = try jpeg(bytes)
            default: return .failure(.invalid("mime_type \(mimeType) is not accepted"))
            }
            guard info.width >= 1, info.height >= 1, info.width <= limits.maxImageSide, info.height <= limits.maxImageSide else {
                return .failure(.invalid("\(info.width)×\(info.height) exceeds \(limits.maxImageSide)×\(limits.maxImageSide)"))
            }
            guard info.decodedBytes <= limits.maxDecodedImageBytes else {
                return .failure(.invalid("decoded image would need \(info.decodedBytes) bytes, limit \(limits.maxDecodedImageBytes)"))
            }
            return .success(info)
        } catch let f as Failure {
            return .failure(f)
        } catch {
            return .failure(.invalid("\(error)"))
        }
    }

    // MARK: PNG

    private static let pngSignature: [UInt8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]

    static func png(_ d: Data) throws -> Info {
        let bytes = [UInt8](d)
        guard bytes.count >= 8 + 12, Array(bytes[0..<8]) == pngSignature else { throw Failure.invalid("not a PNG (signature)") }
        var i = 8
        var width = 0, height = 0, bitDepth = 0, colorType = 0, interlace = 0
        var sawIHDR = false, sawPLTE = false, sawIEND = false, idatDone = false
        var idat = Data()
        while i + 12 <= bytes.count {
            let length = Int(be32(bytes, i))
            guard length <= 0x7FFF_FFFF, i + 12 + length <= bytes.count else { throw Failure.invalid("PNG chunk at \(i) runs past the end") }
            let type = String(decoding: bytes[(i + 4)..<(i + 8)], as: UTF8.self)
            let dataStart = i + 8, dataEnd = dataStart + length
            let crc = be32(bytes, dataEnd)
            guard crc == crc32(bytes, (i + 4)..<dataEnd) else { throw Failure.invalid("PNG chunk \(type) has a bad CRC") }
            if !sawIHDR {
                guard type == "IHDR", length == 13 else { throw Failure.invalid("PNG must start with IHDR") }
                width = Int(be32(bytes, dataStart)); height = Int(be32(bytes, dataStart + 4))
                bitDepth = Int(bytes[dataStart + 8]); colorType = Int(bytes[dataStart + 9])
                let compression = bytes[dataStart + 10], filter = bytes[dataStart + 11]
                interlace = Int(bytes[dataStart + 12])
                guard width >= 1, height >= 1, width <= 0x7FFF_FFFF, height <= 0x7FFF_FFFF else { throw Failure.invalid("PNG IHDR dimensions invalid") }
                guard compression == 0, filter == 0, interlace <= 1 else { throw Failure.invalid("PNG IHDR method fields invalid") }
                guard pngChannels(colorType: colorType, bitDepth: bitDepth) != nil else { throw Failure.invalid("PNG colour type \(colorType)/depth \(bitDepth) invalid") }
                sawIHDR = true
            } else {
                switch type {
                case "IHDR": throw Failure.invalid("PNG has a second IHDR")
                case "PLTE": sawPLTE = true
                case "IDAT":
                    guard !idatDone else { throw Failure.invalid("PNG IDAT chunks are not consecutive") }
                    idat.append(contentsOf: bytes[dataStart..<dataEnd])
                case "IEND":
                    guard length == 0 else { throw Failure.invalid("PNG IEND must be empty") }
                    sawIEND = true
                default:
                    if !idat.isEmpty { idatDone = true }
                }
            }
            i = dataEnd + 4
            if sawIEND { break }
        }
        guard sawIHDR, sawIEND else { throw Failure.invalid("PNG is truncated (no IEND)") }
        guard i == bytes.count else { throw Failure.invalid("PNG has \(bytes.count - i) trailing bytes after IEND") }
        guard !idat.isEmpty else { throw Failure.invalid("PNG has no IDAT") }
        if colorType == 3 && !sawPLTE { throw Failure.invalid("PNG palette image without PLTE") }
        let channels = pngChannels(colorType: colorType, bitDepth: bitDepth)!
        let expected = pngRawSize(width: width, height: height, channels: channels, bitDepth: bitDepth, interlaced: interlace == 1)
        // Decoded allocation as the bridge counts it: expanded 8/16-bit samples.
        let sampleBytes = max(1, bitDepth / 8)
        let decodedChannels = colorType == 3 ? 3 : channels
        let decoded = width * height * decodedChannels * sampleBytes
        try inflateZlib(idat, expectedSize: expected)
        return Info(format: "png", width: width, height: height, encodedBytes: d.count, decodedBytes: decoded)
    }

    static func pngChannels(colorType: Int, bitDepth: Int) -> Int? {
        switch (colorType, bitDepth) {
        case (0, 1), (0, 2), (0, 4), (0, 8), (0, 16): return 1
        case (2, 8), (2, 16): return 3
        case (3, 1), (3, 2), (3, 4), (3, 8): return 1
        case (4, 8), (4, 16): return 2
        case (6, 8), (6, 16): return 4
        default: return nil
        }
    }

    /// Filtered scanline bytes the IDAT stream must inflate to, exactly.
    static func pngRawSize(width: Int, height: Int, channels: Int, bitDepth: Int, interlaced: Bool) -> Int {
        let bpp = channels * bitDepth
        func rowBytes(_ w: Int) -> Int { (w * bpp + 7) / 8 }
        guard interlaced else { return height * (1 + rowBytes(width)) }
        let passes = [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)]
        var total = 0
        for (x0, y0, dx, dy) in passes {
            guard width > x0, height > y0 else { continue }
            let pw = (width - x0 + dx - 1) / dx, ph = (height - y0 + dy - 1) / dy
            total += ph * (1 + rowBytes(pw))
        }
        return total
    }

    /// Inflates a zlib stream (RFC 1950) through a 64 KiB scratch buffer,
    /// checking the header, the exact output size and the Adler-32 trailer.
    static func inflateZlib(_ z: Data, expectedSize: Int) throws {
        guard z.count >= 6 else { throw Failure.invalid("PNG IDAT stream too short") }
        let cmf = z[z.startIndex], flg = z[z.startIndex + 1]
        guard cmf & 0x0F == 8, (Int(cmf) * 256 + Int(flg)) % 31 == 0, flg & 0x20 == 0 else {
            throw Failure.invalid("PNG IDAT is not a zlib/deflate stream")
        }
        let trailer = z.suffix(4)
        let expectedAdler = trailer.reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
        let deflate = z.dropFirst(2).dropLast(4)
        var s1: UInt32 = 1, s2: UInt32 = 0
        var total = 0
        let bufSize = 64 * 1024
        let buf = UnsafeMutablePointer<UInt8>.allocate(capacity: bufSize)
        defer { buf.deallocate() }
        let stream = UnsafeMutablePointer<compression_stream>.allocate(capacity: 1)
        defer { stream.deallocate() }
        guard compression_stream_init(stream, COMPRESSION_STREAM_DECODE, COMPRESSION_ZLIB) == COMPRESSION_STATUS_OK else {
            throw Failure.invalid("inflate init failed")
        }
        defer { compression_stream_destroy(stream) }
        try deflate.withUnsafeBytes { (raw: UnsafeRawBufferPointer) in
            stream.pointee.src_ptr = raw.bindMemory(to: UInt8.self).baseAddress!
            stream.pointee.src_size = raw.count
            while true {
                stream.pointee.dst_ptr = buf
                stream.pointee.dst_size = bufSize
                let status = compression_stream_process(stream, Int32(COMPRESSION_STREAM_FINALIZE.rawValue))
                let produced = bufSize - stream.pointee.dst_size
                total += produced
                guard total <= expectedSize else { throw Failure.invalid("PNG pixel stream is longer than its dimensions allow") }
                for k in 0..<produced {
                    s1 = (s1 + UInt32(buf[k])) % 65521
                    s2 = (s2 + s1) % 65521
                }
                switch status {
                case COMPRESSION_STATUS_END:
                    guard stream.pointee.src_size == 0 else { throw Failure.invalid("PNG IDAT has trailing data after the deflate stream") }
                    return
                case COMPRESSION_STATUS_ERROR:
                    throw Failure.invalid("PNG IDAT deflate stream is corrupt")
                default:
                    // OK with nothing produced and nothing left to read = truncated.
                    if produced == 0 && stream.pointee.src_size == 0 { throw Failure.invalid("PNG IDAT deflate stream is truncated") }
                }
            }
        }
        guard total == expectedSize else {
            throw Failure.invalid("PNG pixel stream is \(total) bytes, dimensions need \(expectedSize)")
        }
        guard (s2 << 16) | s1 == expectedAdler else { throw Failure.invalid("PNG IDAT Adler-32 mismatch") }
    }

    // MARK: JPEG

    static func jpeg(_ d: Data) throws -> Info {
        let b = [UInt8](d)
        guard b.count >= 4, b[0] == 0xFF, b[1] == 0xD8 else { throw Failure.invalid("not a JPEG (no SOI)") }
        var i = 2
        var width = 0, height = 0, components = 0
        var sawSOF = false
        let sofMarkers: Set<UInt8> = [0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF]
        while i + 1 < b.count {
            guard b[i] == 0xFF else { throw Failure.invalid("JPEG marker expected at \(i)") }
            while i + 1 < b.count, b[i + 1] == 0xFF { i += 1 } // fill bytes
            guard i + 1 < b.count else { break }
            let marker = b[i + 1]
            i += 2
            switch marker {
            case 0xD9:
                guard sawSOF else { throw Failure.invalid("JPEG ended without a frame header") }
                return Info(format: "jpeg", width: width, height: height, encodedBytes: d.count, decodedBytes: width * height * components)
            case 0xD8, 0x01, 0xD0...0xD7:
                continue // standalone markers
            case 0x00:
                throw Failure.invalid("JPEG stuffed byte outside scan data")
            default:
                guard i + 2 <= b.count else { throw Failure.invalid("JPEG segment length is truncated") }
                let length = Int(b[i]) << 8 | Int(b[i + 1])
                guard length >= 2, i + length <= b.count else { throw Failure.invalid("JPEG segment 0x\(String(marker, radix: 16)) runs past the end") }
                if sofMarkers.contains(marker) {
                    guard !sawSOF else { throw Failure.invalid("JPEG has more than one frame header") }
                    guard length >= 8 else { throw Failure.invalid("JPEG frame header too short") }
                    height = Int(b[i + 3]) << 8 | Int(b[i + 4])
                    width = Int(b[i + 5]) << 8 | Int(b[i + 6])
                    components = Int(b[i + 7])
                    guard width >= 1, height >= 1 else { throw Failure.invalid("JPEG frame has no defined dimensions (DNL unsupported)") }
                    guard (1...4).contains(components) else { throw Failure.invalid("JPEG frame has \(components) components") }
                    sawSOF = true
                }
                i += length
                if marker == 0xDA {
                    guard sawSOF else { throw Failure.invalid("JPEG scan before frame header") }
                    // Entropy-coded data: skip to the next real marker.
                    while i < b.count {
                        if b[i] != 0xFF { i += 1; continue }
                        guard i + 1 < b.count else { throw Failure.invalid("JPEG scan data truncated") }
                        let m = b[i + 1]
                        if m == 0x00 { i += 2 } else if (0xD0...0xD7).contains(m) { i += 2 } else if m == 0xFF { i += 1 } else { break }
                    }
                    guard i < b.count else { throw Failure.invalid("JPEG scan data truncated (no EOI)") }
                }
            }
        }
        throw Failure.invalid("JPEG is truncated (no EOI)")
    }

    // MARK: helpers

    private static func be32(_ b: [UInt8], _ i: Int) -> UInt32 {
        UInt32(b[i]) << 24 | UInt32(b[i + 1]) << 16 | UInt32(b[i + 2]) << 8 | UInt32(b[i + 3])
    }

    private static let crcTable: [UInt32] = (0..<256).map { n -> UInt32 in
        var c = UInt32(n)
        for _ in 0..<8 { c = (c & 1) != 0 ? 0xEDB8_8320 ^ (c >> 1) : c >> 1 }
        return c
    }

    static func crc32(_ b: [UInt8], _ range: Range<Int>) -> UInt32 {
        var c: UInt32 = 0xFFFF_FFFF
        b.withUnsafeBufferPointer { p in
            for i in range { c = crcTable[Int((c ^ UInt32(p[i])) & 0xFF)] ^ (c >> 8) }
        }
        return c ^ 0xFFFF_FFFF
    }
}

// MARK: - per-connection session

/// Message handling for one authenticated connection, independent of
/// Network.framework so it can be driven with bytes in tests. TLS has already
/// authenticated the peer with one of `keys`; this layer enforces `hello`
/// first (with its proof naming which key), ids, sizes and message types, and
/// never parses more than one envelope per line.
///
/// Captures are bounded and decoded off the caller's queue: a frame is charged
/// against the session's and the listener's in-flight budgets before its JSON
/// is decoded on `decodeQueue`, the image is validated there, and only then
/// does the sink see it (the sink hops to the main thread itself). Session
/// state is touched on the caller's queue only; `onStateQueue` re-enters it.
final class NearbySession {
    enum Disposition: Equatable { case keepOpen, closeAfterFlush(String) }
    enum Event: Equatable {
        case hello(pairId: String, companionName: String, bootstrap: Bool)
        case capture(captureId: String)
        /// A capture refused before the sink saw it (validation, budget, dedup).
        case refused(captureId: String?, code: String, message: String)
        /// A retry of an already accepted capture: acknowledged, not re-delivered.
        case duplicate(captureId: String)
        case violation(String)
    }

    let keys: [NearbyListener.PSKEntry]
    let macName: String
    let limits: NearbyReceiveLimits
    private(set) var pairId: String?
    private(set) var isBootstrap = false
    /// Listener-wide freshness check; TLS already prevents cross-session replay,
    /// this only refuses a companion re-sending the same hello nonce.
    private let acceptNonce: (String) -> Bool
    private weak var sink: CaptureSink?
    private weak var destinations: DestinationProvider?
    private weak var pairing: PairingConfirmer?
    private let events: (Event) -> Void
    private let budget: NearbyReceiveBudget?
    private let decodeQueue: DispatchQueue?
    private let onStateQueue: (@escaping () -> Void) -> Void

    private(set) var bytesInFlight = 0
    private var reservations: [ObjectIdentifier: NearbyReceiveBudget.Reservation] = [:]
    private var admittedPeer: String?
    private var ended = false

    /// Dedup memory keyed by `(pair_id, capture_id)`: the accepted revision and
    /// payload digest, the acknowledgement once the sink answered, and retries
    /// that arrived while the first delivery was still pending. Shared with
    /// the other sessions of the pairing when the listener supplies it, so a
    /// reconnect never causes a second delivery; private to this session
    /// otherwise (tests driving bytes directly).
    let memory: NearbyAckMemory

    init(keys: [NearbyListener.PSKEntry], macName: String, sink: CaptureSink?,
         destinations: DestinationProvider?, pairing: PairingConfirmer?,
         limits: NearbyReceiveLimits = .init(), budget: NearbyReceiveBudget? = nil,
         decodeQueue: DispatchQueue? = nil, memory: NearbyAckMemory? = nil,
         onStateQueue: @escaping (@escaping () -> Void) -> Void = { $0() },
         acceptNonce: @escaping (String) -> Bool = { _ in true }, events: @escaping (Event) -> Void) {
        self.acceptNonce = acceptNonce
        self.keys = keys
        self.macName = macName
        self.limits = limits
        self.budget = budget
        self.decodeQueue = decodeQueue
        self.memory = memory ?? NearbyAckMemory(maxPerPair: limits.maxRememberedCaptures)
        self.onStateQueue = onStateQueue
        self.sink = sink
        self.destinations = destinations
        self.pairing = pairing
        self.events = events
    }

    var helloCompleted: Bool { pairId != nil }
    /// Captures remembered for this session's pairing (pending or acknowledged).
    var rememberedCaptureCount: Int { pairId.map { memory.count(pairId: $0) } ?? 0 }

    /// Releases every budget share and the peer's session slot. Idempotent;
    /// the connection calls it once the stream is closed either way. The
    /// pairing's acknowledgement memory is kept: a delivery still pending in
    /// the sink completes there, and a retry on the next session is answered
    /// from it rather than delivered again.
    func end() {
        guard !ended else { return }
        ended = true
        for r in reservations.values { r.release() }
        reservations.removeAll()
        bytesInFlight = 0
        if let p = admittedPeer { budget?.leaveSession(pairId: p) }
        admittedPeer = nil
    }

    /// Handles one complete line (without its newline). `emit` may be called
    /// synchronously or later (capture acknowledgements come from the sink).
    func handle(line: Data, emit: @escaping (Data) -> Void) -> Disposition {
        let header: RuntimeV1.EnvelopeHeader
        do { header = try RuntimeV1.header(of: line) } catch {
            emit(NearbyV1.errorLine(id: nil, code: "bad_request", message: "undecodable envelope"))
            events(.violation("undecodable envelope"))
            return .closeAfterFlush("undecodable envelope")
        }
        guard header.protocolVersion == RuntimeV1.protocolVersion else {
            emit(NearbyV1.errorLine(id: header.id, code: "unsupported_version",
                                    message: "protocol_version \(header.protocolVersion) is not supported"))
            return .closeAfterFlush("unsupported protocol_version")
        }
        guard !header.id.isEmpty, header.id.utf8.count <= NearbyV1.maxIDBytes else {
            emit(NearbyV1.errorLine(id: nil, code: "bad_request", message: "id must be 1–128 bytes"))
            return .closeAfterFlush("bad id")
        }
        let id = header.id
        guard helloCompleted || header.type == "hello" else {
            emit(NearbyV1.errorLine(id: id, code: "hello_required", message: "first message must be hello"))
            events(.violation("\(header.type) before hello"))
            return .closeAfterFlush("hello required")
        }
        switch header.type {
        case "hello":
            return handleHello(line: line, id: id, emit: emit)
        case "destination_query":
            let destinations = destinations
            guard let destinations else {
                emit(NearbyV1.line(id: id, type: "destination", NearbyV1.DestinationReply(destination: nil)))
                return .keepOpen
            }
            destinations.currentDestination { dest in
                emit(NearbyV1.line(id: id, type: "destination", NearbyV1.DestinationReply(destination: dest)))
            }
            return .keepOpen
        case "capture_submit":
            handleCapture(line: line, id: id, emit: emit)
            return .keepOpen
        case "capture_status":
            handleStatus(line: line, id: id, emit: emit)
            return .keepOpen
        case "capture_insert":
            handleInsert(line: line, id: id, emit: emit)
            return .keepOpen
        default:
            emit(NearbyV1.errorLine(id: id, code: "unknown_type", message: "unknown message type \(header.type)"))
            return .keepOpen
        }
    }

    // MARK: capture_status (additive)

    /// Answers only for a capture this pairing's memory knows: a capture
    /// never accepted (or refused and forgotten) is `unknown_capture`; one
    /// whose delivery is still pending in the sink is reported `received`
    /// with a note (its receipt is owed first); otherwise the sink answers
    /// from the bridge / inbox.
    private func handleStatus(line: Data, id: String, emit: @escaping (Data) -> Void) {
        let env: RuntimeV1.Envelope<NearbyV1.CaptureStatusRequest>
        do { env = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureStatusRequest>.self, from: line) } catch {
            emit(NearbyV1.errorLine(id: id, code: "bad_request", message: "undecodable capture_status: \(error)"))
            return
        }
        let captureId = env.payload.captureId
        guard NearbyV1.isValidID(captureId) else {
            emit(NearbyV1.errorLine(id: id, code: "bad_request", message: "capture_id must be 1–128 ASCII [A-Za-z0-9_-]"))
            return
        }
        guard let pairId, let remembered = memory.lookup(pairId: pairId, captureId: captureId) else {
            emit(NearbyV1.errorLine(id: id, code: "unknown_capture", message: "capture \(captureId) was not accepted on this pairing"))
            return
        }
        guard let ack = remembered.ack else {
            emit(NearbyV1.line(id: id, type: "capture_status_ack",
                               NearbyV1.CaptureStatusAck(captureId: captureId, state: .received, durable: false,
                                                         note: "delivery to the Mac still pending; capture_received not yet sent")))
            return
        }
        guard let sink else {
            emit(NearbyV1.line(id: id, type: "capture_status_ack",
                               NearbyV1.CaptureStatusAck(captureId: captureId, state: ack.durable ? .journaled : .received,
                                                         durable: ack.durable, note: "no capture sink attached")))
            return
        }
        sink.captureStatus(env, reply: emit)
    }

    // MARK: capture_insert (additive)

    /// The companion approves the proposal it was shown and asks for it to be
    /// applied. Ownership is the same rule as `capture_status`: only a
    /// capture this pairing submitted, and only once its receipt was sent —
    /// a pairing cannot reach into another pairing's captures, and cannot
    /// approve something the Mac has not acknowledged receiving. Everything
    /// past that (is there a proposal, is it still the one you read, is the
    /// document still where it was) is the sink's to check.
    private func handleInsert(line: Data, id: String, emit: @escaping (Data) -> Void) {
        let env: RuntimeV1.Envelope<NearbyV1.CaptureInsertRequest>
        do { env = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureInsertRequest>.self, from: line) } catch {
            emit(NearbyV1.errorLine(id: id, code: "bad_request", message: "undecodable capture_insert: \(error)"))
            return
        }
        let captureId = env.payload.captureId
        guard NearbyV1.isValidID(captureId) else {
            emit(NearbyV1.errorLine(id: id, code: "bad_request", message: "capture_id must be 1–128 ASCII [A-Za-z0-9_-]"))
            return
        }
        let digest = env.payload.approvedLatexSha256
        guard digest.utf8.count == 64, digest.allSatisfy({ $0.isHexDigit && !$0.isUppercase }) else {
            emit(NearbyV1.errorLine(id: id, code: "bad_request", message: "approved_latex_sha256 must be 64 lowercase hex digits"))
            return
        }
        guard let pairId, let remembered = memory.lookup(pairId: pairId, captureId: captureId) else {
            emit(NearbyV1.errorLine(id: id, code: "unknown_capture", message: "capture \(captureId) was not accepted on this pairing"))
            return
        }
        guard remembered.ack != nil else {
            emit(NearbyV1.errorLine(id: id, code: "not_ready", message: "delivery to the Mac is still pending; capture_received not yet sent"))
            return
        }
        guard let sink else {
            emit(NearbyV1.errorLine(id: id, code: "unavailable", message: "no capture sink attached"))
            return
        }
        sink.captureInsert(env, reply: emit)
    }

    // MARK: capture path

    private func refuse(id: String, captureId: String?, code: String, message: String, emit: (Data) -> Void) {
        emit(NearbyV1.errorLine(id: id, code: code, message: message))
        events(.refused(captureId: captureId, code: code, message: message))
    }

    /// Caller's queue: budget the frame, then hand the bytes to the decoder.
    private func handleCapture(line: Data, id: String, emit: @escaping (Data) -> Void) {
        guard sink != nil else {
            refuse(id: id, captureId: nil, code: "unavailable", message: "no capture sink attached", emit: emit)
            return
        }
        let frameBytes = line.count + 1
        guard bytesInFlight + frameBytes <= limits.maxSessionBytesInFlight else {
            refuse(id: id, captureId: nil, code: "too_many_in_flight",
                   message: "session has \(bytesInFlight) bytes awaiting acknowledgement; limit \(limits.maxSessionBytesInFlight)", emit: emit)
            return
        }
        var reservation: NearbyReceiveBudget.Reservation?
        if let budget {
            guard let r = budget.reserve(frameBytes) else {
                refuse(id: id, captureId: nil, code: "inbox_full",
                       message: "listener has \(budget.inboxBytesInFlight) bytes awaiting acknowledgement; limit \(budget.limits.maxInboxBytes)", emit: emit)
                return
            }
            reservation = r
            reservations[ObjectIdentifier(r)] = r
        }
        bytesInFlight += frameBytes
        let release: () -> Void = { [weak self] in
            guard let self else { reservation?.release(); return }
            self.bytesInFlight = max(0, self.bytesInFlight - frameBytes)
            if let r = reservation { self.reservations.removeValue(forKey: ObjectIdentifier(r)); r.release() }
        }
        let work: () -> Void = { [weak self] in
            guard let self else { reservation?.release(); return }
            let decoded = self.decodeAndDigest(line: line, id: id)
            self.onStateQueue { [weak self] in
                guard let self, !self.ended else { release(); return }
                switch decoded {
                case .failure(let e):
                    release()
                    self.refuse(id: id, captureId: e.captureId, code: e.code, message: e.message, emit: emit)
                case .success(let v):
                    self.deliver(v, id: id, release: release, emit: emit)
                }
            }
        }
        offQueue(work)
    }

    private func offQueue(_ work: @escaping () -> Void) {
        if let decodeQueue { decodeQueue.async(execute: work) } else { work() }
    }

    struct Parsed {
        var envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>
        var digest: Data
    }
    struct Refusal: Error { var captureId: String?; var code: String; var message: String }

    /// Decode queue, first pass: JSON, cheap field checks and the payload
    /// digest, so a duplicate is recognised before the image is validated.
    /// Touches no session state.
    func decodeAndDigest(line: Data, id: String) -> Result<Parsed, Refusal> {
        let env: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>
        do { env = try RuntimeV1.decodeCaptureSubmit(line) } catch {
            return .failure(.init(captureId: nil, code: "bad_request", message: "undecodable capture_submit: \(error)"))
        }
        let p = env.payload
        guard NearbyV1.isValidID(p.captureId), NearbyV1.isValidID(p.destinationId) else {
            return .failure(.init(captureId: nil, code: "bad_request", message: "capture_id/destination_id must be 1–128 ASCII alphanumerics, - or _"))
        }
        guard RuntimeV1.acceptedCaptureMimeTypes.contains(p.image.mimeType) else {
            return .failure(.init(captureId: p.captureId, code: "unsupported_image", message: "mime_type \(p.image.mimeType) is not accepted"))
        }
        guard p.instructions.utf8.count <= 4096 else {
            return .failure(.init(captureId: p.captureId, code: "bad_request", message: "instructions exceed 4096 bytes"))
        }
        guard p.baseRevision >= 0 else {
            return .failure(.init(captureId: p.captureId, code: "bad_request", message: "base_revision must be non-negative"))
        }
        return .success(Parsed(envelope: env, digest: Self.digest(of: p)))
    }

    /// Decode queue, second pass (new captures only): the image itself.
    func validateImage(_ p: RuntimeV1.CaptureSubmit) -> Result<NearbyImageCheck.Info, Refusal> {
        switch NearbyImageCheck.validate(base64: p.image.dataBase64, mimeType: p.image.mimeType, limits: limits) {
        case .success(let i): return .success(i)
        case .failure(let f): return .failure(.init(captureId: p.captureId, code: f.code, message: f.message))
        }
    }

    /// Payload identity for dedup: everything but `capture_id`/`base_revision`,
    /// which are compared separately so a revision mismatch is named as such.
    static func digest(of p: RuntimeV1.CaptureSubmit) -> Data {
        var h = SHA256()
        for part in [p.destinationId, p.image.mimeType, p.image.dataBase64, p.instructions] {
            h.update(data: Data(part.utf8))
            h.update(data: Data([0]))
        }
        return Data(h.finalize())
    }

    /// State queue: dedup by (pairing, capture_id, base_revision, digest);
    /// a new capture is remembered as pending, validated off-queue, then
    /// delivered (or forgotten and refused, with any coalesced retries).
    private func deliver(_ v: Parsed, id: String, release: @escaping () -> Void, emit: @escaping (Data) -> Void) {
        let p = v.envelope.payload
        let captureId = p.captureId
        let pair = pairId ?? ""
        // Permission first, before dedup memory: a companion set to view-only
        // after an accepted capture is refused on its retry as well, and
        // nothing of a refused capture is remembered.
        if let pairing, !pair.isEmpty, !pairing.capturesPermitted(pairId: pair) {
            release()
            refuse(id: id, captureId: captureId, code: CompanionPermission.refusalCode,
                   message: CompanionPermission.refusalMessage(pairId: pair), emit: emit)
            return
        }
        if let known = memory.lookup(pairId: pair, captureId: captureId) {
            guard known.baseRevision == p.baseRevision else {
                release()
                refuse(id: id, captureId: captureId, code: "revision_mismatch",
                       message: "capture_id \(captureId) was accepted at base_revision \(known.baseRevision), not \(p.baseRevision)", emit: emit)
                return
            }
            guard known.digest == v.digest else {
                release()
                refuse(id: id, captureId: captureId, code: "capture_id_conflict",
                       message: "capture_id \(captureId) already received with a different payload", emit: emit)
                return
            }
            release()
            events(.duplicate(captureId: captureId))
            if let ack = known.ack {
                emit(NearbyV1.line(id: id, type: "capture_received", ack))
            } else if !memory.wait(pairId: pair, captureId: captureId, id: id, emit: emit) {
                // Answered (or forgotten) between the lookup and the wait.
                if let ack = memory.lookup(pairId: pair, captureId: captureId)?.ack {
                    emit(NearbyV1.line(id: id, type: "capture_received", ack))
                } else {
                    refuse(id: id, captureId: captureId, code: "unavailable", message: "capture was not acknowledged", emit: emit)
                }
            }
            return
        }
        guard let sink else {
            release()
            refuse(id: id, captureId: captureId, code: "unavailable", message: "no capture sink attached", emit: emit)
            return
        }
        memory.remember(pairId: pair, captureId: captureId, baseRevision: p.baseRevision, digest: v.digest)
        let memory = memory
        let abandon: () -> Void = {
            // Session gone before the sink saw it: nothing was delivered, so a
            // retry must be delivered afresh.
            for w in memory.forget(pairId: pair, captureId: captureId) {
                w.emit(NearbyV1.errorLine(id: w.id, code: "unavailable", message: "capture was not acknowledged"))
            }
            release()
        }
        offQueue { [weak self] in
            guard let self else { abandon(); return }
            let checked = self.validateImage(p)
            self.onStateQueue { [weak self] in
                guard let self, !self.ended else { abandon(); return }
                switch checked {
                case .failure(let e):
                    let waiting = memory.forget(pairId: pair, captureId: captureId)
                    release()
                    self.refuse(id: id, captureId: captureId, code: e.code, message: e.message, emit: emit)
                    for w in waiting { w.emit(NearbyV1.errorLine(id: w.id, code: e.code, message: e.message)) }
                case .success:
                    self.submit(v.envelope, to: sink, id: id, release: release, emit: emit)
                }
            }
        }
    }

    /// State queue: hand a validated, remembered capture to the sink and
    /// answer it (and any coalesced retries) from the sink's reply. The reply
    /// is applied to the pairing's memory even when this session is gone by
    /// then (the companion dropped mid-delivery): the retry it sends on its
    /// next session is acknowledged from memory, never delivered again.
    private func submit(_ envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>, to sink: CaptureSink, id: String,
                        release: @escaping () -> Void, emit: @escaping (Data) -> Void) {
        let captureId = envelope.payload.captureId
        let pair = pairId ?? ""
        let events = events
        let memory = memory
        let pairing = pairing
        sink.submit(envelope) { [weak self] reply in
            let finish: () -> Void = { [weak self] in
                release()
                if let header = try? RuntimeV1.header(of: reply), header.type == "capture_received",
                   let env = try? JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureReceived>.self, from: reply) {
                    let waiting = memory.acknowledge(pairId: pair, captureId: captureId, ack: env.payload)
                    if !pair.isEmpty {
                        let remembered = memory.lookup(pairId: pair, captureId: captureId).map {
                            NearbyAckMemory.Persisted(captureId: captureId, baseRevision: $0.baseRevision,
                                                      digestHex: Pairing.hex($0.digest), ack: env.payload)
                        }
                        pairing?.noteCapture(pairId: pair, captureId: captureId, remembered: remembered)
                    }
                    events(.capture(captureId: captureId))
                    emit(reply)
                    for w in waiting { w.emit(NearbyV1.line(id: w.id, type: "capture_received", env.payload)) }
                } else {
                    // Refused by the sink: forget it so a retry is delivered again.
                    let waiting = memory.forget(pairId: pair, captureId: captureId)
                    emit(reply)
                    if let err = try? JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.ErrorPayload>.self, from: reply) {
                        events(.refused(captureId: captureId, code: err.payload.code, message: err.payload.message))
                        for w in waiting { w.emit(NearbyV1.errorLine(id: w.id, code: err.payload.code, message: err.payload.message)) }
                    } else {
                        for w in waiting { w.emit(NearbyV1.errorLine(id: w.id, code: "unavailable", message: "capture was not acknowledged")) }
                    }
                }
            }
            if let self { self.onStateQueue(finish) } else { finish() }
        }
    }

    // MARK: hello

    private func handleHello(line: Data, id: String, emit: @escaping (Data) -> Void) -> Disposition {
        guard !helloCompleted else {
            emit(NearbyV1.errorLine(id: id, code: "bad_request", message: "hello already completed"))
            return .closeAfterFlush("duplicate hello")
        }
        let env: RuntimeV1.Envelope<NearbyV1.Hello>
        do { env = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.Hello>.self, from: line) } catch {
            emit(NearbyV1.errorLine(id: id, code: "bad_request", message: "undecodable hello: \(error)"))
            return .closeAfterFlush("undecodable hello")
        }
        let hello = env.payload
        guard hello.protocolVersion == NearbyV1.version else {
            emit(NearbyV1.errorLine(id: id, code: "unsupported_version", message: "nearby protocol_version \(hello.protocolVersion) is not supported"))
            return .closeAfterFlush("unsupported nearby version")
        }
        guard !hello.nonce.isEmpty, hello.nonce.utf8.count <= 128 else {
            emit(NearbyV1.errorLine(id: id, code: "bad_request", message: "nonce must be a 1–128 byte string"))
            return .closeAfterFlush("bad nonce")
        }
        // The claimed pair must hold the key that opened this session; a paired
        // device cannot speak for another pairing (or for a pending one).
        guard let entry = keys.first(where: { $0.identity == hello.pairId }),
              Pairing.verifyHelloProof(hello.proof, psk: entry.key, nonce: hello.nonce) else {
            emit(NearbyV1.errorLine(id: id, code: "pair_mismatch", message: "pair_id is unknown or proof does not match its key"))
            events(.violation("hello for \(hello.pairId) failed proof"))
            return .closeAfterFlush("pair mismatch")
        }
        guard acceptNonce(hello.pairId + ":" + hello.nonce) else {
            emit(NearbyV1.errorLine(id: id, code: "bad_request", message: "hello nonce was already used"))
            events(.violation("stale hello nonce"))
            return .closeAfterFlush("stale nonce")
        }
        // Sessions per peer, checked before a bootstrap code is consumed.
        if let budget {
            guard budget.admitSession(pairId: hello.pairId) else {
                emit(NearbyV1.errorLine(id: id, code: "too_many_sessions",
                                        message: "pair \(hello.pairId) already holds \(budget.limits.maxSessionsPerPeer) sessions"))
                events(.violation("too many sessions for \(hello.pairId)"))
                return .closeAfterFlush("too many sessions")
            }
            admittedPeer = hello.pairId
        }
        isBootstrap = entry.isBootstrap
        let name = String(hello.companionName.prefix(64))
        var pairPsk: String?
        if isBootstrap {
            guard let psk = pairing?.confirmPairing(pairId: hello.pairId, companionName: name, generation: entry.generation) else {
                emit(NearbyV1.errorLine(id: id, code: "pairing_expired", message: "pairing code is no longer valid"))
                return .closeAfterFlush("pairing expired")
            }
            pairPsk = psk.base64EncodedString()
        } else {
            pairing?.notePairSeen(pairId: hello.pairId)
        }
        pairId = hello.pairId
        events(.hello(pairId: hello.pairId, companionName: name, bootstrap: isBootstrap))
        let macName = macName
        let nonce = hello.nonce
        let reply: (NearbyV1.Destination?) -> Void = { dest in
            emit(NearbyV1.line(id: id, type: "hello_ack",
                               NearbyV1.HelloAck(macName: macName, nonce: nonce, destination: dest, pairPsk: pairPsk)))
        }
        if let destinations { destinations.currentDestination(reply) } else { reply(nil) }
        return .keepOpen
    }
}
