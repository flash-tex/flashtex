import Foundation
import XCTest
import FlashTeXDisplayListV3
import FlashTeXProtocol
@testable import FlashTeXMac

/// diag-v1 DIAGs in the Problems panel: the underline is the reported
/// command (`range`, byte columns of `line`), so a click lands on its exact
/// column; the macro chain and TeX's help ride along; `info` is not a problem.
@MainActor
final class EngineV3DiagMappingTests: XCTestCase {
    func testDiagsMapToExactColumns() throws {
        let model = ShellModel()
        let text = "\\documentclass{article}\n\\newcommand\\mycmd[1]{\\textbf{#1 \\undefinedthing}}\n\\begin{document}\nSome naïve text \\mycmd{x} more.\n\\end{document}\n"
        model.replaceProject(entryText: text, named: "main.tex")
        let root = URL(fileURLWithPath: "/tmp/copy/src")
        // Line 4, `\mycmd{x}`: byte columns after "Some naïve text " (ï is 2 bytes).
        let lineStart = Array(text.utf8).split(separator: 0x0A, omittingEmptySubsequences: false).prefix(3).reduce(0) { $0 + $1.count + 1 }
        let from = "Some naïve text ".utf8.count, to = from + "\\mycmd{x}".utf8.count
        let json = """
        {"id":1,"seq":0,"severity":"error","code":"tex/undefined-control-sequence","origin":"tex",
         "message":"Undefined control sequence.","file":"/tmp/copy/src/main.tex","line":4,"col":\(to),"range":[\(from),\(to)],
         "trace":[{"kind":"macro","name":"\\\\mycmd","text":["",""],"def":{"file":"/tmp/copy/src/main.tex","line":2}},
                  {"kind":"file","file":"/tmp/copy/src/main.tex","line":4,"col":\(to),"text":["",""]}],
         "help":["The control sequence at the end of the top line","of your error message was never \\\\def'ed."],"exact":true}
        """
        let pkg = #"{"id":1,"seq":1,"severity":"warning","code":"package/hyperref/x","origin":"package","message":"Package hyperref Warning: x.","file":"/usr/local/texlive/2026/texmf-dist/tex/latex/hyperref/hyperref.sty","line":10,"exact":true}"#
        let info = #"{"id":1,"seq":2,"severity":"info","code":"tex/underfull-hbox","origin":"tex","message":"Underfull \\hbox (badness 10000)","exact":true}"#
        let diags = try [json, pkg, info].map { try DL3Diag.decode(Array($0.utf8)) }
        let problems = EngineV3Session.problems(diags: diags, model: model, projectRoot: root)
        XCTAssertEqual(problems.count, 2, "info is not a problem")
        let e = problems[0]
        XCTAssertEqual(e.severity, .error)
        XCTAssertEqual(e.code, "tex/undefined-control-sequence")
        let src = try XCTUnwrap(e.source)
        XCTAssertEqual(src.path, "main.tex")
        XCTAssertEqual(src.startByte, lineStart + from)
        XCTAssertEqual(String(decoding: Array(text.utf8)[src.startByte ..< src.endByte], as: UTF8.self), "\\mycmd{x}")
        XCTAssertEqual(e.notes, ["in \\mycmd (defined at main.tex:2)"])
        XCTAssertTrue(e.help?.message.hasPrefix("The control sequence") ?? false)
        let w = problems[1]
        XCTAssertNil(w.source)
        XCTAssertTrue(w.message.hasPrefix("hyperref.sty:10: "), w.message)
    }
}
