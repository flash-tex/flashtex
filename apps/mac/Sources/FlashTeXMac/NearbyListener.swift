import Foundation
import Network
import Security
import FlashTeXProtocol

/// TLS-PSK listener for paired companions (proposal nearby-v1).
///
/// Transport: TCP, TLS 1.2 only, single cipher suite
/// `TLS_PSK_WITH_AES_128_GCM_SHA256` (0x00A8), one PSK per pairing keyed by its
/// `pair_id` as the TLS PSK identity. A peer without a matching PSK fails the
/// handshake, so no application byte is ever parsed from an unpaired device.
/// Bonjour (`_flashtex._tcp`) is advertised only for discovery.
///
/// Everything below runs on `queue`; callbacks into the sink/destination
/// provider are theirs to dispatch, and their replies are re-queued here.
final class NearbyListener {
    struct PSKEntry: Equatable {
        let identity: String
        let key: Data
        /// Code-derived key that only authenticates the pairing connection.
        let isBootstrap: Bool
        /// Pairing attempt a bootstrap key belongs to; nil for long-term keys.
        var generation: Int? = nil
        init(identity: String, key: Data, isBootstrap: Bool, generation: Int? = nil) {
            self.identity = identity; self.key = key; self.isBootstrap = isBootstrap; self.generation = generation
        }
    }

    struct Advertisement: Equatable {
        var name: String
        var txt: [String: String]
    }

    struct Configuration {
        var psks: [PSKEntry]
        var macName: String
        var port: UInt16? = nil
        var advertisement: Advertisement? = nil
        var loopbackOnly = false
        /// Receive caps (frame, image, in-flight bytes, sessions, connections).
        var limits = NearbyReceiveLimits()
        var maxLineBytes: Int {
            get { limits.maxLineBytes }
            set { limits.maxLineBytes = newValue }
        }
    }

    enum Event: Equatable {
        case ready(port: UInt16)
        case failed(String)
        case stopped
        case connectionOpened
        case hello(pairId: String, companionName: String, bootstrap: Bool)
        case capture(captureId: String)
        /// A capture refused with an explicit `error` reply (validation, a
        /// cap, a revision mismatch, or the sink's own refusal).
        case captureRefused(identity: String?, captureId: String?, code: String, message: String)
        /// A retry of an accepted capture: acknowledged again, not re-delivered.
        case captureDuplicate(identity: String?, captureId: String)
        /// Byte progress of the line being received on one connection: at most
        /// once per `receivingReportInterval` of growth and once when the line
        /// completes (`bytes` is then the whole line). `expected` stays nil:
        /// JSON Lines carry no length hint.
        case receiving(identity: String?, bytes: Int, expected: Int?)
        case connectionClosed(identity: String?, reason: String)
        /// A connection from an address in handshake backoff, closed before
        /// TLS (`NearbyHandshakeLimiter`); `retryAfter` is in whole seconds.
        case handshakeRefused(address: String, failures: Int, retryAfter: Int)
        /// A connection from an address that already has
        /// `maxPendingPerAddress` handshakes in progress, closed before TLS.
        case handshakePending(address: String, pending: Int)
    }
    static let receivingReportInterval = 64 * 1024

    static let cipherSuite = tls_ciphersuite_t(rawValue: UInt16(TLS_PSK_WITH_AES_128_GCM_SHA256))!
    static let cipherSuiteName = "TLS_PSK_WITH_AES_128_GCM_SHA256"

    let configuration: Configuration
    let queue: DispatchQueue
    private weak var sink: CaptureSink?
    private weak var destinations: DestinationProvider?
    private weak var pairing: PairingConfirmer?
    private let events: (Event) -> Void
    private var listener: NWListener?
    private static let queueKey = DispatchSpecificKey<Bool>()
    private var connections: [ObjectIdentifier: Connection] = [:]
    private(set) var port: UInt16?
    private(set) var advertisedName: String?
    /// Recent `pair_id:nonce` values (bounded); a repeat is refused.
    private var recentNonces: [String] = []
    static let rememberedNonces = 256
    /// Listener-wide in-flight bytes and sessions per peer; shared with the
    /// replacement listener on restart so the caps survive a PSK table change.
    private(set) var budget: NearbyReceiveBudget
    /// Acknowledged/pending captures per pairing, shared by that pairing's
    /// sessions and by the replacement listener on restart, so a reconnecting
    /// companion's retry is acknowledged from memory and never re-delivered.
    private(set) var memory: NearbyAckMemory
    /// Failed handshakes per address; on `queue`, and
    /// carried to the replacement listener by `adoptConnections`.
    private(set) var limiter: NearbyHandshakeLimiter
    /// Off-queue JSON/base64/image work; concurrency is bounded by the budget.
    private let decodeQueue: DispatchQueue

    init(configuration: Configuration, sink: CaptureSink?, destinations: DestinationProvider?,
         pairing: PairingConfirmer?, queue: DispatchQueue = DispatchQueue(label: "flashtex.nearby"),
         memory: NearbyAckMemory? = nil, events: @escaping (Event) -> Void) {
        self.configuration = configuration
        self.queue = queue
        self.budget = NearbyReceiveBudget(limits: configuration.limits)
        // An owner-supplied memory outlives this listener (NearbyState keeps
        // one per session); otherwise the memory is this listener's own and
        // moves only through `adoptConnections`.
        self.memory = memory ?? NearbyAckMemory(maxPerPair: configuration.limits.maxRememberedCaptures)
        self.memory.limit = configuration.limits.maxRememberedCaptures
        self.limiter = NearbyHandshakeLimiter(policy: configuration.limits.handshake)
        self.decodeQueue = DispatchQueue(label: "flashtex.nearby.decode", qos: .utility, attributes: .concurrent)
        queue.setSpecific(key: Self.queueKey, value: true)
        self.sink = sink
        self.destinations = destinations
        self.pairing = pairing
        self.events = events
    }

    // MARK: TLS parameters (shared with the companion-side client)

    static func tlsOptions(psks: [(identity: String, key: Data)]) -> NWProtocolTLS.Options {
        let tls = NWProtocolTLS.Options()
        let sec = tls.securityProtocolOptions
        sec_protocol_options_set_min_tls_protocol_version(sec, .TLSv12)
        sec_protocol_options_set_max_tls_protocol_version(sec, .TLSv12)
        sec_protocol_options_append_tls_ciphersuite(sec, cipherSuite)
        // No resumption: every connection must prove the PSK again, so a
        // forgotten or expired key cannot ride a cached session.
        sec_protocol_options_set_tls_resumption_enabled(sec, false)
        sec_protocol_options_set_tls_tickets_enabled(sec, false)
        for entry in psks {
            sec_protocol_options_add_pre_shared_key(sec, dispatchData(entry.key), dispatchData(Data(entry.identity.utf8)))
        }
        return tls
    }

    static func parameters(psks: [(identity: String, key: Data)], loopbackOnly: Bool,
                           limits: NearbyReceiveLimits = .init()) -> NWParameters {
        let tcp = NWProtocolTCP.Options()
        // Half-open detection: a peer that vanished without a FIN is probed
        // and dropped after about idle + interval × count seconds, which
        // releases its session slot and any in-flight budget.
        tcp.enableKeepalive = true
        tcp.keepaliveIdle = limits.keepaliveIdleSeconds
        tcp.keepaliveInterval = limits.keepaliveIntervalSeconds
        tcp.keepaliveCount = limits.keepaliveCount
        tcp.noDelay = true
        let params = NWParameters(tls: tlsOptions(psks: psks), tcp: tcp)
        params.allowLocalEndpointReuse = true
        params.includePeerToPeer = false
        if loopbackOnly { params.requiredInterfaceType = .loopback }
        return params
    }

    /// Parameters a companion uses to connect with one pairing.
    static func clientParameters(identity: String, psk: Data) -> NWParameters {
        parameters(psks: [(identity, psk)], loopbackOnly: false)
    }

    static func dispatchData(_ data: Data) -> __DispatchData {
        data.withUnsafeBytes { DispatchData(bytes: $0) } as __DispatchData
    }

    /// Negotiated TLS version and cipher suite of an established connection.
    static func negotiated(_ connection: NWConnection) -> (version: tls_protocol_version_t, suite: tls_ciphersuite_t)? {
        guard let meta = connection.metadata(definition: NWProtocolTLS.definition) as? NWProtocolTLS.Metadata else { return nil }
        let sec = meta.securityProtocolMetadata
        return (sec_protocol_metadata_get_negotiated_tls_protocol_version(sec),
                sec_protocol_metadata_get_negotiated_tls_ciphersuite(sec))
    }

    // MARK: lifecycle

    func start() throws {
        let params = Self.parameters(psks: configuration.psks.map { ($0.identity, $0.key) },
                                     loopbackOnly: configuration.loopbackOnly, limits: configuration.limits)
        let nwPort: NWEndpoint.Port = configuration.port.flatMap { NWEndpoint.Port(rawValue: $0) } ?? .any
        let listener = try NWListener(using: params, on: nwPort)
        if let ad = configuration.advertisement {
            listener.service = NWListener.Service(name: ad.name, type: NearbyV1.serviceType, domain: nil,
                                                  txtRecord: NWTXTRecord(ad.txt))
            listener.serviceRegistrationUpdateHandler = { [weak self] change in
                guard let self else { return }
                if case .add(let endpoint) = change, case .service(let name, _, _, _) = endpoint {
                    self.advertisedName = name
                }
            }
        }
        listener.stateUpdateHandler = { [weak self] state in
            guard let self else { return }
            switch state {
            case .ready:
                let p = self.listener?.port?.rawValue ?? 0
                self.port = p
                self.events(.ready(port: p))
            case .failed(let error):
                self.events(.failed(String(describing: error)))
            case .waiting(let error):
                self.events(.failed("waiting: \(error)"))
            case .cancelled:
                self.events(.stopped)
            default: break
            }
        }
        listener.newConnectionHandler = { [weak self] nw in
            guard let self else { nw.cancel(); return }
            // An address that keeps failing the handshake waits out its
            // backoff before the stack spends another handshake on it.
            let address = Self.address(of: nw)
            if let wait = self.limiter.retryAfter(address, now: Date()) {
                nw.cancel()
                self.events(.handshakeRefused(address: address, failures: self.limiter.failures(address),
                                              retryAfter: Int(wait.rounded(.up))))
                return
            }
            // Simultaneous connections would all be accepted before any had
            // failed; one address gets a few handshakes at a time.
            let pending = self.connections.values.filter { $0.address == address && !$0.isAuthenticated }.count
            if pending >= self.limiter.policy.maxPendingPerAddress {
                nw.cancel()
                self.events(.handshakePending(address: address, pending: pending))
                return
            }
            // Unauthenticated peers get no application bytes, so the cap is
            // a close plus an event rather than an error envelope.
            guard self.connections.count < self.configuration.limits.maxConnections else {
                nw.cancel()
                self.events(.connectionClosed(identity: nil, reason: "too many connections (\(self.configuration.limits.maxConnections))"))
                return
            }
            let c = Connection(nw, address: address, owner: self)
            self.connections[ObjectIdentifier(c)] = c
            c.start()
        }
        self.listener = listener
        listener.start(queue: queue)
    }

    /// Stops accepting and advertising. `keepConnections` leaves authenticated
    /// sessions open (used when the PSK table changes after a pairing).
    /// `completion` runs on `queue` once the socket is released, so a
    /// replacement can bind the same port without racing the old one.
    func stop(keepConnections: Bool = false, completion: (() -> Void)? = nil) {
        queue.async {
            if let l = self.listener {
                // Independent of `self`'s lifetime: the owner may drop this
                // instance as soon as it has asked for the stop.
                let events = self.events
                l.stateUpdateHandler = { state in
                    if case .cancelled = state { events(.stopped); completion?() }
                }
                l.cancel()
                self.listener = nil
            } else {
                completion?()
            }
            if !keepConnections {
                for c in self.connections.values { c.close(reason: "listener stopped") }
            }
        }
    }

    /// Runs `work` on `queue`, inline when already there (keeps reply order).
    fileprivate func onQueue(_ work: @escaping () -> Void) {
        if DispatchQueue.getSpecific(key: Self.queueKey) == true { work() } else { queue.async(execute: work) }
    }

    /// Call from outside `queue` only.
    var openConnectionCount: Int { queue.sync { connections.count } }

    /// Moves live sessions to another listener instance (same queue), so a PSK
    /// table change does not drop a companion that just paired. Call from
    /// outside `queue` only.
    func adoptConnections(from other: NearbyListener) {
        precondition(other.queue === queue)
        let keep = Set(configuration.psks.map(\.identity))
        // The pairing code this table stopped serving (cancelled, expired,
        // replaced or consumed): a session that opened with it but has not
        // said hello is told so before the close, instead of a bare FIN.
        let oldBootstrap = other.configuration.psks.first(where: \.isBootstrap)
        let codeWithdrawn = oldBootstrap != nil && oldBootstrap != configuration.psks.first(where: \.isBootstrap)
        queue.sync {
            for (k, c) in other.connections {
                c.owner = self
                connections[k] = c
                // A forgotten pairing loses its live session too; a session that
                // never said hello cannot be re-keyed and is dropped.
                if let id = c.identity {
                    if !keep.contains(id) { c.close(reason: "pairing forgotten") }
                } else if codeWithdrawn {
                    c.refuse(code: "pairing_cancelled", message: "the Mac withdrew the pairing code before hello",
                             reason: "pairing code withdrawn before hello")
                } else {
                    c.close(reason: "key table changed before hello")
                }
            }
            other.connections.removeAll()
            recentNonces = other.recentNonces
            budget = other.budget
            budget.limits = configuration.limits
            // A key-table change (a new code, a stored pairing) must not
            // wipe an address's backoff.
            limiter = other.limiter
            limiter.policy = configuration.limits.handshake
            // Acknowledgement memory follows the pairings that survive; a
            // forgotten pairing's retries must not be answered from it.
            memory = other.memory
            memory.limit = configuration.limits.maxRememberedCaptures
            for id in memory.pairIds where !keep.contains(id) { memory.forget(pairId: id) }
        }
    }

    /// Bytes accepted from every session and not yet acknowledged.
    var inboxBytesInFlight: Int { budget.inboxBytesInFlight }

    /// Closes every live session of one pairing (the Mac cancelling a receive
    /// or dropping a companion). The `.connectionClosed` events follow on
    /// `queue`; a session mid-line stops reading and releases its budget.
    func closeConnections(identity: String, reason: String = "closed by the Mac") {
        queue.async {
            for c in self.connections.values where c.identity == identity { c.close(reason: reason) }
        }
    }

    fileprivate func forget(_ c: Connection) { connections.removeValue(forKey: ObjectIdentifier(c)) }

    /// The peer's host (no port): what handshake backoff is keyed by.
    static func address(of nw: NWConnection) -> String {
        if case .hostPort(let host, _) = nw.endpoint { return "\(host)" }
        return "\(nw.endpoint)"
    }

    /// On `queue`: a connection ended before TLS completed.
    fileprivate func handshakeFailed(address: String) {
        limiter.recordFailure(address, now: Date())
    }

    /// On `queue`: TLS completed with one of the table's keys.
    fileprivate func handshakeSucceeded(address: String) { limiter.recordSuccess(address) }

    /// Call from outside `queue` only.
    var handshakeLimiter: NearbyHandshakeLimiter { queue.sync { limiter } }

    /// On `queue`. Nonces survive a restart only through `adoptConnections`.
    fileprivate func acceptNonce(_ key: String) -> Bool {
        guard !recentNonces.contains(key) else { return false }
        recentNonces.append(key)
        if recentNonces.count > Self.rememberedNonces { recentNonces.removeFirst(recentNonces.count - Self.rememberedNonces) }
        return true
    }
    fileprivate func emitEvent(_ e: Event) { events(e) }

    // MARK: one connection

    fileprivate final class Connection {
        let nw: NWConnection
        weak var owner: NearbyListener?
        private var splitter = LineSplitter()
        private var session: NearbySession?
        private var closing = false
        /// Pending bytes at the last `.receiving` report.
        private var reportedPending = 0
        private(set) var identity: String?
        private let maxLineBytes: Int
        private let limits: NearbyReceiveLimits
        private let queue: DispatchQueue
        /// One armed deadline at a time: handshake, then hello, then the
        /// frame in progress (none while idle between complete lines).
        private var deadline: DispatchWorkItem?
        /// The peer's host, for handshake backoff.
        let address: String
        /// Set once the handshake's outcome went to the limiter.
        private var handshakeCounted = false
        /// TLS completed (with any key in the table).
        var isAuthenticated: Bool { session != nil }

        init(_ nw: NWConnection, address: String, owner: NearbyListener) {
            self.nw = nw
            self.address = address
            self.owner = owner
            self.maxLineBytes = owner.configuration.maxLineBytes
            self.limits = owner.configuration.limits
            self.queue = owner.queue
        }

        func start() {
            nw.stateUpdateHandler = { [weak self] state in
                guard let self else { return }
                switch state {
                case .ready: self.authenticated()
                case .failed(let error):
                    // Typical for a plaintext or wrong-key peer: the handshake
                    // failed. A failed connection still needs cancel to release
                    // its socket; the second, `.cancelled`, pass is a no-op.
                    if self.session == nil { self.countHandshake(failed: true) }
                    self.finish(reason: self.session == nil ? "handshake failed: \(error)" : "failed: \(error)")
                    self.nw.cancel()
                case .cancelled: self.finish(reason: "cancelled")
                default: break
                }
            }
            // A TCP peer that never starts (or never finishes) TLS must not
            // hold one of the connection slots: no bytes are owed to it.
            arm(after: limits.handshakeTimeout) { [weak self] in
                guard let self, self.session == nil else { return }
                self.countHandshake(failed: true)
                self.close(reason: NearbyFrameError.handshakeTimeout(seconds: self.limits.handshakeTimeout).reason)
            }
            nw.start(queue: queue)
        }

        /// Reports this connection's handshake to the limiter, once.
        private func countHandshake(failed: Bool) {
            guard !handshakeCounted, let owner else { return }
            handshakeCounted = true
            if failed { owner.handshakeFailed(address: address) } else { owner.handshakeSucceeded(address: address) }
        }

        // MARK: deadlines (all on `queue`)

        private func arm(after seconds: TimeInterval, _ fire: @escaping () -> Void) {
            deadline?.cancel()
            let item = DispatchWorkItem { [weak self] in
                guard let self, !self.closing else { return }
                self.deadline = nil
                fire()
            }
            deadline = item
            queue.asyncAfter(deadline: .now() + max(0, seconds), execute: item)
        }

        private func disarm() {
            deadline?.cancel()
            deadline = nil
        }

        /// Refuses on framing grounds: the typed error's code goes to the peer
        /// (when it has one), its reason to the listener's event.
        private func refuse(_ error: NearbyFrameError) {
            if let code = error.code { send(NearbyV1.errorLine(id: nil, code: code, message: error.message)) }
            closeAfterFlush(reason: error.reason)
        }

        /// Called only after TLS completed with one of the table keys. The
        /// stack does not say which, so the session is named by `hello`'s proof.
        private func authenticated() {
            guard let owner else { close(reason: "listener gone"); return }
            guard let n = NearbyListener.negotiated(nw), n.version == .TLSv12, n.suite == NearbyListener.cipherSuite else {
                close(reason: "unexpected TLS parameters")
                return
            }
            countHandshake(failed: false)
            var sessionIdentity: String?
            session = NearbySession(keys: owner.configuration.psks, macName: owner.configuration.macName,
                                    sink: owner.sink, destinations: owner.destinations, pairing: owner.pairing,
                                    limits: owner.configuration.limits, budget: owner.budget,
                                    decodeQueue: owner.decodeQueue, memory: owner.memory,
                                    onStateQueue: { [weak self] work in
                                        guard let self, let owner = self.owner else { work(); return }
                                        owner.onQueue(work)
                                    },
                                    acceptNonce: { [weak self] key in self?.owner?.acceptNonce(key) ?? false }) { [weak self, weak owner] e in
                // A capture the sink acknowledges after this connection is
                // gone (the companion dropped mid-delivery) is still reported
                // to the listener that now owns the pairing's memory.
                let listener = self?.owner ?? owner
                let identity = self?.identity ?? sessionIdentity
                switch e {
                case .hello(let p, let n, let b):
                    sessionIdentity = p
                    self?.identity = p
                    self?.disarm()
                    listener?.emitEvent(.hello(pairId: p, companionName: n, bootstrap: b))
                case .capture(let id): listener?.emitEvent(.capture(captureId: id))
                case .refused(let id, let code, let message):
                    listener?.emitEvent(.captureRefused(identity: identity, captureId: id, code: code, message: message))
                case .duplicate(let id):
                    listener?.emitEvent(.captureDuplicate(identity: identity, captureId: id))
                case .violation: break
                }
            }
            owner.emitEvent(.connectionOpened)
            // Authenticated but silent: the slot and the per-peer session
            // count are not held open for a peer that never says hello.
            arm(after: limits.helloTimeout) { [weak self] in
                guard let self, self.identity == nil else { return }
                self.refuse(.helloTimeout(seconds: self.limits.helloTimeout))
            }
            receiveLoop()
        }

        private func receiveLoop() {
            nw.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { [weak self] data, _, isComplete, error in
                guard let self, !self.closing else { return }
                if let data, !data.isEmpty { self.consume(data) }
                if let error { self.finish(reason: "receive error: \(error)"); return }
                if isComplete { self.close(reason: "peer closed"); return }
                if !self.closing { self.receiveLoop() }
            }
        }

        private func consume(_ data: Data) {
            guard let session else { return }
            // Bound before buffering: a chunk that cannot complete the pending
            // line inside the limit is refused without being appended, so the
            // receive buffer never holds more than `maxLineBytes`.
            let pendingBefore = splitter.pendingBytes
            if pendingBefore + data.count > maxLineBytes {
                let room = maxLineBytes - pendingBefore
                let head = room > 0 ? data.prefix(room) : data.prefix(0)
                if !head.contains(0x0A) {
                    refuse(.unterminatedLineTooLong(limit: maxLineBytes))
                    return
                }
            }
            let lines = splitter.append(data)
            reportProgress(completedLines: lines)
            for line in lines {
                if line.count + 1 > maxLineBytes {
                    refuse(.lineTooLong(bytes: line.count + 1, limit: maxLineBytes))
                    return
                }
                let disposition = session.handle(line: line) { [weak self] reply in
                    guard let self, let owner = self.owner else { return }
                    owner.onQueue { self.send(reply) }
                }
                if case .closeAfterFlush(let why) = disposition {
                    closeAfterFlush(reason: why)
                    return
                }
            }
            let pending = splitter.pendingBytes
            if pending >= maxLineBytes {
                refuse(.unterminatedLineTooLong(limit: maxLineBytes))
                return
            }
            // Frame deadline: armed by the first byte of a line, cleared by its
            // newline; a partial frame that outlives it is refused, however
            // slowly it is being fed. Before hello it never outlasts the hello
            // deadline by more than that deadline.
            if pending == 0 {
                if pendingBefore > 0 { disarm() }
                if identity == nil, deadline == nil {
                    arm(after: limits.helloTimeout) { [weak self] in
                        guard let self, self.identity == nil else { return }
                        self.refuse(.helloTimeout(seconds: self.limits.helloTimeout))
                    }
                }
            } else if pendingBefore == 0 || !lines.isEmpty {
                let seconds = identity == nil ? min(limits.frameTimeout, limits.helloTimeout) : limits.frameTimeout
                arm(after: seconds) { [weak self] in
                    guard let self else { return }
                    self.refuse(.frameTimeout(seconds: seconds, pendingBytes: self.splitter.pendingBytes))
                }
            }
        }

        /// Emits `.receiving` for completed lines and for an unterminated line
        /// that grew by at least `receivingReportInterval` since the last report.
        private func reportProgress(completedLines: [Data]) {
            guard let owner else { return }
            for line in completedLines {
                let size = line.count + 1
                if size >= NearbyListener.receivingReportInterval || reportedPending > 0 {
                    owner.emitEvent(.receiving(identity: identity, bytes: size, expected: nil))
                }
                reportedPending = 0
            }
            let pending = splitter.pendingBytes
            if pending >= reportedPending + NearbyListener.receivingReportInterval {
                reportedPending = pending
                owner.emitEvent(.receiving(identity: identity, bytes: pending, expected: nil))
            }
        }

        private func send(_ data: Data) {
            guard !closing else { return }
            nw.send(content: data, completion: .contentProcessed { _ in })
        }

        /// Queues a FIN behind everything already sent, then cancels.
        private func closeAfterFlush(reason: String) {
            guard !closing else { return }
            closing = true
            disarm()
            nw.send(content: nil, contentContext: .finalMessage, isComplete: true,
                    completion: .contentProcessed { [weak self] _ in self?.nw.cancel() })
            reportClosed(reason)
        }

        func close(reason: String) {
            guard !closing else { return }
            closing = true
            disarm()
            nw.cancel()
            reportClosed(reason)
        }

        /// Sends one `error` line (no `id`: it answers no request) and then
        /// closes behind it, so the peer learns why before the FIN.
        func refuse(code: String, message: String, reason: String) {
            guard !closing else { return }
            send(NearbyV1.errorLine(id: nil, code: code, message: message))
            closeAfterFlush(reason: reason)
        }

        /// Terminal state from the stack: the table entry is released only here,
        /// so the NWConnection outlives any FIN still being flushed.
        private func finish(reason: String) {
            if !closing {
                closing = true
                reportClosed(reason)
            }
            disarm()
            session?.end()
            owner?.forget(self)
        }

        private func reportClosed(_ reason: String) {
            owner?.emitEvent(.connectionClosed(identity: identity, reason: reason))
        }
    }
}
