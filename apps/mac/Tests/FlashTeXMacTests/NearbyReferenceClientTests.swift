import Network
import XCTest
import FlashTeXProtocol
import NearbyClient
@testable import FlashTeXMac

/// The reference companion client (apps/mac/tools/nearby-client) against the
/// real Mac stack: Bonjour advertisement + TXT, TLS-PSK, bootstrap pairing,
/// long-term key hand-over, capture into the ShellModel inbox, forget.
/// The CLI commands run in-process (`NearbyCLI.run`), exactly what
/// `.build/debug/nearby-client` executes.
@MainActor
final class NearbyReferenceClientTests: XCTestCase {
    static let fixturePNG: Data = {
        // The 1×1 PNG from protocol/fixtures/capture-submission.json.
        let env = try! RuntimeV1.decodeCaptureSubmit(Data(contentsOf: NearbyListenerTests.fixtureURL))
        return Data(base64Encoded: env.payload.image.dataBase64)!
    }()

    private var tmp: URL!
    override func setUp() {
        CaptureInboxFeature.caretDestinationOverride = false // these cases assert the explicit-pin semantics (destination_query: nothing pinned → null)
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-ref-\(UUID().uuidString)")
        try! FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
    }
    override func tearDown() { CaptureInboxFeature.caretDestinationOverride = nil; try? FileManager.default.removeItem(at: tmp) }

    private func cli(_ args: [String]) async -> (code: Int32, out: [String]) {
        var out: [String] = []
        let code = await NearbyCLI.run(args) { out.append($0) }
        return (code, out)
    }

    func testCLIPairsSendsRetriesAndIsForgottenThroughNearbyState() async throws {
        let store = PairStore(url: tmp.appendingPathComponent("mac-pairs.json"))
        let model = ShellModel()
        model.caretUTF16 = 6
        model.pinAnchorAtCaret()
        let anchor = try XCTUnwrap(model.nearbyDestination)
        let macName = "FlashTeX Ref \(UUID().uuidString.prefix(6))"
        let state = NearbyState(store: store, macName: macName, loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        state.beginPairing()
        let code = try XCTUnwrap(state.pairingCode)
        try await waitUntil("restarted with bootstrap key") { state.log.filter { $0.hasPrefix("ready on port") }.count >= 2 }
        let fp = state.fingerprint
        let clientStore = tmp.appendingPathComponent("client-pairs.json").path
        let png = tmp.appendingPathComponent("dot.png")
        try Self.fixturePNG.write(to: png)

        // pair: browse by fp, read TXT salt, bootstrap TLS, hello → pair_psk.
        let pair = await cli(["pair", "--code", code, "--name", "Reference iPad", "--mac", fp, "--seconds", "10", "--store", clientStore, "-v"])
        XCTAssertEqual(pair.code, 0, pair.out.joined(separator: "\n"))
        XCTAssertEqual(pair.out.last, "paired")
        XCTAssertTrue(pair.out.contains { $0.hasPrefix("found \(macName) (fp \(fp), v=1)") }, "\(pair.out)")
        XCTAssertTrue(pair.out.contains("tls: 1.2 suite 0xa8"), "\(pair.out)")
        XCTAssertTrue(pair.out.contains { $0.contains("destination: \(anchor.destinationId) (\(anchor.projectId)/\(anchor.path) @ rev \(anchor.baseRevision))") }, "\(pair.out)")
        try await waitUntil("pair stored on the Mac") { state.pairs.count == 1 && state.pairingCode == nil }
        XCTAssertEqual(state.pairs.first?.companionName, "Reference iPad")
        let stored = try XCTUnwrap(try PairFile(url: URL(fileURLWithPath: clientStore)).pair(fingerprint: fp))
        XCTAssertEqual(stored.pairId, state.pairs.first?.pairId)
        XCTAssertEqual(stored.psk, state.pairs.first?.pskData, "client stored the long-term key the Mac minted")
        XCTAssertEqual(stored.macName, macName)
        // Mac restarted with the long-term key only.
        try await waitUntil("restarted with long-term key") { state.log.filter { $0.hasPrefix("ready on port") }.count >= 3 }

        // send: reconnect with pair_psk, capture into the ShellModel inbox.
        let send = await cli(["send", "--image", png.path, "--instructions", "transcribe the box", "--capture-id", "ref-cap-1",
                              "--mac", fp, "--seconds", "10", "--store", clientStore, "-v"])
        XCTAssertEqual(send.code, 0, send.out.joined(separator: "\n"))
        XCTAssertTrue(send.out.contains("capture_received ref-cap-1: durable=false has_proposal=false applied=false"), "\(send.out)")
        XCTAssertTrue(send.out.contains { $0.hasPrefix("<< ") && $0.contains("\"type\":\"hello_ack\"") && !$0.contains("pair_psk") },
                      "no key hand-over on a long-term connection: \(send.out)")
        try await waitUntil("inbox") { model.nearbyInbox.lastCaptureId == "ref-cap-1" }
        let received = try XCTUnwrap(model.nearbyInbox.received.first)
        XCTAssertEqual(received.destinationId, anchor.destinationId)
        XCTAssertEqual(received.baseRevision, anchor.baseRevision)
        XCTAssertEqual(received.image.mimeType, "image/png")
        XCTAssertEqual(Data(base64Encoded: received.image.dataBase64), Self.fixturePNG)
        XCTAssertEqual(received.instructions, "transcribe the box")
        XCTAssertEqual(state.lastReceivedCaptureId, "ref-cap-1")

        // Identical retry (same capture_id + payload): acknowledged again, stored once.
        let retry = await cli(["send", "--image", png.path, "--instructions", "transcribe the box", "--capture-id", "ref-cap-1",
                               "--mac", fp, "--store", clientStore])
        XCTAssertEqual(retry.code, 0, retry.out.joined(separator: "\n"))
        XCTAssertEqual(model.nearbyInbox.received.count, 1)
        // Same id, different payload: capture_id_conflict, session stays open (CLI reports it).
        let conflict = await cli(["send", "--image", png.path, "--instructions", "something else", "--capture-id", "ref-cap-1",
                                  "--mac", fp, "--store", clientStore])
        XCTAssertEqual(conflict.code, 2)
        XCTAssertTrue(conflict.out.contains { $0.contains("error capture_id_conflict") }, "\(conflict.out)")

        // status: the pairing is listed and the Mac is visible.
        let status = await cli(["status", "--seconds", "3", "--store", clientStore])
        XCTAssertEqual(status.code, 0)
        XCTAssertTrue(status.out.contains { $0.hasPrefix("\(macName) fp=\(fp) pair_id=\(stored.pairId) as \"Reference iPad\"") && $0.hasSuffix("visible now as \"\(macName)\"") }, "\(status.out)")

        // Forget on the Mac: the stored key is refused at the handshake.
        state.forget(pairId: stored.pairId)
        try await waitUntil("restarted without the key") { state.log.filter { $0.hasPrefix("ready on port") }.count >= 4 }
        let refused = await cli(["send", "--image", png.path, "--mac", fp, "--store", clientStore])
        XCTAssertEqual(refused.code, 3, "refused key → re-pair exit code: \(refused.out)")
        XCTAssertTrue(refused.out.contains { $0.hasPrefix("error: TLS-PSK handshake failed") }, "\(refused.out)")
        XCTAssertEqual(model.nearbyInbox.received.count, 1)
        state.stopAdvertising()
    }

    func testCLIDirectModeAndListenerRefusalsAgainstRealListener() async throws {
        // Direct --host/--port (no Bonjour) against a bare NearbyListener with
        // a bootstrap key, then a wrong-proof hello and a pre-hello message.
        let store = PairStore(url: tmp.appendingPathComponent("mac-pairs.json"))
        let coordinator = PairingCoordinator(store: store)
        _ = coordinator.begin(code: "123456")
        let bootstrap = try XCTUnwrap(coordinator.bootstrapEntry)
        let sink = RecordingSink()
        let dest = FixedDestinations(.init(destinationId: "direct-anchor", projectId: "demo", path: "main.tex", baseRevision: 2))
        let h = ListenerHarness(psks: [bootstrap], sink: sink, destinations: dest, pairing: coordinator)
        try h.start()
        defer { h.stop() }
        let clientStore = tmp.appendingPathComponent("client-pairs.json").path

        let pair = await cli(["pair", "--code", "123456", "--name", "Direct iPad", "--host", "127.0.0.1", "--port", "\(h.port)",
                              "--salt", Pairing.hex(store.salt), "--store", clientStore, "-v"])
        XCTAssertEqual(pair.code, 0, pair.out.joined(separator: "\n"))
        XCTAssertTrue(pair.out.contains { $0.contains("pair_id \(bootstrap.identity)") })
        let record = try XCTUnwrap(store.pair(id: bootstrap.identity))
        XCTAssertEqual(record.companionName, "Direct iPad")
        let stored = try XCTUnwrap(try PairFile(url: URL(fileURLWithPath: clientStore)).pairs.first)
        XCTAssertEqual(stored.psk, record.pskData)
        XCTAssertEqual(stored.fingerprint, Pairing.fingerprint(salt: store.salt), "fp computed from --salt matches the Mac's")
        XCTAssertTrue(h.snapshot.contains(.hello(pairId: bootstrap.identity, companionName: "Direct iPad", bootstrap: true)))

        // Library level, same listener (still holding only the bootstrap key):
        // right key, wrong pair_id claim → pair_mismatch then close.
        let liar = NearbyConnection(host: "127.0.0.1", port: h.port, pairId: bootstrap.identity, psk: bootstrap.key)
        try await liar.connect()
        XCTAssertEqual(liar.negotiated?.tlsv12, true)
        XCTAssertEqual(liar.negotiated?.suite, 0x00A8)
        let lie = NearbyWire.Hello(pairId: "0000000000000000", companionName: "x", nonce: "n",
                                   proof: NearbyCrypto.helloProof(psk: bootstrap.key, nonce: "n"))
        do {
            let _: NearbyWire.Envelope<NearbyWire.HelloAck> = try await liar.request(type: "hello", lie, expecting: "hello_ack")
            XCTFail("expected pair_mismatch")
        } catch let e as NearbyError {
            XCTAssertEqual(e, .remote(code: "pair_mismatch", message: "pair_id is unknown or proof does not match its key"))
            XCTAssertTrue(e.isClosing)
        }
        // A message before hello → hello_required.
        let eager = NearbyConnection(host: "127.0.0.1", port: h.port, pairId: bootstrap.identity, psk: bootstrap.key)
        try await eager.connect()
        do {
            _ = try await eager.destinationQuery()
            XCTFail("expected hello_required")
        } catch let e as NearbyError {
            XCTAssertEqual(e, .remote(code: "hello_required", message: "first message must be hello"))
        }
        // The used code is consumed: a second bootstrap hello gets pairing_expired.
        let again = NearbyConnection(host: "127.0.0.1", port: h.port, pairId: bootstrap.identity, psk: bootstrap.key)
        try await again.connect()
        do {
            try await again.hello(companionName: "second device", expectPairPsk: true)
            XCTFail("expected pairing_expired")
        } catch let e as NearbyError {
            XCTAssertEqual(e, .remote(code: "pairing_expired", message: "pairing code is no longer valid"))
        }
        liar.close(); eager.close(); again.close()
    }

    // MARK: bounded reconnect against the real listener

    /// Forwards to the ShellModel inbox but swallows the first acknowledgement
    /// and cuts the listener instead: the capture *was* delivered, the
    /// companion never learns it. The retry must carry the same capture_id.
    final class AckDroppingSink: CaptureSink {
        let inner: CaptureSink
        let onDrop: () -> Void
        private let lock = NSLock()
        private var dropped = false
        private(set) var deliveries = 0
        private(set) var droppedAt: Date?
        init(_ inner: CaptureSink, onDrop: @escaping () -> Void) { self.inner = inner; self.onDrop = onDrop }
        func submit(_ envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>, reply: @escaping (Data) -> Void) {
            let first: Bool = lock.withLock { deliveries += 1; if dropped { return false }; dropped = true; droppedAt = Date(); return true }
            if first {
                inner.submit(envelope) { _ in self.onDrop() } // stored on the Mac; ack thrown away
            } else {
                inner.submit(envelope, reply: reply)
            }
        }
    }

    final class EventLog: @unchecked Sendable {
        private let lock = NSLock()
        private(set) var events: [(Date, NearbyReconnector.Event)] = []
        func record(_ e: NearbyReconnector.Event) { lock.withLock { events.append((Date(), e)) } }
        var list: [NearbyReconnector.Event] { lock.withLock { events.map(\.1) } }
        var attempts: [Int] { list.compactMap { if case .attempt(let n, _) = $0 { return n }; return nil } }
    }

    static let longTermPSK = Data(repeating: 0x3C, count: 32)
    static let longTermPairId = "feedfacefeedface"
    func longTermPair(macName: String = "Test Mac") -> PairedMac {
        PairedMac(fingerprint: "test-fp", macName: macName, pairId: Self.longTermPairId,
                  pairPsk: Self.longTermPSK.base64EncodedString(), companionName: "Reconnecting iPad")
    }
    var longTermEntry: NearbyListener.PSKEntry { .init(identity: Self.longTermPairId, key: Self.longTermPSK, isBootstrap: false) }

    /// Duplicate capture delivery: the listener drops mid-capture after the
    /// ShellModel inbox stored it; the companion reconnects (bounded backoff)
    /// to a listener restarted on the same port and re-sends the identical
    /// capture_id; the inbox acknowledges the duplicate and keeps one copy.
    func testReconnectorResendsSameCaptureAfterListenerDropAndSamePortRestart() async throws {
        let model = ShellModel()
        model.caretUTF16 = 6
        model.pinAnchorAtCaret()
        let anchor = try XCTUnwrap(model.nearbyDestination)
        let restarted = XCTestExpectation(description: "listener restarted")
        var h1: ListenerHarness!
        var h2: ListenerHarness?
        let sink = AckDroppingSink(model) {
            // The listener goes away with the ack still unsent; bring a new one up on
            // the same port (what NearbyState does after a key-table change/restart).
            h1.listener.stop { DispatchQueue.main.async { restarted.fulfill() } }
        }
        h1 = ListenerHarness(psks: [longTermEntry], sink: sink, destinations: model)
        try h1.start()
        let port = h1.port

        let log = EventLog()
        let policy = ReconnectPolicy(maxAttempts: 6, initialDelay: 0.2, maxDelay: 1, jitter: 0, connectTimeout: 3, requestTimeout: 10)
        let reconnector = NearbyReconnector(pair: longTermPair(), policy: policy,
                                            endpoints: { .hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!) },
                                            onEvent: log.record)
        let session = try await reconnector.connect()
        XCTAssertEqual(session.destination?.destinationId, anchor.destinationId)
        let capture = try session.makeCapture(captureId: "ref-dup-1", image: Self.fixturePNG, mimeType: "image/png", instructions: "once, please")
        let started = Date()
        let submitTask = Task { try await reconnector.submit(capture) }
        // Restart the listener on the same port once the first delivery was swallowed.
        await fulfillment(of: [restarted], timeout: 10)
        h2 = ListenerHarness(psks: [longTermEntry], sink: model, destinations: model, port: port)
        try h2!.start()
        let restartedAt = Date()
        let ack = try await submitTask.value
        let elapsed = Date().timeIntervalSince(started)
        defer { h2?.stop() }

        XCTAssertEqual(ack.captureId, "ref-dup-1")
        XCTAssertFalse(ack.durable)
        XCTAssertEqual(sink.deliveries, 1, "the first listener saw exactly one delivery")
        XCTAssertEqual(model.nearbyInbox.received.count, 1, "delivered twice, stored once")
        XCTAssertEqual(model.nearbyInbox.received.first?.captureId, "ref-dup-1")
        XCTAssertEqual(model.nearbyInbox.received.first?.instructions, "once, please")
        XCTAssertEqual(model.nearbyInbox.lastNote, "Duplicate ref-dup-1 acknowledged again.")
        let attempts = await reconnector.attemptsMade
        XCTAssertEqual(attempts, 2, "one reconnect: \(log.list)")
        XCTAssertEqual(log.attempts, [1, 2])
        XCTAssertTrue(log.list.contains { if case .failed(1, let why, let wait) = $0 { return why.hasPrefix("connection closed:") && wait == 0.2 }; return false }, "\(log.list)")
        XCTAssertTrue(log.list.contains { if case .connected("Test Mac", 2, let d) = $0 { return d?.destinationId == anchor.destinationId }; return false }, "\(log.list)")
        XCTAssertTrue(h2!.snapshot.contains(.capture(captureId: "ref-dup-1")))
        XCTAssertTrue(h1.snapshot.contains { if case .connectionClosed(Self.longTermPairId?, "listener stopped") = $0 { return true }; return false }, "\(h1.snapshot)")
        let sinceRestart = Date().timeIntervalSince(restartedAt)
        print("measured: real-listener drop→duplicate-ack in \(String(format: "%.3f", elapsed))s total; listener back at +\(String(format: "%.3f", restartedAt.timeIntervalSince(started)))s; ack \(String(format: "%.3f", sinceRestart))s after restart; backoff 0.2s")
        XCTAssertLessThan(elapsed, 8)
        await reconnector.shutdown()
    }

    /// Revoked pairing: the Mac forgets the companion (NearbyState.forget)
    /// while a session is live. The live session is closed by the Mac, the
    /// reconnect is refused at the TLS handshake, and the client stops after
    /// that single retry with a terminal, re-pair error — no retry storm.
    func testRevokedPairingIsTerminalAfterOneRefusedReconnect() async throws {
        let store = PairStore(url: tmp.appendingPathComponent("mac-pairs.json"))
        let model = ShellModel()
        model.caretUTF16 = 6
        model.pinAnchorAtCaret()
        let state = NearbyState(store: store, macName: "FlashTeX Revoke", loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        state.beginPairing()
        let code = try XCTUnwrap(state.pairingCode)
        try await waitUntil("restarted with bootstrap key") { state.log.filter { $0.hasPrefix("ready on port") }.count >= 2 }
        let port = try XCTUnwrap(state.port)
        let ep = NWEndpoint.hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!)
        let (pair, boot) = try await NearbyClient.pair(endpoint: ep, salt: store.salt, fingerprint: state.fingerprint, macName: state.macName,
                                                       code: code, companionName: "Revoked iPad")
        boot.close()
        try await waitUntil("pair stored") { state.pairs.count == 1 && state.pairingCode == nil }
        try await waitUntil("restarted with long-term key") { state.log.filter { $0.hasPrefix("ready on port") }.count >= 3 }
        XCTAssertEqual(state.port, port, "same port across the key-table restart")

        let log = EventLog()
        let policy = ReconnectPolicy(maxAttempts: 5, initialDelay: 0.1, maxDelay: 0.1, jitter: 0, connectTimeout: 3, requestTimeout: 10)
        let reconnector = NearbyReconnector(pair: pair, policy: policy, endpoints: { ep }, onEvent: log.record)
        let session = try await reconnector.connect()
        try await waitUntil("hello seen") { state.connectedPairIds == [pair.pairId] }
        let capture = try session.makeCapture(captureId: "ref-revoked-1", image: Self.fixturePNG, mimeType: "image/png", instructions: "")
        _ = try await reconnector.submit(capture)
        XCTAssertEqual(model.nearbyInbox.received.count, 1)

        // Forget on the Mac: the live session is dropped and the key is gone.
        state.forget(pairId: pair.pairId)
        try await waitUntil("restarted without the key") { state.log.filter { $0.hasPrefix("ready on port") }.count >= 4 }
        try await waitUntil("session closed by the Mac") { !session.isOpen }
        XCTAssertTrue(state.log.contains("closed \(pair.pairId): pairing forgotten"), "\(state.log)")
        let started = Date()
        do {
            _ = try await reconnector.submit(capture)
            XCTFail("a forgotten pairing must not deliver")
        } catch let e as NearbyError {
            guard case .handshakeFailed = e else { return XCTFail("unexpected \(e)") }
            XCTAssertTrue(e.needsRepair)
            XCTAssertFalse(e.isRetryable)
        }
        let elapsed = Date().timeIntervalSince(started)
        let attempts = await reconnector.attemptsMade
        XCTAssertEqual(attempts, 2, "initial dial + exactly one refused reconnect: \(log.list)")
        XCTAssertEqual(log.attempts, [1, 2])
        XCTAssertEqual(log.list.filter { if case .failed = $0 { return true }; return false }.count, 0,
                       "the dead session is replaced without a backoff wait, and the refusal is terminal: \(log.list)")
        XCTAssertTrue(log.list.contains { if case .gaveUp(let why) = $0 { return why.hasPrefix("TLS-PSK handshake failed") }; return false }, "\(log.list)")
        XCTAssertEqual(model.nearbyInbox.received.count, 1, "nothing new reached the inbox")
        print("measured: revoked pairing reported terminal \(String(format: "%.3f", elapsed))s after the retry began (1 refused handshake)")

        // The CLI says the same, with exit 3 and no second attempt.
        let clientStore = tmp.appendingPathComponent("client-pairs.json").path
        try PairFile(url: URL(fileURLWithPath: clientStore)).upsert(pair)
        let png = tmp.appendingPathComponent("dot.png")
        try Self.fixturePNG.write(to: png)
        let refused = await cli(["send", "--image", png.path, "--host", "127.0.0.1", "--port", "\(port)", "--attempts", "5", "--retry-delay", "0",
                                 "--store", clientStore])
        XCTAssertEqual(refused.code, 3, refused.out.joined(separator: "\n"))
        XCTAssertFalse(refused.out.contains { $0.hasPrefix("attempt 2") }, "\(refused.out)")
        XCTAssertTrue(refused.out.contains { $0.contains("run `nearby-client pair` again (exit 3)") }, "\(refused.out)")
        await reconnector.shutdown()
        state.stopAdvertising()
    }

    /// `nearby-client doctor` against the real stack: healthy through Bonjour
    /// after pairing; `handshake_refused` (exit 3) once the Mac forgets the
    /// pairing; `not_advertised` (exit 1) once it stops advertising. Read-only:
    /// nothing reaches the inbox.
    func testDoctorReportsExactCodesAgainstNearbyState() async throws {
        let store = PairStore(url: tmp.appendingPathComponent("mac-pairs.json"))
        let model = ShellModel()
        model.caretUTF16 = 6
        model.pinAnchorAtCaret()
        let anchor = try XCTUnwrap(model.nearbyDestination)
        let macName = "FlashTeX Doctor \(UUID().uuidString.prefix(6))"
        let state = NearbyState(store: store, macName: macName, loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        state.beginPairing()
        let code = try XCTUnwrap(state.pairingCode)
        try await waitUntil("restarted with bootstrap key") { state.log.filter { $0.hasPrefix("ready on port") }.count >= 2 }
        let port = try XCTUnwrap(state.port)
        let ep = NWEndpoint.hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!)
        let (pair, boot) = try await NearbyClient.pair(endpoint: ep, salt: store.salt, fingerprint: state.fingerprint, macName: state.macName,
                                                       code: code, companionName: "Doctor iPad")
        boot.close()
        try await waitUntil("pair stored") { state.pairs.count == 1 && state.pairingCode == nil }
        try await waitUntil("restarted with long-term key") { state.log.filter { $0.hasPrefix("ready on port") }.count >= 3 }
        let clientStore = tmp.appendingPathComponent("client-pairs.json").path
        try PairFile(url: URL(fileURLWithPath: clientStore)).upsert(pair)

        let healthy = await cli(["doctor", "--store", clientStore, "--seconds", "10", "--json"])
        XCTAssertEqual(healthy.code, 0, healthy.out.joined(separator: "\n"))
        XCTAssertTrue(healthy.out.contains { $0.hasPrefix("check discovery: ok — \(macName) fp=\(state.fingerprint) v=1 at") }, "\(healthy.out)")
        XCTAssertTrue(healthy.out.contains("check tls: ok — 1.2 suite 0xa8"), "\(healthy.out)")
        XCTAssertTrue(healthy.out.contains("check hello: ok — hello_ack from \"\(macName)\""), "\(healthy.out)")
        XCTAssertTrue(healthy.out.contains("check destination: ok — \(anchor.destinationId) (\(anchor.projectId)/\(anchor.path) @ rev \(anchor.baseRevision))"), "\(healthy.out)")
        XCTAssertTrue(healthy.out.contains("doctor: healthy (exit 0)"), "\(healthy.out)")
        let report = try JSONDecoder().decode(NearbyDoctor.Report.self, from: Data(try XCTUnwrap(healthy.out.last(where: { $0.hasPrefix("{") })).utf8))
        XCTAssertEqual(report.checks.map(\.name), ["store", "discovery", "connect", "tls", "hello", "destination"])
        XCTAssertTrue(model.nearbyInbox.received.isEmpty, "doctor sends no capture")
        try await waitUntil("doctor session closed") { state.connectedPairIds.isEmpty }

        state.forget(pairId: pair.pairId)
        try await waitUntil("restarted without the key") { state.log.filter { $0.hasPrefix("ready on port") }.count >= 4 }
        let revoked = await cli(["doctor", "--store", clientStore, "--host", "127.0.0.1", "--port", "\(port)"])
        XCTAssertEqual(revoked.code, 3, revoked.out.joined(separator: "\n"))
        XCTAssertTrue(revoked.out.contains { $0.hasPrefix("check connect: FAIL code=handshake_refused — TLS-PSK refused for pair_id \(pair.pairId)") }, "\(revoked.out)")
        XCTAssertTrue(revoked.out.contains { $0.hasPrefix("doctor: FAIL code=handshake_refused (exit 3)") }, "\(revoked.out)")

        state.stopAdvertising()
        try await waitUntil("stopped") { !state.isAdvertising }
        let gone = await cli(["doctor", "--store", clientStore, "--seconds", "2"])
        XCTAssertEqual(gone.code, 1, gone.out.joined(separator: "\n"))
        XCTAssertTrue(gone.out.contains { $0.hasPrefix("check discovery: FAIL code=not_advertised — no _flashtex._tcp service with fp \(state.fingerprint)") }, "\(gone.out)")
        XCTAssertTrue(model.nearbyInbox.received.isEmpty)
    }

    /// Revoked destination: the Mac's pinned insertion point disappears
    /// (project replaced) or moves (re-pinned) between building a capture and
    /// delivering it. The client refuses with `destinationChanged` — on a
    /// reused session via `destination_query`, on a fresh one via `hello_ack`
    /// — and nothing lands in the inbox until a capture is rebuilt for the
    /// current anchor.
    func testRevokedDestinationIsTerminalAgainstShellModel() async throws {
        let model = ShellModel()
        model.caretUTF16 = 6
        model.pinAnchorAtCaret()
        let first = try XCTUnwrap(model.nearbyDestination)
        let h = ListenerHarness(psks: [longTermEntry], sink: model, destinations: model)
        try h.start()
        defer { h.stop() }
        let port = h.port
        let log = EventLog()
        let reconnector = NearbyReconnector(pair: longTermPair(), policy: .immediate,
                                            endpoints: { .hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!) },
                                            onEvent: log.record)
        let session = try await reconnector.connect()
        // `caret_context` rides along additively (nearby-v1); this assertion is
        // about the pin the reference client binds to.
        XCTAssertEqual(session.destination?.destinationId, first.destinationId)
        XCTAssertEqual(session.destination?.projectId, first.projectId)
        XCTAssertEqual(session.destination?.path, first.path)
        XCTAssertEqual(session.destination?.baseRevision, first.baseRevision)
        let stale = try session.makeCapture(captureId: "ref-stale-1", image: Self.fixturePNG, mimeType: "image/png", instructions: "stale")

        // Unpinned: the project is replaced, the anchor is gone.
        model.replaceProject(entryText: "\\documentclass{article}\\begin{document}Replaced.\\end{document}")
        XCTAssertNil(model.nearbyDestination)
        do { _ = try await reconnector.submit(stale); XCTFail() } catch let e as NearbyError {
            XCTAssertEqual(e, .destinationChanged(captureDestination: "\(first.destinationId) @ rev \(first.baseRevision)", current: nil))
        }
        XCTAssertEqual(model.nearbyInbox.received.count, 0)
        XCTAssertTrue(session.isOpen, "checked with destination_query on the live session")

        // Re-pinned elsewhere: a different destination id and revision.
        model.caretUTF16 = 3
        model.pinAnchorAtCaret()
        let second = try XCTUnwrap(model.nearbyDestination)
        XCTAssertNotEqual(second.destinationId, first.destinationId)
        do { _ = try await reconnector.submit(stale); XCTFail() } catch let e as NearbyError {
            XCTAssertEqual(e, .destinationChanged(captureDestination: "\(first.destinationId) @ rev \(first.baseRevision)",
                                                  current: "\(second.destinationId) @ rev \(second.baseRevision)"))
        }
        XCTAssertEqual(model.nearbyInbox.received.count, 0)

        // Same check on a fresh connection (hello_ack path): drop the session first.
        await reconnector.close()
        do { _ = try await reconnector.submit(stale); XCTFail() } catch let e as NearbyError {
            guard case .destinationChanged(_, let current) = e else { return XCTFail("unexpected \(e)") }
            XCTAssertEqual(current, "\(second.destinationId) @ rev \(second.baseRevision)")
        }
        XCTAssertEqual(log.attempts, [1, 2])
        XCTAssertEqual(model.nearbyInbox.received.count, 0)

        // Rebuilt for the current anchor: delivered.
        let current = await reconnector.currentSession
        let fresh = try XCTUnwrap(current)
        let rebuilt = try fresh.makeCapture(captureId: "ref-fresh-1", image: Self.fixturePNG, mimeType: "image/png", instructions: "fresh")
        XCTAssertEqual(rebuilt.destinationId, second.destinationId)
        let ack = try await reconnector.submit(rebuilt)
        XCTAssertEqual(ack.captureId, "ref-fresh-1")
        XCTAssertEqual(model.nearbyInbox.received.map(\.captureId), ["ref-fresh-1"])
        XCTAssertEqual(model.nearbyInbox.received.first?.destinationId, second.destinationId)
        // The user can still force a destination (CLI --destination-id): the inbox takes it.
        _ = try await reconnector.submit(stale, requireCurrentDestination: false)
        XCTAssertEqual(model.nearbyInbox.received.map(\.captureId), ["ref-fresh-1", "ref-stale-1"])
        await reconnector.shutdown()
    }

    // MARK: receive caps and validation codes (proposal §4) against the real listener

    /// Records captures and holds their acknowledgements until released, so
    /// bytes stay "accepted but unacknowledged" on the Mac.
    final class HoldingSink: CaptureSink {
        private let lock = NSLock()
        private var held: [(id: String, captureId: String, reply: (Data) -> Void)] = []
        private var holding = true
        private(set) var deliveries: [String] = []
        private static func ack(_ id: String, _ captureId: String) -> Data {
            NearbyV1.line(id: id, type: "capture_received", NearbyV1.CaptureReceived(captureId: captureId, durable: false, hasProposal: false, applied: false))
        }
        func submit(_ envelope: RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>, reply: @escaping (Data) -> Void) {
            let hold: Bool = lock.withLock {
                deliveries.append(envelope.payload.captureId)
                if holding { held.append((envelope.id, envelope.payload.captureId, reply)) }
                return holding
            }
            if !hold { reply(Self.ack(envelope.id, envelope.payload.captureId)) }
        }
        var heldCount: Int { lock.withLock { held.count } }
        /// Acknowledges everything held and acknowledges later deliveries at once.
        func releaseAll() {
            let h: [(id: String, captureId: String, reply: (Data) -> Void)] = lock.withLock { holding = false; let x = held; held = []; return x }
            for e in h { e.reply(Self.ack(e.id, e.captureId)) }
        }
    }

    func testBackpressureCodesAreRetriedOnTheSameSessionAgainstRealListener() async throws {
        let frame = try NearbyWire.line(id: "c-00000000-0000-0000-0000-000000000000", type: "capture_submit",
                                        NearbyWire.CaptureSubmit(captureId: "ref-bp-1", destinationId: "direct-anchor", baseRevision: 2,
                                                                 image: .init(mimeType: "image/png", dataBase64: Self.fixturePNG.base64EncodedString()),
                                                                 instructions: "")).count
        for (code, limits) in [("too_many_in_flight", { (l: inout NearbyReceiveLimits) in l.maxSessionBytesInFlight = frame + frame / 2 }),
                               ("inbox_full", { (l: inout NearbyReceiveLimits) in l.maxInboxBytes = frame + frame / 2 })] {
            var l = NearbyReceiveLimits()
            limits(&l)
            let sink = HoldingSink()
            let dest = FixedDestinations(.init(destinationId: "direct-anchor", projectId: "demo", path: "main.tex", baseRevision: 2))
            let h = ListenerHarness(psks: [longTermEntry], sink: sink, destinations: dest, limits: l)
            try h.start()
            let log = EventLog()
            let policy = ReconnectPolicy(maxAttempts: 4, initialDelay: 0.05, maxDelay: 0.05, jitter: 0, connectTimeout: 3, requestTimeout: 5)
            let r = NearbyReconnector(pair: longTermPair(), policy: policy,
                                      endpoints: { [port = h.port] in .hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!) }, onEvent: log.record)
            let session = try await r.connect()
            let first = try session.makeCapture(captureId: "ref-bp-1", image: Self.fixturePNG, mimeType: "image/png", instructions: "")
            let second = try session.makeCapture(captureId: "ref-bp-2", image: Self.fixturePNG, mimeType: "image/png", instructions: "")
            // One frame is accepted and held unacknowledged; the next exceeds the cap.
            let heldTask = Task { try await session.connection.submitCapture(first) }
            try await waitUntil("first delivered") { sink.heldCount == 1 }
            let started = Date()
            let retried = Task { try await r.submit(second) }
            try await waitUntil("client waiting for acks (\(code))") {
                log.list.contains { if case .waitingForAcks(1) = $0 { return true }; return false }
            }
            XCTAssertTrue(h.snapshot.contains { if case .captureRefused(Self.longTermPairId?, nil, code, _) = $0 { return true }; return false },
                          "\(code) refused with the code: \(h.snapshot)")
            XCTAssertEqual(sink.deliveries, ["ref-bp-1"], "\(code): the second capture never reached the sink")
            sink.releaseAll()
            let firstAck = try await heldTask.value
            let secondAck = try await retried.value
            let elapsed = Date().timeIntervalSince(started)
            XCTAssertEqual(firstAck.captureId, "ref-bp-1")
            XCTAssertEqual(secondAck.captureId, "ref-bp-2")
            XCTAssertEqual(sink.deliveries, ["ref-bp-1", "ref-bp-2"], "\(code): same capture_id re-sent once the ack went out")
            let made = await r.attemptsMade
            XCTAssertEqual(made, 1, "\(code): no reconnect")
            XCTAssertTrue(log.list.contains { if case .failed(1, let why, 0.05) = $0 { return why.contains(code) }; return false }, "\(log.list)")
            XCTAssertFalse(log.list.contains { if case .disconnected = $0 { return true }; return false }, "\(code): session kept")
            XCTAssertTrue(session.isOpen)
            print("measured: real-listener \(code) → wait for acks → same-session re-send acknowledged in \(String(format: "%.3f", elapsed))s")
            sink.releaseAll()
            await r.shutdown()
            h.stop()
        }
    }

    func testTooManySessionsIsRetriedAfterClosingTheOlderConnection() async throws {
        var limits = NearbyReceiveLimits()
        limits.maxSessionsPerPeer = 1
        let sink = RecordingSink()
        let dest = FixedDestinations(.init(destinationId: "direct-anchor", projectId: "demo", path: "main.tex", baseRevision: 2))
        let h = ListenerHarness(psks: [longTermEntry], sink: sink, destinations: dest, limits: limits)
        try h.start()
        defer { h.stop() }
        // An older connection of the same pairing already holds the one session.
        let older = NearbyConnection(host: "127.0.0.1", port: h.port, pairId: Self.longTermPairId, psk: Self.longTermPSK)
        try await older.connect()
        try await older.hello(companionName: "older")
        let log = EventLog()
        let closedOlder = XCTestExpectation(description: "older closed during backoff")
        let policy = ReconnectPolicy(maxAttempts: 4, initialDelay: 0.1, maxDelay: 0.1, jitter: 0, connectTimeout: 3, requestTimeout: 5)
        let r = NearbyReconnector(pair: longTermPair(), policy: policy,
                                  endpoints: { [port = h.port] in .hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!) },
                                  onEvent: log.record,
                                  sleep: { s in
                                      // What a companion does on too_many_sessions: close its older connections, then retry.
                                      older.close(); closedOlder.fulfill()
                                      try await Task.sleep(nanoseconds: UInt64(s * 1_000_000_000))
                                  })
        let started = Date()
        let session = try await r.connect()
        let elapsed = Date().timeIntervalSince(started)
        await fulfillment(of: [closedOlder], timeout: 5)
        XCTAssertTrue(session.isOpen)
        let made = await r.attemptsMade
        XCTAssertEqual(made, 2)
        XCTAssertTrue(log.list.contains { if case .failed(1, let why, 0.1) = $0 { return why.contains("too_many_sessions") }; return false }, "\(log.list)")
        XCTAssertTrue(h.snapshot.contains { if case .connectionClosed(nil, "too many sessions") = $0 { return true }; return false }, "\(h.snapshot)")
        XCTAssertEqual(h.snapshot.filter { if case .hello = $0 { return true }; return false }.count, 2, "older + the successful retry")
        let capture = try session.makeCapture(captureId: "ref-sessions-1", image: Self.fixturePNG, mimeType: "image/png", instructions: "")
        _ = try await r.submit(capture)
        XCTAssertEqual(sink.count, 1)
        print("measured: real-listener too_many_sessions → older closed → reconnect in \(String(format: "%.3f", elapsed))s")
        // Budget spent while the older session stays: terminal, no storm.
        let keeper = NearbyConnection(host: "127.0.0.1", port: h.port, pairId: Self.longTermPairId, psk: Self.longTermPSK)
        await r.shutdown()
        try await waitUntil("session released") { h.snapshot.filter { if case .connectionClosed(Self.longTermPairId?, _) = $0 { return true }; return false }.count >= 2 }
        try await keeper.connect()
        try await keeper.hello(companionName: "keeper")
        let r2 = NearbyReconnector(pair: longTermPair(), policy: ReconnectPolicy(maxAttempts: 3, initialDelay: 0.02, maxDelay: 0.02, jitter: 0, connectTimeout: 3),
                                   endpoints: { [port = h.port] in .hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!) })
        do { _ = try await r2.connect(); XCTFail() } catch let e as NearbyError {
            guard case .attemptsExhausted(3, let last) = e, last.contains("too_many_sessions") else { return XCTFail("unexpected \(e)") }
        }
        keeper.close()
    }

    func testImageRefusalsFromTheRealListenerAreTerminal() async throws {
        var limits = NearbyReceiveLimits()
        limits.maxImageBytes = 1024
        let sink = RecordingSink()
        let dest = FixedDestinations(.init(destinationId: "direct-anchor", projectId: "demo", path: "main.tex", baseRevision: 2))
        let h = ListenerHarness(psks: [longTermEntry], sink: sink, destinations: dest, limits: limits)
        try h.start()
        defer { h.stop() }
        let log = EventLog()
        let r = NearbyReconnector(pair: longTermPair(), policy: ReconnectPolicy(maxAttempts: 4, initialDelay: 0, jitter: 0),
                                  endpoints: { [port = h.port] in .hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!) }, onEvent: log.record)
        let session = try await r.connect()
        // Structurally broken PNG (signature + garbage): passes the client's cheap check, refused by the Mac.
        let broken = Data([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) + Data(repeating: 0x42, count: 64)
        let invalid = try session.makeCapture(captureId: "ref-invalid", image: broken, mimeType: "image/png", instructions: "")
        do { _ = try await r.submit(invalid); XCTFail() } catch let e as NearbyError {
            guard case .remote("invalid_image", _) = e else { return XCTFail("unexpected \(e)") }
            XCTAssertTrue(e.needsNewCapture); XCTAssertFalse(e.isRetryable)
        }
        // Over the Mac's (lowered) size cap, with the client's own 8 MiB check bypassed.
        let big = Self.fixturePNG + Data(repeating: 0, count: 2048)
        let tooLarge = try session.makeCapture(captureId: "ref-large", image: big, mimeType: "image/png", instructions: "")
        do { _ = try await r.submit(tooLarge, validateImage: false); XCTFail() } catch let e as NearbyError {
            guard case .remote("image_too_large", _) = e else { return XCTFail("unexpected \(e)") }
            XCTAssertTrue(e.needsNewCapture)
        }
        // Declared JPEG, PNG bytes: refused on the client before a byte is sent.
        let mismatched = try session.makeCapture(captureId: "ref-mime", image: Self.fixturePNG, mimeType: "image/jpeg", instructions: "")
        do { _ = try await r.submit(mismatched); XCTFail() } catch let e as NearbyError {
            XCTAssertEqual(e, .invalidInput("mime_type image/jpeg but the bytes are not a JPEG"))
        }
        XCTAssertEqual(sink.count, 0, "nothing reached the sink")
        let made = await r.attemptsMade
        XCTAssertEqual(made, 1, "no retry, no reconnect")
        XCTAssertTrue(session.isOpen, "refusals keep the session open")
        let refused = h.snapshot.compactMap { e -> String? in if case .captureRefused(_, _, let code, _) = e { return code }; return nil }
        XCTAssertEqual(refused, ["invalid_image", "image_too_large"])
        // The session is still good for a valid capture.
        let ok = try session.makeCapture(captureId: "ref-ok", image: Self.fixturePNG, mimeType: "image/png", instructions: "")
        let okAck = try await r.submit(ok)
        XCTAssertEqual(okAck.captureId, "ref-ok")
        XCTAssertEqual(sink.count, 1)
        await r.shutdown()
    }

    func testSameSessionRetryIsAcknowledgedWithoutRedeliveryAndRevisionMismatchIsTerminal() async throws {
        let sink = RecordingSink()
        let dest = FixedDestinations(.init(destinationId: "direct-anchor", projectId: "demo", path: "main.tex", baseRevision: 2))
        let h = ListenerHarness(psks: [longTermEntry], sink: sink, destinations: dest)
        try h.start()
        defer { h.stop() }
        let r = NearbyReconnector(pair: longTermPair(), policy: .immediate,
                                  endpoints: { [port = h.port] in .hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!) })
        let session = try await r.connect()
        let capture = try session.makeCapture(captureId: "ref-same-1", image: Self.fixturePNG, mimeType: "image/png", instructions: "once")
        let a1 = try await r.submit(capture)
        let a2 = try await r.submit(capture) // identical retry on the same session
        XCTAssertEqual(a1.captureId, "ref-same-1"); XCTAssertEqual(a2.captureId, "ref-same-1")
        XCTAssertEqual(sink.count, 1, "acknowledged again, not re-delivered")
        XCTAssertTrue(h.snapshot.contains(.captureDuplicate(identity: Self.longTermPairId, captureId: "ref-same-1")), "\(h.snapshot)")
        // Same id, other revision → revision_mismatch (terminal; the destination check is bypassed to reach the Mac).
        var moved = capture; moved.baseRevision = 3
        do { _ = try await r.submit(moved, requireCurrentDestination: false); XCTFail() } catch let e as NearbyError {
            guard case .remote("revision_mismatch", _) = e else { return XCTFail("unexpected \(e)") }
            XCTAssertTrue(e.needsNewCapture); XCTAssertFalse(e.isRetryable)
        }
        // Same id and revision, other payload → capture_id_conflict.
        var changed = capture; changed.instructions = "twice"
        do { _ = try await r.submit(changed); XCTFail() } catch let e as NearbyError {
            guard case .remote("capture_id_conflict", _) = e else { return XCTFail("unexpected \(e)") }
            XCTAssertTrue(e.needsNewCapture)
        }
        XCTAssertEqual(sink.count, 1)
        let made = await r.attemptsMade
        XCTAssertEqual(made, 1)
        XCTAssertTrue(session.isOpen)
        await r.shutdown()
    }

    /// Manual/live harness, skipped unless `FLASHTEX_NEARBY_SERVE_INFO=<path>` is
    /// set: advertises a real NearbyState with a fresh pairing code, writes
    /// `{code, port, fp, name}` to that path, then serves for
    /// `FLASHTEX_NEARBY_SERVE_SECONDS` (default 90) so `.build/debug/nearby-client`
    /// (or the iPad companion) can pair and send from outside the process. The
    /// Mac-side activity log is appended to `<path>.log`.
    func testServeForExternalClient() async throws {
        let env = ProcessInfo.processInfo.environment
        guard let infoPath = env["FLASHTEX_NEARBY_SERVE_INFO"], !infoPath.isEmpty else {
            throw XCTSkip("set FLASHTEX_NEARBY_SERVE_INFO=<path> to serve a live listener for an external client")
        }
        let seconds = Double(env["FLASHTEX_NEARBY_SERVE_SECONDS"] ?? "") ?? 90
        // Loopback only unless a human explicitly asks for the LAN (a real iPad);
        // the automated suite never opens a port beyond loopback.
        let lan = env["FLASHTEX_NEARBY_SERVE_LAN"] == "1"
        // Optional native outage: stop advertising at +RESTART_AT s and come
        // back after DOWN_SECONDS on a fresh ephemeral port, so an external
        // client's bounded reconnect (re-browse by fp) can be measured.
        let restartAt = Double(env["FLASHTEX_NEARBY_SERVE_RESTART_AT"] ?? "")
        let downFor = Double(env["FLASHTEX_NEARBY_SERVE_DOWN_SECONDS"] ?? "") ?? 2
        // Optional revocation: forget every stored pairing at +FORGET_AT s.
        let forgetAt = Double(env["FLASHTEX_NEARBY_SERVE_FORGET_AT"] ?? "")
        let store = PairStore(url: tmp.appendingPathComponent("mac-pairs.json"))
        let model = ShellModel()
        model.caretUTF16 = 6
        model.pinAnchorAtCaret()
        let state = NearbyState(store: store, macName: env["FLASHTEX_NEARBY_SERVE_NAME"] ?? "FlashTeX Serve", loopbackOnly: !lan)
        state.attach(sink: model, destinations: model)
        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        state.beginPairing()
        try await waitUntil("bootstrap key installed") { state.log.filter { $0.hasPrefix("ready on port") }.count >= 2 }
        let info: [String: Any] = ["code": state.pairingCode ?? "", "port": Int(state.port ?? 0), "fp": state.fingerprint,
                                   "name": state.macName, "salt": Pairing.hex(store.salt), "loopback_only": !lan,
                                   "destination": model.nearbyDestination.map { ["destination_id": $0.destinationId, "base_revision": $0.baseRevision] } ?? [:]]
        try JSONSerialization.data(withJSONObject: info).write(to: URL(fileURLWithPath: infoPath))
        let logURL = URL(fileURLWithPath: infoPath + ".log")
        var written = 0
        let started = Date()
        let deadline = started.addingTimeInterval(seconds)
        var outage: (stopAt: Date, resumeAt: Date, done: Bool)? = restartAt.map { (started.addingTimeInterval($0), started.addingTimeInterval($0 + downFor), false) }
        var stopped = false
        var forgotten = false
        func append(_ text: String) {
            if let h = try? FileHandle(forWritingTo: logURL) { h.seekToEndOfFile(); h.write(Data(text.utf8)); try? h.close() }
            else { try? Data(text.utf8).write(to: logURL) }
        }
        let stamp: () -> String = { String(format: "+%.3fs", Date().timeIntervalSince(started)) }
        while Date() < deadline {
            if var o = outage, !o.done {
                if !stopped, Date() >= o.stopAt {
                    state.stopAdvertising()
                    stopped = true
                    append("mac: \(stamp()) outage: stopped advertising (listener gone, port released)\n")
                } else if stopped, Date() >= o.resumeAt {
                    state.startAdvertising()
                    o.done = true
                    outage = o
                    append("mac: \(stamp()) outage: advertising again\n")
                }
            }
            if let f = forgetAt, !forgotten, Date() >= started.addingTimeInterval(f) {
                forgotten = true
                let ids = state.pairs.map(\.pairId)
                ids.forEach { state.forget(pairId: $0) }
                append("mac: \(stamp()) revoked: forgot \(ids)\n")
            }
            let log = state.log
            if log.count > written {
                append(log[written...].map { "mac: \(stamp()) \($0)\n" }.joined())
                written = log.count
            }
            try await Task.sleep(nanoseconds: 100_000_000)
        }
        let summary = "mac: served \(Int(seconds))s; pairs=\(state.pairs.map { "\($0.companionName) \($0.pairId)" }); inbox=\(model.nearbyInbox.received.map(\.captureId))\n"
        if let h = try? FileHandle(forWritingTo: logURL) { h.seekToEndOfFile(); h.write(Data(summary.utf8)); try? h.close() }
        state.stopAdvertising()
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 15, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }
}
