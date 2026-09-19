import AppKit
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Package and class authoring in the editor (lane pkg-editor): `@` as a
/// letter in `.sty`/`.cls` buffers and inside `\makeatletter`, the kernel's
/// authoring vocabulary first in a package buffer, `\RequirePackage{` /
/// `\LoadClass{` completing like `\usepackage{` / `\documentclass{` with
/// the project's own files first, and the New File template for a
/// `.sty`/`.cls`.
@MainActor
final class PackageEditingTests: XCTestCase {
    typealias Kind = SyntaxHighlighter.Kind

    private func spans(_ s: String, language: SyntaxHighlighter.Language = .latex) -> [String] {
        let ns = s as NSString
        return SyntaxHighlighter.runs(of: ns, language: language).map { "\($0.kind):\(ns.substring(with: $0.range))" }
    }

    // MARK: `@` as a letter

    func testAtIsALetterInAPackageBufferAndInsideMakeatletterOnly() {
        // A document: `\@` is a control symbol, `tempdima` plain text.
        XCTAssertEqual(spans("\\@tempdima=1pt"), ["command:\\@"])
        // A package buffer: one control word, everywhere, `\makeatother` or not.
        XCTAssertEqual(spans("\\@tempdima=1pt", language: .package), ["command:\\@tempdima"])
        XCTAssertEqual(spans("\\makeatother\n\\@ifnextchar[{x}{y}", language: .package),
                       ["command:\\makeatother", "command:\\@ifnextchar", "bracket:[", "brace:{", "brace:}", "brace:{", "brace:}"])
        // A document between `\makeatletter` and `\makeatother`: the same,
        // and back to a control symbol after.
        let doc = "\\makeatletter\n\\@tempdima=1pt\n\\makeatother\n\\@tempdima"
        XCTAssertEqual(spans(doc), ["command:\\makeatletter", "command:\\@tempdima", "command:\\makeatother", "command:\\@"])
        // The flag is a line-start state like the mode: `atLetters` follows the lines.
        var h = SyntaxHighlighter()
        h.reset(doc as NSString)
        XCTAssertEqual(h.atLetters, [false, true, true, false])
        XCTAssertTrue(h.atLetter(at: 20, text: doc as NSString), "inside the block")
        XCTAssertFalse(h.atLetter(at: doc.utf16.count, text: doc as NSString), "after \\makeatother")
        // The defined name of `\def\@foo` is one token too.
        XCTAssertEqual(spans("\\def\\@foo{x}", language: .package), ["command:\\def", "definition:\\@foo", "brace:{", "brace:}"])
        // The intelligence layer sees the whole name: the hover token and ⌘-click target.
        let sty = "\\@ifpackageloaded{xcolor}{}{}" as NSString
        XCTAssertEqual(EditorIntelligence.token(in: sty, at: 3, highlighter: { var h = SyntaxHighlighter(language: .package); h.reset(sty); return h }()),
                       .command(name: "@ifpackageloaded", range: NSRange(location: 0, length: 17)))
    }

    /// The incremental invariant holds for the `@` flag: inserting
    /// `\makeatletter` above re-lexes the lines below until the flag and the
    /// mode converge, and `runs(in:)` equals a fresh full lex.
    func testMakeatletterEditRelexesTheLinesBelowUntilConvergence() {
        var text = "\\section{A}\n\\@tempdima=1pt\n\\makeatother\n\\@x\n" as NSString
        var h = SyntaxHighlighter()
        h.reset(text)
        XCTAssertEqual(h.atLetters, [false, false, false, false, false])
        let inserted = "\\makeatletter\n"
        text = text.replacingCharacters(in: NSRange(location: 0, length: 0), with: inserted) as NSString
        _ = h.edit(range: NSRange(location: 0, length: 0), replacementLength: (inserted as NSString).length, text: text)
        XCTAssertEqual(h.atLetters, [false, true, true, true, false, false])
        XCTAssertEqual(h.lastEditLinesLexed, 4, "the edited line, then the lines below until `\\makeatother` restores the stored state")
        XCTAssertEqual(h.runs(in: NSRange(location: 0, length: text.length), text: text), SyntaxHighlighter.runs(of: text))
        // And the other way: deleting it turns the flag back off below.
        text = text.replacingCharacters(in: NSRange(location: 0, length: (inserted as NSString).length), with: "") as NSString
        _ = h.edit(range: NSRange(location: 0, length: (inserted as NSString).length), replacementLength: 0, text: text)
        XCTAssertEqual(h.atLetters, [false, false, false, false, false])
        XCTAssertEqual(h.runs(in: NSRange(location: 0, length: text.length), text: text), SyntaxHighlighter.runs(of: text))
    }

    func testPackagePathsAreLexedAsPackages() {
        let m = ShellModel()
        m.documents = [.init(path: "main.tex", text: ""), .init(path: "mystyle.sty", text: ""), .init(path: "thesis.cls", text: ""),
                       .init(path: "texinputs/0/x.def", text: ""), .init(path: "refs.bib", text: "")]
        for path in ["mystyle.sty", "thesis.cls", "texinputs/0/x.def"] {
            m.activePath = path
            XCTAssertEqual(m.editorLanguage, .package, path)
        }
        m.activePath = "main.tex"
        XCTAssertEqual(m.editorLanguage, .latex)
        m.activePath = "refs.bib"
        XCTAssertEqual(m.editorLanguage, .bibtex)
        XCTAssertTrue(ProjectManifest.isPackagePath("packages/siunitx/siunitx.sty"))
        XCTAssertFalse(ProjectManifest.isPackagePath("style.tex") || ProjectManifest.isPackagePath("sty"))
        XCTAssertEqual(ProjectManifest.packageDisplayName("packages/siunitx/siunitx.sty"), "siunitx.sty")
    }

    // MARK: the kernel vocabulary

    private func offered(_ text: String, packageMode: Bool = false, atLetter: Bool = false, files: [String] = []) -> [Completion.Suggestion] {
        Completion.suggestions(in: text, caretUTF16: (text as NSString).length, metadata: nil,
                               packageMode: packageMode, atLetter: atLetter, projectPackageFiles: files)
    }

    func testKernelCommandsLeadTheListInAPackageBufferOnly() {
        // In a package: the ltclass vocabulary first, in its table order,
        // with the authoring documentation and an argument snippet.
        let pro = offered("\\Pro", packageMode: true)
        XCTAssertEqual(pro.prefix(4).map(\.insertText), ["\\ProvidesPackage", "\\ProvidesClass", "\\ProvidesFile", "\\ProcessOptions"], "\(pro.map(\.label))")
        XCTAssertEqual(pro.first?.label, "\\ProvidesPackage{name}[date]")
        XCTAssertEqual(pro.first?.detail, "package authoring (LaTeX kernel)")
        XCTAssertEqual(pro.first?.snippet, Completion.Snippet(text: "\\ProvidesPackage{}", caretUTF16: 17, stops: [18]))
        XCTAssertNil(pro.first { $0.insertText == "\\ProcessOptions" }?.snippet, "no braced argument, no snippet")
        XCTAssertEqual(CompletionPopup.documentation(for: pro[0]), "\\ProvidesPackage{name}[yyyy/mm/dd vX.Y info]: names the package (must match the file) and its version line.")
        // The exact spelling still leads: `\def` above `\DeclareOption`.
        XCTAssertEqual(offered("\\def", packageMode: true).first?.insertText, "\\def")
        // A name the compiler's vocabulary also has is offered once, as the kernel row.
        let req = offered("\\Require", packageMode: true)
        XCTAssertEqual(req.filter { $0.insertText == "\\RequirePackage" }.count, 1)
        XCTAssertEqual(req.first?.detail, "package authoring (LaTeX kernel)")
        // In a document the same prefix offers the compiler's rows; no kernel row on top.
        XCTAssertFalse(offered("\\Pro").contains { $0.detail == "package authoring (LaTeX kernel)" })
        XCTAssertNotEqual(offered("\\ne").first?.detail, "package authoring (LaTeX kernel)")
    }

    func testAtCommandsCompleteWhenAtIsALetter() {
        // `\@if` is one token only with the flag (the highlighter's answer for the caret).
        XCTAssertEqual(Completion.token(in: "\\@if", caretUTF16: 4, atLetter: true), .command(name: "@if", start: 0, end: 4))
        XCTAssertEqual(Completion.token(in: "\\@if", caretUTF16: 4), .word(text: "if", start: 2, end: 4, context: .none),
                       "in a document `\\@` is a control symbol and `if` a word after it")
        XCTAssertEqual(Completion.completionRange(in: "x \\@if", caretUTF16: 6, atLetter: true), NSRange(location: 2, length: 4))
        let rows = offered("\\@if", packageMode: true, atLetter: true)
        XCTAssertEqual(rows.prefix(3).map(\.insertText), ["\\@ifpackageloaded", "\\@ifclassloaded", "\\@ifpackagewith"], "\(rows.map(\.label))")
        XCTAssertEqual(rows.first?.snippet?.text, "\\@ifpackageloaded{}{}{}")
        XCTAssertTrue(rows.contains { $0.insertText == "\\@ifnextchar" })
        // The automatic-open gate reads the same flag.
        XCTAssertEqual(Completion.caretToken(in: "\\@if" as NSString, caretUTF16: 4, atLetter: true)?.token, .command(name: "@if", start: 0, end: 4))
        // Hover: the category and the documentation line.
        XCTAssertEqual(EditorIntelligence.CommandDocs.category(for: "@ifnextchar"), "Package-authoring command")
        XCTAssertEqual(EditorIntelligence.CommandDocs.documentation(for: "DeclareOption"),
                       "\\DeclareOption{option}{code}: what \\usepackage[option] runs; \\DeclareOption*{code} handles every other option (\\CurrentOption).")
        XCTAssertNil(EditorIntelligence.CommandDocs.table["DeclareOption"], "the kernel table stays out of the inventory drift gate")
    }

    // MARK: `\RequirePackage{` and `\LoadClass{`

    func testRequirePackageAndLoadClassCompleteLikeUsepackageAndDocumentclass() {
        let files = ["mystyle.sty", "texinputs/0/lab.sty", "thesis.cls", "packages/siunitx/siunitx.sty"]
        // The project's own packages first, named by their file, then CTAN's.
        let req = offered("\\RequirePackage{", files: files)
        XCTAssertEqual(req.prefix(3).map(\.label), ["mystyle", "lab", "siunitx"])
        XCTAssertEqual(req[0].detail, "package in this project · mystyle.sty")
        XCTAssertEqual(req[1].detail, "package in this project · lab.sty")
        XCTAssertEqual(req[3].detail, "package (recognised, not implemented by this compiler)")
        XCTAssertEqual(offered("\\usepackage{my", files: files).first?.label, "mystyle")
        XCTAssertEqual(offered("\\PassOptionsToPackage{draft}{my", files: files).first?.label, "mystyle")
        XCTAssertFalse(req.contains { $0.label == "thesis" }, "a class is not a package")
        // `\usepackage{siunitx` with siunitx resolved: one row, the project's.
        XCTAssertEqual(offered("\\usepackage{siunitx", files: files).filter { $0.label == "siunitx" }.count, 1)
        // Classes: the project's `.cls` first, then the standard ones.
        let cls = offered("\\LoadClass{", files: files)
        XCTAssertEqual(cls.prefix(2).map(\.label), ["thesis", "article"])
        XCTAssertEqual(cls[0].detail, "class in this project · thesis.cls")
        XCTAssertEqual(cls[1].detail, "document class")
        XCTAssertEqual(offered("\\documentclass[11pt]{bea").first?.label, "beamer")
        XCTAssertEqual(offered("\\LoadClassWithOptions{rep").first?.label, "report")
        // Typing either opens the list on its own, like `\usepackage{`.
        XCTAssertTrue(Completion.opensAutomatically(Completion.token(in: "\\LoadClass{", caretUTF16: 11)))
        XCTAssertTrue(Completion.opensAutomatically(Completion.token(in: "\\RequirePackage{", caretUTF16: 16)))
    }

    // MARK: macros declared in the project's packages

    static let mystyle = """
    \\NeedsTeXFormat{LaTeX2e}
    \\ProvidesPackage{mystyle}[2026/01/01 v1.0]
    \\RequirePackage{xcolor}
    \\newcommand{\\emphx}[1]{\\textcolor{red}{#1}}
    \\newcommand{\\note}[2][red]{\\textcolor{#1}{#2}}
    \\DeclareRobustCommand{\\brand}{FlashTeX}
    \\def\\pair#1#2{(#1, #2)}
    \\NewDocumentCommand{\\boxed}{o m}{#2}
    \\newif\\ifdraft
    \\newtheorem{lemma}{Lemma}
    \\newenvironment{aside}{\\begin{quote}}{\\end{quote}}
    \\endinput

    """

    func testDeclarationsCarryTheArgumentShape() {
        let d = Completion.declarations(in: Self.mystyle)
        func shape(_ name: String) -> (Int, Bool, String)? { d.first { $0.name == name }.map { ($0.mandatory, $0.optional, $0.definer) } }
        XCTAssertEqual(shape("emphx")?.0, 1); XCTAssertEqual(shape("emphx")?.1, false)
        XCTAssertEqual(shape("note")?.0, 1); XCTAssertEqual(shape("note")?.1, true, "[2][red]: one optional, one mandatory")
        XCTAssertEqual(shape("brand")?.0, 0); XCTAssertEqual(shape("brand")?.2, "DeclareRobustCommand")
        XCTAssertEqual(shape("pair")?.0, 2, "\\def\\pair#1#2")
        XCTAssertEqual(shape("boxed")?.0, 1); XCTAssertEqual(shape("boxed")?.1, true, "xparse `o m`")
        XCTAssertEqual(d.filter { $0.definer == "newif" }.map(\.name), ["ifdraft", "drafttrue", "draftfalse"])
        XCTAssertEqual(d.filter { $0.kind == .environment }.map(\.name), ["lemma", "aside"])
        XCTAssertEqual(d.first { $0.name == "emphx" }?.snippet, Completion.Snippet(text: "\\emphx{}", caretUTF16: 7, stops: [8]))
        XCTAssertEqual(d.first { $0.name == "pair" }?.snippet?.text, "\\pair{}{}")
        XCTAssertNil(d.first { $0.name == "brand" }?.snippet, "no arguments, no snippet")
        // `declaredCommands` is the same scan's command names, unchanged in order.
        XCTAssertEqual(Completion.declaredCommands(in: Self.mystyle), ["emphx", "note", "brand", "pair", "boxed", "ifdraft", "drafttrue", "draftfalse"])
        // The buffer's own declarations get the shape too.
        let ownText = "\\newcommand{\\emphx}[1]{x}\n\\emp"
        let own = Completion.suggestions(in: ownText, caretUTF16: (ownText as NSString).length, metadata: nil)
        XCTAssertEqual(own.first?.label, "\\emphx"); XCTAssertEqual(own.first?.snippet?.text, "\\emphx{}")
    }

    func testMacrosDeclaredInAPackageInputAreOfferedWithTheFileAndTheSnippet() {
        let packages = [Completion.SourceDocument(path: "texinputs/0/mystyle.sty", text: Self.mystyle)]
        let declared = Completion.packageDeclarations(in: packages)
        XCTAssertEqual(declared.first?.file, "mystyle.sty")
        func offered(_ typed: String) -> [Completion.Suggestion] {
            let text = "\\documentclass{article}\\usepackage{mystyle}\n" + typed
            return Completion.suggestions(in: text, caretUTF16: (text as NSString).length, metadata: nil,
                                          declaredElsewhere: ["emphz"], packageDeclarations: declared)
        }
        let e = offered("\\emp")
        XCTAssertEqual(e.prefix(2).map(\.label), ["\\emphz", "\\emphx"], "an open document's macro, then the package's: \(e.map(\.label))")
        XCTAssertEqual(e[1].detail, "declared in mystyle.sty")
        XCTAssertEqual(e[1].snippet, Completion.Snippet(text: "\\emphx{}", caretUTF16: 7, stops: [8]))
        XCTAssertEqual(e[1].kind, .command)
        XCTAssertTrue(e.contains { $0.insertText == "\\emph" }, "the compiler's \\emph stays offered after them: \(e.map(\.insertText))")
        XCTAssertEqual(offered("\\pai").first?.snippet?.text, "\\pair{}{}")
        XCTAssertEqual(offered("\\ifdr").first?.label, "\\ifdraft")
        XCTAssertEqual(offered("\\draftt").first?.detail, "declared in mystyle.sty")
        // A macro the buffer itself declares keeps its own row (and label), whatever the package says.
        let text = "\\newcommand{\\emphx}{y}\n\\emp"
        let here = Completion.suggestions(in: text, caretUTF16: (text as NSString).length, metadata: nil, packageDeclarations: declared)
        XCTAssertEqual(here.first?.detail, "declared in this document")
        XCTAssertEqual(here.filter { $0.label == "\\emphx" }.count, 1)
        // Environments: `\begin{lem` finds the package's `\newtheorem`.
        let env = offered("\\begin{lem")
        XCTAssertEqual(env.first?.label, "lemma")
        XCTAssertEqual(env.first?.detail, "declared in mystyle.sty")
        XCTAssertEqual(offered("\\begin{asi").first?.label, "aside")
        // The scheduler scans the package documents for a command and an environment name only.
        let exec = CompletionTests.ManualExecutor()
        let scheduler = CompletionScheduler(executor: exec.run)
        var delivered: [CompletionScheduler.Outcome] = []
        var req = CompletionScheduler.Request(text: "\\begin{document}\n\\emp", caretUTF16: 21, metadata: nil)
        req.packageDocuments = packages
        scheduler.schedule(req) { delivered.append($0) }
        exec.runAll()
        let deadline = Date().addingTimeInterval(2)
        while delivered.isEmpty, Date() < deadline { RunLoop.main.run(until: Date().addingTimeInterval(0.01)) }
        XCTAssertEqual(delivered.first?.items.first?.label, "\\emphx")
        XCTAssertEqual(delivered.first?.items.first?.detail, "declared in mystyle.sty")
    }

    func testGoToDefinitionOpensThePackageFileAtItsDefinition() async throws {
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("pkg-editor-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tmp) }
        let entry = tmp.appendingPathComponent("main.tex")
        try "\\documentclass{article}\n\\usepackage{mystyle}\n\\begin{document}\n\\emphx{a} \\begin{lemma}x\\end{lemma}\n\\end{document}\n"
            .write(to: entry, atomically: true, encoding: .utf8)
        try Self.mystyle.write(to: tmp.appendingPathComponent("mystyle.sty"), atomically: true, encoding: .utf8)
        let m = ShellModel()
        m.detachWorker()
        // The manifest read the helper would do: mystyle.sty next to the entry
        // (rooted), and a second package on a texinputs mount outside the root (virtual).
        m.manifest.reader = { root, _ in
            let json = """
            {"path":null,"exists":false,"manifest_dir":null,
             "manifest":{"project":{"entry":"main.tex","texinputs":[],"output":null},"fonts":{"text":null,"math":null,"mono":null,"sans":null},
                         "packages":{"source":"ctan","fetch":"ask","pin":{},"path":{}},"library":null},
             "warnings":[],"texinputs":[],"diagnostics":[],"template":"",
             "files":[{"path":"mystyle.sty","kind":"package","texinput":null,"origin":null,"text":\(Self.json(Self.mystyle)),"sha256":"a","bytes":1},
                      {"path":"texinputs/0/lab.sty","kind":"package","texinput":0,"origin":"/shared/tex/lab.sty","text":"\\\\ProvidesPackage{lab}\\n\\\\newcommand{\\\\labnote}[1]{#1}\\n","sha256":"b","bytes":1}]}
            """
            return .success(try! JSONDecoder().decode(ProjectFilesV1.Manifest.self, from: Data(json.utf8)))
        }
        XCTAssertEqual(m.openTex(at: entry), .opened)
        XCTAssertEqual(m.packageInputs.map(\.path), ["mystyle.sty", "texinputs/0/lab.sty"])
        XCTAssertNil(m.packageInputs[0].virtualSource)
        XCTAssertEqual(m.packageInputs[1].virtualSource, "/shared/tex/lab.sty (texinputs[0] of flashtex.toml)")
        XCTAssertEqual(m.packageDocumentsForEditor().map(\.path), ["mystyle.sty", "texinputs/0/lab.sty"])
        // Hover peek before anything is open: the package's definition, named by its file.
        XCTAssertEqual(m.definitionSummary(forCommand: "emphx"), "\\newcommand{\\emphx}{\\textcolor{red}{#1}} (line 4 in mystyle.sty)")
        XCTAssertEqual(m.definitionSummary(forCommand: "labnote"), "\\newcommand{\\labnote}{#1} (line 2 in texinputs/0/lab.sty)")
        XCTAssertNil(m.definitionSummary(forCommand: "section"))
        // ⌘-click on \emphx: the rooted .sty opens as an ordinary member, the definition selected.
        m.caretUTF16 = (m.activeText as NSString).range(of: "\\emphx{a}").location + 2
        m.goToDefinition()
        try await settle { m.activePath == "mystyle.sty" }
        XCTAssertEqual(m.documents.map(\.path), ["main.tex", "mystyle.sty"])
        XCTAssertNil(m.project.readOnlyNote(for: "mystyle.sty"), "a file under the root is an editable member")
        XCTAssertEqual((m.activeText as NSString).substring(with: try XCTUnwrap(m.selection?.nsRange)), "\\newcommand{\\emphx}[1]{\\textcolor{red}{#1}}")
        XCTAssertEqual(m.navigationNote, "Definition: \\newcommand{\\emphx}{\\textcolor{red}{#1}} at line 4 in mystyle.sty.")
        XCTAssertEqual(m.editorLanguage, .package)
        // Now open, it is found the ordinary way (and no longer a package input).
        XCTAssertEqual(m.packageInputs.map(\.path), ["texinputs/0/lab.sty"])
        XCTAssertEqual(m.definitionSummary(forCommand: "emphx"), "\\newcommand{\\emphx}{\\textcolor{red}{#1}} (line 4)")
        // An environment a package declares: \begin{lemma} goes to its \newtheorem.
        m.project.switchDocument(to: "main.tex")
        m.caretUTF16 = (m.activeText as NSString).range(of: "lemma}x").location + 1
        m.goToDefinition()
        XCTAssertEqual(m.activePath, "mystyle.sty")
        XCTAssertEqual((m.activeText as NSString).substring(with: try XCTUnwrap(m.selection?.nsRange)), "\\newtheorem{lemma}{Lemma}")
        // A macro from the virtual mount: opened read-only with the banner, never saved.
        m.project.switchDocument(to: "main.tex")
        m.goToDefinition(ofCommand: "labnote")
        try await settle { m.activePath == "texinputs/0/lab.sty" }
        XCTAssertEqual(m.project.readOnlyNote(for: "texinputs/0/lab.sty"),
                       "lab.sty comes from /shared/tex/lab.sty (texinputs[0] of flashtex.toml) — shown read-only; the compiler reads it from there")
        XCTAssertEqual((m.activeText as NSString).substring(with: try XCTUnwrap(m.selection?.nsRange)), "\\newcommand{\\labnote}[1]{#1}")
        XCTAssertTrue(m.navigationNote?.hasSuffix("in texinputs/0/lab.sty (read-only, from /shared/tex/lab.sty (texinputs[0] of flashtex.toml)).") == true, m.navigationNote ?? "")
        XCTAssertEqual(m.project.changeRefusal(for: "texinputs/0/lab.sty"), m.project.readOnlyNote(for: "texinputs/0/lab.sty"))
        if case .failed(let why) = await m.project.saveDocument("texinputs/0/lab.sty") { XCTAssertTrue(why.contains("read-only")) } else { XCTFail("a virtual member is never saved") }
        XCTAssertFalse(FileManager.default.fileExists(atPath: tmp.appendingPathComponent("texinputs/0/lab.sty").path))
        XCTAssertFalse(m.project.isDirty("texinputs/0/lab.sty"))
        XCTAssertEqual(m.project.listing.last?.origin, .virtual(source: "/shared/tex/lab.sty (texinputs[0] of flashtex.toml)"))
        // The compile request still carries it once (the open member, not the implicit input).
        XCTAssertEqual(m.project.implicitClosureDocuments().map(\.path), [])
        // A standard command: explained, nothing opened.
        m.goToDefinition(ofCommand: "section")
        XCTAssertEqual(m.navigationNote, "\\section has no \\newcommand/\\def/\\DeclareMathOperator definition in the open documents or the project's packages (a standard command).")
    }

    // MARK: diagnostics inside a package

    /// `main.tex` loads `mystyle.sty`; the compiler reports two problems at
    /// package lines, each with the "loaded here" label at the `\usepackage`.
    private static let mainLoading = "\\documentclass{article}\n\\usepackage{mystyle}\n\\begin{document}\nx\n\\end{document}\n"
    private static let styWithProblems = "\\ProvidesPackage{mystyle}\n\\RequirePackage{nosuch}\n\\foo\n"
    private static func packageResult() -> RuntimeV1.CompileResult {
        let load = RuntimeV1.SourceRange(path: "main.tex", startByte: 24, endByte: 44) // `\usepackage{mystyle}`
        let d1 = RuntimeV1.Diagnostic(severity: .error, message: "\\foo is not supported by this compiler version",
                                      source: .init(path: "mystyle.sty", startByte: 50, endByte: 54), recovery: nil, code: "unknown_command",
                                      labels: [.init(source: .init(path: "mystyle.sty", startByte: 50, endByte: 54), text: "", primary: true),
                                               .init(source: load, text: "mystyle.sty is loaded here", primary: false)])
        let d2 = RuntimeV1.Diagnostic(severity: .warning, message: "packages nosuch are recognised but not implemented",
                                      source: .init(path: "mystyle.sty", startByte: 26, endByte: 49), recovery: nil, code: "unsupported_feature",
                                      labels: [.init(source: load, text: "mystyle.sty is loaded here", primary: false)],
                                      notes: ["no project file found: looked for nosuch.sty, nosuch.cls"])
        return RuntimeV1.CompileResult(projectId: "p", revision: 1, status: .recovered, pages: [], diagnostics: [d1, d2], pdfPath: nil)
    }

    func testAProblemInsideAPackageMarksTheUsepackageThatLoadedIt() {
        let result = Self.packageResult()
        // The package buffer draws the two primary marks, nothing else.
        let sty = EditorDiagnostics.report(for: result, resultID: "r", path: "mystyle.sty", compiledText: Self.styWithProblems, currentText: Self.styWithProblems)
        XCTAssertEqual(sty.marks.map(\.message), ["\\foo is not supported by this compiler version", "packages nosuch are recognised but not implemented"])
        XCTAssertEqual(sty.marks.map(\.nsRange), [NSRange(location: 50, length: 4), NSRange(location: 26, length: 23)])
        // The document buffer draws one secondary mark at the `\usepackage`, the worst severity, counting both.
        let main = EditorDiagnostics.report(for: result, resultID: "r", path: "main.tex", compiledText: Self.mainLoading, currentText: Self.mainLoading)
        XCTAssertEqual(main.marks.count, 1)
        let hint = try! XCTUnwrap(main.marks.first)
        XCTAssertEqual(hint.message, "mystyle.sty: 2 problems — loaded here")
        XCTAssertEqual(hint.severity, .error)
        XCTAssertEqual((Self.mainLoading as NSString).substring(with: hint.nsRange), "\\usepackage{mystyle}")
        XCTAssertFalse(hint.hasFix)
        XCTAssertEqual(hint.identity.source.path, "main.tex", "its own identity: never the package buffer's mark")
        XCTAssertNotEqual(hint.id, sty.marks[0].id)
        XCTAssertFalse(EditorDiagnostics.isGap(hint.message), "a red or orange dot in the gutter, not a grey tick")
        // Rebased like a primary mark: an edit above shifts it, an edit inside drops it.
        let shifted = "% note\n" + Self.mainLoading
        let after = EditorDiagnostics.report(for: result, resultID: "r", path: "main.tex", compiledText: Self.mainLoading, currentText: shifted)
        XCTAssertEqual((shifted as NSString).substring(with: try! XCTUnwrap(after.marks.first).nsRange), "\\usepackage{mystyle}")
        let edited = Self.mainLoading.replacingOccurrences(of: "{mystyle}", with: "{mystylez}")
        XCTAssertTrue(EditorDiagnostics.report(for: result, resultID: "r", path: "main.tex", compiledText: Self.mainLoading, currentText: edited).marks.isEmpty)
        // A label pointing at another document, or a non-package source, marks nothing here.
        let other = RuntimeV1.Diagnostic(severity: .error, message: "x", source: .init(path: "ch.tex", startByte: 0, endByte: 1), recovery: nil,
                                         labels: [.init(source: .init(path: "main.tex", startByte: 24, endByte: 44), text: "included here", primary: false)])
        let plain = RuntimeV1.CompileResult(projectId: "p", revision: 1, status: .ok, pages: [], diagnostics: [other], pdfPath: nil)
        XCTAssertTrue(EditorDiagnostics.report(for: plain, resultID: "r", path: "main.tex", compiledText: nil, currentText: Self.mainLoading).marks.isEmpty)
    }

    private func loadedProject(_ tmp: URL) throws -> ShellModel {
        let entry = tmp.appendingPathComponent("main.tex")
        try Self.mainLoading.write(to: entry, atomically: true, encoding: .utf8)
        try Self.styWithProblems.write(to: tmp.appendingPathComponent("mystyle.sty"), atomically: true, encoding: .utf8)
        let m = ShellModel()
        m.detachWorker()
        m.files.policy = .disabled(reason: "test: hermetic (direct writes)")
        m.manifest.reader = { _, _ in
            let json = """
            {"path":null,"exists":false,"manifest_dir":null,
             "manifest":{"project":{"entry":"main.tex","texinputs":[],"output":null},"fonts":{"text":null,"math":null,"mono":null,"sans":null},
                         "packages":{"source":"ctan","fetch":"ask","pin":{},"path":{}},"library":null},
             "warnings":[],"texinputs":[],"diagnostics":[],"template":"",
             "files":[{"path":"mystyle.sty","kind":"package","texinput":null,"origin":null,"text":\(Self.json(Self.styWithProblems)),"sha256":"a","bytes":1}]}
            """
            return .success(try! JSONDecoder().decode(ProjectFilesV1.Manifest.self, from: Data(json.utf8)))
        }
        XCTAssertEqual(m.openTex(at: entry), .opened)
        return m
    }

    func testAProblemsRowForAPackagePathOpensThatFileAtTheSpan() async throws {
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("pkg-editor-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tmp) }
        let m = try loadedProject(tmp)
        m.result = Self.packageResult()
        m.setCompiledDocuments(["main.tex": Self.mainLoading, "mystyle.sty": Self.styWithProblems])
        // The document's gutter carries the hint; the package is not open yet.
        XCTAssertEqual(m.editorMarks.map(\.message), ["mystyle.sty: 2 problems — loaded here"])
        XCTAssertEqual(m.documents.map(\.path), ["main.tex"])
        // The row: "mystyle.sty line 3" — the compiled text is known — and Go to source opens it there.
        let groups = EditorDiagnostics.groups(of: m.displayedDiagnostics, documentOrder: m.documents.map(\.path))
        XCTAssertEqual(EditorDiagnostics.occurrenceLabel(0, of: groups[0], in: m.displayedDiagnostics, texts: m.compiledDocuments), "1 of 1: mystyle.sty line 3")
        let panel = DiagnosticsPanelState()
        m.goToOccurrence(0, of: groups[0], panel: panel)
        try await settle { m.activePath == "mystyle.sty" }
        XCTAssertEqual(m.documents.map(\.path), ["main.tex", "mystyle.sty"])
        XCTAssertEqual((m.activeText as NSString).substring(with: try XCTUnwrap(m.selection?.nsRange)), "\\foo")
        XCTAssertEqual(m.editorMarks.map(\.nsRange), [NSRange(location: 50, length: 4), NSRange(location: 26, length: 23)], "in the package buffer: its own two marks")
        XCTAssertTrue(m.navigationNote?.hasPrefix("\\foo is not supported by this compiler version") == true, m.navigationNote ?? "")
        XCTAssertEqual(m.editorLanguage, .package)
    }

    // MARK: the missing-package quick fixes

    func testMissingPackageQuickFixesCreateTheStyOrAskTheConsentSheet() async throws {
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("pkg-editor-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tmp) }
        let m = try loadedProject(tmp)
        let d = RuntimeV1.Diagnostic(severity: .warning, message: "packages cancel, siunitx are recognised but not implemented",
                                     source: .init(path: "main.tex", startByte: 24, endByte: 44), recovery: nil, code: "unsupported_feature",
                                     suggestion: "", notes: ["no project file found: looked for cancel.sty, cancel.cls"])
        // Both names are offered; mystyle (present) would not be.
        XCTAssertEqual(ProjectPackagesState.missingPackages(for: d, projectRoot: m.project.projectRoot), ["cancel", "siunitx"])
        XCTAssertEqual(ProjectPackagesState.missingPackages(for: d, projectRoot: nil), [], "no root, nothing to write")
        let present = RuntimeV1.Diagnostic(severity: .warning, message: "packages mystyle are recognised but not implemented", source: nil, recovery: nil)
        XCTAssertEqual(ProjectPackagesState.missingPackages(for: present, projectRoot: m.project.projectRoot), [], "the file exists: no create")
        // Create: the template lands next to the entry, opened and active; the fix is gone.
        let created = await m.createPackageFile(named: "cancel")
        XCTAssertEqual(created, .created(path: "cancel.sty"))
        let written = try String(contentsOf: tmp.appendingPathComponent("cancel.sty"), encoding: .utf8)
        XCTAssertTrue(written.hasPrefix("\\NeedsTeXFormat{LaTeX2e}\n\\ProvidesPackage{cancel}["), written)
        XCTAssertEqual(m.activePath, "cancel.sty")
        XCTAssertEqual(m.navigationNote, "Created cancel.sty from the package template; the next compile loads it for \\usepackage{cancel}")
        XCTAssertEqual(ProjectPackagesState.missingPackages(for: d, projectRoot: m.project.projectRoot), ["siunitx"])
        // Again: refused, never overwritten.
        let again = await m.createPackageFile(named: "cancel")
        XCTAssertEqual(again, .refused("cannot create cancel.sty: it is already open"))
        XCTAssertEqual(try String(contentsOf: tmp.appendingPathComponent("cancel.sty"), encoding: .utf8), written)
        let evil = await m.createPackageFile(named: "../evil")
        XCTAssertEqual(evil, .refused("../evil is not a package name"))
        // Fetch…: exactly that name goes to the helper without consent; the sheet shows the offer.
        let state = m.projectPackages
        var asked: [(names: [String], consent: Bool)] = []
        state.resolver = { _, names, consent in
            asked.append((names, consent))
            let rows = names.map { "{\"name\":\"\($0)\",\"status\":\"needs_consent\",\"version\":\"1.0\",\"source_url\":\"https://mirrors.ctan.org/macros/latex/contrib/\($0)/\",\"would_fetch\":[\"\($0).sty\"]}" }
            let json = "{\"cache\":\"/tmp/cache\",\"policy\":{\"source\":\"ctan\",\"fetch\":\"ask\"},\"diagnostics\":[],\"packages\":[\(rows.joined(separator: ","))]}"
            return .success(try! JSONDecoder().decode(ProjectFilesV1.ResolvePackages.self, from: Data(json.utf8)))
        }
        state.presentFetch(["siunitx"])
        try await settle { state.shown }
        XCTAssertEqual(asked.map(\.names), [["siunitx"]])
        XCTAssertEqual(asked.first?.consent, false, "nothing is fetched until the sheet's Fetch")
        XCTAssertEqual(state.offers.map(\.name), ["siunitx"])
        // Not now, then Fetch… again: the declined name is asked about again (a row click is explicit).
        state.notNow()
        XCTAssertFalse(state.shown)
        state.presentFetch(["siunitx"])
        try await settle { state.shown }
        XCTAssertEqual(asked.count, 2)
    }

    private static func json(_ s: String) -> String {
        String(data: try! JSONEncoder().encode(s), encoding: .utf8)!
    }

    private func settle(_ until: @escaping () -> Bool, file: StaticString = #filePath, line: UInt = #line) async throws {
        let deadline = Date().addingTimeInterval(5)
        while !until(), Date() < deadline { try await Task.sleep(nanoseconds: 20_000_000) }
        XCTAssertTrue(until(), "did not settle in 5 s", file: file, line: line)
    }

    // MARK: New File… templates

    func testNewFileKeepsAPackageOrClassExtensionAndWritesTheTemplate() {
        XCTAssertEqual(try NewFilePath.resolve("mystyle.sty").get(), "mystyle.sty")
        XCTAssertEqual(try NewFilePath.resolve("styles/thesis.cls").get(), "styles/thesis.cls")
        XCTAssertEqual(try NewFilePath.resolve("notes").get(), "notes.tex")
        XCTAssertEqual(try NewFilePath.resolve("notes.md").get(), "notes.md.tex", "only .tex/.sty/.cls are kept")
        XCTAssertEqual(NewFilePath.referenceCommand(for: "styles/mystyle.sty"), "\\usepackage{mystyle}")
        XCTAssertNil(NewFilePath.referenceCommand(for: "thesis.cls"))
        XCTAssertEqual(NewFilePath.referenceCommand(for: "ch/a.tex"), "\\input{ch/a}")
        let date = Date(timeIntervalSince1970: 0)
        let sty = try! XCTUnwrap(PackageTemplate.text(for: "styles/mystyle.sty", date: date))
        XCTAssertTrue(sty.hasPrefix("\\NeedsTeXFormat{LaTeX2e}\n\\ProvidesPackage{mystyle}[1970/01/01 v1.0 mystyle]\n"), sty)
        XCTAssertTrue(sty.contains("\\ProcessOptions\\relax"))
        let cls = try! XCTUnwrap(PackageTemplate.text(for: "thesis.cls", date: date))
        XCTAssertTrue(cls.contains("\\ProvidesClass{thesis}[1970/01/01 v1.0 thesis]"))
        XCTAssertTrue(cls.contains("\\LoadClass{article}"))
        XCTAssertNil(PackageTemplate.text(for: "notes.tex"))
        // The template is what the lexer treats as a package: `\@` never splits.
        XCTAssertFalse(spans(sty, language: .package).contains("command:\\@"))
    }

    func testNewStyFileIsWrittenWithTheTemplateAndReferencedWithUsepackage() async throws {
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("pkg-editor-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tmp) }
        let entry = tmp.appendingPathComponent("main.tex")
        try "\\documentclass{article}\n\n\\begin{document}\nx\n\\end{document}\n".write(to: entry, atomically: true, encoding: .utf8)
        let m = ShellModel()
        m.detachWorker()
        XCTAssertEqual(m.openTex(at: entry), .opened)
        m.caretUTF16 = 24 // the blank line after \documentclass
        let outcome = await m.project.newFile("mystyle.sty", insertReference: true)
        XCTAssertEqual(outcome, .created(path: "mystyle.sty"))
        let written = try String(contentsOf: tmp.appendingPathComponent("mystyle.sty"), encoding: .utf8)
        XCTAssertTrue(written.hasPrefix("\\NeedsTeXFormat{LaTeX2e}\n\\ProvidesPackage{mystyle}["), written)
        XCTAssertEqual(m.documents.map(\.path), ["main.tex", "mystyle.sty"])
        XCTAssertEqual(m.pendingEdit?.text.trimmingCharacters(in: .newlines), "\\usepackage{mystyle}", "the reference is a \\usepackage, not an \\input")
        XCTAssertEqual(m.project.listing[1].role, .included(from: "main.tex"))
        // A class: written with its template, nothing to insert, switched to at once.
        m.pendingEdit = nil
        let cls = await m.project.newFile("thesis.cls", insertReference: true)
        XCTAssertEqual(cls, .created(path: "thesis.cls"))
        XCTAssertNil(m.pendingEdit)
        XCTAssertEqual(m.activePath, "thesis.cls")
        XCTAssertEqual(m.editorLanguage, .package)
        XCTAssertTrue(try String(contentsOf: tmp.appendingPathComponent("thesis.cls"), encoding: .utf8).contains("\\LoadClass{article}"))
    }
}
