import Foundation
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
import FlashTeXProtocol
@testable import FlashTeXMac

/// App-parity rows the new engine's app side carries without a real
/// `flashtex-host` or TeX Live, so they run on the hosted mac-app job
/// (docs/evidence/app-parity-2026-10-05/README.md):
/// - A13: a host that keeps dying is restarted at most three times in a
///   minute, then the session stops with "stopped repeatedly", the pages
///   kept on screen stale (`EngineV3Session.restart`);
/// - A14: ⌘B after that limit starts the host again with a fresh budget
///   (`compileNow`);
/// - D2: Export PDF under v3 refuses a destination changed since it was
///   chosen, before the engine is asked and again when publishing, and keeps
///   the newer file (`ExportSession.startExternal` / `finishExternal`);
/// - B3: an overfull box the host reports (DIAG `tex/overfull-hbox`) is a
///   warning row on its source line in the Problems panel.
@MainActor
final class EngineV3AppParityTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-app-parity-\(getpid())")
    private var env = EnvironmentOverride()
    private var dir: URL!

    override func setUpWithError() throws {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.cache.path)
        env.set("FLASHTEX_HOST", "none") // a test that wants a (fake) host sets its own
        // No bundle (never a lock file of this Mac's): nothing asks before the host starts.
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
        env.set("FLASHTEX_BUNDLE_DIGEST", "")
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-app-parity-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        env.restore()
        try? FileManager.default.removeItem(at: dir)
        try? FileManager.default.removeItem(at: Self.cache)
    }

    static let document = "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n"

    private func v3Model(_ text: String? = nil) -> ShellModel {
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: text ?? Self.document, named: "main.tex")
        model.engineV3Enabled = true
        return model
    }

    /// Polls `cond` on the main actor until it holds or `timeout` passes.
    private func waitUntil(_ timeout: TimeInterval, _ cond: @MainActor () -> Bool) async throws -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while !cond() {
            if Date() > deadline { return false }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        return true
    }

    private func failedRepeatedly(_ s: EngineV3Session) -> Bool {
        if case .failed(let why) = s.phase { return why.contains("stopped repeatedly") }
        return false
    }

    private func pagesFixture() throws -> [DL3PreparedPage] {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("FlashTeXDisplayListV3Tests/Fixtures/beamer-overlays.dl3")
        return try DL3Document(frames: Array(Data(contentsOf: url))).orderedPages
    }

    // MARK: A13 crash limit, A14 retry

    /// The fake host (`/usr/bin/false`) exits as soon as it starts. The first
    /// start and three restarts within the minute are allowed; the fourth exit
    /// stops the session ("stopped repeatedly"), the pages kept on screen and
    /// stale. ⌘B then starts it again with a fresh budget: four more starts.
    func testAHostThatKeepsDyingStopsAfterThreeRestartsAndCompileRetries() async throws {
        let fake = "/usr/bin/false"
        try XCTSkipUnless(FileManager.default.isExecutableFile(atPath: fake), "no \(fake)")
        // `killStaleHosts` only ever signals a process whose path ends in /flashtex-host.
        XCTAssertFalse(fake.hasSuffix("/flashtex-host"))
        env.set("FLASHTEX_HOST", fake)
        XCTAssertEqual(EngineV3.locateHost()?.path, fake)
        let pages = try pagesFixture()
        XCTAssertGreaterThanOrEqual(pages.count, 2)

        let model = v3Model()
        defer { model.engineV3Enabled = false }
        let s = model.engineV3
        XCTAssertEqual(s.hostStarts, 1, "enabling v3 starts the host")
        // Two pages on screen before the host's exit reaches the main thread.
        s.handle(.page(pages[0], compileID: 1, timing: .init(), image: nil))
        s.handle(.page(pages[1], compileID: 1, timing: .init(), image: nil))
        XCTAssertEqual(s.pageCount, 2)
        XCTAssertEqual(s.staleCount, 0)

        let stopped = try await waitUntil(20) { self.failedRepeatedly(s) }
        XCTAssertTrue(stopped, "phase \(s.phase), \(s.hostStarts) starts")
        XCTAssertEqual(s.hostStarts, 4, "the first start and three restarts")
        XCTAssertNil(s.hostPID)
        XCTAssertEqual(s.pageCount, 2, "the pages stay on screen")
        XCTAssertEqual(s.staleCount, 2, "every kept page is stale")
        model.flushChrome()
        XCTAssertTrue(model.chrome.staleText?.contains("stopped repeatedly") ?? false, model.chrome.staleText ?? "nil")
        XCTAssertTrue(model.chrome.staleHighlighted)
        // No more starts while it is stopped.
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertEqual(s.hostStarts, 4)

        // A14: ⌘B is a fresh budget.
        s.compileNow(model: model)
        XCTAssertEqual(s.hostStarts, 5, "⌘B starts the host at once")
        let stoppedAgain = try await waitUntil(20) { s.hostStarts == 8 && self.failedRepeatedly(s) }
        XCTAssertTrue(stoppedAgain, "phase \(s.phase), \(s.hostStarts) starts")
        XCTAssertEqual(s.hostStarts, 8, "four more starts: the budget was fresh, not the old one's remainder")
        XCTAssertEqual(s.staleCount, 2)
    }

    // MARK: D2 export overwrite conflict

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
    private func plant(_ url: URL, _ bytes: String) throws -> Snapshot {
        try Data(bytes.utf8).write(to: url)
        try FileManager.default.setAttributes([.modificationDate: Date(timeIntervalSince1970: 1_600_000_000)], ofItemAtPath: url.path)
        return Snapshot(url)
    }

    private func leftoverTemps() -> [String] {
        ((try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? []).filter { $0.contains(".flashtex-export-") }
    }

    func testV3ExportRefusesADestinationChangedSinceItWasChosen() async throws {
        let model = v3Model()
        defer { model.engineV3Enabled = false }
        XCTAssertTrue(model.engineV3Enabled)

        // Before launch: changed since chosen. Refused before the engine is asked.
        let out = dir.appendingPathComponent("conflict.pdf")
        _ = try plant(out, "first\n")
        let chosen = ExportSession.Destination.recordingCurrentDisk(out)
        let after = try plant(out, "changed by someone else\n")
        let report: ExportSession.Report = await withCheckedContinuation { cont in
            model.exportPDFEngineV3(to: chosen) { cont.resume(returning: $0) }
        }
        let conflict = try XCTUnwrap(report.conflict, "\(report.state)")
        XCTAssertEqual(conflict.kind, .modifiedExternally)
        XCTAssertEqual(conflict.ours, chosen.expectedDiskSHA256)
        XCTAssertEqual(conflict.theirs, after.sha256)
        XCTAssertEqual(report.state, .failed(reason: conflict.exportSummary))
        XCTAssertTrue(conflict.exportSummary.contains("the existing file is kept"), conflict.exportSummary)
        XCTAssertEqual(model.captureNote, conflict.exportSummary)
        XCTAssertFalse(model.engineV3.exporting, "the engine was never asked for the PDF")
        XCTAssertEqual(Snapshot(out), after, "the newer file on disk is kept")
        XCTAssertEqual(leftoverTemps(), [])

        // An unchanged destination passes the check; then the engine's own
        // refusal (no host here) is what fails, and nothing is written.
        let same = dir.appendingPathComponent("same.pdf")
        let planted = try plant(same, "%PDF-1.4 existing\n%%EOF\n")
        let r2: ExportSession.Report = await withCheckedContinuation { cont in
            model.exportPDFEngineV3(to: .recordingCurrentDisk(same)) { cont.resume(returning: $0) }
        }
        XCTAssertNil(r2.conflict)
        guard case .failed(let why) = r2.state else { return XCTFail("\(r2.state)") }
        XCTAssertTrue(why.contains("engine-v3 preview"), why)
        XCTAssertEqual(Snapshot(same), planted)
        XCTAssertEqual(leftoverTemps(), [])

        // On publish: the engine's PDF arrives after the destination changed.
        // Refused, the other writer's file kept, the temp file removed.
        let late = dir.appendingPathComponent("late.pdf")
        _ = try plant(late, "first\n")
        var r3: ExportSession.Report?
        XCTAssertTrue(model.exportSession.startExternal(destination: .recordingCurrentDisk(late), cancel: {}) { r3 = $0 })
        let tampered = try plant(late, "tampered meanwhile\n")
        model.exportSession.finishExternal(pdf: Data("%PDF-1.5\n%%EOF\n".utf8))
        let report3 = try XCTUnwrap(r3)
        XCTAssertEqual(report3.conflict?.kind, .modifiedDuringSave)
        XCTAssertEqual(report3.state, .failed(reason: report3.conflict!.exportSummary))
        XCTAssertEqual(Snapshot(late), tampered, "the other writer's file is kept")
        XCTAssertEqual(leftoverTemps(), [])
    }

    // MARK: B3 box warnings

    /// STARTED, the host's DIAG for an overfull box (as `host/diag.rs` builds
    /// it: severity warning, code `tex/overfull-hbox`, the box's first and
    /// last characters), DONE: a warning row on the box's line. With no
    /// project copy (no host) there is no root to strip, so the DIAG names
    /// the file as TeX opened it.
    func testAnOverfullBoxIsAWarningRowOnItsLine() throws {
        let line3 = "Short words then Antidisestablishmentarianism and more text follows here."
        let text = "\\documentclass{article}\n\\begin{document}\n\(line3)\n\\end{document}\n"
        let model = v3Model(text)
        defer { model.engineV3Enabled = false }
        let s = model.engineV3
        s.handle(.started(.object(["id": .int(1)])))
        let json = #"{"id":1,"seq":0,"severity":"warning","code":"tex/overfull-hbox","origin":"tex","message":"Overfull \\hbox (12.5pt too wide) in paragraph at lines 3--3","detail":"\\OT1/cmr/m/n/10 Short words then","file":"./main.tex","line":3,"col":6,"end":{"file":"./main.tex","line":3,"col":45},"lines":[3,3],"exact":true}"#
        s.handle(.diag(try DL3Diag.decode(Array(json.utf8))))
        s.handle(.done(.object(["id": .int(1), "status": .string("ok"), "pages": .int(1)]), compileID: 1))

        let rows = model.engineV3Diagnostics
        XCTAssertEqual(rows.count, 1, rows.map(\.message).description)
        let row = try XCTUnwrap(rows.first)
        XCTAssertEqual(row.severity, .warning)
        XCTAssertEqual(row.message, "Overfull \\hbox")
        XCTAssertEqual(row.code, "tex/overfull-hbox")
        let source = try XCTUnwrap(row.source)
        XCTAssertEqual(source.path, "main.tex")
        let lineStart = Array(text.utf8).count - Array("\(line3)\n\\end{document}\n".utf8).count
        XCTAssertGreaterThanOrEqual(source.startByte, lineStart)
        XCTAssertLessThanOrEqual(source.endByte, lineStart + line3.utf8.count, "on its own line")
        XCTAssertEqual(text.utf8Slice(source.startByte, source.endByte), "words then Antidisestablishmentarianism",
                       "the box's text, first character to last")
        XCTAssertEqual(row.notes?.first, EngineV3Explain.explanation(code: "tex/overfull-hbox", message: ""))
        XCTAssertEqual(s.warningCount, 1)
        XCTAssertEqual(s.errorCount, 0)
        model.flushChrome()
        XCTAssertEqual(model.chrome.problems.warnings, 1)
        XCTAssertEqual(model.chrome.problems.errors, 0)
    }
}
