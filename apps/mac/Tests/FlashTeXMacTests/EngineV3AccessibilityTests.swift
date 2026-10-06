import AppKit
import SwiftUI
import XCTest
import FlashTeXAccessibility
import HostedWindows
@testable import FlashTeXMac

/// VoiceOver over the engine-v3 preview (DESIGN §10 app parity, gaps C18 and
/// C19): each page is the v2 pane's page landmark (same labels, the same
/// line grouping, "Go to source" on each line) read from the display list's
/// glyphs through pdfTeX's glyph-name table, and the Pages rotor lists every
/// page and loads one the pane has not built. The pane tests need a built
/// `flashtex-host` and TeX Live (skipped otherwise).
@MainActor
final class EngineV3AccessibilityTests: XCTestCase {
    struct Host: View {
        let model: ShellModel
        var body: some View { EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom) }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-ax-tests-\(getpid())")
    private var env = EnvironmentOverride()
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore() }

    // MARK: glyph names

    func testGlyphNamesMapByPdfTeXsTableThenTheAGLRules() {
        let table = EngineV3GlyphText.parse("""
        % comment
        \\pdfglyphtounicode{arrowright}{2192}
        \\pdfglyphtounicode{ff}{0066 0066}
        \\pdfglyphtounicode{bad}{ZZZZ}
        """)
        XCTAssertEqual(table, ["arrowright": "\u{2192}", "ff": "ff"])
        func t(_ n: String) -> String? { EngineV3GlyphText.text(forName: n, table: table) }
        XCTAssertEqual(t("arrowright"), "\u{2192}")
        XCTAssertEqual(t("ff"), "ff")
        XCTAssertEqual(t("a"), "a")
        XCTAssertEqual(t("seven"), "7")
        XCTAssertEqual(t("uni00E9"), "\u{00E9}")
        XCTAssertEqual(t("uni00660069"), "fi")
        XCTAssertEqual(t("u1D400"), "\u{1D400}")
        XCTAssertEqual(t("a.sc"), "a")
        XCTAssertEqual(t("f_f_i"), "ffi")
        XCTAssertNil(t("notaglyphname"))
        XCTAssertNil(t(".notdef"))
    }

    /// The real table is TeX Live's `glyphtounicode.tex`, as pdfTeX's
    /// `\pdfgentounicode` uses it.
    func testTeXLivesTableIsReadWhenPresent() throws {
        try XCTSkipIf(EngineV3GlyphText.locate("glyphtounicode.tex") == nil, "no TeX Live")
        let t = EngineV3GlyphText.table
        XCTAssertGreaterThan(t.count, 4000)
        XCTAssertEqual(t["ffi"], "ffi")
        XCTAssertEqual(t["similarequal"], "\u{2243}")
        XCTAssertEqual(t["dotlessi"], "\u{0131}")
    }

    // MARK: the pane

    func waitUntil(_ what: String, timeout: TimeInterval = 120, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    func pane(_ text: String, pages: Int, height: CGFloat = 820) async throws -> (ShellModel, EngineV3PagesView, NSWindow) {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: text, named: "main.tex")
        model.engineV3Enabled = true
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: height), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        model.engineV3.start(model: model)
        try await EngineV3TestHost.awaitReady(model.engineV3)
        let s = model.engineV3
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && s.pageCount == pages && (0 ..< pages).allSatisfy { s.pages[$0] != nil } }
        window.layoutIfNeeded()
        let view = try XCTUnwrap(s.view)
        view.update(revision: s.layoutRevision, zoom: 1)
        view.relayout()
        return (model, view, window)
    }

    static let document = """
    \\documentclass{article}
    \\begin{document}
    Hello world, this is a test.

    Second paragraph: fine office.
    \\newpage
    Page two text.
    \\end{document}

    """

    /// A page is a landmark labelled as on v2 ("Page 1 of 2, 3 lines"), its
    /// value is its text, each line is an element labelled as on v2 with a
    /// "Go to source" action that selects the line in the editor.
    func testAPageReadsItsTextLineByLineAsOnV2() async throws {
        let (model, pages, window) = try await pane(Self.document, pages: 2)
        defer { model.engineV3.stop(); window.contentView = nil }
        let page = try XCTUnwrap(pages.heldPageView(0))
        XCTAssertTrue(page.isAccessibilityElement())
        XCTAssertEqual(page.accessibilityRole(), .group)
        XCTAssertEqual(page.accessibilitySubrole(), PreviewAccessibility.landmarkSubrole)
        XCTAssertEqual(page.accessibilityRoleDescription(), PreviewAccessibility.pageRoleDescription)
        let lines = page.axLines.map(\.text)
        XCTAssertEqual(lines, ["Hello world, this is a test.", "Second paragraph: fine office.", "1"])
        XCTAssertEqual(page.accessibilityLabel(), V2PageText.pageLabel(number: 1, totalPages: 2, lineCount: 3))
        XCTAssertEqual(page.accessibilityValue() as? String, lines.joined(separator: "\n"))
        let children = try XCTUnwrap(page.accessibilityChildren() as? [PreviewAXElement])
        XCTAssertEqual(children.count, 3)
        XCTAssertEqual(children[1].accessibilityLabel(), V2PageText.lineLabel(page: 1, line: page.axLines[1]))
        XCTAssertEqual(children[1].accessibilityRoleDescription(), PreviewAccessibility.lineRoleDescription)
        // The line's frame lies on the page, where the text is drawn.
        XCTAssertTrue(page.bounds.contains(children[1].viewFrame), "\(children[1].viewFrame) in \(page.bounds)")
        // Go to source: the second paragraph's line in main.tex.
        let action = try XCTUnwrap(children[1].accessibilityCustomActions()?.first)
        XCTAssertEqual(action.name, PreviewAccessibility.goToSourceAction)
        XCTAssertTrue(action.handler?() ?? false)
        try await waitUntil("the selection") { model.selection?.path == "main.tex" }
        let sel = try XCTUnwrap(model.selection)
        let lineStart = (Self.document as NSString).range(of: "Second paragraph").location
        XCTAssertEqual(sel.nsRange.location, lineStart, "Go to source lands on the line's first character")
        // The element survives a zoom and moves with the page.
        let before = children[1].viewFrame
        model.previewZoom = 2
        pages.update(revision: model.engineV3.layoutRevision, zoom: 2)
        let after = try XCTUnwrap(page.accessibilityChildren() as? [PreviewAXElement])
        XCTAssertTrue(after[1] === children[1], "the same element after a zoom")
        XCTAssertEqual(after[1].viewFrame.width, before.width * 2, accuracy: 1)
    }

    /// The Pages rotor lists every page ("Page 5 of 6"), searches as v2's
    /// does, and loads a page the pane has not built: it scrolls there and
    /// hands VoiceOver that page's view.
    func testPagesRotorListsEveryPageAndLoadsOneNotBuilt() async throws {
        let body = (1 ... 6).map { "Text of page \($0).\\newpage" }.joined(separator: "\n")
        let doc = "\\documentclass{article}\n\\begin{document}\n\(body)\n\\end{document}\n"
        let (model, pages, window) = try await pane(doc, pages: 6, height: 500)
        defer { model.engineV3.stop(); window.contentView = nil }
        let rotor = pages.pagesRotor
        XCTAssertEqual(rotor.items.map(\.label), (1 ... 6).map { "Page \($0) of 6" })
        XCTAssertEqual(pages.accessibilityCustomRotors().map(\.label), [PreviewPagesRotor.rotorLabel])
        XCTAssertEqual(pages.heldPageView(0)?.accessibilityCustomRotors().map(\.label), [PreviewPagesRotor.rotorLabel])
        XCTAssertNil(pages.heldPageView(5), "page 6 is not built at the top")
        // Page 6's result is a loading token; choosing it scrolls and returns its view.
        let r = rotor.result(for: rotor.items[5])
        XCTAssertNil(r.targetElement)
        XCTAssertEqual((r.itemLoadingToken as? NSNumber)?.intValue, 6)
        let loaded = try XCTUnwrap(rotor.load(NSNumber(value: 6)) as? EngineV3PageView)
        XCTAssertEqual(loaded.previewPageNumber, 6)
        XCTAssertEqual(rotor.loads, [6])
        let clip = try XCTUnwrap(pages.enclosingScrollView?.contentView)
        XCTAssertTrue(clip.bounds.intersects(loaded.frame), "page 6 is in view")
        XCTAssertEqual(loaded.axLines.first?.text, "Text of page 6.")
        // Built now: its result targets the view.
        XCTAssertTrue(rotor.result(for: rotor.items[5]).targetElement as? EngineV3PageView === loaded)
        // v2's search rules: next after page 2 is page 3; the filter "5" finds page 5.
        XCTAssertEqual(PreviewPagesRotor.resolve(items: rotor.items, start: 2, forward: true, filter: "")?.number, 3)
        XCTAssertEqual(PreviewPagesRotor.resolve(items: rotor.items, start: nil, forward: true, filter: "5")?.number, 5)
    }

    // MARK: accents

    /// A spacing accent over or under a letter is that letter's combining
    /// mark, whatever its vertical offset (a capital's accent is raised);
    /// the result is NFC. A lone accent glyph (no letter under it) stays.
    func testAnAccentGlyphCombinesWithTheLetterItSitsOn() {
        typealias G = EngineV3PageText.Glyph
        func g(_ i: Int, _ name: String?, _ text: String, x: Double, w: Double = 5, baseline: Double = 100, inkY: Double? = nil) -> G {
            let cell = CGRect(x: x, y: baseline - 7.5, width: w, height: 10)
            let ink = inkY.map { CGRect(x: x + 1, y: $0, width: w - 2, height: 2) } ?? cell.insetBy(dx: 0.5, dy: 2)
            return G(index: i, name: name, text: text, cell: cell, ink: ink, baseline: baseline)
        }
        // OT1 "café": the acute is drawn before the e, over it.
        var glyphs = [g(0, "c", "c", x: 0), g(1, "a", "a", x: 5), g(2, "f", "f", x: 10), g(3, "acute", "\u{00B4}", x: 15, inkY: 90), g(4, "e", "e", x: 15)]
        var words = EngineV3PageText.words(glyphs)
        XCTAssertEqual(words.map(\.text).joined(), "café")
        XCTAssertEqual(words.last?.text.unicodeScalars.count, 1, "NFC: one scalar")
        // "\'E": the accent raised 2.5 pt above a capital, baseline shifted too.
        glyphs = [g(0, "acute", "\u{00B4}", x: 1, baseline: 97.5, inkY: 85), g(1, "E", "E", x: 0, w: 7)]
        words = EngineV3PageText.words(glyphs)
        XCTAssertEqual(words.map(\.text), ["\u{00C9}"])
        // "\c{c}": the cedilla below.
        glyphs = [g(0, "c", "c", x: 0), g(1, "cedilla", "\u{00B8}", x: 0, inkY: 101)]
        XCTAssertEqual(EngineV3PageText.words(glyphs).map(\.text), ["\u{00E7}"])
        // "\d{o}" and "\b{o}": a period and a macron wholly below the letter.
        glyphs = [g(0, "o", "o", x: 0), g(1, "period", ".", x: 0, inkY: 102)]
        XCTAssertEqual(EngineV3PageText.words(glyphs).map(\.text), ["\u{1ECD}"], "ọ (o, dot below)")
        glyphs = [g(0, "o", "o", x: 0), g(1, "macron", "\u{00AF}", x: 0, inkY: 102)]
        XCTAssertEqual(EngineV3PageText.words(glyphs).map(\.text), ["o\u{0331}"], "o, macron below (no precomposed form)")
        // A period on the next line, under the letter: not a dot below (beyond 0.8 em).
        glyphs = [g(0, "o", "o", x: 0), g(1, "period", ".", x: 0, baseline: 112, inkY: 111)]
        XCTAssertEqual(EngineV3PageText.words(glyphs).map(\.text), ["o", "."])
        // A sentence's period on the baseline after a letter is punctuation.
        glyphs = [g(0, "o", "o", x: 0), g(1, "period", ".", x: 5, inkY: 98.5)]
        XCTAssertEqual(EngineV3PageText.words(glyphs).map(\.text), ["o", "."])
        // A lone accent (\verb or a spacing \'{}) stays as it is.
        glyphs = [g(0, "acute", "\u{00B4}", x: 0), g(1, "a", "a", x: 20)]
        XCTAssertEqual(EngineV3PageText.words(glyphs).map(\.text), ["\u{00B4}", "a"])
    }

    static func accented(_ preamble: String) -> String {
        """
        \\documentclass{article}
        \(preamble)
        \\begin{document}
        caf\\'e \\'Ecole na\\"\\i ve gar\\c{c}on \\"o \\`a \\^o \\~n \\d{o} \\b{o}
        \\end{document}

        """
    }

    /// OT1 (the default): accents are separate glyphs; the page reads the accented letters.
    func testOT1AccentsReadAsAccentedLetters() async throws {
        let (model, pages, window) = try await pane(Self.accented(""), pages: 1)
        defer { model.engineV3.stop(); window.contentView = nil }
        let lines = try XCTUnwrap(pages.heldPageView(0)).axLines.map(\.text)
        XCTAssertEqual(lines.first, "caf\u{00E9} \u{00C9}cole na\u{00EF}ve gar\u{00E7}on \u{00F6} \u{00E0} \u{00F4} \u{00F1} \u{1ECD} o\u{0331}", "\(lines)")
    }

    /// T1: precomposed glyphs (eacute, ...) read the same.
    func testT1PrecomposedGlyphsReadTheSame() async throws {
        let (model, pages, window) = try await pane(Self.accented("\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}"), pages: 1)
        defer { model.engineV3.stop(); window.contentView = nil }
        let lines = try XCTUnwrap(pages.heldPageView(0)).axLines.map(\.text)
        XCTAssertEqual(lines.first, "caf\u{00E9} \u{00C9}cole na\u{00EF}ve gar\u{00E7}on \u{00F6} \u{00E0} \u{00F4} \u{00F1} \u{1ECD} o\u{0331}", "\(lines)")
    }
}
