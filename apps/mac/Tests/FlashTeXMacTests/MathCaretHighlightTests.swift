import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Source → preview for a caret inside math (MathCaretHighlight.swift) on the
/// REAL producer's display list: `display-list-v2-math-nav.json` was produced
/// by flashtex-render 9aaec57a from `display-list-v2-math-nav.tex` (inline
/// `$a^2 + b_1 = \frac{x}{y}$`, `$\sqrt{z}$`, display `\[ \sum_{i=0}^{n}
/// \frac{\sqrt{i}}{2} \]`, the one-glyph `$x$`, text and an `fi` ligature).
/// Measured: every cluster and rule of a formula carries the whole formula's
/// span (delimiters included); no math cluster maps 1:1.
@MainActor
final class MathCaretHighlightTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Fonts")

    private func page() throws -> (RenderingV2.Page, String) {
        let env = try RenderingV2.decode(try Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-math-nav.json")))
        let tex = try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-math-nav.tex"), encoding: .utf8)
        XCTAssertEqual(env.payload.documents.map(\.path), ["main.tex"])
        XCTAssertEqual(SourceDigest.sha256Hex(tex), env.payload.documents[0].sha256, "the fixture .tex is the exact text the list was produced from")
        return (env.payload.pages[0], tex)
    }

    private func bytes(_ tex: String, _ needle: String) -> Int {
        let hay = Array(tex.utf8), n = Array(needle.utf8)
        for i in 0...(hay.count - n.count) where Array(hay[i..<i + n.count]) == n { return i }
        XCTFail("\(needle) not in fixture"); return -1
    }

    private func sourceText(_ tex: String, _ s: RuntimeV1.SourceRange) -> String {
        String(decoding: Array(tex.utf8)[s.startByte..<s.endByte], as: UTF8.self)
    }

    private func onlyFormula(_ hs: [V2Geometry.CaretHighlight], file: StaticString = #filePath, line: UInt = #line) -> V2Geometry.FormulaBox? {
        guard hs.count == 1, case .formula(let box) = hs[0] else {
            XCTFail("expected exactly one formula box, got \(hs)", file: file, line: line); return nil
        }
        return box
    }

    func testInlineFormulaWithFracAndScriptsIsOneEnclosingBox() throws {
        let (page, tex) = try page()
        let formula = "$a^2 + b_1 = \\frac{x}{y}$"
        let start = bytes(tex, formula), end = start + formula.utf8.count
        // Every byte of the formula — the delimiters, the base, `^`, the superscript
        // digit, `_`, the subscript, the \frac command, its numerator and denominator —
        // lights the same enclosing box: 8 glyph clusters (a 2 + b 1 = x y) and the
        // fraction bar rule, items 1...8 of the page (measured on the fixture).
        var seen: V2Geometry.FormulaBox?
        for byte in start..<end {
            guard let box = onlyFormula(V2Geometry.caretHighlights(containing: byte, path: "main.tex", in: page)) else { return }
            if let seen { XCTAssertEqual(box, seen, "byte \(byte)") } else { seen = box }
        }
        let box = try XCTUnwrap(seen)
        XCTAssertEqual(box.source, RuntimeV1.SourceRange(path: "main.tex", startByte: start, endByte: end))
        XCTAssertEqual(sourceText(tex, box.source), formula)
        XCTAssertEqual(box.clusterCount, 8)
        XCTAssertEqual(box.ruleCount, 1)
        XCTAssertEqual(box.itemIndices, Array(1...8))
        XCTAssertEqual(box.rects.count, 9)
        // The bounds enclose every member rectangle, including the fraction bar.
        for r in box.rects {
            XCTAssertGreaterThanOrEqual(r.x, box.bounds.x)
            XCTAssertGreaterThanOrEqual(r.top, box.bounds.top)
            XCTAssertLessThanOrEqual(r.x + r.width, box.bounds.x + box.bounds.width)
            XCTAssertLessThanOrEqual(r.top + r.height, box.bounds.top + box.bounds.height)
        }
        guard case .rule(let bar) = page.items[8] else { return XCTFail("item 8 is the fraction bar") }
        XCTAssertTrue(box.rects.contains(RenderingV2.Rect(x: bar.x, top: bar.top, width: bar.width, height: bar.height)))
        // The superscript `2` (LMRoman7) sits above the base line of `a`; the
        // denominator `y` below the bar: the box spans both.
        guard case .glyphRun(let two) = page.items[2], case .glyphRun(let xy) = page.items[7] else { return XCTFail() }
        XCTAssertLessThan(two.clusters[0].hitRects[0].top, bar.top)
        XCTAssertGreaterThan(xy.clusters[1].hitRects[0].top, bar.top)
        XCTAssertEqual(box.bounds.top, min(box.bounds.top, two.clusters[0].hitRects[0].top))
        // The byte after the closing `$` (a space) is in no item; the `and` after it is plain text.
        XCTAssertTrue(V2Geometry.caretHighlights(containing: end, path: "main.tex", in: page).isEmpty)
        guard case .cluster(let a) = V2Geometry.caretHighlights(containing: end + 1, path: "main.tex", in: page).first else { return XCTFail() }
        XCTAssertNotNil(a.caret, "plain text keeps the exact caret")
    }

    func testSqrtOverbarIsPartOfTheBoxAndDisplayMathHasTwoRules() throws {
        let (page, tex) = try page()
        let sqrt = "$\\sqrt{z}$"
        let s = bytes(tex, sqrt)
        let box = try XCTUnwrap(onlyFormula(V2Geometry.caretHighlights(containing: s + 7, path: "main.tex", in: page))) // the `z`
        XCTAssertEqual(sourceText(tex, box.source), sqrt)
        XCTAssertEqual(box.clusterCount, 2, "√ and z")
        XCTAssertEqual(box.ruleCount, 1, "the overbar")
        guard let lastItemIndex = box.itemIndices.last, let firstItemIndex = box.itemIndices.first else {
            return XCTFail("expected non-empty itemIndices")
        }
        guard case .rule(let overbar) = page.items[lastItemIndex], case .glyphRun(let run) = page.items[firstItemIndex] else { return XCTFail() }
        XCTAssertLessThan(overbar.top, run.clusters[1].hitRects[0].top, "the overbar is above the radicand")
        // The radical sign's hit rect reaches slightly above the overbar (measured:
        // 22 ticks); the box top is the higher of the two, never below the overbar.
        XCTAssertLessThanOrEqual(box.bounds.top, overbar.top)
        XCTAssertEqual(box.bounds.top, min(overbar.top, run.clusters[0].hitRects[0].top))

        let display = "\\[ \\sum_{i=0}^{n} \\frac{\\sqrt{i}}{2} \\]"
        let d = bytes(tex, display)
        for probe in [d, d + 3 /* \sum */, d + 9 /* i=0 */, d + 16 /* n */, d + 24 /* \sqrt */, d + 31 /* 2 */, d + display.utf8.count - 1] {
            let box = try XCTUnwrap(onlyFormula(V2Geometry.caretHighlights(containing: probe, path: "main.tex", in: page)), "byte \(probe)")
            XCTAssertEqual(sourceText(tex, box.source), display)
            XCTAssertEqual(box.clusterCount, 8, "n ∑ i = 0 √ i 2")
            XCTAssertEqual(box.ruleCount, 2, "\\sqrt overbar and fraction bar")
            XCTAssertEqual(box.itemIndices, Array(13...20))
        }
    }

    func testSingleGlyphFormulaAndLigaturesKeepTheClusterFallback() throws {
        let (page, tex) = try page()
        // `$x$`: one cluster carries the 3-byte span; no fan-out → the documented
        // whole-cluster fallback (no exact caret: the cluster is not 1:1).
        let x = bytes(tex, "$x$")
        let hs = V2Geometry.caretHighlights(containing: x + 1, path: "main.tex", in: page)
        guard hs.count == 1, case .cluster(let m) = hs[0] else { return XCTFail("\(hs)") }
        XCTAssertEqual(m.itemIndex, 22)
        XCTAssertNil(m.caret)
        // Inside the `fi` ligature of "fin." (2 source bytes, one glyph): whole
        // cluster at its second byte, exact caret at its first.
        let fi = bytes(tex, "fin.")
        guard case .cluster(let inside) = V2Geometry.caretHighlights(containing: fi + 1, path: "main.tex", in: page).first else { return XCTFail() }
        XCTAssertNil(inside.caret)
        guard case .cluster(let atStart) = V2Geometry.caretHighlights(containing: fi, path: "main.tex", in: page).first else { return XCTFail() }
        XCTAssertNotNil(atStart.caret)
        // Other documents and bytes outside every item match nothing.
        XCTAssertTrue(V2Geometry.caretHighlights(containing: x + 1, path: "chapter.tex", in: page).isEmpty)
        XCTAssertTrue(V2Geometry.caretHighlights(containing: 0, path: "main.tex", in: page).isEmpty)
        // `clusters(containing:)` is unchanged: every glyph cluster of a formula, no rules.
        XCTAssertEqual(V2Geometry.clusters(containing: bytes(tex, "$a^2") + 1, path: "main.tex", in: page).count, 8)
    }

    func testExactClusterWinsOverAFannedOutSpan() throws {
        // If a producer maps a math atom 1:1 (its own bytes) while the enclosing
        // formula span is also carried by other items, the exact caret wins.
        let (page, tex) = try page()
        var items = page.items
        let formula = "$a^2 + b_1 = \\frac{x}{y}$"
        let start = bytes(tex, formula)
        guard case .glyphRun(var a) = items[1] else { return XCTFail() }
        a.clusters[0].sources = [RuntimeV1.SourceRange(path: "main.tex", startByte: start + 1, endByte: start + 2)] // `a` → its own byte
        items[1] = .glyphRun(a)
        let patched = RenderingV2.Page(number: page.number, width: page.width, height: page.height, items: items)
        let hs = V2Geometry.caretHighlights(containing: start + 1, path: "main.tex", in: patched)
        guard hs.count == 1, case .cluster(let m) = hs[0] else { return XCTFail("\(hs)") }
        XCTAssertEqual(m.itemIndex, 1)
        XCTAssertEqual(m.caret, a.clusters[0].carets.first)
        // Any other byte of the formula still gets the box, now without `a`.
        let box = try XCTUnwrap(onlyFormula(V2Geometry.caretHighlights(containing: start + 2, path: "main.tex", in: patched)))
        XCTAssertEqual(box.clusterCount, 7)
        XCTAssertEqual(box.itemIndices, Array(2...8))
    }

    // MARK: the caret's paragraph band

    /// Blank-line delimited paragraphs in the source: the byte after the
    /// previous blank line to the newline before the next one.
    func testParagraphBoundsAreBlankLineDelimited() {
        let tex = "\\begin{document}\nOne one\none.\n\nTwo two\n  \t\nThree.\n\\end{document}\n"
        let one = Array(tex.utf8).firstIndex(of: UInt8(ascii: "O"))!
        let two = Array(tex.utf8).firstIndex(of: UInt8(ascii: "T"))!
        let three = (tex.range(of: "Three")!.lowerBound).utf16Offset(in: tex)
        XCTAssertEqual(V2Geometry.paragraphBounds(containing: one + 3, in: tex), 0..<one + "One one\none.".utf8.count, "the first paragraph runs from the start")
        XCTAssertEqual(V2Geometry.paragraphBounds(containing: two + 1, in: tex), two..<two + "Two two".utf8.count, "a whitespace-only line is blank")
        XCTAssertEqual(V2Geometry.paragraphBounds(containing: three, in: tex), three..<tex.utf8.count - 1, "the last runs to the final newline")
        XCTAssertTrue(V2Geometry.paragraphBounds(containing: two - 1, in: tex).isEmpty, "a caret on the blank line has no paragraph")
        XCTAssertEqual(V2Geometry.paragraphBounds(containing: two - 2, in: tex), 0..<two - 2, "the caret before the newline ending `one.` is still on that line")
        XCTAssertEqual(V2Geometry.paragraphBounds(containing: one + 7, in: tex).lowerBound, 0, "the caret at a line end belongs to that line")
        XCTAssertEqual(V2Geometry.paragraphBounds(containing: 99_999, in: tex), tex.utf8.count..<tex.utf8.count, "out of range clamps to the empty end")
        XCTAssertEqual(V2Geometry.paragraphBounds(containing: 2, in: "no newline at all"), 0..<17)
    }

    /// The real producer's list (`display-list-v2-window`, one paragraph per
    /// page): the caret in the second paragraph bands exactly the second's
    /// rows on its page and nothing on the first's.
    func testCaretInTheSecondParagraphBandsOnlyThatParagraphsRows() throws {
        let env = try RenderingV2.decode(try Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-window.json")))
        let tex = try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-window.tex"), encoding: .utf8)
        XCTAssertEqual(SourceDigest.sha256Hex(tex), env.payload.documents[0].sha256)
        let first = try XCTUnwrap(env.payload.pages.first { $0.number == 3 && $0.resident })
        let second = try XCTUnwrap(env.payload.pages.first { $0.number == 4 && $0.resident })
        // The caret on the first word of the paragraph page 4 lays out.
        guard case .glyphRun(let run) = second.items[0], let source = run.clusters[0].sources?.first else { return XCTFail() }
        let caret = source.startByte + 3
        let paragraph = V2Geometry.paragraphBounds(containing: caret, in: tex)
        XCTAssertTrue(paragraph.contains(source.startByte))
        // The fixture writes `\newpage` on the line right before the text, so it opens the paragraph.
        let paragraphText = String(decoding: Array(tex.utf8)[paragraph], as: UTF8.self)
        XCTAssertTrue(paragraphText.hasPrefix("\\newpage\nHello world."), paragraphText.prefix(30).description)
        XCTAssertEqual(paragraphText.components(separatedBy: "\\newpage").count, 2, "the blank line after the text ends it before the next \\newpage")

        let onFirst = V2Geometry.caretHighlights(containing: caret, path: "main.tex", in: first, paragraph: paragraph)
        XCTAssertTrue(onFirst.isEmpty, "nothing of the first paragraph's page is in the band: \(onFirst)")

        let onSecond = V2Geometry.caretHighlights(containing: caret, path: "main.tex", in: second, paragraph: paragraph)
        XCTAssertEqual(onSecond.count, 2, "the caret's own cluster plus one band: \(onSecond)")
        guard case .cluster = onSecond[0], case .paragraph(let band) = onSecond[1] else { return XCTFail("\(onSecond)") }
        XCTAssertEqual(band.source, RuntimeV1.SourceRange(path: "main.tex", startByte: paragraph.lowerBound, endByte: paragraph.upperBound))
        // Every word of the paragraph, none of the page number (it carries no source).
        let words = second.items.filter { if case .glyphRun(let r) = $0 { return r.clusters.contains { $0.sources?.isEmpty == false } } else { return false } }
        XCTAssertEqual(band.itemCount, words.count)
        let wordTops = Set(words.flatMap { item -> [Int64] in guard case .glyphRun(let r) = item else { return [] }; return r.clusters.flatMap { $0.hitRects.map(\.top) } })
        XCTAssertEqual(band.rows.count, wordTops.count, "one band per line row")
        XCTAssertEqual(band.rows.map(\.top), band.rows.map(\.top).sorted(), "top to bottom")
        for row in band.rows { XCTAssertTrue(wordTops.contains(row.top)); XCTAssertGreaterThan(row.width, 0) }
        let pageNumberTop = second.items.compactMap { item -> Int64? in
            guard case .glyphRun(let r) = item, r.clusters.allSatisfy({ $0.sources?.isEmpty ?? true }) else { return nil }
            return r.clusters.first?.hitRects.first?.top
        }.first
        XCTAssertNotNil(pageNumberTop)
        XCTAssertFalse(band.rows.contains { $0.top == pageNumberTop }, "the page number is not in the paragraph")

        // Without a paragraph (the preference off) nothing changes.
        XCTAssertEqual(V2Geometry.caretHighlights(containing: caret, path: "main.tex", in: second).count, 1)
        // The band is gated per page on the source bounds, so the first page never walks its items.
        let prepared = try V2Frame.prepare(env).prepared
        XCTAssertFalse(prepared[2].mayOverlap(paragraph, path: "main.tex"))
        XCTAssertTrue(prepared[3].mayOverlap(paragraph, path: "main.tex"))
        XCTAssertTrue(PreviewV2View.caretHighlights(page: first, prepared: prepared[2], byte: caret, path: "main.tex", paragraph: paragraph).isEmpty)
        XCTAssertEqual(PreviewV2View.caretHighlights(page: second, prepared: prepared[3], byte: caret, path: "main.tex", paragraph: paragraph).count, 2)
    }

    /// Two paragraphs on ONE page (synthetic, 1:1 clusters): the band merges
    /// the second's rectangles by row — a superscript's taller box shares its
    /// line's row — and leaves the first's rows alone.
    func testBandRowsMergeVerticallyOverlappingRectsAndSkipTheOtherParagraph() {
        let tex = "A a\nA a\n\nB b\nB b^2\n"
        func cluster(_ byte: Int, x: Int64, top: Int64, h: Int64 = 100) -> RenderingV2.Cluster {
            RenderingV2.Cluster(textStartByte: 0, textEndByte: 1, hitRects: [RenderingV2.Rect(x: x, top: top, width: 80, height: h)], carets: [],
                                sources: [RuntimeV1.SourceRange(path: "main.tex", startByte: byte, endByte: byte + 1)])
        }
        func run(_ clusters: [RenderingV2.Cluster]) -> RenderingV2.Item {
            .glyphRun(RenderingV2.GlyphRun(fontId: "f", fontSize: 10, text: "x", glyphs: [], clusters: clusters, paint: .black))
        }
        let b = tex.utf8.firstIndex(of: UInt8(ascii: "B"))!.utf16Offset(in: tex)
        let page = RenderingV2.Page(number: 1, width: 10_000, height: 10_000, items: [
            run([cluster(0, x: 100, top: 1000), cluster(2, x: 200, top: 1000)]),
            run([cluster(4, x: 100, top: 1200), cluster(6, x: 200, top: 1200)]),
            run([cluster(b, x: 100, top: 1600), cluster(b + 2, x: 200, top: 1600)]),
            run([cluster(b + 4, x: 100, top: 1800), cluster(b + 6, x: 200, top: 1800), cluster(b + 8, x: 280, top: 1760, h: 70)]),
            .rule(RenderingV2.Rule(x: 100, top: 1890, width: 260, height: 5, paint: .black, sources: [RuntimeV1.SourceRange(path: "main.tex", startByte: b + 4, endByte: b + 9)])),
        ])
        let paragraph = V2Geometry.paragraphBounds(containing: b + 4, in: tex) // the second row's `B`
        XCTAssertEqual(paragraph, b..<tex.utf8.count - 1)
        let hs = V2Geometry.caretHighlights(containing: b + 4, path: "main.tex", in: page, paragraph: paragraph)
        guard hs.count == 2, case .paragraph(let band) = hs[1] else { return XCTFail("\(hs)") }
        XCTAssertEqual(band.itemCount, 3)
        XCTAssertEqual(band.rows, [
            RenderingV2.Rect(x: 100, top: 1600, width: 180, height: 100),
            RenderingV2.Rect(x: 100, top: 1760, width: 260, height: 140), // the superscript, the line and the rule share a row
        ])
        // The caret in the first paragraph bands its two rows only.
        let firstParagraph = V2Geometry.paragraphBounds(containing: 1, in: tex)
        let onFirst = V2Geometry.caretHighlights(containing: 1, path: "main.tex", in: page, paragraph: firstParagraph)
        guard case .paragraph(let firstBand)? = onFirst.last else { return XCTFail("\(onFirst)") }
        XCTAssertEqual(firstBand.rows.map(\.top), [1000, 1200])
    }

    /// The pane's memo: a stale frame keeps the previous band, a verified one
    /// recomputes it, and a switch of document drops it.
    func testParagraphMemoHoldsTheBandAcrossAStaleFrame() {
        let memo = CaretParagraphMemo()
        let text = "one\n\ntwo\n\nthree"
        XCTAssertEqual(memo.range(stale: true, byte: 1, path: "a.tex", text: text), 0..<3, "nothing held yet: computed from the buffer (nothing to flash from)")
        XCTAssertEqual(memo.range(stale: true, byte: 6, path: "a.tex", text: text), 0..<3, "stale: the previous band, whatever the caret did")
        XCTAssertEqual(memo.range(stale: false, byte: 6, path: "a.tex", text: text), 5..<8, "verified: recomputed")
        XCTAssertEqual(memo.range(stale: false, byte: nil, path: "a.tex", text: text), 5..<8, "no caret keeps the last band of the same document")
        XCTAssertEqual(memo.range(stale: true, byte: 1, path: "b.tex", text: text), 0..<3, "another document: never the old document's band")
        XCTAssertNil(memo.range(stale: false, byte: nil, path: "a.tex", text: text), "back without a caret: nothing of a.tex is held")
    }

    func testModelReportsTheFormulaBoxUnderTheEditorCaret() throws {
        let store = V2FontStore(directories: [Self.fontsDir.path])
        guard store.fonts.contains(where: { $0.url.lastPathComponent == "latinmodern-math.otf" }) else { throw XCTSkip("bundled math font missing") }
        let (_, tex) = try page()
        let model = ShellModel()
        model.replaceProject(entryText: tex)
        let done = expectation(description: "load")
        model.loadDisplayListV2(url: Self.fixtures.appendingPathComponent("display-list-v2-math-nav.json")) { done.fulfill() }
        wait(for: [done], timeout: 20)
        guard case .loaded? = model.displayListV2 else { return XCTFail("expected a prepared frame: \(String(describing: model.displayListV2))") }
        let frac = bytes(tex, "\\frac{x}{y}")
        model.caretUTF16 = (tex as NSString).range(of: "\\frac{x}{y}").location + 6 // inside `x`
        XCTAssertEqual(model.caretByte, frac + 6)
        let boxes = model.caretFormulaBoxes()
        XCTAssertEqual(boxes.count, 1)
        XCTAssertEqual(boxes.first?.page, 1)
        XCTAssertEqual(boxes.first.map { sourceText(tex, $0.box.source) }, "$a^2 + b_1 = \\frac{x}{y}$")
        model.caretUTF16 = (tex as NSString).range(of: "Inline").location + 2
        XCTAssertTrue(model.caretFormulaBoxes().isEmpty, "a caret in text is in no formula")
        // ⌘⇧J reads the display list, not the elided v1 pages: the caret inside
        // the fraction selects the whole formula span the pane boxes — the same
        // navigation a click on that formula performs. Before this was wired it
        // answered "No compile result loaded", because the v2 route asks for
        // `display-list-v2-only` and there are no v1 pages to map onto.
        model.caretUTF16 = (tex as NSString).range(of: "\\frac{x}{y}").location + 6
        model.revealCaretInPreview()
        XCTAssertEqual(model.selection.map { (model.activeText as NSString).substring(with: $0.nsRange) },
                       "$a^2 + b_1 = \\frac{x}{y}$")
        XCTAssertTrue(model.navigationNote?.hasPrefix("Selected") == true, model.navigationNote ?? "nil")
        XCTAssertTrue(model.navigationNote?.hasSuffix("page 1") == true, model.navigationNote ?? "nil")
    }

    /// ⌘⇧J on ordinary text of a v2 frame, and the guard that it no longer
    /// falls through to the runtime-v1 pages the v2 route asks to have elided.
    ///
    /// Regression: `display-list-v2-only` (default on) makes `compile_result.pages`
    /// empty, so the v1 implementation could only ever answer "inside no preview
    /// item" / "No compile result loaded" on the shipped default.
    func testRevealCaretUsesTheDisplayListWhenTheV2PaneIsShowing() throws {
        let (_, tex) = try page()
        let model = ShellModel()
        model.replaceProject(entryText: tex)
        let done = expectation(description: "load")
        model.loadDisplayListV2(url: Self.fixtures.appendingPathComponent("display-list-v2-math-nav.json")) { done.fulfill() }
        wait(for: [done], timeout: 20)
        guard case .loaded? = model.displayListV2 else { return XCTFail("expected a prepared frame") }
        XCTAssertTrue(model.previewV2)
        XCTAssertNil(model.result, "no v1 result at all: the v2 route is the only source here")

        let ns = tex as NSString
        model.caretUTF16 = ns.range(of: "Inline").location + 2
        model.revealCaretInPreview()
        let selected = try XCTUnwrap(model.selection.map { (model.activeText as NSString).substring(with: $0.nsRange) })
        XCTAssertFalse(selected.isEmpty)
        XCTAssertTrue(tex.contains(selected), "the selection is a span of the buffer")
        let note = try XCTUnwrap(model.navigationNote)
        XCTAssertTrue(note.hasPrefix("Selected"), note)
        XCTAssertTrue(note.hasSuffix("page 1"), note)
        XCTAssertFalse(note.contains("No compile result"), note)
        XCTAssertFalse(note.contains("inside no preview item"), note)

        // A byte the producer laid out nothing for still says so, without
        // disturbing the selection.
        let before = model.selection
        model.caretUTF16 = 1 // inside \documentclass, in the preamble
        model.revealCaretInPreview()
        XCTAssertEqual(model.selection, before)
        XCTAssertTrue(model.navigationNote?.contains("inside no preview item") == true, model.navigationNote ?? "nil")
    }
}
