import AppKit
import XCTest
import FlashTeXEditorCore
import HostedWindows
@testable import FlashTeXMac

/// Paste an image as a figure (PASTE-IMAGE-FIGURE): the pure decisions in
/// `PasteImageFigure` (FlashTeXEditorCore, shared with the iPad) and the
/// Mac's pasteboard reading, file writing and one-undo-step insertion.
/// Pasteboards are private named ones, so a run never touches the clipboard.
final class PasteImageFigureTests: XCTestCase {
    typealias P = PasteImageFigure

    // MARK: naming

    func testBaseNameIsATimestamp() {
        let date = Date(timeIntervalSince1970: 1_790_000_000) // 2026-09-21 14:13:20 UTC
        XCTAssertEqual(P.baseName(for: date, timeZone: TimeZone(identifier: "UTC")!), "pasted-20260921-141320")
    }

    func testUniqueFileNameNeverReusesATakenName() {
        XCTAssertEqual(P.uniqueFileName(base: "pasted-1", fileExtension: "png") { _ in false }, "pasted-1.png")
        let taken: Set = ["pasted-1.png", "pasted-1-2.png"]
        XCTAssertEqual(P.uniqueFileName(base: "pasted-1", fileExtension: "png") { taken.contains($0) }, "pasted-1-3.png")
    }

    func testSanitizedBaseNameIsTeXSafe() {
        XCTAssertEqual(P.sanitizedBaseName("Screen Shot 2026-09-30 at 14.02.11"), "Screen-Shot-2026-09-30-at-14-02-11")
        XCTAssertEqual(P.sanitizedBaseName("résumé_v2"), "r-sum-_v2")
        XCTAssertEqual(P.sanitizedBaseName("%#{}"), "pasted")
        XCTAssertEqual(P.sanitizedBaseName("--a--b--"), "a-b")
    }

    func testFolderNormalization() {
        XCTAssertEqual(P.normalizedFolder("figures"), "figures")
        XCTAssertEqual(P.normalizedFolder(" ./img/pasted/ "), "img/pasted")
        XCTAssertEqual(P.normalizedFolder(""), "")
        XCTAssertEqual(P.normalizedFolder("."), "")
        XCTAssertNil(P.normalizedFolder("/tmp"))
        XCTAssertNil(P.normalizedFolder("~/Pictures"))
        XCTAssertNil(P.normalizedFolder("../outside"))
        XCTAssertNil(P.normalizedFolder("my figures"))
        XCTAssertNil(P.normalizedFolder("a//b"))
    }

    func testGraphicspathDecidesTheFolderUnlessItListsTheConfiguredOne() {
        let doc = "\\documentclass{article}\n\\graphicspath{{./img/}{plots/}}\n\\begin{document}\n\\end{document}\n"
        XCTAssertEqual(P.graphicsPaths(in: doc), ["img", "plots"])
        XCTAssertEqual(P.imageFolder(configured: "figures", rootText: doc), "img")
        XCTAssertEqual(P.imageFolder(configured: "plots", rootText: doc), "plots")
        XCTAssertEqual(P.imageFolder(configured: "figures", rootText: "\\documentclass{article}\n"), "figures")
        XCTAssertEqual(P.imageFolder(configured: "../bad", rootText: ""), "figures", "an invalid setting falls back")
        XCTAssertEqual(P.imageFolder(configured: "figures", rootText: "% \\graphicspath{{img/}}\n"), "figures", "a commented graphicspath is not one")
        XCTAssertEqual(P.graphicsPath(folder: "", fileName: "a.png"), "a.png")
    }

    // MARK: graphicx

    func testGraphicxDetection() {
        XCTAssertTrue(P.loadsGraphicx(in: "\\documentclass{article}\n\\usepackage{graphicx}\n"))
        XCTAssertTrue(P.loadsGraphicx(in: "\\documentclass{article}\n\\usepackage[draft]{amsmath, graphicx ,xcolor}\n"))
        XCTAssertTrue(P.loadsGraphicx(in: "\\RequirePackage{graphicx}\n"))
        XCTAssertTrue(P.loadsGraphicx(in: "\\documentclass[11pt]{beamer}\n"), "beamer loads graphicx itself")
        XCTAssertFalse(P.loadsGraphicx(in: "\\documentclass{article}\n% \\usepackage{graphicx}\n"))
        XCTAssertFalse(P.loadsGraphicx(in: "\\documentclass{article}\n\\usepackage{graphics}\n"))
        XCTAssertFalse(P.loadsGraphicx(in: "\\documentclass{article}\n\\begin{document}\n\\usepackage{graphicx}\n"),
                       "only the preamble counts")
    }

    func testGraphicxGoesAfterTheLastUsepackage() {
        let doc = "\\documentclass{article}\n\\usepackage{amsmath}\n\\usepackage[T1]{fontenc}\n\n\\begin{document}\nHi\n\\end{document}\n"
        let g = P.graphicxInsertion(in: doc)
        XCTAssertEqual(g?.text, "\\usepackage{graphicx}\n")
        let out = (doc as NSString).replacingCharacters(in: NSRange(location: g!.location, length: 0), with: g!.text)
        XCTAssertEqual(out, "\\documentclass{article}\n\\usepackage{amsmath}\n\\usepackage[T1]{fontenc}\n\\usepackage{graphicx}\n\n\\begin{document}\nHi\n\\end{document}\n")
    }

    func testGraphicxGoesAfterDocumentclassWithoutPackages() {
        let doc = "\\documentclass{article}\n\\begin{document}\n\\end{document}"
        let g = P.graphicxInsertion(in: doc)!
        XCTAssertEqual((doc as NSString).replacingCharacters(in: NSRange(location: g.location, length: 0), with: g.text),
                       "\\documentclass{article}\n\\usepackage{graphicx}\n\\begin{document}\n\\end{document}")
        let bare = "\\documentclass{article}"
        let h = P.graphicxInsertion(in: bare)!
        XCTAssertEqual(h, .init(location: 23, text: "\n\\usepackage{graphicx}"))
    }

    func testNoGraphicxInsertionWhenLoadedOrNoPreamble() {
        XCTAssertNil(P.graphicxInsertion(in: "\\documentclass{article}\n\\usepackage{graphicx}\n"))
        XCTAssertNil(P.graphicxInsertion(in: "\\section{Intro}\nText.\n"), "an included file has no preamble to edit")
    }

    // MARK: context

    func testContextDetection() {
        let fig = "\\begin{figure}\n  \n\\end{figure}\n"
        XCTAssertTrue(P.context(in: fig, caret: 17).inFigure)
        XCTAssertFalse(P.context(in: fig, caret: fig.utf16.count).inFigure, "after \\end{figure}")
        let math = "Let $x + $ be."
        XCTAssertTrue(P.context(in: math, caret: 9).inMath)
        XCTAssertFalse(P.context(in: math, caret: 2).inMath)
        let eq = "\\begin{equation}\n  a = \n\\end{equation}\n"
        XCTAssertTrue(P.context(in: eq, caret: 23).inMath)
        let para = "  Some text here.\n"
        let mid = P.context(in: para, caret: 6)
        XCTAssertEqual(mid, .init(inMath: false, inFigure: false, textBefore: true, textAfter: true, indent: "  "))
        let start = P.context(in: para, caret: 2)
        XCTAssertFalse(start.textBefore)
        XCTAssertTrue(start.textAfter)
        XCTAssertFalse(P.context(in: "a\n\n", caret: 2).textAfter)
    }

    // MARK: snippets

    func testFigureSnippetSelectsTheCaption() {
        let s = P.snippet(path: "figures/pasted-1.png", label: "pasted-1", options: .init(), context: .init())
        XCTAssertEqual(s.text, """
        \\begin{figure}[htbp]
          \\centering
          \\includegraphics[width=0.8\\linewidth]{figures/pasted-1.png}
          \\caption{Caption}
          \\label{fig:pasted-1}
        \\end{figure}
        """)
        XCTAssertEqual((s.text as NSString).substring(with: s.selection), "Caption")
    }

    func testMidLineFigureGoesOnItsOwnLinesAtTheLineIndent() {
        let ctx = P.Context(textBefore: true, textAfter: true, indent: "\t")
        let s = P.snippet(path: "a.pdf", label: "a", options: .init(width: "", indentUnit: "\t"), context: ctx)
        XCTAssertEqual(s.text, "\n\t\\begin{figure}[htbp]\n\t\t\\centering\n\t\t\\includegraphics{a.pdf}\n\t\t\\caption{Caption}\n\t\t\\label{fig:a}\n\t\\end{figure}\n\t")
    }

    func testBareIncludegraphicsInFiguresMathAndWhenWrappingIsOff() {
        let expected = "\\includegraphics[width=0.8\\linewidth]{f/a.png}"
        for ctx in [P.Context(inFigure: true, textBefore: true), P.Context(inMath: true)] {
            let s = P.snippet(path: "f/a.png", label: "a", options: .init(), context: ctx)
            XCTAssertEqual(s.text, expected)
            XCTAssertEqual(s.selection, NSRange(location: (expected as NSString).length, length: 0))
        }
        XCTAssertEqual(P.snippet(path: "f/a.png", label: "a", options: .init(wrapInFigure: false), context: .init()).text, expected)
        XCTAssertEqual(P.snippet(path: "a.png", label: "a", options: .init(width: "height=3cm"), context: .init(inMath: true)).text,
                       "\\includegraphics[height=3cm]{a.png}", "a key=value width is the whole option list")
    }

    // MARK: plan

    func testPlanAddsGraphicxAndShiftsTheSelection() {
        let doc = "\\documentclass{article}\n\\begin{document}\n\n\\end{document}\n"
        let caret = ("\\documentclass{article}\n\\begin{document}\n" as NSString).length
        let plan = P.plan(text: doc, selection: NSRange(location: caret, length: 0), path: "figures/x.png", label: "x",
                          options: .init(), mathMode: false, ensureGraphicx: true)!
        XCTAssertTrue(plan.addsGraphicx)
        XCTAssertEqual(plan.placement, .figure)
        let out = apply(plan.edits, to: doc)
        XCTAssertTrue(out.hasPrefix("\\documentclass{article}\n\\usepackage{graphicx}\n\\begin{document}\n\\begin{figure}[htbp]\n"))
        XCTAssertEqual((out as NSString).substring(with: plan.selection), "Caption")
    }

    func testPlanInAnIncludedFileTouchesOnlyTheSnippet() {
        let doc = "\\section{A}\n\n"
        let plan = P.plan(text: doc, selection: NSRange(location: 12, length: 0), path: "x.png", label: "x",
                          options: .init(), ensureGraphicx: false)!
        XCTAssertEqual(plan.edits.count, 1)
        XCTAssertFalse(plan.addsGraphicx)
    }

    func testPlanReplacesTheSelection() {
        let doc = "\\documentclass{article}\n\\usepackage{graphicx}\n\\begin{document}\n\\begin{figure}\nOLD\n\\end{figure}\n\\end{document}\n"
        let r = (doc as NSString).range(of: "OLD")
        let plan = P.plan(text: doc, selection: r, path: "y.png", label: "y", options: .init(), ensureGraphicx: true)!
        XCTAssertEqual(plan.placement, .bare)
        XCTAssertFalse(plan.addsGraphicx, "already loaded")
        let out = apply(plan.edits, to: doc)
        XCTAssertTrue(out.contains("\\begin{figure}\n\\includegraphics[width=0.8\\linewidth]{y.png}\n\\end{figure}"))
        XCTAssertEqual(plan.selection.length, 0)
    }

    func testPackagesThatLoadGraphicx() {
        for pkg in ["tikz", "pgf", "pgfplots", "adjustbox", "mwe"] {
            XCTAssertTrue(P.loadsGraphicx(in: "\\documentclass{article}\n\\usepackage{\(pkg)}\n"), pkg)
        }
    }

    func testThePreambleIsRefused() {
        let doc = "\\documentclass{article}\n\n\\begin{document}\n\n\\end{document}\n"
        XCTAssertTrue(P.context(in: doc, caret: 24).inPreamble)
        XCTAssertNil(P.plan(text: doc, selection: NSRange(location: 24, length: 0), path: "a.png", label: "a", options: .init(), ensureGraphicx: true))
        XCTAssertFalse(P.context(in: doc, caret: 41).inPreamble)
        XCTAssertFalse(P.context(in: "% \\begin{document}\n\n", caret: 0).inPreamble, "a commented \\begin{document}")
        XCTAssertFalse(P.context(in: "\\section{A}\n", caret: 0).inPreamble, "an included file has no preamble")
    }

    func testACommentedBeginOpensNothing() {
        let doc = "% \\begin{figure}\nText \n"
        XCTAssertFalse(P.context(in: doc, caret: 22).inFigure)
        XCTAssertTrue(P.context(in: "\\begin{figure}% note\n\n", caret: 21).inFigure, "a trailing comment keeps the \\begin")
    }

    func testTextAfterIsMeasuredFromTheSelectionEnd() {
        let doc = "keep REPLACED\n"
        let ctx = P.context(in: doc, caret: 5, selectionEnd: 13)
        XCTAssertTrue(ctx.textBefore)
        XCTAssertFalse(ctx.textAfter, "nothing follows the selection on its line")
    }

    func testAMidLineSplitLeavesNoTrailingOrLeadingBlanks() {
        let doc = "One two.   three\n"
        let plan = P.plan(text: doc, selection: NSRange(location: 9, length: 0), path: "a.png", label: "a",
                          options: .init(), mathMode: false, ensureGraphicx: false)!
        let out = apply(plan.edits, to: doc)
        XCTAssertTrue(out.hasPrefix("One two.\n\\begin{figure}[htbp]\n"), out)
        XCTAssertTrue(out.hasSuffix("\\end{figure}\nthree\n"), out)
    }

    private func apply(_ edits: [LaTeXEditing.LineEdit], to text: String) -> String {
        let s = NSMutableString(string: text)
        for e in edits.sorted(by: { $0.range.location > $1.range.location }) { s.replaceCharacters(in: e.range, with: e.replacement) }
        return s as String
    }

    // MARK: Mac: pasteboard, file, editor

    private var pasteboards: [NSPasteboard] = []
    private var directories: [URL] = []
    private var suites: [String] = []

    override func tearDown() {
        suites.forEach { UserDefaults.standard.removePersistentDomain(forName: $0) }
        pasteboards.forEach { $0.releaseGlobally() }
        directories.forEach { try? FileManager.default.removeItem(at: $0) }
        super.tearDown()
    }

    /// Preferences on a throwaway defaults suite (default on), never the app's.
    @MainActor
    private func isolatedPreferences() -> PasteImagePreferences {
        let suite = "paste-image-test-\(UUID().uuidString)"
        suites.append(suite)
        return PasteImagePreferences(defaults: UserDefaults(suiteName: suite)!)
    }

    private func pasteboard() -> NSPasteboard {
        let pb = NSPasteboard(name: .init("dev.flashtex.test.paste-image.\(UUID().uuidString)"))
        pb.clearContents()
        pasteboards.append(pb)
        return pb
    }

    private func projectDirectory() throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("paste-image-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        directories.append(dir)
        return dir
    }

    private static func tiffData() -> Data {
        let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 4, pixelsHigh: 3, bitsPerSample: 8, samplesPerPixel: 4,
                                   hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
        return rep.tiffRepresentation!
    }

    @MainActor
    func testReadingThePasteboard() throws {
        let text = pasteboard()
        text.setString("hello", forType: .string)
        text.setData(Self.tiffData(), forType: .tiff)
        XCTAssertNil(PasteImage.read(text), "text on the pasteboard is an ordinary paste")

        let tiff = pasteboard()
        tiff.setData(Self.tiffData(), forType: .tiff)
        guard case .data(_, .tiff) = PasteImage.read(tiff) else { return XCTFail("TIFF data is an image paste") }

        let pdf = pasteboard()
        pdf.declareTypes([.tiff, .pdf], owner: nil)
        pdf.setData(Self.tiffData(), forType: .tiff)
        pdf.setData(Data("%PDF-1.4".utf8), forType: .pdf)
        guard case .data(_, .pdf) = PasteImage.read(pdf) else { return XCTFail("PDF (vector) wins over its TIFF rendering") }

        // Text-like flavours beside an image are text (the review's probe cases).
        for type in [NSPasteboard.PasteboardType.html, .rtf, .rtfd, PasteImage.flatRTFDType] {
            let p = pasteboard()
            p.declareTypes([type, .png], owner: nil)
            p.setData(Data("x".utf8), forType: type)
            p.setData(Data([0x89, 0x50]), forType: .png)
            XCTAssertNil(PasteImage.read(p), "\(type.rawValue) + PNG is an ordinary paste")
            XCTAssertFalse(PasteImage.wouldHandle(p, preferences: isolatedPreferences()))
        }
        // Chrome's Copy Image: PNG plus HTML that is only the <img>.
        let chrome = pasteboard()
        chrome.declareTypes([.html, .png], owner: nil)
        chrome.setString("<meta charset='utf-8'><img src=\"https://example.com/a.png\"/>", forType: .html)
        chrome.setData(Data([0x89, 0x50]), forType: .png)
        guard case .data(_, .png) = PasteImage.read(chrome) else { return XCTFail("Chrome's Copy Image is an image paste") }
        XCTAssertTrue(PasteImage.wouldHandle(chrome, preferences: isolatedPreferences()))
        // HTML with visible text around the <img> stays text.
        let article = pasteboard()
        article.declareTypes([.html, .png], owner: nil)
        article.setString("<meta charset='utf-8'><p>See <img src=\"a.png\"> here</p>", forType: .html)
        article.setData(Data([0x89, 0x50]), forType: .png)
        XCTAssertNil(PasteImage.read(article), "HTML with text is an ordinary paste")
        XCTAssertTrue(PasteImage.isImageOnlyHTML("<html><body>\n <!--StartFragment--><IMG SRC='x.png'>&nbsp;</body></html>"))
        XCTAssertFalse(PasteImage.isImageOnlyHTML("<img src=a.png><img src=b.png>"), "two images")
        XCTAssertFalse(PasteImage.isImageOnlyHTML("<meta charset='utf-8'>"), "no image")

        // A web URL beside the image: the image only when the URL names an image file.
        let page = pasteboard()
        page.declareTypes([.URL, .png], owner: nil)
        page.setString("https://example.com/article", forType: .URL)
        page.setData(Data([0x89, 0x50]), forType: .png)
        XCTAssertNil(PasteImage.read(page), "a page URL is what was meant")
        let copied = pasteboard()
        copied.declareTypes([.URL, .png], owner: nil)
        copied.setString("https://example.com/img/plot.png?x=1", forType: .URL)
        copied.setData(Data([0x89, 0x50]), forType: .png)
        guard case .data(_, .png) = PasteImage.read(copied) else { return XCTFail("Copy Image with the image's own URL") }
        let both = pasteboard()
        both.declareTypes([.URL, .string, .png], owner: nil)
        both.setString("https://example.com/img/plot.png", forType: .URL)
        both.setString("https://example.com/img/plot.png", forType: .string)
        both.setData(Data([0x89, 0x50]), forType: .png)
        XCTAssertNil(PasteImage.read(both), "the URL is not the only text-like flavour")

        let dir = try projectDirectory()
        let png = dir.appendingPathComponent("shot.png")
        try Data([0x89]).write(to: png)
        let file = pasteboard()
        file.writeObjects([png as NSURL])
        XCTAssertEqual(PasteImage.read(file), .file(png), "a copied image file, even with its name as text")

        let tex = dir.appendingPathComponent("main.tex")
        try Data().write(to: tex)
        let texFile = pasteboard()
        texFile.writeObjects([tex as NSURL])
        XCTAssertNil(PasteImage.read(texFile), "a non-image file pastes as before")
    }

    @MainActor
    func testSavingConvertsTIFFAndNeverOverwrites() throws {
        let root = try projectDirectory()
        let date = Date(timeIntervalSince1970: 1_790_000_000)
        let first = try PasteImage.save(.data(Self.tiffData(), .tiff), projectRoot: root, folder: "figures", date: date)
        let second = try PasteImage.save(.data(Self.tiffData(), .tiff), projectRoot: root, folder: "figures", date: date)
        let base = P.baseName(for: date)
        XCTAssertEqual(first, "figures/\(base).png")
        XCTAssertEqual(second, "figures/\(base)-2.png")
        let bytes = try Data(contentsOf: root.appendingPathComponent(first))
        XCTAssertEqual(Array(bytes.prefix(4)), [0x89, 0x50, 0x4E, 0x47], "TIFF is written as PNG")
        let pdf = try PasteImage.save(.data(Data("%PDF-1.4".utf8), .pdf), projectRoot: root, folder: "", date: date)
        XCTAssertEqual(pdf, "\(base).pdf", "PDF stays PDF")
    }

    @MainActor
    func testAFolderThatResolvesOutsideTheProjectIsRefused() throws {
        let root = try projectDirectory()
        let outside = try projectDirectory()
        try FileManager.default.createSymbolicLink(at: root.appendingPathComponent("figures"), withDestinationURL: outside)
        XCTAssertThrowsError(try PasteImage.save(.data(Data("%PDF".utf8), .pdf), projectRoot: root, folder: "figures")) {
            XCTAssertEqual($0 as? PasteImage.SaveError, .outsideProject("figures"))
        }
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: outside.path), [])
    }

    @MainActor
    func testOversizedImagesAreRefused() {
        let big = PasteImage.Source.data(Data(count: PasteImage.maximumBytes + 1), .png)
        XCTAssertNotNil(PasteImage.refusal(for: big))
        XCTAssertNil(PasteImage.refusal(for: .data(Data(count: 10), .png)))
    }

    @MainActor
    func testAnImageFileInsideTheProjectIsReferencedInPlace() throws {
        let root = try projectDirectory()
        try FileManager.default.createDirectory(at: root.appendingPathComponent("img"), withIntermediateDirectories: true)
        let inside = root.appendingPathComponent("img/plot.pdf")
        try Data("%PDF".utf8).write(to: inside)
        XCTAssertEqual(try PasteImage.save(.file(inside), projectRoot: root, folder: "figures"), "img/plot.pdf")
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("figures").path))

        let outside = try projectDirectory().appendingPathComponent("My Plot.v2.png")
        try Data([0x89]).write(to: outside)
        XCTAssertEqual(try PasteImage.save(.file(outside), projectRoot: root, folder: "figures"), "figures/My-Plot-v2.png")
    }

    /// Runs a paste and, when it was taken, waits for the background save
    /// and the main-thread insertion. Returns (taken, inserted).
    @MainActor
    private func paste(_ pb: NSPasteboard, _ co: SourceEditorView.Coordinator, _ tv: NSTextView,
                       _ prefs: PasteImagePreferences) -> (taken: Bool, inserted: Bool) {
        let done = expectation(description: "paste finished")
        var inserted = false
        let taken = co.pasteImage(from: pb, in: tv, preferences: prefs) { inserted = $0; done.fulfill() }
        if taken { wait(for: [done], timeout: 10) }
        return (taken, inserted)
    }

    private func host(root: URL?, editingRoot: Bool = true, rootPath: String = "main.tex", rootText: String? = "",
                      notes: @escaping (String) -> Void = { _ in }) -> PasteImage.Host {
        PasteImage.Host(projectRoot: root, activePath: "main.tex", rootPath: rootPath, editingRoot: editingRoot, rootText: rootText,
                        ensureGraphicxInRoot: { nil }, note: notes)
    }

    /// The whole paste in a real text view: one undo step removes the figure
    /// and the graphicx line together; the saved file stays.
    @MainActor
    func testPasteInsertsAFigureAsOneUndoStep() throws {
        let root = try projectDirectory()
        let doc = "\\documentclass{article}\n\\begin{document}\n\n\\end{document}\n"
        var notes: [String] = []
        let (co, tv, window) = editor(doc, host: host(root: root, rootText: doc, notes: { notes.append($0) }))
        defer { window.close() }
        let caret = ("\\documentclass{article}\n\\begin{document}\n" as NSString).length
        tv.setSelectedRange(NSRange(location: caret, length: 0))
        let pb = pasteboard()
        pb.setData(Self.tiffData(), forType: .tiff)

        let r = paste(pb, co, tv, isolatedPreferences())
        XCTAssertTrue(r.taken)
        XCTAssertTrue(r.inserted)
        XCTAssertTrue(tv.string.contains("\\usepackage{graphicx}\n\\begin{document}\n\\begin{figure}[htbp]"))
        XCTAssertEqual((tv.string as NSString).substring(with: tv.selectedRange()), "Caption")
        let saved = try FileManager.default.contentsOfDirectory(atPath: root.appendingPathComponent("figures").path)
        XCTAssertEqual(saved.count, 1)
        XCTAssertTrue(notes.last?.contains("figures/") == true)

        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, doc, "one undo removes the figure and the graphicx line")
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: root.appendingPathComponent("figures").path).count, 1,
                       "undo keeps the saved image")
    }

    /// The buffer changed while the file was being written: the insertion
    /// uses the current selection, never the stale offset.
    @MainActor
    func testAnEditDuringTheSaveRevalidatesTheSelection() throws {
        let root = try projectDirectory()
        let (co, tv, window) = editor("abc\n", host: host(root: root, rootText: nil))
        defer { window.close() }
        tv.setSelectedRange(NSRange(location: 4, length: 0))
        let pb = pasteboard()
        pb.setData(Self.tiffData(), forType: .tiff)
        let done = expectation(description: "paste finished")
        XCTAssertTrue(co.pasteImage(from: pb, in: tv, preferences: isolatedPreferences()) { _ in done.fulfill() })
        tv.string = "zz\nabc\n" // typed meanwhile (no user edit path: nothing else listens here)
        tv.setSelectedRange(NSRange(location: 7, length: 0))
        wait(for: [done], timeout: 10)
        XCTAssertTrue(tv.string.hasPrefix("zz\nabc\n\\begin{figure}[htbp]"), tv.string)
    }

    @MainActor
    func testUnsavedDocumentWritesNothingAndSaysSo() throws {
        var notes: [String] = []
        let (co, tv, window) = editor("x", host: host(root: nil, notes: { notes.append($0) }))
        defer { window.close() }
        let pb = pasteboard()
        pb.setData(Self.tiffData(), forType: .tiff)
        XCTAssertTrue(co.pasteImage(from: pb, in: tv, preferences: isolatedPreferences()), "image data: taken, with a message")
        XCTAssertEqual(tv.string, "x")
        XCTAssertEqual(notes, [PasteImage.unsavedNote])

        // A copied file falls through to the ordinary paste (its name), with the same message.
        let dir = try projectDirectory()
        let png = dir.appendingPathComponent("shot.png")
        try Data([0x89]).write(to: png)
        let file = pasteboard()
        file.writeObjects([png as NSURL])
        XCTAssertFalse(co.pasteImage(from: file, in: tv, preferences: isolatedPreferences()))
        XCTAssertEqual(notes.count, 2)
        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: dir.path), ["shot.png"], "nothing written")
    }

    @MainActor
    func testThePreambleGetsTheOrdinaryPaste() throws {
        let root = try projectDirectory()
        var notes: [String] = []
        let doc = "\\documentclass{article}\n\n\\begin{document}\n\\end{document}\n"
        let (co, tv, window) = editor(doc, host: host(root: root, rootText: doc, notes: { notes.append($0) }))
        defer { window.close() }
        tv.setSelectedRange(NSRange(location: 24, length: 0))
        let pb = pasteboard()
        pb.setData(Self.tiffData(), forType: .tiff)
        XCTAssertFalse(co.pasteImage(from: pb, in: tv, preferences: isolatedPreferences()))
        XCTAssertEqual(tv.string, doc)
        XCTAssertEqual(notes, [PasteImage.preambleNote])
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("figures").path), "no file written")
    }

    @MainActor
    func testTextAndDisabledPastesAreLeftToAppKit() throws {
        let root = try projectDirectory()
        let (co, tv, window) = editor("x", host: host(root: root))
        defer { window.close() }
        let prefs = isolatedPreferences()
        let text = pasteboard()
        text.setString("plain", forType: .string)
        XCTAssertFalse(co.pasteImage(from: text, in: tv, preferences: prefs))

        let image = pasteboard()
        image.setData(Self.tiffData(), forType: .tiff)
        prefs.enabled = false
        XCTAssertFalse(co.pasteImage(from: image, in: tv, preferences: prefs))
        XCTAssertEqual(tv.string, "x")

        // The text view's own Paste reads `imagePasteboard` and, when the
        // image paste takes it, never falls through to AppKit's paste (which
        // would read the real clipboard).
        prefs.enabled = true
        let done = expectation(description: "paste finished")
        tv.imagePasteboard = { image }
        tv.imagePasteHandler = { pb in co.pasteImage(from: pb, in: tv, preferences: prefs) { _ in done.fulfill() } }
        tv.setSelectedRange(NSRange(location: 1, length: 0))
        tv.paste(nil)
        wait(for: [done], timeout: 10)
        XCTAssertTrue(tv.string.hasPrefix("x\n\\begin{figure}[htbp]"), tv.string)
    }

    /// Menu validation: Paste is enabled for an image-only pasteboard the
    /// image paste takes, and `wouldHandle` follows the setting and the text rule.
    @MainActor
    func testWouldHandleAndPasteValidation() throws {
        let prefs = isolatedPreferences()
        let image = pasteboard()
        image.setData(Self.tiffData(), forType: .tiff)
        let text = pasteboard()
        text.setString("t", forType: .string)
        text.setData(Self.tiffData(), forType: .tiff)
        XCTAssertTrue(PasteImage.wouldHandle(image, preferences: prefs))
        XCTAssertFalse(PasteImage.wouldHandle(text, preferences: prefs))
        prefs.enabled = false
        XCTAssertFalse(PasteImage.wouldHandle(image, preferences: prefs))

        let tv = CompletingTextView(frame: NSRect(x: 0, y: 0, width: 100, height: 100))
        tv.isRichText = false
        tv.imagePasteboard = { image }
        tv.imagePasteHandler = { _ in false }
        let item = NSMenuItem(title: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
        if PasteImagePreferences.shared.enabled {
            XCTAssertTrue(tv.validateUserInterfaceItem(item), "an image-only pasteboard enables Paste")
        }
        tv.isEditable = false
        XCTAssertFalse(tv.validateUserInterfaceItem(item), "a read-only buffer never pastes")
    }

    /// Adding graphicx to the open root while another document is active: a
    /// direct buffer write that schedules autosave, skipped with the helper attached.
    @MainActor
    func testEnsureGraphicxInTheOpenRootBuffer() throws {
        let saved = EditorPreferences.shared.autosave
        EditorPreferences.shared.autosave = true
        defer { EditorPreferences.shared.autosave = saved }
        let dir = try projectDirectory()
        let main = dir.appendingPathComponent("main.tex")
        let body = "\\documentclass{article}\n\\begin{document}\n\\input{chap}\n\\end{document}\n"
        try body.write(to: main, atomically: true, encoding: .utf8)
        let model = ShellModel()
        model.files.policy = .disabled(reason: "test: no helper binary")
        XCTAssertEqual(model.openTex(at: main), .opened)
        model.documents.append(.init(path: "chap.tex", text: "\\section{A}\n"))
        model.activePath = "chap.tex"

        XCTAssertEqual(model.ensureGraphicxInEntryBuffer(helperAttached: true), "Add \\usepackage{graphicx} to main.tex.")
        XCTAssertEqual(model.documents[0].text, body, "the helper's revisions are not bypassed")

        let note = try XCTUnwrap(model.ensureGraphicxInEntryBuffer(helperAttached: false))
        XCTAssertTrue(note.contains("does not remove it"), note)
        XCTAssertTrue(model.documents[0].text.contains("\\documentclass{article}\n\\usepackage{graphicx}\n"))
        model.flushPendingAutosave()
        XCTAssertTrue(try String(contentsOf: main, encoding: .utf8).contains("\\usepackage{graphicx}"), "autosave was scheduled")
        XCTAssertNil(model.ensureGraphicxInEntryBuffer(helperAttached: false), "already loaded")

        let host = model.imagePasteHost()
        XCTAssertFalse(host.editingRoot)
        XCTAssertEqual(host.rootPath, "main.tex")
        XCTAssertNotNil(host.rootText)
    }

    /// The root is not open: its \graphicspath and graphicx come from disk.
    @MainActor
    func testARootThatIsNotOpenIsReadFromDisk() throws {
        let root = try projectDirectory()
        try "\\documentclass{article}\n\\graphicspath{{img/}}\n\\begin{document}\n\\end{document}\n"
            .write(to: root.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        var notes: [String] = []
        let h = PasteImage.Host(projectRoot: root, activePath: "main.tex", rootPath: "main.tex", editingRoot: false, rootText: nil,
                                ensureGraphicxInRoot: { XCTFail("not open: never written"); return nil }, note: { notes.append($0) })
        let (co, tv, window) = editor("\\section{A}\n\n", host: h)
        defer { window.close() }
        tv.setSelectedRange(NSRange(location: 12, length: 0))
        let pb = pasteboard()
        pb.setData(Self.tiffData(), forType: .tiff)
        XCTAssertTrue(paste(pb, co, tv, isolatedPreferences()).inserted)
        XCTAssertTrue(tv.string.contains("{img/pasted-"), "the disk root's \\graphicspath decides the folder")
        XCTAssertTrue(notes.last?.contains("Add \\usepackage{graphicx} to main.tex.") == true, notes.last ?? "")
    }

    func testPreferencesPersistAndDefaultOn() throws {
        let suite = "paste-image-test-\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        MainActor.assumeIsolated {
            let a = PasteImagePreferences(defaults: defaults)
            XCTAssertTrue(a.enabled)
            XCTAssertEqual(a.folder, "figures")
            XCTAssertEqual(a.width, "0.8\\linewidth")
            XCTAssertTrue(a.wrapInFigure)
            a.enabled = false
            a.folder = "img"
            a.wrapInFigure = false
            let b = PasteImagePreferences(defaults: defaults)
            XCTAssertFalse(b.enabled)
            XCTAssertEqual(b.folder, "img")
            XCTAssertFalse(b.wrapInFigure)
        }
    }

    @MainActor
    private func editor(_ text: String, host: PasteImage.Host) -> (SourceEditorView.Coordinator, CompletingTextView, NSWindow) {
        let view = SourceEditorView(text: .constant(text), imagePasteHost: { host })
        let co = view.makeCoordinator()
        let tv = CompletingTextView(frame: NSRect(x: 0, y: 0, width: 400, height: 200))
        tv.isRichText = false
        tv.allowsUndo = true
        tv.string = text
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 400, height: 200), styleMask: [.titled], defer: true)
        window.isReleasedWhenClosed = false
        window.contentView = tv
        return (co, tv, window)
    }
}
