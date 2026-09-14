import Foundation
import Network

public enum NearbyError: Error, CustomStringConvertible, Equatable {
    case browseFailed(String)
    case noMatchingMac(String)
    case unsupportedService(String)
    /// TCP/DNS never got as far as TLS — connection refused, no route, path
    /// unsatisfied, name resolution failed. The Mac may simply be restarting:
    /// retryable with backoff.
    case unreachable(String)
    /// TLS never reached `.ready` — wrong or forgotten PSK, code expired on
    /// the Mac, or a plaintext/other listener. Terminal: re-pair.
    case handshakeFailed(String)
    /// The Mac closed (or the network dropped) while a request was pending.
    case closed(String)
    /// An `error` reply. `isClosing` is true for codes the Mac follows with a close.
    case remote(code: String, message: String)
    case protocolViolation(String)
    case timeout(String)
    case invalidInput(String)
    /// A client-side bound was hit (in-flight requests); nothing was sent.
    case overloaded(String)
    /// The Mac no longer reports the destination a capture was built for
    /// (unpinned or re-pinned since). Terminal: reselect on the Mac and build
    /// a new capture (transfer-v1 "stale/invalid destination").
    case destinationChanged(captureDestination: String, current: String?)
    /// The bounded retry budget ran out; `last` is the final retryable failure.
    case attemptsExhausted(attempts: Int, last: String)
    /// The awaiting task was cancelled.
    case cancelled

    public var description: String {
        switch self {
        case .browseFailed(let s): return "Bonjour browse failed: \(s)"
        case .noMatchingMac(let s): return "no matching Mac: \(s)"
        case .unsupportedService(let s): return "unsupported service: \(s)"
        case .unreachable(let s): return "Mac unreachable: \(s)"
        case .handshakeFailed(let s): return "TLS-PSK handshake failed: \(s)"
        case .closed(let s): return "connection closed: \(s)"
        case .remote(let c, let m): return "Mac replied error \(c): \(m)"
        case .protocolViolation(let s): return "protocol violation: \(s)"
        case .timeout(let s): return "timed out: \(s)"
        case .invalidInput(let s): return "invalid input: \(s)"
        case .overloaded(let s): return "client overloaded: \(s)"
        case .destinationChanged(let d, let c): return "destination changed: capture targets \(d) but the Mac now reports \(c ?? "nothing pinned")"
        case .attemptsExhausted(let n, let last): return "gave up after \(n) attempt\(n == 1 ? "" : "s"); last failure: \(last)"
        case .cancelled: return "cancelled"
        }
    }

    /// True when the Mac closes after this error (§8): re-pair or fix input.
    public var isClosing: Bool {
        if case .remote(let code, _) = self { return NearbyWire.closingErrorCodes.contains(code) }
        return false
    }

    /// Failures a bounded reconnect may retry: the Mac was not there, the
    /// connection dropped, a reply did not arrive, or the Mac asked for a
    /// bounded wait (`too_many_in_flight` / `inbox_full`: session stays open;
    /// `too_many_sessions`: close older connections, reconnect). Everything
    /// else is terminal — a refused key, any other `error` reply, bad input, a
    /// protocol violation — and retrying would only repeat it (proposal §7.5).
    public var isRetryable: Bool {
        switch self {
        case .unreachable, .closed, .timeout, .browseFailed, .noMatchingMac: return true
        case .remote(let code, _): return NearbyWire.backpressureErrorCodes.contains(code) || code == "too_many_sessions"
        case .handshakeFailed, .protocolViolation, .invalidInput, .unsupportedService,
             .overloaded, .destinationChanged, .attemptsExhausted, .cancelled: return false
        }
    }

    /// Backpressure: the Mac kept the session open and will accept the same
    /// capture once its outstanding acknowledgements have gone out.
    public var isBackpressure: Bool {
        if case .remote(let code, _) = self { return NearbyWire.backpressureErrorCodes.contains(code) }
        return false
    }

    /// Terminal because the pairing itself was refused: the user must re-pair.
    /// `capture_not_permitted` is deliberately not here: the pairing is intact,
    /// the Mac's user set this companion to view-only (`needsPermission`).
    public var needsRepair: Bool {
        switch self {
        case .handshakeFailed: return true
        case .remote(let code, _): return code == "pair_mismatch" || code == "pairing_expired"
        default: return false
        }
    }

    /// Terminal for now, without re-pairing or a new capture: the Mac's user
    /// set this companion to view-only. Session stays open; the same capture
    /// is accepted once the permission is changed in Nearby Companion.
    public var needsPermission: Bool {
        if case .remote(let code, _) = self { return NearbyWire.permissionErrorCodes.contains(code) }
        return false
    }

    /// Terminal because the capture itself was refused (image, id, revision,
    /// or a destination the Mac no longer holds — reported before sending or
    /// refused by its bridge): a retry with the same bytes repeats it; build a
    /// new capture.
    public var needsNewCapture: Bool {
        switch self {
        case .remote(let code, _): return NearbyWire.captureInputErrorCodes.contains(code)
        case .invalidInput, .destinationChanged: return true
        default: return false
        }
    }

    /// The destination this capture was built against is not what the Mac
    /// holds now (reported by the Mac before sending, or refused by its
    /// bridge): reselect on the Mac and re-read `hello_ack.destination`
    /// before building the new capture.
    public var needsNewDestination: Bool {
        switch self {
        case .destinationChanged: return true
        case .remote(let code, _): return NearbyWire.destinationErrorCodes.contains(code)
        default: return false
        }
    }
}

/// One TLS-PSK connection to a Mac carrying runtime-v1 JSON Lines
/// (proposal §3–§4). Requests are matched to replies by envelope `id`.
/// All Network.framework callbacks and state live on `queue`.
///
/// Bounds (no unbounded buffers): at most `maxInFlightRequests` awaiting
/// replies (`overloaded` beyond that, nothing sent); an inbound line longer
/// than `maxInboundLineBytes` closes the connection (the Mac's replies are a
/// few hundred bytes, so this is far below the 12 MiB wire bound); every
/// timer is cancelled as soon as its request completes.
public final class NearbyConnection: @unchecked Sendable { // all mutable state is confined to `queue`
    public enum Direction { case sent, received }

    public let pairId: String
    public let psk: Data
    public let endpoint: NWEndpoint
    /// Bonjour service endpoints are resolved to host:port before dialing so a
    /// refused handshake is reported instead of hanging in `.preparing`
    /// (see NearbyResolver). Set false to dial the service endpoint directly.
    public var resolveServiceEndpoints = true
    /// The endpoint actually dialed (after resolution).
    public private(set) var dialed: NWEndpoint?
    /// Requests awaiting a reply beyond this fail at once with `overloaded`.
    public var maxInFlightRequests = 8
    /// An inbound line (or unterminated tail) longer than this closes the connection.
    public var maxInboundLineBytes = 1 << 20
    private var nw: NWConnection!
    private let queue: DispatchQueue
    private var splitter = LineSplitter()
    private struct Pending { let type: String; let timer: DispatchWorkItem; let resume: (Result<Data, Error>) -> Void }
    private var pending: [String: Pending] = [:] {
        didSet {
            closedLock.withLock { pendingSnapshot = pending.count }
            if pending.isEmpty, !idleWaiters.isEmpty { let w = idleWaiters; idleWaiters.removeAll(); for i in w { i.timer.cancel(); i.resume(.success(())) } }
        }
    }
    private var pendingSnapshot = 0
    /// Requests awaiting a reply right now. Safe from any thread.
    public var pendingRequestCount: Int { closedLock.withLock { pendingSnapshot } }
    private struct IdleWaiter { let serial: Int; let timer: DispatchWorkItem; let resume: (Result<Void, Error>) -> Void }
    private var idleWaiterSerial = 0
    private var idleWaiters: [IdleWaiter] = []
    private var connectWaiter: ((Result<Void, Error>) -> Void)?
    private var connectTimer: DispatchWorkItem?
    private var lastWaitingError: NWError?
    private var isReady = false
    private var closedReason: String?
    private let closedLock = NSLock()
    private var closedSnapshot: String?
    /// Observes every line both ways (the CLI's `-v`). Called on `queue`.
    public var onLine: ((Direction, Data) -> Void)?
    /// Called once, on `queue`, when the connection ends for any reason.
    public var onClose: ((String) -> Void)?
    public private(set) var negotiated: (tlsv12: Bool, suite: UInt16)?

    public init(endpoint: NWEndpoint, pairId: String, psk: Data,
                queue: DispatchQueue = DispatchQueue(label: "flashtex.nearby.connection")) {
        self.pairId = pairId
        self.psk = psk
        self.queue = queue
        self.endpoint = endpoint
    }

    public convenience init(host: String, port: UInt16, pairId: String, psk: Data,
                            queue: DispatchQueue = DispatchQueue(label: "flashtex.nearby.connection")) {
        self.init(endpoint: .hostPort(host: NWEndpoint.Host(host), port: NWEndpoint.Port(rawValue: port)!),
                  pairId: pairId, psk: psk, queue: queue)
    }

    /// Why the connection ended, or nil while it is usable. Safe from any thread.
    public var closeReason: String? { closedLock.withLock { closedSnapshot } }
    public var isOpen: Bool { closeReason == nil }

    // MARK: lifecycle

    /// Starts the connection and returns once TLS is `.ready` (the PSK
    /// handshake succeeded). Throws `unreachable` (TCP/DNS refused the dial),
    /// `handshakeFailed` (the key was refused) or `timeout`.
    public func connect(timeout: TimeInterval = 10) async throws {
        var target = endpoint
        if resolveServiceEndpoints, case .service = endpoint {
            do { target = try await NearbyResolver.resolve(endpoint, timeout: timeout).endpoint } catch let e as NearbyError {
                // A vanished service is "not there", not a refused key.
                if case .browseFailed(let why) = e { throw NearbyError.unreachable(why) }
                throw e
            }
        }
        let dial = target
        try await withCheckedThrowingContinuation { (cont: CheckedContinuation<Void, Error>) in
            queue.async {
                guard self.nw == nil else { cont.resume(throwing: NearbyError.invalidInput("connect() called twice")); return }
                if let why = self.closedReason { cont.resume(throwing: NearbyError.closed(why)); return }
                self.dialed = dial
                self.nw = NWConnection(to: dial, using: NearbyCrypto.parameters(pairId: self.pairId, psk: self.psk))
                self.connectWaiter = { cont.resume(with: $0) }
                self.nw.stateUpdateHandler = { [weak self] state in self?.stateChanged(state) }
                let timer = DispatchWorkItem { [weak self] in
                    guard let self, let w = self.connectWaiter else { return }
                    self.connectWaiter = nil
                    let why = self.lastWaitingError.map { "waiting: \($0)" } ?? "no TLS handshake within \(timeout)s"
                    w(.failure(NearbyError.timeout(why)))
                    self.finish(reason: why)
                    self.nw.cancel()
                }
                self.connectTimer = timer
                self.nw.start(queue: self.queue)
                self.queue.asyncAfter(deadline: .now() + timeout, execute: timer)
            }
        }
    }

    public func close() {
        queue.async { self.finish(reason: "closed by client"); self.nw?.cancel() }
    }

    private func failConnect(_ error: NearbyError, reason: String) {
        guard let w = connectWaiter else { return }
        connectWaiter = nil
        w(.failure(error))
        finish(reason: reason)
        nw.cancel()
    }

    private func stateChanged(_ state: NWConnection.State) {
        switch state {
        case .ready:
            isReady = true
            connectTimer?.cancel(); connectTimer = nil
            negotiated = NearbyCrypto.negotiated(nw)
            if let w = connectWaiter { connectWaiter = nil; w(.success(())) }
            receiveLoop()
        case .waiting(let error):
            // Network.framework parks a connection here and retries on its own
            // schedule. A companion with a bounded retry budget wants the
            // verdict now: a TLS error is a refused key (terminal), anything
            // else (refused port, no route, unsatisfied path, DNS) is
            // "unreachable" and the caller's backoff decides when to try again.
            lastWaitingError = error
            guard connectWaiter != nil else { return }
            if case .tls = error {
                failConnect(.handshakeFailed(String(describing: error)), reason: "handshake failed: \(error)")
            } else {
                failConnect(.unreachable(String(describing: error)), reason: "unreachable: \(error)")
            }
        case .failed(let error):
            if connectWaiter != nil {
                if case .tls = error {
                    failConnect(.handshakeFailed(String(describing: error)), reason: "handshake failed: \(error)")
                } else if case .dns = error {
                    failConnect(.unreachable(String(describing: error)), reason: "unreachable: \(error)")
                } else if case .posix(let code) = error, code == .ECONNREFUSED || code == .EHOSTUNREACH || code == .ENETUNREACH || code == .ENETDOWN {
                    failConnect(.unreachable(String(describing: error)), reason: "unreachable: \(error)")
                } else {
                    // A reset during the handshake is how a wrong key surfaces on
                    // some paths; without TLS metadata it stays classified as refused.
                    failConnect(.handshakeFailed(String(describing: error)), reason: "handshake failed: \(error)")
                }
                return
            }
            finish(reason: isReady ? "failed: \(error)" : "handshake failed: \(error)")
            nw.cancel()
        case .cancelled:
            finish(reason: "cancelled")
        default: break
        }
    }

    private func finish(reason: String) {
        guard closedReason == nil else { return }
        closedReason = reason
        closedLock.withLock { closedSnapshot = reason }
        connectTimer?.cancel(); connectTimer = nil
        let waiting = pending
        let idle = idleWaiters
        idleWaiters.removeAll()
        pending.removeAll()
        for (_, p) in waiting { p.timer.cancel(); p.resume(.failure(NearbyError.closed(reason))) }
        for i in idle { i.timer.cancel(); i.resume(.failure(NearbyError.closed(reason))) }
        onClose?(reason)
    }

    /// Resolves once no request awaits a reply (backpressure recovery: the
    /// Mac answered `too_many_in_flight`/`inbox_full` and wants the companion
    /// to wait for its outstanding acknowledgements). Throws `timeout` after
    /// `timeout`, `closed` if the connection ends first.
    public func waitUntilIdle(timeout: TimeInterval = 30) async throws {
        try await withCheckedThrowingContinuation { (cont: CheckedContinuation<Void, Error>) in
            queue.async {
                if let why = self.closedReason { cont.resume(throwing: NearbyError.closed(why)); return }
                if self.pending.isEmpty { cont.resume(returning: ()); return }
                self.idleWaiterSerial += 1
                let serial = self.idleWaiterSerial
                let timer = DispatchWorkItem { [weak self] in
                    guard let self, let i = self.idleWaiters.firstIndex(where: { $0.serial == serial }) else { return }
                    let w = self.idleWaiters.remove(at: i)
                    w.resume(.failure(NearbyError.timeout("\(self.pending.count) requests still awaiting a reply after \(timeout)s")))
                }
                self.idleWaiters.append(IdleWaiter(serial: serial, timer: timer, resume: { cont.resume(with: $0) }))
                self.queue.asyncAfter(deadline: .now() + timeout, execute: timer)
            }
        }
    }

    private func receiveLoop() {
        nw.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { [weak self] data, _, isComplete, error in
            guard let self, self.closedReason == nil else { return }
            if let data, !data.isEmpty {
                for line in self.splitter.append(data) {
                    if line.count >= self.maxInboundLineBytes {
                        self.finish(reason: "peer sent an oversized line (\(line.count) bytes)"); self.nw.cancel(); return
                    }
                    self.handle(line: line)
                    if self.closedReason != nil { return }
                }
                if self.splitter.pendingBytes >= self.maxInboundLineBytes {
                    self.finish(reason: "peer sent an oversized unterminated line"); self.nw.cancel(); return
                }
            }
            if let error { self.finish(reason: "receive error: \(error)"); self.nw.cancel(); return }
            if isComplete { self.finish(reason: "peer closed"); self.nw.cancel(); return }
            self.receiveLoop()
        }
    }

    private func take(_ id: String) -> Pending? {
        guard let p = pending.removeValue(forKey: id) else { return nil }
        p.timer.cancel()
        return p
    }

    private func handle(line: Data) {
        onLine?(.received, line)
        guard let header = try? NearbyWire.header(of: line) else {
            finish(reason: "undecodable line from Mac"); nw.cancel(); return
        }
        if header.type == "error" {
            let payload = (try? JSONDecoder().decode(NearbyWire.ErrorLine.self, from: line))?.payload
                ?? .init(code: "unknown", message: String(decoding: line, as: UTF8.self))
            let err = NearbyError.remote(code: payload.code, message: payload.message)
            if let id = header.id, let p = take(id) {
                p.resume(.failure(err))
            } else {
                // `id: null` (line_too_long, undecodable envelope): nothing can be
                // attributed, and the Mac closes next.
                let waiting = pending
                pending.removeAll()
                for (_, p) in waiting { p.timer.cancel(); p.resume(.failure(err)) }
            }
            return
        }
        guard let id = header.id, let p = take(id) else { return } // unsolicited: ignored
        guard header.type == p.type else {
            p.resume(.failure(NearbyError.protocolViolation("expected \(p.type) for \(id), got \(header.type)")))
            return
        }
        p.resume(.success(line))
    }

    // MARK: requests

    /// Sends one envelope and waits for the reply with the same `id` and
    /// `replyType`, or an `error` reply, or the connection closing, or
    /// `timeout`. Fails with `overloaded` (nothing sent) beyond
    /// `maxInFlightRequests` outstanding requests.
    public func request<Req: Codable, Rep: Codable>(type: String, _ payload: Req, expecting replyType: String,
                                                    id: String? = nil, timeout: TimeInterval = 30) async throws -> NearbyWire.Envelope<Rep> {
        let reqID = id ?? "c-" + UUID().uuidString.lowercased()
        let line = try NearbyWire.line(id: reqID, type: type, payload)
        let reply: Data = try await withCheckedThrowingContinuation { cont in
            queue.async {
                if let why = self.closedReason { cont.resume(throwing: NearbyError.closed(why)); return }
                guard self.nw != nil, self.isReady else { cont.resume(throwing: NearbyError.closed("connect() did not complete")); return }
                guard self.pending[reqID] == nil else { cont.resume(throwing: NearbyError.invalidInput("request id \(reqID) is already in flight")); return }
                guard self.pending.count < self.maxInFlightRequests else {
                    cont.resume(throwing: NearbyError.overloaded("\(self.pending.count) requests already await a reply (limit \(self.maxInFlightRequests))"))
                    return
                }
                let timer = DispatchWorkItem { [weak self] in
                    guard let self, let p = self.take(reqID) else { return }
                    p.resume(.failure(NearbyError.timeout("no \(replyType) for \(type) \(reqID) within \(timeout)s")))
                }
                self.pending[reqID] = Pending(type: replyType, timer: timer, resume: { cont.resume(with: $0) })
                self.onLine?(.sent, line)
                self.nw.send(content: line, completion: .contentProcessed { [weak self] error in
                    guard let self, let error, let p = self.take(reqID) else { return }
                    p.resume(.failure(NearbyError.closed("send failed: \(error)")))
                })
                self.queue.asyncAfter(deadline: .now() + timeout, execute: timer)
            }
        }
        return try NearbyWire.decode(reply, as: Rep.self)
    }

    /// `hello` (must be the first request). Verifies the echoed nonce; when
    /// `expectPairPsk` (bootstrap connection) the ack must carry a 32-byte key.
    @discardableResult
    public func hello(companionName: String, nonce: String = UUID().uuidString, expectPairPsk: Bool = false,
                      timeout: TimeInterval = 30) async throws -> NearbyWire.HelloAck {
        let h = NearbyWire.Hello(pairId: pairId, companionName: companionName, nonce: nonce,
                                 proof: NearbyCrypto.helloProof(psk: psk, nonce: nonce))
        let ack: NearbyWire.Envelope<NearbyWire.HelloAck> = try await request(type: "hello", h, expecting: "hello_ack", timeout: timeout)
        guard ack.payload.nonce == nonce else {
            throw NearbyError.protocolViolation("hello_ack nonce \(ack.payload.nonce) != sent \(nonce)")
        }
        if expectPairPsk {
            guard let b64 = ack.payload.pairPsk, let key = Data(base64Encoded: b64), key.count == NearbyCrypto.pskLength else {
                throw NearbyError.protocolViolation("bootstrap hello_ack without a 32-byte pair_psk")
            }
        }
        return ack.payload
    }

    public func destinationQuery(timeout: TimeInterval = 30) async throws -> NearbyWire.Destination? {
        let r: NearbyWire.Envelope<NearbyWire.DestinationReply> =
            try await request(type: "destination_query", NearbyWire.Empty(), expecting: "destination", timeout: timeout)
        return r.payload.destination
    }

    /// `capture_submit` → `capture_received`. Retry after a disconnect with the
    /// *same* `capture_id` and payload; the Mac de-duplicates. With
    /// `validateImage` (default) the cheap checks the Mac would fail anyway —
    /// size ≤ 8 MiB, signature matches `mime_type`, valid base64 — are refused
    /// here as `invalidInput` before a byte is sent; pass false to exercise the
    /// Mac's `image_too_large` / `invalid_image` replies.
    public func submitCapture(_ capture: NearbyWire.CaptureSubmit, requestID: String? = nil,
                              timeout: TimeInterval = 60, validateImage: Bool = true) async throws -> NearbyWire.CaptureReceived {
        guard NearbyWire.isValidID(capture.captureId) else { throw NearbyError.invalidInput("capture_id must be 1–128 ASCII [A-Za-z0-9_-]") }
        guard NearbyWire.isValidID(capture.destinationId) else { throw NearbyError.invalidInput("destination_id must be 1–128 ASCII [A-Za-z0-9_-]") }
        guard NearbyWire.acceptedMimeTypes.contains(capture.image.mimeType) else { throw NearbyError.invalidInput("mime_type must be image/png or image/jpeg") }
        guard capture.instructions.utf8.count <= 4096 else { throw NearbyError.invalidInput("instructions exceed 4096 bytes") }
        guard capture.baseRevision >= 0 else { throw NearbyError.invalidInput("base_revision must be non-negative") }
        if validateImage {
            guard let bytes = Data(base64Encoded: capture.image.dataBase64) else { throw NearbyError.invalidInput("image data_base64 is not valid base64") }
            if let why = NearbyWire.checkImage(bytes, mimeType: capture.image.mimeType) { throw NearbyError.invalidInput(why) }
        }
        let r: NearbyWire.Envelope<NearbyWire.CaptureReceived> =
            try await request(type: "capture_submit", capture, expecting: "capture_received", id: requestID, timeout: timeout)
        return r.payload
    }

    /// `capture_status` → `capture_status_ack` (additive; nearby-v1 §4). A
    /// status probe, never a re-delivery: the Mac answers from its bridge /
    /// inbox for a capture it acknowledged on this pairing.
    public func captureStatus(captureId: String, requestID: String? = nil, timeout: TimeInterval = 30) async throws -> NearbyWire.CaptureStatus {
        guard NearbyWire.isValidID(captureId) else { throw NearbyError.invalidInput("capture_id must be 1–128 ASCII [A-Za-z0-9_-]") }
        let r: NearbyWire.Envelope<NearbyWire.CaptureStatus> =
            try await request(type: "capture_status", NearbyWire.CaptureStatusRequest(captureId: captureId),
                              expecting: "capture_status_ack", id: requestID, timeout: timeout)
        guard r.payload.captureId == captureId else {
            throw NearbyError.protocolViolation("capture_status_ack for \(captureId) names capture \(r.payload.captureId)")
        }
        return r.payload
    }

    /// nearby-v1 `capture_insert` (additive): approve the proposal this
    /// companion was shown and ask the Mac to apply it.
    ///
    /// `approvedLatex` must be the exact text the person read — normally
    /// `capture_status.latex` as it was rendered on screen. It is hashed, not
    /// sent: the Mac inserts *its* proposal, and only when its proposal is
    /// still the one that hash names. A Mac that predates the message answers
    /// `unknown_type`; one whose proposal moved on answers `proposal_changed`.
    public func captureInsert(captureId: String, approvedLatex: String,
                              requestID: String? = nil, timeout: TimeInterval = 60) async throws -> NearbyWire.CaptureInsertAck {
        guard NearbyWire.isValidID(captureId) else { throw NearbyError.invalidInput("capture_id must be 1–128 ASCII [A-Za-z0-9_-]") }
        let request = NearbyWire.CaptureInsertRequest(captureId: captureId,
                                                      approvedLatexSha256: NearbyWire.proposalDigest(approvedLatex))
        let r: NearbyWire.Envelope<NearbyWire.CaptureInsertAck> =
            try await self.request(type: "capture_insert", request, expecting: "capture_insert_ack", id: requestID, timeout: timeout)
        guard r.payload.captureId == captureId else {
            throw NearbyError.protocolViolation("capture_insert_ack for \(captureId) names capture \(r.payload.captureId)")
        }
        return r.payload
    }
}
