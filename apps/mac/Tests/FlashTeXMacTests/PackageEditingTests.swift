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
