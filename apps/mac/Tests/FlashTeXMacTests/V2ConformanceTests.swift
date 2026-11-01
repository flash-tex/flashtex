import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Consumer-side conformance to the `display-list-v2` draft contract
/// (docs/contracts/runtime-v1-display-list-v2.md at main ad922ea), items
/// D1/D2/D3/D6/D9 of docs/proposals/contract-draft-review-ad922ea.md.
/// Real-helper tests skip cleanly without `FLASHTEX_PREVIEW_CONTROLLER` and
/// `FLASHTEX_COMPILER`.
@MainActor
final class V2ConformanceTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let helper = ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) }
    static let compiler = ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"].map { URL(fileURLWithPath: $0) }

    private func waitUntil(timeout: TimeInterval = 20, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout after \(timeout)s (load-sensitive)") }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    // MARK: D1 — the direct route binds every declared document to the request text BEFORE paint

    func testDirectRouteRefusesASiblingWhoseDocumentsAreNotTheRequestTextAndKeepsTheLastFrame() async throws {
        let tex = try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.tex"), encoding: .utf8)
        let python = URL(fileURLWithPath: "/usr/bin/python3")
        guard FileManager.default.isExecutableFile(atPath: python.path) else { throw XCTSkip("python3 missing") }
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: tex)
        model.attachWorker(at: python, arguments: [Self.fixtures.appendingPathComponent("fake_worker_v2.py").path,
                                                   Self.fixtures.appendingPathComponent("display-list-v2-text.json").path])
        model.previewV2 = true
        model.setLiveV2(true)
        model.compile()
        try await waitUntil { model.inFlightRevision == nil && model.displayListV2?.frame != nil && model.displayListV2?.isLoading == false }
        guard case .loaded(let first, let firstSource)? = model.displayListV2, case .worker(let id, _, _, let line) = firstSource else { return XCTFail() }
        XCTAssertEqual(id, model.resultID)
        var envelope = try JSONSerialization.jsonObject(with: line) as! [String: Any]
        var payload = envelope["payload"] as! [String: Any]
        var docs = payload["documents"] as! [[String: Any]]
        XCTAssertEqual(docs[0]["sha256"] as? String, SourceDigest.sha256Hex(tex), "the accepted sibling attests the request text")

        func deliver(_ mutate: (inout [String: Any]) -> Void, file: StaticString = #filePath, fileLine: UInt = #line) async throws -> RenderingV2.ValidationError? {
            var doc = docs[0]; mutate(&doc); var d = docs; d[0] = doc
            payload["documents"] = d; envelope["payload"] = payload
            let done = expectation(description: "delivered")
            let refused = V2Live.sourceMismatchesRefused, accepted = V2Live.linesAccepted
            var seen: RenderingV2.ValidationError?
            let last = V2Live.lastLiveRefusal
            model.receiveDisplayListV2(id: id, line: try JSONSerialization.data(withJSONObject: envelope)) {
                if let now = V2Live.lastLiveRefusal, now != last { seen = now }
                done.fulfill()
            }
            await fulfillment(of: [done], timeout: 20)
            XCTAssertEqual(V2Live.linesAccepted, accepted + 1, "the line was for the applied result", file: file, line: fileLine)
            if seen?.code == "source_mismatch" { XCTAssertEqual(V2Live.sourceMismatchesRefused, refused + 1, file: file, line: fileLine) }
            return seen
        }
        // Same length, different bytes: refused by the raw SHA-256 before paint; the verified frame stays.
        let sha = docs[0]["sha256"] as! String
        let bySha = try await deliver { $0["sha256"] = String(sha.reversed()) }
        XCTAssertEqual(bySha?.code, "source_mismatch")
        XCTAssertTrue(bySha?.message.contains("main.tex sha256 \(String(sha.reversed()).prefix(12))… differs from the requested text") == true, bySha?.message ?? "nil")
        XCTAssertEqual(model.displayListV2?.frame?.preparedNonce, first.preparedNonce, "the last verified frame is kept")
        XCTAssertFalse(model.displayListV2?.isLoading == true)
        XCTAssertTrue(model.workerStatus.hasPrefix("display_list refused: [source_mismatch]"), model.workerStatus)
        // Wrong byte_length: refused before hashing.
        let byLength = try await deliver { $0["byte_length"] = (docs[0]["byte_length"] as! Int) + 1 }
        XCTAssertEqual(byLength?.code, "source_mismatch")
        XCTAssertTrue(byLength?.message.contains("byte_length \(tex.utf8.count + 1) differs from the requested text (\(tex.utf8.count) bytes)") == true, byLength?.message ?? "nil")
        XCTAssertEqual(model.displayListV2?.frame?.preparedNonce, first.preparedNonce)
        // A document the request did not carry: the structural validator refuses first (the run's provenance
        // names main.tex, which is no longer declared); the frame is kept either way.
        let byPath = try await deliver { $0["path"] = "other.tex" }
        XCTAssertEqual(byPath?.code, "invalid_display_list")
        XCTAssertEqual(model.displayListV2?.frame?.preparedNonce, first.preparedNonce)
        // …and the binding itself refuses an undeclared-by-request document when the structure is fine.
        XCTAssertEqual(V2Live.sourceBindingFailure(of: first.list, requestID: id, compiled: [:])?.message,
                       "display_list \(id) declares main.tex, which the compile request did not carry")
        XCTAssertEqual(V2Live.sourceBindingFailure(of: first.list, requestID: id, compiled: ["main.tex": tex]), nil)
        // No documents at all: refused.
        payload["documents"] = [[String: Any]](); envelope["payload"] = payload
        let none = expectation(description: "none")
        model.receiveDisplayListV2(id: id, line: try JSONSerialization.data(withJSONObject: envelope)) { none.fulfill() }
        await fulfillment(of: [none], timeout: 20)
        XCTAssertTrue(model.captureNote?.contains("Display list refused") == true, model.captureNote ?? "")
        XCTAssertEqual(model.displayListV2?.frame?.preparedNonce, first.preparedNonce)
        // The untouched sibling for the same result still verifies (a new frame instance replaces the kept one).
        let ok = try await deliver { _ in }
        XCTAssertNil(ok)
        XCTAssertNotEqual(model.displayListV2?.frame?.preparedNonce, first.preparedNonce)
        XCTAssertEqual(model.displayListV2?.frame?.list.revision, model.result?.revision)
        // The binding is to the REQUEST text, not the live buffer: typing after the compile does not refuse the sibling
        // (the v1 preview is shown for the same request text), while a sibling for the typed text is not the request's.
        model.updateActiveText(tex + "% typed\n")
        let stillRequestText = try await deliver { _ in }
        XCTAssertNil(stillRequestText, "typing after the compile does not invalidate the request's own sibling")
        model.detachWorker()
    }

    // MARK: D2 — the helper candidate's membership_generation must be the project's current one

    private func candidate(generation: Int, session: String = "s1", request: String = "pc-7", project: String = "demo",
                           compileRevision: Int = 3, versions: [String: Int] = ["main.tex": 2], list: Data? = nil) throws -> DisplayCandidateFrame {
        let bytes = try list ?? Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.json"))
        return DisplayCandidateFrame(sessionID: session, requestID: request, projectID: project, compileRevision: compileRevision,
                                     sourceVersions: versions, membershipGeneration: generation, displayList: bytes,
                                     frameBytes: bytes.count, receivedNs: MonotonicClock.nowNs())
    }

    func testGateRefusesACandidateWhoseMembershipGenerationIsNotTheProjectsCurrentOne() throws {
        let applied = DisplayCandidateAppliedPreview(requestID: "pc-7", compileRevision: 3, sourceVersions: ["main.tex": 2], editorRevision: 5)
        var gate = DisplayCandidateGate(negotiated: true, sessionID: "s1", projectID: "demo", applied: applied, appliedResultID: "pc-7",
                                        activePath: "main.tex", displayedEditorRevision: nil)
        // No generation learned yet (before the `snapshot` sent on ready): the comparison is not skipped,
        // the candidate is refused with the typed `membership_unknown` reason (first-generation gate).
        XCTAssertNil(gate.membershipGeneration)
        let unknown = try XCTUnwrap(gate.rejection(of: try candidate(generation: 1)))
        XCTAssertTrue(unknown.hasPrefix(DisplayCandidates.membershipUnknown + ":"), unknown)
        XCTAssertTrue(unknown.contains("candidate names generation 1"), unknown)
        // The project is at generation 4 (an open/detach happened): a candidate compiled at generation 3 is stale.
        gate.membershipGeneration = 4
        XCTAssertEqual(gate.rejection(of: try candidate(generation: 3)), "membership generation 3 is older than the project's learned generation 4")
        XCTAssertNil(gate.rejection(of: try candidate(generation: 4)))
        // The helper's generation is the index generation and advances on every durable edit the shell makes after
        // the learn (edit replies do not carry it): the learned value is a floor, a newer generation is not stale.
        XCTAssertNil(gate.rejection(of: try candidate(generation: 5)))
        // The generation check sits with the identity checks, before the applied-preview checks.
        gate.applied = nil
        XCTAssertTrue(gate.rejection(of: try candidate(generation: 3))!.hasPrefix("membership generation 3"))
        // The model's gate carries the project's generation into admission (nil until learned).
        let model = ShellModel()
        XCTAssertNil(model.displayCandidates.gate(activePath: "main.tex", appliedResultID: nil, membershipGeneration: model.project.membershipGeneration).membershipGeneration)
        XCTAssertEqual(model.displayCandidates.gate(activePath: "main.tex", appliedResultID: nil, membershipGeneration: 9).membershipGeneration, 9)
    }

    /// Real helper + real producer: a candidate that names the generation from
    /// before an `open_document` is refused at admission; the previously
    /// verified frame and the v1 preview stay.
    func testRealHelperRefusesACandidateFromBeforeAnOpenDocument() async throws {
        guard let helper = Self.helper, FileManager.default.isExecutableFile(atPath: helper.path),
              let render = ProcessInfo.processInfo.environment["FLASHTEX_RENDER"].map({ URL(fileURLWithPath: $0) }),
              FileManager.default.isExecutableFile(atPath: render.path) else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_RENDER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("v2conf-d2-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root); unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        let tex = root.appendingPathComponent("project/main.tex")
        try "\\documentclass{article}\n\\begin{document}\nHello generations, fi.\n\\end{document}\n".write(to: tex, atomically: true, encoding: .utf8)
        try "Appendix via helper.\n".write(to: root.appendingPathComponent("project/appendix.tex"), atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Fonts")
        if ProcessInfo.processInfo.environment["FLASHTEX_LM_DIR"] == nil { setenv("FLASHTEX_LM_DIR", fontsDir.path, 1) }
        let previousCompiler = ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"]
        setenv("FLASHTEX_COMPILER", render.path, 1) // attachController hands the helper flashtex-render as its producer
        defer { if let previousCompiler { setenv("FLASHTEX_COMPILER", previousCompiler, 1) } else { unsetenv("FLASHTEX_COMPILER") } }
        setenv("FLASHTEX_DISPLAY_CANDIDATES", "1", 1)
        defer { unsetenv("FLASHTEX_DISPLAY_CANDIDATES") }

        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        model.attachController(at: helper)
        try await waitUntil { model.displayCandidatesNegotiated }
        try await waitUntil(timeout: 40) { if case .loaded? = model.displayListV2 { return true } else { return false } }
        guard case .loaded(let first, let source)? = model.displayListV2, case .worker(_, _, _, let line) = source else { return XCTFail() }
        // The shell learns the generation the candidate was compiled at.
        let snapshot = await model.project.refreshSnapshot()
        let before = try XCTUnwrap(snapshot).generation
        // An open_document between the compile and the candidate: the generation advances.
        let opened = await model.project.openDocument("appendix.tex")
        XCTAssertEqual(opened, .opened(path: "appendix.tex"))
        let after = try XCTUnwrap(model.project.membershipGeneration)
        XCTAssertGreaterThan(after, before)
        let applied = try XCTUnwrap(model.displayCandidates.applied)
        let session = model.controller!.config.sessionID, projectID = model.controller!.config.projectID
        // A candidate for the applied preview that still names the OLD generation: refused at admission.
        let stale = try candidate(generation: before, session: session, request: applied.requestID, project: projectID,
                                  compileRevision: applied.compileRevision, versions: applied.sourceVersions, list: line)
        let refused = model.displayCandidates.refused, published = model.displayCandidates.published
        model.handleDisplayCandidate(stale)
        XCTAssertEqual(model.displayCandidates.refused, refused + 1)
        XCTAssertEqual(model.displayCandidates.lastRefusal, "membership generation \(before) is older than the project's learned generation \(after)")
        XCTAssertTrue(model.displayCandidates.status.contains("refused"), model.displayCandidates.status)
        XCTAssertEqual(model.displayCandidates.published, published)
        XCTAssertEqual(model.displayListV2?.frame?.preparedNonce, first.preparedNonce, "the previously verified frame stays")
        XCTAssertNotNil(model.result, "v1 untouched")
        model.detachController()
    }

    // MARK: D9 — the direct route with a runtime-v1 fixture on screen

    static let protocolFixtures = fixtures.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("protocol/fixtures")

    /// Review row D9 read `receiveDisplayListV2` as dropping the sibling while a
    /// fixture is shown. Measured behaviour on the direct route: a fixture is
    /// replaced by the worker's compile_result BEFORE its sibling arrives, so the
    /// sibling is applied; only a line whose id is the FIXTURE's own applied id
    /// (no worker result for it) is dropped, counted, and the fixture stays.
    func testFixtureOnScreenDoesNotLoseTheNextLiveFrameAndAFixtureIdLineIsDropped() async throws {
        let tex = try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.tex"), encoding: .utf8)
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: tex)
        let fixture = Self.protocolFixtures.appendingPathComponent("compile-result.json")
        guard FileManager.default.fileExists(atPath: fixture.path) else { throw XCTSkip("protocol/fixtures/compile-result.json missing") }
        model.loadFixtures(request: nil, result: fixture)
        XCTAssertEqual(model.previewSource, .fixture)
        XCTAssertNil(model.loadError, model.loadError ?? "")
        let fixtureID = try XCTUnwrap(model.resultID)
        model.previewV2 = true
        model.setLiveV2(true) // pane visible
        XCTAssertTrue(model.requestedLayoutCapabilities.contains(V2Live.capability))
        // A sibling line for the fixture's own id while the fixture is applied: dropped, counted, fixture untouched.
        var line = try JSONSerialization.jsonObject(with: try Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.json"))) as! [String: Any]
        line["id"] = fixtureID
        let dropped = V2Live.staleLinesDropped
        model.receiveDisplayListV2(id: fixtureID, line: try JSONSerialization.data(withJSONObject: line))
        XCTAssertEqual(V2Live.staleLinesDropped, dropped + 1)
        XCTAssertNil(model.displayListV2)
        XCTAssertEqual(model.previewSource, .fixture)
        XCTAssertEqual(model.resultID, fixtureID)
        // The next compile from the fixture state: the worker's result replaces the fixture, its sibling is applied.
        let python = URL(fileURLWithPath: "/usr/bin/python3")
        guard FileManager.default.isExecutableFile(atPath: python.path) else { throw XCTSkip("python3 missing") }
        model.attachWorker(at: python, arguments: [Self.fixtures.appendingPathComponent("fake_worker_v2.py").path,
                                                   Self.fixtures.appendingPathComponent("display-list-v2-text.json").path])
        let accepted = V2Live.linesAccepted
        // The fixture's sibling compile-request.json seeded a 16-byte editor; the fake producer's
        // template geometry is for the v2 text fixture, so edit back to that text before compiling.
        model.updateActiveText(tex)
        XCTAssertEqual(model.previewSource, .fixture, "editing does not leave the fixture state")
        model.compile()
        do {
            try await waitUntil { model.inFlightRevision == nil && model.displayListV2?.frame != nil && model.displayListV2?.isLoading == false }
        } catch {
            XCTFail("timeout: status=\(model.workerStatus) source=\(model.previewSource) v2=\(String(describing: model.displayListV2?.source)) note=\(model.captureNote ?? "") log=\(model.workerLog.suffix(2))")
            throw error
        }
        XCTAssertEqual(model.previewSource, .worker("python3"))
        XCTAssertEqual(V2Live.linesAccepted, accepted + 1)
        XCTAssertEqual(V2Live.staleLinesDropped, dropped + 1, "nothing dropped: the fixture never cost a live frame")
        guard case .loaded(let frame, let source)? = model.displayListV2 else { return XCTFail() }
        XCTAssertTrue(source.isLive)
        XCTAssertEqual(frame.list.revision, model.result?.revision)
        model.detachWorker()
    }

    // MARK: D6 — outgoing helper lines are bounded at the helper's measured 1 MiB stdin limit

    /// A text that makes an `edit` line of exactly `lineBytes` bytes (newline included).
    private static func editText(sessionID: String, id: String, lineBytes: Int, revision: Int, sha: String) throws -> String {
        func line(_ fill: Int) throws -> (String, Int) {
            let text = "\\begin{document}\n" + String(repeating: "x", count: fill) + "\\end{document}\n"
            let payload: PreviewControllerClient.JSONObject = ["path": "main.tex", "expected_revision": revision, "expected_sha256": sha, "text": text]
            return (text, try PreviewControllerClient.requestLine(sessionID: sessionID, id: id, type: "edit", payload: payload).count)
        }
        let (_, probe) = try line(1000)
        let (text, size) = try line(1000 + lineBytes - probe)
        XCTAssertEqual(size, lineBytes)
        return text
    }

    func testHelperRequestAboveOneMiBIsRefusedLocallyBeforeSending() throws {
        XCTAssertEqual(PreviewControllerClient.maxRequestLineBytes, 1024 * 1024, "the helper's MAX_FRAME (measured 2026-09-12)")
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("v2conf-d6-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        // `/usr/bin/tee <config>` stands in for the helper: it stays alive and echoes
        // whatever reaches its stdin. The refused line must never reach it.
        var config = PreviewControllerClient.Config(sessionID: "d6-local", projectID: "d6", entryPath: "main.tex")
        config.storePaths = [root]
        var echoed = Data()
        let client = try PreviewControllerClient(executable: URL(fileURLWithPath: "/usr/bin/tee"), config: config) { event in
            if case .protocolViolation = event { echoed.append(1) } // tee echoes our request, which is not a helper frame
        }
        defer { client.close() }
        let sha = String(repeating: "0", count: 64)
        let refused = PreviewControllerClient.requestsRefusedTooLarge
        // One byte over: refused locally, typed, before any write.
        let over = try Self.editText(sessionID: "d6-local", id: "pc-1", lineBytes: 1024 * 1024 + 1, revision: 1, sha: sha)
        XCTAssertThrowsError(try client.edit(path: "main.tex", expectedRevision: 1, expectedSHA256: sha, text: over)) { error in
            guard let e = error as? PreviewControllerClient.RequestTooLarge else { return XCTFail("\(error)") }
            XCTAssertEqual(e.type, "edit")
            XCTAssertEqual(e.lineBytes, 1024 * 1024 + 1)
            XCTAssertTrue(e.errorDescription!.contains("exceeds the helper's 1048576-byte stdin line limit; not sent"), e.errorDescription!)
        }
        XCTAssertEqual(PreviewControllerClient.requestsRefusedTooLarge, refused + 1)
        // Two MiB: the same typed refusal (never split).
        let two = try Self.editText(sessionID: "d6-local", id: "pc-1", lineBytes: 2 * 1024 * 1024, revision: 1, sha: sha)
        XCTAssertThrowsError(try client.edit(path: "main.tex", expectedRevision: 1, expectedSHA256: sha, text: two)) {
            XCTAssertEqual(($0 as? PreviewControllerClient.RequestTooLarge)?.lineBytes, 2 * 1024 * 1024)
        }
        XCTAssertEqual(PreviewControllerClient.requestsRefusedTooLarge, refused + 2)
        // Exactly 1 MiB (refused requests consumed pc-1 and pc-2; this one is pc-3): admitted by the client.
        let exact = try Self.editText(sessionID: "d6-local", id: "pc-3", lineBytes: 1024 * 1024, revision: 1, sha: sha)
        XCTAssertEqual(try client.edit(path: "main.tex", expectedRevision: 1, expectedSHA256: sha, text: exact), "pc-3")
        XCTAssertEqual(PreviewControllerClient.requestsRefusedTooLarge, refused + 2)
        _ = echoed
    }

    /// The real helper: exactly 1 MiB is a durable edit; the client's refusal
    /// of the next byte keeps the helper alive (it would otherwise answer
    /// "truncated or oversized input" and exit — measured, see the evidence).
    func testRealHelperAdmitsExactlyOneMiBAndStaysAliveBehindTheClientBound() async throws {
        guard let helper = Self.helper, let compiler = Self.compiler,
              FileManager.default.isExecutableFile(atPath: helper.path), FileManager.default.isExecutableFile(atPath: compiler.path) else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("v2conf-d6-real-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: root.appendingPathComponent("ledger"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        try "\\begin{document}\nHello.\n\\end{document}\n".write(to: root.appendingPathComponent("project/main.tex"), atomically: true, encoding: .utf8)
        var config = PreviewControllerClient.Config(sessionID: "d6-real", projectID: "d6", entryPath: "main.tex")
        config.projectRoot = root.appendingPathComponent("project")
        config.privateLedgerRoot = root.appendingPathComponent("ledger")
        config.compilerPath = compiler
        var ready = false, results: [(String, PreviewControllerClient.JSONObject)] = [], errors: [String] = [], exited = false
        let client = try PreviewControllerClient(executable: helper, config: config) { event in
            switch event {
            case .ready: ready = true
            case .result(let id, let payload): results.append((id, payload))
            case .error(_, let message): errors.append(message)
            case .exited: exited = true
            default: break
            }
        }
        defer { client.close() }
        try await waitUntil { ready }
        let docID = try client.document(path: "main.tex")
        try await waitUntil { results.contains { $0.0 == docID } }
        let doc = try XCTUnwrap(results.first { $0.0 == docID }?.1["document"] as? PreviewControllerClient.JSONObject)
        let revision = try XCTUnwrap(doc["revision"] as? Int), sha = try XCTUnwrap(doc["source_sha256"] as? String)
        // Exactly 1 MiB: durable.
        let nextID = "pc-2" // the client's second id
        let exact = try Self.editText(sessionID: "d6-real", id: nextID, lineBytes: 1024 * 1024, revision: revision, sha: sha)
        let editID = try client.edit(path: "main.tex", expectedRevision: revision, expectedSHA256: sha, text: exact)
        XCTAssertEqual(editID, nextID)
        try await waitUntil(timeout: 40) { results.contains { $0.0 == editID } || !errors.isEmpty || exited }
        XCTAssertTrue(errors.isEmpty, "\(errors)")
        XCTAssertFalse(exited)
        let edited = try XCTUnwrap(results.first { $0.0 == editID }?.1["document"] as? PreviewControllerClient.JSONObject)
        XCTAssertEqual(edited["revision"] as? Int, revision + 1)
        let sha2 = try XCTUnwrap(edited["source_sha256"] as? String)
        // One byte more: refused by the client, the helper never sees it and keeps answering.
        let over = try Self.editText(sessionID: "d6-real", id: "pc-3", lineBytes: 1024 * 1024 + 1, revision: revision + 1, sha: sha2)
        XCTAssertThrowsError(try client.edit(path: "main.tex", expectedRevision: revision + 1, expectedSHA256: sha2, text: over)) {
            XCTAssertEqual(($0 as? PreviewControllerClient.RequestTooLarge)?.lineBytes, 1024 * 1024 + 1)
        }
        let aliveID = try client.document(path: "main.tex")
        try await waitUntil { results.contains { $0.0 == aliveID } || exited }
        XCTAssertFalse(exited, "the helper exited: the oversized line reached it")
        XCTAssertEqual((results.first { $0.0 == aliveID }?.1["document"] as? PreviewControllerClient.JSONObject)?["revision"] as? Int, revision + 1)
        XCTAssertTrue(client.isRunning)
    }
}
