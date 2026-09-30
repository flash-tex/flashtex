import XCTest
@testable import FlashTeXEditorCore

/// PLAN M7: wrap mode — `<<selection>>`, bare `*` across lines, list
/// markers, and the `table` and `align` transformers.
final class WrapTests: XCTestCase {
    typealias T = TeXpand
    let engine = T.Engine()

    func wrap(_ abbr: String, _ selection: String, scope: T.ScopeStack = .text()) -> String {
        engine.expandToString(abbr, in: T.Context(scope: scope, selection: selection))
    }

    func testBareStarDistributesLinesAndStripsMarkers() {
        XCTAssertEqual(wrap("enum>item*", "- apples\n* pears\n\n3. plums"),
                       "\\begin{enumerate}\n  \\item apples\n  \\item pears\n  \\item plums\n\\end{enumerate}",
                       "three lines + enum>item* → three items")
        XCTAssertEqual(wrap("items>item{@: }*", "a\nb"), "\\begin{itemize}\n  \\item 1: \n    a\n  \\item 2: \n    b\n\\end{itemize}",
                       "an item that already has its argument takes the line as its body; the counter runs")
        XCTAssertEqual(wrap("enum>item*", "  \n "), "error: the selection has no lines to distribute")
        XCTAssertEqual(engine.expandToString("enum>item*"), "error: a bare `*` repeats once per selected line; it works when wrapping a selection")
    }

    func testSelectionFillsTheInnermostLastElement() {
        XCTAssertEqual(wrap("sec", "Introduction"), "\\section{Introduction}", "the first missing argument")
        XCTAssertEqual(wrap("thm#main", "Every $x$ is $x$."), "\\begin{theorem}\\label{thm:main}\n  Every \\$x\\$ is \\$x\\$.\n\\end{theorem}",
                       "no argument slot: the body")
        XCTAssertEqual(wrap("fig>cap", "An overview"), "\\begin{figure}\n  \\centering\n  \\caption{An overview}\n\\end{figure}",
                       "the last element, not the outer one")
        XCTAssertEqual(wrap("eq", "E = mc^2"), "\\begin{equation}\n  E = mc^2\n\\end{equation}")
        XCTAssertEqual(wrap("frame{T}", "line one\n  nested\nline two"),
                       "\\begin{frame}{T}\n  line one\n    nested\n  line two\n\\end{frame}", "relative indentation kept")
    }

    func testTableTransformer() {
        let csv = "Name,Score,Time\nAda,10,3\nAlan,9,4"
        XCTAssertEqual(wrap("btab", csv), """
        \\begin{tabular}{lll}
          \\toprule
          Name & Score & Time \\\\
          \\midrule
          Ada & 10 & 3 \\\\
          Alan & 9 & 4 \\\\
          \\bottomrule
        \\end{tabular}
        """, "CSV + btab → a booktabs table")
        XCTAssertEqual(wrap("tab:lrr", "a\t1\t2\nb\t3"), "\\begin{tabular}{lrr}\n  a & 1 & 2 \\\\\n  b & 3 & \n\\end{tabular}",
                       "TSV, the given spec, short rows padded")
        XCTAssertEqual(wrap("btab.float#res", "x,y\n1,2"), """
        \\begin{table}
          \\centering
          \\caption{$1}
          \\label{tab:res}
          \\begin{tabular}{ll}
            \\toprule
            x & y \\\\
            \\midrule
            1 & 2 \\\\
            \\bottomrule
          \\end{tabular}
        \\end{table}
        """)
    }

    func testAlignTransformer() {
        XCTAssertEqual(wrap("align", "a = b + c\nf(x) \\leq g(x)\nx &= y"),
                       "\\begin{align}\n  a &= b + c \\\\\n  f(x) &\\leq g(x) \\\\\n  x &= y\n\\end{align}")
        XCTAssertEqual(T.alignRow("{a=b} = c"), "{a=b} &= c", "not inside braces")
        XCTAssertEqual(T.alignRow("x \\leqslant y"), "x \\leqslant y", "a relation's prefix is not the relation")
        XCTAssertEqual(T.alignRow("no relation"), "no relation")
    }

    func testWrapLines() {
        XCTAssertEqual(T.wrapLines("  - a\n2) b\n\\item c\n+d"), ["a", "b", "c", "+d"])
        XCTAssertEqual(T.tableCells(["a, {b,c}, d"]), [["a", "{b,c}", "d"]])
    }
}
