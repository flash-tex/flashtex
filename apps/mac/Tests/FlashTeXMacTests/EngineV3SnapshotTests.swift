import Foundation
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
@testable import FlashTeXMac

/// Instant reopen's stored pages: saved per project, used only while every
/// document still hashes the same and every other input file is unchanged,
/// stale until the compile sends each page, kept under the disk budget.
final class EngineV3SnapshotTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-snapshots-\(getpid())")
    override func setUp() { setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1); try? FileManager.default.removeItem(at: Self.cache) }
    override func tearDown() { unsetenv("FLASHTEX_V3_CACHE"); try? FileManager.default.removeItem(at: Self.cache) }

    func fixture() throws -> DL3Document {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("FlashTeXDisplayListV3Tests/Fixtures/beamer-overlays.dl3")
        return try DL3Document(frames: Array(try Data(contentsOf: url)))
    }

    /// A project folder: main.tex (the editor's), a chapter, a bibliography, a figure.
    func projectFolder() throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-snapproj-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir.appendingPathComponent("figs"), withIntermediateDirectories: true)
        try "\\documentclass{beamer}\n...".write(to: dir.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        try "Chapter one.".write(to: dir.appendingPathComponent("chapter1.tex"), atomically: true, encoding: .utf8)
        try "@book{a, title={A}}".write(to: dir.appendingPathComponent("refs.bib"), atomically: true, encoding: .utf8)
        try Data([0x89, 0x50, 0x4e, 0x47]).write(to: dir.appendingPathComponent("figs/plot.png"))
        try "not an input".write(to: dir.appendingPathComponent("notes.md"), atomically: true, encoding: .utf8)
        return dir
    }

    let docs = [(path: "main.tex", text: "\\documentclass{beamer}\n...")]

    /// Saves a two-image snapshot of `root` as the session would (inputs without the editor's documents).
    @discardableResult
    func save(_ key: String, root: URL, doc: DL3Document) throws -> [String: String] {
        let pages = doc.orderedPages
        let sizes = pages.map { CGSize(width: $0.widthPt, height: $0.heightPt) }
        let hashes = EngineV3Snapshot.hashes(docs)
        let inputs = EngineV3Snapshot.others(try XCTUnwrap(EngineV3Snapshot.inputs(root: root)), documents: hashes.keys)
        EngineV3Snapshot.save(projectKey: key, main: "main.tex", documents: hashes, inputs: inputs, sizes: sizes,
                              pages: [0: pages[0], 1: pages[1]], forms: doc.forms, pixelsPerPoint: 1.5, dark: false)
        return inputs
    }

    /// Rewrites a file outside the app, with a later modification time.
    func touch(_ url: URL, _ text: String) throws {
        let before = try FileManager.default.attributesOfItem(atPath: url.path)[.modificationDate] as? Date ?? Date()
        try text.write(to: url, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.modificationDate: before.addingTimeInterval(5)], ofItemAtPath: url.path)
    }

    func testInputsListTheProjectsInputFiles() throws {
        let root = try projectFolder()
        defer { try? FileManager.default.removeItem(at: root) }
        let inputs = try XCTUnwrap(EngineV3Snapshot.inputs(root: root))
        XCTAssertEqual(Set(inputs.keys), ["main.tex", "chapter1.tex", "refs.bib", "figs/plot.png"])
        XCTAssertEqual(Set(EngineV3Snapshot.others(inputs, documents: ["main.tex"]).keys), ["chapter1.tex", "refs.bib", "figs/plot.png"])
    }

    func testSaveLoadAndInvalidate() throws {
        let doc = try fixture()
        let pages = doc.orderedPages
        let root = try projectFolder()
        defer { try? FileManager.default.removeItem(at: root) }
        let inputs = try save("p1", root: root, doc: doc)
        XCTAssertEqual(Set(inputs.keys), ["chapter1.tex", "refs.bib", "figs/plot.png"])
        let (s, dir) = try XCTUnwrap(EngineV3Snapshot.load(projectKey: "p1", root: root, documents: docs))
        XCTAssertEqual(s.pages.count, pages.count)
        XCTAssertEqual(s.pages.compactMap(\.image).count, 2)
        let img = try XCTUnwrap(EngineV3Snapshot.image(dir.appendingPathComponent(try XCTUnwrap(s.pages[0].image))))
        XCTAssertEqual(img.width, Int((pages[0].widthPt * 1.5).rounded(.up)))
        // An edit outside the app: not used.
        XCTAssertNil(EngineV3Snapshot.load(projectKey: "p1", root: root, documents: [(path: "main.tex", text: "changed")]))
        XCTAssertNil(EngineV3Snapshot.load(projectKey: "p2", root: root, documents: docs))
        // A new input file (a figure added): not used.
        try Data([1]).write(to: root.appendingPathComponent("figs/new.pdf"))
        XCTAssertNil(EngineV3Snapshot.load(projectKey: "p1", root: root, documents: docs))
        try FileManager.default.removeItem(at: root.appendingPathComponent("figs/new.pdf"))
        XCTAssertNotNil(EngineV3Snapshot.load(projectKey: "p1", root: root, documents: docs))
        // A file that is not an input: still used.
        try touch(root.appendingPathComponent("notes.md"), "changed notes")
        XCTAssertNotNil(EngineV3Snapshot.load(projectKey: "p1", root: root, documents: docs))
        // A later save keeps only its own page images.
        let sizes = pages.map { CGSize(width: $0.widthPt, height: $0.heightPt) }
        EngineV3Snapshot.save(projectKey: "p1", main: "main.tex", documents: EngineV3Snapshot.hashes(docs), inputs: inputs, sizes: sizes,
                              pages: [1: pages[1]], forms: doc.forms, pixelsPerPoint: 1.5, dark: false)
        let names = try FileManager.default.contentsOfDirectory(atPath: dir.path).sorted()
        XCTAssertEqual(names, ["manifest.json", "page-1.png"])
    }

    /// A chapter the editor has not opened, changed outside the app: the snapshot is not used.
    func testChapterChangedOutsideTheApp() throws {
        let root = try projectFolder()
        defer { try? FileManager.default.removeItem(at: root) }
        try save("p1", root: root, doc: try fixture())
        XCTAssertNotNil(EngineV3Snapshot.load(projectKey: "p1", root: root, documents: docs))
        try touch(root.appendingPathComponent("chapter1.tex"), "Chapter one, rewritten.")
        XCTAssertNil(EngineV3Snapshot.load(projectKey: "p1", root: root, documents: docs))
    }

    /// The bibliography changed outside the app (same size even): not used.
    func testBibChangedOutsideTheApp() throws {
        let root = try projectFolder()
        defer { try? FileManager.default.removeItem(at: root) }
        try save("p1", root: root, doc: try fixture())
        XCTAssertNotNil(EngineV3Snapshot.load(projectKey: "p1", root: root, documents: docs))
        try touch(root.appendingPathComponent("refs.bib"), "@book{b, title={B}}")
        XCTAssertNil(EngineV3Snapshot.load(projectKey: "p1", root: root, documents: docs))
    }

    /// Between STARTED (a new host document: `keep:false`) and the first
    /// PAGE, every stored page stays stale; the host's "current" ranges and
    /// a DONE that did not send a page never make a stored page look current.
    @MainActor
    func testStoredPagesStayStaleUntilTheirPageArrives() throws {
        let doc = try fixture()
        let pages = doc.orderedPages
        let root = try projectFolder()
        defer { try? FileManager.default.removeItem(at: root) }
        try save("p1", root: root, doc: doc)
        let found = try XCTUnwrap(EngineV3Snapshot.load(projectKey: "p1", root: root, documents: docs))
        let n = pages.count
        XCTAssertGreaterThanOrEqual(n, 2)

        let s = EngineV3Session()
        s.showStored(found)
        XCTAssertEqual(s.pageCount, n)
        XCTAssertEqual(s.staleCount, n)
        s.handle(.started(.object(["keep": .bool(false)])))
        XCTAssertEqual(s.staleCount, n, "STARTED keep:false before the first PAGE")
        XCTAssertEqual(s.stale, Set(0 ..< n))
        s.handle(.pages(.object(["count": .int(Int64(n)), "current": .array([.array([.int(0), .int(Int64(n - 1))])])])))
        XCTAssertEqual(s.staleCount, n, "the host's current ranges cover only pages it sent")

        s.handle(.page(pages[0], compileID: 1, timing: .init(), image: nil))
        XCTAssertEqual(s.stale, Set(1 ..< n), "the real page 0 is current, the stored ones stay stale")
        s.handle(.done(.object(["status": .string("failed"), "pages": .int(Int64(n))]), compileID: 1))
        XCTAssertEqual(s.stale, Set(1 ..< n), "a DONE without those pages keeps them stale")
        XCTAssertNotNil(s.snapshot)

        s.handle(.started(.object(["keep": .bool(true)])))
        for i in 1 ..< n { s.handle(.page(pages[i], compileID: 2, timing: .init(), image: nil)) }
        XCTAssertEqual(s.stale, [])
        s.handle(.done(.object(["status": .string("ok"), "pages": .int(Int64(n))]), compileID: 2))
        XCTAssertEqual(s.staleCount, 0)
        XCTAssertNil(s.snapshot, "every stored page was replaced")
    }

    /// The folder check runs off the main thread after the pages are on
    /// screen (stale); a changed input drops them.
    @MainActor
    func testStoredPagesDroppedWhenAnInputChanged() async throws {
        let doc = try fixture()
        let root = try projectFolder()
        defer { try? FileManager.default.removeItem(at: root) }
        try save("p1", root: root, doc: doc)
        let found = try XCTUnwrap(EngineV3Snapshot.loadDocuments(projectKey: "p1", documents: docs))
        let s = EngineV3Session()
        s.showStored(found)
        try touch(root.appendingPathComponent("chapter1.tex"), "Chapter one, changed outside the app.")
        s.validateStored(found, root: root)
        XCTAssertNotNil(s.snapshot, "shown at once, checked later")
        XCTAssertEqual(s.staleCount, s.pageCount)
        let start = Date()
        while s.snapshot != nil, Date().timeIntervalSince(start) < 10 { try await Task.sleep(nanoseconds: 10_000_000) }
        XCTAssertNil(s.snapshot, "dropped")
        XCTAssertEqual(s.pageCount, 0)
        XCTAssertEqual(s.staleCount, 0)

        // Unchanged inputs: kept.
        try save("p1", root: root, doc: doc)
        let again = try XCTUnwrap(EngineV3Snapshot.loadDocuments(projectKey: "p1", documents: docs))
        let t = EngineV3Session()
        t.showStored(again)
        t.validateStored(again, root: root)
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertNotNil(t.snapshot)
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
        // Between the compile's STARTED and its first PAGE, the stored pages are still stale.
        var staleAtStarted: Int?
        var sawPage = false
        var staleOtherAtFirstPage: Bool?
        b.engineV3.afterEvent = { [weak session = b.engineV3] out in
            switch out {
            case .started: if !sawPage, staleAtStarted == nil { staleAtStarted = session?.staleCount }
            case .page(let p, _, _, _):
                if !sawPage, let session {
                    // The other stored page is still marked stale.
                    let other = p.page.index == 0 ? 1 : 0
                    staleOtherAtFirstPage = session.stale.contains(other)
                }
                sawPage = true
            default: break
            }
        }
        b.engineV3.start(model: b)
        defer { b.engineV3.stop() }
        try await wait { b.engineV3.statusNote.hasPrefix("ok") && b.engineV3.staleCount == 0 }
        XCTAssertNil(b.engineV3.snapshot, "the compile's pages replaced the stored ones")
        XCTAssertEqual(staleAtStarted, 2, "STARTED before the first PAGE: the stored pages stay stale")
        XCTAssertEqual(staleOtherAtFirstPage, true, "after the first PAGE the other stored page is still stale")
    }
}
