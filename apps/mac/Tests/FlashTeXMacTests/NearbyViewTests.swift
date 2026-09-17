import AppKit
import Foundation
import Network
import SwiftUI
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// `PairingFlowController` against a real `NearbyState` (loopback TLS-PSK):
/// what the Nearby Companion window shows for each transport event, durable
/// recovery across a simulated relaunch, stale reconnects, and cancel/resume.
/// Owner: mac-pairing-ui.
@MainActor
final class NearbyViewControllerTests: XCTestCase {
    private var dir: URL!
    private var announced: [String] = []

    override func setUp() {
        super.setUp()
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-view-\(UUID().uuidString)")
        announced = []
    }

    override func tearDown() {
        try? FileManager.default.removeItem(at: dir)
        super.tearDown()
    }

    private func makeState(name: String = "Flow Mac") -> (NearbyState, ShellModel) {
        let store = PairStore(url: dir.appendingPathComponent("pairs.json"))
        let model = ShellModel()
        let state = NearbyState(store: store, macName: name, loopbackOnly: true)
        state.attach(sink: model, destinations: model)
        return (state, model)
    }

    private func makeController(_ state: NearbyState) -> PairingFlowController {
        PairingFlowController(nearby: state, journal: PairingJournal(url: dir.appendingPathComponent("pairing-session.json")),
                              announcer: { [weak self] in self?.announced.append($0) })
    }

    struct TimedOut: Error {}

    private func waitUntil(_ what: String, timeout: TimeInterval = 6, file: StaticString = #filePath, line: UInt = #line,
                           _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        XCTFail("timed out waiting for \(what)", file: file, line: line)
        throw TimedOut()
    }

    /// Opens a bootstrap connection with `code` and completes hello; returns
    /// the client and the long-term key from `hello_ack`.
    private func pair(code: String, port: UInt16, salt: Data, companion: String) async throws -> (NearbyTestClient, Data?) {
        let derived = Pairing.derive(code: code, salt: salt)
        let client = NearbyTestClient(port: port, identity: derived.pairId, psk: derived.psk)
        try await waitUntil("client ready (\(String(describing: client.failure)))") { client.isReady }
        let nonce = UUID().uuidString
        client.send(id: "h", type: "hello", NearbyV1.Hello(pairId: derived.pairId, companionName: companion, nonce: nonce,
                                                           proof: Pairing.helloProof(psk: derived.psk, nonce: nonce)))
        try await waitUntil("hello_ack") { client.lineCount >= 1 }
        let ack = try JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.HelloAck>.self, from: client.allLines[0])
        return (client, ack.payload.pairPsk.flatMap { Data(base64Encoded: $0) })
    }

    /// A `capture_submit` line big enough for several `.receiving` reports.
    private func bigCaptureLine(captureId: String) throws -> Data {
        var submit = try JSONDecoder().decode(RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>.self,
                                              from: Data(contentsOf: NearbyListenerTests.fixtureURL)).payload
        submit.captureId = captureId
        submit.image = .init(mimeType: "image/png", dataBase64: TestImages.png(width: 260, height: 260, noise: true).base64EncodedString())
        let line = NearbyV1.line(id: captureId, type: "capture_submit", submit)
        XCTAssertGreaterThan(line.count, 3 * NearbyListener.receivingReportInterval)
        return line
    }

    /// Sends `line[..<upTo]` in small chunks so progress reports interleave.
    private func trickle(_ client: NearbyTestClient, _ line: Data, upTo: Int) async throws {
        var i = 0
        while i < upTo {
            let end = min(i + 16 * 1024, upTo)
            client.send(line[i..<end])
            i = end
            try await Task.sleep(nanoseconds: 2_000_000)
        }
    }

    // MARK: live flow

    func testShowCodePairsAndJournalsThroughTheController() async throws {
        let (state, model) = makeState()
        let c = makeController(state)
        XCTAssertEqual(c.phase, .off)
        XCTAssertEqual(c.phase.title, "Off")

        c.showCode()
        guard case .codeShown(let a) = c.phase else { return XCTFail("\(c.phase)") }
        XCTAssertEqual(a.code, state.pairingCode)
        XCTAssertEqual(a.generation, 1)
        XCTAssertEqual(c.journal.pending, a, "the attempt is durable as soon as it is shown")
        XCTAssertEqual(c.journal.generation, 1)
        XCTAssertEqual(announced.last, "Pairing code \(Pairing.spokenCode(a.code)), valid for 120 seconds.")
        try await waitUntil("advertising with the bootstrap key") { state.isAdvertising && state.port != nil }
        XCTAssertTrue(c.machine.isAdvertising)
        XCTAssertFalse(c.phase.canShowCode(), "one code at a time")
        XCTAssertTrue(c.phase.canCancel())

        XCTAssertEqual(state.coordinator.current?.generation, 1, "the transport serves the journal's generation")
        let (client, longTerm) = try await pair(code: a.code, port: state.port!, salt: state.store.salt, companion: "Flow iPad")
        XCTAssertNotNil(longTerm)
        try await waitUntil("paired") { if case .paired = c.phase { return true }; return false }
        XCTAssertEqual(c.phase, .paired(.init(pairId: a.pairId, companionName: "Flow iPad", generation: 1)))
        XCTAssertEqual(state.store.pair(id: a.pairId)?.generation, 1, "pairs.json v2 records the confirming attempt")
        XCTAssertTrue(c.announcements.contains("A companion connected; verifying the code."), "live .connectionOpened → verifying: \(c.announcements)")
        XCTAssertNil(c.journal.pending, "journal cleared once the pairing is stored")
        XCTAssertEqual(state.pairs.map(\.companionName), ["Flow iPad"])
        XCTAssertTrue(announced.contains("Paired with Flow iPad."), "\(announced)")
        XCTAssertTrue(c.staleInputs.isEmpty, "\(c.staleInputs)")
        // The transport's own code state is consistent with ours.
        XCTAssertNil(state.pairingCode)
        try await waitUntil("connected") { state.connectedPairIds == [a.pairId] }
        XCTAssertEqual(PairingAccessibility.deviceRow(state.pairs[0], connected: true).value.hasPrefix("Connected."), true)

        // A capture is announced with its id; the inbox row is not durable.
        var fixture = try Data(contentsOf: NearbyListenerTests.fixtureURL)
        if fixture.last != 0x0A { fixture.append(0x0A) }
        client.send(fixture)
        try await waitUntil("capture") { state.lastReceivedCaptureId == "fixture-capture-1" }
        XCTAssertEqual(announced.filter { $0 == "Received capture fixture-capture-1." }.count, 1, "announced once, from the live event: \(announced)")
        XCTAssertEqual(model.nearbyInbox.received.map(\.captureId), ["fixture-capture-1"])

        c.dismiss()
        XCTAssertEqual(c.phase, .advertising)
        // Forget through the controller ends with the device gone and the state advertising.
        c.forget(pairId: a.pairId)
        XCTAssertEqual(state.pairs, [])
        XCTAssertEqual(c.phase, .advertising)
        try await waitUntil("session closed") { client.isClosed }
        state.stopAdvertising()
        XCTAssertEqual(c.phase, .off)
    }

    func testCancelWithdrawsTheBootstrapKeyAndClearsTheJournal() async throws {
        let (state, _) = makeState()
        let c = makeController(state)
        c.showCode()
        guard case .codeShown(let a) = c.phase else { return XCTFail("\(c.phase)") }
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let port = state.port!

        c.cancel()
        XCTAssertEqual(c.phase, .advertising, "cancel returns to advertising; the listener stays up")
        XCTAssertNil(state.pairingCode)
        XCTAssertNil(state.coordinator.current)
        XCTAssertNil(c.journal.pending)
        XCTAssertEqual(announced.last, "Pairing cancelled.")
        XCTAssertTrue(c.staleInputs.isEmpty, "our own withdrawal is not mistaken for expiry: \(c.staleInputs)")
        try await waitUntil("restarted without the key") { state.log.filter { $0.hasPrefix("ready on port") }.count >= 2 }
        XCTAssertEqual(state.port, port)
        let derived = Pairing.derive(code: a.code, salt: state.store.salt)
        let late = NearbyTestClient(port: port, identity: derived.pairId, psk: derived.psk)
        try await waitUntil("cancelled code refused") { late.isFailed }
        XCTAssertFalse(late.isReady)
        XCTAssertEqual(c.phase, .advertising, "a stale bootstrap attempt changes nothing")
        state.stopAdvertising()
    }

    // MARK: durable recovery

    func testRelaunchRestoresThePendingCodeAndResumeServesTheSameCode() async throws {
        // Launch 1: show a code, then "quit" (drop the state and controller).
        var code = ""
        var generation = 0
        do {
            let (state, _) = makeState()
            let c = makeController(state)
            c.showCode()
            guard case .codeShown(let a) = c.phase else { return XCTFail("\(c.phase)") }
            code = a.code
            generation = a.generation
            try await waitUntil("advertising") { state.port != nil }
            c.setAdvertising(false)
            XCTAssertEqual(c.phase, .interrupted(a, .transportStopped, detail: "advertising was turned off"))
            XCTAssertFalse(state.isAdvertising)
            XCTAssertEqual(c.journal.pending, a, "still pending: the user has not cancelled")
        }

        // Launch 2: same store and journal, fresh transport.
        let (state, _) = makeState()
        XCTAssertEqual(state.pairs, [])
        let c = makeController(state)
        guard case .interrupted(let a, .relaunch, let detail) = c.phase else { return XCTFail("\(c.phase)") }
        XCTAssertEqual(a.code, code)
        XCTAssertEqual(a.generation, generation)
        XCTAssertEqual(detail, "FlashTeX was quit while the code was valid")
        XCTAssertEqual(c.phase.title, "Pairing interrupted")
        XCTAssertTrue(c.phase.canResume())
        XCTAssertTrue(c.phase.canCancel())
        XCTAssertFalse(c.phase.canShowCode())
        XCTAssertEqual(announced.last, "A pairing from a previous launch is waiting. Resume it or cancel.")
        XCTAssertNil(state.pairingCode, "the transport has not been asked for anything yet")

        c.resume()
        XCTAssertEqual(c.phase, .codeShown(a))
        XCTAssertEqual(state.pairingCode, code, "resumed through beginPairing(resuming:): the transport shows the same code")
        XCTAssertEqual(state.codeExpiresAt.map { Int($0.timeIntervalSince1970) }, Int(a.expiresAt.timeIntervalSince1970))
        XCTAssertEqual(state.coordinator.current?.generation, generation, "same attempt generation as the journal")
        XCTAssertEqual(c.machine.generation, generation, "resume did not mint a new generation")
        try await waitUntil("advertising after resume") { state.isAdvertising && state.port != nil }
        let (client, longTerm) = try await pair(code: code, port: state.port!, salt: state.store.salt, companion: "Resumed iPad")
        XCTAssertNotNil(longTerm, "the resumed code confirms a pairing")
        try await waitUntil("paired") { if case .paired = c.phase { return true }; return false }
        XCTAssertEqual(state.pairs.map(\.companionName), ["Resumed iPad"])
        XCTAssertEqual(state.store.pair(id: a.pairId)?.generation, generation)
        XCTAssertNil(c.journal.pending)
        XCTAssertNil(state.coordinator.current, "bootstrap key dropped after confirmation")
        XCTAssertNil(state.pairingCode)
        _ = client
        state.stopAdvertising()
    }

    func testRelaunchWithExpiredCodeIsDismissedAndNewCodeUsesNewGeneration() async throws {
        let journal = PairingJournal(url: dir.appendingPathComponent("pairing-session.json"))
        let old = PairingFlow.Attempt(generation: journal.nextGeneration(), code: "111111", pairId: "stale",
                                      startedAt: Date().addingTimeInterval(-300), expiresAt: Date().addingTimeInterval(-180))
        journal.setPending(old)
        let (state, _) = makeState()
        let c = makeController(state)
        guard case .interrupted(let a, .relaunch, _) = c.phase, a.code == old.code, a.generation == 1 else { return XCTFail("\(c.phase)") }
        XCTAssertFalse(c.phase.canResume())
        XCTAssertTrue(c.phase.canDismiss())
        XCTAssertTrue(c.phase.canShowCode())
        c.dismiss()
        XCTAssertEqual(c.phase, .off)
        XCTAssertNil(c.journal.pending)
        c.showCode()
        guard case .codeShown(let b) = c.phase else { return XCTFail("\(c.phase)") }
        XCTAssertEqual(b.generation, 2)
        XCTAssertTrue(c.apply(.confirmed(pairId: "stale", companionName: "ghost", generation: 1)).stale,
                      "a reconnect from the pre-relaunch attempt cannot pair")
        XCTAssertEqual(c.phase, .codeShown(b))
        c.cancel()
        state.stopAdvertising()
    }

    func testResumedCodeExpiresIntoAnErrorStateAndDropsTheKey() async throws {
        let journal = PairingJournal(url: dir.appendingPathComponent("pairing-session.json"))
        let short = PairingFlow.Attempt(generation: journal.nextGeneration(), code: "222222", pairId: "short",
                                        startedAt: Date(), expiresAt: Date().addingTimeInterval(1.2))
        journal.setPending(short)
        let (state, _) = makeState()
        let c = makeController(state)
        XCTAssertTrue(c.phase.canResume())
        c.resume()
        guard case .codeShown(let shown) = c.phase, shown.code == short.code else { return XCTFail("\(c.phase)") }
        XCTAssertEqual(state.coordinator.current?.code, "222222")
        XCTAssertEqual(state.pairingCode, "222222")
        try await waitUntil("expired", timeout: 5) { if case .failed = c.phase { return true }; return false }
        XCTAssertEqual(c.phase, .failed(reason: "The pairing code expired before a companion paired.", generation: 1))
        XCTAssertNil(c.journal.pending)
        XCTAssertNil(state.coordinator.bootstrapEntry, "an expired bootstrap key is no longer offered")
        XCTAssertNil(state.coordinator.current, "the resumed attempt is dropped from the coordinator on expiry")
        XCTAssertTrue(c.phase.canDismiss())
        c.dismiss()
        XCTAssertEqual(c.phase, .advertising)
        state.stopAdvertising()
    }

    // MARK: stale reconnects and replacement

    func testReplacedCodeMakesTheOldSessionStaleEndToEnd() async throws {
        let (state, _) = makeState()
        let c = makeController(state)
        c.showCode()
        guard case .codeShown(let first) = c.phase else { return XCTFail("\(c.phase)") }
        try await waitUntil("advertising") { state.port != nil }
        c.cancel()
        c.showCode()
        guard case .codeShown(let second) = c.phase else { return XCTFail("\(c.phase)") }
        XCTAssertEqual(second.generation, 2)
        XCTAssertNotEqual(first.pairId, second.pairId)
        try await waitUntil("second key served") { state.coordinator.current?.code == second.code && state.isAdvertising }
        try await waitUntil("listener restarted for the second code") {
            // Restarts coalesce; what matters is a ready after the second code was issued.
            let issued = state.log.lastIndex { $0.hasPrefix("pairing code issued") } ?? -1
            let ready = state.log.lastIndex { $0.hasPrefix("ready on port") } ?? -1
            return ready > issued
        }

        // The old session reports late through every path the machine accepts.
        XCTAssertTrue(c.apply(.confirmed(pairId: first.pairId, companionName: "Old", generation: first.generation)).stale)
        XCTAssertTrue(c.apply(.peerGone(pairId: first.pairId, reason: "old session gone", generation: 2)).ignored, "a close for a session we are not verifying is a no-op")
        XCTAssertEqual(c.phase, .codeShown(second))
        XCTAssertEqual(state.pairs, [], "nothing was stored for the stale attempt")
        // A record persisted by an older attempt (generation 1) reconnecting as
        // a bootstrap hello is stale by its stored generation, not by pair id.
        XCTAssertTrue(state.store.upsert(PairRecord(pairId: second.pairId, psk: Pairing.mintLongTermPSK().base64EncodedString(),
                                                    companionName: "Ghost", createdAt: Date(), lastSeenAt: nil, generation: 1)))
        c.observe(.hello(pairId: second.pairId, companionName: "Ghost", bootstrap: true))
        XCTAssertEqual(c.staleInputs.count, 2, "\(c.staleInputs)")
        XCTAssertEqual(c.phase, .codeShown(second))
        XCTAssertTrue(state.store.remove(pairId: second.pairId))

        // The old bootstrap key is refused by TLS; the new one pairs.
        let oldDerived = Pairing.derive(code: first.code, salt: state.store.salt)
        let old = NearbyTestClient(port: state.port!, identity: oldDerived.pairId, psk: oldDerived.psk)
        try await waitUntil("old code refused") { old.isFailed }
        XCTAssertEqual(c.phase, .codeShown(second))
        let (_, key) = try await pair(code: second.code, port: state.port!, salt: state.store.salt, companion: "New iPad")
        XCTAssertNotNil(key)
        try await waitUntil("paired") { if case .paired = c.phase { return true }; return false }
        XCTAssertEqual(state.pairs.map(\.pairId), [second.pairId])
        state.stopAdvertising()
    }

    func testBootstrapSessionThatFailsHelloInterruptsAndResumeKeepsTheCode() async throws {
        let (state, _) = makeState()
        let c = makeController(state)
        c.showCode()
        guard case .codeShown(let a) = c.phase else { return XCTFail("\(c.phase)") }
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let derived = Pairing.derive(code: a.code, salt: state.store.salt)

        // A peer with the right code but a wrong proof: TLS opens, hello is refused, session closes.
        let bad = NearbyTestClient(port: state.port!, identity: derived.pairId, psk: derived.psk)
        try await waitUntil("bad client ready") { bad.isReady }
        try await waitUntil("verifying") { c.phase == .verifying(a) }
        XCTAssertEqual(c.phase.title, "Verifying companion")
        bad.send(id: "h", type: "hello", NearbyV1.Hello(pairId: derived.pairId, companionName: "Bad", nonce: "n",
                                                        proof: Pairing.helloProof(psk: Data(repeating: 1, count: 32), nonce: "n")))
        try await waitUntil("interrupted by the peer") { if case .interrupted(_, .peerGone, _) = c.phase { return true }; return false }
        guard case .interrupted(let same, .peerGone, let detail) = c.phase else { return XCTFail("\(c.phase)") }
        XCTAssertEqual(same, a)
        XCTAssertEqual(detail, "pair mismatch")
        XCTAssertTrue(c.phase.canResume())
        XCTAssertTrue(c.phase.canCancel())
        XCTAssertEqual(state.pairs, [])

        // Resume keeps the same code and generation without touching the transport.
        let readyLines = state.log.filter { $0.hasPrefix("ready on port") }.count
        c.resume()
        XCTAssertEqual(c.phase, .codeShown(a))
        XCTAssertEqual(state.pairingCode, a.code)
        XCTAssertEqual(state.log.filter { $0.hasPrefix("ready on port") }.count, readyLines, "no listener restart for a peer-gone resume")
        let (_, key) = try await pair(code: a.code, port: state.port!, salt: state.store.salt, companion: "Good iPad")
        XCTAssertNotNil(key)
        try await waitUntil("paired") { if case .paired = c.phase { return true }; return false }
        XCTAssertEqual(state.store.pair(id: a.pairId)?.generation, a.generation)
        state.stopAdvertising()
    }

    func testAlreadyPairedCompanionConnectingDuringACodeIsNotThePairingPeer() async throws {
        let (state, _) = makeState()
        let c = makeController(state)
        let psk = Pairing.mintLongTermPSK()
        XCTAssertTrue(state.store.upsert(PairRecord(pairId: "old-friend", psk: psk.base64EncodedString(), companionName: "Old Friend",
                                                    createdAt: Date(), lastSeenAt: nil, generation: nil)))
        state.refreshPairs()
        c.showCode()
        guard case .codeShown(let a) = c.phase else { return XCTFail("\(c.phase)") }
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let friend = NearbyTestClient(port: state.port!, identity: "old-friend", psk: psk)
        try await waitUntil("friend ready") { friend.isReady }
        try await waitUntil("verifying") { c.phase == .verifying(a) }
        let nonce = UUID().uuidString
        friend.send(id: "h", type: "hello", NearbyV1.Hello(pairId: "old-friend", companionName: "Old Friend", nonce: nonce,
                                                           proof: Pairing.helloProof(psk: psk, nonce: nonce)))
        try await waitUntil("hello_ack") { friend.lineCount >= 1 }
        try await waitUntil("back to code shown") { c.phase == .codeShown(a) }
        XCTAssertEqual(state.connectedPairIds, ["old-friend"])
        c.cancel()
        XCTAssertEqual(c.phase, .advertising)
        state.stopAdvertising()
    }

    func testLiveReceivingProgressCompletesAndCanBeCancelled() async throws {
        let (state, model) = makeState()
        let c = makeController(state)
        c.showCode()
        guard case .codeShown(let a) = c.phase else { return XCTFail("\(c.phase)") }
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let (client, _) = try await pair(code: a.code, port: state.port!, salt: state.store.salt, companion: "Sender")
        try await waitUntil("paired") { if case .paired = c.phase { return true }; return false }

        // A large capture: progress in bytes (total unknown), then received.
        let line = try bigCaptureLine(captureId: "big-1")
        try await trickle(client, line, upTo: line.count / 2)
        try await waitUntil("receiving") { if case .receiving = c.phase { return true }; return false }
        guard case .receiving(let r) = c.phase else { return XCTFail("\(c.phase)") }
        XCTAssertEqual(r.pairId, a.pairId)
        XCTAssertEqual(r.companionName, "Sender")
        XCTAssertNil(r.total, "JSON lines carry no length; the window says so")
        XCTAssertGreaterThan(r.bytes, 0)
        XCTAssertLessThan(r.bytes, line.count)
        XCTAssertEqual(c.phase.title, "Receiving capture")
        XCTAssertTrue(c.phase.detail().hasPrefix("Receiving \(r.bytes) bytes from Sender"), c.phase.detail())
        XCTAssertTrue(c.phase.canCancel())
        client.send(line[(line.count / 2)...])
        try await waitUntil("capture received") { state.lastReceivedCaptureId == "big-1" }
        try await waitUntil("back to advertising") { c.phase == .advertising }
        XCTAssertTrue(announced.contains("Received capture big-1 from Sender."), "\(announced)")
        XCTAssertEqual(model.nearbyInbox.received.map(\.captureId), ["big-1"])

        // Cancel mid-line: the Mac closes the session; the flow says so, the pairing stays.
        let second = try bigCaptureLine(captureId: "big-2")
        try await trickle(client, second, upTo: second.count / 2)
        try await waitUntil("receiving again") { if case .receiving = c.phase { return true }; return false }
        c.cancel()
        guard case .failed(let reason, _) = c.phase else { return XCTFail("\(c.phase)") }
        XCTAssertTrue(reason.hasPrefix("Receive from Sender cancelled after "), reason)
        XCTAssertTrue(reason.hasSuffix("the companion can resend it with the same capture_id."), reason)
        try await waitUntil("session closed by the Mac") { client.isClosed }
        try await waitUntil("disconnected") { state.connectedPairIds.isEmpty }
        XCTAssertEqual(state.pairs.count, 1)
        XCTAssertEqual(model.nearbyInbox.received.map(\.captureId), ["big-1"], "the cancelled capture never landed")
        guard case .failed = c.phase else { return XCTFail("the close event must not overwrite the cancel notice: \(c.phase)") }
        XCTAssertTrue(c.phase.canDismiss())
        c.dismiss()
        XCTAssertEqual(c.phase, .advertising)
        state.stopAdvertising()
    }

    func testRefusedCaptureEndsReceivingWithTheErrorCode() async throws {
        let (state, model) = makeState() // the listener holds its sink weakly; keep the model alive
        let c = makeController(state)
        c.showCode()
        guard case .codeShown(let a) = c.phase else { return XCTFail("\(c.phase)") }
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let (client, _) = try await pair(code: a.code, port: state.port!, salt: state.store.salt, companion: "Sender")
        try await waitUntil("paired") { if case .paired = c.phase { return true }; return false }
        // Big enough to report progress, then refused: the payload is not a PNG.
        var submit = try JSONDecoder().decode(RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>.self,
                                              from: Data(contentsOf: NearbyListenerTests.fixtureURL)).payload
        submit.captureId = "junk-1"
        submit.image = .init(mimeType: "image/png", dataBase64: Data(repeating: 0x41, count: 200 * 1024).base64EncodedString())
        let line = NearbyV1.line(id: "junk-1", type: "capture_submit", submit)
        try await trickle(client, line, upTo: line.count / 2)
        try await waitUntil("receiving") { if case .receiving = c.phase { return true }; return false }
        client.send(line[(line.count / 2)...])
        try await waitUntil("refused") { state.lastReceiveError?.captureId == "junk-1" }
        try await waitUntil("flow back to advertising") { c.phase == .advertising }
        XCTAssertTrue(announced.contains { $0.hasPrefix("Capture junk-1 from Sender refused: ") }, "\(announced)")
        XCTAssertNil(state.lastReceivedCaptureId)
        XCTAssertEqual(model.nearbyInbox.received, [])
        state.stopAdvertising()
    }

    // MARK: cancellation and reconnect (mac-pairing-ui-2)

    /// Cancel while a companion holds the bootstrap session (verifying): the
    /// companion receives `error pairing_cancelled` before the close, no
    /// record is written, the journal is cleared, and a hello it sends
    /// afterwards goes nowhere.
    func testCancelDuringVerifyingTellsTheCompanionAndLeavesNoRecord() async throws {
        let (state, _) = makeState()
        let c = makeController(state)
        c.showCode()
        guard case .codeShown(let a) = c.phase else { return XCTFail("\(c.phase)") }
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let derived = Pairing.derive(code: a.code, salt: state.store.salt)
        let peer = NearbyTestClient(port: state.port!, identity: derived.pairId, psk: derived.psk)
        try await waitUntil("peer ready") { peer.isReady }
        try await waitUntil("verifying") { c.phase == .verifying(a) }
        XCTAssertEqual(c.phase.step, .init(index: 3, status: .active))

        c.cancel()
        XCTAssertEqual(c.phase, .advertising)
        XCTAssertNil(c.journal.pending)
        XCTAssertNil(state.coordinator.current)
        try await waitUntil("companion told") { peer.lineCount >= 1 }
        struct Unrequested: Decodable { var id: String?; var type: String; var payload: NearbyV1.ErrorPayload }
        let e = try JSONDecoder().decode(Unrequested.self, from: peer.allLines[0])
        XCTAssertEqual(e.type, "error")
        XCTAssertEqual(e.payload.code, "pairing_cancelled")
        XCTAssertNil(e.id, "answers no request")
        XCTAssertEqual(e.payload.message, "the Mac withdrew the pairing code before hello")
        try await waitUntil("companion closed") { peer.isClosed }
        XCTAssertEqual(state.pairs, [], "no half-paired record")
        XCTAssertEqual(state.store.pairs, [])
        XCTAssertFalse(state.log.contains { $0.hasPrefix("stored pairing") }, "\(state.log)")
        try await waitUntil("close logged") { state.log.contains("closed unauthenticated: pairing code withdrawn before hello") }
        // A late hello with the cancelled code cannot pair: the session is gone.
        let nonce = UUID().uuidString
        peer.send(id: "h", type: "hello", NearbyV1.Hello(pairId: derived.pairId, companionName: "Late", nonce: nonce,
                                                         proof: Pairing.helloProof(psk: derived.psk, nonce: nonce)))
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertEqual(state.pairs, [])
        XCTAssertEqual(c.phase, .advertising)
        XCTAssertTrue(c.staleInputs.isEmpty, "\(c.staleInputs)")
        XCTAssertFalse(announced.contains { $0.contains("disconnected") }, "an unauthenticated peer has no name to announce: \(announced)")
        state.stopAdvertising()
    }

    /// A paired companion that drops and reconnects with its long-term key
    /// is back without any click on the Mac; both events are announced with
    /// its name, and the pairing phase is untouched. A forgotten (revoked)
    /// companion fails the TLS handshake and is neither connected nor announced.
    func testKnownCompanionReconnectIsAnnouncedAndRevokedIsRefused() async throws {
        let (state, _) = makeState()
        let c = makeController(state)
        c.showCode()
        guard case .codeShown(let a) = c.phase else { return XCTFail("\(c.phase)") }
        try await waitUntil("advertising") { state.isAdvertising && state.port != nil }
        let (first, longTerm) = try await pair(code: a.code, port: state.port!, salt: state.store.salt, companion: "Roaming iPad")
        let key = try XCTUnwrap(longTerm)
        try await waitUntil("paired") { if case .paired = c.phase { return true }; return false }
        try await waitUntil("connected") { state.connectedPairIds == [a.pairId] }
        c.dismiss()
        XCTAssertEqual(c.phase, .advertising)

        first.cancel()
        try await waitUntil("disconnect announced") { self.announced.contains { $0.hasPrefix("Roaming iPad disconnected: ") } }
        try await waitUntil("not connected") { state.connectedPairIds.isEmpty }
        XCTAssertEqual(c.phase, .advertising, "a paired companion dropping is not a pairing event")

        // Reconnect with the long-term key: hello(bootstrap: false).
        let again = NearbyTestClient(port: state.port!, identity: a.pairId, psk: key)
        try await waitUntil("reconnected TLS") { again.isReady }
        let nonce = UUID().uuidString
        again.send(id: "h2", type: "hello", NearbyV1.Hello(pairId: a.pairId, companionName: "Roaming iPad", nonce: nonce,
                                                           proof: Pairing.helloProof(psk: key, nonce: nonce)))
        try await waitUntil("hello_ack") { again.lineCount >= 1 }
        try await waitUntil("reconnect announced") { self.announced.contains("Roaming iPad reconnected.") }
        try await waitUntil("connected again") { state.connectedPairIds == [a.pairId] }
        XCTAssertEqual(c.phase, .advertising)
        XCTAssertEqual(state.pairs.count, 1, "no second record for a reconnect")
        XCTAssertTrue(c.staleInputs.isEmpty, "\(c.staleInputs)")

        // Revoke: the session closes with the reason and a new handshake fails.
        let before = announced.count
        c.forget(pairId: a.pairId)
        try await waitUntil("revoked session closed") { again.isClosed }
        XCTAssertEqual(state.pairs, [])
        let revoked = NearbyTestClient(port: state.port!, identity: a.pairId, psk: key)
        try await waitUntil("revoked key refused") { revoked.isFailed }
        XCTAssertFalse(revoked.isReady)
        try await Task.sleep(nanoseconds: 200_000_000)
        XCTAssertTrue(state.connectedPairIds.isEmpty)
        XCTAssertEqual(announced[before...].filter { $0.hasPrefix("Roaming iPad") }, [],
                       "a forgotten companion has no record to name: \(announced[before...])")
        XCTAssertTrue(state.log.contains { $0.hasPrefix("closed unauthenticated: handshake failed") }, "\(state.log)")
        state.stopAdvertising()
    }

    func testControllerIsSharedPerStateAndSurvivesWindowReopen() {
        let (state, _) = makeState()
        let journal = PairingJournal(url: dir.appendingPathComponent("pairing-session.json"))
        let a = PairingFlowController.controller(for: state, journal: journal)
        let b = PairingFlowController.controller(for: state)
        XCTAssertTrue(a === b)
        XCTAssertTrue(a.journal === journal)
    }
}

// MARK: - evidence: the real app window per state

/// Opt-in (`FLASHTEX_NEARBY_APP_EVIDENCE_DIR=<dir>`): launches the built
/// `FlashTeXMac` with `FLASHTEX_OPEN_WINDOW=nearby FLASHTEX_NO_ACTIVATE=1`
/// and `FLASHTEX_NEARBY_AUTOSTART=code`, drives it as a companion over
/// loopback (the code is read from the redirected journal, the port from
/// `lsof`), and captures the Nearby Companion window by id at each state.
/// Never activates the app; skipped unless the directory is set.
@MainActor
final class NearbyAppEvidenceTests: XCTestCase {
    struct TimedOut: Error {}

    private func waitUntil(_ what: String, timeout: TimeInterval = 15, _ cond: () throws -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if try cond() { return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        XCTFail("timed out waiting for \(what)")
        throw TimedOut()
    }

    private func run(_ exe: String, _ args: [String]) throws -> String {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: exe)
        p.arguments = args
        let out = Pipe()
        p.standardOutput = out
        p.standardError = Pipe()
        try p.run()
        p.waitUntilExit()
        return String(decoding: out.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
    }

    private func windowNumber(pid: Int32, title: String) -> Int? {
        guard let list = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]] else { return nil }
        return list.first { ($0[kCGWindowOwnerPID as String] as? Int32) == pid && ($0[kCGWindowName as String] as? String) == title }
            .flatMap { $0[kCGWindowNumber as String] as? Int }
    }

    func testCaptureRealWindowThroughAPairing() async throws {
        guard let out = ProcessInfo.processInfo.environment["FLASHTEX_NEARBY_APP_EVIDENCE_DIR"], !out.isEmpty else {
            throw XCTSkip("set FLASHTEX_NEARBY_APP_EVIDENCE_DIR to capture real-window evidence")
        }
        let outDir = URL(fileURLWithPath: out)
        try FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)
        let exe = Bundle(for: NearbyAppEvidenceTests.self).bundleURL.deletingLastPathComponent().appendingPathComponent("FlashTeXMac")
        guard FileManager.default.isExecutableFile(atPath: exe.path) else { throw XCTSkip("no FlashTeXMac next to the test bundle: \(exe.path)") }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-app-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: dir) }
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let journalURL = dir.appendingPathComponent("pairing-session.json")
        let storeURL = dir.appendingPathComponent("pairs.json")

        let app = Process()
        app.executableURL = exe
        app.environment = ProcessInfo.processInfo.environment.merging([
            "FLASHTEX_NO_ACTIVATE": "1", "FLASHTEX_AUTOATTACH": "0", "FLASHTEX_OPEN_WINDOW": "nearby",
            "FLASHTEX_NEARBY_AUTOSTART": "code",
            "FLASHTEX_PAIR_STORE": storeURL.path, "FLASHTEX_PAIRING_JOURNAL": journalURL.path,
            "FLASHTEX_REPO": NearbyListenerTests.fixtureURL.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().path,
        ]) { _, new in new }
        try app.run()
        defer { app.terminate() }

        func shot(_ name: String) async throws {
            try await Task.sleep(nanoseconds: 700_000_000)
            guard let w = windowNumber(pid: app.processIdentifier, title: "Nearby Companion") else { return XCTFail("no Nearby Companion window for \(name)") }
            let path = outDir.appendingPathComponent("nearby-app-\(name).png").path
            _ = try run("/usr/sbin/screencapture", ["-x", "-o", "-l", String(w), path])
            XCTAssertTrue(FileManager.default.fileExists(atPath: path), path)
        }

        // Code shown (auto-started), served on a loopback port found via lsof.
        try await waitUntil("journal pending") { PairingJournal(url: journalURL).pending != nil }
        let attempt = try XCTUnwrap(PairingJournal(url: journalURL).pending)
        var port: UInt16?
        try await waitUntil("listening port") {
            let text = try run("/usr/sbin/lsof", ["-a", "-p", String(app.processIdentifier), "-iTCP", "-sTCP:LISTEN", "-P", "-n"])
            port = text.split(separator: "\n").compactMap { line -> UInt16? in
                guard let range = line.range(of: ":", options: .backwards) else { return nil }
                return UInt16(line[range.upperBound...].split(separator: " ").first ?? "")
            }.first
            return port != nil
        }
        try await shot("1-code-shown")

        let salt = PairStore(url: storeURL).salt
        let derived = Pairing.derive(code: attempt.code, salt: salt)
        let client = NearbyTestClient(port: try XCTUnwrap(port), identity: derived.pairId, psk: derived.psk)
        try await waitUntil("client ready (\(String(describing: client.failure)))") { client.isReady }
        try await shot("2-verifying")
        let nonce = UUID().uuidString
        client.send(id: "h", type: "hello", NearbyV1.Hello(pairId: derived.pairId, companionName: "Evidence iPad", nonce: nonce,
                                                           proof: Pairing.helloProof(psk: derived.psk, nonce: nonce)))
        try await waitUntil("hello_ack") { client.lineCount >= 1 }
        try await waitUntil("pair stored") { PairStore(url: storeURL).pairs.count == 1 }
        try await shot("3-paired")

        // Receiving: half a large capture, then the rest.
        var submit = try JSONDecoder().decode(RuntimeV1.Envelope<RuntimeV1.CaptureSubmit>.self,
                                              from: Data(contentsOf: NearbyListenerTests.fixtureURL)).payload
        submit.captureId = "evidence-1"
        submit.image = .init(mimeType: "image/png", dataBase64: TestImages.png(width: 320, height: 320, noise: true).base64EncodedString())
        let line = NearbyV1.line(id: "evidence-1", type: "capture_submit", submit)
        var i = 0
        while i < line.count / 2 {
            let end = min(i + 16 * 1024, line.count / 2)
            client.send(line[i..<end])
            i = end
            try await Task.sleep(nanoseconds: 2_000_000)
        }
        try await shot("4-receiving")
        client.send(line[(line.count / 2)...])
        try await waitUntil("capture_received") { client.lineCount >= 2 }
        try await shot("5-received")
        client.cancel()
        try await shot("6-disconnected")
        XCTAssertEqual(PairStore(url: storeURL).pairs.first?.generation, attempt.generation, "pairs.json v2 carries the attempt")
    }
}
