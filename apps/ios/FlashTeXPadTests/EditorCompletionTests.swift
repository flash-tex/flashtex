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
        // Byte-identical to the Mac's copy and the generated one (gen-supported-latex.py check --installed).
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

    /// A vocabulary with class-scoped rows, for the gate's rules: the bundled
    /// one is generated under article (RQ5) and has none today.
    private static let scoped = LaTeXVocabulary(commands: [
        .init(name: "frac", arguments: "{#1}{#2}", description: "LaTeX kernel", mode: .text, origin: "text_dispatch"),
        .init(name: "frametitle", arguments: "{#1}", description: "beamer", mode: .text, origin: "text_dispatch", requiresClass: "beamer"),
        .init(name: "email", arguments: "{#1}", description: "AMS top matter", mode: .text, origin: "text_dispatch", requiresClass: "amsart,amsbook,amsproc"),
        .init(name: "address", arguments: "{#1}", description: "AMS and letter", mode: .text, origin: "text_dispatch", requiresClass: "letter,amsart,amsbook,amsproc"),
    ], environments: [
        .init(name: "frame", description: "a boxed frame", mode: .text),
        .init(name: "block", description: "beamer block", mode: .text, requiresClass: "beamer"),
    ], compilerVersion: "test")

    func testShorterNamesRankFirstAndClassScopedCommandsAreGated() {
        let bundled = LocalCompletion.Context(vocabulary: PadModel.bundledVocabulary)
        XCTAssertEqual(LocalCompletion.suggestions(in: "\\frac", caretByte: 5, context: bundled).first?.text, "\\frac")
        // babel's `\frq` (a measured package command with no annotation) is
        // shorter, but held back while an annotated command matches.
        let bundledFr = LocalCompletion.suggestions(in: "\\fr", caretByte: 3, context: bundled).map(\.text)
        XCTAssertEqual(bundledFr.first, "\\frac")
        XCTAssertFalse(bundledFr.contains("\\frq"), "\(bundledFr)")
        XCTAssertEqual(LocalCompletion.suggestions(in: "\\frq", caretByte: 4, context: bundled).first?.text, "\\frq")
        let ctx = LocalCompletion.Context(vocabulary: Self.scoped)
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
    /// the whole string, so the commands vanished under their own classes.
    func testCommaListClassScopeOffersUnderEachListedClassOnly() {
        let ctx = LocalCompletion.Context(vocabulary: Self.scoped)
        func offers(_ text: String, _ command: String) -> Bool {
            LocalCompletion.suggestions(in: text, caretByte: text.utf8.count, context: ctx).contains { $0.text == command }
        }
        for cls in ["amsart", "amsbook", "amsproc"] {
            XCTAssertTrue(offers("\\documentclass{\(cls)}\n\\ema", "\\email"), cls)
            XCTAssertTrue(offers("\\documentclass{\(cls)}\n\\addr", "\\address"), cls)
        }
        XCTAssertTrue(offers("\\documentclass{letter}\n\\addr", "\\address"))
        XCTAssertFalse(offers("\\documentclass{article}\n\\ema", "\\email"))
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

/// iPad edits reach the file they were opened from (autosave), guarded by
/// the hash of the version the buffer was made from, and unsaved text
/// survives a crash as a draft offered on relaunch.
@MainActor
final class AutosaveTests: XCTestCase {
    var dir: URL!
    var file: URL!
    let base = "\\documentclass{article}\n"

    override func setUp() async throws {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("autosave-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        file = dir.appendingPathComponent("main.tex")
        try Data(base.utf8).write(to: file)
    }

    override func tearDown() async throws { try? FileManager.default.removeItem(at: dir) }

    private func onDisk() throws -> String { try String(contentsOf: file, encoding: .utf8) }

    /// Only existing API: open, type, wait. Before: the buffer never reached
    /// the file (the editor had no save at all).
    func testEditsReachTheFileWithoutAnyAction() async throws {
        let m = PadModel(link: MacLink(store: nil))
        m.open(url: file)
        XCTAssertNil(m.openError)
        m.textChanged(base + "Hello.\n", caret: (base as NSString).length + 7)
        let end = Date().addingTimeInterval(5)
        while (try? onDisk()) != base + "Hello.\n", Date() < end { try await Task.sleep(nanoseconds: 50_000_000) }
        XCTAssertEqual(try onDisk(), base + "Hello.\n")
        XCTAssertEqual(m.saveState, .saved)
    }

    private func model(drafts: PadDraftStore? = nil) -> PadModel {
        let m = PadModel(link: MacLink(store: nil), draftStore: drafts)
        m.autosaveDelay = 0.05
        m.draftDelay = 0.01
        return m
    }

    func testAnExternalChangeIsSurfacedAndNeverOverwritten() async throws {
        let m = model()
        m.open(url: file)
        try Data("changed on the Mac\n".utf8).write(to: file)
        m.textChanged(base + "mine\n")
        await m.settleSave()
        guard case .conflict(let disk?) = m.saveState else { return XCTFail("\(m.saveState)") }
        XCTAssertEqual(disk.text, "changed on the Mac\n")
        XCTAssertEqual(try onDisk(), "changed on the Mac\n", "the other version is not overwritten")
        m.textChanged(base + "mine, more\n")
        await m.settleSave()
        XCTAssertEqual(try onDisk(), "changed on the Mac\n", "later edits wait for the choice too")
        m.keepBufferOverDisk()
        await m.settleSave()
        XCTAssertEqual(try onDisk(), base + "mine, more\n")
        XCTAssertEqual(m.saveState, .saved)
    }

    func testUsingTheFilesVersionReplacesTheBuffer() async throws {
        let m = model()
        m.open(url: file)
        try Data("theirs\n".utf8).write(to: file)
        m.textChanged(base + "mine\n")
        await m.settleSave()
        await m.reloadFromDisk()
        XCTAssertEqual(m.document?.text, "theirs\n")
        XCTAssertEqual(m.saveState, .saved)
        m.textChanged("theirs\nthen mine\n")
        await m.settleSave()
        XCTAssertEqual(try onDisk(), "theirs\nthen mine\n", "saves against the reloaded version")
    }

    /// The app dies before its autosave: the unsaved text is on disk as a
    /// draft, offered on relaunch, and restoring it saves it to the file.
    func testUnsavedTextSurvivesACrashAndIsOfferedOnRelaunch() async throws {
        let drafts = PadDraftStore(directory: dir.appendingPathComponent("support/drafts"))
        let m = model(drafts: drafts)
        m.autosaveDelay = 3600
        m.open(url: file)
        m.textChanged(base + "not saved yet\n")
        await m.settleDraft()
        XCTAssertEqual(try onDisk(), base, "the autosave never ran")

        let again = model(drafts: drafts) // relaunch
        let offered = try XCTUnwrap(again.recoverableDrafts.first)
        XCTAssertEqual(offered.draft.text, base + "not saved yet\n")
        XCTAssertEqual(offered.draft.title, "main.tex")
        again.restoreDraft(offered)
        XCTAssertNil(again.openError)
        XCTAssertEqual(again.document?.text, base + "not saved yet\n")
        await again.settleSave()
        XCTAssertEqual(try onDisk(), base + "not saved yet\n")
        await drafts.flush()
        XCTAssertTrue(PadModel(link: MacLink(store: nil), draftStore: drafts).recoverableDrafts.isEmpty, "a restored and saved draft is removed")
    }

    /// A draft restored over a file that changed since it was written is a
    /// conflict, not a silent overwrite of the other version.
    func testARestoredDraftOverAChangedFileIsAConflict() async throws {
        let drafts = PadDraftStore(directory: dir.appendingPathComponent("support/drafts"))
        let m = model(drafts: drafts)
        m.autosaveDelay = 3600
        m.open(url: file)
        m.textChanged(base + "draft\n")
        await m.settleDraft()
        try Data("edited on the Mac meanwhile\n".utf8).write(to: file)
        let again = model(drafts: drafts)
        again.restoreDraft(try XCTUnwrap(again.recoverableDrafts.first))
        await again.settleSave()
        guard case .conflict(let disk?) = again.saveState else { return XCTFail("\(again.saveState)") }
        XCTAssertEqual(disk.text, "edited on the Mac meanwhile\n")
        XCTAssertEqual(try onDisk(), "edited on the Mac meanwhile\n")
        XCTAssertEqual(again.document?.text, base + "draft\n", "the draft text is in the editor to keep or drop")
    }

    // MARK: review of #2023 (daniel-parent)

    private func second(_ text: String = "B\n") throws -> URL {
        let url = dir.appendingPathComponent("other.tex")
        try Data(text.utf8).write(to: url)
        return url
    }

    /// Finding 1 (blocker): open A, open B without editing A, type. The
    /// editor kept showing A (every opened document is revision 1), so the
    /// keystroke put A's text into B's buffer and autosave wrote it over B.
    func testOpeningAnotherFileShowsItAndNeverWritesTheOldTextIntoIt() async throws {
        let b = try second()
        let m = model()
        let editor = EditorController()
        EditorView.connect(editor, to: m)
        m.open(url: file)
        EditorView.show(try XCTUnwrap(m.document), in: editor, model: m)
        XCTAssertEqual(editor.text, base)
        m.open(url: b)
        EditorView.show(try XCTUnwrap(m.document), in: editor, model: m)
        XCTAssertEqual(editor.text, "B\n", "the editor shows the file just opened")
        editor.select(NSRange(location: 2, length: 0))
        editor.type("x")
        await m.settleSave()
        XCTAssertEqual(try String(contentsOf: b, encoding: .utf8), "B\nx")
        XCTAssertEqual(try onDisk(), base, "A untouched")
    }

    /// Finding 2: edit A, open B within the draft delay, and A's save
    /// conflicts. The save result was dropped (another document was open)
    /// and A's draft task was cancelled: the edits were nowhere.
    func testSwitchingFilesKeepsEditsWhoseSaveConflicts() async throws {
        let drafts = PadDraftStore(directory: dir.appendingPathComponent("support/drafts"))
        let b = try second()
        let m = model(drafts: drafts)
        m.draftDelay = 3600
        m.open(url: file)
        m.textChanged(base + "A edits\n")
        try Data("A changed on the Mac\n".utf8).write(to: file)
        m.open(url: b) // well within the draft delay
        await m.settleSave()
        await drafts.flush()
        XCTAssertEqual(m.recoverableDrafts.map(\.draft.text), [base + "A edits\n"], "offered at once")
        XCTAssertEqual(try onDisk(), "A changed on the Mac\n", "the other version is not overwritten")
        let again = model(drafts: drafts) // and still there after a relaunch
        XCTAssertEqual(again.recoverableDrafts.map(\.draft.text), [base + "A edits\n"])
    }

    /// Finding 3: A's save is slow (a file provider holds the file), B is
    /// edited meanwhile; B's save waited for A's and was then dropped.
    func testASlowSaveOfTheOldFileDoesNotSwallowTheNewOne() async throws {
        let b = try second()
        let m = model()
        m.open(url: file)
        m.textChanged(base + "A\n")
        let hold = DispatchSemaphore(value: 0)
        let holding = DispatchSemaphore(value: 0)
        let url = file!
        DispatchQueue.global().async {
            var err: NSError?
            NSFileCoordinator(filePresenter: nil).coordinate(writingItemAt: url, options: [], error: &err) { _ in
                holding.signal()
                hold.wait()
            }
        }
        holding.wait()
        m.saveNow() // A's save now waits on the coordinator
        m.open(url: b)
        m.textChanged("B\nedited\n")
        try await Task.sleep(nanoseconds: 300_000_000) // B's autosave ran and found A's in flight
        hold.signal()
        await m.settleSave()
        let end = Date().addingTimeInterval(3)
        while (try? String(contentsOf: b, encoding: .utf8)) != "B\nedited\n", Date() < end { try await Task.sleep(nanoseconds: 50_000_000) }
        XCTAssertEqual(try String(contentsOf: b, encoding: .utf8), "B\nedited\n")
        XCTAssertEqual(m.saveState, .saved)
        XCTAssertEqual(try onDisk(), base + "A\n")
    }

    /// Finding 4: after a relaunch offers A's draft, editing and saving B
    /// replaced and then deleted the single draft file: A's text was gone.
    func testEditingAnotherFileKeepsTheOfferedDraft() async throws {
        let drafts = PadDraftStore(directory: dir.appendingPathComponent("support/drafts"))
        let b = try second()
        let first = model(drafts: drafts)
        first.autosaveDelay = 3600
        first.open(url: file)
        first.textChanged(base + "A unsaved\n")
        await first.settleDraft()
        let second = model(drafts: drafts) // relaunch: A's draft offered
        XCTAssertEqual(second.recoverableDrafts.map(\.draft.text), [base + "A unsaved\n"])
        second.open(url: b) // not restored: B is edited and saved instead
        second.textChanged("B\nedited\n")
        await second.settleDraft()
        await second.settleSave()
        await drafts.flush()
        XCTAssertEqual(try String(contentsOf: b, encoding: .utf8), "B\nedited\n")
        let third = model(drafts: drafts) // relaunch again
        XCTAssertEqual(third.recoverableDrafts.map(\.draft.text), [base + "A unsaved\n"], "A's draft survives until Restore or Discard")
    }

    /// Finding 5: "Use the file's version" loaded the snapshot taken when
    /// the conflict was found, though the file changed again since.
    func testUsingTheFilesVersionReadsTheFileAgain() async throws {
        let m = model()
        m.open(url: file)
        try Data("theirs\n".utf8).write(to: file)
        m.textChanged(base + "mine\n")
        await m.settleSave()
        guard case .conflict? = Optional(m.saveState) else { return XCTFail("\(m.saveState)") }
        try Data("theirs, later\n".utf8).write(to: file)
        await m.reloadFromDisk()
        XCTAssertEqual(m.document?.text, "theirs, later\n")
        m.textChanged("theirs, later\nmine\n")
        await m.settleSave()
        XCTAssertEqual(try onDisk(), "theirs, later\nmine\n", "saved against the version actually loaded")
        XCTAssertEqual(m.saveState, .saved)
    }

    func testDiscardingTheOfferedDraftRemovesIt() async throws {
        let drafts = PadDraftStore(directory: dir.appendingPathComponent("support/drafts"))
        drafts.save(PadUnsavedDraft(source: .bundledSample, title: "demo.tex", text: "x", diskSha256: nil))
        await drafts.flush()
        let m = model(drafts: drafts)
        let offered = try XCTUnwrap(m.recoverableDrafts.first)
        m.discardDraft(offered)
        await drafts.flush()
        XCTAssertTrue(m.recoverableDrafts.isEmpty)
        XCTAssertTrue(model(drafts: drafts).recoverableDrafts.isEmpty)
    }
}
