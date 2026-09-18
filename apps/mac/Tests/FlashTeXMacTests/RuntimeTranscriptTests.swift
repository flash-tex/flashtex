import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Runs main's transcript validator (`scripts/check_runtime.py`, cab39a2) over
/// the JSON Lines the shell actually sent to and received from the worker
/// double, and checks that the shell's own apply/ignore decisions agree with
/// the validator's per-response classification (`current` / `stale_ignore`),
/// including asynchronous same-revision layout-capability switches.
@MainActor
final class RuntimeTranscriptTests: XCTestCase {
    static let repoRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent()
    static let checker = repoRoot.appendingPathComponent("scripts/check_runtime.py")
    static let extended = RuntimeV1.LayoutCapabilities.supported

    struct CheckerReport: Decodable {
        struct Response: Decodable {
            var id: String
            var type: String?
            var preview: String?
            var accepted_layout_capabilities: [String]?
            var missing_layout_capabilities: [String]?
        }
        struct Failure: Decodable { var line: Int?; var message: String }
        var valid: Bool
        var errors: [Failure]
        var responses: [Response]
        var pending_request_ids: [String]
    }

    private var transcriptURL: URL!

    override func setUpWithError() throws {
        try super.setUpWithError()
        guard FileManager.default.isReadableFile(atPath: Self.checker.path) else {
            throw XCTSkip("scripts/check_runtime.py not found at \(Self.checker.path)")
        }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-transcript-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        transcriptURL = dir.appendingPathComponent("shell.jsonl")
    }

    override func tearDown() {
        if let url = transcriptURL { try? FileManager.default.removeItem(at: url.deletingLastPathComponent()) }
        super.tearDown()
    }

    /// `python3 scripts/check_runtime.py <transcript>` → (exit status, parsed report).
    private func runChecker(_ url: URL, extraArguments: [String] = []) throws -> (status: Int32, report: CheckerReport) {
        let p = Process()
        p.executableURL = WorkerClientTests.python
        p.arguments = [Self.checker.path, url.path] + extraArguments
        let out = Pipe(), err = Pipe()
        p.standardOutput = out; p.standardError = err
        try p.run()
        let data = out.fileHandleForReading.readDataToEndOfFile()
        let stderr = String(decoding: err.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
        p.waitUntilExit()
        let report = try XCTUnwrap(try? JSONDecoder().decode(CheckerReport.self, from: data),
                                   "checker output was not the expected JSON: \(String(decoding: data, as: UTF8.self)) stderr: \(stderr)")
        return (p.terminationStatus, report)
    }

    private func attachedModel() throws -> ShellModel {
        let model = ShellModel()
        model.transcript = try RuntimeTranscript(url: transcriptURL)
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        model.autoCompile = false
        return model
    }

    func testShellTranscriptPassesTheRuntimeValidatorAndAgreesOnStaleClassification() async throws {
        let model = try attachedModel()
        let applied = ResultIDChanges(model: model)
        defer { applied.stop() }

        // 1. Legacy request (no layout_capabilities field at all).
        model.requestedLayoutCapabilities = []
        model.updateActiveText("%caps\nplain one\n")
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        let legacyID = try XCTUnwrap(model.resultID)

        // 2. Capability switch with no edit: same revision, new id.
        model.requestedLayoutCapabilities = Self.extended + ["future-v9"]
        model.compile()
        XCTAssertEqual(model.inFlightRevision, model.result?.revision)
        try await waitUntil { model.inFlightRevision == nil }
        let switchedID = try XCTUnwrap(model.resultID)
        XCTAssertNotEqual(switchedID, legacyID)
        XCTAssertEqual(model.negotiation.accepted, Self.extended)
        XCTAssertEqual(model.negotiation.missing, ["future-v9"])

        // 3. Edit under the extended set; 4. a substitution reply; 5. a worker error.
        model.requestedLayoutCapabilities = Self.extended
        model.updateActiveText("%caps\nedited two\n")
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        model.updateActiveText("%caps:sub\nComic\n")
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertEqual(model.fontSubstitutions.count, 1)
        model.updateActiveText("%error\n")
        model.compile()
        let errorID = try XCTUnwrap(model.latestRequestID)
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertTrue(model.workerStatus.hasPrefix("worker error"), model.workerStatus)

        // 6. Asynchronous same-revision mode switch: the slow legacy request is
        // still pending when the extended request for the same revision goes
        // out. Transport order: req A, req B, reply A (stale), reply B (current).
        model.requestedLayoutCapabilities = []
        model.updateActiveText("%caps\n%slow\nasync switch\n")
        model.compile()
        let slowID = try XCTUnwrap(model.latestRequestID)
        model.requestedLayoutCapabilities = Self.extended
        model.compile()
        let asyncID = try XCTUnwrap(model.latestRequestID)
        XCTAssertNotEqual(asyncID, slowID)
        XCTAssertEqual(model.inFlightRequests.count, 2)
        try await waitUntil(timeout: 15) { model.inFlightRequests.isEmpty }
        XCTAssertEqual(model.resultID, asyncID)
        XCTAssertFalse(applied.ids.contains(slowID), "superseded reply must not be applied")
        XCTAssertFalse(model.workerStatus.contains("violation"), model.workerStatus)
        model.detachWorker()

        // The validator accepts the whole exchange and classifies exactly the
        // replies the shell applied as `current`.
        let (status, report) = try runChecker(transcriptURL)
        XCTAssertEqual(status, 0, "\(report.errors)")
        XCTAssertTrue(report.valid, "\(report.errors)")
        XCTAssertEqual(report.errors.map(\.message), [])
        XCTAssertEqual(report.pending_request_ids, [])
        let byID = Dictionary(uniqueKeysWithValues: report.responses.map { ($0.id, $0) })
        XCTAssertEqual(Set(byID.keys), Set(applied.ids + [slowID, errorID]), "every reply is a response the shell saw")
        for response in report.responses {
            let shellApplied = applied.ids.contains(response.id)
            switch response.type ?? "compile_result" {
            case "error":
                XCTAssertEqual(response.id, errorID)
                XCTAssertFalse(shellApplied)
            default:
                XCTAssertEqual(response.preview, shellApplied ? "current" : "stale_ignore",
                               "\(response.id): shell \(shellApplied ? "applied" : "ignored") it")
            }
        }
        XCTAssertEqual(byID[slowID]?.preview, "stale_ignore")
        XCTAssertEqual(byID[asyncID]?.preview, "current")
        XCTAssertEqual(byID[asyncID]?.accepted_layout_capabilities, Self.extended.sorted())
        XCTAssertEqual(byID[switchedID]?.missing_layout_capabilities, ["future-v9"])
        XCTAssertEqual(byID[legacyID]?.accepted_layout_capabilities, [])
        XCTAssertEqual(byID[legacyID]?.missing_layout_capabilities, [])
        XCTAssertEqual(report.responses.count, 7)
    }

    func testShellAndValidatorBothRejectUnnegotiatedShapesAndSwitchesAreOrderedByRevision() async throws {
        let model = try attachedModel()
        // Rapid legacy/extended switching with edits: revisions never decrease,
        // each reply is bound to its own request.
        for i in 0..<4 {
            model.requestedLayoutCapabilities = i % 2 == 0 ? [] : Self.extended
            model.updateActiveText("%caps\nswitch \(i)\n")
            model.compile()
            try await waitUntil { model.inFlightRevision == nil }
            XCTAssertEqual(model.negotiation.accepted, i % 2 == 0 ? [] : Self.extended)
        }
        var (status, report) = try runChecker(transcriptURL)
        XCTAssertEqual(status, 0, "\(report.errors)")
        XCTAssertEqual(report.responses.map(\.preview), ["current", "current", "current", "current"])

        // A reply carrying a rule/font hint the request never negotiated: the
        // shell rejects it as a protocol violation and so does the validator.
        model.requestedLayoutCapabilities = []
        model.updateActiveText("%caps:unrequested\nSneaky\n")
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertTrue(model.workerStatus.hasPrefix("protocol violation:"), model.workerStatus)
        model.detachWorker()
        (status, report) = try runChecker(transcriptURL)
        XCTAssertEqual(status, 1)
        XCTAssertFalse(report.valid)
        let invalid = try XCTUnwrap(report.errors.first { $0.message.contains("requires accepted") }, "\(report.errors)")
        XCTAssertEqual(invalid.line, 10, "the fifth reply (line 10) is the invalid one")
        // The rejected reply is not a terminal response, so its request stays pending.
        XCTAssertEqual(report.pending_request_ids, ["mac-5"])
        XCTAssertEqual(report.responses.count, 4, "the invalid reply is not classified")
    }

    private func waitUntil(timeout: TimeInterval = 10, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }
}

/// Every id `ShellModel.resultID` takes, in order (one per applied result).
///
/// GH-680: this used to be reconstructed by observing `resultID` and reading
/// it back on a later main-actor turn (`@Observable`'s willChange notification
/// fires before the new value lands, so the old code deferred the read). That
/// is lossy: if a second apply happens before the deferred read for the first
/// one runs -- which needs no more than a delayed Task scheduling turn, far
/// more likely under full-suite load -- both deferred reads see the *second*
/// id, and the dedup-by-last-value check silently drops the first one. Hooking
/// `onResultApplied` instead records the id synchronously, in the same call
/// that sets it, so no apply can ever be missed or coalesced regardless of
/// scheduling pressure.
@MainActor
private final class ResultIDChanges {
    private(set) var ids: [String] = []
    private weak var model: ShellModel?

    init(model: ShellModel) {
        self.model = model
        model.onResultApplied = { [weak self] id in self?.ids.append(id) }
    }

    func stop() { model?.onResultApplied = nil }
}
