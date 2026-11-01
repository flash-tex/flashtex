import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// D2 first-generation gate (Commander follow-up to the v2-conformance lane):
/// the display-candidate gate never skips the membership comparison. The
/// shell learns the project's authoritative `membership_generation` from the
/// helper with a `snapshot` sent on `ready` (before the `document` request
/// whose reply admits the first compile), a candidate that arrives before a
/// generation has been learned is refused with the typed
/// `membership_unknown` reason (v1 stays), and a learned generation is the
/// floor a candidate must reach — the helper's `membership_generation` is
/// the project-index generation, advancing on every durable edit as well as
/// on open/detach, and edit replies do not carry it — so a candidate from
/// before a membership change the shell learned is refused (V2ConformanceTests
/// covers the real open_document) while the shell's own later edits are not.
/// Detach/exit forgets the generation so the next helper session learns its own.
///
/// Real-helper cases use the helper and compiler from this tree (the v1
/// compiler declines display-list-v2, so candidates are injected); the
/// painting case additionally needs `FLASHTEX_RENDER`. All skip cleanly
/// without their binaries.
@MainActor
final class FirstGenerationGateTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Fonts")
    static var helper: URL? {
        guard let p = ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"], FileManager.default.isExecutableFile(atPath: p) else { return nil }
        return URL(fileURLWithPath: p)
    }
    static var render: URL? {
        guard let p = ProcessInfo.processInfo.environment["FLASHTEX_RENDER"], FileManager.default.isExecutableFile(atPath: p) else { return nil }
        return URL(fileURLWithPath: p)
    }

    private func candidate(generation: Int, session: String = "s1", request: String = "pc-7", project: String = "demo",
                           compileRevision: Int = 3, versions: [String: Int] = ["main.tex": 2], list: Data? = nil) throws -> DisplayCandidateFrame {
        let bytes = try list ?? Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.json"))
        return DisplayCandidateFrame(sessionID: session, requestID: request, projectID: project, compileRevision: compileRevision,
                                     sourceVersions: versions, membershipGeneration: generation, displayList: bytes,
                                     frameBytes: bytes.count, receivedNs: MonotonicClock.nowNs())
    }

    private func waitUntil(_ what: String, _ timeout: TimeInterval = 20, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timed out waiting for \(what)"); throw XCTSkip("timeout: \(what) (load-sensitive)") }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    // MARK: pure gate

    func testGateRefusesBeforeAGenerationIsLearnedAndComparesAfterwards() throws {
        let applied = DisplayCandidateAppliedPreview(requestID: "pc-7", compileRevision: 3, sourceVersions: ["main.tex": 2], editorRevision: 5)
        var gate = DisplayCandidateGate(negotiated: true, sessionID: "s1", projectID: "demo", applied: applied, appliedResultID: "pc-7",
                                        activePath: "main.tex", displayedEditorRevision: nil)
        XCTAssertNil(gate.membershipGeneration)
        // Before the first snapshot: refused, typed, naming the candidate's claim — never admitted unchecked.
        for generation in [0, 1, 7] {
            let why = try XCTUnwrap(gate.rejection(of: try candidate(generation: generation)))
            XCTAssertTrue(why.hasPrefix("\(DisplayCandidates.membershipUnknown): "), why)
            XCTAssertTrue(why.contains("candidate names generation \(generation)"), why)
        }
        XCTAssertEqual(DisplayCandidates.membershipUnknown, "membership_unknown")
        // The unknown-generation refusal sits with the identity checks, before the applied-preview checks.
        gate.applied = nil
        XCTAssertTrue(try XCTUnwrap(gate.rejection(of: try candidate(generation: 1))).hasPrefix(DisplayCandidates.membershipUnknown))
        gate.applied = applied
        // After the first snapshot: compared against the learned floor. The helper's number is the project index
        // generation (advances on every durable edit too; edit replies do not carry it), so a candidate older than
        // the learned value was compiled before a membership change the shell knows about and is refused, while a
        // newer one (the shell's own later edits) is not stale.
        gate.membershipGeneration = 1
        XCTAssertNil(gate.rejection(of: try candidate(generation: 1)))
        XCTAssertEqual(gate.rejection(of: try candidate(generation: 0)), "membership generation 0 is older than the project's learned generation 1")
        XCTAssertNil(gate.rejection(of: try candidate(generation: 2)))
        gate.membershipGeneration = 7
        XCTAssertEqual(gate.rejection(of: try candidate(generation: 6)), "membership generation 6 is older than the project's learned generation 7")
        XCTAssertNil(gate.rejection(of: try candidate(generation: 7)))
        // Forgotten again (detach/exit): refused again, never compared against the old session's number.
        gate.membershipGeneration = nil
        XCTAssertTrue(try XCTUnwrap(gate.rejection(of: try candidate(generation: 1))).hasPrefix(DisplayCandidates.membershipUnknown))
    }

    func testModelGateRefusesWhileTheProjectKnowsNoGeneration() throws {
        let model = ShellModel()
        XCTAssertNil(model.project.membershipGeneration)
        let gate = model.displayCandidates.gate(activePath: "main.tex", appliedResultID: "pc-7", membershipGeneration: model.project.membershipGeneration)
        XCTAssertNil(gate.membershipGeneration)
        // Not negotiated wins first (nothing was attached); the generation refusal follows once negotiated.
        XCTAssertEqual(gate.rejection(of: try candidate(generation: 1)), "display candidates not negotiated for this session")
        var negotiated = gate
        negotiated.negotiated = true; negotiated.sessionID = "s1"; negotiated.projectID = "demo"
        XCTAssertTrue(try XCTUnwrap(negotiated.rejection(of: try candidate(generation: 1))).hasPrefix(DisplayCandidates.membershipUnknown))
        // adoptSnapshot / forgetMembership are the two transitions the model's gate reads.
        XCTAssertNil(model.project.adoptSnapshot(["source_versions": ["main.tex": 1]]), "membership_generation is required")
        XCTAssertNil(model.project.membershipGeneration)
        XCTAssertEqual(model.project.adoptSnapshot(["source_versions": ["main.tex": 1], "membership_generation": 4]),
                       ProjectDocuments.Snapshot(versions: ["main.tex": 1], generation: 4))
        XCTAssertEqual(model.project.membershipGeneration, 4)
        XCTAssertEqual(model.project.sourceVersions, ["main.tex": 1])
        model.project.forgetMembership()
        XCTAssertNil(model.project.membershipGeneration)
        XCTAssertEqual(model.project.sourceVersions, [:])
    }

    // MARK: real helper (v1 compiler; candidates injected)

    private struct Project { var root: URL; var tex: URL }

    private func project(named name: String, text: String) throws -> Project {
        guard ShellModel.locateCompiler() != nil else { throw XCTSkip("set FLASHTEX_COMPILER to a built binary") }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("fgg-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        let tex = root.appendingPathComponent("project/main.tex")
        try text.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        return Project(root: root, tex: tex)
    }

    private func learnedLines(_ model: ShellModel) -> [String] { model.workerLog.filter { $0.hasPrefix("display-candidate: learned membership generation") } }

    /// The generation is learned on `ready` with the opt-in (no explicit
    /// membership operation), a candidate before it is learned is refused
    /// as `membership_unknown`, the same candidate afterwards passes the
    /// membership check, and detaching forgets the generation.
    func testHelperReadyLearnsTheGenerationAndCandidatesBeforeItAreRefused() async throws {
        guard let helper = Self.helper else { throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER to a helper built from this tree") }
        let p = try project(named: "learn", text: "\\begin{document}\nHello generations.\n\\end{document}\n")
        defer { try? FileManager.default.removeItem(at: p.root); unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        setenv("FLASHTEX_DISPLAY_CANDIDATES", "1", 1)
        defer { unsetenv("FLASHTEX_DISPLAY_CANDIDATES") }
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: p.tex), .opened)
        XCTAssertNil(model.project.membershipGeneration)
        model.attachController(at: helper)
        defer { model.detachController() }
        try await waitUntil("negotiation") { model.displayCandidatesNegotiated }
        try await waitUntil("initial preview") { model.result?.revision == model.editorRevision && model.previewSource == .worker("flashtex-preview-controller") && model.inFlightRevision == nil }
        // Learned on ready, without any open/detach/project_status, and before the first preview was applied.
        let generation = try XCTUnwrap(model.project.membershipGeneration, "the ready-time snapshot taught the generation: \(model.workerLog.suffix(12))")
        XCTAssertEqual(model.project.sourceVersions["main.tex"], 1)
        let learned = try XCTUnwrap(learnedLines(model).last)
        XCTAssertTrue(learned.contains("generation \(generation) (1 member(s)"), learned)
        XCTAssertEqual(learnedLines(model).count, 1, "learned exactly once for this session")
        XCTAssertEqual(model.project.helperSyncs, 0, "no member sync was needed to learn it")
        // A well-formed candidate for the applied preview with the learned generation passes the membership check
        // (the v1 compiler produced no sibling; the gate's later checks are the applied identity, which holds).
        let applied = try XCTUnwrap(model.displayCandidates.applied)
        let session = model.controller!.config.sessionID, projectID = model.controller!.config.projectID
        let frame = try candidate(generation: generation, session: session, request: applied.requestID, project: projectID,
                                  compileRevision: applied.compileRevision, versions: applied.sourceVersions)
        XCTAssertNil(model.displayCandidates.gate(activePath: "main.tex", appliedResultID: model.resultID, membershipGeneration: model.project.membershipGeneration).rejection(of: frame))
        let older = try candidate(generation: generation - 1, session: session, request: applied.requestID, project: projectID,
                                  compileRevision: applied.compileRevision, versions: applied.sourceVersions)
        XCTAssertEqual(model.displayCandidates.gate(activePath: "main.tex", appliedResultID: model.resultID, membershipGeneration: model.project.membershipGeneration).rejection(of: older),
                       "membership generation \(generation - 1) is older than the project's learned generation \(generation)")
        // The shell's own edits advance the helper's generation without a new learn: the next candidate is not stale.
        model.updateActiveText("\\begin{document}\nHello generations, edited.\n\\end{document}\n")
        let edited = model.editorRevision
        try await waitUntil("edited preview") { model.result?.revision == edited && model.inFlightRevision == nil }
        XCTAssertEqual(model.project.membershipGeneration, generation, "no membership operation ran: the learned floor is unchanged")
        let later = try XCTUnwrap(model.displayCandidates.applied)
        let newer = try candidate(generation: generation + 1, session: session, request: later.requestID, project: projectID,
                                  compileRevision: later.compileRevision, versions: later.sourceVersions)
        XCTAssertNil(model.displayCandidates.gate(activePath: "main.tex", appliedResultID: model.resultID, membershipGeneration: model.project.membershipGeneration).rejection(of: newer))
        // …and a membership operation raises the floor: the pre-open candidate is refused (V2ConformanceTests covers the real open).
        model.project.adoptSnapshot(["source_versions": ["main.tex": 2], "membership_generation": generation + 5])
        XCTAssertEqual(model.displayCandidates.gate(activePath: "main.tex", appliedResultID: model.resultID, membershipGeneration: model.project.membershipGeneration).rejection(of: newer),
                       "membership generation \(generation + 1) is older than the project's learned generation \(generation + 5)")

        // The state before the first snapshot (a candidate racing the learn): refused, typed, v1 untouched.
        model.project.forgetMembership()
        let refused = model.displayCandidates.refused, received = model.displayCandidates.received
        let v1 = model.result?.revision
        model.handleDisplayCandidate(newer)
        XCTAssertEqual(model.displayCandidates.received, received + 1)
        XCTAssertEqual(model.displayCandidates.refused, refused + 1)
        let why = try XCTUnwrap(model.displayCandidates.lastRefusal)
        XCTAssertTrue(why.hasPrefix("\(DisplayCandidates.membershipUnknown): "), why)
        XCTAssertTrue(model.displayCandidates.status.contains(DisplayCandidates.membershipUnknown), model.displayCandidates.status)
        XCTAssertNil(model.displayCandidates.pending, "not queued")
        XCTAssertNil(model.displayCandidates.validating)
        XCTAssertNil(model.displayListV2)
        XCTAssertEqual(model.result?.revision, v1, "v1 kept")
        // Learned again by the explicit snapshot request (the helper is one edit further on): the same candidate is
        // compared again — and is now older than the learned generation, i.e. refused by comparison, not skipped.
        let refreshed = await model.project.refreshSnapshot()
        XCTAssertEqual(try XCTUnwrap(refreshed).generation, generation + 1, "one durable edit advanced the helper's generation by one")
        XCTAssertEqual(model.displayCandidates.gate(activePath: "main.tex", appliedResultID: model.resultID, membershipGeneration: model.project.membershipGeneration).rejection(of: frame),
                       "membership generation \(generation) is older than the project's learned generation \(generation + 1)")
        XCTAssertNil(model.displayCandidates.gate(activePath: "main.tex", appliedResultID: model.resultID, membershipGeneration: model.project.membershipGeneration).rejection(of: newer))
        // Detach forgets it: the next session learns its own on ready.
        model.detachController()
        XCTAssertNil(model.project.membershipGeneration)
        XCTAssertEqual(model.project.sourceVersions, [:])
    }

    /// A toggle after attach (route OFF at attach) learns the generation with
    /// the opt-in, so the first candidate of the enabled route is compared.
    func testEnablingTheRouteLaterLearnsTheGenerationWithTheOptIn() async throws {
        guard let helper = Self.helper else { throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER to a helper built from this tree") }
        let p = try project(named: "toggle", text: "\\begin{document}\nHello toggle.\n\\end{document}\n")
        defer { try? FileManager.default.removeItem(at: p.root); unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        unsetenv("FLASHTEX_DISPLAY_CANDIDATES")
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertFalse(model.displayCandidates.requested)
        XCTAssertEqual(model.openTex(at: p.tex), .opened)
        model.attachController(at: helper)
        defer { model.detachController() }
        try await waitUntil("initial preview") { model.result?.revision == model.editorRevision && model.previewSource == .worker("flashtex-preview-controller") }
        XCTAssertNil(model.project.membershipGeneration, "the route was off: no snapshot was sent on ready")
        XCTAssertTrue(learnedLines(model).isEmpty)
        model.setDisplayCandidates(true)
        try await waitUntil("negotiation") { model.displayCandidatesNegotiated && model.project.membershipGeneration != nil }
        XCTAssertEqual(learnedLines(model).count, 1)
        // A restart (fresh opt-in) learns it again for the restarted helper compiler.
        model.displayCandidatesRestartHelper()
        try await waitUntil("re-negotiation after restart") { model.displayCandidatesNegotiated && learnedLines(model).count == 2 }
        XCTAssertNotNil(model.project.membershipGeneration)
    }

    // MARK: real helper + real producer: the first candidate of a session paints only after the learn

    func testFirstCandidateOfASessionIsComparedAgainstTheReadyTimeGeneration() async throws {
        guard let helper = Self.helper, let render = Self.render else { throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_RENDER to built binaries") }
        let p = try project(named: "paint", text: "\\documentclass{article}\n\\begin{document}\nHello first candidate, fi.\n\\end{document}\n")
        defer { try? FileManager.default.removeItem(at: p.root); unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT") }
        if ProcessInfo.processInfo.environment["FLASHTEX_LM_DIR"] == nil { setenv("FLASHTEX_LM_DIR", Self.fontsDir.path, 1) }
        let previousCompiler = ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"]
        setenv("FLASHTEX_COMPILER", render.path, 1)
        defer { if let previousCompiler { setenv("FLASHTEX_COMPILER", previousCompiler, 1) } else { unsetenv("FLASHTEX_COMPILER") } }
        setenv("FLASHTEX_DISPLAY_CANDIDATES", "1", 1)
        defer { unsetenv("FLASHTEX_DISPLAY_CANDIDATES") }
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: p.tex), .opened)
        model.attachController(at: helper)
        defer { model.detachController() }
        try await waitUntil("negotiation") { model.displayCandidatesNegotiated }
        try await waitUntil("first painted candidate", 40) { if case .loaded? = model.displayListV2 { return true } else { return false } }
        // No explicit membership operation ran; the ready-time learn preceded the first candidate, which was compared, not skipped.
        let generation = try XCTUnwrap(model.project.membershipGeneration)
        let learned = try XCTUnwrap(model.workerLog.firstIndex { $0.hasPrefix("display-candidate: learned membership generation \(generation)") })
        let admitted = try XCTUnwrap(model.workerLog.firstIndex { $0.hasPrefix("display-candidate: admitted") })
        XCTAssertLessThan(learned, admitted, "the generation was learned before the first candidate arrived: \(model.workerLog)")
        XCTAssertFalse(model.workerLog.contains { $0.contains(DisplayCandidates.membershipUnknown) }, "no candidate raced the learn: \(model.workerLog.filter { $0.contains(DisplayCandidates.membershipUnknown) })")
        XCTAssertEqual(model.displayCandidates.refused, 0)
        XCTAssertGreaterThanOrEqual(model.displayCandidates.published, 1)
        XCTAssertEqual(model.project.helperSyncs, 0)
    }
}
