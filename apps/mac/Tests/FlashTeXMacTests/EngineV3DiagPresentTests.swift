import Foundation
import XCTest
import FlashTeXDisplayListV3
import FlashTeXProtocol
@testable import FlashTeXMac

/// Lane DIAG-PARITY (docs/evidence/diag-parity-2026-10-05): the new engine's
/// diagnostics at least as good as the old engine's. The DIAGs below are the
/// host's for the corpus cases they are named after (trimmed to the keys
/// that matter); the texts are the cases' own.
@MainActor
final class EngineV3DiagPresentTests: XCTestCase {
    let root = URL(fileURLWithPath: "/tmp/copy/src")

    func diag(_ fields: String) throws -> DL3Diag {
        try DL3Diag.decode(Array("{\"id\":1,\"seq\":0,\"exact\":true,\(fields)}".utf8))
    }

    func rows(_ diags: [DL3Diag], _ texts: [String: String], mode: EngineV3ErrorPolicy.Mode = .bestEffort) -> [RuntimeV1.Diagnostic] {
        let model = ShellModel()
        model.replaceProject(entryText: texts["main.tex"] ?? "", named: "main.tex")
        return EngineV3Session.problems(diags: diags, model: model, projectRoot: root, texts: texts, mode: mode)
    }

    func marked(_ r: RuntimeV1.Diagnostic, _ texts: [String: String]) -> String? {
        guard let s = r.source else { return nil }
        return texts[s.path]?.utf8Slice(s.startByte, s.endByte)
    }

    func fixed(_ r: RuntimeV1.Diagnostic, _ texts: [String: String]) -> [String: String] {
        guard let f = r.help?.replacement, let path = f.path ?? r.source?.path, let text = texts[path] else { return texts }
        var b = Array(text.utf8)
        b.replaceSubrange(f.startByte ..< f.endByte, with: Array(f.text.utf8))
        var out = texts
        out[path] = String(decoding: b, as: UTF8.self)
        return out
    }

    func testHeadlinesAreShortAndClean() {
        let h = EngineV3DiagPresent.headline
        XCTAssertEqual(h("latex/undefined-reference", "LaTeX Warning: Reference `sec:intr' on page 1 undefined on input line 6."),
                       "Reference `sec:intr' on page 1 undefined")
        XCTAssertEqual(h("latex/file-not-found", "LaTeX Error: File `nosuchpackage.sty' not found."), "File `nosuchpackage.sty' not found")
        XCTAssertEqual(h("package/hyperref/x", "Package hyperref Warning: Token not allowed in a PDF string (Unicode): removing `math shift' on input line 6."),
                       "hyperref: Token not allowed in a PDF string (Unicode): removing `math shift'")
        XCTAssertEqual(h("tex/undefined-control-sequence", "Undefined control sequence."), "Undefined control sequence")
        XCTAssertEqual(h("tex/extra-right-delimiter", "Extra \\right."), "Extra \\right.")
        XCTAssertEqual(h("tex/overfull-hbox", "Overfull \\hbox (7.77853pt too wide) in paragraph at lines 7--8"), "Overfull \\hbox")
        XCTAssertEqual(EngineV3DiagPresent.boxNote("Overfull \\hbox (7.77853pt too wide) in paragraph at lines 7--8"),
                       "7.78pt too wide, in the paragraph at lines 7–8")
        XCTAssertEqual(EngineV3DiagPresent.boxNote("Underfull \\hbox (badness 10000) in paragraph at lines 5--8"),
                       "badness 10000, in the paragraph at lines 5–8")
    }

    /// overfull-hbox, overfull-many: the box's own text is underlined (from
    /// its first character to its last), and fifty of them are one group.
    func testABoxIsUnderlinedOnItsLineAndBoxesGroup() throws {
        let line7 = "Short words then Antidisestablishmentarianism and more text follows here."
        let text = "\\documentclass{article}\n\\begin{document}\nA\n\nB\n\n\(line7)\n\n\\end{document}\n"
        let texts = ["main.tex": text]
        let box = { (pt: String) in try self.diag(#""severity":"warning","code":"tex/overfull-hbox","origin":"tex","message":"Overfull \\hbox (\#(pt)pt too wide) in paragraph at lines 7--7","detail":"\\OT1/cmr/m/n/10 Short words","file":"/tmp/copy/src/main.tex","line":7,"col":6,"end":{"file":"/tmp/copy/src/main.tex","line":7,"col":45},"lines":[7,7]"#) }
        let r = rows([try box("7.77853"), try box("12.5")], texts)
        XCTAssertEqual(r.count, 2)
        XCTAssertEqual(marked(r[0], texts), "words then Antidisestablishmentarianism")
        XCTAssertEqual(r[0].message, "Overfull \\hbox")
        XCTAssertEqual(r[0].notes?.first, EngineV3Explain.explanation(code: "tex/overfull-hbox", message: ""), "the explanation first")
        XCTAssertFalse(r[0].notes?.contains { $0.contains("\\OT1") } ?? true, "TeX's font display only without a source")
        XCTAssertEqual(r[0].notes?[1], "7.78pt too wide, in the paragraph at lines 7–7")
        let groups = EditorDiagnostics.groups(of: r)
        XCTAssertEqual(groups.count, 1)
        XCTAssertEqual(groups[0].title, "2× Overfull \\hbox")
    }

    /// package-not-found, undefined-ref, hyperref-token: the stop reports
    /// fold into their cause, LaTeX's summaries into the rows they sum up,
    /// and identical rows into one; the counts follow.
    func testRowsThatSayTheSameThingFold() throws {
        let text = "\\documentclass{article}\n\\usepackage{amsmath}\n\\usepackage{graphicx}\n\\usepackage{nosuchpackage}\n\\begin{document}\nText.\n\\end{document}\n"
        let texts = ["main.tex": text]
        let at = #""file":"/tmp/copy/src/main.tex","line":5,"col":6,"range":[0,6]"#
        let cause = try diag(#""severity":"error","code":"latex/file-not-found","origin":"latex","message":"LaTeX Error: File `nosuchpackage.sty' not found.","#+at)
        let stop = try diag(#""severity":"error","code":"tex/emergency-stop","origin":"tex","message":"Emergency stop.","fatal":true,"#+at)
        let fatal = try diag(#""severity":"error","code":"tex/fatal-error-no-output","origin":"tex","message":"==> Fatal error occurred, no output PDF file produced!","fatal":true,"exact":false,"file":"/tmp/copy/src/main.tex","line":5"#)
        let r = rows([cause, stop, fatal], texts)
        XCTAssertEqual(r.count, 1, r.map(\.message).description)
        XCTAssertEqual(r[0].severity, .error)
        XCTAssertEqual(r[0].message, "File `nosuchpackage.sty' not found")
        XCTAssertTrue(r[0].notes?.contains(EngineV3Explain.explanation(code: "tex/emergency-stop", message: "")!) ?? false)

        let ref = try diag(#""severity":"warning","code":"latex/undefined-reference","origin":"latex","message":"LaTeX Warning: Reference `a' on page 1 undefined on input line 6.","file":"/tmp/copy/src/main.tex","line":6"#)
        let sum = try diag(#""severity":"warning","code":"latex/there-were-undefined-references","origin":"latex","message":"LaTeX Warning: There were undefined references.","file":"/tmp/copy/src/main.tex","line":7"#)
        let tok = try diag(#""severity":"warning","code":"package/hyperref/token-not-allowed-in-a-pdf-string-unicode-removing","origin":"package","message":"Package hyperref Warning: Token not allowed in a PDF string (Unicode): removing `math shift' on input line 6.","file":"/tmp/copy/src/main.tex","line":6,"col":4"#)
        let r2 = rows([ref, sum, tok, tok], texts)
        XCTAssertEqual(r2.map(\.code), ["latex/undefined-reference", "package/hyperref/token-not-allowed-in-a-pdf-string-unicode-removing"])
        // A summary with nothing to sum up stays.
        XCTAssertEqual(rows([sum], texts).count, 1)
    }

    /// undefined-ref, undefined-cite: "did you mean" over the project's
    /// labels and bibliography keys, on the key inside the \ref / \cite.
    func testUndefinedReferenceAndCitationOfferTheCloseKey() throws {
        let text = "\\documentclass{article}\n\\begin{document}\n\\section{Intro}\\label{sec:intro}\nSee Section~\\ref{sec:intr}, \\cite{knuth84,lamport}.\n\\bibliography{refs}\n\\end{document}\n"
        let bib = "@book{knuth1984,\n  title = {The TeXbook},\n}\n@book{lamport94, title={LaTeX}}\n"
        let texts = ["main.tex": text, "refs.bib": bib]
        let ref = try diag(#""severity":"warning","code":"latex/undefined-reference","origin":"latex","message":"LaTeX Warning: Reference `sec:intr' on page 1 undefined on input line 4.","file":"/tmp/copy/src/main.tex","line":4,"range":[12,26]"#)
        let cite = try diag(#""severity":"warning","code":"latex/undefined-citation","origin":"latex","message":"LaTeX Warning: Citation `knuth84' on page 1 undefined on input line 4.","file":"/tmp/copy/src/main.tex","line":4,"range":[28,50]"#)
        let r = rows([ref, cite], texts)
        XCTAssertEqual(marked(r[0], texts), "\\ref{sec:intr}")
        XCTAssertEqual(r[0].help?.message, "did you mean sec:intro?")
        XCTAssertTrue(fixed(r[0], texts)["main.tex"]!.contains("\\ref{sec:intro}"))
        XCTAssertEqual(r[0].notes?.first, "No \\label{sec:intr} exists, so the reference prints ??. Check the key.")
        XCTAssertEqual(r[1].help?.message, "did you mean knuth1984?")
        XCTAssertTrue(fixed(r[1], texts)["main.tex"]!.contains("\\cite{knuth1984,lamport}"))
    }

    /// multiply-defined-label: LaTeX reports it while reading the .aux; the
    /// row goes to the second \label, the first is a secondary label.
    func testADuplicateLabelPointsAtBothLabels() throws {
        let text = "\\documentclass{article}\n\\begin{document}\n\\section{A}\\label{sec:a}\n\\section{B}\\label{sec:a}\n\\end{document}\n"
        let texts = ["main.tex": text]
        let d = try diag(#""severity":"warning","code":"latex/multiply-defined-label","origin":"latex","message":"LaTeX Warning: Label `sec:a' multiply defined.","file":"/tmp/xyz/main.aux","line":5,"col":39"#)
        let r = rows([d], texts)
        let src = try XCTUnwrap(r[0].source)
        XCTAssertEqual(src.path, "main.tex")
        XCTAssertEqual(marked(r[0], texts), "\\label{sec:a}")
        XCTAssertEqual(src.startByte, text.utf8.count - "\\label{sec:a}\n\\end{document}\n".utf8.count)
        XCTAssertEqual(r[0].labels?.filter { !$0.primary }.map(\.text), ["first defined at main.tex:3"])
    }

    /// error-in-macro: the row stays on the use; the definition of \note is
    /// a secondary label, the undefined \emphh inside it another, and the
    /// fix is made there (the use is right, the definition is not).
    func testAnErrorInAMacroPointsAtTheUseAndTheDefinition() throws {
        let text = "\\documentclass{article}\n\\newcommand{\\note}[1]{\\textbf{Note:} \\emphh{#1}}\n\\begin{document}\nFirst use: \\note{hello}.\n\\end{document}\n"
        let texts = ["main.tex": text]
        let d = try diag(##""severity":"error","code":"tex/undefined-control-sequence","origin":"tex","message":"Undefined control sequence.","file":"/tmp/copy/src/main.tex","line":4,"col":23,"range":[11,23],"trace":[{"kind":"macro","name":"\\note","text":["#1->\\textbf {Note:} \\emphh ","{#1}"]},{"kind":"file","file":"/tmp/copy/src/main.tex","line":4,"col":23,"text":["",""]}],"help":["The control sequence at the end of the top line"]"##)
        let r = rows([d], texts, mode: .strict)[0]
        XCTAssertEqual(marked(r, texts), "\\note{hello}")
        XCTAssertTrue(r.notes?.contains("in \\note (defined at main.tex:2)") ?? false, r.notes?.description ?? "")
        let secondary = r.labels?.filter { !$0.primary } ?? []
        XCTAssertEqual(secondary.map { texts["main.tex"]!.utf8Slice($0.source.startByte, $0.source.endByte) }, ["\\note", "\\emphh"])
        XCTAssertEqual(r.help?.message, "did you mean \\emph? (in the definition of \\note, main.tex:2)")
        XCTAssertTrue(fixed(r, texts)["main.tex"]!.contains("\\newcommand{\\note}[1]{\\textbf{Note:} \\emph{#1}}"))
        // Kernel macros say nothing to the author: no "in \GenericWarning".
        let w = try diag(#""severity":"warning","code":"latex/x","origin":"latex","message":"LaTeX Warning: x.","file":"/tmp/copy/src/main.tex","line":4,"trace":[{"kind":"macro","name":"\\GenericWarning ","text":["",""]}]"#)
        XCTAssertNil(rows([w], texts)[0].notes)
    }

    /// include-missing, align-no-amsmath, \mathbb without amssymb.
    func testMissingIncludeAndMissingPackagesAreLocatedAndFixed() throws {
        let text = "\\documentclass{article}\n\\begin{document}\nText.\n\\include{chap9}\n\\begin{align}x&=1\\end{align}\n$\\mathbb{R}$\n\\end{document}\n"
        let texts = ["main.tex": text]
        let nofile = try diag(#""severity":"warning","code":"latex/no-file","origin":"latex","message":"No file chap9.tex.","exact":false"#)
        let env = try diag(#""severity":"error","code":"latex/environment-undefined","origin":"latex","message":"LaTeX Error: Environment align undefined.","file":"/tmp/copy/src/main.tex","line":5,"range":[0,13]"#)
        let cs = try diag(#""severity":"error","code":"tex/undefined-control-sequence","origin":"tex","message":"Undefined control sequence.","file":"/tmp/copy/src/main.tex","line":6,"range":[1,8],"trace":[{"kind":"file","file":"/tmp/copy/src/main.tex","line":6,"col":8,"text":["$\\mathbb",""]}]"#)
        let r = rows([nofile, env, cs], texts)
        XCTAssertEqual(marked(r[0], texts), "\\include{chap9}")
        XCTAssertEqual(r[1].notes?.first, "The align environment is defined by the amsmath package, which is not loaded.")
        XCTAssertEqual(r[1].help?.message, "add \\usepackage{amsmath}")
        XCTAssertTrue(fixed(r[1], texts)["main.tex"]!.contains("\\usepackage{amsmath}\n\\begin{document}"))
        XCTAssertEqual(r[2].notes?.first, "\\mathbb is defined by the amssymb package, which is not loaded.")
        XCTAssertEqual(r[2].help?.message, "add \\usepackage{amssymb}")
    }
}
