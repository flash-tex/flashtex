import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// Capture-acceptance follow-ups (lane mac-nearby-errors): the shell's copy of
/// the bridge's pinned destination follows ordinary edits exactly as the bridge
/// does, a dropped pin is announced (never re-pinned), and `prepare` leaves the
/// row in the state the bridge holds. Hermetic against `Fixtures/fake_bridge.py`
/// (which applies the same anchor rule as `crates/bridge` `edit`); the last
/// test runs the real bridge when `FLASHTEX_BRIDGE` is set.
@MainActor
final class NearbyErrorsTests: XCTestCase {
    private static func anchor(_ start: Int, _ end: Int, valid: Bool = true, revision: Int = 3) -> TransferV1.Anchor {
        .init(destinationId: "d1", projectId: "p", path: "main.tex", pinnedRevision: revision, currentRevision: revision,
              startByte: start, endByte: end, valid: valid,
              binding: .init(projectId: "p", path: "main.tex", revision: revision, startByte: start, endByte: end, sourceSha256: "x"))
    }

    // MARK: (2) the shell mirrors the bridge's anchor rule

    /// Truth table copied from `crates/bridge/src/lib.rs` `edit`: an insertion
    /// anywhere inside or at either end of the pin, any overlap, or an edit that
    /// removes a zero-width pin's position invalidates; an edit entirely before
    /// shifts; an edit entirely after leaves the bytes alone; every edit
    /// advances `current_revision`; an already-invalid anchor is not touched.
    func testDestinationTrackingMirrorsTheBridgeAnchorRule() {
        let pin = Self.anchor(10, 14)
        func follow(_ s: Int, _ e: Int, _ rep: Int, _ a: TransferV1.Anchor = pin) -> TransferV1.Anchor {
            DestinationTracking.follow(a, startByte: s, endByte: e, replacementBytes: rep, revision: 4)
        }
        // Entirely before: shifted by the net byte delta.
        var t = follow(0, 2, 5)
        XCTAssertEqual([t.startByte, t.endByte, t.currentRevision], [13, 17, 4]); XCTAssertTrue(t.valid)
        t = follow(2, 6, 0)
        XCTAssertEqual([t.startByte, t.endByte], [6, 10]); XCTAssertTrue(t.valid)
        // Ending exactly at the pin's start is "before" (shifted), not an overlap.
        t = follow(8, 10, 1)
        XCTAssertEqual([t.startByte, t.endByte], [9, 13]); XCTAssertTrue(t.valid)
        // Entirely after: untouched bytes, revision follows.
        t = follow(14, 20, 3)
        XCTAssertEqual([t.startByte, t.endByte, t.currentRevision], [10, 14, 4]); XCTAssertTrue(t.valid)
        t = follow(30, 30, 3)
        XCTAssertEqual([t.startByte, t.endByte], [10, 14]); XCTAssertTrue(t.valid)
        // Insertion at the start, inside, or at the end of the pin: ambiguous affinity → invalid.
        for at in [10, 12, 14] {
            t = follow(at, at, 1)
            XCTAssertFalse(t.valid, "insertion at \(at)"); XCTAssertEqual(t.currentRevision, 4)
            XCTAssertEqual([t.startByte, t.endByte], [10, 14], "bytes are kept for display")
        }
        // Overlaps (partial, containing, contained).
        XCTAssertFalse(follow(8, 11, 0).valid)
        XCTAssertFalse(follow(13, 20, 0).valid)
        XCTAssertFalse(follow(5, 20, 0).valid)
        XCTAssertFalse(follow(11, 13, 2).valid)
        // Zero-width pin: an insertion exactly there or a deletion across it invalidates;
        // a deletion ending at it shifts it.
        let point = Self.anchor(10, 10)
        XCTAssertFalse(follow(10, 10, 1, point).valid)
        XCTAssertFalse(follow(9, 11, 0, point).valid)
        XCTAssertFalse(follow(10, 12, 0, point).valid)
        t = follow(8, 10, 0, point)
        XCTAssertTrue(t.valid); XCTAssertEqual([t.startByte, t.endByte], [8, 8])
        t = follow(10, 12, 0, Self.anchor(10, 14))
        XCTAssertFalse(t.valid, "deleting the pin's first bytes overlaps it")
        // Already invalid: exactly what the bridge does (it filters on `valid`) — nothing changes.
        let dead = Self.anchor(10, 14, valid: false)
        XCTAssertEqual(follow(0, 2, 5, dead), dead)
        XCTAssertEqual(follow(12, 12, 1, dead), dead)
        // The region form uses the replacement's UTF-8 length and the old end.
        let r = SourceMapping.ChangedRegion(startByte: 0, oldEndByte: 1, newEndByte: 3, replacement: "é\u{301}")
        XCTAssertEqual(DestinationTracking.follow(pin, region: r, revision: 4).startByte, 10 + "é\u{301}".utf8.count - 1)
    }

    /// With a bridge attached the companion is told the bridge's pin while it
    /// is valid and `null` otherwise — never the local anchor, whose id the
    /// bridge does not know. Without a bridge, the local anchor.
    func testAnnouncedDestinationIsNilWhileTheBridgePinIsInvalid() {
        let model = ShellModel()
        let local = InsertionAnchor(id: "local-1", path: "main.tex", byteOffset: 5, revision: 7, contextAfter: " Fl")
        let valid = Self.anchor(5, 5, revision: 7)
        // The announcement now also carries the caret context so the companion
        // can say how a capture landing there will be wrapped (nearby-v1,
        // additive). The pin/no-pin rule this test is about is unchanged.
        let announced = model.announcedNearbyDestination(bridgeAttached: true, bridgeAnchor: valid, localAnchor: local)
        XCTAssertEqual(announced?.destinationId, "d1")
        XCTAssertEqual(announced?.projectId, "p")
        XCTAssertEqual(announced?.path, "main.tex")
        XCTAssertEqual(announced?.baseRevision, 7)
        XCTAssertNil(model.announcedNearbyDestination(bridgeAttached: true, bridgeAnchor: Self.anchor(5, 5, valid: false), localAnchor: local),
                     "a dropped pin is announced as no destination")
        XCTAssertNil(model.announcedNearbyDestination(bridgeAttached: true, bridgeAnchor: nil, localAnchor: local),
                     "the bridge has nothing pinned: the local anchor's id would be refused")
        XCTAssertEqual(model.announcedNearbyDestination(bridgeAttached: false, bridgeAnchor: nil, localAnchor: local)?.destinationId, "local-1")
        XCTAssertEqual(model.announcedNearbyDestination(bridgeAttached: false, bridgeAnchor: nil, localAnchor: local)?.baseRevision, 7)
        XCTAssertNil(model.announcedNearbyDestination(bridgeAttached: false, bridgeAnchor: nil, localAnchor: nil))
    }

    /// End to end against the fake bridge: an edit before the pin shifts it
    /// (same id, still advertised), an edit after leaves it, an edit that
    /// overlaps it drops the advertised destination, tells the user, refuses a
    /// local submit without sending, and the (fake) bridge agrees when a
    /// companion sends anyway. Only a new pin restores an advertised
    /// destination — the shell never re-pins by itself.
    func testOverlappingEditDropsTheAdvertisedPinAndTellsTheUser() async throws {
        let model = ShellModel()
        model.autoCompile = false
        let store = try await ShellModelBridgeTests.attach(model, ledger: nil)
        defer { try? FileManager.default.removeItem(at: store); model.detachBridge() }
        let bridge = try XCTUnwrap(model.bridge)
        XCTAssertEqual(model.activeText, "Hello FlashTeX.\n")
        model.caretUTF16 = 5 // "Hello| FlashTeX.\n" → byte 5
        model.pinAnchorAtCaret()
        try await ShellModelBridgeTests.waitUntil { model.bridgeDestination != nil }
        let pinned = try XCTUnwrap(model.bridgeDestination)
        let advertised = try XCTUnwrap(model.nearbyDestination)
        XCTAssertEqual(advertised.destinationId, pinned.destinationId)
        XCTAssertEqual(pinned.startByte, 5)

        // Edit entirely before the pin: shifted, same id, same advertised destination.
        model.updateActiveText("Hi, hello FlashTeX.\n") // "Hello" → "Hi, hello" (+4 bytes)
        var rev = model.editorRevision
        try await ShellModelBridgeTests.waitUntil { bridge.shadow["main.tex"]?.revision == rev }
        var d = try XCTUnwrap(model.bridgeDestination)
        XCTAssertTrue(d.valid)
        XCTAssertEqual([d.startByte, d.endByte], [9, 9])
        XCTAssertEqual(d.currentRevision, rev)
        XCTAssertEqual(d.pinnedRevision, pinned.pinnedRevision, "the pin's own revision never moves")
        XCTAssertEqual(model.nearbyDestination, advertised, "an edit before the pin keeps the announced destination")
        XCTAssertFalse(model.captureNote?.contains("dropped") == true)

        // Edit entirely after: untouched bytes, revision follows.
        model.updateActiveText("Hi, hello FlashTeX!\n")
        rev = model.editorRevision
        try await ShellModelBridgeTests.waitUntil { bridge.shadow["main.tex"]?.revision == rev }
        d = try XCTUnwrap(model.bridgeDestination)
        XCTAssertTrue(d.valid); XCTAssertEqual([d.startByte, d.endByte], [9, 9]); XCTAssertEqual(d.currentRevision, rev)
        XCTAssertEqual(model.nearbyDestination, advertised)

        // The fake bridge still accepts a capture at the shifted pin (the mirror agrees with it).
        let image = try BridgeClientTests.fixtureCapture().image
        let ok = await model.submitCapture(image: image, captureId: "nearby-errors-shifted", instructions: "t")
        XCTAssertEqual(ok?.durable, true, model.captureNote ?? "")

        // Edit overlapping the pin (delete the space at byte 9): dropped, announced, told.
        model.updateActiveText("Hi, helloFlashTeX!\n")
        rev = model.editorRevision
        try await ShellModelBridgeTests.waitUntil { bridge.shadow["main.tex"]?.revision == rev }
        d = try XCTUnwrap(model.bridgeDestination, "the pin stays listed (as invalid), it is not silently removed")
        XCTAssertFalse(d.valid)
        XCTAssertEqual(d.destinationId, pinned.destinationId)
        XCTAssertEqual(d.currentRevision, rev)
        XCTAssertNil(model.nearbyDestination, "companions are told there is no destination")
        XCTAssertTrue(model.captureNote?.contains("dropped by an edit") == true, model.captureNote ?? "nil")
        XCTAssertTrue(bridge.log.contains { $0.contains("dropped") }, "\(bridge.log.suffix(3))")
        // Local submit is refused before anything is sent.
        let capturesBefore = model.bridgeCaptures.count
        let refused = await model.submitCapture(image: image, captureId: "nearby-errors-local-dropped", instructions: "t")
        XCTAssertNil(refused)
        XCTAssertTrue(model.captureNote?.contains("pin again") == true, model.captureNote ?? "nil")
        XCTAssertEqual(model.bridgeCaptures.count, capturesBefore, "nothing was sent")
        // A companion that skips the destination check: the bridge's refusal is the same code.
        let stale = RuntimeV1.CaptureSubmit(captureId: "nearby-errors-stale", destinationId: pinned.destinationId,
                                            baseRevision: pinned.pinnedRevision, image: image, instructions: "t")
        switch await model.forwardNearbyCapture(stale) {
        case .success: XCTFail("a capture at a dropped pin must be refused by the bridge")
        case .failure(let e): XCTAssertEqual(e.code, "destination_reselection_required", e.message)
        }
        // A further edit does not resurrect or move the dropped pin.
        model.updateActiveText("Hi, helloFlashTeX!!\n")
        rev = model.editorRevision
        try await ShellModelBridgeTests.waitUntil { bridge.shadow["main.tex"]?.revision == rev }
        XCTAssertEqual(model.bridgeDestination?.valid, false)
        XCTAssertNil(model.nearbyDestination)

        // Only the user's new pin restores an advertised destination (new id, current revision).
        model.caretUTF16 = 9
        model.pinAnchorAtCaret()
        try await ShellModelBridgeTests.waitUntil { model.bridgeDestination?.valid == true }
        let fresh = try XCTUnwrap(model.nearbyDestination)
        XCTAssertNotEqual(fresh.destinationId, advertised.destinationId)
        XCTAssertEqual(fresh.baseRevision, rev)
        let ok2 = await model.submitCapture(image: image, captureId: "nearby-errors-fresh", instructions: "t")
        XCTAssertEqual(ok2?.durable, true, model.captureNote ?? "")
    }

    // MARK: (3) prepare leaves the row in the state the bridge holds

    /// `proposal_missing` means the capture is received and still convertible:
    /// the row must not claim `.proposed`. `capture_rejected` is terminal.
    func testPrepareWithoutAProposalLeavesTheRowReceived() async throws {
        let model = ShellModel()
        model.autoCompile = false
        let store = try await ShellModelBridgeTests.attach(model, ledger: nil)
        defer { try? FileManager.default.removeItem(at: store); model.detachBridge() }
        let bridge = try XCTUnwrap(model.bridge)
        model.caretUTF16 = 5
        model.pinAnchorAtCaret()
        try await ShellModelBridgeTests.waitUntil { model.bridgeDestination != nil }
        let image = try BridgeClientTests.fixtureCapture().image
        let received = await model.submitCapture(image: image, captureId: "nearby-errors-prepare", instructions: "t")
        XCTAssertEqual(received?.durable, true, model.captureNote ?? "")
        XCTAssertEqual(model.bridgeCaptures.last?.state, .received)

        do { _ = try await bridge.prepare(captureId: "nearby-errors-prepare", expectedRevision: model.editorRevision); XCTFail() }
        catch let f as BridgeClient.Failure { XCTAssertEqual(f.code, "proposal_missing", f.text) }
        let row = try XCTUnwrap(model.bridgeCaptures.first { $0.captureId == "nearby-errors-prepare" })
        XCTAssertEqual(row.state, .received, "no proposal exists; the row must not say .proposed")
        XCTAssertTrue(row.note.contains("proposal_missing"), row.note)
        XCTAssertEqual(model.latestConvertibleCapture?.captureId, "nearby-errors-prepare", "still convertible")

        let rejected = await model.bridgeRejectAndWait(captureId: "nearby-errors-prepare")
        XCTAssertTrue(rejected)
        do { _ = try await bridge.prepare(captureId: "nearby-errors-prepare", expectedRevision: model.editorRevision); XCTFail() }
        catch let f as BridgeClient.Failure { XCTAssertEqual(f.code, "capture_rejected", f.text) }
        XCTAssertEqual(model.bridgeCaptures.first { $0.captureId == "nearby-errors-prepare" }?.state, .rejected)
        XCTAssertNil(model.latestConvertibleCapture)
    }

    // MARK: real bridge

    /// The real `flashtex-bridge` applies the same anchor rule: after an edit
    /// that overlaps the pin, the shell announces no destination and the
    /// bridge refuses a capture bound to the old id; `proposal_missing` leaves
    /// the row received.
    func testRealBridgeAgreesWithTheMirroredAnchorRuleAndPrepareState() async throws {
        guard let path = ProcessInfo.processInfo.environment["FLASHTEX_BRIDGE"], FileManager.default.isExecutableFile(atPath: path) else {
            throw XCTSkip("set FLASHTEX_BRIDGE to the built flashtex-bridge binary")
        }
        let store = try BridgeClientTests.tempStore()
        defer { try? FileManager.default.removeItem(at: store) }
        let model = ShellModel()
        model.autoCompile = false
        let attached = await model.attachBridgeAndWait(executable: URL(fileURLWithPath: path), storeDirectory: store, ledger: nil, discoverLedger: false)
        XCTAssertTrue(attached, model.captureNote ?? model.bridgeStatus)
        defer { model.detachBridge() }
        let bridge = try XCTUnwrap(model.bridge)
        model.caretUTF16 = 5
        model.pinAnchorAtCaret()
        try await ShellModelBridgeTests.waitUntil { model.bridgeDestination != nil }
        let pinned = try XCTUnwrap(model.bridgeDestination)
        let image = try BridgeClientTests.fixtureCapture().image

        // Before the pin: shifted and still accepted by the real bridge.
        model.updateActiveText("Hi, hello FlashTeX.\n")
        var rev = model.editorRevision
        try await ShellModelBridgeTests.waitUntil { bridge.shadow["main.tex"]?.revision == rev }
        XCTAssertEqual(model.bridgeDestination?.startByte, 9)
        XCTAssertEqual(model.nearbyDestination?.destinationId, pinned.destinationId)
        let ok = await model.submitCapture(image: image, captureId: "real-nearby-errors-shifted", instructions: "t")
        XCTAssertEqual(ok?.durable, true, model.captureNote ?? "")
        do { _ = try await bridge.prepare(captureId: "real-nearby-errors-shifted", expectedRevision: rev); XCTFail() }
        catch let f as BridgeClient.Failure { XCTAssertEqual(f.code, "proposal_missing", f.text) }
        XCTAssertEqual(model.bridgeCaptures.first { $0.captureId == "real-nearby-errors-shifted" }?.state, .received)

        // Overlapping the pin: dropped locally; the real bridge refuses the old id.
        model.updateActiveText("Hi, helloFlashTeX.\n")
        rev = model.editorRevision
        try await ShellModelBridgeTests.waitUntil { bridge.shadow["main.tex"]?.revision == rev }
        XCTAssertEqual(model.bridgeDestination?.valid, false)
        XCTAssertNil(model.nearbyDestination)
        XCTAssertTrue(model.captureNote?.contains("dropped by an edit") == true, model.captureNote ?? "nil")
        let stale = RuntimeV1.CaptureSubmit(captureId: "real-nearby-errors-stale", destinationId: pinned.destinationId,
                                            baseRevision: pinned.pinnedRevision, image: image, instructions: "t")
        switch await model.forwardNearbyCapture(stale) {
        case .success: XCTFail("refused by the real bridge")
        case .failure(let e): XCTAssertEqual(e.code, "destination_reselection_required", e.message)
        }
        XCTAssertFalse(FileManager.default.fileExists(atPath: store.appendingPathComponent("real-nearby-errors-stale.json").path), "refused before the journal")
        // Fresh pin: accepted again.
        model.caretUTF16 = 9
        model.pinAnchorAtCaret()
        try await ShellModelBridgeTests.waitUntil { model.bridgeDestination?.valid == true }
        XCTAssertEqual(model.nearbyDestination?.baseRevision, rev)
        let ok2 = await model.submitCapture(image: image, captureId: "real-nearby-errors-fresh", instructions: "t")
        XCTAssertEqual(ok2?.durable, true, model.captureNote ?? "")
    }
}
