import Foundation
import XCTest
@testable import FlashTeXMac

/// Forward and reverse search in the engine-v3 preview, across a
/// multi-file project (`\input`), and the caret follower's target.
@MainActor
final class EngineV3SearchTests: XCTestCase {
    /// Environment set for a test and put back after it (never just unset).
    private var env = EnvironmentOverride()
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-search-\(getpid())")
    override func setUp() {
        OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path)
        CaretFollow.enabledOverride = true
        CaretFollow.debounceOverride = 0
    }
    override func tearDown() {
        env.restore()
        CaretFollow.enabledOverride = nil
        CaretFollow.debounceOverride = nil
    }

    func waitUntil(_ what: String, timeout: TimeInterval = 90, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    func testForwardAndReverseAcrossAnInput() async throws {
        guard EngineV3.locateHost() != nil else { throw XCTSkip("no flashtex-host built") }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-search-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let main = "\\documentclass{article}\n\\begin{document}\nAlpha beta gamma.\n\n\\input{chap}\n\\end{document}\n"
        let chap = "First chapter line.\nSecond chapter line with delta.\n"
        try main.write(to: dir.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        try chap.write(to: dir.appendingPathComponent("chap.tex"), atomically: true, encoding: .utf8)
        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: dir.appendingPathComponent("main.tex"), dirty: .discard), .opened)
        model.engineV3Enabled = true
        model.engineV3.start(model: model)
        defer { model.engineV3.stop() }
        let s = model.engineV3
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && s.pageCount == 1 }

        // Forward: the caret on "gamma" (main.tex line 3) → the glyph at that column.
        let gammaByte = main.utf8.distance(from: main.startIndex, to: main.range(of: "gamma")!.lowerBound)
        let place = try XCTUnwrap(s.place(path: "main.tex", byte: gammaByte, in: main))
        XCTAssertEqual(place.page, 0)
        // Reverse at that glyph: main.tex line 3, the same column.
        let back = try XCTUnwrap(s.source(page: 0, at: CGPoint(x: place.rect.midX, y: place.rect.midY)))
        XCTAssertEqual(back.path, "main.tex"); XCTAssertEqual(back.line, 3)
        XCTAssertEqual(back.col, "Alpha beta ".utf8.count)

        // A line of the \input file: forward from chap.tex line 2, reverse opens chap.tex.
        let chapPlace = try XCTUnwrap(s.place(path: "chap.tex", line: 2, col: nil))
        XCTAssertGreaterThan(chapPlace.rect.minY, place.rect.maxY, "the chapter's second line is below main's line 3")
        let hit = try XCTUnwrap(s.source(page: 0, at: CGPoint(x: chapPlace.rect.minX + 2, y: chapPlace.rect.midY)))
        XCTAssertEqual(hit.path, "chap.tex"); XCTAssertEqual(hit.line, 2)
        model.navigateEngineV3(path: hit.path, line: hit.line, col: hit.col)
        try await waitUntil("chap.tex opened and selected") { model.activePath == "chap.tex" && model.selection?.path == "chap.tex" }
        let sel = try XCTUnwrap(model.selection)
        let lineStart = ("First chapter line.\n" as NSString).length
        XCTAssertEqual(sel.nsRange.location, lineStart + (hit.col ?? 0))

        // The caret follower aims at the caret's glyph after an edit.
        model.activePath = "main.tex"
        model.caretUTF16 = (main as NSString).range(of: "gamma").location
        model.updateActiveText(main.replacingOccurrences(of: "Alpha", with: "Alpha!"))
        try await waitUntil("the follower's request") { model.caretFollow.request?.target.page == 0 }
    }
}
