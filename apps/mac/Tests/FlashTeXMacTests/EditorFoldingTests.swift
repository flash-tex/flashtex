import AppKit
import XCTest
@testable import FlashTeXMac

/// Code folding (EditorFolding.swift): region computation, range shifting,
/// unfold-on-reveal, Fold All / Unfold All, and the placeholder never entering
/// the storage (compile/save input).
@MainActor
final class EditorFoldingTests: XCTestCase {
    typealias EF = EditorFolding

    // MARK: environments

    func testEnvironmentRegionsSkipUnbalancedAndOneLine() {
        let s = """
        \\begin{itemize}
        \\item a
        \\begin{enumerate}
        \\item b
        \\end{enumerate}
        \\end{itemize}
        \\begin{open}
        never closed
        \\begin{oneline}\\end{oneline}
        """ as NSString
        let regions = EF.regions(in: s)
        let envs = regions.compactMap { r -> String? in
            if case .environment(let n) = r.kind { return n }
            return nil
        }
        XCTAssertEqual(envs, ["itemize", "enumerate"], "unbalanced open and one-line oneline are not foldable")
        let itemize = regions.first { if case .environment("itemize") = $0.kind { return true }; return false }!
        XCTAssertEqual(s.substring(with: itemize.header), "\\begin{itemize}")
        XCTAssertTrue(s.substring(with: itemize.hidden).hasPrefix("\\item a\n"), s.substring(with: itemize.hidden))
        XCTAssertTrue(s.substring(with: itemize.hidden).contains("\\end{itemize}"))
        let enumerate = regions.first { if case .environment("enumerate") = $0.kind { return true }; return false }!
        XCTAssertTrue(itemize.contains(enumerate.header.location), "nested enumerate sits inside itemize")
        XCTAssertEqual(EF.innermost(at: enumerate.header.location, in: regions)?.header, enumerate.header)
        XCTAssertEqual(EF.innermost(at: itemize.header.location, in: regions)?.header, itemize.header)
    }

    func testVerbatimEnvironmentIsFoldableAndInnerBeginsAreIgnored() {
        let s = "\\begin{verbatim}\n\\begin{itemize}\n\\end{verbatim}\n" as NSString
        let regions = EF.regions(in: s)
        XCTAssertEqual(regions.count, 1)
        XCTAssertEqual(regions[0].kind, .environment("verbatim"))
    }

    func testDocumentEnvironmentIsFoldable() {
        let s = "\\begin{document}\nHello.\n\\end{document}\n" as NSString
        let regions = EF.regions(in: s)
        XCTAssertTrue(regions.contains { if case .environment("document") = $0.kind { return true }; return false })
    }

    // MARK: sections

    func testSectionRegionsRunToTheNextSameOrHigherHeading() {
        let s = """
        \\section{A}
        body a
        \\subsection{B}
        body b
        \\section{C}
        body c
        \\end{document}
        """ as NSString
        let regions = EF.regions(in: s)
        let sections = regions.filter { if case .section = $0.kind { return true }; return false }
        XCTAssertEqual(sections.count, 3)
        XCTAssertEqual(s.substring(with: sections[0].header), "\\section{A}")
        XCTAssertTrue(s.substring(with: sections[0].hidden).contains("\\subsection{B}"))
        XCTAssertFalse(s.substring(with: sections[0].hidden).contains("\\section{C}"), "A stops before C")
        XCTAssertTrue(s.substring(with: sections[1].hidden).contains("body b"))
        XCTAssertFalse(s.substring(with: sections[1].hidden).contains("\\section{C}"))
        XCTAssertTrue(s.substring(with: sections[2].hidden).contains("body c"))
        XCTAssertFalse(s.substring(with: sections[2].hidden).contains("\\end{document}"), "section stops at \\end{document}")
    }

    func testPartAndChapterAreTheSameLevel() {
        let s = "\\part{P}\npart body\n\\chapter{Ch}\nch body\n" as NSString
        let regions = EF.regions(in: s).filter { if case .section = $0.kind { return true }; return false }
        XCTAssertEqual(regions.count, 2)
        XCTAssertFalse(s.substring(with: regions[0].hidden).contains("\\chapter{Ch}"))
    }

    func testSubparagraphIsTheLowestSectionLevel() {
        let s = "\\paragraph{P}\np\n\\subparagraph{S}\ns\n\\paragraph{Q}\nq\n" as NSString
        let regions = EF.regions(in: s).filter { if case .section = $0.kind { return true }; return false }
        XCTAssertEqual(regions.count, 3)
        XCTAssertTrue(s.substring(with: regions[0].hidden).contains("\\subparagraph{S}"))
        XCTAssertFalse(s.substring(with: regions[0].hidden).contains("\\paragraph{Q}"))
    }

    func testCommentedBeginDoesNotFold() {
        let s = "% \\begin{itemize}\n\\begin{center}\nx\n\\end{center}\n" as NSString
        let regions = EF.regions(in: s)
        XCTAssertEqual(regions.map { r -> String in
            if case .environment(let n) = r.kind { return n }
            return "?"
        }, ["center"])
    }

    // MARK: range shifting

    func testEditBeforeAFoldShiftsItAndEditInsideUnfolds() {
        let hidden = NSRange(location: 20, length: 10)
        XCTAssertEqual(EF.shift(hidden, edit: NSRange(location: 0, length: 0), replacementLength: 5),
                       NSRange(location: 25, length: 10))
        XCTAssertEqual(EF.shift(hidden, edit: NSRange(location: 0, length: 3), replacementLength: 0),
                       NSRange(location: 17, length: 10))
        XCTAssertNil(EF.shift(hidden, edit: NSRange(location: 22, length: 1), replacementLength: 0), "inside")
        XCTAssertNil(EF.shift(hidden, edit: NSRange(location: 20, length: 0), replacementLength: 1), "insertion at start")
        XCTAssertNil(EF.shift(hidden, edit: NSRange(location: 30, length: 0), replacementLength: 1), "insertion at end")
        XCTAssertNil(EF.shift(hidden, edit: NSRange(location: 19, length: 1), replacementLength: 0), "deletion touching the start")
        XCTAssertEqual(EF.shift(hidden, edit: NSRange(location: 40, length: 0), replacementLength: 3), hidden, "after")
    }

    func testStoreDropsAFoldWhoseEnvironmentNoLongerParses() {
        let text = "\\begin{a}\nbody\n\\end{a}\n" as NSString
        let store = EditorFoldStore()
        XCTAssertTrue(store.foldInnermost(at: 0, in: text))
        XCTAssertEqual(store.folds.count, 1)
        // Delete the \\end{a} line. The fold's hidden range no longer matches a region.
        let broken = "\\begin{a}\nbody\n" as NSString
        let end = text.range(of: "\\end{a}\n")
        store.applyEdit(end, replacementLength: 0, newText: broken)
        XCTAssertTrue(store.folds.isEmpty, "unbalanced \\begin drops the fold")
    }

    func testStoreShiftsFoldsAcrossAnEditBeforeThem() {
        let text = "head\n\\begin{a}\nbody\n\\end{a}\n" as NSString
        let store = EditorFoldStore()
        XCTAssertTrue(store.foldInnermost(at: 5, in: text))
        let hiddenBefore = store.folds[0].hidden
        let grown = "XXXXhead\n\\begin{a}\nbody\n\\end{a}\n" as NSString
        store.applyEdit(NSRange(location: 0, length: 0), replacementLength: 4, newText: grown)
        XCTAssertEqual(store.folds.count, 1)
        XCTAssertEqual(store.folds[0].hidden.location, hiddenBefore.location + 4)
        XCTAssertEqual(store.folds[0].hidden.length, hiddenBefore.length)
    }

    func testUnfoldCoveringRevealsATargetInsideAFold() {
        let text = "\\begin{a}\nsecret\n\\end{a}\n" as NSString
        let store = EditorFoldStore()
        XCTAssertTrue(store.foldInnermost(at: 0, in: text))
        let secret = text.range(of: "secret")
        XCTAssertTrue(store.unfoldCovering(secret))
        XCTAssertTrue(store.folds.isEmpty)
        XCTAssertFalse(store.unfoldCovering(secret), "already unfolded")
    }

    func testFoldAllAndUnfoldAll() {
        let text = """
        \\section{A}
        a
        \\begin{a}
        x
        \\end{a}
        \\section{B}
        b
        """ as NSString
        let store = EditorFoldStore()
        store.foldAll(in: text)
        XCTAssertGreaterThanOrEqual(store.folds.count, 3, "section A, environment a, section B")
        store.unfoldAll()
        XCTAssertTrue(store.folds.isEmpty)
    }

    func testKeystrokeWithNoFoldsDoesNotRescanRegions() {
        let text = "\\begin{a}\nbody\n\\end{a}\nplain" as NSString
        let store = EditorFoldStore()
        store.applyEdit(NSRange(location: (text as NSString).length, length: 0), replacementLength: 1, newText: text)
        XCTAssertEqual(store.regionComputeCount, 0, "no folds: applying an edit must not scan")
        _ = store.foldInnermost(at: 0, in: text)
        let afterFold = store.regionComputeCount
        store.unfoldAll()
        store.applyEdit(NSRange(location: 0, length: 0), replacementLength: 1, newText: ("x" as NSString))
        XCTAssertEqual(store.regionComputeCount, afterFold, "unfolded already: still no scan on a keystroke")
    }

    func testCachedRegionsAreReusedUntilAnEdit() {
        let text = "\\begin{a}\nbody\n\\end{a}\n" as NSString
        let store = EditorFoldStore()
        _ = store.regions(in: text)
        _ = store.regions(in: text)
        XCTAssertEqual(store.regionComputeCount, 1)
        store.applyEdit(NSRange(location: 0, length: 0), replacementLength: 1, newText: text)
        _ = store.regions(in: text)
        XCTAssertEqual(store.regionComputeCount, 2)
    }

    func testPlaceholderNeverEntersTheStorageString() {
        let original = "\\begin{a}\nbody\n\\end{a}\n"
        let store = EditorFoldStore()
        XCTAssertTrue(store.foldInnermost(at: 0, in: original as NSString))
        XCTAssertEqual(original, "\\begin{a}\nbody\n\\end{a}\n", "the source string is untouched")
        XCTAssertFalse(store.folds.isEmpty)
        XCTAssertFalse((original as NSString).substring(with: store.folds[0].hidden).contains("…"))
    }

    func testVimLinewiseMotionSkipsAFold() {
        let text = "\\begin{a}\nline2\nline3\n\\end{a}\nnext\n" as NSString
        let store = EditorFoldStore()
        XCTAssertTrue(store.foldInnermost(at: 0, in: text))
        let hidden = store.folds[0].hidden
        let inside = hidden.location + 1
        let after = store.adjustLinewise(from: 0, to: inside)
        XCTAssertEqual(after, NSMaxRange(hidden), "j over a fold lands after it")
        let up = store.adjustLinewise(from: NSMaxRange(hidden), to: inside)
        XCTAssertEqual(up, hidden.location - 1, "k over a fold lands on the header line")
    }

    // MARK: large document

    func testRegionComputationAt560KB() throws {
        let load = IMEHarness.loadAverage1() ?? 0
        if load > 20, ProcessInfo.processInfo.environment["FLASHTEX_BENCH_FORCE"] == nil {
            throw XCTSkip("load \(load) > 20")
        }
        let text = LargeDocumentEditorTests.proseDocument(bytes: 560_000) as NSString
        let pairs = EditorNavigation.environmentPairs(in: text)
        let outline = DocumentOutline.scan(text as String)
        let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        let regions = EditorFolding.regions(in: text, pairs: pairs, outline: outline)
        let c1 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        let ms = Double(c1 - c0) / 1e6
        XCTAssertFalse(regions.isEmpty)
        XCTAssertLessThan(ms, 2.0, "derivation from cached pairs+outline: \(ms) ms")
        let store = EditorFoldStore()
        store.replaceCache(regions: regions)
        let r0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        _ = store.regions(in: text)
        let r1 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        XCTAssertEqual(store.regionComputeCount, 0)
        XCTAssertLessThan(Double(r1 - r0) / 1e6, 0.2, "cache hit")
    }

    // MARK: hosted editor (TextKit 1 glyph hiding)

    func testHostedFoldHidesGlyphsLeavesStorageAndUnfoldsOnReveal() async throws {
        HostedWindowSupport.prepare()
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless()
        defer { window.orderOut(nil) }
        tv.installFolding()
        let original = "\\begin{a}\nsecret-body\n\\end{a}\nafter\n"
        tv.string = original
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        tv.layoutManager?.ensureLayout(for: tv.textContainer!)
        XCTAssertTrue(tv.folds.foldInnermost(at: 0, in: original as NSString), "\\begin{a}…\\end{a} is foldable")
        tv.foldingDidChange()
        XCTAssertEqual(tv.string, original, "folded characters stay in the storage")
        XCTAssertFalse(tv.string.contains("…"), "the placeholder is not in the text")
        XCTAssertEqual(tv.folds.folds.count, 1)
        let hidden = tv.folds.folds[0].hidden
        XCTAssertTrue(tv.string.contains("secret-body"))
        tv.layoutManager?.ensureLayout(for: tv.textContainer!)
        if let lm = tv.layoutManager, lm.numberOfGlyphs > 0 {
            let g = min(lm.glyphIndexForCharacter(at: hidden.location), lm.numberOfGlyphs - 1)
            XCTAssertTrue(lm.propertyForGlyph(at: g).contains(.null), "TextKit-1 null glyph hides the body")
        } else {
            XCTFail("no glyphs after fold")
        }
        XCTAssertTrue(tv.folds.unfoldCovering((original as NSString).range(of: "secret-body")))
        XCTAssertTrue(tv.folds.folds.isEmpty)
        tv.layoutManager?.ensureLayout(for: tv.textContainer!)
        if let lm = tv.layoutManager, lm.numberOfGlyphs > 0 {
            let g = min(lm.glyphIndexForCharacter(at: hidden.location), lm.numberOfGlyphs - 1)
            XCTAssertFalse(lm.propertyForGlyph(at: g).contains(.null))
        }
    }
}
