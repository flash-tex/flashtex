import FlashTeXEditorCore
import FlashTeXPadKit
import XCTest
@testable import FlashTeXPad

/// The completion pipeline: the keystroke path on the main actor is cheap
/// (no document scan), the list is computed off-main after a debounce, and
/// the suggestions carry the shared vocabulary's documentation and snippets
/// in the `\`, `\begin{`, `\ref{` and `\cite{` contexts.
@MainActor
final class EditorCompletionTests: XCTestCase {
    var model: PadModel!

    override func setUp() async throws {
        model = PadModel(link: MacLink(store: nil))
        model.completionDelay = 0.02
    }

    private func open(_ text: String, name: String = "doc.tex") throws {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("completion-\(UUID())-\(name)")
        try text.write(to: url, atomically: true, encoding: .utf8)
        model.open(url: url)
        XCTAssertNil(model.openError)
    }

    // MARK: keystroke budget

    func testTextChangedOn200KBDocumentReturnsUnderOneMillisecond() async throws {
        let text = EditorHighlightTests.largeDocument()
        try open(text)
        XCTAssertGreaterThan(model.document!.utf8Count, 200_000)
        var samples: [Double] = []
        var current = text
        for i in 0..<20 {
            current += i % 2 == 0 ? "x" : "\\alpha "
            let start = DispatchTime.now().uptimeNanoseconds
            model.textChanged(current, caret: (current as NSString).length, mathMode: false)
            samples.append(Double(DispatchTime.now().uptimeNanoseconds - start) / 1_000_000)
        }
        samples.sort()
        print("editor.textChanged.200KB: median \(samples[samples.count / 2]) ms, max \(samples.last!) ms")
        XCTAssertLessThan(samples[samples.count / 2], 1, "median textChanged cost (ms) on the main actor")
        XCTAssertLessThan(samples[samples.count * 9 / 10], 1, "p90 textChanged cost (ms)")
        XCTAssertEqual(model.document?.revision, 21)
        current += "\\alp"
        model.textChanged(current, caret: (current as NSString).length, mathMode: true)
        await model.settleCompletions()
        XCTAssertGreaterThan(model.completionPasses, 0)
        XCTAssertEqual(model.completions.first?.text, "\\alpha", "the pass after the last edit saw `\\alp` (\(model.completions.map(\.text)))")
    }

    func testCompletionsAreDebouncedIntoOnePass() async throws {
        try open("\\documentclass{article}\n")
        model.completionDelay = 0.05
        var text = model.document!.text
        for c in "\\secti" {
            text.append(c)
            model.textChanged(text, caret: (text as NSString).length)
        }
        XCTAssertEqual(model.completionPasses, 0, "nothing computed synchronously")
        await model.settleCompletions()
        XCTAssertEqual(model.completionPasses, 1, "six keystrokes, one pass")
        XCTAssertEqual(model.completions.first?.text, "\\section")
    }

    func testStalePassesAreDropped() async throws {
        try open("")
        model.textChanged("\\al", caret: 3)
        let first = model.completionTask
        model.textChanged("\\be", caret: 3)
        await first?.value
        await model.settleCompletions()
        XCTAssertEqual(model.completionPasses, 1)
        XCTAssertTrue(model.completions.allSatisfy { $0.text.hasPrefix("\\be") }, "\(model.completions.map(\.text))")
    }

    // MARK: contexts

    func testVocabularyIsBundledAndDecodes() {
        let v = PadModel.bundledVocabulary
        XCTAssertGreaterThan(v.commands.count, 200)
        XCTAssertGreaterThan(v.environments.count, 20)
        XCTAssertNotNil(v.command(named: "frac"))
        XCTAssertEqual(v.command(named: "frac")?.arguments, "{num}{den}")
        XCTAssertTrue(v.environments.contains { $0.name == "itemize" })
        // Byte-identical to the Mac's copy and the compiler's, as sync-supported-latex.sh keeps it.
        let url = Bundle.main.url(forResource: "supported-latex", withExtension: "json")!
        let data = try! Data(contentsOf: url)
        XCTAssertTrue(String(decoding: data.prefix(60), as: UTF8.self).contains("flashtex-supported-latex/1"))
    }

    func testCommandContextOffersVocabularyWithDocsAndSnippets() {
        let ctx = LocalCompletion.Context(vocabulary: PadModel.bundledVocabulary)
        let s = LocalCompletion.suggestions(in: "\\fr", caretByte: 3, context: ctx)
        let frac = s.first { $0.text == "\\frac" }
        XCTAssertNotNil(frac)
        XCTAssertEqual(frac?.snippet?.text, "\\frac{}{}")
        XCTAssertEqual(frac?.snippet?.caretUTF16, 6)
        XCTAssertEqual(frac?.snippet?.stops, [8, 9])
        XCTAssertFalse(frac!.detail.isEmpty, "one-line documentation from the inventory")
        XCTAssertEqual(frac?.replaceStart, 0)
        XCTAssertEqual(frac?.replaceEnd, 3)
    }

    func testDocumentCommandsRankFirstAndOpenEnvironmentsClose() {
        let ctx = LocalCompletion.Context(vocabulary: PadModel.bundledVocabulary)
        let text = "\\mycmd \\mycmd \\begin{itemize}\n\\m"
        let s = LocalCompletion.suggestions(in: text, caretByte: text.utf8.count, context: ctx)
        XCTAssertEqual(s.first?.text, "\\mycmd")
        XCTAssertEqual(s.first?.detail, "typed elsewhere in this document")
        let e = LocalCompletion.suggestions(in: "\\begin{itemize}\n\\e", caretByte: 18, context: ctx)
        XCTAssertEqual(e.first?.text, "\\end{itemize}")
        XCTAssertEqual(e.first?.kind, .environmentClose)
    }

    func testShorterNamesRankFirstAndClassScopedCommandsAreGated() {
        let ctx = LocalCompletion.Context(vocabulary: PadModel.bundledVocabulary)
        let fr = LocalCompletion.suggestions(in: "\\fr", caretByte: 3, context: ctx)
        XCTAssertEqual(fr.first?.text, "\\frac")
        XCTAssertTrue(fr.contains { $0.text == "\\frametitle" }, "no class declared: beamer's command is offered")
        let article = "\\documentclass[11pt]{article}\n\\fr"
        let gated = LocalCompletion.suggestions(in: article, caretByte: article.utf8.count, context: ctx)
        XCTAssertFalse(gated.contains { $0.text == "\\frametitle" }, "an article never has \\frametitle: \(gated.map(\.text)) \(gated.map(\.detail))")
        let beamer = "\\documentclass{beamer}\n\\fr"
        XCTAssertTrue(LocalCompletion.suggestions(in: beamer, caretByte: beamer.utf8.count, context: ctx).contains { $0.text == "\\frametitle" })
        // `frame` is universal (a boxed frame outside beamer); `block` is beamer's alone.
        let env = "\\documentclass{article}\n\\begin{bl"
        XCTAssertFalse(LocalCompletion.suggestions(in: env, caretByte: env.utf8.count, context: ctx).contains { $0.text == "block" })
        let slide = "\\documentclass{beamer}\n\\begin{bl"
        XCTAssertTrue(LocalCompletion.suggestions(in: slide, caretByte: slide.utf8.count, context: ctx).contains { $0.text == "block" })
    }

    /// #1068 review: the AMS top matter is scoped to "amsart,amsbook,amsproc"
    /// and `\address` to "letter,amsart,amsbook,amsproc"; the iPad compared
    /// the whole string, so the commands vanished under their own classes,
    /// and a command already used in the text bypassed the gate.
    func testCommaListClassScopeOffersUnderEachListedClassOnly() {
        let ctx = LocalCompletion.Context(vocabulary: PadModel.bundledVocabulary)
        func offers(_ text: String, _ command: String) -> Bool {
            LocalCompletion.suggestions(in: text, caretByte: text.utf8.count, context: ctx).contains { $0.text == command }
        }
        for cls in ["amsart", "amsbook", "amsproc"] {
            XCTAssertTrue(offers("\\documentclass{\(cls)}\n\\ema", "\\email"), cls)
            XCTAssertTrue(offers("\\documentclass{\(cls)}\n\\addr", "\\address"), cls)
        }
        XCTAssertTrue(offers("\\documentclass{letter}\n\\addr", "\\address"))
        XCTAssertFalse(offers("\\documentclass{article}\n\\ema", "\\email"))
        // Used earlier in the text does not reopen the gate.
        XCTAssertFalse(offers("\\documentclass{article}\n\\email{x}\n\\ema", "\\email"))
        XCTAssertTrue(offers("\\documentclass{amsart}\n\\email{x}\n\\ema", "\\email"))
    }

    func testClassListTokensAreTrimmed() {
        XCTAssertTrue(LaTeXVocabulary.classOffers("letter, amsart", documentClass: "amsart"))
        XCTAssertTrue(LaTeXVocabulary.classOffers(" letter ,amsart", documentClass: "letter"))
        XCTAssertFalse(LaTeXVocabulary.classOffers("letter, amsart", documentClass: "article"))
        XCTAssertTrue(LaTeXVocabulary.classOffers("letter", documentClass: nil))
        XCTAssertTrue(LaTeXVocabulary.classOffers(nil, documentClass: "article"))
    }

    func testMathModeFiltersTheVocabulary() {
        let text = "\\su"
        let math = LocalCompletion.suggestions(in: text, caretByte: 3, context: .init(vocabulary: PadModel.bundledVocabulary, mathMode: true))
        let prose = LocalCompletion.suggestions(in: text, caretByte: 3, context: .init(vocabulary: PadModel.bundledVocabulary, mathMode: false))
        XCTAssertTrue(math.contains { $0.text == "\\sum" })
        XCTAssertFalse(prose.contains { $0.text == "\\sum" }, "a math-only symbol is not offered in prose")
        XCTAssertTrue(prose.contains { $0.text == "\\subsection" })
        XCTAssertFalse(math.contains { $0.text == "\\subsection" })
    }

    func testBeginContextOffersSkeletons() {
        let ctx = LocalCompletion.Context(vocabulary: PadModel.bundledVocabulary)
        let text = "  \\begin{ite"
        let s = LocalCompletion.suggestions(in: text, caretByte: text.utf8.count, context: ctx)
        let itemize = s.first { $0.text == "itemize" }
        XCTAssertNotNil(itemize)
        XCTAssertEqual(itemize?.kind, .environment)
        XCTAssertEqual(itemize?.snippet?.text, "itemize}\n      \\item \n  \\end{itemize}", "the line's indent plus one unit")
        XCTAssertEqual(itemize?.replaceStart, 9)
        let fig = LocalCompletion.suggestions(in: "\\begin{fig", caretByte: 10, context: ctx).first { $0.text == "figure" }
        XCTAssertEqual(fig?.snippet?.stops.count, 3, "figure: caption, label, and the end (the caret is in \\includegraphics)")
        XCTAssertTrue(fig!.snippet!.text.contains("\\caption{}"))
    }

    func testEndRefAndCiteContexts() {
        let ctx = LocalCompletion.Context(vocabulary: PadModel.bundledVocabulary)
        let end = LocalCompletion.suggestions(in: "\\begin{align}\n\\end{", caretByte: 19, context: ctx)
        XCTAssertEqual(end.first?.text, "align")
        XCTAssertNil(end.first?.snippet)
        let refs = LocalCompletion.suggestions(in: "\\label{eq:one}\\label{fig:a} \\ref{e", caretByte: 34, context: ctx)
        XCTAssertEqual(refs.map(\.text), ["eq:one"])
        let cites = LocalCompletion.suggestions(in: "\\cite{knuth84, lamport} \\citep{k", caretByte: 32, context: ctx)
        XCTAssertEqual(cites.map(\.text), ["knuth84"])
    }

    // MARK: model wiring

    func testShowAndDismiss() async throws {
        try open("\\alp")
        model.caretMoved(4)
        await model.settleCompletions()
        XCTAssertFalse(model.completions.isEmpty)
        model.handle(.dismiss)
        XCTAssertTrue(model.completions.isEmpty)
        XCTAssertTrue(model.completionsDismissed)
        model.caretMoved(3)
        await model.settleCompletions()
        XCTAssertTrue(model.completions.isEmpty, "stays hidden until an edit or ⌃Space")
        model.handle(.showCompletions)
        await model.settleCompletions()
        XCTAssertFalse(model.completions.isEmpty)
        XCTAssertFalse(model.completionsDismissed)
    }

    func testTabAcceptsTheFirstSuggestionThroughTheEditor() async throws {
        try open("\\fr")
        let editor = EditorController()
        editor.load(text: model.document!.text, caret: 3, revision: model.document!.revision)
        editor.onChange = { [unowned self] text, caret, math in editor.loadedRevision = self.model.textChanged(text, caret: caret, mathMode: math) }
        editor.onCommand = { [unowned self] c in self.model.handle(c) }
        model.editor = editor
        model.caretMoved(3)
        await model.settleCompletions()
        XCTAssertEqual(model.completions.first?.text, "\\frac")
        XCTAssertTrue(model.handle(.acceptOrNextStop))
        XCTAssertEqual(editor.text, "\\frac{}{}")
        XCTAssertEqual(editor.selectedRange.location, 6)
        XCTAssertEqual(model.document?.text, "\\frac{}{}", "the model followed the editor")
        XCTAssertEqual(model.document?.revision, editor.loadedRevision, "no reload of the editor's own edit")
    }

    func testAcceptWithoutAnEditorEditsTheDocument() async throws {
        try open("\\begin{itemize}\n\\e")
        model.caretMoved(18)
        await model.settleCompletions()
        let s = try XCTUnwrap(model.completions.first)
        XCTAssertEqual(s.text, "\\end{itemize}")
        model.accept(s)
        XCTAssertEqual(model.document?.text, "\\begin{itemize}\n\\end{itemize}")
        XCTAssertEqual(model.caretUTF16, 29)
    }

    func testDiagnosticsToggle() {
        XCTAssertFalse(model.diagnosticsPanelVisible)
        model.handle(.toggleDiagnostics)
        XCTAssertTrue(model.diagnosticsPanelVisible)
    }
}
