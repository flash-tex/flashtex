import Network
import XCTest
@testable import FlashTeXMac

/// The pairing code is a 256-bit secret, and failed handshakes back off per
/// address and end a pairing attempt (proposal nearby-v1 §5, lane
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
            l.recordFailure("10.0.0.9", bootstrap: nil, now: t0)
        }
        XCTAssertEqual(l.retryAfter("10.0.0.9", now: t0), 1)
        XCTAssertNil(l.retryAfter("10.0.0.10", now: t0), "other addresses are not affected")
        XCTAssertNil(l.retryAfter("10.0.0.9", now: t0.addingTimeInterval(1)))
        l.recordFailure("10.0.0.9", bootstrap: nil, now: t0.addingTimeInterval(1))
        XCTAssertEqual(l.retryAfter("10.0.0.9", now: t0.addingTimeInterval(1)), 2)
        for k in 0..<20 { l.recordFailure("10.0.0.9", bootstrap: nil, now: t0.addingTimeInterval(2 + Double(k))) }
        XCTAssertEqual(l.retryAfter("10.0.0.9", now: t0.addingTimeInterval(21)), 300, "capped at maxBackoff")
        XCTAssertNil(l.retryAfter("10.0.0.9", now: t0.addingTimeInterval(21 + 600)), "forgotten after forgetAfter")
    }

    func testSuccessClearsTheAddress() {
        var l = NearbyHandshakeLimiter()
        for _ in 0..<5 { l.recordFailure("a", bootstrap: nil, now: t0) }
        XCTAssertNotNil(l.retryAfter("a", now: t0))
        l.recordSuccess("a")
        XCTAssertNil(l.retryAfter("a", now: t0))
        XCTAssertEqual(l.failures("a"), 0)
    }

    func testLockoutFiresOncePerCodeAcrossAddresses() {
        var l = NearbyHandshakeLimiter()
        var fired: [Int] = []
        for i in 0..<25 where l.recordFailure("10.0.1.\(i)", bootstrap: "code-1", now: t0) { fired.append(i) }
        XCTAssertEqual(fired, [9], "the 10th failure locks the code out, once")
        // A new code starts counting again; failures with no code served never lock out.
        XCTAssertFalse(l.recordFailure("x", bootstrap: "code-2", now: t0))
        XCTAssertEqual(l.bootstrapFailures, 1)
        for _ in 0..<20 { XCTAssertFalse(l.recordFailure("y", bootstrap: nil, now: t0)) }
    }

    func testTrackedAddressesAreBounded() {
        var p = NearbyHandshakeLimiter.Policy()
        p.maxTrackedAddresses = 4
        var l = NearbyHandshakeLimiter(policy: p)
        for i in 0..<10 { l.recordFailure("h\(i)", bootstrap: nil, now: t0.addingTimeInterval(Double(i))) }
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

    func testFailedHandshakesWhileACodeIsServedLockThePairingOut() throws {
        var limits = NearbyReceiveLimits()
        limits.handshake.freeFailures = 100 // one loopback address: test the per-code count alone
        limits.handshake.bootstrapLockout = 3
        let boot = Pairing.derive(code: Pairing.generateCode(), salt: Pairing.generateSalt())
        let h = ListenerHarness(psks: [.init(identity: boot.pairId, key: boot.psk, isBootstrap: true, generation: 1)],
                                sink: RecordingSink(), destinations: nil, limits: limits)
        try h.start()
        defer { h.stop() }
        for _ in 0..<4 { failHandshake(h, identity: boot.pairId) }
        waitForEvent(h, "locked out") { $0 == .pairingLockedOut(failures: 3) }
        waitForEvent(h, "fourth failure counted") { _ in h.listener.handshakeLimiter.bootstrapFailures == 4 }
        XCTAssertEqual(h.snapshot.filter { if case .pairingLockedOut = $0 { return true }; return false }.count, 1, "\(h.snapshot)")
    }
}

/// NearbyState withdraws the code when the listener reports the lockout.
@MainActor
final class NearbyPairingLockoutStateTests: XCTestCase {
    func testLockoutWithdrawsThePairingCode() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-lockout-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: dir) }
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        var limits = NearbyReceiveLimits()
        limits.handshake.freeFailures = 100
        limits.handshake.bootstrapLockout = 2
        let state = NearbyState(store: store, macName: "Lockout Mac", loopbackOnly: true, limits: limits)
        state.attach(sink: RecordingSink(), destinations: FixedDestinations(nil))
        state.startAdvertising()
        try await eventually(timeout: loopbackWait) { state.isAdvertising && state.port != nil }
        state.beginPairing()
        let code = try XCTUnwrap(state.pairingCode)
        XCTAssertEqual(code.count, Pairing.codeLength)
        try await eventually(timeout: loopbackWait) { state.log.filter { $0.hasPrefix("ready on port") }.count >= 2 }
        let port = try XCTUnwrap(state.port)
        let pairId = Pairing.derive(code: code, salt: store.salt).pairId
        for _ in 0..<2 {
            let c = NearbyTestClient(port: port, identity: pairId, psk: Data(repeating: 0x11, count: 32))
            try await eventually(timeout: loopbackWait) { c.isFailed }
            XCTAssertTrue(c.isFailed)
            c.cancel()
        }
        try await eventually(timeout: loopbackWait) { state.pairingCode == nil }
        XCTAssertNil(state.pairingCode, "\(state.log)")
        XCTAssertNil(state.coordinator.current)
        XCTAssertTrue(state.log.contains("pairing code withdrawn after 2 failed handshakes"), "\(state.log)")
        state.stopAdvertising()
    }
}
