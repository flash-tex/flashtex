import XCTest
import Darwin
@testable import FlashTeXMac

/// Gap 4: the exact export's cancel / error / overwrite-conflict / timeout /
/// success paths never leave a partial or damaged file at the destination.
/// The real `flashtex-pdf-exact` (`FLASHTEX_PDF_EXACT`) drives (a), (b), (d),
/// (e); a FIFO as the display list blocks the tool in `open(2)` for the
/// cancel/timeout cases. (c) needs no tool (refused before launch) or a
/// scripted tool that tampers with the destination while "rendering".
@MainActor
final class ExportSessionTests: XCTestCase {
    static var tool: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_PDF_EXACT"].map { URL(fileURLWithPath: $0) }
            .flatMap { FileManager.default.isExecutableFile(atPath: $0.path) ? $0 : nil }
    }
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let fontsDir = fixtures.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Fonts").path

    private var dir: URL!

    override func setUpWithError() throws {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("export-session-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        setenv("FLASHTEX_FONT_DIRS", Self.fontsDir, 1)
    }

    override func tearDownWithError() throws {
        unsetenv("FLASHTEX_FONT_DIRS")
        try? FileManager.default.removeItem(at: dir)
    }

    // MARK: helpers

    private func requireTool() throws -> URL {
        guard let tool = Self.tool else { throw XCTSkip("set FLASHTEX_PDF_EXACT to a built flashtex-pdf-exact") }
        return tool
    }

    /// 1-minute load average; timing assertions are skipped above 20.
    private func loadAverage() -> Double {
        var loads = [Double](repeating: 0, count: 3)
        return getloadavg(&loads, 3) > 0 ? loads[0] : 0
    }

    /// Behavioural assertions (pid gone, file untouched) always run; the
    /// elapsed-time bounds are asserted only on a calm machine (1-min load ≤ 20).
    private func assertElapsed(_ elapsed: TimeInterval, under bound: TimeInterval, _ what: String, file: StaticString = #filePath, line: UInt = #line) {
        let load = loadAverage()
        print("ExportSessionTests: \(what) took \(elapsed)s at 1-min load \(load)")
        if load > 20 { print("ExportSessionTests: timing bound skipped (1-min load \(load) > 20)"); return }
        XCTAssertLessThan(elapsed, bound, "\(what) took \(elapsed)s at load \(load)", file: file, line: line)
    }

    /// A FIFO nobody writes: `flashtex-pdf-exact from-v2` blocks opening it.
    private func blockingList() throws -> URL {
        let fifo = dir.appendingPathComponent("blocked-list.json")
        guard mkfifo(fifo.path, 0o600) == 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno)) }
        return fifo
    }

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

    private func isAlive(_ pid: Int32) -> Bool { kill(pid, 0) == 0 || errno == EPERM }

    private func run(_ session: ExportSession, tool: URL, list: URL, destination: ExportSession.Destination,
                     timeout: TimeInterval = 60, fontDirs: [String] = [],
                     onRunning: (@MainActor (Int32) -> Void)? = nil) async -> ExportSession.Report {
        await withCheckedContinuation { cont in
            session.start(tool: tool, list: list, destination: destination, fontDirs: fontDirs, timeout: timeout) { cont.resume(returning: $0) }
            if case .running(let pid, _) = session.state { onRunning?(pid) }
        }
    }

    // MARK: (a) cancel mid-run

    func testCancelTerminatesOnlyTheLaunchedToolAndLeavesNoOutput() async throws {
        let tool = try requireTool()
        let list = try blockingList()
        let out = dir.appendingPathComponent("cancelled.pdf")
        let before = try plantExisting(out)
        // An unrelated process must survive the cancel: only our pid is signalled.
        let bystander = Process()
        bystander.executableURL = URL(fileURLWithPath: "/bin/sleep"); bystander.arguments = ["60"]
        try bystander.run()
        defer { bystander.terminate() }

        let model = ShellModel()
        let session = model.exportSession
        let started = Date()
        var pidSeen: Int32 = 0
        let report: ExportSession.Report = await withCheckedContinuation { cont in
            model.exportPDFExact(listURL: list, tool: tool, destination: .recordingCurrentDisk(out)) { cont.resume(returning: $0) }
            guard case .running(let pid, let at) = session.state else { return XCTFail("not running: \(session.state)") }
            pidSeen = pid
            XCTAssertGreaterThan(pid, 0)
            XCTAssertLessThan(abs(at.timeIntervalSinceNow), 5)
            XCTAssertTrue(self.isAlive(pid), "tool pid \(pid) should be blocked on the FIFO")
            XCTAssertEqual(model.captureNote, "Exporting exact PDF…")
            // A second start on the same session is refused while running.
            session.start(tool: tool, list: list, destination: .recordingCurrentDisk(out), fontDirs: []) { second in
                XCTAssertEqual(second.state, .failed(reason: "an export is already running"))
            }
            XCTAssertTrue(session.state.isRunning)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { model.cancelExactExport() }
        }
        let elapsed = Date().timeIntervalSince(started)
        XCTAssertEqual(report.state, .cancelled)
        XCTAssertEqual(session.state, .cancelled)
        XCTAssertNil(report.exitCode)
        XCTAssertEqual(model.captureNote, "Exact export cancelled; nothing was written to cancelled.pdf.")
        XCTAssertFalse(isAlive(pidSeen), "tool pid \(pidSeen) still alive after cancel")
        XCTAssertTrue(bystander.isRunning, "cancel must not signal unrelated processes")
        XCTAssertEqual(Snapshot(out), before, "existing destination must be byte- and mtime-identical")
        XCTAssertEqual(leftoverTemps(), [])
        assertElapsed(elapsed, under: 10, "cancel")
        // cancel() after completion is a no-op.
        session.cancel()
        XCTAssertEqual(session.state, .cancelled)
    }

    // MARK: (b) tool refusal / exit ≠ 0

    func testRefusalLeavesTheExistingFileUntouched() async throws {
        let tool = try requireTool()
        let bad = dir.appendingPathComponent("bad.json")
        try Data("{}".utf8).write(to: bad)
        let out = dir.appendingPathComponent("refused.pdf")
        let before = try plantExisting(out)
        let model = ShellModel()
        let report: ExportSession.Report = await withCheckedContinuation { cont in
            model.exportPDFExact(listURL: bad, tool: tool, destination: .recordingCurrentDisk(out)) { cont.resume(returning: $0) }
        }
        guard case .failed(let reason) = report.state else { return XCTFail("\(report.state)") }
        XCTAssertTrue(reason.hasPrefix("Exact export refused (exit "), reason)
        XCTAssertNotEqual(report.exitCode, 0)
        XCTAssertNotNil(report.exitCode)
        XCTAssertFalse(reason.hasSuffix("no message"), "the tool's refusal text must be surfaced: \(reason)")
        XCTAssertEqual(model.captureNote, reason)
        XCTAssertNil(report.conflict)
        XCTAssertEqual(Snapshot(out), before)
        XCTAssertEqual(leftoverTemps(), [])
        // Fresh destination: still nothing is created.
        let fresh = dir.appendingPathComponent("fresh-refused.pdf")
        let r2: ExportSession.Report = await withCheckedContinuation { cont in
            model.exportPDFExact(listURL: bad, tool: tool, destination: .recordingCurrentDisk(fresh)) { cont.resume(returning: $0) }
        }
        XCTAssertEqual(r2.exitCode, report.exitCode)
        XCTAssertFalse(FileManager.default.fileExists(atPath: fresh.path))
        XCTAssertEqual(leftoverTemps(), [])
    }

    // MARK: (c) overwrite conflict

    func testDestinationChangedSinceChosenIsRefusedBeforeLaunch() async throws {
        let out = dir.appendingPathComponent("conflict.pdf")
        _ = try plantExisting(out, bytes: "first\n")
        let chosen = ExportSession.Destination.recordingCurrentDisk(out)
        XCTAssertEqual(chosen.expectedDiskSHA256, ExportSession.diskSHA256(out))
        let after = try plantExisting(out, bytes: "changed by someone else\n")
        // A tool that cannot be launched: if the session tried, the reason would say so.
        let noTool = dir.appendingPathComponent("no-such-tool")
        let model = ShellModel()
        let report: ExportSession.Report = await withCheckedContinuation { cont in
            model.exportPDFExact(listURL: Self.fixtures.appendingPathComponent("display-list-v2-text.json"), tool: noTool, destination: chosen) { cont.resume(returning: $0) }
        }
        let conflict = try XCTUnwrap(report.conflict)
        XCTAssertEqual(conflict.kind, .modifiedExternally)
        XCTAssertEqual(conflict.ours, chosen.expectedDiskSHA256)
        XCTAssertEqual(conflict.theirs, after.sha256)
        XCTAssertFalse(conflict.viaHelper)
        XCTAssertEqual(report.state, .failed(reason: conflict.exportSummary))
        XCTAssertEqual(model.exportSession.conflict, conflict)
        XCTAssertTrue(conflict.exportSummary.contains("conflict.pdf was modified on disk since you chose it"), conflict.exportSummary)
        XCTAssertTrue(conflict.exportSummary.contains("the existing file is kept"), conflict.exportSummary)
        XCTAssertEqual(model.captureNote, conflict.exportSummary)
        XCTAssertEqual(Snapshot(out), after, "the newer file on disk is kept")
        XCTAssertEqual(leftoverTemps(), [])

        // Appeared since chosen (chosen when absent) and deleted since chosen.
        let appeared = dir.appendingPathComponent("appeared.pdf")
        let chosenAbsent = ExportSession.Destination.recordingCurrentDisk(appeared)
        XCTAssertNil(chosenAbsent.expectedDiskSHA256)
        let planted = try plantExisting(appeared)
        let r2 = await run(model.exportSession, tool: noTool, list: out, destination: chosenAbsent)
        XCTAssertEqual(r2.conflict?.kind, .alreadyExists)
        XCTAssertEqual(Snapshot(appeared), planted)
        let gone = dir.appendingPathComponent("gone.pdf")
        _ = try plantExisting(gone)
        let chosenPresent = ExportSession.Destination.recordingCurrentDisk(gone)
        try FileManager.default.removeItem(at: gone)
        let r3 = await run(model.exportSession, tool: noTool, list: out, destination: chosenPresent)
        XCTAssertEqual(r3.conflict?.kind, .deletedExternally)
        XCTAssertFalse(FileManager.default.fileExists(atPath: gone.path), "a refusal never creates the file")
        // Unchanged destination with a launch failure: the conflict check passed, the launch is what failed.
        let same = dir.appendingPathComponent("same.pdf")
        _ = try plantExisting(same)
        let r4 = await run(model.exportSession, tool: noTool, list: out, destination: .recordingCurrentDisk(same))
        XCTAssertNil(r4.conflict)
        guard case .failed(let why) = r4.state else { return XCTFail("\(r4.state)") }
        XCTAssertTrue(why.hasPrefix("could not launch no-such-tool"), why)
    }

    func testDestinationChangedWhileRenderingIsRefusedAndTempRemoved() async throws {
        // A scripted "tool" that produces an output AND tampers with the
        // destination meanwhile (as another writer would).
        let out = dir.appendingPathComponent("tampered.pdf")
        _ = try plantExisting(out, bytes: "first\n")
        let script = dir.appendingPathComponent("tamper.sh")
        try """
        #!/bin/sh
        # args: from-v2 LIST --out TEMP [--font-dir …]
        printf '%%PDF-1.4 rendered\\n%%%%EOF\\n' > "$4"
        printf 'tampered meanwhile\\n' > '\(out.path)'
        exit 0
        """.write(to: script, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: script.path)
        let chosen = ExportSession.Destination.recordingCurrentDisk(out)
        let session = ExportSession()
        let report = await run(session, tool: script, list: out, destination: chosen, fontDirs: [])
        XCTAssertEqual(report.exitCode, 0)
        XCTAssertEqual(report.conflict?.kind, .modifiedDuringSave)
        XCTAssertTrue(report.conflict?.exportSummary.contains("changed while the PDF was being produced") == true, "\(report.state)")
        XCTAssertEqual(report.state, .failed(reason: report.conflict!.exportSummary))
        XCTAssertEqual(try String(contentsOf: out, encoding: .utf8), "tampered meanwhile\n", "the other writer's file is kept, not overwritten by the render")
        XCTAssertEqual(leftoverTemps(), [])
    }

    // MARK: (d) timeout

    func testTimeoutTerminatesTheToolAndLeavesTheDestinationUntouched() async throws {
        let tool = try requireTool()
        let list = try blockingList()
        let out = dir.appendingPathComponent("timeout.pdf")
        let before = try plantExisting(out)
        let model = ShellModel()
        let started = Date()
        var pidSeen: Int32 = 0
        let report: ExportSession.Report = await withCheckedContinuation { cont in
            model.exportPDFExact(listURL: list, tool: tool, destination: .recordingCurrentDisk(out), timeout: 1) { cont.resume(returning: $0) }
            if case .running(let pid, _) = model.exportSession.state { pidSeen = pid }
        }
        let elapsed = Date().timeIntervalSince(started)
        guard case .failed(let reason) = report.state else { return XCTFail("\(report.state)") }
        XCTAssertTrue(reason.contains("did not finish within 1 s; terminated"), reason)
        XCTAssertTrue(reason.contains("Nothing was written to timeout.pdf"), reason)
        XCTAssertTrue(model.captureNote?.hasPrefix("Exact export failed to run: ") == true, model.captureNote ?? "")
        XCTAssertNil(report.exitCode)
        XCTAssertNil(report.conflict)
        XCTAssertGreaterThan(pidSeen, 0)
        XCTAssertFalse(isAlive(pidSeen), "tool pid \(pidSeen) still alive after timeout")
        XCTAssertEqual(Snapshot(out), before)
        XCTAssertEqual(leftoverTemps(), [])
        XCTAssertGreaterThanOrEqual(elapsed, 1)
        assertElapsed(elapsed, under: 12, "timeout path")
    }

    // MARK: (e) success

    func testSuccessReplacesAtomicallyAndReportsTheFileBytesAndSHA() async throws {
        let tool = try requireTool()
        let list = Self.fixtures.appendingPathComponent("display-list-v2-text.json")
        let out = dir.appendingPathComponent("success.pdf")
        let before = try plantExisting(out)
        let model = ShellModel()
        let report: ExportSession.Report = await withCheckedContinuation { cont in
            model.exportPDFExact(listURL: list, tool: tool, destination: .recordingCurrentDisk(out)) { cont.resume(returning: $0) }
        }
        guard case .succeeded(let bytes, let sha) = report.state else { return XCTFail("\(report.state) \(report.stderr)") }
        XCTAssertEqual(report.exitCode, 0)
        let onDisk = Snapshot(out)
        XCTAssertEqual(onDisk.size, bytes)
        XCTAssertEqual(onDisk.sha256, sha)
        XCTAssertNotEqual(onDisk, before)
        let data = try Data(contentsOf: out)
        XCTAssertTrue(data.starts(with: Array("%PDF-".utf8)))
        XCTAssertGreaterThan(bytes, 1000)
        XCTAssertEqual(model.captureNote, "Exported exact PDF (\(bytes) bytes, sha256 \(sha.prefix(12))) to \(out.path)")
        XCTAssertEqual(model.exportSession.state, report.state)
        XCTAssertNil(model.exportSession.conflict)
        XCTAssertEqual(leftoverTemps(), [], "the sibling temp file is renamed, never left behind")
        // The pre-session entry point (ExactPDFExportTests' shape) reports the same bytes.
        let out2 = dir.appendingPathComponent("success-2.pdf")
        let outcome: ExactPDFExport.Outcome? = await withCheckedContinuation { cont in
            model.exportPDFExact(listURL: list, tool: tool, to: out2) { cont.resume(returning: $0) }
        }
        XCTAssertEqual(outcome?.succeeded, true)
        XCTAssertEqual(outcome?.bytes, bytes)
        XCTAssertEqual(ExportSession.diskSHA256(out2), sha)
    }
}
