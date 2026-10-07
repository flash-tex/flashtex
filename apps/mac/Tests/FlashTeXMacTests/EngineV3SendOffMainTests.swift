import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXMac

/// APP-EDITOR-INSTANT: the COMPILE's encoding and socket write, and the
/// project copy's file, are off the main thread (`EngineV3Session.sendQueue`).
/// Named EngineV3* so the `mac app: engine-v3 host tests` job runs it with a host.
@MainActor
final class EngineV3SendOffMainTests: XCTestCase {
    private var env = EnvironmentOverride()
    override func setUp() async throws {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", FileManager.default.temporaryDirectory.appendingPathComponent("v3-send-\(getpid())").path)
    }
    override func tearDown() async throws { MainThreadProbe.stop(); env.restore() }

    // MARK: the first compile's send

    /// The first COMPILE of a 4.5 MB document carries the whole text as a
    /// buffer, and the project copy's file is written first: both off the
    /// main thread (the send queue). Measured: `compile` on main, from the
    /// edit to the request handed to the queue.
    func testTheFirstCompileOfAFourMegabyteDocumentIsSentOffMain() async throws {
        try EngineV3TestHost.require()
        let text = EditorInstantTests.book(sections: 1080)
        let model = ShellModel()
        model.replaceProject(entryText: text)
        model.engineV3Enabled = true
        model.autoCompile = true
        let s = model.engineV3
        defer { s.stop() }
        MainThreadProbe.start()
        s.start(model: model)
        try await EngineV3TestHost.awaitReady(s)
        let deadline = Date().addingTimeInterval(600)
        while s.doneCount < 1, Date() < deadline { try await Task.sleep(nanoseconds: 50_000_000) }
        XCTAssertGreaterThanOrEqual(s.doneCount, 1, "the first DONE")
        MainThreadProbe.stop()
        let compile = MainThreadProbe.buckets["v3.compile"], send = MainThreadProbe.buckets["v3.send"]
        // What used to run on main for that compile, timed here: encoding the
        // COMPILE with the whole buffer, and writing the copy's file.
        var req = DL3CompileRequest(id: 1, root: "/tmp", main: "main.tex")
        req.buffers = [("main.tex", text)]
        let e0 = MonotonicClock.nowNs()
        let frame = req.json.data()
        let e1 = MonotonicClock.nowNs()
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("editor-instant-\(getpid()).tex")
        try? Data(text.utf8).write(to: tmp)
        let e2 = MonotonicClock.nowNs()
        try? FileManager.default.removeItem(at: tmp)
        let o: [String: Any] = ["document_bytes": text.utf8.count, "status": s.statusNote, "frame_bytes": frame.count,
                                "moved_encode_ms": Double(e1 &- e0) / 1e6, "moved_file_write_ms": Double(e2 &- e1) / 1e6,
                                "compile_ms": (compile?.samples ?? []).map { Double($0) / 1e6 },
                                "send_ms": (send?.samples ?? []).map { Double($0) / 1e6 }]
        if let d = try? JSONSerialization.data(withJSONObject: o, options: [.sortedKeys]) { print("EditorInstantSend: " + String(decoding: d, as: UTF8.self)) }
        let worst = Double(compile?.maxNs ?? 0) / 1e6
        XCTAssertGreaterThan(compile?.count ?? 0, 0, "a compile was sent")
        XCTAssertLessThan(worst, 8, "the first compile's main-thread time (the 4.5 MB buffer is encoded and written off main)")
    }

}
