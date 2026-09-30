import Foundation
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
@testable import FlashTeXMac

/// Instant reopen's stored pages: saved per project, used only while every
/// document still hashes the same, kept under the disk budget.
final class EngineV3SnapshotTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-snapshots-\(getpid())")
    override func setUp() { setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1); try? FileManager.default.removeItem(at: Self.cache) }
    override func tearDown() { unsetenv("FLASHTEX_V3_CACHE"); try? FileManager.default.removeItem(at: Self.cache) }

    func fixture() throws -> DL3Document {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("FlashTeXDisplayListV3Tests/Fixtures/beamer-overlays.dl3")
        return try DL3Document(frames: Array(try Data(contentsOf: url)))
    }

    func testSaveLoadAndInvalidate() throws {
        let doc = try fixture()
        let pages = doc.orderedPages
        let docs = [(path: "main.tex", text: "\\documentclass{beamer}\n...")]
        let sizes = pages.map { CGSize(width: $0.widthPt, height: $0.heightPt) }
        EngineV3Snapshot.save(projectKey: "p1", main: "main.tex", documents: EngineV3Snapshot.hashes(docs), sizes: sizes,
                              pages: [0: pages[0], 1: pages[1]], forms: doc.forms, pixelsPerPoint: 1.5, dark: false)
        let (s, dir) = try XCTUnwrap(EngineV3Snapshot.load(projectKey: "p1", documents: docs))
        XCTAssertEqual(s.pages.count, pages.count)
        XCTAssertEqual(s.pages.compactMap(\.image).count, 2)
        let img = try XCTUnwrap(EngineV3Snapshot.image(dir.appendingPathComponent(try XCTUnwrap(s.pages[0].image))))
        XCTAssertEqual(img.width, Int((pages[0].widthPt * 1.5).rounded(.up)))
        // An edit outside the app: not used.
        XCTAssertNil(EngineV3Snapshot.load(projectKey: "p1", documents: [(path: "main.tex", text: "changed")]))
        XCTAssertNil(EngineV3Snapshot.load(projectKey: "p2", documents: docs))
        // A later save keeps only its own page images.
        EngineV3Snapshot.save(projectKey: "p1", main: "main.tex", documents: EngineV3Snapshot.hashes(docs), sizes: sizes,
                              pages: [1: pages[1]], forms: doc.forms, pixelsPerPoint: 1.5, dark: false)
        let names = try FileManager.default.contentsOfDirectory(atPath: dir.path).sorted()
        XCTAssertEqual(names, ["manifest.json", "page-1.png"])
    }

    /// A reopen shows the stored pages synchronously, stale, before any host
    /// is running; the compile then replaces them.
    @MainActor
    func testReopenShowsStoredPagesBeforeTheHost() async throws {
        guard EngineV3.locateHost() != nil else { throw XCTSkip("no flashtex-host built") }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-reopen-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let file = dir.appendingPathComponent("paper.tex")
        try "\\documentclass{article}\n\\begin{document}\nOne.\n\\newpage\nTwo.\n\\end{document}\n".write(to: file, atomically: true, encoding: .utf8)
        let stored = UserDefaults.standard.object(forKey: EngineV3.enabledKey)
        defer { if let stored { UserDefaults.standard.set(stored, forKey: EngineV3.enabledKey) } else { UserDefaults.standard.removeObject(forKey: EngineV3.enabledKey) } }
        func wait(_ cond: @escaping () -> Bool) async throws {
            let start = Date()
            while !cond() { if Date().timeIntervalSince(start) > 60 { XCTFail("timeout"); return }; try await Task.sleep(nanoseconds: 50_000_000) }
        }
        let a = ShellModel()
        XCTAssertEqual(a.openTex(at: file, dirty: .discard), .opened)
        a.engineV3Enabled = true
        a.engineV3.start(model: a)
        try await wait { a.engineV3.statusNote.hasPrefix("ok") && a.engineV3.pageCount == 2 }
        let key = try XCTUnwrap(EngineV3Snapshot.key(for: a))
        try await wait { FileManager.default.fileExists(atPath: EngineV3Snapshot.directory(projectKey: key).appendingPathComponent("manifest.json").path) }
        a.engineV3.stop()

        let b = ShellModel()
        b.engineV3Enabled = true // the preview is on before the open (the app restarted with it on)
        XCTAssertEqual(b.openTex(at: file, dirty: .discard), .opened)
        // Synchronously, in the open's own turn: the stored pages, stale.
        XCTAssertNotNil(b.engineV3.snapshot)
        XCTAssertEqual(b.engineV3.pageCount, 2)
        XCTAssertEqual(b.engineV3.staleCount, 2)
        XCTAssertNotNil(b.engineV3.snapshotImageURL(0))
        b.engineV3.start(model: b)
        defer { b.engineV3.stop() }
        try await wait { b.engineV3.statusNote.hasPrefix("ok") && b.engineV3.staleCount == 0 }
        XCTAssertNil(b.engineV3.snapshot, "the compile's pages replaced the stored ones")
    }
}
