import Network
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Transport metrics line in the Nearby window (mac-nearby-transport-2,
/// follow-up 2): counters derived from listener events, and the line
/// `NearbyState` publishes after real sessions on loopback.
final class NearbyMetricsTests: XCTestCase {
    func testCountersFollowListenerEvents() {
        var m = NearbyTransportMetrics()
        XCTAssertEqual(m.line, "frames 0 (0 refused, 0 duplicates) · sessions 0 (0 reconnects) · closed 0 (0 timed out)")
        m.record(.ready(port: 1))
        m.record(.connectionOpened)
        m.record(.hello(pairId: "a", companionName: "A", bootstrap: true))
        m.record(.capture(captureId: "c1"))
        m.record(.captureDuplicate(identity: "a", captureId: "c1"))
        m.record(.captureRefused(identity: "a", captureId: "c2", code: "invalid_image", message: "x"))
        m.record(.receiving(identity: "a", bytes: 1, expected: nil))
        m.record(.connectionClosed(identity: "a", reason: "peer closed"))
        m.record(.ready(port: 1)) // key-table restart
        m.record(.hello(pairId: "a", companionName: "A", bootstrap: false)) // reconnect
        m.record(.hello(pairId: "b", companionName: "B", bootstrap: false)) // another pairing: not a reconnect
        m.record(.connectionClosed(identity: nil, reason: "handshake timed out after 10s"))
        m.record(.connectionClosed(identity: "b", reason: "frame timed out after 60s"))
        m.record(.failed("x")); m.record(.stopped)
        XCTAssertEqual(m.frames, 3); XCTAssertEqual(m.accepted, 1); XCTAssertEqual(m.refused, 1); XCTAssertEqual(m.duplicates, 1)
        XCTAssertEqual(m.sessions, 3); XCTAssertEqual(m.reconnects, 1)
        XCTAssertEqual(m.closed, 3); XCTAssertEqual(m.timedOut, 2)
        XCTAssertEqual(m.listenerRestarts, 1)
        XCTAssertEqual(m.line, "frames 3 (1 refused, 1 duplicate) · sessions 3 (1 reconnect) · closed 3 (2 timed out) · restarts 1")
        m.reset()
        XCTAssertEqual(m.line, NearbyTransportMetrics().line)
        // Reset keeps what the listener already knows: another ready is still a
        // restart, a known pairing is still a reconnect.
        m.record(.ready(port: 1))
        m.record(.hello(pairId: "a", companionName: "A", bootstrap: false))
        XCTAssertEqual(m.listenerRestarts, 1)
        XCTAssertEqual(m.reconnects, 1)
    }
}

@MainActor
final class NearbyMetricsStateTests: XCTestCase {
    static let psk = Data(repeating: 0x4D, count: 32)
    static let pairId = "pair-m"
    let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-metrics-\(UUID().uuidString)")
    override func tearDown() { try? FileManager.default.removeItem(at: tmp) }

    private func waitUntil(_ what: String, timeout: TimeInterval = loopbackWait, file: StaticString = #filePath, line: UInt = #line,
                           _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)", file: file, line: line)
    }

    func hello(_ c: NearbyTestClient, nonce: String) {
        c.send(id: "h", type: "hello", NearbyV1.Hello(pairId: Self.pairId, companionName: "Metrics iPad", nonce: nonce,
                                                      proof: Pairing.helloProof(psk: Self.psk, nonce: nonce)))
    }

    /// One pairing: a capture, a retry on a reconnect (duplicate, not
    /// re-delivered), a refused capture, and a silent authenticated peer that
    /// hits the hello deadline — all visible on the window's line.
    func testStatePublishesTheLineAfterRealSessions() async throws {
        let store = PairStore(url: tmp.appendingPathComponent("pairs.json"))
        XCTAssertTrue(store.upsert(PairRecord(pairId: Self.pairId, psk: Self.psk.base64EncodedString(), companionName: "Metrics iPad",
                                              createdAt: Date(), lastSeenAt: nil)))
        let model = ShellModel()
        var limits = NearbyReceiveLimits()
        limits.helloTimeout = 0.3
        let state = NearbyState(store: store, macName: "Metrics Mac", loopbackOnly: true, limits: limits)
        state.attach(sink: model, destinations: model)
        state.startAdvertising()
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let port = try XCTUnwrap(state.port)
        defer { state.stopAdvertising() }
        XCTAssertEqual(state.metrics.listenerRestarts, 0)

        let png = TestImages.png(width: 16, height: 16)
        let submit = RuntimeV1.CaptureSubmit(captureId: "m-1", destinationId: "d", baseRevision: 1,
                                             image: .init(mimeType: "image/png", dataBase64: png.base64EncodedString()), instructions: "m")
        let a = NearbyTestClient(port: port, identity: Self.pairId, psk: Self.psk)
        try await waitUntil("a ready") { a.isReady }
        hello(a, nonce: "m-a")
        try await waitUntil("hello a") { state.metrics.sessions == 1 }
        a.send(id: "s1", type: "capture_submit", submit)
        try await waitUntil("accepted") { state.metrics.accepted == 1 }
        a.cancel()
        try await waitUntil("closed a") { state.metrics.closed == 1 }

        let b = NearbyTestClient(port: port, identity: Self.pairId, psk: Self.psk)
        try await waitUntil("b ready") { b.isReady }
        hello(b, nonce: "m-b")
        try await waitUntil("reconnect counted") { state.metrics.reconnects == 1 }
        b.send(id: "s2", type: "capture_submit", submit) // retry after the drop
        try await waitUntil("duplicate") { state.metrics.duplicates == 1 }
        XCTAssertEqual(model.nearbyInbox.received.count, 1, "the retry was answered from memory")
        var bad = submit
        bad.captureId = "m-2"
        bad.image = .init(mimeType: "image/png", dataBase64: Data(repeating: 0x41, count: 64).base64EncodedString())
        b.send(id: "s3", type: "capture_submit", bad)
        try await waitUntil("refused") { state.metrics.refused == 1 }

        let mute = NearbyTestClient(port: port, identity: Self.pairId, psk: Self.psk)
        try await waitUntil("mute ready") { mute.isReady }
        try await waitUntil("hello deadline") { state.metrics.timedOut == 1 }
        try await waitUntil("mute closed") { mute.isClosed }

        XCTAssertEqual(state.metrics.frames, 3)
        XCTAssertEqual(state.metrics.sessions, 2)
        XCTAssertEqual(state.metrics.closed, 2)
        XCTAssertEqual(state.metrics.line, "frames 3 (1 refused, 1 duplicate) · sessions 2 (1 reconnect) · closed 2 (1 timed out)")
        XCTAssertTrue(state.log.contains { $0.hasPrefix("closed unauthenticated: hello timed out after 0.30s") }, "\(state.log)")
        b.cancel()
        try await waitUntil("closed b") { state.metrics.closed == 3 }
        state.resetMetrics()
        XCTAssertEqual(state.metrics.line, NearbyTransportMetrics().line)
        // A key-table restart (pairing code) counts as a listener restart.
        state.beginPairing()
        try await waitUntil("restart") { state.metrics.listenerRestarts == 1 }
        XCTAssertTrue(state.metrics.line.hasSuffix("· restarts 1"), state.metrics.line)
        state.cancelPairing()
    }
}
