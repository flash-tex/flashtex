import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Outline structure (lane mac-editor-dx-3, DocumentOutline.swift): captions
/// of floats and theorem-like environments, `\newtheorem` names, and the
/// follow-caret current item.
final class OutlineStructureTests: XCTestCase {
    func testFloatsAndTheoremsCarryCaptions() {
        let text = """
        \\newtheorem{claim}{Claim}
        \\part{One}
        \\chapter{Intro}
        \\section{Setup}
        \\begin{figure}
          \\includegraphics{x}
          \\caption{Loss  curves over
          epochs}\\label{fig:loss}
        \\end{figure}
        \\begin{theorem}[Main result]
        x
        \\end{theorem}
        \\begin{claim}
        Every   bounded sequence has a convergent subsequence and more words than fit
        \\end{claim}
        \\begin{itemize}\\item a\\end{itemize}
        \\begin{table*}\\caption{Table cap}\\end{table*}
        """
        let items = DocumentOutline.scan(text)
        XCTAssertEqual(items.filter { $0.kind == .section }.map { "\($0.command):\($0.level)" }, ["part:0", "chapter:0", "section:1"])
        let envs = items.filter { $0.kind == .environment }
        XCTAssertEqual(envs.map(\.title), ["figure", "theorem", "claim", "itemize", "table*"])
        XCTAssertEqual(envs[0].caption, "Loss curves over epochs")
        XCTAssertEqual(envs[0].displayTitle, "figure: Loss curves over epochs")
        XCTAssertEqual(envs[1].caption, "Main result")
        XCTAssertEqual(envs[2].caption?.count, 60, "first words, ellipsised")
        XCTAssertTrue(envs[2].caption?.hasPrefix("Every bounded sequence") == true && envs[2].caption?.hasSuffix("…") == true)
        XCTAssertNil(envs[3].caption, "plain environments have no caption")
        XCTAssertEqual(envs[4].caption, "Table cap")
        XCTAssertEqual(items.first { $0.kind == .label }?.title, "fig:loss")
        // An unclosed float still reads its caption; a caption outside a float is ignored.
        let open = DocumentOutline.scan("\\begin{figure}\\caption{c}")
        XCTAssertEqual(open.first?.caption, "c")
        XCTAssertNil(DocumentOutline.scan("\\caption{x}\\begin{figure}\\end{figure}").first?.caption)
    }

    func testCurrentItemFollowsTheCaret() {
        let text = "\\section{A}\ntext\n\\subsection{B}\n\\begin{proof}\nx\n\\end{proof}\n\\section{C}\n"
        let items = DocumentOutline.scan(text)
        XCTAssertNil(DocumentOutline.current(at: 0, in: []))
        XCTAssertEqual(DocumentOutline.current(at: 0, in: items)?.title, "A")
        XCTAssertEqual(DocumentOutline.current(at: 14, in: items)?.title, "A")
        XCTAssertEqual(DocumentOutline.current(at: 20, in: items)?.title, "B")
        XCTAssertEqual(DocumentOutline.current(at: 45, in: items)?.title, "B", "a section wins over an environment inside it")
        XCTAssertEqual(DocumentOutline.current(at: text.utf16.count, in: items)?.title, "C")
        // Without sections the innermost environment at or before the caret is current.
        let envOnly = DocumentOutline.scan("\\begin{itemize}\n\\begin{enumerate}\nx\n\\end{enumerate}\n\\end{itemize}")
        XCTAssertEqual(DocumentOutline.current(at: 35, in: envOnly)?.title, "enumerate")
        XCTAssertEqual(DocumentOutline.current(at: 3, in: envOnly)?.title, "itemize")
    }

    /// The status bar's breadcrumb chain: enclosing sections only, outermost
    /// first, ancestors being the nearest preceding lower level.
    func testBreadcrumbChain() {
        let text = """
        preamble text
        \\chapter{One}
        \\section{Setup}
        \\subsection{Detail}
        \\section{Results}
        tail text
        """
        let items = DocumentOutline.scan(text)
        let ns = text as NSString
        func caretAfter(_ needle: String) -> Int {
            let r = ns.range(of: needle)
            return r.location + r.length
        }
        // Inside \subsection{Detail}: chapter › section › subsection.
        XCTAssertEqual(DocumentOutline.breadcrumb(at: caretAfter("Detail}"), in: items).map(\.title),
                       ["One", "Setup", "Detail"])
        // After \section{Results}: the subsection is closed by the new section.
        XCTAssertEqual(DocumentOutline.breadcrumb(at: ns.length, in: items).map(\.title),
                       ["One", "Results"])
        // Before any section (in the preamble): empty.
        XCTAssertEqual(DocumentOutline.breadcrumb(at: 0, in: items), [])
        XCTAssertEqual(DocumentOutline.breadcrumb(at: caretAfter("preamble"), in: items), [])
    }
}
