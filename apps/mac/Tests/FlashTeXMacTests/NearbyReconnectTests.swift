import Network
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Disconnect/reconnect and bounded decoding at the listener (mac-nearby-transport-2):
/// a companion that drops mid-frame, a retry after a drop-before-ack that is
/// acknowledged from the pairing's memory and never re-delivered, the
/// acknowledgement memory across a same-port restart, and the framing
/// deadlines (handshake, hello, partial frame) that keep half-open or
/// slow-loris peers from holding a slot. Loopback only; no device required.
final class NearbyReconnectTests: XCTestCase {
    static let psk = NearbyListenerTests.pskA
    static let pairId = "pair-r"
    var entry: NearbyListener.PSKEntry { .init(identity: Self.pairId, key: Self.psk, isBootstrap: false) }

    func hello(_ client: NearbyTestClient, nonce: String = UUID().uuidString) {
        client.send(id: "h", type: "hello", NearbyV1.Hello(pairId: Self.pairId, companionName: "Dropping iPad", nonce: nonce,
                                                           proof: Pairing.helloProof(psk: Self.psk, nonce: nonce)))
    }
    func type(_ line: Data) -> String? { (try? RuntimeV1.header(of: line))?.type }
    func id(_ line: Data) -> String? { (try? RuntimeV1.header(of: line))?.id }
    func errorCode(_ line: Data) -> String? {
        (try? JSONDecoder().decode(NearbyListenerTests.ErrorLine.self, from: line))?.payload.code
    }
    func ack(_ line: Data) -> NearbyV1.CaptureReceived? {
        let e = try? JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureReceived>.self, from: line)
        return e?.type == "capture_received" ? e?.payload : nil
    }

    static let png = TestImages.png(width: 24, height: 24)
    func submit(_ captureId: String = "cap-r1", revision: Int = 2) -> RuntimeV1.CaptureSubmit {
        .init(captureId: captureId, destinationId: "dest-r", baseRevision: revision,
              image: .init(mimeType: "image/png", dataBase64: Self.png.base64EncodedString()), instructions: "reconnect")
    }
    func frame(_ id: String, _ s: RuntimeV1.CaptureSubmit) -> Data { NearbyV1.line(id: id, type: "capture_submit", s) }

    func connect(_ h: ListenerHarness, file: StaticString = #filePath, line: UInt = #line) -> NearbyTestClient {
        let c = NearbyTestClient(port: h.port, identity: Self.pairId, psk: Self.psk)
        XCTAssertEqual(XCTWaiter.wait(for: [c.ready], timeout: 5), .completed, "\(String(describing: c.failure))", file: file, line: line)
        return c
    }

    /// Polls `cond` (any thread) up to `timeout`; fails with `what` otherwise.
    @discardableResult
    func waitUntil(_ what: String, timeout: TimeInterval = 5, file: StaticString = #filePath, line: UInt = #line,
                   _ cond: () -> Bool) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return true }
            usleep(10_000)
        }
        XCTFail("timed out waiting for \(what)", file: file, line: line)
        return false
    }

    func closedEvents(_ h: ListenerHarness, identity: String?) -> [String] {
        h.snapshot.compactMap { if case .connectionClosed(let i, let r) = $0, i == identity { return r }; return nil }
    }

    static func loadAverage() -> Double {
        var l = [Double](repeating: 0, count: 3)
        return getloadavg(&l, 3) >= 1 ? l[0] : 0
    }

    // MARK: drop mid-frame, reconnect once

    /// The companion vanishes with half a capture line in flight: nothing is
    /// delivered, its budget and session slot are released, and its next
    /// session (same pairing) delivers the capture exactly once.
    func testPeerDropMidFrameReleasesEverythingAndTheReconnectDeliversOnce() throws {
        let sink = RecordingSink()
        let h = ListenerHarness(psks: [entry], sink: sink, destinations: nil)
        try h.start()
        defer { h.stop() }

        let a = connect(h)
        hello(a)
        XCTAssertEqual(type(try a.lines(atLeast: 1)[0]), "hello_ack")
        let full = frame("s1", submit())
        a.send(full.prefix(full.count / 2)) // no newline: the frame never completes
        usleep(100_000)
        a.cancel()
        waitUntil("listener saw the drop") { !closedEvents(h, identity: Self.pairId).isEmpty }
        waitUntil("slot released") { h.listener.openConnectionCount == 0 }
        XCTAssertEqual(sink.count, 0, "a partial frame is never delivered")
        XCTAssertEqual(h.listener.inboxBytesInFlight, 0)
        XCTAssertEqual(h.listener.budget.sessionCount(pairId: Self.pairId), 0, "the dropped session's slot is free")
        XCTAssertEqual(h.listener.memory.count(pairId: Self.pairId), 0, "nothing was remembered for a frame that never arrived")

        let b = connect(h)
        hello(b)
        XCTAssertEqual(type(try b.lines(atLeast: 1)[0]), "hello_ack")
        b.send(full)
        let lines = try b.lines(atLeast: 2)
        XCTAssertEqual(ack(lines[1])?.captureId, "cap-r1")
        XCTAssertEqual(id(lines[1]), "s1")
        XCTAssertEqual(sink.count, 1, "delivered exactly once")
        XCTAssertEqual(h.snapshot.filter { $0 == .capture(captureId: "cap-r1") }.count, 1)
        XCTAssertEqual(h.snapshot.filter { if case .hello = $0 { return true }; return false }.count, 2)
        b.cancel()
    }

    // MARK: retry after drop-before-ack

    /// The sink is still holding the acknowledgement when the companion drops;
    /// its retry on a new session of the same pairing waits for that one
    /// answer (never a second delivery), and a later retry is answered from
    /// memory. The pairing store still counts the capture once.
    func testRetryAfterDropBeforeAckIsAcknowledgedFromMemoryNotRedelivered() throws {
        let sink = DeferredSink()
        let h = ListenerHarness(psks: [entry], sink: sink, destinations: nil)
        try h.start()
        defer { h.stop() }

        let a = connect(h)
        hello(a)
        XCTAssertEqual(type(try a.lines(atLeast: 1)[0]), "hello_ack")
        a.send(frame("s1", submit()))
        waitUntil("first delivery pending in the sink") { sink.pendingCount == 1 }
        a.cancel()
        waitUntil("listener saw the drop") { !closedEvents(h, identity: Self.pairId).isEmpty }
        waitUntil("slot released") { h.listener.openConnectionCount == 0 }
        XCTAssertEqual(h.listener.inboxBytesInFlight, 0, "the dropped session's budget is released while the sink still holds the capture")
        XCTAssertEqual(h.listener.memory.count(pairId: Self.pairId), 1, "the pending delivery is remembered for the pairing")

        let b = connect(h)
        hello(b)
        XCTAssertEqual(type(try b.lines(atLeast: 1)[0]), "hello_ack")
        b.send(frame("s2", submit())) // identical retry, new envelope id
        waitUntil("retry recognised") { h.snapshot.contains(.captureDuplicate(identity: Self.pairId, captureId: "cap-r1")) }
        XCTAssertEqual(sink.count, 1, "the retry is not delivered again")
        XCTAssertEqual(b.lineCount, 1, "no answer until the sink answers the first delivery")

        sink.flush()
        let lines = try b.lines(atLeast: 2)
        XCTAssertEqual(id(lines[1]), "s2", "the retry is answered with its own envelope id")
        XCTAssertEqual(ack(lines[1])?.captureId, "cap-r1")
        XCTAssertEqual(h.snapshot.filter { $0 == .capture(captureId: "cap-r1") }.count, 1)

        // Acknowledged now: a further retry is answered from memory at once.
        b.send(frame("s3", submit()))
        let more = try b.lines(atLeast: 3)
        XCTAssertEqual(id(more[2]), "s3")
        XCTAssertEqual(ack(more[2])?.captureId, "cap-r1")
        XCTAssertEqual(sink.count, 1)
        // …and a different payload under the same id is still a conflict.
        var other = submit()
        other.instructions = "changed"
        b.send(frame("s4", other))
        XCTAssertEqual(errorCode(try b.lines(atLeast: 4)[3]), "capture_id_conflict")
        XCTAssertEqual(h.listener.memory.count(pairId: Self.pairId), 1)
        b.cancel()
    }

    /// A drop while the image is still being validated leaves nothing behind:
    /// the retry is delivered afresh (exactly once overall).
    func testDropDuringValidationForgetsThePendingEntry() throws {
        let sink = RecordingSink()
        let queue = DispatchQueue(label: "nearby.test.reconnect.validation")
        let h = ListenerHarness(psks: [entry], sink: sink, destinations: nil, queue: queue)
        try h.start()
        defer { h.stop() }
        let a = connect(h)
        hello(a)
        XCTAssertEqual(type(try a.lines(atLeast: 1)[0]), "hello_ack")
        // Park the listener queue so the frame is budgeted and remembered but
        // its validation result cannot be applied until the peer is gone.
        let gate = DispatchSemaphore(value: 0)
        let big = TestImages.png(width: 96, height: 96, noise: true)
        let s = RuntimeV1.CaptureSubmit(captureId: "cap-v1", destinationId: "dest-r", baseRevision: 1,
                                        image: .init(mimeType: "image/png", dataBase64: big.base64EncodedString()), instructions: "v")
        a.send(frame("v1", s))
        waitUntil("remembered as pending") { h.listener.memory.count(pairId: Self.pairId) == 1 }
        queue.async { gate.wait() }
        a.cancel()
        usleep(200_000)
        gate.signal()
        waitUntil("listener saw the drop") { !closedEvents(h, identity: Self.pairId).isEmpty }
        waitUntil("pending entry forgotten or acknowledged", timeout: 15) {
            h.listener.memory.count(pairId: Self.pairId) == 0 || h.listener.memory.lookup(pairId: Self.pairId, captureId: "cap-v1")?.ack != nil
        }
        let b = connect(h)
        hello(b)
        XCTAssertEqual(type(try b.lines(atLeast: 1)[0]), "hello_ack")
        b.send(frame("v2", s))
        let replies = try b.lines(atLeast: 2, timeout: 15)
        XCTAssertEqual(replies.count >= 2 ? ack(replies[1])?.captureId : nil, "cap-v1",
                       "\(replies.map { String(decoding: $0, as: UTF8.self) })")
        XCTAssertEqual(sink.count, 1, "delivered exactly once whichever side of the drop validation landed on")
        b.cancel()
    }

    // MARK: memory across a same-port restart

    /// The acknowledgement memory moves with the sessions on a key-table
    /// restart (what NearbyState does after a pairing), so a retry against the
    /// replacement listener is still not re-delivered; a forgotten pairing
    /// loses its memory with its keys.
    func testAckMemorySurvivesSamePortRestartAndFollowsForgottenPairings() throws {
        let sink = RecordingSink()
        let queue = DispatchQueue(label: "nearby.test.reconnect.restart")
        let h1 = ListenerHarness(psks: [entry], sink: sink, destinations: nil, queue: queue)
        try h1.start()
        let a = connect(h1)
        hello(a)
        XCTAssertEqual(type(try a.lines(atLeast: 1)[0]), "hello_ack")
        a.send(frame("s1", submit()))
        XCTAssertEqual(ack(try a.lines(atLeast: 2)[1])?.captureId, "cap-r1")
        XCTAssertEqual(sink.count, 1)

        let other = NearbyListener.PSKEntry(identity: "pair-other", key: NearbyListenerTests.pskB, isBootstrap: false)
        let h2 = ListenerHarness(psks: [entry, other], sink: sink, destinations: nil, port: h1.port, queue: queue)
        h2.listener.adoptConnections(from: h1.listener)
        let stopped = XCTestExpectation(description: "old listener released its port")
        h1.listener.stop(keepConnections: true) { stopped.fulfill() }
        XCTAssertEqual(XCTWaiter.wait(for: [stopped], timeout: 5), .completed)
        try h2.start()
        XCTAssertEqual(h2.port, h1.port)
        XCTAssertTrue(h2.listener.memory === h1.listener.memory, "one memory object across the restart")

        let b = connect(h2)
        hello(b)
        XCTAssertEqual(type(try b.lines(atLeast: 1)[0]), "hello_ack")
        b.send(frame("s2", submit()))
        XCTAssertEqual(ack(try b.lines(atLeast: 2)[1])?.captureId, "cap-r1")
        XCTAssertEqual(sink.count, 1, "acknowledged from the adopted memory, not re-delivered")
        XCTAssertTrue(h2.snapshot.contains(.captureDuplicate(identity: Self.pairId, captureId: "cap-r1")))
        // The adopted session keeps working too.
        a.send(frame("s3", submit()))
        XCTAssertEqual(ack(try a.lines(atLeast: 3)[2])?.captureId, "cap-r1")
        XCTAssertEqual(sink.count, 1)

        // Forgetting the pairing drops its sessions and its memory.
        let h3 = ListenerHarness(psks: [other], sink: sink, destinations: nil, port: h2.port, queue: queue)
        h3.listener.adoptConnections(from: h2.listener)
        XCTAssertEqual(h3.listener.memory.count(pairId: Self.pairId), 0)
        XCTAssertEqual(XCTWaiter.wait(for: [a.closed, b.closed], timeout: 5), .completed)
        let stopped2 = XCTestExpectation(description: "h2 stopped")
        h2.listener.stop(keepConnections: true) { stopped2.fulfill() }
        XCTAssertEqual(XCTWaiter.wait(for: [stopped2], timeout: 5), .completed)
        h3.stop()
    }

    // MARK: framing deadlines (half-open, slow-loris)

    /// Each deadline closes a peer that stalls at that stage, with a typed
    /// reason the listener reports; a peer that keeps going is untouched.
    func testHandshakeHelloAndFrameDeadlinesCloseStalledPeers() throws {
        let load = Self.loadAverage()
        print("measured: 1-min load \(load)")
        try XCTSkipIf(load > 20, "timing test skipped under load \(load)")
        var limits = NearbyReceiveLimits()
        limits.handshakeTimeout = 0.4
        limits.helloTimeout = 0.4
        limits.frameTimeout = 0.6
        let sink = RecordingSink()
        let h = ListenerHarness(psks: [entry], sink: sink, destinations: nil, limits: limits)
        try h.start()
        defer { h.stop() }

        // 1. TCP peer that never starts TLS: closed after handshakeTimeout, no bytes owed.
        let queue = DispatchQueue(label: "nearby.test.reconnect.plain")
        let plain = NWConnection(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: h.port)!, using: .tcp)
        let tcpReady = XCTestExpectation(description: "tcp ready"), tcpEnded = XCTestExpectation(description: "tcp ended")
        plain.stateUpdateHandler = { state in
            switch state {
            case .ready: tcpReady.fulfill()
            case .failed, .cancelled: tcpEnded.fulfill()
            default: break
            }
        }
        plain.start(queue: queue)
        XCTAssertEqual(XCTWaiter.wait(for: [tcpReady], timeout: 5), .completed)
        var got = Data()
        func drain() {
            plain.receive(minimumIncompleteLength: 1, maximumLength: 4096) { data, _, complete, error in
                if let data { got.append(data) }
                if complete || error != nil { tcpEnded.fulfill(); return }
                drain()
            }
        }
        drain()
        let t0 = Date()
        XCTAssertEqual(XCTWaiter.wait(for: [tcpEnded], timeout: 5), .completed, "silent TCP peer must be dropped")
        let handshakeClose = Date().timeIntervalSince(t0)
        XCTAssertTrue(got.isEmpty, "no application bytes to an unauthenticated peer: \(got as NSData)")
        waitUntil("handshake close event") { closedEvents(h, identity: nil).contains("handshake timed out after 0.40s") }
        plain.cancel()

        // 2. Authenticated peer that never says hello: hello_timeout, then close.
        let mute = connect(h)
        let t1 = Date()
        let l = try mute.lines(atLeast: 1)
        XCTAssertEqual(errorCode(l[0]), "hello_timeout")
        XCTAssertEqual(XCTWaiter.wait(for: [mute.closed], timeout: 5), .completed)
        let helloClose = Date().timeIntervalSince(t1)
        waitUntil("hello close event") { closedEvents(h, identity: nil).contains("hello timed out after 0.40s") }
        XCTAssertEqual(h.listener.budget.sessionCount(pairId: Self.pairId), 0)

        // 3. Slow-loris: hello, then one byte of a frame at a time, never a newline.
        let slow = connect(h)
        hello(slow)
        XCTAssertEqual(type(try slow.lines(atLeast: 1)[0]), "hello_ack")
        let full = frame("s1", submit())
        let t2 = Date()
        var offset = 0
        let feeder = DispatchSource.makeTimerSource(queue: queue)
        feeder.schedule(deadline: .now(), repeating: .milliseconds(50))
        feeder.setEventHandler {
            guard offset < full.count - 1 else { return }
            slow.send(full[offset..<offset + 1]); offset += 1
        }
        feeder.resume()
        let sl = try slow.lines(atLeast: 2)
        feeder.cancel()
        XCTAssertEqual(errorCode(sl[1]), "frame_timeout")
        XCTAssertEqual(XCTWaiter.wait(for: [slow.closed], timeout: 5), .completed)
        let frameClose = Date().timeIntervalSince(t2)
        waitUntil("frame close event") { closedEvents(h, identity: Self.pairId).contains("frame timed out after 0.60s") }
        XCTAssertEqual(sink.count, 0)
        waitUntil("budget released") { h.listener.inboxBytesInFlight == 0 && h.listener.budget.sessionCount(pairId: Self.pairId) == 0 }

        // 4. A peer that keeps moving is untouched: hello inside the deadline,
        //    a frame in two halves inside the frame deadline, then idle far
        //    longer than every deadline, then another frame.
        let live = connect(h)
        usleep(150_000)
        hello(live)
        XCTAssertEqual(type(try live.lines(atLeast: 1)[0]), "hello_ack")
        live.send(full.prefix(full.count / 2))
        usleep(250_000)
        live.send(full.suffix(from: full.count / 2))
        XCTAssertEqual(ack(try live.lines(atLeast: 2)[1])?.captureId, "cap-r1")
        usleep(900_000) // idle between frames: no deadline is armed
        XCTAssertTrue(live.isReady && !live.isClosed, "idle authenticated session is kept")
        live.send(frame("s2", submit("cap-r2")))
        XCTAssertEqual(ack(try live.lines(atLeast: 3)[2])?.captureId, "cap-r2")
        XCTAssertEqual(sink.count, 2)
        print(String(format: "measured: handshake close %.2fs, hello close %.2fs, frame close %.2fs (deadlines 0.4/0.4/0.6s), load %.1f",
                     handshakeClose, helloClose, frameClose, load))
        XCTAssertGreaterThanOrEqual(handshakeClose, 0.3)
        XCTAssertGreaterThanOrEqual(helloClose, 0.2)
        XCTAssertGreaterThanOrEqual(frameClose, 0.4)
        live.cancel()
    }

    // MARK: bounded decoding before allocation

    /// A chunk that cannot complete the pending line inside `maxLineBytes` is
    /// refused before it is buffered (typed `line_too_long`), while a chunk
    /// whose newline falls inside the limit is accepted.
    func testOversizedPartialFrameIsRefusedBeforeItIsBuffered() throws {
        var limits = NearbyReceiveLimits()
        limits.maxLineBytes = 4096
        let sink = RecordingSink()
        let h = ListenerHarness(psks: [entry], sink: sink, destinations: nil, limits: limits)
        try h.start()
        defer { h.stop() }

        let c = connect(h)
        hello(c)
        XCTAssertEqual(type(try c.lines(atLeast: 1)[0]), "hello_ack")
        // 3000 pending + a 2000-byte chunk with no newline: cannot end inside 4096.
        c.send(Data(repeating: 0x20, count: 3000))
        usleep(100_000)
        c.send(Data(repeating: 0x20, count: 2000))
        let l = try c.lines(atLeast: 2)
        XCTAssertEqual(errorCode(l[1]), "line_too_long")
        XCTAssertEqual(XCTWaiter.wait(for: [c.closed], timeout: 5), .completed)
        waitUntil("close reason") { closedEvents(h, identity: Self.pairId).contains("unterminated line too long") }

        // Pending + chunk again exceed the limit, but this time the newline
        // falls inside it: accepted (an unknown-type line is answered, not
        // closed), and the bytes after the newline start the next line.
        let d = connect(h)
        hello(d)
        XCTAssertEqual(type(try d.lines(atLeast: 1)[0]), "hello_ack")
        var line = Data("{\"protocol_version\":1,\"id\":\"pad\",\"type\":\"noop\",\"payload\":{\"x\":\"".utf8)
        line.append(Data(repeating: 0x61, count: 3500 - line.count))
        d.send(line)
        usleep(100_000)
        var tail = Data(repeating: 0x61, count: 500)
        tail.append(Data("\"}}\n".utf8))
        tail.append(Data("{\"protocol_version\":1,\"id\":\"next\",\"payload\":{\"pad\":\"".utf8))
        tail.append(Data(repeating: 0x78, count: 120))
        XCTAssertGreaterThan(3500 + tail.count, 4096, "the chunk does exceed the limit")
        XCTAssertLessThanOrEqual(3500 + 504, 4096, "but the line it completes fits")
        d.send(tail)
        let dl = try d.lines(atLeast: 2)
        XCTAssertEqual(errorCode(dl[1]), "unknown_type")
        XCTAssertEqual(id(dl[1]), "pad")
        d.send(Data("\"},\"type\":\"destination_query\"}\n".utf8))
        XCTAssertEqual(type(try d.lines(atLeast: 3)[2]), "destination")
        XCTAssertTrue(d.isReady && !d.isClosed)
        d.cancel()

        // Typed errors carry the codes the wire shows.
        XCTAssertEqual(NearbyFrameError.lineTooLong(bytes: 5000, limit: 4096).code, "line_too_long")
        XCTAssertEqual(NearbyFrameError.unterminatedLineTooLong(limit: 4096).code, "line_too_long")
        XCTAssertEqual(NearbyFrameError.frameTimeout(seconds: 60, pendingBytes: 7).code, "frame_timeout")
        XCTAssertEqual(NearbyFrameError.frameTimeout(seconds: 60, pendingBytes: 7).message, "frame not completed within 60s (7 bytes pending)")
        XCTAssertEqual(NearbyFrameError.helloTimeout(seconds: 10).code, "hello_timeout")
        XCTAssertNil(NearbyFrameError.handshakeTimeout(seconds: 10).code, "no application bytes to an unauthenticated peer")
        XCTAssertEqual(NearbyFrameError.handshakeTimeout(seconds: 10).reason, "handshake timed out after 10s")
    }

    // MARK: half-open detection parameters

    /// The listener's TCP options carry the keepalive schedule so a peer that
    /// vanished without a FIN is dropped in bounded time (idle + interval × count).
    func testKeepaliveScheduleIsOnTheListenerParameters() {
        var limits = NearbyReceiveLimits()
        XCTAssertEqual(limits.keepaliveIdleSeconds, 15)
        XCTAssertEqual(limits.keepaliveIntervalSeconds, 5)
        XCTAssertEqual(limits.keepaliveCount, 4)
        limits.keepaliveIdleSeconds = 7; limits.keepaliveIntervalSeconds = 3; limits.keepaliveCount = 2
        let params = NearbyListener.parameters(psks: [(Self.pairId, Self.psk)], loopbackOnly: true, limits: limits)
        let tcp = try? XCTUnwrap(params.defaultProtocolStack.transportProtocol as? NWProtocolTCP.Options)
        XCTAssertEqual(tcp?.enableKeepalive, true)
        XCTAssertEqual(tcp?.keepaliveIdle, 7)
        XCTAssertEqual(tcp?.keepaliveInterval, 3)
        XCTAssertEqual(tcp?.keepaliveCount, 2)
        XCTAssertEqual(tcp?.noDelay, true)
        let defaults = NearbyReceiveLimits()
        XCTAssertEqual(defaults.handshakeTimeout, 10)
        XCTAssertEqual(defaults.helloTimeout, 10)
        XCTAssertEqual(defaults.frameTimeout, 60)
        XCTAssertGreaterThanOrEqual(Double(defaults.maxLineBytes) / defaults.frameTimeout, 200 * 1024,
                                    "a full frame at 200 KiB/s fits inside the frame deadline")
    }
}
