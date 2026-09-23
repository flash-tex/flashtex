import CoreGraphics
import ImageIO
import Network
import UniformTypeIdentifiers
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Thrown by a `waitUntil` helper on timeout, after it has already recorded an `XCTFail`,
/// so callers halt instead of proceeding to index data that never arrived (issue #773).
private struct NearbyWaitTimedOut: Error {}

// MARK: - test doubles

final class RecordingSink: CaptureSink {
    private let lock = NSLock()
    private(set) var envelopes: [RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>] = []
    var durable = false
    func submit(_ envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>, reply: @escaping (Data) -> Void) {
        lock.withLock { envelopes.append(envelope) }
        let ack = NearbyV1.CaptureReceived(captureId: envelope.payload.captureId, durable: durable, hasProposal: false, applied: false)
        reply(NearbyV1.line(id: envelope.id, type: "capture_received", ack))
    }
    var count: Int { lock.withLock { envelopes.count } }
}

final class FixedDestinations: DestinationProvider {
    var destination: NearbyV1.Destination?
    init(_ d: NearbyV1.Destination?) { destination = d }
    func currentDestination(_ reply: @escaping (NearbyV1.Destination?) -> Void) { reply(destination) }
}

/// Companion-side client: NWConnection with the same TLS-PSK parameters,
/// JSON Lines framing, and a queue of received lines.
final class NearbyTestClient {
    let connection: NWConnection
    private let queue = DispatchQueue(label: "nearby.test.client")
    private var splitter = LineSplitter()
    private let lock = NSLock()
    private var lines: [Data] = []
    private var waiters: [(Int, XCTestExpectation)] = []
    let ready = XCTestExpectation(description: "client ready")
    let failed = XCTestExpectation(description: "client failed")
    let closed = XCTestExpectation(description: "client closed")
    private(set) var failure: NWError?
    // Flags for async (main-actor) tests, which must not block on XCTWaiter.
    private(set) var isReady = false
    private(set) var isFailed = false
    private(set) var isClosed = false

    init(port: UInt16, identity: String, psk: Data) {
        connection = NWConnection(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!,
                                  using: NearbyListener.clientParameters(identity: identity, psk: psk))
        connection.stateUpdateHandler = { [weak self] state in
            guard let self else { return }
            switch state {
            case .ready: self.isReady = true; self.ready.fulfill(); self.receiveLoop()
            case .failed(let e): self.failure = e; self.isFailed = true; self.isClosed = true; self.failed.fulfill(); self.closed.fulfill()
            case .waiting(let e): self.failure = e; self.isFailed = true; self.failed.fulfill()
            case .cancelled: self.isClosed = true; self.closed.fulfill()
            default: break
            }
        }
        connection.start(queue: queue)
    }

    private func receiveLoop() {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { [weak self] data, _, complete, error in
            guard let self else { return }
            if let data, !data.isEmpty {
                let new = self.splitter.append(data)
                self.lock.withLock {
                    self.lines.append(contentsOf: new)
                    self.waiters.removeAll { n, exp in
                        if self.lines.count >= n { exp.fulfill(); return true }
                        return false
                    }
                }
            }
            if complete || error != nil { self.isClosed = true; self.closed.fulfill(); return }
            self.receiveLoop()
        }
    }

    func send(_ data: Data) {
        connection.send(content: data, completion: .contentProcessed { _ in })
    }

    func send<P: Codable>(id: String, type: String, _ payload: P) {
        send(NearbyV1.line(id: id, type: type, payload))
    }

    struct TimedOut: Error {}

    /// Waits until at least `count` lines have arrived; returns them all.
    /// Throws (after recording a failure) rather than returning a short array, so a
    /// caller that indexes the result can't trap on an array that never reached `count`.
    @discardableResult
    func lines(atLeast count: Int, timeout: TimeInterval = 5, file: StaticString = #filePath, line: UInt = #line) throws -> [Data] {
        let exp = XCTestExpectation(description: "\(count) lines")
        lock.withLock {
            if lines.count >= count { exp.fulfill() } else { waiters.append((count, exp)) }
        }
        if XCTWaiter.wait(for: [exp], timeout: timeout) != .completed {
            XCTFail("timed out waiting for \(count) lines (have \(lock.withLock { lines.count }))", file: file, line: line)
            throw TimedOut()
        }
        return lock.withLock { lines }
    }

    var lineCount: Int { lock.withLock { lines.count } }
    var allLines: [Data] { lock.withLock { lines } }

    func cancel() { connection.cancel() }
}

/// Listener wrapper that collects events and waits for `.ready`.
final class ListenerHarness {
    let listener: NearbyListener
    private let lock = NSLock()
    private(set) var events: [NearbyListener.Event] = []
    let ready = XCTestExpectation(description: "listener ready")
    private(set) var port: UInt16 = 0

    init(psks: [NearbyListener.PSKEntry], sink: CaptureSink?, destinations: DestinationProvider?,
         pairing: PairingConfirmer? = nil, advertise: NearbyListener.Advertisement? = nil,
         limits: NearbyReceiveLimits = .init(), port: UInt16? = nil, queue: DispatchQueue? = nil) {
        let config = NearbyListener.Configuration(psks: psks, macName: "Test Mac", port: port, advertisement: advertise,
                                                  loopbackOnly: advertise == nil, limits: limits)
        var capture: ((NearbyListener.Event) -> Void)!
        let box = EventBox()
        capture = { box.handler?($0) }
        listener = NearbyListener(configuration: config, sink: sink, destinations: destinations, pairing: pairing,
                                  queue: queue ?? DispatchQueue(label: "nearby.test.listener"), events: capture)
        box.handler = { [weak self] e in
            guard let self else { return }
            self.lock.withLock { self.events.append(e) }
            if case .ready(let p) = e { self.port = p; self.ready.fulfill() }
        }
    }
    private final class EventBox { var handler: ((NearbyListener.Event) -> Void)? }

    func start(file: StaticString = #filePath, line: UInt = #line) throws {
        try listener.start()
        if XCTWaiter.wait(for: [ready], timeout: 5) != .completed {
            XCTFail("listener never became ready: \(snapshot)", file: file, line: line)
        }
    }
    var snapshot: [NearbyListener.Event] { lock.withLock { events } }
    func stop() { listener.stop() }
}

// MARK: - tests

final class NearbyListenerTests: XCTestCase {
    static let pskA = Data(repeating: 0xA5, count: 32)
    static let pskB = Data(repeating: 0x5A, count: 32)
    static let fixtureURL = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("protocol/fixtures/capture-submission.json")

    func hello(_ client: NearbyTestClient, pairId: String, psk: Data, nonce: String = UUID().uuidString) {
        client.send(id: "h1", type: "hello", NearbyV1.Hello(pairId: pairId, companionName: "Test iPad", nonce: nonce,
                                                            proof: Pairing.helloProof(psk: psk, nonce: nonce)))
    }

    func decode<P: Codable>(_ line: Data, as: P.Type = P.self) throws -> RuntimeV1.Envelope<P> {
        try JSONDecoder().decode(RuntimeV1.Envelope<P>.self, from: line)
    }

    /// `error` envelopes may carry `"id": null` (transfer-v1) for unidentifiable requests.
    struct ErrorLine: Decodable {
        var protocolVersion: Int, id: String?, type: String, payload: NearbyV1.ErrorPayload
        enum CodingKeys: String, CodingKey { case protocolVersion = "protocol_version", id, type, payload }
    }
    func decodeError(_ line: Data) throws -> ErrorLine {
        let e = try JSONDecoder().decode(ErrorLine.self, from: line)
        XCTAssertEqual(e.type, "error")
        return e
    }

    func testHelloCaptureAndDestinationRoundTrip() throws {
        let sink = RecordingSink()
        let dest = FixedDestinations(.init(destinationId: "mac-anchor-1", projectId: "demo", path: "main.tex", baseRevision: 3))
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: Self.pskA, isBootstrap: false)], sink: sink, destinations: dest)
        try h.start()
        defer { h.stop() }

        let client = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskA)
        XCTAssertEqual(XCTWaiter.wait(for: [client.ready], timeout: 5), .completed, "\(client.failure.map(String.init(describing:)) ?? "no error")")
        hello(client, pairId: "pair-a", psk: Self.pskA, nonce: "n-1")
        var lines = try client.lines(atLeast: 1)
        let ack: RuntimeV1.Envelope<NearbyV1.HelloAck> = try decode(lines[0])
        XCTAssertEqual(ack.type, "hello_ack")
        XCTAssertEqual(ack.id, "h1")
        XCTAssertEqual(ack.payload.macName, "Test Mac")
        XCTAssertEqual(ack.payload.nonce, "n-1")
        XCTAssertEqual(ack.payload.destination?.destinationId, "mac-anchor-1")
        XCTAssertEqual(ack.payload.destination?.baseRevision, 3)
        XCTAssertNil(ack.payload.pairPsk, "long-term connections never receive a new PSK")

        // The exact fixture line, as the companion produces it.
        var fixture = try Data(contentsOf: Self.fixtureURL)
        while fixture.last == 0x0A { fixture.removeLast() }
        fixture.append(0x0A)
        client.send(fixture)
        lines = try client.lines(atLeast: 2)
        let received: RuntimeV1.Envelope<NearbyV1.CaptureReceived> = try decode(lines[1])
        XCTAssertEqual(received.type, "capture_received")
        XCTAssertEqual(received.id, "fixture-capture-request")
        XCTAssertEqual(received.payload, .init(captureId: "fixture-capture-1", durable: false, hasProposal: false, applied: false))
        XCTAssertEqual(sink.count, 1)
        XCTAssertEqual(sink.envelopes.first?.payload.destinationId, "fixture-anchor-1")

        dest.destination = nil
        client.send(id: "q1", type: "destination_query", NearbyV1.Empty())
        lines = try client.lines(atLeast: 3)
        let q: RuntimeV1.Envelope<NearbyV1.DestinationReply> = try decode(lines[2])
        XCTAssertEqual(q.type, "destination")
        XCTAssertNil(q.payload.destination)
        XCTAssertTrue(String(decoding: lines[2], as: UTF8.self).contains("\"destination\":null"), "null must be explicit")

        // Unknown types get an error but keep the session open.
        client.send(id: "x1", type: "bogus", NearbyV1.Empty())
        lines = try client.lines(atLeast: 4)
        let err: RuntimeV1.Envelope<NearbyV1.ErrorPayload> = try decode(lines[3])
        XCTAssertEqual(err.payload.code, "unknown_type")
        client.send(id: "q2", type: "destination_query", NearbyV1.Empty())
        XCTAssertEqual(try client.lines(atLeast: 5).count, 5)

        let events = h.snapshot
        XCTAssertTrue(events.contains(.connectionOpened), "\(events)")
        XCTAssertTrue(events.contains(.hello(pairId: "pair-a", companionName: "Test iPad", bootstrap: false)))
        XCTAssertTrue(events.contains(.capture(captureId: "fixture-capture-1")))
        client.cancel()
    }

    func testWrongPSKIsRefusedBeforeAnyLineIsParsed() throws {
        let sink = RecordingSink()
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: Self.pskA, isBootstrap: false)], sink: sink, destinations: nil)
        try h.start()
        defer { h.stop() }

        // Right identity, wrong key.
        let wrongKey = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskB)
        XCTAssertEqual(XCTWaiter.wait(for: [wrongKey.failed], timeout: 5), .completed, "handshake should fail")
        XCTAssertEqual(XCTWaiter.wait(for: [wrongKey.ready], timeout: 0.5), .timedOut)
        // Unknown identity.
        let unknown = NearbyTestClient(port: h.port, identity: "nobody", psk: Self.pskA)
        XCTAssertEqual(XCTWaiter.wait(for: [unknown.failed], timeout: 5), .completed)
        XCTAssertEqual(XCTWaiter.wait(for: [unknown.ready], timeout: 0.5), .timedOut)

        XCTAssertEqual(sink.count, 0)
        let events = h.snapshot
        XCTAssertFalse(events.contains { if case .connectionOpened = $0 { return true }; return false },
                       "no session may exist for an unauthenticated peer: \(events)")
        XCTAssertFalse(events.contains { if case .hello = $0 { return true }; return false })
        wrongKey.cancel(); unknown.cancel()
    }

    func testOversizedLineClosesConnection() throws {
        let sink = RecordingSink()
        var limits = NearbyReceiveLimits()
        limits.maxLineBytes = 4096
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: Self.pskA, isBootstrap: false)], sink: sink,
                                destinations: nil, limits: limits)
        try h.start()
        defer { h.stop() }

        // A complete line over the limit.
        let c1 = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskA)
        XCTAssertEqual(XCTWaiter.wait(for: [c1.ready], timeout: 5), .completed)
        hello(c1, pairId: "pair-a", psk: Self.pskA)
        _ = try c1.lines(atLeast: 1)
        var big = Data(repeating: 0x20, count: 5000); big.append(0x0A)
        c1.send(big)
        let lines = try c1.lines(atLeast: 2)
        let err = try decodeError(lines[1])
        XCTAssertEqual(err.payload.code, "line_too_long")
        XCTAssertNil(err.id)
        XCTAssertEqual(XCTWaiter.wait(for: [c1.closed], timeout: 5), .completed, "connection must close after the error")

        // An unterminated line that grows past the limit.
        let c2 = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskA)
        XCTAssertEqual(XCTWaiter.wait(for: [c2.ready], timeout: 5), .completed)
        hello(c2, pairId: "pair-a", psk: Self.pskA)
        _ = try c2.lines(atLeast: 1)
        c2.send(Data(repeating: 0x7B, count: 5000))
        let lines2 = try c2.lines(atLeast: 2)
        let err2 = try decodeError(lines2[1])
        XCTAssertEqual(err2.payload.code, "line_too_long")
        XCTAssertEqual(XCTWaiter.wait(for: [c2.closed], timeout: 5), .completed)
        XCTAssertEqual(sink.count, 0)
    }

    func testMessagesBeforeHelloAndMismatchedHelloAreRefused() throws {
        let sink = RecordingSink()
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: Self.pskA, isBootstrap: false),
                                       .init(identity: "pair-b", key: Self.pskB, isBootstrap: false)],
                                sink: sink, destinations: nil)
        try h.start()
        defer { h.stop() }

        let c1 = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskA)
        XCTAssertEqual(XCTWaiter.wait(for: [c1.ready], timeout: 5), .completed)
        var fixture = try Data(contentsOf: Self.fixtureURL)
        if fixture.last != 0x0A { fixture.append(0x0A) }
        c1.send(fixture)
        let l1 = try c1.lines(atLeast: 1)
        guard !l1.isEmpty else { return XCTFail("events: \(h.snapshot)") }
        let e1: RuntimeV1.Envelope<NearbyV1.ErrorPayload> = try decode(l1[0])
        XCTAssertEqual(e1.payload.code, "hello_required")
        XCTAssertEqual(XCTWaiter.wait(for: [c1.closed], timeout: 5), .completed)
        XCTAssertEqual(sink.count, 0)

        // Authenticated as pair-b, claiming pair-a (proof made with its own key).
        let c2 = NearbyTestClient(port: h.port, identity: "pair-b", psk: Self.pskB)
        XCTAssertEqual(XCTWaiter.wait(for: [c2.ready], timeout: 5), .completed, "\(String(describing: c2.failure))")
        hello(c2, pairId: "pair-a", psk: Self.pskB)
        let e2: RuntimeV1.Envelope<NearbyV1.ErrorPayload> = try decode(c2.lines(atLeast: 1)[0])
        XCTAssertEqual(e2.payload.code, "pair_mismatch")
        XCTAssertEqual(XCTWaiter.wait(for: [c2.closed], timeout: 5), .completed)

        // Correct pair-b hello works, proving the server picked the right PSK.
        let c3 = NearbyTestClient(port: h.port, identity: "pair-b", psk: Self.pskB)
        XCTAssertEqual(XCTWaiter.wait(for: [c3.ready], timeout: 5), .completed)
        hello(c3, pairId: "pair-b", psk: Self.pskB, nonce: "same")
        let l3 = try c3.lines(atLeast: 1)
        guard !l3.isEmpty else { return XCTFail("events: \(h.snapshot)") }
        let ack: RuntimeV1.Envelope<NearbyV1.HelloAck> = try decode(l3[0])
        XCTAssertEqual(ack.type, "hello_ack")
        XCTAssertTrue(h.snapshot.contains(.hello(pairId: "pair-b", companionName: "Test iPad", bootstrap: false)))

        // Re-using a hello nonce on a new connection is refused.
        let c4 = NearbyTestClient(port: h.port, identity: "pair-b", psk: Self.pskB)
        XCTAssertEqual(XCTWaiter.wait(for: [c4.ready], timeout: 5), .completed)
        hello(c4, pairId: "pair-b", psk: Self.pskB, nonce: "same")
        let e4: RuntimeV1.Envelope<NearbyV1.ErrorPayload> = try decode(c4.lines(atLeast: 1)[0])
        XCTAssertEqual(e4.payload.code, "bad_request")
        XCTAssertEqual(XCTWaiter.wait(for: [c4.closed], timeout: 5), .completed)
        c3.cancel()
    }

    func testBootstrapPairingHandsOverLongTermPSKAndSurvivesRestart() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-\(UUID().uuidString)")
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        XCTAssertNil(store.loadError)
        let coordinator = PairingCoordinator(store: store)
        let pending = coordinator.begin(code: "123456")
        let bootstrap = try XCTUnwrap(coordinator.bootstrapEntry)
        XCTAssertTrue(bootstrap.isBootstrap)

        let sink = RecordingSink()
        let queue = DispatchQueue(label: "nearby.test.shared")
        let h1 = ListenerHarness(psks: [bootstrap], sink: sink, destinations: nil, pairing: coordinator, queue: queue)
        try h1.start()

        // Companion side: same derivation from the typed code and the TXT salt.
        let derived = Pairing.derive(code: "123456", salt: store.salt)
        XCTAssertEqual(derived, pending.derived)
        let client = NearbyTestClient(port: h1.port, identity: derived.pairId, psk: derived.psk)
        XCTAssertEqual(XCTWaiter.wait(for: [client.ready], timeout: 5), .completed, "\(String(describing: client.failure))")
        hello(client, pairId: derived.pairId, psk: derived.psk)
        let ack: RuntimeV1.Envelope<NearbyV1.HelloAck> = try decode(client.lines(atLeast: 1)[0])
        let longTerm = try XCTUnwrap(Data(base64Encoded: try XCTUnwrap(ack.payload.pairPsk)))
        XCTAssertEqual(longTerm.count, 32)
        XCTAssertNotEqual(longTerm, derived.psk)
        let record = try XCTUnwrap(store.pair(id: derived.pairId))
        XCTAssertEqual(record.pskData, longTerm)
        XCTAssertEqual(record.companionName, "Test iPad")
        XCTAssertNil(coordinator.current, "a confirmed code is consumed")
        XCTAssertTrue(h1.snapshot.contains(.hello(pairId: derived.pairId, companionName: "Test iPad", bootstrap: true)))

        // Restart with the long-term key only (what NearbyState does), same port,
        // adopting the live session.
        let h2 = ListenerHarness(psks: [.init(identity: derived.pairId, key: longTerm, isBootstrap: false)], sink: sink,
                                 destinations: nil, pairing: coordinator, port: h1.port, queue: queue)
        h2.listener.adoptConnections(from: h1.listener)
        let stopped = XCTestExpectation(description: "old listener released its port")
        h1.listener.stop(keepConnections: true) { stopped.fulfill() }
        XCTAssertEqual(XCTWaiter.wait(for: [stopped], timeout: 5), .completed)
        try h2.start()
        XCTAssertEqual(h2.port, h1.port)
        XCTAssertEqual(h2.listener.openConnectionCount, 1)

        // The adopted session keeps working…
        client.send(id: "q", type: "destination_query", NearbyV1.Empty())
        XCTAssertEqual(try client.lines(atLeast: 2).count, 2)
        // …the bootstrap key no longer authenticates…
        let stale = NearbyTestClient(port: h2.port, identity: derived.pairId, psk: derived.psk)
        XCTAssertEqual(XCTWaiter.wait(for: [stale.failed], timeout: 5), .completed)
        // …and the long-term key does, without another hand-over.
        let again = NearbyTestClient(port: h2.port, identity: derived.pairId, psk: longTerm)
        XCTAssertEqual(XCTWaiter.wait(for: [again.ready], timeout: 5), .completed, "\(String(describing: again.failure))")
        hello(again, pairId: derived.pairId, psk: longTerm)
        let ack2: RuntimeV1.Envelope<NearbyV1.HelloAck> = try decode(again.lines(atLeast: 1)[0])
        XCTAssertNil(ack2.payload.pairPsk)
        XCTAssertNotNil(store.pair(id: derived.pairId)?.lastSeenAt)

        // A second bootstrap attempt with an expired code is refused at hello.
        _ = coordinator.begin(code: "654321", lifetime: -1)
        XCTAssertNil(coordinator.bootstrapEntry)
        XCTAssertNil(coordinator.confirmPairing(pairId: Pairing.derive(code: "654321", salt: store.salt).pairId, companionName: "x", generation: nil))

        client.cancel(); again.cancel(); stale.cancel()
        h2.stop()
        try? FileManager.default.removeItem(at: dir)
    }

    func testBonjourAdvertisingReachesReady() throws {
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: Self.pskA, isBootstrap: false)], sink: nil,
                                destinations: nil, advertise: .init(name: "FlashTeX Test \(UUID().uuidString.prefix(6))",
                                                                    txt: ["v": "1", "name": "Test Mac", "fp": "0123456789abcdef", "salt": "00"]))
        try h.start()
        XCTAssertGreaterThan(h.port, 0)
        h.stop()
    }

    // MARK: pure session (no network)

    func testSessionRejectsUnsupportedVersionsAndDuplicateHello() {
        let key = NearbyListener.PSKEntry(identity: "p", key: NearbyListenerTests.pskA, isBootstrap: false)
        let session = NearbySession(keys: [key], macName: "M", sink: nil, destinations: nil, pairing: nil) { _ in }
        var out: [Data] = []
        let emit: (Data) -> Void = { out.append($0) }
        XCTAssertEqual(session.handle(line: Data("not json".utf8), emit: emit), .closeAfterFlush("undecodable envelope"))
        XCTAssertTrue(String(decoding: out[0], as: UTF8.self).contains("\"id\":null"))
        let v2 = Data("{\"protocol_version\":2,\"id\":\"a\",\"type\":\"hello\",\"payload\":{}}".utf8)
        XCTAssertEqual(session.handle(line: v2, emit: emit), .closeAfterFlush("unsupported protocol_version"))
        let proof = Pairing.helloProof(psk: key.key, nonce: "n")
        let old = NearbyV1.line(id: "h", type: "hello", NearbyV1.Hello(pairId: "p", companionName: "c", protocolVersion: 0, nonce: "n", proof: proof))
        XCTAssertEqual(session.handle(line: old.dropLast(), emit: emit), .closeAfterFlush("unsupported nearby version"))
        let badProof = NearbyV1.line(id: "h", type: "hello", NearbyV1.Hello(pairId: "p", companionName: "c", nonce: "n",
                                                                            proof: Pairing.helloProof(psk: key.key, nonce: "m")))
        XCTAssertEqual(session.handle(line: badProof.dropLast(), emit: emit), .closeAfterFlush("pair mismatch"))
        XCTAssertFalse(session.helloCompleted)
        let good = NearbyV1.line(id: "h", type: "hello", NearbyV1.Hello(pairId: "p", companionName: "c", nonce: "n", proof: proof))
        XCTAssertEqual(session.handle(line: good.dropLast(), emit: emit), .keepOpen)
        XCTAssertTrue(session.helloCompleted)
        XCTAssertEqual(session.handle(line: good.dropLast(), emit: emit), .closeAfterFlush("duplicate hello"))
        XCTAssertEqual(out.count, 6)
    }

    func testSessionValidatesCaptureSubmitFields() throws {
        let sink = RecordingSink()
        let key = NearbyListener.PSKEntry(identity: "p", key: NearbyListenerTests.pskA, isBootstrap: false)
        let session = NearbySession(keys: [key], macName: "M", sink: sink, destinations: nil, pairing: nil) { _ in }
        var out: [Data] = []
        let emit: (Data) -> Void = { out.append($0) }
        let hello = NearbyV1.line(id: "h", type: "hello", NearbyV1.Hello(pairId: "p", companionName: "c", nonce: "n",
                                                                         proof: Pairing.helloProof(psk: key.key, nonce: "n")))
        _ = session.handle(line: hello.dropLast(), emit: emit)
        var env = try RuntimeV1.decodeCaptureSubmit(Data(contentsOf: Self.fixtureURL))
        env.payload.image.mimeType = "image/gif"
        _ = session.handle(line: try RuntimeV1.encodeLine(env).dropLast(), emit: emit)
        let e1: RuntimeV1.Envelope<NearbyV1.ErrorPayload> = try decode(out[1])
        XCTAssertEqual(e1.payload.code, "unsupported_image")
        env.payload.image.mimeType = "image/png"
        env.payload.captureId = "bad id!"
        _ = session.handle(line: try RuntimeV1.encodeLine(env).dropLast(), emit: emit)
        let e2: RuntimeV1.Envelope<NearbyV1.ErrorPayload> = try decode(out[2])
        XCTAssertEqual(e2.payload.code, "bad_request")
        env.payload.captureId = "ok-1"
        env.payload.instructions = String(repeating: "x", count: 4097)
        _ = session.handle(line: try RuntimeV1.encodeLine(env).dropLast(), emit: emit)
        let e3: RuntimeV1.Envelope<NearbyV1.ErrorPayload> = try decode(out[3])
        XCTAssertEqual(e3.payload.code, "bad_request")
        XCTAssertEqual(sink.count, 0)
        env.payload.instructions = "ok"
        XCTAssertEqual(session.handle(line: try RuntimeV1.encodeLine(env).dropLast(), emit: emit), .keepOpen)
        XCTAssertEqual(sink.count, 1)
        let ack: RuntimeV1.Envelope<NearbyV1.CaptureReceived> = try decode(out[4])
        XCTAssertEqual(ack.payload.captureId, "ok-1")
    }

    /// Additive `capture_status`: refused for an id the pairing never
    /// submitted, answered by the sink for an acknowledged one (same session
    /// or a later one sharing the pairing's memory), `unavailable` from a sink
    /// that predates the message.
    func testSessionAnswersCaptureStatusOnlyForAcknowledgedCaptures() throws {
        final class StatusSink: CaptureSink {
            let inner = RecordingSink()
            var ack: NearbyV1.CaptureStatusAck?
            func submit(_ envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>, reply: @escaping (Data) -> Void) { inner.submit(envelope, reply: reply) }
            func captureStatus(_ envelope: RuntimeV1.Envelope<NearbyV1.CaptureStatusRequest>, reply: @escaping (Data) -> Void) {
                guard let ack, ack.captureId == envelope.payload.captureId else {
                    reply(NearbyV1.errorLine(id: envelope.id, code: "unknown_capture", message: "not on this Mac")); return
                }
                reply(NearbyV1.line(id: envelope.id, type: "capture_status_ack", ack))
            }
        }
        let sink = StatusSink()
        let key = NearbyListener.PSKEntry(identity: "p", key: NearbyListenerTests.pskA, isBootstrap: false)
        let memory = NearbyAckMemory(maxPerPair: 8)
        let session = NearbySession(keys: [key], macName: "M", sink: sink, destinations: nil, pairing: nil, memory: memory) { _ in }
        var out: [Data] = []
        let emit: (Data) -> Void = { out.append($0) }
        let hello = NearbyV1.line(id: "h", type: "hello", NearbyV1.Hello(pairId: "p", companionName: "c", nonce: "n",
                                                                         proof: Pairing.helloProof(psk: key.key, nonce: "n")))
        _ = session.handle(line: hello.dropLast(), emit: emit)
        // Never submitted → unknown_capture (nothing leaks about other captures).
        _ = session.handle(line: NearbyV1.line(id: "s0", type: "capture_status", NearbyV1.CaptureStatusRequest(captureId: "fixture-capture-1")).dropLast(), emit: emit)
        let e0: RuntimeV1.Envelope<NearbyV1.ErrorPayload> = try decode(out[1])
        XCTAssertEqual(e0.id, "s0"); XCTAssertEqual(e0.payload.code, "unknown_capture")
        _ = session.handle(line: NearbyV1.line(id: "s1", type: "capture_status", NearbyV1.CaptureStatusRequest(captureId: "bad id!")).dropLast(), emit: emit)
        let e1: RuntimeV1.Envelope<NearbyV1.ErrorPayload> = try decode(out[2])
        XCTAssertEqual(e1.payload.code, "bad_request")
        // Submit, then ask: the sink answers with the proposal text.
        let env = try RuntimeV1.decodeCaptureSubmit(Data(contentsOf: Self.fixtureURL))
        XCTAssertEqual(session.handle(line: try RuntimeV1.encodeLine(env).dropLast(), emit: emit), .keepOpen)
        let received: RuntimeV1.Envelope<NearbyV1.CaptureReceived> = try decode(out[3])
        XCTAssertEqual(received.payload.captureId, "fixture-capture-1")
        sink.ack = .init(captureId: "fixture-capture-1", state: .proposalReady, durable: true, latex: "\\begin{tikzpicture}\\end{tikzpicture}", note: "awaiting review")
        _ = session.handle(line: NearbyV1.line(id: "s2", type: "capture_status", NearbyV1.CaptureStatusRequest(captureId: "fixture-capture-1")).dropLast(), emit: emit)
        let a2: RuntimeV1.Envelope<NearbyV1.CaptureStatusAck> = try decode(out[4])
        XCTAssertEqual(a2.id, "s2"); XCTAssertEqual(a2.type, "capture_status_ack")
        XCTAssertEqual(a2.payload.state, "proposal_ready")
        XCTAssertEqual(a2.payload.latex, "\\begin{tikzpicture}\\end{tikzpicture}")
        XCTAssertEqual(sink.inner.count, 1, "a status probe is not a delivery")
        // A later session of the same pairing shares the memory: still answered.
        let later = NearbySession(keys: [key], macName: "M", sink: sink, destinations: nil, pairing: nil, memory: memory) { _ in }
        var out2: [Data] = []
        _ = later.handle(line: NearbyV1.line(id: "h2", type: "hello", NearbyV1.Hello(pairId: "p", companionName: "c", nonce: "n2",
                                                                                     proof: Pairing.helloProof(psk: key.key, nonce: "n2"))).dropLast()) { out2.append($0) }
        sink.ack = .init(captureId: "fixture-capture-1", state: .inserted, durable: true, latex: "x", newRevision: 9)
        _ = later.handle(line: NearbyV1.line(id: "s3", type: "capture_status", NearbyV1.CaptureStatusRequest(captureId: "fixture-capture-1")).dropLast()) { out2.append($0) }
        let a3: RuntimeV1.Envelope<NearbyV1.CaptureStatusAck> = try decode(out2[1])
        XCTAssertEqual(a3.payload.state, "inserted"); XCTAssertEqual(a3.payload.newRevision, 9)
        // A sink without the method (older adapters) answers `unavailable`, not a guess.
        let plain = RecordingSink()
        let s3 = NearbySession(keys: [key], macName: "M", sink: plain, destinations: nil, pairing: nil, memory: memory) { _ in }
        var out3: [Data] = []
        _ = s3.handle(line: NearbyV1.line(id: "h3", type: "hello", NearbyV1.Hello(pairId: "p", companionName: "c", nonce: "n3",
                                                                                  proof: Pairing.helloProof(psk: key.key, nonce: "n3"))).dropLast()) { out3.append($0) }
        _ = s3.handle(line: NearbyV1.line(id: "s4", type: "capture_status", NearbyV1.CaptureStatusRequest(captureId: "fixture-capture-1")).dropLast()) { out3.append($0) }
        let e4: RuntimeV1.Envelope<NearbyV1.ErrorPayload> = try decode(out3[1])
        XCTAssertEqual(e4.payload.code, "unavailable")

        // Fresh memory (the entry was evicted, or pairs.json lost it) while the
        // bridge journal still knows the capture: the gate refuses first; the
        // companion re-delivers its saved envelope (idempotent on the Mac —
        // acknowledged again, delivered to the sink once more only because this
        // memory never saw it), and the probe is then answered from the journal.
        let fresh = NearbySession(keys: [key], macName: "M", sink: sink, destinations: nil, pairing: nil, memory: NearbyAckMemory(maxPerPair: 8)) { _ in }
        var out4: [Data] = []
        _ = fresh.handle(line: NearbyV1.line(id: "h4", type: "hello", NearbyV1.Hello(pairId: "p", companionName: "c", nonce: "n4",
                                                                                     proof: Pairing.helloProof(psk: key.key, nonce: "n4"))).dropLast()) { out4.append($0) }
        _ = fresh.handle(line: NearbyV1.line(id: "s5", type: "capture_status", NearbyV1.CaptureStatusRequest(captureId: "fixture-capture-1")).dropLast()) { out4.append($0) }
        let e5: RuntimeV1.Envelope<NearbyV1.ErrorPayload> = try decode(out4[1])
        XCTAssertEqual(e5.payload.code, "unknown_capture")
        XCTAssertEqual(fresh.handle(line: try RuntimeV1.encodeLine(env).dropLast()) { out4.append($0) }, .keepOpen)
        let again: RuntimeV1.Envelope<NearbyV1.CaptureReceived> = try decode(out4[2])
        XCTAssertEqual(again.payload.captureId, "fixture-capture-1")
        _ = fresh.handle(line: NearbyV1.line(id: "s6", type: "capture_status", NearbyV1.CaptureStatusRequest(captureId: "fixture-capture-1")).dropLast()) { out4.append($0) }
        let a6: RuntimeV1.Envelope<NearbyV1.CaptureStatusAck> = try decode(out4[3])
        XCTAssertEqual(a6.payload.state, "inserted")
    }
}

// MARK: - pairing and store

final class PairingTests: XCTestCase {
    func testHKDFDerivationIsDeterministicAndPinned() throws {
        let salt = try XCTUnwrap(Pairing.data(hex: "000102030405060708090a0b0c0d0e0f"))
        let a = Pairing.derive(code: "123456", salt: salt)
        let b = Pairing.derive(code: "123456", salt: salt)
        XCTAssertEqual(a, b)
        XCTAssertEqual(a.psk.count, 32)
        XCTAssertEqual(a.pairId.count, 16)
        XCTAssertNotEqual(a, Pairing.derive(code: "123457", salt: salt))
        XCTAssertNotEqual(a, Pairing.derive(code: "123456", salt: Pairing.generateSalt()))
        // Pinned vector for the companion implementation (see docs/nearby-v1-proposal.md).
        XCTAssertEqual(a.pairId, Pairing.vectorPairID)
        XCTAssertEqual(Pairing.hex(a.psk), Pairing.vectorPSKHex)
        XCTAssertEqual(Pairing.fingerprint(salt: salt), Pairing.vectorFingerprint)
        XCTAssertEqual(Pairing.helloProof(psk: a.psk, nonce: "n-1"), Pairing.vectorHelloProof)
        XCTAssertTrue(Pairing.verifyHelloProof(Pairing.vectorHelloProof, psk: a.psk, nonce: "n-1"))
        XCTAssertFalse(Pairing.verifyHelloProof(Pairing.vectorHelloProof, psk: a.psk, nonce: "n-2"))
        XCTAssertFalse(Pairing.verifyHelloProof("not base64!", psk: a.psk, nonce: "n-1"))
        XCTAssertEqual(Pairing.generateCode().count, 6)
        XCTAssertTrue(Pairing.generateCode().allSatisfy(\.isNumber))
        XCTAssertNotEqual(Pairing.mintLongTermPSK(), Pairing.mintLongTermPSK())
    }

    func testPairStoreRoundTripAndPermissions() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-store-\(UUID().uuidString)")
        let url = dir.appendingPathComponent("FlashTeX/pairs.json")
        let store = PairStore(url: url)
        XCTAssertNil(store.loadError)
        XCTAssertEqual(store.salt.count, Pairing.saltLength)
        let record = PairRecord(pairId: "abcdef0123456789", psk: Pairing.mintLongTermPSK().base64EncodedString(),
                                companionName: "iPad", createdAt: Date(timeIntervalSince1970: 1_700_000_000), lastSeenAt: nil)
        XCTAssertTrue(store.upsert(record))
        let perms = try FileManager.default.attributesOfItem(atPath: url.path)[.posixPermissions] as? Int
        XCTAssertEqual(perms, 0o600)

        let reloaded = PairStore(url: url)
        XCTAssertEqual(reloaded.salt, store.salt)
        XCTAssertEqual(reloaded.pairs, [record])
        reloaded.touch(pairId: record.pairId)
        XCTAssertNotNil(reloaded.pair(id: record.pairId)?.lastSeenAt)
        XCTAssertTrue(reloaded.remove(pairId: record.pairId))
        XCTAssertEqual(PairStore(url: url).pairs, [])

        // A corrupt file is left alone and reported.
        try Data("{}".utf8).write(to: url)
        let corrupt = PairStore(url: url)
        XCTAssertNotNil(corrupt.loadError)
        XCTAssertFalse(corrupt.upsert(record))
        XCTAssertEqual(try Data(contentsOf: url), Data("{}".utf8))
        try? FileManager.default.removeItem(at: dir)
    }
}

// MARK: - ShellModel integration

@MainActor
final class ShellModelNearbyTests: XCTestCase {
    func testInboxAcknowledgesNonDurablyAndRefusesConflicts() throws {
        let model = ShellModel()
        let env = try RuntimeV1.decodeCaptureSubmit(Data(contentsOf: NearbyListenerTests.fixtureURL))
        guard case .success(let ack) = model.receiveNearbyCapture(env.payload) else { return XCTFail() }
        XCTAssertEqual(ack, .init(captureId: "fixture-capture-1", durable: false, hasProposal: false, applied: false))
        XCTAssertEqual(model.nearbyInbox.received.count, 1)
        XCTAssertEqual(model.nearbyInbox.lastCaptureId, "fixture-capture-1")
        // Identical retry: acknowledged again, stored once.
        guard case .success = model.receiveNearbyCapture(env.payload) else { return XCTFail() }
        XCTAssertEqual(model.nearbyInbox.received.count, 1)
        var other = env.payload
        other.instructions = "different"
        guard case .failure(let err) = model.receiveNearbyCapture(other) else { return XCTFail() }
        XCTAssertEqual(err.code, "capture_id_conflict")
        XCTAssertTrue(model.proposals.isEmpty, "a received capture is not a proposal")

        // Sink wrapper produces the wire line.
        let exp = expectation(description: "reply")
        var reply = Data()
        model.submit(env) { reply = $0; exp.fulfill() }
        wait(for: [exp], timeout: 2)
        let decoded = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureReceived>.self, from: reply)
        XCTAssertEqual(decoded.id, env.id)
        XCTAssertFalse(decoded.payload.durable)
    }

    /// `capture_status` without a bridge: inbox captures are `received`,
    /// anything else `unknown_capture`; the mapping from a bridge row plus the
    /// live session state is pure and covers every phase the iPad shows.
    func testCaptureStatusFromInboxAndBridgeRowMapping() async throws {
        let model = ShellModel()
        let env = try RuntimeV1.decodeCaptureSubmit(Data(contentsOf: NearbyListenerTests.fixtureURL))
        guard case .failure(let unknown) = await model.nearbyCaptureStatus(captureId: "never-sent") else { return XCTFail() }
        XCTAssertEqual(unknown.code, "unknown_capture")
        _ = model.receiveNearbyCapture(env.payload)
        guard case .success(let inbox) = await model.nearbyCaptureStatus(captureId: "fixture-capture-1") else { return XCTFail() }
        XCTAssertEqual(inbox.state, "received"); XCTAssertFalse(inbox.durable); XCTAssertNil(inbox.latex)
        let exp = expectation(description: "reply")
        var reply = Data()
        model.captureStatus(NearbyV1.envelope(id: "q1", type: "capture_status", NearbyV1.CaptureStatusRequest(captureId: "fixture-capture-1"))) { reply = $0; exp.fulfill() }
        await fulfillment(of: [exp], timeout: 2)
        let line = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureStatusAck>.self, from: reply)
        XCTAssertEqual(line.id, "q1"); XCTAssertEqual(line.type, "capture_status_ack"); XCTAssertEqual(line.payload.state, "received")

        typealias Cap = BridgeSession.Capture
        func row(proposal: String? = nil, applied: Int? = nil, rejected: Bool = false) throws -> TransferV1.CaptureStatus {
            var j: [String: Any] = ["capture_id": "c", "rejected": rejected]
            if let proposal { j["proposal"] = ["latex": proposal, "ambiguities": [], "required_dependencies": ["tikz"]] }
            if let applied { j["applied"] = ["edit_id": "e1", "new_revision": applied] }
            return try JSONDecoder().decode(TransferV1.CaptureStatus.self, from: JSONSerialization.data(withJSONObject: j))
        }
        let m = ShellModel.captureStatusAck
        XCTAssertEqual(m("c", Cap(captureId: "c", destinationId: "d", state: .received, note: "r"), try row()).state, "journaled")
        XCTAssertEqual(m("c", nil, try row()).state, "journaled")
        XCTAssertEqual(m("c", Cap(captureId: "c", destinationId: "d", state: .converting, note: "converting…"), try row()).state, "converting")
        let ready = m("c", Cap(captureId: "c", destinationId: "d", state: .proposed, note: "p"), try row(proposal: "\\alpha"))
        XCTAssertEqual(ready.state, "proposal_ready"); XCTAssertEqual(ready.latex, "\\alpha")
        XCTAssertEqual(m("c", Cap(captureId: "c", destinationId: "d", state: .prepared, note: "p"), try row(proposal: "\\alpha")).state, "proposal_ready")
        XCTAssertEqual(m("c", nil, try row(proposal: "\\alpha")).state, "proposal_ready", "bridge row alone after a Mac restart")
        let ins = m("c", Cap(captureId: "c", destinationId: "d", state: .confirmed, note: "ok"), try row(proposal: "\\alpha", applied: 12))
        XCTAssertEqual(ins.state, "inserted"); XCTAssertEqual(ins.newRevision, 12); XCTAssertEqual(ins.latex, "\\alpha")
        XCTAssertEqual(m("c", nil, try row(proposal: "\\alpha", rejected: true)).state, "rejected")
        XCTAssertEqual(m("c", Cap(captureId: "c", destinationId: "d", state: .failed, note: "provider_disabled"), try row()).note, "provider_disabled")
        XCTAssertEqual(m("c", Cap(captureId: "c", destinationId: "d", state: .failed, note: "x"), try row()).state, "failed")
        XCTAssertEqual(m("c", Cap(captureId: "c", destinationId: "d", state: .needsReselection, note: "x"), try row()).state, "failed")
        XCTAssertEqual(m("c", Cap(captureId: "c", destinationId: "d", state: .uncertain, note: "x"), nil).state, "uncertain")
        XCTAssertEqual(m("c", Cap(captureId: "c", destinationId: "d", state: .rejected, note: "x"), nil).state, "rejected")
    }

    func testDestinationFollowsPinnedAnchor() {
        let model = ShellModel()
        XCTAssertNil(model.nearbyDestination)
        model.caretUTF16 = 6
        model.pinAnchorAtCaret()
        let d = model.nearbyDestination
        XCTAssertEqual(d?.destinationId, model.anchor?.id)
        XCTAssertEqual(d?.path, "main.tex")
        XCTAssertEqual(d?.projectId, model.result?.projectId)
        XCTAssertEqual(d?.baseRevision, model.anchor?.revision)
        let exp = expectation(description: "dest")
        var got: NearbyV1.Destination?
        model.currentDestination { got = $0; exp.fulfill() }
        wait(for: [exp], timeout: 2)
        XCTAssertEqual(got, d)
    }
}

// MARK: - NearbyState (what the window drives)

@MainActor
final class NearbyStateTests: XCTestCase {
    func testPairForgetAndRestartThroughState() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-state-\(UUID().uuidString)")
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        let model = ShellModel()
        let state = NearbyState(store: store, macName: "State Mac", loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        XCTAssertEqual(state.txtRecord["fp"], state.fingerprint)
        XCTAssertEqual(state.txtRecord["v"], "1")

        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let port = try XCTUnwrap(state.port)

        state.beginPairing()
        let code = try XCTUnwrap(state.pairingCode)
        XCTAssertEqual(code.count, 6)
        XCTAssertNotNil(state.codeExpiresAt)
        // Restarting for the bootstrap key keeps the port.
        try await waitUntil("restarted with bootstrap key") {
            state.log.filter { $0.hasPrefix("ready on port") }.count >= 2
        }
        XCTAssertEqual(state.port, port)

        let derived = Pairing.derive(code: code, salt: store.salt)
        let client = NearbyTestClient(port: port, identity: derived.pairId, psk: derived.psk)
        try await waitUntil("client ready (\(String(describing: client.failure)))") { client.isReady }
        let nonce = UUID().uuidString
        client.send(id: "h", type: "hello", NearbyV1.Hello(pairId: derived.pairId, companionName: "State iPad", nonce: nonce,
                                                           proof: Pairing.helloProof(psk: derived.psk, nonce: nonce)))
        try await waitUntil("hello_ack") { client.lineCount >= 1 }
        let ack = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.HelloAck>.self, from: client.allLines[0])
        XCTAssertEqual(ack.payload.macName, "State Mac")
        XCTAssertNotNil(ack.payload.pairPsk)
        try await waitUntil("pair stored") { state.pairs.count == 1 && state.pairingCode == nil }
        XCTAssertEqual(state.pairs.first?.companionName, "State iPad")
        try await waitUntil("connected") { state.connectedPairIds == [derived.pairId] }

        // Capture through the state → ShellModel inbox.
        var fixture = try Data(contentsOf: NearbyListenerTests.fixtureURL)
        if fixture.last != 0x0A { fixture.append(0x0A) }
        client.send(fixture)
        try await waitUntil("capture_received") { client.lineCount >= 2 }
        try await waitUntil("capture noted") { state.lastReceivedCaptureId == "fixture-capture-1" }
        XCTAssertEqual(model.nearbyInbox.lastCaptureId, "fixture-capture-1")

        // Forget: record gone, session closed, listener still up on the same port.
        state.forget(pairId: derived.pairId)
        XCTAssertEqual(state.pairs, [])
        try await waitUntil("session closed") { client.isClosed }
        try await waitUntil("still advertising") { state.isAdvertising && state.port == port && state.connectedPairIds.isEmpty }
        let longTerm = try XCTUnwrap(Data(base64Encoded: try XCTUnwrap(ack.payload.pairPsk)))
        let gone = NearbyTestClient(port: port, identity: derived.pairId, psk: longTerm)
        try await waitUntil("forgotten key refused") { gone.isFailed }
        XCTAssertFalse(gone.isReady)

        state.stopAdvertising()
        XCTAssertFalse(state.isAdvertising)
        XCTAssertNil(state.port)
        try? FileManager.default.removeItem(at: dir)
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 5, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        XCTFail("timed out waiting for \(what)")
        throw NearbyWaitTimedOut()
    }
}

// MARK: - plaintext peers and bridge forwarding

final class NearbyPlaintextTests: XCTestCase {
    /// The companion at origin/agent/aarush-macbook/companion-capture e7ce5b9
    /// connects with `NWParameters.tcp` and sends its hello line in the clear.
    /// The listener must refuse that cleanly: handshake failure, no session,
    /// nothing parsed, one closed event, and it keeps serving TLS peers.
    func testPlainTCPClientIsRefusedWithoutParsingAndListenerSurvives() throws {
        let sink = RecordingSink()
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: NearbyListenerTests.pskA, isBootstrap: false)],
                                sink: sink, destinations: nil)
        try h.start()
        defer { h.stop() }

        let queue = DispatchQueue(label: "nearby.test.plain")
        let plain = NWConnection(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: h.port)!, using: .tcp)
        let ready = XCTestExpectation(description: "tcp ready")
        let ended = XCTestExpectation(description: "server ended the plaintext connection")
        plain.stateUpdateHandler = { state in
            switch state {
            case .ready: ready.fulfill()
            case .failed, .cancelled: ended.fulfill()
            default: break
            }
        }
        plain.start(queue: queue)
        XCTAssertEqual(XCTWaiter.wait(for: [ready], timeout: 5), .completed, "TCP itself connects; TLS is what refuses")
        let theirHello = "{\"protocol_version\":1,\"type\":\"hello\",\"id\":\"\(UUID().uuidString)\",\"payload\":{\"role\":\"companion\"}}\n"
        plain.send(content: Data(theirHello.utf8), completion: .contentProcessed { _ in })
        var fixture = try Data(contentsOf: NearbyListenerTests.fixtureURL)
        if fixture.last != 0x0A { fixture.append(0x0A) }
        plain.send(content: fixture, completion: .contentProcessed { _ in })
        var received = Data()
        func drain() {
            plain.receive(minimumIncompleteLength: 1, maximumLength: 65536) { data, _, complete, error in
                if let data { received.append(data) }
                if complete || error != nil { ended.fulfill(); return }
                drain()
            }
        }
        drain()
        XCTAssertEqual(XCTWaiter.wait(for: [ended], timeout: 5), .completed, "server must drop a plaintext peer")
        // Whatever came back is a TLS alert at most, never a JSON line.
        XCTAssertFalse(String(decoding: received, as: UTF8.self).contains("protocol_version"), "no plaintext reply")
        XCTAssertEqual(sink.count, 0)
        let events = h.snapshot
        XCTAssertFalse(events.contains(.connectionOpened), "\(events)")
        let closed = events.filter { if case .connectionClosed(let id, _) = $0 { return id == nil }; return false }
        XCTAssertEqual(closed.count, 1, "exactly one closed event for the refused peer: \(events)")
        plain.cancel()

        // Still serving paired peers.
        let good = NearbyTestClient(port: h.port, identity: "pair-a", psk: NearbyListenerTests.pskA)
        XCTAssertEqual(XCTWaiter.wait(for: [good.ready], timeout: 5), .completed, "\(String(describing: good.failure))")
        let nonce = "after-plain"
        good.send(id: "h", type: "hello", NearbyV1.Hello(pairId: "pair-a", companionName: "x", nonce: nonce,
                                                         proof: Pairing.helloProof(psk: NearbyListenerTests.pskA, nonce: nonce)))
        XCTAssertEqual(try good.lines(atLeast: 1).count, 1)
        good.cancel()
    }
}

@MainActor
final class NearbyBridgeForwardingTests: XCTestCase {
    private func waitUntil(_ what: String, timeout: TimeInterval = 10, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        XCTFail("timed out waiting for \(what)")
        throw NearbyWaitTimedOut()
    }

    /// With the fake bridge attached, a nearby capture is forwarded and the
    /// bridge's durable acknowledgement (or error) is what the companion gets;
    /// `hello_ack.destination` follows the bridge's pinned anchor.
    func testNearbyCaptureIsForwardedToAttachedBridge() async throws {
        let model = ShellModel()
        model.autoCompile = false
        let store = try BridgeClientTests.tempStore()
        let ok = await model.attachBridgeAndWait(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeBridge.path], storeDirectory: store)
        XCTAssertTrue(ok, model.bridgeStatus)

        model.caretUTF16 = 6
        model.pinAnchorAtCaret()
        try await waitUntil("bridge pin") { model.bridgeDestination != nil }
        let dest = try XCTUnwrap(model.nearbyDestination)
        XCTAssertEqual(dest.destinationId, model.bridgeDestination?.destinationId)
        XCTAssertEqual(dest.baseRevision, model.bridgeDestination?.pinnedRevision)

        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-fwd-\(UUID().uuidString)")
        let state = NearbyState(store: PairStore(url: dir.appendingPathComponent("pairs.json")), macName: "Bridge Mac", loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        let psk = Pairing.mintLongTermPSK()
        XCTAssertTrue(state.store.upsert(PairRecord(pairId: "companion1", psk: psk.base64EncodedString(), companionName: "c", createdAt: Date(), lastSeenAt: nil)))
        state.refreshPairs()
        state.startAdvertising()
        try await waitUntil("advertising") { state.port != nil }
        let client = NearbyTestClient(port: state.port!, identity: "companion1", psk: psk)
        try await waitUntil("client ready") { client.isReady }
        let nonce = UUID().uuidString
        client.send(id: "h", type: "hello", NearbyV1.Hello(pairId: "companion1", companionName: "c", nonce: nonce,
                                                           proof: Pairing.helloProof(psk: psk, nonce: nonce)))
        try await waitUntil("hello_ack") { client.lineCount >= 1 }
        let ack = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.HelloAck>.self, from: client.allLines[0])
        XCTAssertEqual(ack.payload.destination, dest)

        var submit = try BridgeClientTests.fixtureCapture()
        submit.destinationId = dest.destinationId
        submit.baseRevision = dest.baseRevision
        submit.captureId = "nearby-1"
        client.send(id: "c1", type: "capture_submit", submit)
        try await waitUntil("capture_received") { client.lineCount >= 2 }
        let received = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureReceived>.self, from: client.allLines[1])
        XCTAssertEqual(received.id, "c1")
        XCTAssertTrue(received.payload.durable, "the bridge's acknowledgement is durable")
        XCTAssertEqual(model.bridgeCaptures.last?.captureId, "nearby-1")
        XCTAssertEqual(model.nearbyInbox.received.count, 0, "forwarded captures are not kept in the fallback inbox")
        XCTAssertEqual(model.nearbyInbox.lastCaptureId, "nearby-1")

        // Bridge errors pass through with their code.
        submit.captureId = "nearby-2"
        submit.instructions = "%error:invalid_image"
        client.send(id: "c2", type: "capture_submit", submit)
        try await waitUntil("error") { client.lineCount >= 3 }
        let err = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.ErrorPayload>.self, from: client.allLines[2])
        XCTAssertEqual(err.id, "c2")
        XCTAssertEqual(err.payload.code, "invalid_image")

        // Without the bridge the inbox answers non-durably.
        model.detachBridge()
        submit.captureId = "nearby-3"
        submit.instructions = "plain"
        client.send(id: "c3", type: "capture_submit", submit)
        try await waitUntil("fallback ack") { client.lineCount >= 4 }
        let fallback = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureReceived>.self, from: client.allLines[3])
        XCTAssertFalse(fallback.payload.durable)
        XCTAssertEqual(model.nearbyInbox.received.count, 1)

        client.cancel()
        state.stopAdvertising()
        try? FileManager.default.removeItem(at: dir)
    }
}

// MARK: - bounded receive (mac-nearby-transport)

/// Sink whose acknowledgements are held until the test releases them, so
/// in-flight accounting and duplicate coalescing can be observed.
final class DeferredSink: CaptureSink {
    private let lock = NSLock()
    private(set) var pending: [(envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>, reply: (Data) -> Void)] = []
    private(set) var delivered = 0
    var durable = false
    var refuseWith: NearbyV1.ErrorPayload?
    func submit(_ envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>, reply: @escaping (Data) -> Void) {
        lock.withLock { delivered += 1; pending.append((envelope, reply)) }
    }
    var count: Int { lock.withLock { delivered } }
    var pendingCount: Int { lock.withLock { pending.count } }
    /// Answers the oldest pending capture.
    func flush() {
        let next: (envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>, reply: (Data) -> Void)? = lock.withLock {
            pending.isEmpty ? nil : pending.removeFirst()
        }
        guard let next else { return }
        if let e = refuseWith {
            next.reply(NearbyV1.errorLine(id: next.envelope.id, code: e.code, message: e.message))
        } else {
            let ack = NearbyV1.CaptureReceived(captureId: next.envelope.payload.captureId, durable: durable, hasProposal: false, applied: false)
            next.reply(NearbyV1.line(id: next.envelope.id, type: "capture_received", ack))
        }
    }
}

/// Test images encoded with ImageIO (the same encoder the companion uses).
enum TestImages {
    static func png(width: Int, height: Int, noise: Bool = false, alpha: Bool = true, interlaced: Bool = false) -> Data {
        encode(width: width, height: height, noise: noise, alpha: alpha, type: UTType.png,
               properties: interlaced ? [kCGImagePropertyPNGDictionary: [kCGImagePropertyPNGInterlaceType: 1]] : nil)
    }
    static func jpeg(width: Int, height: Int, noise: Bool = false) -> Data {
        encode(width: width, height: height, noise: noise, alpha: false, type: UTType.jpeg, properties: nil)
    }
    private static func encode(width: Int, height: Int, noise: Bool, alpha: Bool, type: UTType, properties: [CFString: Any]?) -> Data {
        let cs = CGColorSpaceCreateDeviceRGB()
        let info = alpha ? CGImageAlphaInfo.premultipliedLast.rawValue : CGImageAlphaInfo.noneSkipLast.rawValue
        let ctx = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0, space: cs, bitmapInfo: info)!
        if noise {
            let p = ctx.data!.assumingMemoryBound(to: UInt8.self)
            var g = SystemRandomNumberGenerator()
            for i in 0..<(ctx.bytesPerRow * height) { p[i] = UInt8.random(in: 0...255, using: &g) }
        } else {
            ctx.setFillColor(CGColor(red: 0.2, green: 0.4, blue: 0.8, alpha: 1))
            ctx.fill(CGRect(x: 0, y: 0, width: width, height: height))
            ctx.setStrokeColor(CGColor(red: 0, green: 0, blue: 0, alpha: 1))
            ctx.setLineWidth(3)
            ctx.move(to: CGPoint(x: 4, y: 4)); ctx.addLine(to: CGPoint(x: width - 4, y: height - 4)); ctx.strokePath()
        }
        let out = NSMutableData()
        let dest = CGImageDestinationCreateWithData(out, type.identifier as CFString, 1, nil)!
        CGImageDestinationAddImage(dest, ctx.makeImage()!, properties as CFDictionary?)
        CGImageDestinationFinalize(dest)
        return out as Data
    }
}

final class NearbyImageCheckTests: XCTestCase {
    let limits = NearbyReceiveLimits()

    func testValidPNGAndJPEGReportDimensions() throws {
        let png = TestImages.png(width: 37, height: 23)
        let p = try NearbyImageCheck.validate(bytes: png, mimeType: "image/png", limits: limits).get()
        XCTAssertEqual(p.format, "png"); XCTAssertEqual(p.width, 37); XCTAssertEqual(p.height, 23)
        XCTAssertEqual(p.encodedBytes, png.count)
        XCTAssertEqual(p.decodedBytes, 37 * 23 * 4)
        let jpg = TestImages.jpeg(width: 40, height: 30)
        let j = try NearbyImageCheck.validate(bytes: jpg, mimeType: "image/jpeg", limits: limits).get()
        XCTAssertEqual(j.format, "jpeg"); XCTAssertEqual(j.width, 40); XCTAssertEqual(j.height, 30)
        XCTAssertEqual(j.decodedBytes, 40 * 30 * 3)
        // The fixture the companion sends (1×1 PNG) and a base64 round trip.
        let fixture = try RuntimeV1.decodeCaptureSubmit(Data(contentsOf: NearbyListenerTests.fixtureURL)).payload.image
        let f = try NearbyImageCheck.validate(base64: fixture.dataBase64, mimeType: fixture.mimeType, limits: limits).get()
        XCTAssertEqual(f.width, 1); XCTAssertEqual(f.height, 1)
        // Noise (incompressible) exercises the streaming inflater across many buffers.
        let big = TestImages.png(width: 300, height: 300, noise: true)
        XCTAssertGreaterThan(big.count, 64 * 1024)
        XCTAssertEqual(try NearbyImageCheck.validate(bytes: big, mimeType: "image/png", limits: limits).get().width, 300)
    }

    func testInterlacedPNGRawSizeAndEncoder() throws {
        // Adam7 for an 8×8 RGBA8 image: 5+5+9+18+34+68+132.
        XCTAssertEqual(NearbyImageCheck.pngRawSize(width: 8, height: 8, channels: 4, bitDepth: 8, interlaced: true), 271)
        XCTAssertEqual(NearbyImageCheck.pngRawSize(width: 8, height: 8, channels: 4, bitDepth: 8, interlaced: false), 264)
        XCTAssertEqual(NearbyImageCheck.pngRawSize(width: 1, height: 1, channels: 1, bitDepth: 1, interlaced: true), 2)
        XCTAssertEqual(NearbyImageCheck.pngRawSize(width: 3, height: 5, channels: 3, bitDepth: 16, interlaced: false), 5 * 19)
        let data = TestImages.png(width: 21, height: 13, interlaced: true)
        let interlace = [UInt8](data)[28]
        try XCTSkipUnless(interlace == 1, "ImageIO did not write an Adam7 PNG on this system; formula covered above")
        XCTAssertEqual(try NearbyImageCheck.validate(bytes: data, mimeType: "image/png", limits: limits).get().width, 21)
    }

    func testTruncatedCorruptAndMismatchedImagesAreInvalid() throws {
        let png = TestImages.png(width: 64, height: 48)
        let jpg = TestImages.jpeg(width: 64, height: 48)
        func code(_ d: Data, _ mime: String) -> String? {
            if case .failure(let f) = NearbyImageCheck.validate(bytes: d, mimeType: mime, limits: limits) { return f.code }
            return nil
        }
        XCTAssertEqual(code(png.prefix(png.count / 2), "image/png"), "invalid_image", "truncated PNG")
        XCTAssertEqual(code(png.prefix(png.count - 1), "image/png"), "invalid_image", "PNG missing one byte of IEND")
        var corrupt = png
        corrupt[png.count / 2] ^= 0xFF
        XCTAssertEqual(code(corrupt, "image/png"), "invalid_image", "flipped IDAT byte fails the CRC")
        var trailing = png; trailing.append(contentsOf: [1, 2, 3])
        XCTAssertEqual(code(trailing, "image/png"), "invalid_image", "bytes after IEND")
        XCTAssertEqual(code(png, "image/jpeg"), "invalid_image", "PNG bytes declared as JPEG")
        XCTAssertEqual(code(jpg, "image/png"), "invalid_image", "JPEG bytes declared as PNG")
        XCTAssertEqual(code(jpg.prefix(jpg.count / 2), "image/jpeg"), "invalid_image", "truncated JPEG scan")
        XCTAssertEqual(code(jpg.prefix(jpg.count - 2), "image/jpeg"), "invalid_image", "JPEG without EOI")
        XCTAssertEqual(code(Data("hello".utf8), "image/png"), "invalid_image")
        XCTAssertEqual(code(Data(), "image/png"), "invalid_image")
        XCTAssertEqual(code(png, "image/gif"), "invalid_image")
        if case .failure(let f) = NearbyImageCheck.validate(base64: "not base64!!", mimeType: "image/png", limits: limits) {
            XCTAssertEqual(f.code, "invalid_image")
        } else { XCTFail() }
        // A CRC-consistent PNG whose pixel stream is too short: rewrite IHDR
        // height (with a fresh CRC) so the inflated data no longer matches.
        var taller = [UInt8](png)
        taller[23] = 0x60 // height 48 -> 96
        let crc = NearbyImageCheck.crc32(taller, 12..<29)
        taller[29] = UInt8(crc >> 24); taller[30] = UInt8((crc >> 16) & 0xFF); taller[31] = UInt8((crc >> 8) & 0xFF); taller[32] = UInt8(crc & 0xFF)
        XCTAssertEqual(code(Data(taller), "image/png"), "invalid_image", "pixel stream shorter than the dimensions claim")
    }

    func testSizeCapsUseBridgeCodes() throws {
        var small = NearbyReceiveLimits()
        small.maxImageBytes = 200
        let png = TestImages.png(width: 64, height: 48)
        XCTAssertGreaterThan(png.count, 200)
        if case .failure(let f) = NearbyImageCheck.validate(bytes: png, mimeType: "image/png", limits: small) {
            XCTAssertEqual(f.code, "image_too_large")
        } else { XCTFail() }
        let base64 = png.base64EncodedString()
        if case .failure(let f) = NearbyImageCheck.validate(base64: base64, mimeType: "image/png", limits: small) {
            XCTAssertEqual(f.code, "image_too_large", "refused on base64 length before decoding")
        } else { XCTFail() }
        XCTAssertEqual(small.maxImageBase64Bytes, 268)
        var narrow = NearbyReceiveLimits()
        narrow.maxImageSide = 60
        if case .failure(let f) = NearbyImageCheck.validate(bytes: png, mimeType: "image/png", limits: narrow) {
            XCTAssertEqual(f.code, "invalid_image"); XCTAssertTrue(f.message.contains("64×48"))
        } else { XCTFail() }
        var shallow = NearbyReceiveLimits()
        shallow.maxDecodedImageBytes = 64 * 48 * 4 - 1
        if case .failure(let f) = NearbyImageCheck.validate(bytes: png, mimeType: "image/png", limits: shallow) {
            XCTAssertEqual(f.code, "invalid_image"); XCTAssertTrue(f.message.contains("decoded"))
        } else { XCTFail() }
        XCTAssertNoThrow(try NearbyImageCheck.validate(bytes: png, mimeType: "image/png", limits: limits).get())
    }
}

final class NearbySessionBoundsTests: XCTestCase {
    let key = NearbyListener.PSKEntry(identity: "p", key: NearbyListenerTests.pskA, isBootstrap: false)

    struct Driver {
        let session: NearbySession
        var out: [Data] = []
        var events: [NearbySession.Event] = []
    }

    /// Session with inline decoding (no decode queue) and a completed hello.
    func makeSession(sink: CaptureSink?, limits: NearbyReceiveLimits = .init(), budget: NearbyReceiveBudget? = nil,
                     pairId: String = "p", nonce: String = "n", events: @escaping (NearbySession.Event) -> Void = { _ in }) -> NearbySession {
        let keys = [NearbyListener.PSKEntry(identity: pairId, key: NearbyListenerTests.pskA, isBootstrap: false)]
        let s = NearbySession(keys: keys, macName: "M", sink: sink, destinations: nil, pairing: nil,
                              limits: limits, budget: budget, events: events)
        let hello = NearbyV1.line(id: "h", type: "hello", NearbyV1.Hello(pairId: pairId, companionName: "c", nonce: nonce,
                                                                          proof: Pairing.helloProof(psk: NearbyListenerTests.pskA, nonce: nonce)))
        XCTAssertEqual(s.handle(line: hello.dropLast(), emit: { _ in }), .keepOpen)
        return s
    }

    func capture(_ id: String = "c1", captureId: String = "cap-1", revision: Int = 1, image: Data? = nil,
                 mime: String = "image/png", instructions: String = "x") -> Data {
        let img = image ?? TestImages.png(width: 16, height: 16)
        let submit = RuntimeV1.CaptureSubmit(captureId: captureId, destinationId: "dest-1", baseRevision: revision,
                                             image: .init(mimeType: mime, dataBase64: img.base64EncodedString()), instructions: instructions)
        return NearbyV1.line(id: id, type: "capture_submit", submit).dropLast()
    }

    func errorCode(_ line: Data) -> String? {
        (try? JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.ErrorPayload>.self, from: line))?.payload.code
    }
    func ackId(_ line: Data) -> String? {
        let e = try? JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureReceived>.self, from: line)
        return e?.type == "capture_received" ? e?.id : nil
    }

    func testImageValidationRefusesBeforeTheSinkSeesIt() {
        let sink = RecordingSink()
        var events: [NearbySession.Event] = []
        let s = makeSession(sink: sink) { events.append($0) }
        var out: [Data] = []
        let png = TestImages.png(width: 16, height: 16)
        XCTAssertEqual(s.handle(line: capture(image: png.prefix(png.count - 8)), emit: { out.append($0) }), .keepOpen)
        XCTAssertEqual(errorCode(out[0]), "invalid_image")
        XCTAssertEqual(events.last, .refused(captureId: "cap-1", code: "invalid_image", message: "PNG is truncated (no IEND)"))
        _ = s.handle(line: capture(image: TestImages.jpeg(width: 16, height: 16), mime: "image/png"), emit: { out.append($0) })
        XCTAssertEqual(errorCode(out[1]), "invalid_image")
        var limits = NearbyReceiveLimits(); limits.maxImageBytes = 64
        let tight = makeSession(sink: sink, limits: limits)
        _ = tight.handle(line: capture(image: png), emit: { out.append($0) })
        XCTAssertEqual(errorCode(out[2]), "image_too_large")
        _ = s.handle(line: capture(revision: -1), emit: { out.append($0) })
        XCTAssertEqual(errorCode(out[3]), "bad_request")
        XCTAssertEqual(sink.count, 0, "nothing invalid reaches the sink")
        XCTAssertEqual(s.bytesInFlight, 0, "refused frames are not charged")
        _ = s.handle(line: capture(image: png), emit: { out.append($0) })
        XCTAssertEqual(ackId(out[4]), "c1")
        XCTAssertEqual(sink.count, 1)
        XCTAssertEqual(s.bytesInFlight, 0)
    }

    func testPerSessionBytesInFlightCapIsExplicitAndRecovers() {
        let sink = DeferredSink()
        let frame = capture("a", captureId: "cap-a")
        var limits = NearbyReceiveLimits()
        limits.maxSessionBytesInFlight = frame.count + 1 + frame.count / 2 // one frame fits, two do not
        var events: [NearbySession.Event] = []
        let s = makeSession(sink: sink, limits: limits) { events.append($0) }
        var out: [Data] = []
        _ = s.handle(line: capture("a", captureId: "cap-a"), emit: { out.append($0) })
        XCTAssertEqual(sink.count, 1)
        XCTAssertEqual(s.bytesInFlight, frame.count + 1)
        _ = s.handle(line: capture("b", captureId: "cap-b"), emit: { out.append($0) })
        XCTAssertEqual(errorCode(out[0]), "too_many_in_flight")
        XCTAssertEqual(sink.count, 1, "the over-cap frame is refused, not queued")
        XCTAssertTrue(events.contains { if case .refused(_, "too_many_in_flight", _) = $0 { return true }; return false })
        sink.flush()
        XCTAssertEqual(ackId(out[1]), "a")
        XCTAssertEqual(s.bytesInFlight, 0)
        _ = s.handle(line: capture("b", captureId: "cap-b"), emit: { out.append($0) })
        XCTAssertEqual(sink.count, 2)
        sink.flush()
        XCTAssertEqual(ackId(out[2]), "b")
    }

    func testListenerWideInboxCapAndSessionsPerPeer() {
        let frame = capture("a", captureId: "cap-a")
        var limits = NearbyReceiveLimits()
        limits.maxInboxBytes = frame.count + 1
        limits.maxSessionsPerPeer = 1
        let budget = NearbyReceiveBudget(limits: limits)
        let sink = DeferredSink()
        let s1 = makeSession(sink: sink, limits: limits, budget: budget, pairId: "p", nonce: "n1")
        let s2 = makeSession(sink: sink, limits: limits, budget: budget, pairId: "q", nonce: "n2")
        var out1: [Data] = [], out2: [Data] = []
        _ = s1.handle(line: capture("a", captureId: "cap-a"), emit: { out1.append($0) })
        XCTAssertEqual(budget.inboxBytesInFlight, frame.count + 1)
        _ = s2.handle(line: capture("b", captureId: "cap-b"), emit: { out2.append($0) })
        XCTAssertEqual(errorCode(out2[0]), "inbox_full")
        XCTAssertEqual(sink.count, 1)
        sink.flush()
        XCTAssertEqual(budget.inboxBytesInFlight, 0)
        _ = s2.handle(line: capture("b", captureId: "cap-b"), emit: { out2.append($0) })
        XCTAssertEqual(sink.count, 2)
        sink.flush()
        XCTAssertEqual(ackId(out2[1]), "b")

        // A second session for "p" is refused at hello while s1 lives.
        XCTAssertEqual(budget.sessionCount(pairId: "p"), 1)
        let keys = [NearbyListener.PSKEntry(identity: "p", key: NearbyListenerTests.pskA, isBootstrap: false)]
        let s3 = NearbySession(keys: keys, macName: "M", sink: sink, destinations: nil, pairing: nil, limits: limits, budget: budget) { _ in }
        let hello = NearbyV1.line(id: "h", type: "hello", NearbyV1.Hello(pairId: "p", companionName: "c", nonce: "n3",
                                                                          proof: Pairing.helloProof(psk: NearbyListenerTests.pskA, nonce: "n3")))
        var out3: [Data] = []
        XCTAssertEqual(s3.handle(line: hello.dropLast(), emit: { out3.append($0) }), .closeAfterFlush("too many sessions"))
        XCTAssertEqual(errorCode(out3[0]), "too_many_sessions")
        XCTAssertFalse(s3.helloCompleted)
        s3.end()
        XCTAssertEqual(budget.sessionCount(pairId: "p"), 1, "a refused hello holds no slot")
        s1.end()
        XCTAssertEqual(budget.sessionCount(pairId: "p"), 0)
        let s4 = NearbySession(keys: keys, macName: "M", sink: sink, destinations: nil, pairing: nil, limits: limits, budget: budget) { _ in }
        XCTAssertEqual(s4.handle(line: hello.dropLast(), emit: { _ in }), .keepOpen)
        XCTAssertEqual(budget.sessionCount(pairId: "p"), 1)

        // Ending a session with a capture still pending releases its share.
        _ = s4.handle(line: capture("c", captureId: "cap-c"), emit: { _ in })
        XCTAssertEqual(budget.inboxBytesInFlight, frame.count + 1)
        s4.end()
        XCTAssertEqual(budget.inboxBytesInFlight, 0)
        sink.flush() // the late reply must not double-release
        XCTAssertEqual(budget.inboxBytesInFlight, 0)
    }

    func testDuplicateIsAcknowledgedNotRedeliveredAndMismatchesAreRefused() {
        let sink = DeferredSink()
        var events: [NearbySession.Event] = []
        let s = makeSession(sink: sink) { events.append($0) }
        var out: [Data] = []
        let png = TestImages.png(width: 16, height: 16)
        // Two identical submissions while the first is still pending: one delivery, two acks.
        _ = s.handle(line: capture("r1", image: png), emit: { out.append($0) })
        _ = s.handle(line: capture("r2", image: png), emit: { out.append($0) })
        XCTAssertEqual(sink.count, 1)
        XCTAssertEqual(out.count, 0)
        XCTAssertEqual(events.last, .duplicate(captureId: "cap-1"))
        sink.flush()
        XCTAssertEqual(out.map(ackId), ["r1", "r2"])
        XCTAssertEqual(s.bytesInFlight, 0)
        // A later retry is answered from memory.
        _ = s.handle(line: capture("r3", image: png), emit: { out.append($0) })
        XCTAssertEqual(ackId(out[2]), "r3")
        XCTAssertEqual(sink.count, 1)
        XCTAssertEqual(events.filter { $0 == .duplicate(captureId: "cap-1") }.count, 2)
        // Same id, other revision: refused, not delivered.
        _ = s.handle(line: capture("r4", revision: 2, image: png), emit: { out.append($0) })
        XCTAssertEqual(errorCode(out[3]), "revision_mismatch")
        XCTAssertEqual(events.last, .refused(captureId: "cap-1", code: "revision_mismatch",
                                             message: "capture_id cap-1 was accepted at base_revision 1, not 2"))
        // Same id and revision, other payload: conflict.
        _ = s.handle(line: capture("r5", image: png, instructions: "other"), emit: { out.append($0) })
        XCTAssertEqual(errorCode(out[4]), "capture_id_conflict")
        XCTAssertEqual(sink.count, 1)
        // A different capture id is new.
        _ = s.handle(line: capture("r6", captureId: "cap-2", image: png), emit: { out.append($0) })
        XCTAssertEqual(sink.count, 2)
        sink.flush()
        XCTAssertEqual(ackId(out[5]), "r6")
        XCTAssertEqual(s.rememberedCaptureCount, 2)
    }

    func testSinkRefusalIsNotRememberedAndBoundsHold() {
        let sink = DeferredSink()
        var events: [NearbySession.Event] = []
        var limits = NearbyReceiveLimits()
        limits.maxRememberedCaptures = 3
        let s = makeSession(sink: sink, limits: limits) { events.append($0) }
        var out: [Data] = []
        let png = TestImages.png(width: 16, height: 16)
        sink.refuseWith = .init(code: "unavailable", message: "bridge down")
        _ = s.handle(line: capture("r1", image: png), emit: { out.append($0) })
        _ = s.handle(line: capture("r2", image: png), emit: { out.append($0) }) // coalesced onto the pending one
        sink.flush()
        XCTAssertEqual(out.map(errorCode), ["unavailable", "unavailable"])
        XCTAssertEqual(events.last, .refused(captureId: "cap-1", code: "unavailable", message: "bridge down"))
        XCTAssertEqual(s.rememberedCaptureCount, 0, "a refused capture is forgotten so a retry is delivered again")
        sink.refuseWith = nil
        _ = s.handle(line: capture("r3", image: png), emit: { out.append($0) })
        XCTAssertEqual(sink.count, 2)
        sink.flush()
        XCTAssertEqual(ackId(out[2]), "r3")
        // Memory is bounded: acknowledged entries are evicted oldest first, pending ones kept.
        for i in 2...5 {
            _ = s.handle(line: capture("x\(i)", captureId: "cap-\(i)", image: png), emit: { _ in })
            if i < 5 { sink.flush() }
        }
        XCTAssertEqual(s.rememberedCaptureCount, 3)
        _ = s.handle(line: capture("again", captureId: "cap-1", image: png), emit: { out.append($0) })
        XCTAssertEqual(sink.count, 7, "an evicted id is delivered again")
        s.end()
        // The memory belongs to the pairing, not the session (mac-nearby-transport-2):
        // a reconnect is answered from it; a forgotten pairing drops it.
        XCTAssertEqual(s.rememberedCaptureCount, 3)
        s.memory.forget(pairId: "p")
        XCTAssertEqual(s.rememberedCaptureCount, 0)
    }
}

final class NearbyBoundedTransportTests: XCTestCase {
    static let pskA = NearbyListenerTests.pskA

    func hello(_ client: NearbyTestClient, pairId: String, psk: Data, nonce: String = UUID().uuidString) {
        client.send(id: "h1", type: "hello", NearbyV1.Hello(pairId: pairId, companionName: "Bound iPad", nonce: nonce,
                                                            proof: Pairing.helloProof(psk: psk, nonce: nonce)))
    }
    func type(_ line: Data) -> String? { (try? RuntimeV1.header(of: line))?.type }
    func id(_ line: Data) -> String? { (try? RuntimeV1.header(of: line))?.id }

    /// A multi-megabyte capture from one peer is decoded off the listener
    /// queue: another peer's query is answered while it is still being
    /// validated, and the main thread keeps turning (ShellModel is the sink).
    @MainActor
    func testSlowCaptureDoesNotBlockOtherPeersOrTheMainThread() async throws {
        let model = ShellModel()
        model.autoCompile = false
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: Self.pskA, isBootstrap: false),
                                       .init(identity: "pair-b", key: NearbyListenerTests.pskB, isBootstrap: false)],
                                sink: model, destinations: FixedDestinations(nil))
        try h.listener.start()
        try await waitUntil("ready") { h.port != 0 }
        let a = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskA)
        let b = NearbyTestClient(port: h.port, identity: "pair-b", psk: NearbyListenerTests.pskB)
        try await waitUntil("clients ready") { a.isReady && b.isReady }
        hello(a, pairId: "pair-a", psk: Self.pskA)
        hello(b, pairId: "pair-b", psk: NearbyListenerTests.pskB)
        try await waitUntil("hello acks") { a.lineCount >= 1 && b.lineCount >= 1 }

        // >1 MiB of incompressible PNG: CRC + inflate take measurable time in a debug build.
        let noise = TestImages.png(width: 640, height: 640, noise: true)
        XCTAssertGreaterThan(noise.count, 1024 * 1024)
        let submit = RuntimeV1.CaptureSubmit(captureId: "big-1", destinationId: "dest", baseRevision: 1,
                                             image: .init(mimeType: "image/png", dataBase64: noise.base64EncodedString()), instructions: "big")
        // Main-thread stall meter while the capture is in flight.
        var maxGap: TimeInterval = 0
        var last = Date()
        let meter = Timer.scheduledTimer(withTimeInterval: 0.005, repeats: true) { _ in
            let now = Date(); maxGap = max(maxGap, now.timeIntervalSince(last)); last = now
        }
        let sentAt = Date()
        a.send(id: "big", type: "capture_submit", submit)
        var order: [String] = []
        // Interleave queries from b while a's capture is being received and validated.
        for i in 0..<20 {
            b.send(id: "q\(i)", type: "destination_query", NearbyV1.Empty())
            try await Task.sleep(nanoseconds: 5_000_000)
            if b.lineCount >= 2 + i, order.isEmpty || order.last != "b" { order.append("b") }
            if a.lineCount >= 2, !order.contains("a") { order.append("a") }
        }
        try await waitUntil("big ack", timeout: 90) { a.lineCount >= 2 }
        guard a.lineCount >= 2 else { return XCTFail("no acknowledgement for the big capture") }
        let elapsed = Date().timeIntervalSince(sentAt)
        try await waitUntil("queries answered", timeout: 10) { b.lineCount >= 21 }
        meter.invalidate()
        let ack = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureReceived>.self, from: a.allLines[1])
        XCTAssertEqual(ack.payload.captureId, "big-1")
        XCTAssertEqual(model.nearbyInbox.lastCaptureId, "big-1", "delivered to the main-actor inbox after validation")
        XCTAssertEqual(order.first, "b", "peer b was answered before peer a's capture was acknowledged (took \(elapsed)s)")
        XCTAssertLessThan(maxGap, 0.25, "main thread stalled for \(maxGap)s while a \(noise.count)-byte capture was validated")
        XCTAssertEqual(h.listener.inboxBytesInFlight, 0)
        a.cancel(); b.cancel()
        h.stop()
    }

    func testConnectionCapClosesExtraPeersWithoutASession() throws {
        var limits = NearbyReceiveLimits()
        limits.maxConnections = 2
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: Self.pskA, isBootstrap: false)], sink: nil,
                                destinations: nil, limits: limits)
        try h.start()
        defer { h.stop() }
        let c1 = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskA)
        let c2 = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskA)
        XCTAssertEqual(XCTWaiter.wait(for: [c1.ready, c2.ready], timeout: 5), .completed)
        let c3 = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskA)
        // The client sees a reset during its handshake (`.waiting`/`.failed`), never `.ready`.
        XCTAssertEqual(XCTWaiter.wait(for: [c3.failed], timeout: 5), .completed, "third connection is closed: \(String(describing: c3.failure))")
        XCTAssertFalse(c3.isReady)
        c3.cancel()
        XCTAssertTrue(h.snapshot.contains(.connectionClosed(identity: nil, reason: "too many connections (2)")), "\(h.snapshot)")
        XCTAssertEqual(h.snapshot.filter { $0 == .connectionOpened }.count, 2)
        c1.cancel()
        XCTAssertEqual(XCTWaiter.wait(for: [c1.closed], timeout: 5), .completed)
        // A slot frees up once the stack reports the close.
        var c4: NearbyTestClient?
        for _ in 0..<20 {
            let c = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskA)
            if XCTWaiter.wait(for: [c.ready], timeout: 1) == .completed { c4 = c; break }
        }
        XCTAssertNotNil(c4, "a connection is accepted again after one closed")
        c2.cancel(); c4?.cancel()
    }

    func testSessionsPerPeerAndInFlightCapsOverTheWire() throws {
        var limits = NearbyReceiveLimits()
        limits.maxSessionsPerPeer = 1
        let sink = DeferredSink()
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: Self.pskA, isBootstrap: false)], sink: sink,
                                destinations: nil, limits: limits)
        try h.start()
        defer { h.stop() }
        let c1 = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskA)
        XCTAssertEqual(XCTWaiter.wait(for: [c1.ready], timeout: 5), .completed)
        hello(c1, pairId: "pair-a", psk: Self.pskA)
        XCTAssertEqual(type(try c1.lines(atLeast: 1)[0]), "hello_ack")
        let c2 = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.pskA)
        XCTAssertEqual(XCTWaiter.wait(for: [c2.ready], timeout: 5), .completed)
        hello(c2, pairId: "pair-a", psk: Self.pskA)
        let e = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.ErrorPayload>.self, from: c2.lines(atLeast: 1)[0])
        XCTAssertEqual(e.payload.code, "too_many_sessions")
        XCTAssertEqual(XCTWaiter.wait(for: [c2.closed], timeout: 5), .completed)
        XCTAssertEqual(h.listener.budget.sessionCount(pairId: "pair-a"), 1)

        // Duplicate over the wire while pending, then the in-flight cap.
        let png = TestImages.png(width: 32, height: 32)
        let submit = RuntimeV1.CaptureSubmit(captureId: "w-1", destinationId: "dest", baseRevision: 3,
                                             image: .init(mimeType: "image/png", dataBase64: png.base64EncodedString()), instructions: "w")
        c1.send(id: "s1", type: "capture_submit", submit)
        c1.send(id: "s2", type: "capture_submit", submit)
        let waited = XCTestExpectation(description: "delivered once")
        DispatchQueue.global().async {
            while sink.count < 1 { usleep(5_000) }
            usleep(100_000)
            waited.fulfill()
        }
        XCTAssertEqual(XCTWaiter.wait(for: [waited], timeout: 5), .completed)
        XCTAssertEqual(sink.count, 1, "the retry was coalesced")
        XCTAssertGreaterThan(h.listener.inboxBytesInFlight, 0)
        sink.flush()
        let lines = try c1.lines(atLeast: 3)
        XCTAssertEqual(lines[1...2].map(type), ["capture_received", "capture_received"])
        XCTAssertEqual(Set(lines[1...2].compactMap(id)), ["s1", "s2"])
        XCTAssertTrue(h.snapshot.contains(.captureDuplicate(identity: "pair-a", captureId: "w-1")))
        XCTAssertEqual(h.listener.inboxBytesInFlight, 0)
        var other = submit; other.baseRevision = 4
        c1.send(id: "s3", type: "capture_submit", other)
        let l4 = try c1.lines(atLeast: 4)
        XCTAssertEqual(type(l4[3]), "error")
        XCTAssertTrue(h.snapshot.contains { if case .captureRefused("pair-a", "w-1", "revision_mismatch", _) = $0 { return true }; return false }, "\(h.snapshot)")

        // Closing the session frees the peer slot and any pending share.
        var pending = submit; pending.captureId = "w-2"
        c1.send(id: "s4", type: "capture_submit", pending)
        let delivered = XCTestExpectation(description: "w-2 delivered")
        DispatchQueue.global().async { while sink.count < 2 { usleep(5_000) }; delivered.fulfill() }
        XCTAssertEqual(XCTWaiter.wait(for: [delivered], timeout: 5), .completed)
        c1.cancel()
        let freed = XCTestExpectation(description: "slot freed")
        DispatchQueue.global().async {
            while h.listener.budget.sessionCount(pairId: "pair-a") != 0 || h.listener.inboxBytesInFlight != 0 { usleep(5_000) }
            freed.fulfill()
        }
        XCTAssertEqual(XCTWaiter.wait(for: [freed], timeout: 5), .completed)
        sink.flush()
        XCTAssertEqual(h.listener.inboxBytesInFlight, 0)
    }

    @MainActor
    private func waitUntil(_ what: String, timeout: TimeInterval = 5, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
        throw NearbyWaitTimedOut()
    }
}

@MainActor
final class NearbyStateErrorTests: XCTestCase {
    /// The window's error state: a refused capture is named with its code, a
    /// duplicate is counted, and the log carries both; `clearReceiveErrors`
    /// acknowledges without losing the log.
    func testRefusalsAndDuplicatesAreVisibleState() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-err-\(UUID().uuidString)")
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        let model = ShellModel()
        model.autoCompile = false
        let state = NearbyState(store: store, macName: "Err Mac", loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        let psk = Pairing.mintLongTermPSK()
        XCTAssertTrue(store.upsert(PairRecord(pairId: "companion9", psk: psk.base64EncodedString(), companionName: "c", createdAt: Date(), lastSeenAt: nil)))
        state.refreshPairs()
        state.startAdvertising()
        try await waitUntil("advertising") { state.port != nil }
        let client = NearbyTestClient(port: state.port!, identity: "companion9", psk: psk)
        try await waitUntil("client ready") { client.isReady }
        let nonce = UUID().uuidString
        client.send(id: "h", type: "hello", NearbyV1.Hello(pairId: "companion9", companionName: "c", nonce: nonce,
                                                           proof: Pairing.helloProof(psk: psk, nonce: nonce)))
        try await waitUntil("hello_ack") { client.lineCount >= 1 }
        XCTAssertNil(state.lastReceiveError)

        let png = TestImages.png(width: 24, height: 24)
        var submit = RuntimeV1.CaptureSubmit(captureId: "e-1", destinationId: "dest", baseRevision: 1,
                                             image: .init(mimeType: "image/png", dataBase64: png.prefix(png.count / 2).base64EncodedString()),
                                             instructions: "broken")
        client.send(id: "c1", type: "capture_submit", submit)
        try await waitUntil("refusal visible") { state.lastReceiveError != nil }
        let err = try XCTUnwrap(state.lastReceiveError)
        XCTAssertEqual(err.code, "invalid_image")
        XCTAssertEqual(err.captureId, "e-1")
        XCTAssertEqual(err.pairId, "companion9")
        XCTAssertEqual(state.receiveErrors, [err])
        XCTAssertTrue(state.log.contains { $0.hasPrefix("refused invalid_image: e-1 from companion9") }, "\(state.log)")
        XCTAssertNil(model.nearbyInbox.lastCaptureId, "nothing reached the inbox")

        submit.image.dataBase64 = png.base64EncodedString()
        client.send(id: "c2", type: "capture_submit", submit)
        client.send(id: "c3", type: "capture_submit", submit)
        try await waitUntil("acks") { client.lineCount >= 4 }
        try await waitUntil("duplicate counted") { state.duplicateCaptureCount == 1 }
        XCTAssertEqual(state.lastDuplicateCaptureId, "e-1")
        XCTAssertEqual(state.lastReceivedCaptureId, "e-1")
        XCTAssertEqual(model.nearbyInbox.received.count, 1, "the retry was not re-delivered")
        submit.baseRevision = 2
        client.send(id: "c4", type: "capture_submit", submit)
        try await waitUntil("revision refusal") { state.receiveErrors.count == 2 }
        XCTAssertEqual(state.lastReceiveError?.code, "revision_mismatch")
        XCTAssertEqual(model.nearbyInbox.received.count, 1)

        state.clearReceiveErrors()
        XCTAssertNil(state.lastReceiveError)
        XCTAssertTrue(state.receiveErrors.isEmpty)
        XCTAssertTrue(state.log.contains { $0.hasPrefix("refused revision_mismatch") })
        XCTAssertEqual(state.limits, NearbyReceiveLimits())
        client.cancel()
        state.stopAdvertising()
        try? FileManager.default.removeItem(at: dir)
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 5, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
        throw NearbyWaitTimedOut()
    }
}

// MARK: - transcript acceptance (mac-nearby-transport)

/// Replays a companion session transcript against the listener on loopback.
///
/// Provenance of `Fixtures/nearby-companion-session.jsonl`: the shape of the
/// iPad Pro 13" (M5) simulator run recorded in
/// `docs/evidence/companion-simulator/README.md` §5 — runtime-v1
/// `capture_submit` envelopes with `.sortedKeys` ordering and `/` escaped as
/// `\/`, `capture-<8 hex>` ids, `default-anchor` / revision 1, the companion's
/// default instructions, a 400×300 opaque photo-library PNG and a 1408×1510
/// RGBA pencil drawing with a transparent background, **each line emitted
/// twice byte-identically** (the double-print bug in that run). The captured
/// `launch-1-stdout.log` was not committed and the companion still connects
/// in plaintext (no TLS-PSK, no v1 `hello`), so the images were re-drawn with
/// ImageIO and the `hello` line is the §8 shape with the pinned pairing vector
/// (code 123456, salt 00…0f) and a proof computed independently in Python.
/// This is therefore a recorded-shape transcript, not the original bytes.
@MainActor
final class NearbyTranscriptAcceptanceTests: XCTestCase {
    static let transcriptURL = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        .appendingPathComponent("Fixtures/nearby-companion-session.jsonl")
    static let vectorPSK = Pairing.data(hex: Pairing.vectorPSKHex)!

    struct Transcript {
        var lines: [Data]
        var hello: RuntimeV1.Envelope<NearbyV1.Hello>
        var captures: [RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>]
    }

    func load() throws -> Transcript {
        let raw = try Data(contentsOf: Self.transcriptURL)
        var splitter = LineSplitter()
        let lines = splitter.append(raw)
        XCTAssertEqual(splitter.pendingBytes, 0, "transcript ends with a newline")
        let hello = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.Hello>.self, from: lines[0])
        let captures = try lines.dropFirst().map { try RuntimeV1.decodeCaptureSubmit($0) }
        return Transcript(lines: lines, hello: hello, captures: captures)
    }

    /// Sends one line the way a companion's socket delivers it: in chunks.
    func stream(_ line: Data, to client: NearbyTestClient) async throws {
        var i = 0
        while i < line.count {
            let end = min(i + 16 * 1024, line.count)
            client.send(line[i..<end])
            i = end
            try await Task.sleep(nanoseconds: 1_000_000)
        }
    }

    func testTranscriptMatchesTheRecordedSessionShape() throws {
        let t = try load()
        XCTAssertEqual(t.lines.count, 5, "hello + 2 captures × 2 (double emission)")
        XCTAssertEqual(t.hello.type, "hello")
        XCTAssertEqual(t.hello.payload.pairId, Pairing.vectorPairID)
        XCTAssertEqual(t.hello.payload.protocolVersion, 1)
        XCTAssertTrue(String(decoding: t.lines[0], as: UTF8.self).contains("\"role\":\"companion\""), "the companion's extra key is kept")
        XCTAssertTrue(Pairing.verifyHelloProof(t.hello.payload.proof, psk: Self.vectorPSK, nonce: t.hello.payload.nonce),
                      "the Python-computed proof verifies against the Swift vector")
        XCTAssertEqual(t.lines[1], t.lines[2]); XCTAssertEqual(t.lines[3], t.lines[4])
        XCTAssertNotEqual(t.lines[1], t.lines[3])
        XCTAssertEqual(t.captures.map(\.payload.captureId), ["capture-3A2F48D1", "capture-3A2F48D1", "capture-23361924", "capture-23361924"])
        XCTAssertEqual(Set(t.captures.map(\.payload.destinationId)), ["default-anchor"])
        XCTAssertEqual(Set(t.captures.map(\.payload.baseRevision)), [1])
        for l in t.lines.dropFirst() {
            let s = String(decoding: l, as: UTF8.self)
            XCTAssertTrue(s.hasPrefix("{\"id\":\""), "sorted keys as the companion's encoder emits them")
            XCTAssertTrue(s.contains("\\/"), "slashes escaped as the companion's encoder emits them")
        }
        let limits = NearbyReceiveLimits()
        let photo = try NearbyImageCheck.validate(base64: t.captures[0].payload.image.dataBase64, mimeType: "image/png", limits: limits).get()
        XCTAssertEqual(photo.width, 400); XCTAssertEqual(photo.height, 300)
        XCTAssertEqual(photo.decodedBytes, 400 * 300 * 3, "opaque RGB")
        let drawing = try NearbyImageCheck.validate(base64: t.captures[2].payload.image.dataBase64, mimeType: "image/png", limits: limits).get()
        XCTAssertEqual(drawing.width, 1408); XCTAssertEqual(drawing.height, 1510)
        XCTAssertEqual(drawing.decodedBytes, 1408 * 1510 * 4, "RGBA, as PKDrawing.image exports")
        XCTAssertLessThan(t.lines.map(\.count).max()!, NearbyV1.maxLineBytes)
    }

    func testRecordedSessionIsAcceptedOnLoopbackWithDuplicatesAbsorbed() async throws {
        let t = try load()
        let model = ShellModel()
        model.autoCompile = false
        let h = ListenerHarness(psks: [.init(identity: Pairing.vectorPairID, key: Self.vectorPSK, isBootstrap: false)],
                                sink: model, destinations: FixedDestinations(nil))
        try h.listener.start()
        try await waitUntil("ready") { h.port != 0 }
        defer { h.stop() }

        // Connection 1: the transcript, verbatim, line by line.
        let c1 = NearbyTestClient(port: h.port, identity: Pairing.vectorPairID, psk: Self.vectorPSK)
        try await waitUntil("client ready (\(String(describing: c1.failure)))") { c1.isReady }
        for line in t.lines { try await stream(line + [0x0A], to: c1) }
        try await waitUntil("five replies", timeout: 90) { c1.lineCount >= 5 }
        let replies = c1.allLines
        guard replies.count >= 5 else { return XCTFail("got \(replies.count) replies: \(replies.map { String(decoding: $0.prefix(120), as: UTF8.self) })") }
        let ack = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.HelloAck>.self, from: replies[0])
        XCTAssertEqual(ack.type, "hello_ack")
        XCTAssertEqual(ack.id, t.hello.id)
        XCTAssertEqual(ack.payload.nonce, t.hello.payload.nonce)
        XCTAssertNil(ack.payload.pairPsk, "a stored pairing gets no new key")
        XCTAssertTrue(String(decoding: replies[0], as: UTF8.self).contains("\"destination\":null"))
        let received = try replies[1...4].map { try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureReceived>.self, from: $0) }
        XCTAssertEqual(received.map(\.type), Array(repeating: "capture_received", count: 4))
        XCTAssertEqual(received.map(\.id), t.captures.map(\.id), "every line, including the duplicates, is acknowledged with its own id")
        XCTAssertEqual(received.map(\.payload.captureId), t.captures.map(\.payload.captureId))
        XCTAssertEqual(received.map(\.payload.durable), [false, false, false, false], "no bridge attached")
        XCTAssertEqual(model.nearbyInbox.received.count, 2, "each capture reached the main-actor inbox once")
        XCTAssertEqual(model.nearbyInbox.received.map(\.captureId), ["capture-3A2F48D1", "capture-23361924"])
        XCTAssertEqual(model.nearbyInbox.received[1].image.dataBase64, t.captures[2].payload.image.dataBase64, "payload delivered unchanged")
        let events = h.snapshot
        XCTAssertTrue(events.contains(.hello(pairId: Pairing.vectorPairID, companionName: "iPad Pro 13-inch (M5)", bootstrap: false)), "\(events)")
        XCTAssertEqual(events.filter { if case .capture = $0 { return true }; return false }.count, 2)
        XCTAssertEqual(events.filter { $0 == .captureDuplicate(identity: Pairing.vectorPairID, captureId: "capture-3A2F48D1") }.count, 1)
        XCTAssertEqual(events.filter { $0 == .captureDuplicate(identity: Pairing.vectorPairID, captureId: "capture-23361924") }.count, 1)
        XCTAssertFalse(events.contains { if case .captureRefused = $0 { return true }; return false }, "nothing refused: \(events)")
        XCTAssertEqual(h.listener.inboxBytesInFlight, 0)
        XCTAssertFalse(c1.isClosed, "the session stays open")

        // Connection 2: the identical transcript again (a naive replay). The
        // hello nonce was already used, so the Mac refuses before any capture.
        let c2 = NearbyTestClient(port: h.port, identity: Pairing.vectorPairID, psk: Self.vectorPSK)
        try await waitUntil("client 2 ready") { c2.isReady }
        for line in t.lines { try await stream(line + [0x0A], to: c2) }
        try await waitUntil("replay refused") { c2.lineCount >= 1 }
        let err = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.ErrorPayload>.self, from: c2.allLines[0])
        XCTAssertEqual(err.payload.code, "bad_request")
        XCTAssertTrue(err.payload.message.contains("nonce"))
        try await waitUntil("replay closed") { c2.isClosed }
        XCTAssertEqual(c2.lineCount, 1)
        XCTAssertEqual(model.nearbyInbox.received.count, 2)

        // Connection 3: reconnect with a fresh hello and retry the same
        // captures (§7.4). The pairing's acknowledgement memory outlives the
        // session (mac-nearby-transport-2), so the retries are acknowledged
        // by the listener and never reach the inbox again.
        let c3 = NearbyTestClient(port: h.port, identity: Pairing.vectorPairID, psk: Self.vectorPSK)
        try await waitUntil("client 3 ready") { c3.isReady }
        let nonce = UUID().uuidString
        c3.send(id: "h3", type: "hello", NearbyV1.Hello(pairId: Pairing.vectorPairID, companionName: "iPad Pro 13-inch (M5)", nonce: nonce,
                                                        proof: Pairing.helloProof(psk: Self.vectorPSK, nonce: nonce)))
        try await waitUntil("hello 3") { c3.lineCount >= 1 }
        for line in t.lines.dropFirst() { try await stream(line + [0x0A], to: c3) }
        try await waitUntil("retries acknowledged", timeout: 90) { c3.lineCount >= 5 }
        guard c3.lineCount >= 5 else { return XCTFail("got \(c3.lineCount) replies on the retry connection") }
        let again = try c3.allLines[1...4].map { try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureReceived>.self, from: $0) }
        XCTAssertEqual(again.map(\.payload.captureId), t.captures.map(\.payload.captureId))
        XCTAssertEqual(model.nearbyInbox.received.count, 2, "identical retries after a reconnect are not stored twice")
        XCTAssertEqual(model.nearbyInbox.lastNote, "Received capture-23361924 (image/png, 75600 base64 bytes); not journaled.",
                       "the retries were answered from the pairing's memory, so the inbox never saw them")
        let dupes = h.snapshot.filter { if case .captureDuplicate(Pairing.vectorPairID?, _) = $0 { return true }; return false }
        XCTAssertEqual(dupes.count, 6, "two duplicates on connection 1, four retries on the reconnect: \(dupes)")
        c1.cancel(); c3.cancel()
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 5, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
        throw NearbyWaitTimedOut()
    }
}

// MARK: - events, progress, cancellation, resume, generations (mac-nearby-transport)

final class PairingGenerationTests: XCTestCase {
    func testReplacedAttemptCannotConfirmEvenWithTheSamePairId() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-gen-\(UUID().uuidString)")
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        let c = PairingCoordinator(store: store)
        let first = c.begin(code: "123456")
        XCTAssertEqual(first.generation, 1)
        XCTAssertEqual(c.bootstrapEntry?.generation, 1)
        // Same code again (the 10^-6 collision, forced): same pair_id, new generation.
        let second = c.begin(code: "123456")
        XCTAssertEqual(second.generation, 2)
        XCTAssertEqual(second.derived.pairId, first.derived.pairId)
        XCTAssertNil(c.confirmPairing(pairId: first.derived.pairId, companionName: "old", generation: 1))
        XCTAssertEqual(c.lastRefusal, "bootstrap key from replaced attempt 1; current is 2")
        XCTAssertNil(store.pair(id: first.derived.pairId))
        XCTAssertNotNil(c.confirmPairing(pairId: second.derived.pairId, companionName: "new", generation: 2))
        XCTAssertNil(c.lastRefusal)
        XCTAssertEqual(c.confirmedGeneration(pairId: second.derived.pairId), 2)
        XCTAssertNil(c.current, "confirmed attempt is consumed")
        XCTAssertNil(c.confirmPairing(pairId: second.derived.pairId, companionName: "again", generation: 2))
        XCTAssertEqual(c.lastRefusal, "no pairing code is pending")
        // A journal-supplied generation is honoured and later internal ones stay above it.
        XCTAssertEqual(c.begin(code: "222222", generation: 41).generation, 41)
        XCTAssertEqual(c.begin(code: "333333").generation, 42)
        XCTAssertNil(c.confirmPairing(pairId: "nobody", companionName: "x", generation: 42))
        XCTAssertEqual(c.lastRefusal, "pair_id nobody is not the pending attempt")
        try? FileManager.default.removeItem(at: dir)
    }

    /// Over the wire: a listener still holding the bootstrap key of a replaced
    /// code refuses that session's hello at `confirmPairing` (pairing_expired).
    func testSessionKeyedByReplacedCodeIsRefusedAtHello() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-gen2-\(UUID().uuidString)")
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        let c = PairingCoordinator(store: store)
        let old = c.begin(code: "111111")
        let oldKey = try XCTUnwrap(c.bootstrapEntry)
        let h = ListenerHarness(psks: [oldKey], sink: nil, destinations: nil, pairing: c)
        try h.start()
        defer { h.stop() }
        _ = c.begin(code: "999999") // replaced before the listener was rebuilt
        let client = NearbyTestClient(port: h.port, identity: old.derived.pairId, psk: old.derived.psk)
        XCTAssertEqual(XCTWaiter.wait(for: [client.ready], timeout: 5), .completed, "the stale key still authenticates TLS")
        let nonce = "stale-gen"
        client.send(id: "h", type: "hello", NearbyV1.Hello(pairId: old.derived.pairId, companionName: "late", nonce: nonce,
                                                           proof: Pairing.helloProof(psk: old.derived.psk, nonce: nonce)))
        let e = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.ErrorPayload>.self, from: client.lines(atLeast: 1)[0])
        XCTAssertEqual(e.payload.code, "pairing_expired")
        XCTAssertEqual(XCTWaiter.wait(for: [client.closed], timeout: 5), .completed)
        XCTAssertEqual(store.pairs, [])
        XCTAssertTrue(c.lastRefusal?.contains("not the pending attempt") ?? false, "\(String(describing: c.lastRefusal))")
        try? FileManager.default.removeItem(at: dir)
    }
}

final class NearbyReceivingProgressTests: XCTestCase {
    func testProgressIsReportedPer64KiBAndOnCompletionOnly() throws {
        let sink = RecordingSink()
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: NearbyListenerTests.pskA, isBootstrap: false)],
                                sink: sink, destinations: nil)
        try h.start()
        defer { h.stop() }
        let client = NearbyTestClient(port: h.port, identity: "pair-a", psk: NearbyListenerTests.pskA)
        XCTAssertEqual(XCTWaiter.wait(for: [client.ready], timeout: 5), .completed)
        let nonce = "prog"
        client.send(id: "h", type: "hello", NearbyV1.Hello(pairId: "pair-a", companionName: "p", nonce: nonce,
                                                           proof: Pairing.helloProof(psk: NearbyListenerTests.pskA, nonce: nonce)))
        _ = try client.lines(atLeast: 1)
        client.send(id: "q", type: "destination_query", NearbyV1.Empty())
        _ = try client.lines(atLeast: 2)
        XCTAssertFalse(h.snapshot.contains { if case .receiving = $0 { return true }; return false }, "small lines report no progress")

        // A ~330 KiB line, delivered in 16 KiB chunks with pauses so the server sees many reads.
        let png = TestImages.png(width: 300, height: 300, noise: true)
        let submit = RuntimeV1.CaptureSubmit(captureId: "prog-1", destinationId: "d", baseRevision: 1,
                                             image: .init(mimeType: "image/png", dataBase64: png.base64EncodedString()), instructions: "p")
        let line = NearbyV1.line(id: "c", type: "capture_submit", submit)
        XCTAssertGreaterThan(line.count, 4 * NearbyListener.receivingReportInterval)
        var i = 0
        while i < line.count {
            let end = min(i + 16 * 1024, line.count)
            client.send(line[i..<end])
            i = end
            usleep(2_000)
        }
        _ = try client.lines(atLeast: 3)
        let progress = h.snapshot.compactMap { e -> Int? in
            if case .receiving("pair-a", let bytes, nil) = e { return bytes }
            return nil
        }
        XCTAssertEqual(progress.last, line.count, "completion reports the whole line")
        XCTAssertGreaterThanOrEqual(progress.count, 2, "at least one interim report: \(progress)")
        XCTAssertEqual(progress, progress.sorted(), "monotonic")
        XCTAssertLessThanOrEqual(progress.count, line.count / NearbyListener.receivingReportInterval + 2, "bounded rate: \(progress)")
        for (a, b) in zip(progress, progress.dropFirst()).dropLast() {
            XCTAssertGreaterThanOrEqual(b - a, NearbyListener.receivingReportInterval, "interim reports at least 64 KiB apart: \(progress)")
        }
        XCTAssertEqual(sink.count, 1)
        client.cancel()
    }
}

@MainActor
final class NearbyStateEventTests: XCTestCase {
    private func waitUntil(_ what: String, timeout: TimeInterval = 5, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
        throw NearbyWaitTimedOut()
    }

    func testOnEventCloseConnectionAndResumedPairing() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-ev-\(UUID().uuidString)")
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        let model = ShellModel()
        model.autoCompile = false
        let state = NearbyState(store: store, macName: "Event Mac", loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        var events: [NearbyListener.Event] = []
        state.onEvent = { events.append($0) }

        // (4) Resume an interrupted attempt: the published code/expiry follow the resumed values.
        let expires = Date().addingTimeInterval(20)
        state.beginPairing(resuming: "123456", expiresAt: expires, generation: 7)
        XCTAssertEqual(state.pairingCode, "123456")
        XCTAssertEqual(state.codeExpiresAt, expires)
        XCTAssertEqual(state.coordinator.current?.code, "123456")
        XCTAssertEqual(state.coordinator.current?.generation, 7)
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        XCTAssertTrue(events.contains { if case .ready = $0 { return true }; return false }, "(1) onEvent saw the listener come up: \(events)")
        let port = try XCTUnwrap(state.port)

        // The resumed code pairs a companion.
        let derived = Pairing.derive(code: "123456", salt: store.salt)
        let client = NearbyTestClient(port: port, identity: derived.pairId, psk: derived.psk)
        try await waitUntil("client ready (\(String(describing: client.failure)))") { client.isReady }
        let nonce = UUID().uuidString
        client.send(id: "h", type: "hello", NearbyV1.Hello(pairId: derived.pairId, companionName: "Resumed iPad", nonce: nonce,
                                                           proof: Pairing.helloProof(psk: derived.psk, nonce: nonce)))
        try await waitUntil("hello_ack") { client.lineCount >= 1 }
        let ack = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.HelloAck>.self, from: client.allLines[0])
        XCTAssertNotNil(ack.payload.pairPsk)
        try await waitUntil("paired") { state.pairs.count == 1 && state.pairingCode == nil }
        XCTAssertEqual(state.coordinator.confirmedGeneration(pairId: derived.pairId), 7)
        try await waitUntil("connected") { state.connectedPairIds == [derived.pairId] }
        XCTAssertTrue(events.contains(.hello(pairId: derived.pairId, companionName: "Resumed iPad", bootstrap: true)), "\(events)")

        // (2) Progress reaches the state and onEvent while a large line is in flight...
        let png = TestImages.png(width: 260, height: 260, noise: true)
        let submit = RuntimeV1.CaptureSubmit(captureId: "cancel-me", destinationId: "d", baseRevision: 1,
                                             image: .init(mimeType: "image/png", dataBase64: png.base64EncodedString()), instructions: "p")
        let line = NearbyV1.line(id: "c", type: "capture_submit", submit)
        XCTAssertGreaterThan(line.count, 3 * NearbyListener.receivingReportInterval)
        let half = line.count / 2
        var i = 0
        while i < half {
            let end = min(i + 16 * 1024, half)
            client.send(line[i..<end])
            i = end
            try await Task.sleep(nanoseconds: 2_000_000)
        }
        try await waitUntil("progress visible") { (state.receivingBytes[derived.pairId] ?? 0) > 0 }
        let seen = try XCTUnwrap(state.receivingBytes[derived.pairId])
        XCTAssertLessThan(seen, line.count)
        XCTAssertTrue(events.contains { if case .receiving(derived.pairId, seen, nil) = $0 { return true }; return false })

        // (3) ...and the Mac can cancel it: the session closes, budget and progress are released, the pairing stays.
        state.closeConnection(pairId: derived.pairId)
        try await waitUntil("client closed") { client.isClosed }
        try await waitUntil("disconnected") { state.connectedPairIds.isEmpty && state.receivingBytes[derived.pairId] == nil }
        XCTAssertTrue(events.contains { if case .connectionClosed(derived.pairId, "closed by the Mac") = $0 { return true }; return false }, "\(events)")
        XCTAssertEqual(state.pairs.count, 1, "closing a session does not forget the pairing")
        XCTAssertNil(model.nearbyInbox.lastCaptureId, "the half-received capture never reached the inbox")
        XCTAssertTrue(state.log.contains("closing sessions of \(derived.pairId)"))
        XCTAssertTrue(state.isAdvertising)

        // A reconnect with the long-term key still works after the cancel.
        let longTerm = try XCTUnwrap(Data(base64Encoded: try XCTUnwrap(ack.payload.pairPsk)))
        let again = NearbyTestClient(port: port, identity: derived.pairId, psk: longTerm)
        try await waitUntil("reconnected") { again.isReady }
        let n2 = UUID().uuidString
        again.send(id: "h2", type: "hello", NearbyV1.Hello(pairId: derived.pairId, companionName: "Resumed iPad", nonce: n2,
                                                           proof: Pairing.helloProof(psk: longTerm, nonce: n2)))
        try await waitUntil("hello again") { again.lineCount >= 1 }
        XCTAssertEqual(try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.HelloAck>.self, from: again.allLines[0]).type, "hello_ack")

        // (4b) An expired attempt is not resumed.
        let before = state.log.count
        state.beginPairing(resuming: "654321", expiresAt: Date().addingTimeInterval(-1))
        XCTAssertNil(state.pairingCode)
        XCTAssertEqual(state.log.suffix(from: before), ["not resuming pairing code: already expired"])
        XCTAssertNil(state.coordinator.current)

        again.cancel()
        state.stopAdvertising()
        try? FileManager.default.removeItem(at: dir)
    }
}

// MARK: - generation API and activity summary (mac-nearby-transport refill)

final class PairingPersistedGenerationTests: XCTestCase {
    func testPersistedGenerationRefusesOlderAttemptsAndAllowsNewerRepair() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-pgen-\(UUID().uuidString)")
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        let c = PairingCoordinator(store: store)
        let p5 = c.begin(code: "123456", generation: 5)
        XCTAssertNotNil(c.confirmPairing(pairId: p5.derived.pairId, companionName: "first", generation: 5))
        XCTAssertEqual(store.pair(id: p5.derived.pairId)?.generation, 5, "generation persisted on the record")
        XCTAssertEqual(c.confirmedGeneration(pairId: p5.derived.pairId), 5)
        XCTAssertEqual(PairStore(url: store.url).pair(id: p5.derived.pairId)?.generation, 5, "survives reload")

        // An older attempt for the same pair_id (forced collision) is refused by the persisted record.
        let p3 = c.begin(code: "123456", generation: 3)
        XCTAssertNil(c.confirmPairing(pairId: p3.derived.pairId, companionName: "stale", generation: 3))
        XCTAssertEqual(c.lastRefusal, "pair \(p3.derived.pairId) was already confirmed by attempt 5; this attempt is 3")
        XCTAssertEqual(store.pair(id: p5.derived.pairId)?.companionName, "first")
        // The same generation cannot confirm twice either.
        _ = c.begin(code: "123456", generation: 5)
        XCTAssertNil(c.confirmPairing(pairId: p5.derived.pairId, companionName: "again", generation: 5))
        XCTAssertTrue(c.lastRefusal?.hasSuffix("this attempt is 5") ?? false)
        // A newer attempt re-pairs (the user showed a new code on purpose).
        let p6 = c.begin(code: "123456", generation: 6)
        XCTAssertNotNil(c.confirmPairing(pairId: p6.derived.pairId, companionName: "renewed", generation: 6))
        XCTAssertEqual(store.pair(id: p6.derived.pairId)?.generation, 6)
        XCTAssertEqual(store.pair(id: p6.derived.pairId)?.companionName, "renewed")
        try? FileManager.default.removeItem(at: dir)
    }

    func testCaptureSummaryPersistsAndNeverMentionsTheKey() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-act-\(UUID().uuidString)")
        let url = dir.appendingPathComponent("pairs.json")
        let store = PairStore(url: url)
        let psk = Pairing.mintLongTermPSK()
        XCTAssertTrue(store.upsert(PairRecord(pairId: "abcdef0123456789", psk: psk.base64EncodedString(), companionName: "iPad",
                                              createdAt: Date(timeIntervalSince1970: 1_700_000_000), lastSeenAt: nil, generation: 2)))
        var a = try XCTUnwrap(store.pair(id: "abcdef0123456789")).activity
        XCTAssertEqual(a.captureCount, 0)
        XCTAssertNil(a.lastCaptureAt)
        XCTAssertTrue(a.summary.hasPrefix("iPad (abcdef0123456789), never seen, 0 captures"), a.summary)
        let t1 = Date(timeIntervalSince1970: 1_700_000_100)
        store.recordCapture(pairId: "abcdef0123456789", captureId: "cap-1", at: t1)
        store.recordCapture(pairId: "abcdef0123456789", captureId: String(repeating: "x", count: 300), at: t1.addingTimeInterval(1))
        store.recordCapture(pairId: "nobody", captureId: "ignored")
        a = try XCTUnwrap(store.pair(id: "abcdef0123456789")).activity
        XCTAssertEqual(a.captureCount, 2)
        XCTAssertEqual(a.lastCaptureId?.count, 128, "last id is bounded")
        XCTAssertEqual(a.lastCaptureAt, t1.addingTimeInterval(1))
        XCTAssertEqual(a.lastSeenAt, t1.addingTimeInterval(1))
        XCTAssertEqual(a.generation, 2)
        XCTAssertTrue(a.summary.contains("2 captures, last xxxx"), a.summary)
        XCTAssertFalse(a.summary.contains(psk.base64EncodedString()))
        XCTAssertFalse("\(a)".contains(psk.base64EncodedString()), "the activity value carries no key material")

        // Persisted in pairs.json v2 and readable after reload; a file without the keys still decodes.
        let reloaded = PairStore(url: url)
        XCTAssertEqual(reloaded.pair(id: "abcdef0123456789")?.activity, a)
        let raw = try XCTUnwrap(String(data: Data(contentsOf: url), encoding: .utf8))
        XCTAssertTrue(raw.contains("\"capture_count\":2") || raw.contains("\"capture_count\" : 2"), raw)
        var obj = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any])
        var pairs = try XCTUnwrap(obj["pairs"] as? [[String: Any]])
        pairs[0].removeValue(forKey: "capture_count"); pairs[0].removeValue(forKey: "last_capture_at"); pairs[0].removeValue(forKey: "last_capture_id")
        obj["pairs"] = pairs
        try JSONSerialization.data(withJSONObject: obj).write(to: url)
        let stripped = PairStore(url: url)
        XCTAssertNil(stripped.loadError)
        XCTAssertEqual(stripped.pair(id: "abcdef0123456789")?.activity.captureCount, 0)
        try? FileManager.default.removeItem(at: dir)
    }
}

@MainActor
final class NearbyStateGenerationAPITests: XCTestCase {
    private func waitUntil(_ what: String, timeout: TimeInterval = 5, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
        throw NearbyWaitTimedOut()
    }

    func testFreshCodeCarriesTheJournalGenerationAndCapturesAreCounted() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-genapi-\(UUID().uuidString)")
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        let model = ShellModel()
        model.autoCompile = false
        let state = NearbyState(store: store, macName: "Gen Mac", loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        state.beginPairing(generation: 12)
        let code = try XCTUnwrap(state.pairingCode)
        XCTAssertEqual(state.coordinator.current?.generation, 12)
        XCTAssertNotNil(state.codeExpiresAt)
        XCTAssertTrue(state.log.contains { $0.hasPrefix("pairing code issued for") && $0.hasSuffix("(attempt 12)") }, "\(state.log)")
        XCTAssertFalse(state.log.contains { $0.contains("resumed") })
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let port = try XCTUnwrap(state.port)

        let derived = Pairing.derive(code: code, salt: store.salt)
        let client = NearbyTestClient(port: port, identity: derived.pairId, psk: derived.psk)
        try await waitUntil("client ready") { client.isReady }
        let nonce = UUID().uuidString
        client.send(id: "h", type: "hello", NearbyV1.Hello(pairId: derived.pairId, companionName: "Gen iPad", nonce: nonce,
                                                           proof: Pairing.helloProof(psk: derived.psk, nonce: nonce)))
        try await waitUntil("paired") { state.pairs.count == 1 && state.pairingCode == nil }
        XCTAssertEqual(state.pairs.first?.generation, 12, "the confirmed record persists the journal generation")
        XCTAssertEqual(state.coordinator.confirmedGeneration(pairId: derived.pairId), 12)
        XCTAssertEqual(state.activity[derived.pairId]?.captureCount, 0)
        try await waitUntil("connected") { state.connectedPairIds == [derived.pairId] }

        // Two distinct captures and one duplicate: the summary counts two.
        var fixture = try Data(contentsOf: NearbyListenerTests.fixtureURL)
        if fixture.last != 0x0A { fixture.append(0x0A) }
        client.send(fixture)
        client.send(fixture)
        var second = try RuntimeV1.decodeCaptureSubmit(fixture).payload
        second.captureId = "fixture-capture-2"
        client.send(id: "c2", type: "capture_submit", second)
        try await waitUntil("three acks") { client.lineCount >= 4 }
        try await waitUntil("summary updated") { state.activity[derived.pairId]?.captureCount == 2 }
        let a = try XCTUnwrap(state.activity[derived.pairId])
        // Captures are validated concurrently, so either may have been acknowledged last.
        XCTAssertTrue(["fixture-capture-1", "fixture-capture-2"].contains(a.lastCaptureId ?? ""), String(describing: a.lastCaptureId))
        XCTAssertNotNil(a.lastCaptureAt)
        XCTAssertNotNil(a.lastSeenAt)
        XCTAssertEqual(a.generation, 12)
        XCTAssertEqual(PairStore(url: store.url).pair(id: derived.pairId)?.captureCount, 2, "persisted")
        XCTAssertFalse(state.log.joined().contains(store.pair(id: derived.pairId)!.psk))
        client.cancel()
        state.stopAdvertising()
        try? FileManager.default.removeItem(at: dir)
    }
}
