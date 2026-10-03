import AppKit
import SwiftUI
import XCTest
@testable import FlashTeXPreviewV3
import HostedWindows
@testable import FlashTeXMac

/// The caret on the page and the page labels in the engine-v3 preview
/// (DESIGN §10 app parity, gaps C12 and C17): as on the v2 pane, the
/// caret's line has a faint band, its glyph is tinted and a caret bar
/// stands at its column, following the caret; each page says "page N" at
/// its bottom right. The pane tests need a built `flashtex-host` and TeX
/// Live (skipped otherwise).
@MainActor
final class EngineV3CaretMarkTests: XCTestCase {
    struct Host: View {
        let model: ShellModel
        var body: some View {
            // The caret is not an input: the model tells the pages view (scheduleCaretMark).
            EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom)
        }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-caret-tests-\(getpid())")
    private var env = EnvironmentOverride()
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore() }

    func waitUntil(_ what: String, timeout: TimeInterval = 120, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    func testRowsGroupGlyphsByBaseline() {
        func g(_ x: CGFloat, _ y: CGFloat, col: UInt16) -> DL3GlyphRef {
            DL3GlyphRef(span: 1, col: col, origin: CGPoint(x: x, y: y), cell: CGRect(x: x, y: y - 7.5, width: 5, height: 10), ink: .zero)
        }
        let rows = EngineV3CaretMark.rows([g(0, 100, col: 0), g(5, 100.2, col: 1), g(0, 112, col: 2)])
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(rows[0].minX, 0); XCTAssertEqual(rows[0].maxX, 10)
        XCTAssertEqual(rows[0].height, 10.2, accuracy: 1e-9, "one row, 0.2 pt of baseline jitter")
    }

    // MARK: the caret's place in the compiled text (pure)

    /// Typing since the compile: the caret is moved into the compiled text
    /// (unchanged before the edit, shifted after it, held to it inside it).
    func testTheCaretIsMovedIntoTheCompiledText() {
        func map(_ caret: Int, _ current: String, _ compiled: String) -> Int {
            EngineV3CaretPlace.map(caret: caret, current: current as NSString, compiled: compiled as NSString)
        }
        let compiled = "ab\ncd\nef", typed = "ab\ncXYd\nef" // "XY" typed after the c
        XCTAssertEqual(map(1, typed, compiled), 1, "before the edit: unchanged")
        XCTAssertEqual(map(6, typed, compiled), 4, "right after the typing: where it began (c|d)")
        XCTAssertEqual(map(5, typed, compiled), 4, "inside the typed text: held to the edit")
        XCTAssertEqual(map(10, typed, compiled), 8, "after the edit: shifted")
        XCTAssertEqual(map(3, compiled, compiled), 3, "no edit: unchanged")
        // A line break typed above: the caret's line is the compiled text's.
        let before = "a\nb\nc", after = "a\n\nb\nc"
        let at = map(5, after, before)
        XCTAssertEqual(at, 4)
        XCTAssertEqual(EngineV3CaretPlace.LineTable(before as NSString).place(at, in: before as NSString).line, 3,
                       "c is on line 3 of the compiled text (line 4 of the editor's)")
    }

    /// The comparison runs 2,048 units at a time: differences on either side
    /// of a chunk boundary, and texts of different lengths.
    func testCommonPrefixAndSuffixAcrossChunks() {
        let base = String(repeating: "abcdefghij", count: 1_000) // 10,000 units
        for at in [0, 1, 2_047, 2_048, 2_049, 5_000, 9_999] {
            var chars = Array(base.utf16)
            chars[at] = 0x5A // "Z"
            let changed = String(decoding: chars, as: UTF16.self) as NSString
            XCTAssertEqual(EngineV3CaretPlace.commonPrefix(base as NSString, changed), at, "prefix, difference at \(at)")
            XCTAssertEqual(EngineV3CaretPlace.commonSuffix(base as NSString, changed, skip: 0), 10_000 - at - 1, "suffix, difference at \(at)")
        }
        let longer = (base + "tail") as NSString
        XCTAssertEqual(EngineV3CaretPlace.commonPrefix(base as NSString, longer), 10_000)
        XCTAssertEqual(EngineV3CaretPlace.commonSuffix(base as NSString, longer, skip: 10_000), 0)
        XCTAssertEqual(EngineV3CaretPlace.map(caret: 10_004, current: longer, compiled: base as NSString), 10_000)
    }

    /// The line table's (line, byte column) is the one the slow path worked
    /// out (`CaretSync.byteOffset` then `lineAndColumn`), at every caret
    /// place of a text with two- and four-byte characters and empty lines;
    /// inside a surrogate pair it is the pair's start.
    func testTheLineTableMatchesTheByteScan() {
        let text = "a\u{e9}\u{1F600}b\n\n\u{2014}x y\nlast \u{e9}\u{e9}"
        let ns = text as NSString
        let table = EngineV3CaretPlace.LineTable(ns)
        XCTAssertEqual(table.starts, [0, 6, 7, 12])
        for u in 0...ns.length {
            let got = table.place(u, in: ns)
            let byte = try! XCTUnwrap(CaretSync.byteOffset(ofCaretUTF16: u, in: text))
            let want = EngineV3Session.lineAndColumn(byte: byte, in: text)
            XCTAssertEqual(got.line, want.0, "line at \(u)")
            XCTAssertEqual(got.col, want.1, "column at \(u)")
        }
        XCTAssertEqual(table.place(4, in: ns).col, 7, "after a, é and the emoji: 1 + 2 + 4 bytes")
        XCTAssertEqual(table.place(3, in: ns).col, 3, "inside the emoji's surrogate pair: its start")
    }

    static let doc = "\\documentclass{article}\n\\begin{document}\nAlpha beta gamma delta.\n\nSecond paragraph here.\n\\newpage\nPage two.\n\\end{document}\n"

    func pane() async throws -> (ShellModel, EngineV3PagesView, NSWindow) {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: Self.doc, named: "main.tex")
        model.engineV3Enabled = true
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 800), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        model.engineV3.start(model: model)
        try await EngineV3TestHost.awaitReady(model.engineV3)
        let s = model.engineV3
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount == 2 && s.pages[0] != nil }
        window.layoutIfNeeded()
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: 1)
        pages.relayout()
        return (model, pages, window)
    }

    /// The caret in "gamma" marks gamma's line and puts the bar before the
    /// glyph at its column; moving the caret moves the mark; the mark's
    /// layer is drawn over the page at that place.
    func testTheCaretIsMarkedOnThePageAndFollows() async throws {
        let (model, pages, window) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        let gamma = (Self.doc as NSString).range(of: "gamma").location
        model.caretUTF16 = gamma
        pages.setCaret(path: model.activePath, utf16: gamma, stamp: s.contentStamp)
        let mark = try XCTUnwrap(pages.caretMark, "a mark for the caret")
        XCTAssertEqual(mark.page, 0)
        XCTAssertEqual(mark.band.count, 1, "one row: the line is one row on the page")
        // The bar stands at the start of gamma's "g": the glyph forward search puts at that column.
        let place = try XCTUnwrap(s.place(path: "main.tex", byte: Self.doc.utf8.distance(from: Self.doc.startIndex, to: Self.doc.range(of: "gamma")!.lowerBound), in: Self.doc))
        let bar = try XCTUnwrap(mark.bar)
        XCTAssertEqual(bar.minX, place.rect.minX, accuracy: 0.01)
        XCTAssertTrue(mark.band[0].contains(CGPoint(x: bar.minX, y: bar.midY)))
        XCTAssertFalse(pages.caretMarkLayer.isHidden)
        XCTAssertNotNil(pages.caretMarkLayer.bar.path)
        let barInView = pages.convertFromLayer(pages.caretMarkLayer.convert(try XCTUnwrap(pages.caretMarkLayer.bar.path?.boundingBox), to: pages.layer))
        let expected = try XCTUnwrap(pages.viewPoint(page: 0, CGPoint(x: bar.minX, y: bar.midY)))
        XCTAssertTrue(barInView.insetBy(dx: -1, dy: -1).contains(expected), "\(barInView) at \(expected)")
        // The second paragraph: another line, lower on the page.
        let second = (Self.doc as NSString).range(of: "paragraph").location
        pages.setCaret(path: model.activePath, utf16: second, stamp: s.contentStamp)
        let mark2 = try XCTUnwrap(pages.caretMark)
        XCTAssertGreaterThan(mark2.band[0].minY, mark.band[0].maxY)
        // A caret in the preamble maps to nothing: no mark.
        pages.setCaret(path: model.activePath, utf16: 3, stamp: s.contentStamp)
        XCTAssertNil(pages.caretMark)
        XCTAssertTrue(pages.caretMarkLayer.isHidden)
    }

    /// The mark follows the model's caret with no call from SwiftUI (the
    /// model schedules it), and while text typed since the compile is not
    /// yet compiled, it marks the compiled place: words typed before gamma
    /// on its line, and a line break typed above it, leave the bar on
    /// gamma's glyph (the source map's lines and columns are the compiled text's).
    func testTheMarkFollowsTheModelAndStaysOnTheCompiledPlace() async throws {
        let (model, pages, window) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        let gamma = (Self.doc as NSString).range(of: "gamma").location
        model.caretUTF16 = gamma
        try await waitUntil("the scheduled mark") { pages.caretMark != nil }
        let before = try XCTUnwrap(pages.caretMark)
        let bar = try XCTUnwrap(before.bar)
        // Edit without compiling: the compiled text stays what the pages show.
        model.autoCompile = false
        let compiledBefore = model.compiledDocuments["main.tex"]
        XCTAssertEqual(compiledBefore, Self.doc)
        let edited = Self.doc.replacingOccurrences(of: "\\begin{document}\nAlpha", with: "\\begin{document}\n\nNew words. Alpha")
        model.updateActiveText(edited)
        model.caretUTF16 = (edited as NSString).range(of: "gamma").location
        try await waitUntil("the mark after the edit") { pages.caretKey?.utf16 == model.caretUTF16 }
        XCTAssertEqual(model.compiledDocuments["main.tex"], Self.doc, "no compile ran")
        let after = try XCTUnwrap(pages.caretMark, "still marked")
        XCTAssertEqual(after.page, before.page)
        XCTAssertEqual(try XCTUnwrap(after.bar).minX, bar.minX, accuracy: 0.01, "the bar stays on gamma's glyph")
        XCTAssertEqual(after.band, before.band, "on gamma's line")
        _ = s
    }

    /// A caret move alone (arrows, a click) is marked on the next run-loop
    /// turn; an edit waits for the settle (150 ms) and is marked once, so
    /// typing does not rebuild the caret page's glyph index on every key.
    func testACaretMoveIsMarkedAtOnceAndTypingSettles() async throws {
        let (model, pages, window) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        model.autoCompile = false
        try await Task.sleep(nanoseconds: 400_000_000) // the first compile's settled mark has run
        let gamma = (Self.doc as NSString).range(of: "gamma").location
        model.caretUTF16 = gamma
        try await Task.sleep(nanoseconds: 30_000_000)
        XCTAssertEqual(pages.caretKey?.utf16, gamma, "a caret move: marked on the next turn")
        // As NSTextView types: the selection moves first, then the text
        // change is reported (insertText, deleteBackward), in one turn.
        let edited = Self.doc.replacingOccurrences(of: "Alpha", with: "Alpha x")
        model.caretUTF16 = gamma + 2
        model.updateActiveText(edited)
        try await Task.sleep(nanoseconds: 30_000_000)
        XCTAssertEqual(pages.caretKey?.utf16, gamma, "typing: not yet, the mark waits for the settle")
        try await waitUntil("the settled mark", timeout: 5) { pages.caretKey?.utf16 == gamma + 2 }
        XCTAssertNotNil(pages.caretMark)
    }

    // MARK: the edit window

    func testTheWindowFollowsEditsAsTheTextsCompare() {
        // Every edit sequence: the window's map equals the full comparison's.
        let compiled = "alpha beta\ngamma delta\nepsilon zeta\n" as NSString
        typealias Edit = (location: Int, delete: Int, insert: String)
        let sequences: [[Edit]] = [
            [(6, 0, "XY")],                             // typing inside a line
            [(6, 0, "X"), (7, 0, "Y"), (8, 0, "Z")],    // typing on
            [(6, 0, "X"), (6, 1, "")],                  // typed and deleted again
            [(0, 0, "head\n")],                         // a line above everything
            [(20, 3, "")],                              // a deletion
            [(30, 0, "A"), (2, 0, "B")],                // after, then before
            [(2, 0, "B"), (31, 0, "A")],                // before, then after
            [(11, 0, "\n\n"), (25, 2, "QQQ")],
        ]
        for (seq, widen) in sequences.flatMap({ s in [(s, 0), (s, 3)] }) {
            let current = NSMutableString(string: compiled)
            var w = EngineV3CaretPlace.Window(path: "main.tex")
            for e in seq {
                current.replaceCharacters(in: NSRange(location: e.location, length: e.delete), with: e.insert)
                let n = (e.insert as NSString).length
                // `widen`: the storage reports a wider range than the characters
                // changed (attribute fixes), same change in length.
                let lo = max(0, e.location - widen), hi = min(current.length, e.location + n + widen)
                w.edit(newRange: NSRange(location: lo, length: hi - lo), delta: n - e.delete, length: current.length)
            }
            XCTAssertEqual(w.expectedLength(compiled: compiled.length), current.length)
            // Outside the edited region the two maps agree exactly; inside it
            // both hold the caret to the compiled region (the window's region
            // can be wider than the minimal one, never narrower).
            let p = EngineV3CaretPlace.commonPrefix(current, compiled)
            let sfx = EngineV3CaretPlace.commonSuffix(current, compiled, skip: p)
            for c in 0...current.length where c <= min(p, w.start) || c >= max(current.length - sfx, w.endCurrent) {
                XCTAssertEqual(w.map(c), EngineV3CaretPlace.map(caret: c, current: current, compiled: compiled), "\(seq) at \(c)")
            }
            // Narrowed (compared inside the region only), the window is the
            // minimal one: the same map as the full comparison everywhere.
            w.narrow(current: current, compiled: compiled)
            for c in 0...current.length {
                XCTAssertEqual(w.map(c), EngineV3CaretPlace.map(caret: c, current: current, compiled: compiled), "narrowed \(seq) widen \(widen) at \(c)")
            }
        }
        // No edit: identity.
        let w = EngineV3CaretPlace.Window(path: "main.tex")
        XCTAssertEqual(w.map(7), 7)
        XCTAssertEqual(w.expectedLength(compiled: 40), 40)
    }

    /// Glyphs that share a column (an inline formula's all carry its closing
    /// `$`): a caret at that column takes the first, after it the last, so
    /// the bar after `$` stands after the formula.
    func testACaretAfterAFormulaStandsAfterIt() {
        func g(_ x: CGFloat, col: UInt16) -> DL3GlyphRef {
            DL3GlyphRef(span: 7, col: col, origin: CGPoint(x: x, y: 100), cell: CGRect(x: x, y: 92.5, width: 5, height: 10), ink: .zero)
        }
        let line = [g(0, col: 2), g(10, col: 9), g(15, col: 9), g(20, col: 9), g(30, col: 11)]
        XCTAssertEqual(DL3SourceIndex.pick(line, col: 9)?.origin.x, 10, "at the closing $: the formula's first glyph")
        XCTAssertEqual(DL3SourceIndex.pick(line, col: 10)?.origin.x, 20, "after it: the last")
        XCTAssertEqual(DL3SourceIndex.pick(line, col: 2)?.origin.x, 0)
        XCTAssertEqual(DL3SourceIndex.pick(line, col: 0)?.origin.x, 0, "before every column: the first")
    }

    /// With the editor on screen, its edits keep the window: the caret is
    /// moved into the compiled text without comparing the texts, and the
    /// mark stays on the compiled place (gamma's glyph) after a line typed
    /// above it and words typed before it on its line.
    func testTheEditorsEditsMoveTheCaretWithoutComparingTexts() async throws {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: Self.doc, named: "main.tex")
        model.engineV3Enabled = true
        model.autoCompile = false
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 900, height: 800), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        let container = NSView(frame: NSRect(x: 0, y: 0, width: 900, height: 800))
        let hosting = NSHostingView(rootView: Host(model: model))
        hosting.frame = NSRect(x: 300, y: 0, width: 600, height: 800)
        let tv = NSTextView(usingTextLayoutManager: false) // TextKit 1, as the editor
        tv.frame = NSRect(x: 0, y: 0, width: 300, height: 800)
        tv.setAccessibilityLabel("LaTeX source")
        tv.string = Self.doc
        container.addSubview(tv)
        container.addSubview(hosting)
        window.contentView = container
        let s = model.engineV3
        s.start(model: model)
        defer { s.stop(); window.contentView = nil }
        try await EngineV3TestHost.awaitReady(s)
        if !s.statusNote.hasPrefix("ok") { s.compile(model: model, reason: "explicit") }
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount == 2 && s.pages[0] != nil }
        window.layoutIfNeeded()
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: 1)
        pages.relayout()
        let gamma = (Self.doc as NSString).range(of: "gamma").location
        pages.setCaret(path: "main.tex", utf16: gamma, stamp: s.contentStamp)
        let before = try XCTUnwrap(pages.caretMark)
        let mapped = s.caretMapsByWindow
        // Edits through the editor, as typing makes them.
        let alpha = (Self.doc as NSString).range(of: "Alpha").location
        tv.insertText("New line.\n", replacementRange: NSRange(location: alpha, length: 0))
        let words = (tv.string as NSString).range(of: "gamma").location
        tv.insertText("more ", replacementRange: NSRange(location: words, length: 0))
        model.updateActiveText(tv.string)
        let caret = (tv.string as NSString).range(of: "gamma").location
        pages.setCaret(path: "main.tex", utf16: caret, stamp: s.contentStamp)
        XCTAssertGreaterThan(s.caretMapsByWindow, mapped, "mapped by the edit window")
        let after = try XCTUnwrap(pages.caretMark)
        XCTAssertEqual(after.page, before.page)
        XCTAssertEqual(try XCTUnwrap(after.bar).minX, try XCTUnwrap(before.bar).minX, accuracy: 0.01, "still gamma's glyph")
        XCTAssertEqual(after.band, before.band)
    }

    /// Each page says "page N" at its bottom right, in the label colour of
    /// its appearance (as the v2 pane's).
    func testEachPageIsLabelled() async throws {
        let (model, pages, window) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        let v = try XCTUnwrap(pages.heldPageView(1))
        XCTAssertEqual(v.label.string as? String, "page 2")
        // Bottom right of the page in the view's own (flipped) coordinates.
        let frame = v.labelFrameInView
        XCTAssertGreaterThan(frame.minY, v.bounds.height / 2, "at the bottom: \(frame) in \(v.bounds)")
        XCTAssertEqual(frame.maxX, v.bounds.width - 4, accuracy: 0.5)
        XCTAssertEqual(v.label.alignmentMode, .right)
        // Where the layer tree really puts it, in the view's coordinates.
        let shown = v.convertFromLayer(v.label.frame)
        XCTAssertEqual(shown.minY, frame.minY, accuracy: 0.5, "the label layer is at the bottom on screen: \(shown) in \(v.bounds)")
        pages.setAppearance(.dark)
        XCTAssertEqual(v.label.foregroundColor, NSColor(white: 0.7, alpha: 1).cgColor, "the dark page's label colour")
    }
}
