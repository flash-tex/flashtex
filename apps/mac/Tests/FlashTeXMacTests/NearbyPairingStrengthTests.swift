import Network
import XCTest
@testable import FlashTeXMac

/// The pairing code is a 256-bit secret, and failed handshakes back off per
/// address without ever ending a pairing attempt (proposal nearby-v1 §5, lane
/// TRANSFER-AUDIT). The 6-digit code these replace could be tested offline
/// from one recorded handshake in about a second.
final class PairingCodeStrengthTests: XCTestCase {
    func testGeneratedCodeCarries256RandomBits() {
        let code = Pairing.generateCode()
        XCTAssertEqual(code.count, 52)
        XCTAssertTrue(code.allSatisfy { Pairing.codeAlphabet.contains($0) }, code)
        XCTAssertEqual(Pairing.canonicalCode(code), code)
        XCTAssertNotEqual(Pairing.generateCode(), Pairing.generateCode())
        XCTAssertGreaterThanOrEqual(Pairing.codeSecretBytes * 8, 256)
        XCTAssertGreaterThanOrEqual(Pairing.codeLength * 5, Pairing.codeSecretBytes * 8)
    }

    func testBase32IsCrockfordMostSignificantBitFirst() {
        XCTAssertEqual(Pairing.base32(Data(repeating: 0, count: 32)), String(repeating: "0", count: 52))
        // 256 one-bits: 51 full characters, then one bit padded with zeros.
        XCTAssertEqual(Pairing.base32(Data(repeating: 0xFF, count: 32)), String(repeating: "Z", count: 51) + "G")
        XCTAssertEqual(Pairing.base32(Data([0x08, 0x86, 0x42])), "12344") // 00001 00010 00011 00100 0010+0
    }

    func testTypedCodeIsCanonicalisedAndWeakCodesAreRefused() {
        let code = Pairing.generateCode()
        let shown = Pairing.displayCode(code)
        XCTAssertEqual(shown.split(separator: " ").count, 13)
        XCTAssertEqual(Pairing.canonicalCode(shown), code)
        XCTAssertEqual(Pairing.canonicalCode(shown.lowercased().replacingOccurrences(of: " ", with: "-")), code)
        XCTAssertEqual(Pairing.canonicalCode("o" + String(repeating: "i", count: 50) + "L"), "0" + String(repeating: "1", count: 51))
        XCTAssertNil(Pairing.canonicalCode("123456"), "the former six digits are not a code")
        XCTAssertNil(Pairing.canonicalCode(String(code.dropLast())))
        XCTAssertNil(Pairing.canonicalCode(code + "0"))
        XCTAssertNil(Pairing.canonicalCode(String(code.dropLast()) + "U"), "U is not in the alphabet")
        XCTAssertEqual(Pairing.clipboardCode(shown), code)

        let salt = Pairing.generateSalt()
        XCTAssertEqual(Pairing.derive(code: shown.lowercased(), salt: salt), Pairing.derive(code: code, salt: salt),
                       "spacing and case never change the key")
    }

    func testQRPayloadCarriesTheStrongCodeAndRefusesWeakOnes() throws {
        let salt = try XCTUnwrap(Pairing.data(hex: "000102030405060708090a0b0c0d0e0f"))
        let code = Pairing.generateCode()
        let p = PairingBootstrapPayload(code: code, salt: salt, macName: "Studio")
        XCTAssertTrue(p.urlString.hasPrefix("flashtex-nearby://pair?v=2&code=\(code)&salt="), p.urlString)
        XCTAssertEqual(try PairingBootstrapPayload.parse(p.urlString).get(), p)
        let fp = Pairing.fingerprint(salt: salt)
        XCTAssertThrowsError(try PairingBootstrapPayload.parse("flashtex-nearby://pair?v=1&code=123456&salt=\(Pairing.hex(salt))&fp=\(fp)").get())
        XCTAssertThrowsError(try PairingBootstrapPayload.parse("flashtex-nearby://pair?v=2&code=123456&salt=\(Pairing.hex(salt))&fp=\(fp)").get())
    }

    /// Announcements and summaries say a code is shown; they never read 52
    /// characters aloud (the code element itself still spells it).
    func testAnnouncementsDoNotReadTheWholeCode() {
        let code = Pairing.generateCode()
        let a = PairingFlow.Attempt(generation: 1, code: code, pairId: "p", startedAt: Date(), expiresAt: Date().addingTimeInterval(120))
        var m = PairingFlow.Machine(phase: .advertising, generation: 0, isAdvertising: true)
        let out = m.apply(.codeIssued(a))
        for case .announce(let text) in out.effects { XCTAssertFalse(text.contains(code.prefix(4)), text) }
        XCTAssertFalse(PairingFlow.Phase.codeShown(a).accessibilityValue(now: Date()).contains(code.prefix(4)))
        XCTAssertEqual(Pairing.spokenCode("ABCD12"), "A B C D, 1 2")
    }
}

final class NearbyHandshakeLimiterTests: XCTestCase {
    let t0 = Date(timeIntervalSince1970: 1_800_000_000)

    func testBackoffDoublesAfterTheFreeFailuresAndIsCapped() {
        var l = NearbyHandshakeLimiter()
        for i in 0..<3 {
            XCTAssertNil(l.retryAfter("10.0.0.9", now: t0), "failure \(i) is free")
            l.recordFailure("10.0.0.9", now: t0)
        }
        XCTAssertEqual(l.retryAfter("10.0.0.9", now: t0), 1)
        XCTAssertNil(l.retryAfter("10.0.0.10", now: t0), "other addresses are not affected")
        XCTAssertNil(l.retryAfter("10.0.0.9", now: t0.addingTimeInterval(1)))
        l.recordFailure("10.0.0.9", now: t0.addingTimeInterval(1))
        XCTAssertEqual(l.retryAfter("10.0.0.9", now: t0.addingTimeInterval(1)), 2)
        for k in 0..<20 { l.recordFailure("10.0.0.9", now: t0.addingTimeInterval(2 + Double(k))) }
        XCTAssertEqual(l.retryAfter("10.0.0.9", now: t0.addingTimeInterval(21)), 300, "capped at maxBackoff")
        XCTAssertNil(l.retryAfter("10.0.0.9", now: t0.addingTimeInterval(21 + 600)), "forgotten after forgetAfter")
    }

    func testSuccessClearsTheAddress() {
        var l = NearbyHandshakeLimiter()
        for _ in 0..<5 { l.recordFailure("a", now: t0) }
        XCTAssertNotNil(l.retryAfter("a", now: t0))
        l.recordSuccess("a")
        XCTAssertNil(l.retryAfter("a", now: t0))
        XCTAssertEqual(l.failures("a"), 0)
    }

    func testTrackedAddressesAreBounded() {
        var p = NearbyHandshakeLimiter.Policy()
        p.maxTrackedAddresses = 4
        var l = NearbyHandshakeLimiter(policy: p)
        for i in 0..<10 { l.recordFailure("h\(i)", now: t0.addingTimeInterval(Double(i))) }
        XCTAssertEqual(l.entries.count, 4)
        XCTAssertEqual(Set(l.entries.keys), ["h6", "h7", "h8", "h9"], "the stalest are dropped")
    }
}

/// The limiter wired into the real listener, over loopback TLS-PSK.
final class NearbyHandshakeBackoffListenerTests: XCTestCase {
    static let psk = Data(repeating: 0xA5, count: 32)
    static let wrong = Data(repeating: 0x5A, count: 32)

    private func failHandshake(_ h: ListenerHarness, identity: String) {
        let c = NearbyTestClient(port: h.port, identity: identity, psk: Self.wrong)
        XCTAssertEqual(XCTWaiter.wait(for: [c.failed], timeout: loopbackWait), .completed, "handshake should fail")
        c.cancel()
    }

    private func waitForEvent(_ h: ListenerHarness, _ what: String, _ match: @escaping (NearbyListener.Event) -> Bool) {
        let deadline = Date().addingTimeInterval(loopbackWait)
        while !h.snapshot.contains(where: match), Date() < deadline { usleep(10_000) }
        XCTAssertTrue(h.snapshot.contains(where: match), "\(what): \(h.snapshot)")
    }

    func testAnAddressThatKeepsFailingIsRefusedBeforeTLS() throws {
        var limits = NearbyReceiveLimits()
        limits.handshake.freeFailures = 2
        limits.handshake.baseBackoff = 60
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: Self.psk, isBootstrap: false)], sink: RecordingSink(),
                                destinations: nil, limits: limits)
        try h.start()
        defer { h.stop() }
        failHandshake(h, identity: "pair-a")
        failHandshake(h, identity: "pair-a")
        waitForEvent(h, "two failures counted") { _ in h.listener.handshakeLimiter.entries.values.contains { $0.failures == 2 } }

        // Even the right key is not given a handshake while the address waits.
        let right = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.psk)
        waitForEvent(h, "refused before TLS") {
            if case .handshakeRefused(_, 2, let wait) = $0 { return wait > 0 && wait <= 60 }
            return false
        }
        XCTAssertEqual(XCTWaiter.wait(for: [right.ready], timeout: 1), .timedOut, "no handshake during backoff")
        XCTAssertFalse(h.snapshot.contains(.connectionOpened))
        right.cancel()
    }

    /// Simultaneous connections from one address would all be accepted
    /// before any had failed; only `maxPendingPerAddress` get a handshake.
    func testSimultaneousHandshakesFromOneAddressAreCapped() throws {
        let h = ListenerHarness(psks: [.init(identity: "pair-a", key: Self.psk, isBootstrap: false)], sink: RecordingSink(),
                                destinations: nil, limits: NearbyReceiveLimits())
        try h.start()
        defer { h.stop() }
        // Plain TCP peers that never start TLS stay in the handshake until
        // `handshakeTimeout` (10 s), so the count below is not a race.
        let stalled = (0..<5).map { _ in NWConnection(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: h.port)!, using: .tcp) }
        for (i, c) in stalled.enumerated() { c.start(queue: DispatchQueue(label: "stalled.\(i)")) }
        defer { for c in stalled { c.cancel() } }
        waitForEvent(h, "three refused") { _ in
            h.snapshot.filter { if case .handshakePending(_, 2) = $0 { return true }; return false }.count == 3
        }
        XCTAssertEqual(h.listener.openConnectionCount, 2)
        XCTAssertEqual(h.listener.handshakeLimiter.entries.count, 0, "a refusal is not a failed handshake")
        // The right key from the same address waits for a free slot too.
        let right = NearbyTestClient(port: h.port, identity: "pair-a", psk: Self.psk)
        defer { right.cancel() }
        XCTAssertEqual(XCTWaiter.wait(for: [right.ready], timeout: 1), .timedOut)
    }
}

/// An unauthenticated peer cannot take the pairing code away: failed
/// handshakes cost the failing address time, never the owner's open code.
@MainActor
final class NearbyPairingLockoutStateTests: XCTestCase {
    /// A journal written by an older build holds a six-digit code; resuming
    /// it would serve a bootstrap key that is crackable offline.
    func testAWeakJournaledCodeIsNotResumed() {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-weak-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: dir) }
        let state = NearbyState(store: PairStore(url: dir.appendingPathComponent("pairs.json")), macName: "Weak Mac", loopbackOnly: true)
        state.beginPairing(resuming: "123456", expiresAt: Date().addingTimeInterval(60))
        XCTAssertNil(state.pairingCode)
        XCTAssertNil(state.coordinator.current)
        XCTAssertEqual(state.log.last, "not resuming pairing code: not a full-strength code")
        XCTAssertFalse(state.isAdvertising)
    }

    func testFailedHandshakesDoNotWithdrawThePairingCode() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-lockout-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: dir) }
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        // Default limits: what a hostile peer on the LAN meets.
        let state = NearbyState(store: store, macName: "Lockout Mac", loopbackOnly: true, limits: NearbyReceiveLimits())
        state.attach(sink: RecordingSink(), destinations: FixedDestinations(nil))
        state.startAdvertising()
        try await eventually(timeout: loopbackWait) { state.isAdvertising && state.port != nil }
        state.beginPairing()
        let code = try XCTUnwrap(state.pairingCode)
        try await eventually(timeout: loopbackWait) { state.log.filter { $0.hasPrefix("ready on port") }.count >= 2 }
        let port = try XCTUnwrap(state.port)
        let pairId = Pairing.derive(code: code, salt: store.salt).pairId
        // Many wrong-key handshakes at once (concurrent connections are
        // accepted before any of them has failed), then more one by one.
        let burst = (0..<14).map { _ in NearbyTestClient(port: port, identity: pairId, psk: Data(repeating: 0x11, count: 32)) }
        try await eventually(timeout: loopbackWait) { burst.allSatisfy { $0.isClosed || $0.isFailed } }
        for c in burst { c.cancel() }
        for _ in 0..<4 {
            let c = NearbyTestClient(port: port, identity: pairId, psk: Data(repeating: 0x22, count: 32))
            try await eventually(timeout: loopbackWait) { c.isClosed || c.isFailed }
            c.cancel()
        }
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertEqual(state.pairingCode, code, "the code is still served: \(state.log)")
        XCTAssertNotNil(state.coordinator.current)
        XCTAssertFalse(state.log.contains { $0.contains("withdrawn") }, "\(state.log)")
        state.stopAdvertising()
    }
}
