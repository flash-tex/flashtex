import XCTest
@testable import FlashTeXMac

/// Gap 4: the export's cancel / failure / overwrite-conflict / success paths
/// never leave a partial or damaged file at the destination. The PDF comes
/// from the engine host's export run (ExactPDFExport.swift); the session
/// holds the Cancel and publishes the bytes (`startExternal` /
/// `finishExternal`), so these cases drive it directly and need no tool. The
/// end-to-end export through a host is EngineV3ExportTests.
@MainActor
final class ExportSessionTests: XCTestCase {
    private var dir: URL!

    override func setUpWithError() throws {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("export-session-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: dir)
    }

    // MARK: helpers

    private struct Snapshot: Equatable {
        var sha256: String?
        var mtime: Date?
        var size: Int?
        init(_ url: URL) {
            sha256 = ExportSession.diskSHA256(url)
            let attrs = try? FileManager.default.attributesOfItem(atPath: url.path)
            mtime = attrs?[.modificationDate] as? Date
            size = attrs?[.size] as? Int
        }
    }

    /// Writes `bytes` at `url` with an old mtime so "unchanged" is provable.
    private func plantExisting(_ url: URL, bytes: String = "%PDF-1.4 existing\n%%EOF\n") throws -> Snapshot {
        try Data(bytes.utf8).write(to: url)
        try FileManager.default.setAttributes([.modificationDate: Date(timeIntervalSince1970: 1_600_000_000)], ofItemAtPath: url.path)
        return Snapshot(url)
    }

    private func leftoverTemps() -> [String] {
        ((try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? []).filter { $0.contains(".flashtex-export-") }
    }

    /// A finished PDF's bytes (what the host's export run hands over).
    private static let pdf = Data("%PDF-1.5\n1 0 obj << /Type /Catalog >> endobj\n%%EOF\n".utf8)

    // MARK: (a) cancel mid-run

    func testCancelAsksTheProducerToStopAndLeavesNoOutput() throws {
        let out = dir.appendingPathComponent("cancelled.pdf")
        let before = try plantExisting(out)
        let session = ExportSession()
        var cancels = 0
        var report: ExportSession.Report?
        XCTAssertTrue(session.startExternal(destination: .recordingCurrentDisk(out), cancel: {
            cancels += 1
            session.finishExternal(pdf: nil) // the producer stopped: no PDF
        }) { report = $0 })
        guard case .running(let pid, let at) = session.state else { return XCTFail("not running: \(session.state)") }
        XCTAssertEqual(pid, 0, "the producer is the engine host, not a process of this session")
        XCTAssertLessThan(abs(at.timeIntervalSinceNow), 5)
        // A second start on the same session is refused while running.
        var second: ExportSession.Report?
        XCTAssertFalse(session.startExternal(destination: .recordingCurrentDisk(out), cancel: {}) { second = $0 })
        XCTAssertEqual(second?.state, .failed(reason: "an export is already running"))
        XCTAssertTrue(session.state.isRunning)
        session.cancel()
        XCTAssertEqual(cancels, 1)
        XCTAssertEqual(report?.state, .cancelled)
        XCTAssertEqual(session.state, .cancelled)
        XCTAssertNil(report?.exitCode)
        XCTAssertEqual(Snapshot(out), before, "existing destination must be byte- and mtime-identical")
        XCTAssertEqual(leftoverTemps(), [])
        // cancel() and a late PDF after completion are no-ops.
        session.cancel()
        session.finishExternal(pdf: Self.pdf)
        XCTAssertEqual(cancels, 1)
        XCTAssertEqual(session.state, .cancelled)
        XCTAssertEqual(Snapshot(out), before)
    }

    // MARK: (b) the producer fails

    func testFailureLeavesTheExistingFileUntouched() throws {
        let out = dir.appendingPathComponent("refused.pdf")
        let before = try plantExisting(out)
        let session = ExportSession()
        var report: ExportSession.Report?
        XCTAssertTrue(session.startExternal(destination: .recordingCurrentDisk(out), cancel: {}) { report = $0 })
        session.finishExternal(pdf: nil, failure: "the host's export run failed")
        guard case .failed(let reason)? = report?.state else { return XCTFail("\(String(describing: report?.state))") }
        XCTAssertEqual(reason, "Export failed: the host's export run failed. Nothing was written to refused.pdf.")
        XCTAssertNil(report?.conflict)
        XCTAssertEqual(Snapshot(out), before)
        XCTAssertEqual(leftoverTemps(), [])
        // Fresh destination: still nothing is created, an empty PDF included.
        let fresh = dir.appendingPathComponent("fresh-refused.pdf")
        XCTAssertTrue(session.startExternal(destination: .recordingCurrentDisk(fresh), cancel: {}))
        session.finishExternal(pdf: Data(), failure: "no pages")
        XCTAssertFalse(FileManager.default.fileExists(atPath: fresh.path))
        XCTAssertEqual(leftoverTemps(), [])
    }

    /// Through the window: with no engine running, Export PDF… to a chosen
    /// file says why in the status line and writes nothing.
    func testTheWindowsExportWithoutTheEngineWritesNothing() throws {
        let out = dir.appendingPathComponent("no-engine.pdf")
        let before = try plantExisting(out)
        let model = ShellModel()
        var report: ExportSession.Report?
        model.exportPDFEngineV3(to: .recordingCurrentDisk(out)) { report = $0 }
        guard case .failed(let reason)? = report?.state else { return XCTFail("\(String(describing: report?.state))") }
        XCTAssertTrue(reason.hasPrefix("Export failed: "), reason)
        XCTAssertTrue(reason.contains("Nothing was written to no-engine.pdf"), reason)
        XCTAssertEqual(model.captureNote, reason)
        XCTAssertEqual(Snapshot(out), before)
        XCTAssertEqual(leftoverTemps(), [])
    }

    // MARK: (c) overwrite conflict

    func testDestinationChangedSinceChosenIsRefusedBeforeLaunch() throws {
        let out = dir.appendingPathComponent("conflict.pdf")
        _ = try plantExisting(out, bytes: "first\n")
        let chosen = ExportSession.Destination.recordingCurrentDisk(out)
        XCTAssertEqual(chosen.expectedDiskSHA256, ExportSession.diskSHA256(out))
        let after = try plantExisting(out, bytes: "changed by someone else\n")
        let session = ExportSession()
        var cancelled = false
        var report: ExportSession.Report?
        XCTAssertFalse(session.startExternal(destination: chosen, cancel: { cancelled = true }) { report = $0 }, "refused before the producer starts")
        XCTAssertFalse(cancelled)
        let conflict = try XCTUnwrap(report?.conflict)
        XCTAssertEqual(conflict.kind, .modifiedExternally)
        XCTAssertEqual(conflict.ours, chosen.expectedDiskSHA256)
        XCTAssertEqual(conflict.theirs, after.sha256)
        XCTAssertFalse(conflict.viaHelper)
        XCTAssertEqual(report?.state, .failed(reason: conflict.exportSummary))
        XCTAssertEqual(session.conflict, conflict)
        XCTAssertTrue(conflict.exportSummary.contains("conflict.pdf was modified on disk since you chose it"), conflict.exportSummary)
        XCTAssertTrue(conflict.exportSummary.contains("the existing file is kept"), conflict.exportSummary)
        XCTAssertEqual(Snapshot(out), after, "the newer file on disk is kept")
        XCTAssertEqual(leftoverTemps(), [])

        // Appeared since chosen (chosen when absent) and deleted since chosen.
        let appeared = dir.appendingPathComponent("appeared.pdf")
        let chosenAbsent = ExportSession.Destination.recordingCurrentDisk(appeared)
        XCTAssertNil(chosenAbsent.expectedDiskSHA256)
        let planted = try plantExisting(appeared)
        var r2: ExportSession.Report?
        XCTAssertFalse(session.startExternal(destination: chosenAbsent, cancel: {}) { r2 = $0 })
        XCTAssertEqual(r2?.conflict?.kind, .alreadyExists)
        XCTAssertEqual(Snapshot(appeared), planted)
        let gone = dir.appendingPathComponent("gone.pdf")
        _ = try plantExisting(gone)
        let chosenPresent = ExportSession.Destination.recordingCurrentDisk(gone)
        try FileManager.default.removeItem(at: gone)
        var r3: ExportSession.Report?
        XCTAssertFalse(session.startExternal(destination: chosenPresent, cancel: {}) { r3 = $0 })
        XCTAssertEqual(r3?.conflict?.kind, .deletedExternally)
        XCTAssertFalse(FileManager.default.fileExists(atPath: gone.path), "a refusal never creates the file")
        // Unchanged destination: the conflict check passes and the export starts.
        let same = dir.appendingPathComponent("same.pdf")
        _ = try plantExisting(same)
        XCTAssertTrue(session.startExternal(destination: .recordingCurrentDisk(same), cancel: {}))
        XCTAssertNil(session.conflict)
        session.finishExternal(pdf: nil)
        XCTAssertEqual(session.state, .cancelled)
    }

    func testDestinationChangedWhileProducingIsRefusedAndTempRemoved() throws {
        let out = dir.appendingPathComponent("tampered.pdf")
        _ = try plantExisting(out, bytes: "first\n")
        let session = ExportSession()
        var report: ExportSession.Report?
        XCTAssertTrue(session.startExternal(destination: .recordingCurrentDisk(out), cancel: {}) { report = $0 })
        // Another writer changes the destination while the PDF is produced.
        try Data("tampered meanwhile\n".utf8).write(to: out)
        session.finishExternal(pdf: Self.pdf)
        XCTAssertEqual(report?.conflict?.kind, .modifiedDuringSave)
        XCTAssertTrue(report?.conflict?.exportSummary.contains("changed while the PDF was being produced") == true, "\(String(describing: report?.state))")
        XCTAssertEqual(report?.state, .failed(reason: report?.conflict?.exportSummary ?? ""))
        XCTAssertEqual(try String(contentsOf: out, encoding: .utf8), "tampered meanwhile\n", "the other writer's file is kept, not overwritten by the export")
        XCTAssertEqual(leftoverTemps(), [])
    }

    // MARK: (e) success

    func testSuccessReplacesAtomicallyAndReportsTheFileBytesAndSHA() throws {
        let out = dir.appendingPathComponent("success.pdf")
        let before = try plantExisting(out)
        let session = ExportSession()
        var report: ExportSession.Report?
        XCTAssertTrue(session.startExternal(destination: .recordingCurrentDisk(out), cancel: {}) { report = $0 })
        session.finishExternal(pdf: Self.pdf)
        guard case .succeeded(let bytes, let sha)? = report?.state else { return XCTFail("\(String(describing: report?.state))") }
        let onDisk = Snapshot(out)
        XCTAssertEqual(onDisk.size, bytes)
        XCTAssertEqual(bytes, Self.pdf.count)
        XCTAssertEqual(onDisk.sha256, sha)
        XCTAssertNotEqual(onDisk, before)
        XCTAssertEqual(try Data(contentsOf: out), Self.pdf)
        XCTAssertEqual(session.state, report?.state)
        XCTAssertNil(session.conflict)
        XCTAssertEqual(leftoverTemps(), [], "the sibling temp file is renamed, never left behind")
    }
}
