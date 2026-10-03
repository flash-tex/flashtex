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

    /// A pane beside an editor (`NSTextView`, TextKit 1, "LaTeX source") in
    /// one window, as in the app, showing `model`'s active text after its
    /// first compile. The editor is not bound to the model: a test gives the
    /// model the editor's text (`updateActiveText`) and the editor the
    /// model's (`tv.string = …`) in the order the app's binding does.
    func editorPane(_ model: ShellModel, autoCompile: Bool, pageCount: Int = 2) async throws
        -> (EngineV3Session, EngineV3PagesView, NSTextView, NSWindow) {
        model.engineV3Enabled = true
        model.autoCompile = autoCompile
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 900, height: 800), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        let container = NSView(frame: NSRect(x: 0, y: 0, width: 900, height: 800))
        let hosting = NSHostingView(rootView: Host(model: model))
        hosting.frame = NSRect(x: 300, y: 0, width: 600, height: 800)
        let tv = NSTextView(usingTextLayoutManager: false)
        tv.frame = NSRect(x: 0, y: 0, width: 300, height: 800)
        tv.setAccessibilityLabel("LaTeX source")
        tv.string = model.activeText
        container.addSubview(tv)
        container.addSubview(hosting)
        window.contentView = container
        let s = model.engineV3
        s.start(model: model)
        try await EngineV3TestHost.awaitReady(s)
        if !s.statusNote.hasPrefix("ok") { s.compile(model: model, reason: "explicit") }
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pageCount == pageCount && s.pages[0] != nil }
        window.layoutIfNeeded()
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: 1)
        pages.relayout()
        return (s, pages, tv, window)
    }

    /// Every compile sent has finished and read `text`.
    func waitForCompiled(_ s: EngineV3Session, _ model: ShellModel, _ text: String, path: String = "main.tex") async throws {
        try await waitUntil("the compile of the new text") {
            model.compiledDocuments[path] == text && !s.compiling && s.statusNote.hasPrefix("ok") && s.pages[0] != nil
        }
        s.view?.relayout()
    }

    /// The bar for the caret at `utf16` of `text` stands where forward search
    /// puts that character (the glyph at its column).
    func assertBar(_ pages: EngineV3PagesView, _ s: EngineV3Session, at utf16: Int, of text: String,
                   path: String = "main.tex", _ what: String, line: UInt = #line) throws {
        pages.setCaret(path: path, utf16: utf16, stamp: s.contentStamp)
        let mark = try XCTUnwrap(pages.caretMark, "\(what): a mark", line: line)
        let byte = text.utf8.distance(from: text.startIndex, to: text.utf16.index(text.startIndex, offsetBy: utf16))
        let place = try XCTUnwrap(s.place(path: path, byte: byte, in: text), "\(what): forward search", line: line)
        XCTAssertEqual(mark.page, place.page, "\(what): page", line: line)
        XCTAssertEqual(try XCTUnwrap(mark.bar, line: line).minX, place.rect.minX, accuracy: 0.01, "\(what): the bar at the character's glyph", line: line)
    }

    /// Typing with auto-compile on: each key goes out through the fast path
    /// from the storage notification, which the caret windows also follow,
    /// in one observer, windows first. Until the compiles land the bar stays
    /// on the compiled place; after, it stands on the typed text's place.
    /// (With the fast path first, each compile's window would also take
    /// its own edit, and a caret after the typing would land a character
    /// to the left.)
    func testTypingThroughTheFastPathMarksTheTypedPlace() async throws {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: Self.doc, named: "main.tex")
        let (s, pages, tv, window) = try await editorPane(model, autoCompile: true)
        defer { s.stop(); window.contentView = nil }
        let gamma = (Self.doc as NSString).range(of: "gamma").location
        try assertBar(pages, s, at: gamma, of: Self.doc, "compiled")
        let compiledBar = try XCTUnwrap(pages.caretMark?.bar)
        let sent = s.fastEditsSent, mapped = s.caretMapsByWindow
        // As NSTextView types: the storage edit (the fast path sends it), then
        // the binding gives the model the text.
        for (i, ch) in "more ".enumerated() {
            s.nextKeystrokeNs = MonotonicClock.nowNs()
            tv.insertText(String(ch), replacementRange: NSRange(location: gamma + i, length: 0))
            model.updateActiveText(tv.string)
        }
        XCTAssertEqual(s.fastEditsSent, sent + 5, "each key went out through the fast path")
        let typed = tv.string
        let caret = (typed as NSString).range(of: "gamma").location
        XCTAssertEqual(caret, gamma + 5)
        if model.compiledDocuments["main.tex"] == Self.doc {
            // Not yet compiled: the caret stays on the compiled gamma.
            pages.setCaret(path: "main.tex", utf16: caret, stamp: s.contentStamp)
            XCTAssertEqual(try XCTUnwrap(pages.caretMark?.bar).minX, compiledBar.minX, accuracy: 0.01, "the compiled place")
        }
        try await waitForCompiled(s, model, typed)
        try assertBar(pages, s, at: caret, of: typed, "after the fast path's compiles")
        // After the edited region (the storage reports the rest of the
        // typed line as edited): where a doubled edit would shift the caret.
        try assertBar(pages, s, at: (typed as NSString).range(of: "paragraph").location, of: typed, "a line below the typing")
        XCTAssertGreaterThan(s.caretMapsByWindow, mapped, "mapped by the compile's window")
        // Mid-word: the "e" of "more".
        try assertBar(pages, s, at: gamma + 3, of: typed, "inside the typed word")
    }

    /// A reload (DocumentFiles.adoptReloadedDocument), a restored snapshot or
    /// any `updateActiveText` from outside the editor: the model takes the
    /// text first and its compile is sent while the editor still shows the
    /// old one, then SwiftUI replaces the editor's text. That compile's
    /// window cannot follow anything (it would take the replacement as an
    /// edit from the old text and drift past the old text's end): it is
    /// invalid and the texts are compared.
    func testAReloadOutsideTheEditorIsComparedNotFollowed() async throws {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: Self.doc, named: "main.tex")
        let (s, pages, tv, window) = try await editorPane(model, autoCompile: true)
        defer { s.stop(); window.contentView = nil }
        let reloaded = Self.doc.replacingOccurrences(of: "\\begin{document}\n",
                                                     with: "\\begin{document}\nA paragraph the reload added, above everything.\n\n")
        let windows = s.caretWindows.count
        model.updateActiveText(reloaded)
        XCTAssertGreaterThan(s.caretWindows.count, windows, "the reload's compile is sent before the editor shows its text")
        XCTAssertTrue(s.caretWindows.values.allSatisfy(\.invalid), "sent from outside the editor: no edit window")
        tv.string = reloaded // SwiftUI's update (SourceEditorView.updateNSView)
        try await waitForCompiled(s, model, reloaded)
        let two = (reloaded as NSString).range(of: "two").location
        XCTAssertGreaterThan(two, (Self.doc as NSString).length, "past the old text's end")
        let mapped = s.caretMapsByWindow
        try assertBar(pages, s, at: two, of: reloaded, "after the reload")
        try assertBar(pages, s, at: (reloaded as NSString).range(of: "gamma").location, of: reloaded, "gamma after the reload")
        XCTAssertEqual(s.caretMapsByWindow, mapped, "compared, not mapped by a window")
        // Typing in the editor again. The first key's compile goes out from
        // the storage notification, before the binding brings the model in
        // step with the editor: compared too. From the second key on, the
        // compiles' windows are trusted again.
        let delta = (reloaded as NSString).range(of: "delta").location
        for (i, ch) in "xy".enumerated() {
            s.nextKeystrokeNs = MonotonicClock.nowNs()
            tv.insertText(String(ch), replacementRange: NSRange(location: delta + i, length: 0))
            model.updateActiveText(tv.string)
            XCTAssertEqual(s.caretWindows.values.contains { !$0.invalid }, i > 0, "key \(i + 1): a trusted window")
        }
        let typed = tv.string
        try await waitForCompiled(s, model, typed)
        let before = s.caretMapsByWindow
        try assertBar(pages, s, at: delta + 2, of: typed, "typed after the reload")
        XCTAssertGreaterThan(s.caretMapsByWindow, before, "the editor's own edits: mapped by the window again")
    }

    /// Editing another document (an `\input` chapter shown in the editor)
    /// leaves the main document's windows unusable: switching to it and back
    /// replaces the editor's text, the chapter's edits are not the main
    /// text's, and the caret in the main document is compared, landing on
    /// its own glyph. Back in the main document, a compile (⌘B) goes out
    /// before SwiftUI puts the main text back in the editor: its window
    /// starts while the editor still shows the chapter, and only the
    /// whole-text replacement rule (`caretStorageEdited`) keeps it from
    /// taking the replacement as an edit of the chapter's text and holding
    /// a caret past the chapter's length at that length (Omega, at 80, would
    /// be marked at 64: this test fails without the rule).
    func testEditingAnotherDocumentLeavesTheCaretCompared() async throws {
        try EngineV3TestHost.require()
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-caret-docs-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let main = "\\documentclass{article}\n\\begin{document}\nAlpha beta gamma delta.\n\n\\input{chap}\n\nOmega psi chi.\n\\end{document}\n"
        let chap = "First chapter line.\nSecond chapter line.\n"
        try main.write(to: dir.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        try chap.write(to: dir.appendingPathComponent("chap.tex"), atomically: true, encoding: .utf8)
        let model = ShellModel()
        XCTAssertEqual(model.openTex(at: dir.appendingPathComponent("main.tex"), dirty: .discard), .opened)
        let (s, pages, tv, window) = try await editorPane(model, autoCompile: true, pageCount: 1)
        defer { s.stop(); window.contentView = nil }
        let omega = (main as NSString).range(of: "Omega").location
        try assertBar(pages, s, at: omega, of: main, "before")
        // The chapter in the editor, as the app shows another document.
        _ = await model.project.openDocument("chap.tex")
        model.activePath = "chap.tex"
        tv.string = model.activeText
        XCTAssertEqual(tv.string, chap)
        // Typed in the chapter: a line above Omega's on the page.
        s.nextKeystrokeNs = MonotonicClock.nowNs()
        tv.insertText("An added chapter line.\n", replacementRange: NSRange(location: 0, length: 0))
        model.updateActiveText(tv.string)
        let chapTyped = tv.string
        try await waitForCompiled(s, model, chapTyped, path: "chap.tex")
        // Back to the main document (its text unchanged), with a compile
        // sent before the editor shows it again.
        model.activePath = "main.tex"
        let stamp = s.contentStamp
        s.compileNow(model: model)
        // The editor waits for it here (it may even land first): its window
        // started with the chapter on screen.
        try await waitUntil("the compile") { s.contentStamp > stamp && !s.compiling && s.caretWindow?.path == "main.tex" && s.pages[0] != nil }
        XCTAssertEqual(tv.string, chapTyped)
        XCTAssertEqual(s.caretWindow?.invalid, false)
        tv.string = model.activeText
        XCTAssertEqual(tv.string, main)
        XCTAssertEqual(s.caretWindow?.invalid, true, "the whole text replaced: nothing to follow")
        XCTAssertGreaterThan(omega, (chapTyped as NSString).length, "past the chapter's length")
        s.view?.relayout()
        let mapped = s.caretMapsByWindow
        try assertBar(pages, s, at: omega, of: main, "after editing the chapter")
        XCTAssertEqual(s.caretMapsByWindow, mapped, "compared: no window follows the main text through the chapter's edits")
    }

    /// A model text arriving while an input method composes in the editor
    /// comes from outside it (the editor's marked text is not given to the
    /// model until the commit), and a model text with the same bytes
    /// changes nothing: the first invalidates the windows, the second not.
    func testAModelTextMidCompositionIsOutsideAndTheSameBytesChangeNothing() async throws {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: Self.doc, named: "main.tex")
        let (s, _, tv, window) = try await editorPane(model, autoCompile: false)
        defer { s.stop(); window.contentView = nil }
        XCTAssertEqual(s.caretWindow?.invalid, false, "the compile's window")
        // The same bytes from outside the editor: nothing to invalidate.
        model.updateActiveText(String(Self.doc.utf8.map { Character(UnicodeScalar($0)) }))
        XCTAssertEqual(s.caretWindow?.invalid, false, "the same bytes")
        XCTAssertFalse(s.caretEditorOutOfStep)
        // Composing in the editor, then a model text from outside.
        let gamma = (Self.doc as NSString).range(of: "gamma").location
        window.makeFirstResponder(tv)
        tv.setMarkedText("か", selectedRange: NSRange(location: 1, length: 0), replacementRange: NSRange(location: gamma, length: 0))
        XCTAssertTrue(tv.hasMarkedText())
        XCTAssertEqual(s.caretWindow?.invalid, false, "the composition is followed")
        model.updateActiveText(Self.doc.replacingOccurrences(of: "delta", with: "delta epsilon"))
        XCTAssertEqual(s.caretWindow?.invalid, true, "a model text mid-composition is from outside the editor")
        XCTAssertTrue(s.caretEditorOutOfStep)
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
