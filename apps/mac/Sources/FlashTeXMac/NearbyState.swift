import Combine
import Foundation

/// Holds the one pairing in progress and finalizes it from the listener queue.
/// Kept off the main actor because TLS sessions confirm pairings synchronously.
final class PairingCoordinator: PairingConfirmer {
    /// One pairing attempt. `generation` increases with every `begin` (or is
    /// supplied by the caller's journal) and travels with the bootstrap key,
    /// so a session opened by a replaced code is refused even if its
    /// `pair_id` collides with the current one.
    struct Pending: Equatable {
        let code: String
        let derived: Pairing.Derived
        let expiresAt: Date
        let generation: Int
    }

    let store: PairStore
    private let lock = NSLock()
    private var pending: Pending?
    private var lastGeneration = 0
    /// Called (on an arbitrary queue) when a bootstrap connection became a pairing.
    var onConfirmed: ((PairRecord) -> Void)?
    /// Why the last `confirmPairing` refused, for the window and tests.
    private(set) var lastRefusal: String?

    init(store: PairStore) { self.store = store }

    var current: Pending? { lock.withLock { pending } }
    /// Generation that confirmed a pairing, from pairs.json v2 (`PairRecord.generation`).
    func confirmedGeneration(pairId: String) -> Int? { store.pair(id: pairId)?.generation }

    /// Starts (or resumes) an attempt. Pass the journal's `generation` to
    /// resume one; otherwise the next internal generation is used. Any earlier
    /// pending attempt is replaced.
    func begin(code: String = Pairing.generateCode(), lifetime: TimeInterval = Pairing.codeLifetime, generation: Int? = nil) -> Pending {
        begin(code: code, expiresAt: Date().addingTimeInterval(lifetime), generation: generation)
    }

    /// Same, with the exact expiry a journal recorded for a resumed attempt.
    func begin(code: String, expiresAt: Date, generation: Int? = nil) -> Pending {
        lock.withLock {
            let g = generation ?? (lastGeneration + 1)
            lastGeneration = max(lastGeneration, g)
            let p = Pending(code: code, derived: Pairing.derive(code: code, salt: store.salt), expiresAt: expiresAt, generation: g)
            pending = p
            return p
        }
    }

    func cancel() { lock.withLock { pending = nil } }

    /// Bootstrap PSK entry for the listener while a code is valid.
    var bootstrapEntry: NearbyListener.PSKEntry? {
        guard let p = current, Date() < p.expiresAt else { return nil }
        return .init(identity: p.derived.pairId, key: p.derived.psk, isBootstrap: true, generation: p.generation)
    }

    func confirmPairing(pairId: String, companionName: String, generation: Int?) -> Data? {
        let record: PairRecord? = lock.withLock {
            guard let p = pending else { lastRefusal = "no pairing code is pending"; return nil }
            guard p.derived.pairId == pairId else { lastRefusal = "pair_id \(pairId) is not the pending attempt"; return nil }
            guard generation == nil || generation == p.generation else {
                lastRefusal = "bootstrap key from replaced attempt \(generation ?? -1); current is \(p.generation)"
                return nil
            }
            guard Date() < p.expiresAt else { lastRefusal = "pairing code expired"; return nil }
            // Persisted defence: a pairing already confirmed by this or a newer
            // attempt cannot be re-confirmed by an older bootstrap session.
            if let existing = store.pair(id: pairId)?.generation, existing >= p.generation {
                lastRefusal = "pair \(pairId) was already confirmed by attempt \(existing); this attempt is \(p.generation)"
                return nil
            }
            let psk = Pairing.mintLongTermPSK()
            let r = PairRecord(pairId: pairId, psk: psk.base64EncodedString(), companionName: companionName,
                               createdAt: Date(), lastSeenAt: Date(), generation: p.generation) // pairs.json v2
            guard store.upsert(r) else { lastRefusal = "pair store refused the record"; return nil } // not persisted → not paired
            lastRefusal = nil
            pending = nil
            return r
        }
        guard let record else { return nil }
        onConfirmed?(record)
        return record.pskData
    }

    func notePairSeen(pairId: String) { store.touch(pairId: pairId) }
    func noteCapture(pairId: String, captureId: String, remembered: NearbyAckMemory.Persisted?) {
        store.recordCapture(pairId: pairId, captureId: captureId, remembered: remembered)
    }
    /// Read from the store on every capture (listener queue), so the window's
    /// pop-up applies to the next `capture_submit` without a restart.
    func capturesPermitted(pairId: String) -> Bool { store.capturesPermitted(pairId: pairId) }
}

/// UI-facing state for the nearby listener: advertising, port, pairing code,
/// paired companions, last received capture, and the explicit receive-error
/// state (refused captures, duplicates) the window shows.
@MainActor
final class NearbyState: ObservableObject {
    /// One refused capture, as the listener reported it to the companion.
    struct ReceiveError: Equatable, Identifiable {
        let id: UUID
        let date: Date
        let pairId: String?
        let captureId: String?
        let code: String
        let message: String
        var summary: String {
            "\(code): \(captureId ?? "?") from \(pairId ?? "unauthenticated") — \(message)"
        }
    }

    @Published private(set) var isAdvertising = false
    @Published private(set) var port: UInt16?
    @Published private(set) var status = "off"
    @Published private(set) var pairingCode: String?
    @Published private(set) var codeExpiresAt: Date?
    @Published private(set) var pairs: [PairRecord] = []
    @Published private(set) var connectedPairIds: [String] = []
    @Published private(set) var lastReceivedCaptureId: String?
    @Published private(set) var log: [String] = []
    /// Most recent refusal, until `clearReceiveErrors()`; nil means none since.
    @Published private(set) var lastReceiveError: ReceiveError?
    /// Recent refusals, newest last (bounded).
    @Published private(set) var receiveErrors: [ReceiveError] = []
    /// Retries of already accepted captures (acknowledged, not re-delivered).
    @Published private(set) var duplicateCaptureCount = 0
    @Published private(set) var lastDuplicateCaptureId: String?
    static let maxReceiveErrors = 20
    /// Transport counters for the window's metrics line (`NearbyMetrics.swift`).
    @Published private(set) var metrics = NearbyTransportMetrics()
    /// Set by the Captures panel's "Pairing code…" before it opens the Nearby
    /// window; the window shows a code and clears it (one click to a code).
    @Published var codeRequested = false

    /// Raw listener events, forwarded on the main actor before `handle` acts
    /// on them (the pairing flow controller's `observe(_:)` consumes these).
    var onEvent: ((NearbyListener.Event) -> Void)?
    /// Last `.receiving` progress per connected pairing (bytes of the line in
    /// flight, or of the line just completed).
    @Published private(set) var receivingBytes: [String: Int] = [:]

    let macName: String
    let store: PairStore
    let coordinator: PairingCoordinator
    let loopbackOnly: Bool
    let limits: NearbyReceiveLimits
    private let queue = DispatchQueue(label: "flashtex.nearby.listener")
    private var listener: NearbyListener?
    /// Acknowledgement memory shared by every listener this state creates, so
    /// `stopAdvertising()`/`startAdvertising()` within a session does not
    /// re-deliver a companion's retry; seeded from `pairs.json` (bounded,
    /// acknowledged entries only) and trimmed when a pairing is forgotten.
    let ackMemory: NearbyAckMemory
    private weak var sink: CaptureSink?
    private weak var destinations: DestinationProvider?
    private var expiry: DispatchWorkItem?
    private var wantAdvertising = false

    init(store: PairStore = PairStore(url: PairStore.defaultURL()),
         macName: String = Host.current().localizedName ?? "Mac", loopbackOnly: Bool = false,
         limits: NearbyReceiveLimits = .init()) {
        self.store = store
        self.macName = macName
        self.loopbackOnly = loopbackOnly
        self.limits = limits
        self.coordinator = PairingCoordinator(store: store)
        self.pairs = store.pairs
        self.ackMemory = NearbyAckMemory(maxPerPair: limits.maxRememberedCaptures)
        for r in store.pairs { ackMemory.restore(pairId: r.pairId, entries: r.rememberedCaptures ?? []) }
        if let e = store.loadError { log.append(e); status = e }
        coordinator.onConfirmed = { [weak self] record in
            Task { @MainActor in self?.pairingConfirmed(record) }
        }
    }

    var fingerprint: String { Pairing.fingerprint(salt: store.salt) }
    var serviceType: String { NearbyV1.serviceType }
    var cipherSuite: String { NearbyListener.cipherSuiteName }

    var txtRecord: [String: String] {
        ["v": "1", "name": macName, "fp": fingerprint, "salt": Pairing.hex(store.salt)]
    }

    /// Where captures and destination queries go. The App attaches the ShellModel.
    func attach(sink: CaptureSink, destinations: DestinationProvider) {
        self.sink = sink
        self.destinations = destinations
        if listener != nil { restartListener() }
    }

    // MARK: advertising

    func startAdvertising() {
        wantAdvertising = true
        restartListener()
    }

    func stopAdvertising() {
        wantAdvertising = false
        cancelPairing()
        listener?.stop()
        listener = nil
        isAdvertising = false
        port = nil
        connectedPairIds = []
        status = "off"
        note("stopped")
    }

    /// Rebuilds the listener from the pair store plus any pending bootstrap key,
    /// keeping already-authenticated connections alive.
    private func restartListener() {
        guard wantAdvertising else { return }
        var psks = pairs.compactMap { r -> NearbyListener.PSKEntry? in
            guard let k = r.pskData, k.count == Pairing.pskLength else { return nil }
            return .init(identity: r.pairId, key: k, isBootstrap: false)
        }
        if let b = coordinator.bootstrapEntry { psks.append(b) }
        let config = NearbyListener.Configuration(
            psks: psks, macName: macName, port: port,
            advertisement: .init(name: macName, txt: txtRecord), loopbackOnly: loopbackOnly, limits: limits)
        let next = NearbyListener(configuration: config, sink: sink, destinations: destinations,
                                  pairing: coordinator, queue: queue, memory: ackMemory) { [weak self] event in
            Task { @MainActor in self?.handle(event) }
        }
        let previous = listener
        listener = next
        status = "starting…"
        let start: () -> Void = { [weak self, weak next] in
            Task { @MainActor in
                guard let self, let next, self.listener === next else { return }
                do { try next.start() } catch {
                    self.status = "listener failed: \(error.localizedDescription)"
                    self.note(self.status)
                    self.isAdvertising = false
                }
            }
        }
        if let old = previous {
            next.adoptConnections(from: old)
            old.stop(keepConnections: true, completion: start)
        } else {
            start()
        }
    }

    private func handle(_ event: NearbyListener.Event) {
        onEvent?(event)
        metrics.record(event)
        switch event {
        case .ready(let p):
            port = p
            isAdvertising = true
            status = "advertising \(serviceType) on port \(p)"
            note("ready on port \(p)")
        case .failed(let why):
            status = "listener error: \(why)"
            note(status)
            if port != nil, listener != nil {
                // The requested port may be gone; try once more with an ephemeral one.
                port = nil
                listener = nil
                restartListener()
            } else {
                isAdvertising = false
            }
        case .stopped:
            break
        case .connectionOpened:
            note("authenticated connection")
        case .hello(let id, let name, let bootstrap):
            connectedPairIds.append(id)
            note(bootstrap ? "paired \(name) (\(id))" : "hello from \(name) (\(id))")
        case .capture(let id):
            lastReceivedCaptureId = id
            pairs = store.pairs // capture_count / last_capture_* summaries moved
            note("capture \(id)")
        case .captureRefused(let pairId, let captureId, let code, let message):
            let e = ReceiveError(id: UUID(), date: Date(), pairId: pairId, captureId: captureId, code: code, message: message)
            lastReceiveError = e
            receiveErrors.append(e)
            if receiveErrors.count > Self.maxReceiveErrors { receiveErrors.removeFirst(receiveErrors.count - Self.maxReceiveErrors) }
            note("refused \(e.summary)")
        case .captureDuplicate(let pairId, let captureId):
            duplicateCaptureCount += 1
            lastDuplicateCaptureId = captureId
            note("duplicate \(captureId) from \(pairId ?? "?") acknowledged again, not re-delivered")
        case .receiving(let id, let bytes, _):
            if let id { receivingBytes[id] = bytes }
        case .connectionClosed(let id, let reason):
            if let id { receivingBytes.removeValue(forKey: id) }
            if let id, let i = connectedPairIds.firstIndex(of: id) { connectedPairIds.remove(at: i) }
            note("closed \(id ?? "unauthenticated"): \(reason)")
        case .handshakeRefused(let address, let failures, let wait):
            note("refused \(address) before TLS: \(failures) failed handshakes, retry in \(wait) s")
        case .handshakePending(let address, let pending):
            note("refused \(address) before TLS: \(pending) handshakes from it already in progress")
        }
    }

    // MARK: pairing

    /// Shows a fresh code for `Pairing.codeLifetime`; the derived bootstrap PSK
    /// is accepted only until then.
    func beginPairing() { beginPairing(generation: nil) }

    /// Shows a fresh code carrying the caller's journal `generation` (nil =
    /// the coordinator's next one), so the confirmed record persists it.
    func beginPairing(generation: Int?) {
        if !wantAdvertising { wantAdvertising = true }
        let p = coordinator.begin(generation: generation)
        show(p)
        note("pairing code issued for \(p.derived.pairId) (attempt \(p.generation))")
        restartListener()
    }

    /// Resumes an attempt that was interrupted (relaunch, advertising off):
    /// serves `code` until `expiresAt` with the same published code/expiry and
    /// timer as `beginPairing()`. An already expired attempt is not resumed,
    /// nor one journaled by an older build with a weak (six-digit) code: its
    /// handshake could be cracked offline (proposal §5).
    func beginPairing(resuming code: String, expiresAt: Date, generation: Int? = nil) {
        guard expiresAt > Date() else {
            note("not resuming pairing code: already expired")
            return
        }
        guard Pairing.canonicalCode(code) == code else {
            note("not resuming pairing code: not a full-strength code")
            return
        }
        if !wantAdvertising { wantAdvertising = true }
        let p = coordinator.begin(code: code, expiresAt: expiresAt, generation: generation)
        show(p)
        note("pairing code resumed for \(p.derived.pairId)")
        restartListener()
    }

    private func show(_ p: PairingCoordinator.Pending) {
        pairingCode = p.code
        codeExpiresAt = p.expiresAt
        expiry?.cancel()
        let item = DispatchWorkItem { [weak self] in self?.pairingExpired() }
        expiry = item
        DispatchQueue.main.asyncAfter(deadline: .now() + max(0, p.expiresAt.timeIntervalSinceNow), execute: item)
    }

    /// Closes every live session of one pairing (cancels a receive in
    /// progress). The pairing itself is kept; `connectedPairIds` updates when
    /// the listener reports the close.
    func closeConnection(pairId: String) {
        guard let listener else { return }
        listener.closeConnections(identity: pairId)
        note("closing sessions of \(pairId)")
    }

    func cancelPairing() {
        expiry?.cancel()
        expiry = nil
        let had = pairingCode != nil
        coordinator.cancel()
        pairingCode = nil
        codeExpiresAt = nil
        if had, listener != nil { restartListener() }
    }

    private func pairingExpired() {
        guard pairingCode != nil else { return }
        note("pairing code expired")
        cancelPairing()
    }

    private func pairingConfirmed(_ record: PairRecord) {
        expiry?.cancel()
        expiry = nil
        pairingCode = nil
        codeExpiresAt = nil
        pairs = store.pairs
        note("stored pairing \(record.pairId) for \(record.companionName)")
        // Drop the bootstrap key and add the long-term one; the confirming
        // connection is adopted, not dropped.
        restartListener()
    }

    func forget(pairId: String) {
        store.remove(pairId: pairId)
        ackMemory.forget(pairId: pairId)
        pairs = store.pairs
        note("forgot \(pairId)")
        if listener != nil { restartListener() }
    }

    func refreshPairs() { pairs = store.pairs }

    /// Changes a companion's permission (persisted in pairs.json v3). Takes
    /// effect on that companion's next `capture_submit`; live sessions stay
    /// open either way. A receive already in flight is not interrupted.
    func setPermission(pairId: String, _ permission: CompanionPermission) {
        guard let record = store.pair(id: pairId) else { return }
        guard record.effectivePermission != permission else { return }
        guard store.setPermission(pairId: pairId, permission) else {
            note("could not persist permission for \(pairId)")
            return
        }
        pairs = store.pairs
        note("\(record.companionName) (\(pairId)) permission: \(permission.rawValue)")
    }

    /// Activity summaries for the window, keyed by pair_id; never includes the key.
    var activity: [String: PairActivity] {
        Dictionary(uniqueKeysWithValues: pairs.map { ($0.pairId, $0.activity) })
    }

    func resetMetrics() { metrics.reset() }

    /// Acknowledges the error state in the UI; the log keeps its lines.
    func clearReceiveErrors() {
        lastReceiveError = nil
        receiveErrors = []
    }

    private func note(_ line: String) {
        log.append(line)
        if log.count > 100 { log.removeFirst(log.count - 100) }
    }
}

/// What the pairing window shows per companion: seen/captured timestamps and
/// counts from pairs.json v2. Built from a `PairRecord` without its key.
struct PairActivity: Equatable {
    var pairId: String
    var companionName: String
    var createdAt: Date
    var lastSeenAt: Date?
    var captureCount: Int
    var lastCaptureAt: Date?
    var lastCaptureId: String?
    var generation: Int?

    /// One line for logs and captions; no PSK material can appear here.
    var summary: String {
        var parts = ["\(companionName) (\(pairId))"]
        parts.append(lastSeenAt.map { "seen \(Pairing.stamp($0))" } ?? "never seen")
        parts.append("\(captureCount) capture\(captureCount == 1 ? "" : "s")")
        if let id = lastCaptureId, let at = lastCaptureAt { parts.append("last \(id) at \(Pairing.stamp(at))") }
        if let g = generation { parts.append("attempt \(g)") }
        return parts.joined(separator: ", ")
    }
}

extension PairRecord {
    var activity: PairActivity {
        .init(pairId: pairId, companionName: companionName, createdAt: createdAt, lastSeenAt: lastSeenAt,
              captureCount: captureCount ?? 0, lastCaptureAt: lastCaptureAt, lastCaptureId: lastCaptureId, generation: generation)
    }
}
