import Network
import XCTest
import FlashTeXProtocol
import NearbyClient
@testable import FlashTeXMac

/// The owner's capture flow against the REAL helpers (lane lane-capture-flow):
/// the iPad connects, the owner keeps typing on the Mac, sends a capture, and
/// it lands at the caret — no `destination_reselection_required`, no pin.
/// Conversion runs through the real bridge's `openai-compatible` provider
/// against a loopback HTTP stub started here (the bridge is launched with
/// `FLASHTEX_CONVERSION_BASE_URL=http://127.0.0.1:<port>/v1`; nothing leaves
/// the machine), so `capture_prepare_insert` and the durable apply through
/// the real edit ledger are exercised, not faked. Skipped unless
/// `FLASHTEX_BRIDGE` and `FLASHTEX_EDIT_LEDGER` point at built helpers.
///
/// Companion to `CaptureAcceptanceTests` (which pins the explicit-pin contract
/// and switches the caret destination off); here it is on, as in the app.
@MainActor
final class CaptureFlowAcceptanceTests: XCTestCase {
    override func setUp() { CaptureInboxFeature.caretDestinationOverride = true }
    override func tearDown() { CaptureInboxFeature.caretDestinationOverride = nil }

    static let psk = Data(repeating: 0x5A, count: 32)
    static let pairId = "captureflowpair1"
    static var pair: PairedMac {
        PairedMac(fingerprint: "test-fp", macName: "Test Mac", pairId: pairId, pairPsk: psk.base64EncodedString(), companionName: "Flow iPad")
    }
    static var entry: NearbyListener.PSKEntry { .init(identity: pairId, key: psk, isBootstrap: false) }
    static let fixturePNG: Data = {
        let env = try! RuntimeV1.decodeCaptureSubmit(Data(contentsOf: NearbyListenerTests.fixtureURL))
        return Data(base64Encoded: env.payload.image.dataBase64)!
    }()
    static let demoText = "Hello FlashTeX.\n"

    private static func env(_ name: String) -> URL? {
        guard let p = ProcessInfo.processInfo.environment[name], FileManager.default.isExecutableFile(atPath: p) else { return nil }
        return URL(fileURLWithPath: p)
    }
    private func requireHelpers() throws -> (bridge: URL, ledger: ShellModel.LedgerLaunch) {
        guard let b = Self.env("FLASHTEX_BRIDGE"), let l = Self.env("FLASHTEX_EDIT_LEDGER") else {
            throw XCTSkip("set FLASHTEX_BRIDGE and FLASHTEX_EDIT_LEDGER to the built helpers")
        }
        return (b, .init(executable: l))
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 20, file: StaticString = #filePath, line: UInt = #line,
                           _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 25_000_000)
        }
        XCTFail("timed out after \(timeout)s waiting for \(what)", file: file, line: line)
    }

    /// Attaches the real bridge with the openai-compatible provider pointed at `stub`
    /// (no key: loopback), plus the real edit ledger, on `store`.
    private func attach(_ model: ShellModel, store: URL, stub: LoopbackProviderStub) async throws {
        let (bridge, ledger) = try requireHelpers()
        var environment: [String: String] = ["FLASHTEX_CONVERSION_BASE_URL": stub.base, "FLASHTEX_CONVERSION_MODEL": "loopback-fixture",
                                             "NO_PROXY": "127.0.0.1,localhost"]
        if let path = ProcessInfo.processInfo.environment["PATH"] { environment["PATH"] = path }
        let ok = await model.attachBridgeAndWait(executable: bridge, arguments: ["--conversion-provider", "openai-compatible"],
                                                 storeDirectory: store, provider: .none, environment: environment,
                                                 ledger: ledger, discoverLedger: false)
        XCTAssertTrue(ok, model.captureNote ?? model.bridgeStatus)
        XCTAssertTrue(model.bridgeAttached)
        XCTAssertTrue(model.bridge?.ledgerUsable == true, model.bridge?.ledgerStatus ?? "")
    }

    private func journalFiles(in store: URL, captureId: String) -> [String] {
        ((try? FileManager.default.contentsOfDirectory(atPath: store.path)) ?? []).filter { $0.hasPrefix(captureId) && $0.hasSuffix(".json") }
    }

    private func reconnector(port: UInt16) -> NearbyReconnector {
        NearbyReconnector(pair: Self.pair, policy: .immediate,
                          endpoints: { .hostPort(host: "127.0.0.1", port: NWEndpoint.Port(rawValue: port)!) })
    }

    /// Simulates the editor adopting the staged edit, then waits for the bridge's confirmation.
    private func adopt(_ model: ShellModel, captureId: String, expectedRange: NSRange,
                       file: StaticString = #filePath, line: UInt = #line) async throws -> String {
        let pending = try XCTUnwrap(model.pendingEdit, file: file, line: line)
        XCTAssertEqual(pending.nsRange, expectedRange, file: file, line: line)
        let text = model.activeText as NSString
        let after = text.replacingCharacters(in: pending.nsRange, with: pending.text)
        model.editApplied(pending, newText: after)
        try await waitUntil("insertion confirmed", file: file, line: line) {
            model.bridgeCaptures.first { $0.captureId == captureId }?.state == .confirmed
        }
        return after
    }

    // MARK: the owner's scenario

    /// iPad connects (hello_ack names the caret, `mac-caret-1`) → the owner keeps
    /// typing at the caret → the iPad sends the capture still naming the id and
    /// revision it read at hello → accepted and journaled once (the Mac re-binds
    /// the automatic destination at the caret; the real bridge's caret anchor
    /// ignores the stale revision) → converted by the real bridge → the owner
    /// types more and moves the caret → Insert lands exactly at the new caret,
    /// through the real ledger, with the wrap journaled. Never a refusal.
    func testTypingAfterTheIPadConnectedStillInsertsAtTheCaret() async throws {
        let stub = try LoopbackProviderStub(latex: "x = 2y + 1")
        defer { stub.stop() }
        let store = try BridgeClientTests.tempStore()
        defer { try? FileManager.default.removeItem(at: store) }
        let model = ShellModel()
        model.autoCompile = false
        XCTAssertEqual(model.activeText, Self.demoText)
        try await attach(model, store: store, stub: stub)
        model.caretUTF16 = 5 // "Hello| FlashTeX."

        let h = ListenerHarness(psks: [Self.entry], sink: model, destinations: model)
        try h.start()
        defer { h.stop() }
        let r = reconnector(port: h.port)
        let session = try await r.connect()
        let atHello = try XCTUnwrap(session.destination, "hello_ack names the caret without any pin")
        XCTAssertEqual(atHello.destinationId, "mac-caret-1")
        XCTAssertEqual(model.bridgeDestination?.mode, .caret)
        XCTAssertEqual(model.bridgeDestination?.startByte, 5)

        // The owner keeps typing exactly where the caret (and the pin) is.
        model.updateActiveText("Hello!! FlashTeX.\n")
        model.caretUTF16 = 7
        let typedRevision = model.editorRevision
        try await waitUntil("document_edit sent") { model.bridge?.shadow["main.tex"]?.revision == typedRevision }
        XCTAssertEqual(model.bridgeDestination?.valid, true, "typing at the caret never drops the automatic destination")
        XCTAssertEqual(model.bridgeDestination?.startByte, 7, "the mirror followed the caret")
        let requeried = try await session.connection.destinationQuery()
        XCTAssertEqual(requeried?.destinationId, "mac-caret-1", "same handle after typing")
        XCTAssertEqual(requeried?.baseRevision, typedRevision)

        // The capture as the iPad built it from hello_ack: stale revision, same id.
        let capture = try session.makeCapture(captureId: "flow-typed-1", image: Self.fixturePNG, mimeType: "image/png",
                                              instructions: "the equation", destination: atHello)
        let ack = try await r.submit(capture, requireCurrentDestination: false)
        XCTAssertEqual(ack.captureId, "flow-typed-1")
        XCTAssertTrue(ack.durable, "journaled by the real bridge, not refused")
        XCTAssertEqual(journalFiles(in: store, captureId: "flow-typed-1").count, 1)
        XCTAssertEqual(model.captureInbox.item("flow-typed-1")?.autoPinned, true)
        XCTAssertFalse(h.snapshot.contains { if case .captureRefused = $0 { return true }; return false }, "\(h.snapshot)")

        // Real conversion through the loopback provider: the bare formula the
        // provider returned is delimited for the caret it was received at (mid-line).
        let converted = await model.convertCapture(captureId: "flow-typed-1")
        let proposal = try XCTUnwrap(converted, model.captureNote ?? "")
        XCTAssertEqual(proposal.latex, "$x = 2y + 1$")
        XCTAssertEqual(stub.requests, 1)

        // The owner types more and moves the caret to a new blank line before approving.
        model.updateActiveText("Hello!! FlashTeX.\n\n")
        model.caretUTF16 = 19
        let approvalRevision = model.editorRevision
        try await waitUntil("second document_edit sent") { model.bridge?.shadow["main.tex"]?.revision == approvalRevision }
        let item = try XCTUnwrap(model.captureInbox.item("flow-typed-1"))
        let preview = try XCTUnwrap(model.captureInsertionPreview(captureId: item.id, latex: proposal.latex))
        XCTAssertEqual(preview.decision.wrapping?.kind, .asIs, "already delimited: never wrapped twice")
        XCTAssertEqual(preview.text, "$x = 2y + 1$")

        let outcome = await model.insertCaptureFromInbox(item, latex: proposal.latex)
        XCTAssertEqual(outcome, .inserted(byteOffset: 19), model.captureNote ?? "")
        let after = try await adopt(model, captureId: "flow-typed-1", expectedRange: NSRange(location: 19, length: 0))
        XCTAssertEqual(after, "Hello!! FlashTeX.\n\n$x = 2y + 1$")
        XCTAssertEqual(model.bridge?.durable?.text, after, "durable through the real edit ledger")
        let status = try await model.bridge!.status(captureId: "flow-typed-1")
        XCTAssertEqual(status.prepared?.startByte, 19)
        XCTAssertEqual(status.prepared?.wrap?.kind, "as_is", "the wrap is journaled with the edit")
        XCTAssertNotNil(status.applied)
        XCTAssertEqual(journalFiles(in: store, captureId: "flow-typed-1").count, 1)
        print("measured: typed-after-connect capture inserted at byte 19 via real bridge+ledger; wrap kind=\(status.prepared?.wrap?.kind ?? "-")")
        await r.shutdown()
        model.detachBridge()
    }

    // MARK: bridge restart

    /// The app relaunches between receipt and insert: the bridge forgot every
    /// anchor, the companion still holds `mac-caret-1`. A resend naming that id
    /// is accepted (identical receipt, one journal entry) because the Mac
    /// re-pins the id at its caret; Insert then lands at the new Mac's caret,
    /// and a block proposal (TikZ) is put on its own lines by the journaled
    /// wrap — through the real edit ledger, which must accept the additive field.
    func testBridgeRestartAcceptsTheCompanionsOldDestinationIdAndWrapsAtTheCaret() async throws {
        let tikz = "\\begin{tikzpicture}\\draw (0,0) -- (1,1);\\end{tikzpicture}"
        let stub = try LoopbackProviderStub(latex: tikz)
        defer { stub.stop() }
        let store = try BridgeClientTests.tempStore()
        defer { try? FileManager.default.removeItem(at: store) }

        let model1 = ShellModel()
        model1.autoCompile = false
        try await attach(model1, store: store, stub: stub)
        model1.caretUTF16 = 15
        let announced1 = await model1.nearbyDestinationPinningCaretIfNeeded()
        let d1 = try XCTUnwrap(announced1)
        XCTAssertEqual(d1.destinationId, "mac-caret-1")
        let capture = RuntimeV1.CaptureSubmit(captureId: "flow-restart-1", destinationId: d1.destinationId, baseRevision: d1.baseRevision,
                                              image: .init(mimeType: "image/png", dataBase64: Self.fixturePNG.base64EncodedString()),
                                              instructions: "the sketch")
        guard case .success(let ack1) = await model1.forwardNearbyCapture(capture, pairId: Self.pairId) else { return XCTFail() }
        XCTAssertTrue(ack1.durable)
        let converted = await model1.convertCapture(captureId: "flow-restart-1")
        let proposal = try XCTUnwrap(converted, model1.captureNote ?? "")
        XCTAssertEqual(proposal.latex, tikz)
        XCTAssertNil(model1.pendingEdit)

        // "Quit": the helpers exit, the journal stays.
        model1.detachBridge()
        try await waitUntil("bridge process gone") { RealHelperProcess.pid(commandLineContaining: "flashtex-bridge --store \(store.path)") == nil }

        // "Relaunch": a fresh shell on the same store; the caret is elsewhere now.
        let model2 = ShellModel()
        model2.autoCompile = false
        try await attach(model2, store: store, stub: stub)
        XCTAssertNil(model2.nearbyDestination, "anchors are in-memory on the bridge")
        model2.caretUTF16 = 5 // "Hello| FlashTeX."
        guard case .success(let ack2) = await model2.forwardNearbyCapture(capture, pairId: Self.pairId) else {
            return XCTFail("a resend naming the old automatic id must be accepted: \(model2.captureNote ?? "")")
        }
        XCTAssertEqual(ack2.captureId, ack1.captureId)
        XCTAssertTrue(ack2.durable)
        XCTAssertTrue(ack2.hasProposal, "the journal still holds the proposal")
        XCTAssertEqual(journalFiles(in: store, captureId: "flow-restart-1").count, 1)
        XCTAssertEqual(model2.bridgeDestination?.destinationId, "mac-caret-1")
        XCTAssertEqual(model2.bridgeDestination?.mode, .caret)
        XCTAssertEqual(model2.bridgeDestination?.startByte, 5)
        XCTAssertEqual(stub.requests, 1, "no second paid conversion")

        // The relaunched shell has no proposal row yet: capture_status supplies it for the review.
        let row = try await model2.bridge!.status(captureId: "flow-restart-1")
        let reviewed = RuntimeV1.CaptureProposal(captureId: "flow-restart-1", latex: try XCTUnwrap(row.proposal?.latex),
                                                 ambiguities: row.proposal?.ambiguities ?? [], requiredDependencies: row.proposal?.requiredDependencies ?? [])
        model2.enqueue(reviewed)
        let preview = try XCTUnwrap(model2.captureInsertionPreview(captureId: "flow-restart-1", latex: reviewed.latex))
        XCTAssertEqual(preview.decision, .wrap(.init(kind: .asIs, prefix: "\n", suffix: "\n")), "a block mid-line gets its own lines")
        let outcome = await model2.approveBridgeProposal(reviewed, latex: reviewed.latex)
        XCTAssertEqual(outcome, .inserted(byteOffset: 5), model2.captureNote ?? "")
        let after = try await adopt(model2, captureId: "flow-restart-1", expectedRange: NSRange(location: 5, length: 0))
        XCTAssertEqual(after, "Hello\n\(tikz)\n FlashTeX.\n")
        XCTAssertEqual(model2.bridge?.durable?.text, after, "the real edit ledger accepted the edit with its wrap")
        let status = try await model2.bridge!.status(captureId: "flow-restart-1")
        XCTAssertEqual(status.prepared?.wrap, .init(prefix: "\n", suffix: "\n", kind: "as_is"))
        XCTAssertNotNil(status.applied)
        model2.detachBridge()
    }

    // MARK: explicit pin: still strict, but with a way out

    /// An explicit ⌘⌥P pin keeps today's contract: an edit that overlaps it
    /// drops it and Insert is refused — with a sentence that says what to do,
    /// and the inspector's "Insert at caret" recovery, which re-binds the
    /// capture at the caret and inserts it there.
    func testAnExplicitPinOverlappedByAnEditIsRefusedWithAnActionableNoteAndInsertAtCaretRecovers() async throws {
        let stub = try LoopbackProviderStub(latex: "$x^{2}$")
        defer { stub.stop() }
        let store = try BridgeClientTests.tempStore()
        defer { try? FileManager.default.removeItem(at: store) }
        let model = ShellModel()
        model.autoCompile = false
        try await attach(model, store: store, stub: stub)
        model.caretUTF16 = 5
        model.pinAnchorAtCaret()
        try await waitUntil("bridge pin") { model.bridgeDestination?.valid == true }
        let pinned = try XCTUnwrap(model.nearbyDestination)
        XCTAssertEqual(pinned.destinationId, "mac-anchor-1")
        XCTAssertFalse(model.captureDestinationIsAutomatic)
        let announced = await model.nearbyDestinationPinningCaretIfNeeded()
        XCTAssertEqual(announced?.destinationId, "mac-anchor-1", "an explicit pin is announced as-is")

        let capture = RuntimeV1.CaptureSubmit(captureId: "flow-pinned-1", destinationId: pinned.destinationId, baseRevision: pinned.baseRevision,
                                              image: .init(mimeType: "image/png", dataBase64: Self.fixturePNG.base64EncodedString()),
                                              instructions: "the square")
        guard case .success = await model.forwardNearbyCapture(capture, pairId: Self.pairId) else { return XCTFail(model.captureNote ?? "") }
        let converted = await model.convertCapture(captureId: "flow-pinned-1")
        let proposal = try XCTUnwrap(converted, model.captureNote ?? "")

        // The user deletes the space the pin sits on: the explicit pin is gone.
        model.updateActiveText("HelloFlashTeX.\n")
        let edited = model.editorRevision
        try await waitUntil("document_edit sent") { model.bridge?.shadow["main.tex"]?.revision == edited }
        XCTAssertEqual(model.bridgeDestination?.valid, false)
        let item = try XCTUnwrap(model.captureInbox.item("flow-pinned-1"))
        model.caretUTF16 = 14 // end of "HelloFlashTeX."
        let refused = await model.insertCaptureFromInbox(item, latex: proposal.latex)
        guard case .needsReselection = refused else { return XCTFail("\(refused)") }
        let note = try XCTUnwrap(model.captureNote)
        XCTAssertTrue(note.contains("destination_reselection_required"), note)
        XCTAssertTrue(note.contains("Insert at caret"), "the note says what to do: \(note)")
        XCTAssertNil(model.pendingEdit)
        XCTAssertEqual(model.activeText, "HelloFlashTeX.\n")

        // One click: the capture is re-bound at the caret and inserted there.
        let recovered = await model.recoverCaptureAtCaret(item)
        XCTAssertEqual(recovered, .inserted(byteOffset: 14), model.captureNote ?? "")
        XCTAssertEqual(model.bridgeDestination?.mode, .caret, "the dead pin's id was taken over as a caret anchor")
        let after = try await adopt(model, captureId: "flow-pinned-1", expectedRange: NSRange(location: 14, length: 0))
        XCTAssertEqual(after, "HelloFlashTeX.$x^{2}$\n")
        XCTAssertEqual(model.bridge?.durable?.text, after)
        model.detachBridge()
    }
}

/// A one-connection-at-a-time HTTP/1.1 server on 127.0.0.1 that answers every
/// `POST …/chat/completions` with one fixed proposal in the OpenAI-compatible
/// shape the bridge parses (`crates/bridge/src/openai.rs`), like the Rust
/// `Stub` in `crates/bridge/tests/conversion_provider.rs`.
final class LoopbackProviderStub: @unchecked Sendable {
    let listener: NWListener
    private let reply: Data
    private let lock = NSLock()
    private var count = 0
    private(set) var base = ""
    var requests: Int { lock.withLock { count } }

    init(latex: String) throws {
        let content = try JSONSerialization.data(withJSONObject: ["latex": latex, "ambiguities": [], "required_dependencies": []])
        let body: [String: Any] = [
            "id": "chatcmpl-loopback-0001", "object": "chat.completion", "model": "loopback-fixture",
            "choices": [["index": 0, "finish_reason": "stop",
                         "message": ["role": "assistant", "content": String(decoding: content, as: UTF8.self)]]],
            "usage": ["prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2],
        ]
        reply = try JSONSerialization.data(withJSONObject: body)
        let parameters = NWParameters.tcp
        parameters.requiredInterfaceType = .loopback
        listener = try NWListener(using: parameters, on: .any)
        let ready = DispatchSemaphore(value: 0)
        listener.stateUpdateHandler = { [weak self] state in
            if case .ready = state, let port = self?.listener.port?.rawValue {
                self?.base = "http://127.0.0.1:\(port)/v1"
                ready.signal()
            }
            if case .failed = state { ready.signal() }
        }
        listener.newConnectionHandler = { [weak self] connection in self?.serve(connection) }
        listener.start(queue: DispatchQueue(label: "loopback-provider-stub"))
        guard ready.wait(timeout: .now() + 5) == .success, !base.isEmpty else {
            throw XCTSkip("loopback provider stub could not listen")
        }
    }

    func stop() { listener.cancel() }

    private func serve(_ connection: NWConnection) {
        connection.start(queue: DispatchQueue(label: "loopback-provider-stub.connection"))
        var buffer = Data()
        func read() {
            connection.receive(minimumIncompleteLength: 1, maximumLength: 1 << 20) { [weak self] data, _, isComplete, error in
                guard let self, error == nil, let data else { connection.cancel(); return }
                buffer.append(data)
                if let split = buffer.range(of: Data("\r\n\r\n".utf8)) {
                    let head = String(decoding: buffer[..<split.lowerBound], as: UTF8.self)
                    let length = head.split(separator: "\r\n")
                        .first { $0.lowercased().hasPrefix("content-length:") }
                        .flatMap { Int($0.split(separator: ":", maxSplits: 1)[1].trimmingCharacters(in: .whitespaces)) } ?? 0
                    if buffer.count - split.upperBound >= length {
                        self.lock.withLock { self.count += 1 }
                        let response = Data("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: \(self.reply.count)\r\nConnection: close\r\n\r\n".utf8) + self.reply
                        connection.send(content: response, completion: .contentProcessed { _ in connection.cancel() })
                        return
                    }
                }
                if isComplete { connection.cancel() } else { read() }
            }
        }
        read()
    }
}
