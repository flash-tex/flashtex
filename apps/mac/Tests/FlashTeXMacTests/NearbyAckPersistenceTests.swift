import Network
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Acknowledgement memory across `NearbyState.stopAdvertising()` and a
/// relaunch (mac-nearby-transport-3). Before this lane the memory lived with
/// the listener object, so turning advertising off and on within a session
/// re-delivered a companion's retry. Now the state owns one memory for the
/// session and the acknowledged entries are persisted (bounded) with the
/// pairing record in `pairs.json`, so a restarted listener — or a relaunched
/// app — acknowledges the retry again and never hands it to the sink twice.
/// Loopback only.
@MainActor
final class NearbyAckPersistenceTests: XCTestCase {
    static let psk = Data(repeating: 0x7D, count: 32)
    static let pairId = "pair-persist"
    let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-persist-\(UUID().uuidString)")
    var storeURL: URL { tmp.appendingPathComponent("pairs.json") }
    private var retained: [AnyObject] = []

    override func tearDown() { try? FileManager.default.removeItem(at: tmp) }

    func makeState(store: PairStore? = nil, limits: NearbyReceiveLimits = .init()) -> (NearbyState, PairStore, RecordingSink) {
        let store = store ?? PairStore(url: storeURL)
        if store.pair(id: Self.pairId) == nil {
            XCTAssertTrue(store.upsert(PairRecord(pairId: Self.pairId, psk: Self.psk.base64EncodedString(), companionName: "Persist iPad",
                                                  createdAt: Date(), lastSeenAt: nil)))
        }
        let sink = RecordingSink()
        let destinations = FixedDestinations(.init(destinationId: "dest-p", projectId: "proj-p", path: "main.tex", baseRevision: 2))
        retained.append(destinations)
        let state = NearbyState(store: store, macName: "Persist Mac", loopbackOnly: true, limits: limits)
        state.attach(sink: sink, destinations: destinations)
        return (state, store, sink)
    }

    struct WaitTimeout: Error, CustomStringConvertible {
        let what: String
        var description: String { "timed out waiting for \(what)" }
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 8, file: StaticString = #filePath, line: UInt = #line,
                           _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw WaitTimeout(what: what)
    }

    func connect(_ state: NearbyState, file: StaticString = #filePath, line: UInt = #line) async throws -> NearbyTestClient {
        try await waitUntil("advertising", file: file, line: line) { state.isAdvertising && state.port != nil }
        let c = NearbyTestClient(port: try XCTUnwrap(state.port, file: file, line: line), identity: Self.pairId, psk: Self.psk)
        try await waitUntil("client ready (\(String(describing: c.failure)))", file: file, line: line) { c.isReady || c.isClosed }
        XCTAssertTrue(c.isReady, String(describing: c.failure), file: file, line: line)
        let nonce = UUID().uuidString
        c.send(id: "h", type: "hello", NearbyV1.Hello(pairId: Self.pairId, companionName: "Persist iPad", nonce: nonce,
                                                      proof: Pairing.helloProof(psk: Self.psk, nonce: nonce)))
        try await waitUntil("hello_ack", file: file, line: line) { c.lineCount >= 1 }
        return c
    }

    /// `waitUntil` must throw on timeout. The old `XCTFail` + fall-through let
    /// callers keep going and index `allLines[1]` on an empty reply, turning
    /// a slow loopback client into `Fatal error: Index out of range`.
    func testWaitUntilThrowsOnTimeout() async {
        do {
            try await waitUntil("never true", timeout: 0.05) { false }
            XCTFail("waitUntil fell through instead of throwing on timeout")
        } catch {
            XCTAssertTrue("\(error)".contains("never true"), "unexpected error: \(error)")
        }
    }

    static let png = TestImages.png(width: 24, height: 24)
    func submit(_ captureId: String, revision: Int = 2, instructions: String = "handwritten-3f9") -> RuntimeV1.CaptureSubmit {
        .init(captureId: captureId, destinationId: "dest-p", baseRevision: revision,
              image: .init(mimeType: "image/png", dataBase64: Self.png.base64EncodedString()), instructions: instructions)
    }
    func ack(_ line: Data) -> NearbyV1.CaptureReceived? {
        let e = try? JSONDecoder().decode(RuntimeV1.Envelope<NearbyV1.CaptureReceived>.self, from: line)
        return e?.type == "capture_received" ? e?.payload : nil
    }
    func errorCode(_ line: Data) -> String? {
        (try? JSONDecoder().decode(NearbyListenerTests.ErrorLine.self, from: line))?.payload.code
    }

    /// Off/on within a session: the retry against the new listener is a
    /// duplicate (acknowledged, counted, not delivered); a different payload
    /// or revision under the same id is still named as a conflict.
    func testRetryAfterStopAndStartAdvertisingIsAcknowledgedNotRedelivered() async throws {
        let (state, store, sink) = makeState()
        state.startAdvertising()
        let a = try await connect(state)
        a.send(id: "s1", type: "capture_submit", submit("cap-p1"))
        try await waitUntil("ack 1") { a.lineCount >= 2 }
        XCTAssertEqual(ack(a.allLines[1])?.captureId, "cap-p1")
        XCTAssertEqual(sink.count, 1)
        try await waitUntil("persisted") { store.pair(id: Self.pairId)?.rememberedCaptures?.count == 1 }
        let persisted = try XCTUnwrap(store.pair(id: Self.pairId)?.rememberedCaptures?.first)
        XCTAssertEqual(persisted.captureId, "cap-p1")
        XCTAssertEqual(persisted.baseRevision, 2)
        XCTAssertEqual(persisted.ack.captureId, "cap-p1")
        XCTAssertEqual(persisted.digestHex.count, 64)

        state.stopAdvertising()
        try await waitUntil("client closed by the stop") { a.isClosed }
        XCTAssertFalse(state.isAdvertising)
        XCTAssertEqual(state.ackMemory.count(pairId: Self.pairId), 1, "the state keeps the session's memory")

        state.startAdvertising()
        let b = try await connect(state)
        b.send(id: "s2", type: "capture_submit", submit("cap-p1"))
        try await waitUntil("ack 2") { b.lineCount >= 2 }
        XCTAssertEqual(ack(b.allLines[1])?.captureId, "cap-p1", "acknowledged again after the restart")
        XCTAssertEqual(sink.count, 1, "not re-delivered")
        try await waitUntil("duplicate counted") { state.duplicateCaptureCount == 1 }
        b.send(id: "s3", type: "capture_submit", submit("cap-p1", instructions: "other"))
        try await waitUntil("conflict") { b.lineCount >= 3 }
        XCTAssertEqual(errorCode(b.allLines[2]), "capture_id_conflict")
        b.send(id: "s4", type: "capture_submit", submit("cap-p1", revision: 3))
        try await waitUntil("mismatch") { b.lineCount >= 4 }
        XCTAssertEqual(errorCode(b.allLines[3]), "revision_mismatch")
        XCTAssertEqual(sink.count, 1)
        b.cancel()
        state.stopAdvertising()
    }

    /// Relaunch: a fresh `PairStore` + `NearbyState` over the same
    /// `pairs.json` seeds the memory from the record, so the first retry
    /// after the relaunch is acknowledged from disk; a new capture is
    /// delivered and persisted in turn; the file never carries the image.
    func testRelaunchSeedsTheMemoryFromPairsJSON() async throws {
        do {
            let (state, _, sink) = makeState()
            state.startAdvertising()
            let a = try await connect(state)
            a.send(id: "s1", type: "capture_submit", submit("cap-relaunch"))
            try await waitUntil("ack") { a.lineCount >= 2 }
            XCTAssertEqual(sink.count, 1)
            a.cancel()
            state.stopAdvertising()
        }
        let raw = try String(contentsOf: storeURL, encoding: .utf8)
        XCTAssertTrue(raw.contains("\"remembered_captures\""), raw)
        XCTAssertTrue(raw.contains("\"digest_sha256\""), raw)
        XCTAssertFalse(raw.contains(Self.png.base64EncodedString().prefix(40)), "no image bytes in pairs.json")
        XCTAssertFalse(raw.contains("handwritten-3f9"), "no instructions in pairs.json")

        let (state, store, sink) = makeState()
        XCTAssertEqual(state.ackMemory.count(pairId: Self.pairId), 1, "seeded from the record")
        XCTAssertEqual(state.ackMemory.lookup(pairId: Self.pairId, captureId: "cap-relaunch")?.ack?.captureId, "cap-relaunch")
        state.startAdvertising()
        let b = try await connect(state)
        b.send(id: "s2", type: "capture_submit", submit("cap-relaunch"))
        try await waitUntil("ack from disk") { b.lineCount >= 2 }
        XCTAssertEqual(ack(b.allLines[1])?.captureId, "cap-relaunch")
        XCTAssertEqual(sink.count, 0, "acknowledged from the persisted entry, never delivered after the relaunch")
        try await waitUntil("duplicate counted") { state.duplicateCaptureCount == 1 }
        b.send(id: "s3", type: "capture_submit", submit("cap-new"))
        try await waitUntil("new ack") { b.lineCount >= 3 }
        XCTAssertEqual(sink.count, 1)
        try await waitUntil("second entry persisted") { store.pair(id: Self.pairId)?.rememberedCaptures?.count == 2 }
        XCTAssertEqual(store.pair(id: Self.pairId)?.rememberedCaptures?.map(\.captureId), ["cap-relaunch", "cap-new"])
        XCTAssertEqual(store.pair(id: Self.pairId)?.captureCount, 2)
        b.cancel()
        state.stopAdvertising()
    }

    /// The record keeps at most `PairStore.maxRememberedCaptures` entries
    /// (oldest dropped), a re-acknowledged id is not duplicated, the seeded
    /// memory honours its own bound, and forgetting the pairing drops both
    /// the persisted entries and the live memory.
    func testPersistedEntriesAreBoundedAndFollowTheForgottenPairing() throws {
        let store = PairStore(url: storeURL)
        XCTAssertTrue(store.upsert(PairRecord(pairId: Self.pairId, psk: Self.psk.base64EncodedString(), companionName: "Persist iPad",
                                              createdAt: Date(), lastSeenAt: nil)))
        func entry(_ i: Int) -> NearbyAckMemory.Persisted {
            .init(captureId: "cap-\(i)", baseRevision: 1, digestHex: String(repeating: "ab", count: 32),
                  ack: .init(captureId: "cap-\(i)", durable: false, hasProposal: false, applied: false))
        }
        for i in 0..<(PairStore.maxRememberedCaptures + 10) {
            store.recordCapture(pairId: Self.pairId, captureId: "cap-\(i)", remembered: entry(i))
        }
        store.recordCapture(pairId: Self.pairId, captureId: "cap-70", remembered: entry(70))
        let list = try XCTUnwrap(store.pair(id: Self.pairId)?.rememberedCaptures)
        XCTAssertEqual(list.count, PairStore.maxRememberedCaptures)
        XCTAssertEqual(list.first?.captureId, "cap-10", "oldest dropped, cap-70 re-acknowledged moved to the end")
        XCTAssertEqual(list.last?.captureId, "cap-70")
        XCTAssertEqual(list.filter { $0.captureId == "cap-70" }.count, 1)
        XCTAssertEqual(store.pair(id: Self.pairId)?.captureCount, PairStore.maxRememberedCaptures + 11)

        // A reloaded store round-trips the entries; a tighter listener bound
        // trims the seeded memory, oldest first.
        let reloaded = PairStore(url: storeURL)
        XCTAssertEqual(reloaded.pair(id: Self.pairId)?.rememberedCaptures, list)
        var limits = NearbyReceiveLimits()
        limits.maxRememberedCaptures = 8
        let state = NearbyState(store: reloaded, macName: "Persist Mac", loopbackOnly: true, limits: limits)
        XCTAssertEqual(state.ackMemory.count(pairId: Self.pairId), 8)
        XCTAssertNil(state.ackMemory.lookup(pairId: Self.pairId, captureId: "cap-10"))
        XCTAssertNotNil(state.ackMemory.lookup(pairId: Self.pairId, captureId: "cap-70"))
        XCTAssertEqual(state.ackMemory.snapshot(pairId: Self.pairId).map(\.captureId), Array(list.suffix(8)).map(\.captureId))

        state.forget(pairId: Self.pairId)
        XCTAssertEqual(state.ackMemory.count(pairId: Self.pairId), 0)
        XCTAssertNil(reloaded.pair(id: Self.pairId))
        XCTAssertFalse(try String(contentsOf: storeURL, encoding: .utf8).contains("cap-70"))
    }

    /// A v2 file without `remembered_captures` still decodes (nil), and a
    /// pending (unacknowledged) entry is never part of a snapshot.
    func testOlderRecordsDecodeAndPendingEntriesAreNotPersisted() throws {
        let v2 = """
        {"version": 2, "salt": "\(Pairing.hex(Pairing.generateSalt()))", "pairs": [{"pair_id": "old", "psk": "\(Self.psk.base64EncodedString())",
         "companion_name": "Old", "created_at": "2026-09-12T00:00:00Z", "generation": 3, "capture_count": 4}]}
        """
        try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        try Data(v2.utf8).write(to: storeURL)
        let store = PairStore(url: storeURL)
        XCTAssertNil(store.loadError)
        XCTAssertEqual(store.pair(id: "old")?.captureCount, 4)
        XCTAssertNil(store.pair(id: "old")?.rememberedCaptures)

        let memory = NearbyAckMemory(maxPerPair: 4)
        memory.remember(pairId: "old", captureId: "pending", baseRevision: 1, digest: Data(repeating: 1, count: 32))
        memory.remember(pairId: "old", captureId: "done", baseRevision: 1, digest: Data(repeating: 2, count: 32))
        _ = memory.acknowledge(pairId: "old", captureId: "done", ack: .init(captureId: "done", durable: true, hasProposal: true, applied: false))
        XCTAssertEqual(memory.snapshot(pairId: "old").map(\.captureId), ["done"])
        XCTAssertEqual(memory.snapshot(pairId: "old").first?.ack.durable, true)
        // Restoring over live entries leaves them alone.
        memory.restore(pairId: "old", entries: [.init(captureId: "pending", baseRevision: 9, digestHex: "00", ack: .init(captureId: "pending", durable: false, hasProposal: false, applied: false))])
        XCTAssertEqual(memory.lookup(pairId: "old", captureId: "pending")?.baseRevision, 1)
        XCTAssertNil(memory.lookup(pairId: "old", captureId: "pending")?.ack)
    }
}
