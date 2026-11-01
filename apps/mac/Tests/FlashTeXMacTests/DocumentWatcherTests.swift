import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// `DocumentWatcher` (lane mac-document-files-2, follow-up 2): a vnode
/// DispatchSource on the open document drives `refreshDiskStatus()` so an
/// external edit becomes the explicit conflict state while the app is
/// frontmost, with no polling. Real temp files; the direct file layer (the
/// status path is the same one `applicationDidBecomeActive` uses). Event
/// delivery is asynchronous, so the assertions wait (bounded) for the
/// coalesced delivery; the test is skipped on a heavily loaded machine
/// rather than timing out spuriously.
@MainActor
final class DocumentWatcherTests: XCTestCase {
    private func requireQuietMachine() throws {
        var avg = [Double](repeating: 0, count: 3)
        if getloadavg(&avg, 3) == 3, avg[0] > 20 { throw XCTSkip("1-min load \(avg[0]) > 20; watcher timing is load-sensitive") }
    }

    private func tempDir() throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-watch-\(UUID().uuidString)").resolvingSymlinksInPath()
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: dir) }
        return dir
    }

    private func settles(_ timeout: TimeInterval = 5, _ cond: () -> Bool) async -> Bool {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { return false }
            try? await Task.sleep(nanoseconds: 20_000_000)
        }
        return true
    }

    func testExternalEditsAndDeletesReachRefreshDiskStatusWithoutPolling() async throws {
        try requireQuietMachine()
        let dir = try tempDir()
        let url = dir.appendingPathComponent("paper.tex")
        try "v1\n".write(to: url, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.detachWorker()
        model.files.policy = .disabled(reason: "test: direct")
        model.documentWatcher.debounce = 0.05
        XCTAssertEqual(model.openTex(at: url), .opened)
        XCTAssertTrue(model.documentWatcher.isWatching, model.documentWatcher.status)
        XCTAssertEqual(model.documentWatcher.url, url)
        XCTAssertNil(model.files.conflict)

        // In-place write by another program (same inode).
        model.updateActiveText("v1 typed\n")
        let handle = try FileHandle(forWritingTo: url)
        try handle.seekToEnd()
        try handle.write(contentsOf: Data("appended by someone else\n".utf8))
        try handle.close()
        let modified = await settles { model.files.conflict?.kind == .modifiedExternally }
        XCTAssertTrue(modified, "no conflict from the watcher: \(model.documentWatcher.status); deliveries \(model.documentWatcher.deliveries)")
        XCTAssertEqual(model.activeText, "v1 typed\n", "the buffer is untouched")
        XCTAssertEqual(model.files.lastDiskState, .modified)
        XCTAssertGreaterThanOrEqual(model.documentWatcher.deliveries, 1)

        // Atomic replace (rename over the path: a new inode) is followed. A
        // delivery only *starts* the (async) status refresh, so wait for the
        // state it produces — the replacement's digest — not for the counter.
        let d0 = model.documentWatcher.deliveries
        let v3 = SourceDigest.sha256Hex("v3 replaced atomically\n")
        try "v3 replaced atomically\n".write(to: url, atomically: true, encoding: .utf8)
        let replaced = await settles { model.documentWatcher.deliveries > d0 && model.files.conflict?.theirs == v3 }
        XCTAssertTrue(replaced, "the rename was not delivered or its status never settled: deliveries \(model.documentWatcher.deliveries) (was \(d0)), " +
                      "theirs \(model.files.conflict?.theirs ?? "nil"); \(model.documentWatcher.status)")
        XCTAssertTrue(model.documentWatcher.isWatching, "re-armed on the replacement inode: \(model.documentWatcher.status)")
        XCTAssertEqual(model.files.conflict?.theirs, v3)
        // The replacement's own later edit is still seen (proves the re-arm).
        let d1 = model.documentWatcher.deliveries
        try "v4 after replace\n".write(to: url, atomically: true, encoding: .utf8)
        let again = await settles { model.documentWatcher.deliveries > d1 && model.files.conflict?.theirs == SourceDigest.sha256Hex("v4 after replace\n") }
        XCTAssertTrue(again, "edit after the replace not delivered (deliveries \(model.documentWatcher.deliveries))")

        // Our own save must not read as an external change; the watch survives it.
        XCTAssertTrue(model.overwriteOnDisk())
        XCTAssertNil(model.files.conflict)
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertNil(model.files.conflict, "own save reported as external: \(model.captureNote ?? "-")")
        XCTAssertTrue(model.documentWatcher.isWatching)
        XCTAssertFalse(model.isDirty)

        // Deletion: a notice (not a blocking conflict); the watcher waits for the path.
        try FileManager.default.removeItem(at: url)
        let deleted = await settles { model.captureNote?.contains("deleted on disk") == true }
        XCTAssertTrue(deleted, model.captureNote ?? "-")
        XCTAssertNil(model.files.conflict)
        XCTAssertFalse(model.documentWatcher.isWatching, model.documentWatcher.status)
        // Save recreates it and re-arms the watch; an external edit afterwards is seen.
        model.updateActiveText("v5 recreated\n")
        XCTAssertTrue(model.saveTex())
        XCTAssertTrue(model.documentWatcher.isWatching, model.documentWatcher.status)
        try await Task.sleep(nanoseconds: 200_000_000)
        XCTAssertNil(model.files.conflict)
        try "v6 external after recreate\n".write(to: url, atomically: true, encoding: .utf8)
        let after = await settles { model.files.conflict?.kind == .modifiedExternally }
        XCTAssertTrue(after)

        // Opening another file moves the watch; the old file is no longer reported.
        let other = dir.appendingPathComponent("other.tex")
        try "other\n".write(to: other, atomically: true, encoding: .utf8)
        XCTAssertEqual(model.openTex(at: other, dirty: .discard), .opened) // the v5 buffer is dirty against v6 on disk
        XCTAssertEqual(model.documentWatcher.url, other)
        XCTAssertNil(model.files.conflict)
        let d2 = model.documentWatcher.deliveries
        try "v7 old file\n".write(to: url, atomically: true, encoding: .utf8)
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertEqual(model.documentWatcher.deliveries, d2, "the old file is not watched any more")
        XCTAssertNil(model.files.conflict)
        model.documentWatcher.stop()
        XCTAssertFalse(model.documentWatcher.isWatching)
        XCTAssertNil(model.documentWatcher.url)
    }

    func testWatcherIsOptionalAndHarmlessWithoutAFile() throws {
        let watcher = DocumentWatcher()
        XCTAssertFalse(watcher.watch(URL(fileURLWithPath: "/nonexistent/flashtex-\(UUID().uuidString).tex")))
        XCTAssertFalse(watcher.isWatching)
        XCTAssertTrue(watcher.status.contains("not watching"), watcher.status)
        XCTAssertFalse(watcher.rearm())
        watcher.stop()
        let model = ShellModel()
        model.detachWorker()
        XCTAssertNil(model.documentURL)
        model.watchOpenDocument()
        XCTAssertFalse(model.documentWatcher.isWatching)
    }
}
