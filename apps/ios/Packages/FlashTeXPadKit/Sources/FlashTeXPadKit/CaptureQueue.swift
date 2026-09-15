import Foundation
import NearbyClient

/// One capture the iPad drew or picked, and what the Mac said about it.
/// Status vocabulary is exactly what nearby-v1/transfer-v1 give a companion:
///   drafted  → sending → received {durable, has_proposal, applied}  |  refused {code}  |  disconnected (retry same id)
/// After the receipt the additive `capture_status` request (nearby-v1 §4)
/// reports the Mac-side outcome in `outcome`: received (inbox) → journaled →
/// converting → proposal_ready (with the LaTeX/TikZ text, read-only here) →
/// inserted / rejected / failed. A Mac that predates the message answers
/// `unknown_type`; the record then says so instead of guessing. Re-sending is
/// still a delivery guarantee (`NearbyAckMemory` answers from the cached ack),
/// never a status probe.
public struct CaptureRecord: Identifiable, Equatable {
    public enum Source: String, Equatable, Codable { case pencil = "Apple Pencil canvas", photo = "photo picker", sample = "bundled sample image", fixture = "1×1 PNG fixture" }
    public enum Status: Equatable, Codable {
        case drafted
        case sending(attempt: Int)
        case received(NearbyWire.CaptureReceived)
        case refused(code: String, message: String)
        case disconnected(reason: String, attempt: Int)
        case discarded

        public var label: String {
            switch self {
            case .drafted: return "drafted — not sent"
            case .sending(let n): return n == 1 ? "sending…" : "re-sending (attempt \(n), same capture_id)…"
            case .received(let r):
                var s = r.durable ? "received — journaled by the Mac's bridge (durable)" : "received — Mac inbox, not journaled (durable:false)"
                if r.applied { s += "; already applied" } else if r.hasProposal { s += "; proposal already existed" }
                return s
            case .refused(let code, _): return "refused by the Mac: \(code)"
            case .disconnected(let why, _): return "not acknowledged (\(why)) — retry re-sends the same capture_id"
            case .discarded: return "discarded before sending"
            }
        }
        public var isTerminal: Bool {
            switch self { case .received, .refused, .discarded: return true; default: return false }
        }
    }

    public let id: String // capture_id
    public var source: Source
    public var png: Data
    public var instructions: String
    public var destinationId: String?
    public var baseRevision: Int?
    /// The pairing (`pair_id`) and Mac (`fp`) the first delivery went to —
    /// frozen with the envelope: retries, status polling and re-delivery are
    /// refused on a link to any other Mac (its destination ids mean nothing
    /// there and the payload would leak). nil until the first attempt.
    public var pairId: String?
    public var macFingerprint: String?
    public var status: Status
    public var createdAt: Date
    public var pixelSize: (width: Int, height: Int)?
    /// Last `capture_status_ack` from the Mac (nil until the first poll answers).
    public var outcome: NearbyWire.CaptureStatus?
    /// Why the outcome cannot be shown (older Mac: `unknown_type`; `unknown_capture`; transport).
    public var outcomeProblem: String?

    public static func == (a: CaptureRecord, b: CaptureRecord) -> Bool {
        a.id == b.id && a.status == b.status && a.instructions == b.instructions && a.png == b.png
            && a.outcome == b.outcome && a.outcomeProblem == b.outcomeProblem
    }

    public init(id: String = "cap-" + UUID().uuidString.lowercased(), source: Source, png: Data, instructions: String,
                pixelSize: (width: Int, height: Int)? = nil) {
        self.id = id; self.source = source; self.png = png; self.instructions = instructions
        self.status = .drafted; self.createdAt = Date(); self.pixelSize = pixelSize
    }

    /// Human label for the Mac-side outcome (the wire `state` verbatim when unknown).
    public var outcomeLabel: String? {
        guard let o = outcome else { return outcomeProblem.map { "outcome unavailable: \($0)" } }
        switch o.state {
        case "received": return "on the Mac (inbox, no bridge attached) — not converted yet"
        case "journaled": return "journaled by the Mac's bridge — waiting for Convert Capture on the Mac"
        case "converting": return "converting on the Mac…"
        case "proposal_ready": return "proposal ready — review and approve it on the Mac"
        case "inserted": return "inserted on the Mac" + (o.newRevision.map { " (revision \($0))" } ?? "")
        case "rejected": return "rejected on the Mac"
        case "failed": return "conversion failed on the Mac"
        case "uncertain": return "Mac bridge relaunched — journaling uncertain; retry is safe"
        default: return "Mac reports state “\(o.state)”"
        }
    }
    /// Polling stops here.
    public var outcomeIsFinal: Bool { outcome?.isFinal ?? false }
}

/// Sends captures through `MacLink` (transfer-v1 `capture_submit` over the
/// nearby session) and keeps one record per capture_id. Retries after a
/// disconnect re-send the identical payload with the same id; the Mac
/// de-duplicates (nearby-v1 §4). Every change is written to `store` when one
/// is attached, so a relaunch shows the same list.
/// `@unchecked Sendable`: `_records`, `_lastStoreError` and `_redeliveries`
/// are serialized by `lock`. `link` and `store` are already `@unchecked Sendable`.
public final class CaptureQueue: @unchecked Sendable {
    /// Snapshot of the list. Mutations go through `update`/`draft` under `lock`
    /// because `refreshOutcome` resumes off the main actor after `await` and
    /// PadModel copies this array into a `@Published` property.
    public var records: [CaptureRecord] { lock.withLock { _records } }
    private var _records: [CaptureRecord] = []
    private let lock = NSLock()
    public let link: MacLink
    public let store: CaptureStore?
    public static let maxInstructionBytes = 4096
    /// Bound on the list (and the PNGs on disk): oldest terminal records go first.
    public static let maxRecords = 50

    public init(link: MacLink, store: CaptureStore? = nil) {
        self.link = link
        self.store = store
        _records = store?.load() ?? []
    }

    public func record(_ id: String) -> CaptureRecord? { lock.withLock { _records.first { $0.id == id } } }

    private func update(_ id: String, _ f: (inout CaptureRecord) -> Void) {
        lock.withLock {
            guard let i = _records.firstIndex(where: { $0.id == id }) else { return }
            f(&_records[i])
            persistLocked()
        }
    }

    /// Caller must hold `lock`. `store.save` runs under that lock so a concurrent
    /// `records` read cannot observe a torn list; CaptureStore has its own lock too.
    private func persistLocked() {
        guard let store else { return }
        if _records.count > Self.maxRecords {
            var kept = _records
            for i in stride(from: kept.count - 1, through: 0, by: -1) where kept.count > Self.maxRecords && kept[i].status.isTerminal { kept.remove(at: i) }
            _records = kept
        }
        do { try store.save(_records) } catch { _lastStoreError = "\(error)" }
    }
    public var lastStoreError: String? { lock.withLock { _lastStoreError } }
    private var _lastStoreError: String?

    /// Client-side checks the Mac would fail anyway (`NearbyWire.checkImage`,
    /// instruction bound), before anything is queued.
    public static func validate(png: Data, instructions: String) -> String? {
        if let why = NearbyWire.checkImage(png, mimeType: "image/png") { return why }
        if instructions.utf8.count > maxInstructionBytes { return "instructions exceed \(maxInstructionBytes) bytes" }
        return nil
    }

    @discardableResult
    public func draft(_ r: CaptureRecord) -> CaptureRecord {
        lock.withLock {
            _records.insert(r, at: 0)
            persistLocked()
        }
        return r
    }

    public func discard(_ id: String) {
        update(id) { if !$0.status.isTerminal { $0.status = .discarded } }
    }

    /// `capture_submit` → `capture_received`; any `error` reply is terminal for
    /// this payload (`refused`); a closed connection leaves the record
    /// retryable with the same id.
    public func send(_ id: String) async {
        guard let r = record(id), !r.status.isTerminal else { return }
        let attempt: Int
        if case .disconnected(_, let n) = r.status { attempt = n + 1 } else { attempt = 1 }
        update(id) { $0.status = .sending(attempt: attempt) }
        do {
            guard let session = link.session, let pair = link.pair else { throw NearbyError.closed("not connected to a Mac") }
            if let bound = r.pairId, bound != pair.pairId {
                throw NearbyError.invalidInput("this capture was first sent to \(r.macFingerprint.map { "the Mac \($0)" } ?? "another Mac") (pair \(bound)); the current link is \(pair.macName) (pair \(pair.pairId)) — discard it and draft a new capture for this Mac")
            }
            // The pin is resolved once, for the first attempt, and persisted
            // before the bytes leave: an uncertain retry (receipt lost, app
            // relaunched mid-send) must carry the identical payload under the
            // same capture_id, or the Mac refuses it (revision_mismatch /
            // capture_id_conflict). A moved pin needs a new capture.
            let dest: NearbyWire.Destination
            if let d = r.destinationId, let rev = r.baseRevision, r.pairId != nil {
                dest = NearbyWire.Destination(destinationId: d, projectId: "", path: "", baseRevision: rev)
            } else {
                let fresh = try await session.destinationQuery()
                guard let fresh else {
                    throw NearbyError.invalidInput("the Mac has no pinned insertion point (Edit > Pin Insertion Point on the Mac)")
                }
                dest = fresh
                update(id) { $0.destinationId = dest.destinationId; $0.baseRevision = dest.baseRevision; $0.pairId = pair.pairId; $0.macFingerprint = pair.fingerprint }
            }
            let submit = try session.makeCapture(captureId: id, image: r.png, mimeType: "image/png", instructions: r.instructions, destination: dest)
            let ack = try await session.submitCapture(submit)
            update(id) { $0.status = .received(ack) }
        } catch let e as NearbyError {
            switch e {
            case .remote(let code, let message) where e.needsNewCapture || e.isClosing:
                update(id) { $0.status = .refused(code: code, message: message) }
            case .closed, .timeout, .unreachable:
                update(id) { $0.status = .disconnected(reason: e.description, attempt: attempt) }
            case .invalidInput(let why):
                update(id) { $0.status = .refused(code: "invalid_input", message: why) }
            default:
                update(id) { $0.status = .disconnected(reason: e.description, attempt: attempt) }
            }
        } catch {
            update(id) { $0.status = .disconnected(reason: "\(error)", attempt: attempt) }
        }
    }

    /// One `capture_status` probe for a received capture. Returns the ack,
    /// or nil when the Mac cannot answer (recorded in `outcomeProblem`; an
    /// `unknown_type` reply means the Mac predates the message and polling
    /// should stop).
    @discardableResult
    public func refreshOutcome(_ id: String) async -> NearbyWire.CaptureStatus? {
        guard let r = record(id), case .received = r.status else { return nil }
        guard let session = link.session, let pair = link.pair else {
            update(id) { $0.outcomeProblem = "not connected to the Mac" }
            return nil
        }
        guard r.pairId == pair.pairId else {
            update(id) { $0.outcomeProblem = "sent to another Mac (pair \(r.pairId ?? "?")); the current link is \(pair.macName) — nothing asked or re-sent here" }
            return nil
        }
        do {
            let s: NearbyWire.CaptureStatus
            do { s = try await session.captureStatus(captureId: id) } catch NearbyError.remote("unknown_capture", _) {
                // The Mac's per-pairing acknowledgement memory no longer holds
                // this id (evicted, or a new key table) although its bridge
                // journal may: re-deliver the exact saved envelope — same
                // capture_id, destination and base_revision, so the Mac / bridge
                // de-duplicate — and ask again once it is acknowledged.
                try await redeliver(r, session: session)
                s = try await session.captureStatus(captureId: id)
            }
            update(id) { $0.outcome = s; $0.outcomeProblem = nil }
            return s
        } catch let e as NearbyError {
            let why: String
            if case .remote(let code, let message) = e {
                why = code == "unknown_type" ? "this Mac's FlashTeX predates capture_status (unknown_type) — review on the Mac" : "\(code): \(message)"
            } else { why = e.description }
            update(id) { $0.outcomeProblem = why }
            return nil
        } catch {
            update(id) { $0.outcomeProblem = "\(error)" }
            return nil
        }
    }

    /// Re-sends a received capture byte-for-byte (saved destination and
    /// base_revision, not a fresh `destination_query`). Records the new receipt.
    public var redeliveries: [String] { lock.withLock { _redeliveries } }
    private var _redeliveries: [String] = []
    private func redeliver(_ r: CaptureRecord, session: NearbySession) async throws {
        guard let destinationId = r.destinationId, let baseRevision = r.baseRevision else {
            throw NearbyError.remote(code: "unknown_capture", message: "no saved destination to re-deliver with")
        }
        let dest = NearbyWire.Destination(destinationId: destinationId, projectId: "", path: "", baseRevision: baseRevision)
        let submit = try session.makeCapture(captureId: r.id, image: r.png, mimeType: "image/png", instructions: r.instructions, destination: dest)
        let ack = try await session.submitCapture(submit)
        lock.withLock { _redeliveries.append(r.id) }
        update(r.id) { $0.status = .received(ack) }
    }

    /// True when another poll makes sense: received, not final, and the Mac
    /// did not say it cannot answer.
    public func shouldPoll(_ id: String) -> Bool {
        guard let r = record(id), case .received = r.status, !r.outcomeIsFinal else { return false }
        if let pair = link.pair, r.pairId != pair.pairId { return false }
        if let p = r.outcomeProblem, p.contains("unknown_type") || p.contains("unknown_capture") { return false }
        return true
    }
}
