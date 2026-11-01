import XCTest
@testable import FlashTeXEditorCore

/// PLAN M10b (owner addition): the structure editor's core — finding the
/// grid around the caret, editing it, and writing it back.
final class StructureTests: XCTestCase {
    typealias T = TeXpand
    let all = T.StructureProvider.builtIns

    func target(_ text: String, at marker: String = "‸") -> T.StructureTarget? {
        let r = (text as NSString).range(of: marker)
        let clean = (text as NSString).replacingCharacters(in: r, with: "")
        return T.structure(at: r.location, in: clean as NSString, providers: all)
    }

    func testFindsTheInnermostGrid() throws {
        let m = try XCTUnwrap(target("$\\begin{pmatrix}\n  a & b \\\\\n  c & ‸d\n\\end{pmatrix}$"))
        XCTAssertEqual(m.provider.name, "matrix")
        XCTAssertEqual(m.document.environment, "pmatrix")
        XCTAssertEqual(m.document.cells, [["a", "b"], ["c", "d"]])
        XCTAssertEqual(m.range.location, 1)
        let nested = try XCTUnwrap(target("\\begin{align}\n  A &= \\begin{bmatrix} 1 & ‸0 \\\\ 0 & 1 \\end{bmatrix}\n\\end{align}"))
        XCTAssertEqual(nested.document.environment, "bmatrix", "the innermost")
        let outer = try XCTUnwrap(target("\\begin{align}\n  A &‸= \\begin{bmatrix} 1 \\end{bmatrix}\n\\end{align}"))
        XCTAssertEqual(outer.document.environment, "align")
        XCTAssertNil(target("Text ‸ outside."))
        XCTAssertNil(target("\\begin{itemize} \\item ‸ \\end{itemize}"), "not a grid")
        XCTAssertNil(T.structure(at: 20, in: "$\\begin{pmatrix} a & b \\end{pmatrix}$", providers: all.filter { $0.name != "matrix" }),
                     "a provider switched off")
    }

    func testTabularSpecAndRules() throws {
        let t = try XCTUnwrap(target("  \\begin{tabular}[t]{l|c|r}\n    \\hline\n    A & B & ‸C \\\\\n    \\hline\n    1 & 2 & 3 \\\\\n  \\end{tabular}"))
        XCTAssertEqual(t.document.leading, "[t]")
        XCTAssertEqual(t.document.colspec?.text, "l|c|r")
        XCTAssertEqual(t.indent, "  ")
        XCTAssertEqual(t.document.grid.rows, [.rule("\\hline"), .cells(["A", "B", "C"]), .rule("\\hline"), .cells(["1", "2", "3"])])
        let x = try XCTUnwrap(target("\\begin{tabularx}{\\linewidth}{lX}\n  a & ‸b\n\\end{tabularx}"))
        XCTAssertEqual(x.document.leading, "{\\linewidth}")
        XCTAssertEqual(x.document.colspec?.text, "lX")
    }

    func testEditsKeepTheSpecInSync() throws {
        var d = try XCTUnwrap(target("\\begin{tabular}{l|c|r}\n  A & B & ‸C\n\\end{tabular}")).document
        d.addColumn(after: 0)
        XCTAssertEqual(d.colspec?.text, "l|l|c|r", "a column like its neighbour, the rule kept between")
        XCTAssertEqual(d.cells, [["A", "", "B", "C"]])
        d.removeColumn(2)
        XCTAssertEqual(d.colspec?.text, "l|l|r")
        d.addColumn(after: 2)
        XCTAssertEqual(d.colspec?.text, "l|l|rr")
        d.removeColumn(0)
        XCTAssertEqual(d.colspec?.text, "l|rr", "no leading rule left over")
        var star = try XCTUnwrap(T.ColumnSpec("*{3}{c}|l"))
        XCTAssertEqual(star.text, "*{3}{c}|l", "an unedited spec stays as written")
        star.insertColumn(after: 3)
        XCTAssertEqual(star.text, "ccc|ll", "*{n}{…} expands once a column changes")
        XCTAssertEqual(T.ColumnSpec(">{\\bfseries}lp{2cm}")?.columnCount, 2)
        XCTAssertNil(T.ColumnSpec("l@"))
    }

    func testRowsCellsAndRender() throws {
        var t = try XCTUnwrap(target("  \\begin{pmatrix}\n    a & ‸b \\\\\n    c & d\n  \\end{pmatrix}"))
        t.document.setCell(0, 1, "x")
        t.document.addRow(after: 1)
        t.document.setCell(2, 0, "e")
        t.document.addColumn(after: 1)
        t.document.environment = "bmatrix"
        let out = t.document.render(indent: t.indent, unit: "  ")
        XCTAssertEqual(out.text, "\\begin{bmatrix}\n    a & x & \\\\\n    c & d & \\\\\n    e & &\n  \\end{bmatrix}")
        XCTAssertEqual((out.text as NSString).substring(from: out.cellOffsets[0][1]).prefix(1), "x")
        t.document.removeRow(0)
        XCTAssertEqual(t.document.cells, [["c", "d", ""], ["e", "", ""]])
        var one = T.StructureDocument(environment: "matrix", leading: "", colspec: nil, grid: T.Grid(rows: [.cells(["a"])]))
        one.removeRow(0)
        one.removeColumn(0)
        XCTAssertEqual(one.cells, [["a"]], "the last row and column stay")
        let booktabs = try XCTUnwrap(target("\\begin{tabular}{ll}\n  \\toprule\n  A & ‸B \\\\\n  \\midrule\n  1 & 2 \\\\\n  \\bottomrule\n\\end{tabular}"))
        XCTAssertEqual(booktabs.document.render(indent: "", unit: "  ").text,
                       "\\begin{tabular}{ll}\n  \\toprule\n  A & B \\\\\n  \\midrule\n  1 & 2 \\\\\n  \\bottomrule\n\\end{tabular}", "round trip")
    }

    func testEditsKeepUntouchedRowsSpecsAndBreaks() throws {
        var t = try XCTUnwrap(target("\\begin{tabular}{*{3}{c}}\n  \\multicolumn{3}{c}{Title} \\\\[2pt]\n  a &‸b& c \\\\*\n  d & e & f\n\\end{tabular}"))
        XCTAssertEqual(t.document.columnCount, 3)
        XCTAssertEqual(t.document.cells[0], ["\\multicolumn{3}{c}{Title}"], "a multicolumn{3} row spans every column")
        t.document.setCell(1, 1, "x")
        var out = t.document.render(indent: "", unit: "  ")
        XCTAssertEqual(out.text, "\\begin{tabular}{*{3}{c}}\n  \\multicolumn{3}{c}{Title} \\\\[2pt]\n  a &x& c \\\\*\n  d & e & f\n\\end{tabular}",
                       "only the edited cell's span changes; the spec, \\\\[2pt] and \\\\* stay")
        XCTAssertEqual((out.text as NSString).substring(from: out.cellOffsets[1][1]).prefix(1), "x")
        t.document.addRow(after: 0)
        out = t.document.render(indent: "", unit: "  ")
        XCTAssertTrue(out.text.contains("Title} \\\\[2pt]\n  & & \\\\\n  a &x& c"), out.text)
    }

    func testRemoveRowKeepsSourcesInStep() throws {
        var t = try XCTUnwrap(target("\\begin{tabular}{ll}\n  \\hline\n  a & b \\\\[2pt]\n  c&‸d \\\\*\n  e & f\n\\end{tabular}"))
        t.document.removeRow(0)
        XCTAssertEqual(t.document.sources.count, t.document.grid.rows.count)
        XCTAssertEqual(t.document.grid.rows, [.rule("\\hline"), .cells(["c", "d"]), .cells(["e", "f"])])
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, "\\begin{tabular}{ll}\n  \\hline\n  c&d \\\\*\n  e & f\n\\end{tabular}",
                       "the rows left keep their own text and breaks")
        t.document.addRow(after: -1)
        t.document.removeRow(1)
        XCTAssertEqual(t.document.sources.count, t.document.grid.rows.count)
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, "\\begin{tabular}{ll}\n  \\hline\n  & \\\\\n  e & f\n\\end{tabular}")
    }

    func spans() throws -> T.StructureDocument {
        try XCTUnwrap(target("""
        \\begin{tabular}{cccc}
          \\multicolumn{2}{c}{AB} & C & D \\\\
          a & \\multicolumn{2}{|c|}{\\textbf{BC}} & d \\\\
          a & b & \\multicolumn{2}{c}{CD} \\\\
          a & b & c & ‸d
        \\end{tabular}
        """)).document
    }

    func testColumnsCountMulticolumnSpans() throws {
        let d = try spans()
        XCTAssertEqual(d.columnCount, 4)
        XCTAssertEqual((0..<4).map { d.cellIndex(atColumn: $0, inRow: 1) }, [0, 1, 1, 2], "the middle span covers columns 1 and 2")
        XCTAssertEqual((0..<3).map { d.column(ofCell: $0, inRow: 0) }, [0, 2, 3])
        XCTAssertEqual(d.span(ofCell: 2, inRow: 2), 2)
        XCTAssertEqual(T.Grid.cell("\\multicolumn{2}{|c|}{\\textbf{x}}", spanning: 3), "\\multicolumn{3}{|c|}{\\textbf{x}}")
        XCTAssertEqual(T.Grid.cell("\\multicolumn{2}{|c|}{\\textbf{x}}", spanning: 1, columnSpec: "| c |"), "\\textbf{x}",
                       "one column with the column's own spec: a plain cell")
        XCTAssertEqual(T.Grid.cell("\\multicolumn{2}{|c|}{\\textbf{x}}", spanning: 1, columnSpec: "c"), "\\multicolumn{1}{|c|}{\\textbf{x}}",
                       "a spec of its own is kept at one column")
        XCTAssertEqual(T.Grid.cell("\\multicolumn{2}{|c|}{\\textbf{x}}", spanning: 1), "\\multicolumn{1}{|c|}{\\textbf{x}}", "no table spec: kept")
        XCTAssertEqual(T.Grid.cell("x", spanning: 1), "x")
    }

    func testRemoveColumnNarrowsSpans() throws {
        var d = try spans()
        d.removeColumn(1)
        XCTAssertEqual(d.colspec?.text, "ccc")
        XCTAssertEqual(d.cells, [["AB", "C", "D"], ["a", "\\multicolumn{1}{|c|}{\\textbf{BC}}", "d"], ["a", "\\multicolumn{2}{c}{CD}"], ["a", "c", "d"]],
                       "a span at the start or middle narrows (to a plain cell when its spec is the column's); a plain cell goes")
        d = try spans()
        d.removeColumn(3)
        XCTAssertEqual(d.cells, [["\\multicolumn{2}{c}{AB}", "C"], ["a", "\\multicolumn{2}{|c|}{\\textbf{BC}}"], ["a", "b", "CD"], ["a", "b", "c"]],
                       "a span at the end narrows")
        let out = d.render(indent: "", unit: "  ").text
        XCTAssertTrue(out.contains("\n  a & b & CD \\\\\n"), out)
        XCTAssertTrue(out.contains("\n  \\multicolumn{2}{c}{AB} & C \\\\\n"), out)
        d = try spans()
        d.removeColumn(0)
        XCTAssertEqual(d.cells.map(\.count), [3, 2, 2, 3])
        XCTAssertEqual(d.cells[0], ["AB", "C", "D"])
    }

    func testAddColumnWidensSpansItFallsInside() throws {
        var d = try spans()
        d.addColumn(after: 0)
        XCTAssertEqual(d.colspec?.text, "ccccc")
        XCTAssertEqual(d.cells, [["\\multicolumn{3}{c}{AB}", "C", "D"], ["a", "", "\\multicolumn{2}{|c|}{\\textbf{BC}}", "d"],
                                 ["a", "", "b", "\\multicolumn{2}{c}{CD}"], ["a", "", "b", "c", "d"]],
                       "inside the leading span it widens; elsewhere a cell goes in")
        d = try spans()
        d.addColumn(after: 1)
        XCTAssertEqual(d.cells, [["\\multicolumn{2}{c}{AB}", "", "C", "D"], ["a", "\\multicolumn{3}{|c|}{\\textbf{BC}}", "d"],
                                 ["a", "b", "", "\\multicolumn{2}{c}{CD}"], ["a", "b", "", "c", "d"]],
                       "after a span's last column it stays; inside the middle span it widens")
        d = try spans()
        d.addColumn(after: 2)
        XCTAssertEqual(d.cells[2], ["a", "b", "\\multicolumn{3}{c}{CD}"], "the trailing span widens")
        XCTAssertEqual(d.cells.map { T.Grid.span(of: $0) }, [5, 5, 5, 5])
        d = try spans()
        d.addColumn(after: -1)
        XCTAssertEqual(d.cells[0], ["", "\\multicolumn{2}{c}{AB}", "C", "D"])
    }

    func testNarrowingToOneColumnKeepsASpecOfItsOwn() throws {
        var d = try XCTUnwrap(target("\\begin{tabular}{|c|c|c|}\n  \\multicolumn{2}{|c|}{A} & B \\\\\n  x & \\multicolumn{2}{c|}{Y} \\\\\n  1 & 2 & ‸3\n\\end{tabular}")).document
        d.removeColumn(1)
        XCTAssertEqual(d.colspec?.text, "|c|c|")
        XCTAssertEqual(d.cells[0], ["A", "B"], "`|c|` is the first column's own spec in `|c|c|`")
        XCTAssertEqual(d.cells[1], ["x", "Y"], "`c|` is the second column's")
        d = try XCTUnwrap(target("\\begin{tabular}{lcc}\n  \\multicolumn{2}{|c|}{A} & B \\\\\n  1 & 2 & ‸3\n\\end{tabular}")).document
        d.removeColumn(0)
        XCTAssertEqual(d.cells[0], ["\\multicolumn{1}{|c|}{A}", "B"], "a real override stays a \\multicolumn{1}")
        XCTAssertEqual(d.columnCount, 2)
        XCTAssertEqual(T.ColumnSpec(">{\\bfseries}l@{:}c<{x}|")?.columnText(1), "c<{x}|")
        XCTAssertEqual(T.ColumnSpec(">{\\bfseries}l@{:}>{\\it}c")?.columnText(1), ">{\\it}c")
        XCTAssertEqual(T.ColumnSpec(">{\\bfseries}l@{:}>{\\it}c")?.columnText(0), ">{\\bfseries}l@{:}")
    }

    func testUnbracedMulticolumnArgumentsParseTheSameEverywhere() throws {
        for cell in ["\\multicolumn{2}{c}{AB}", "\\multicolumn{2}c{AB}", "\\multicolumn2c{AB}", "\\multicolumn 2 c {AB}", "\\multicolumn2{c}AB"] {
            XCTAssertEqual(T.Grid.span(ofCell: cell), 2, cell)
            let wider = T.Grid.cell(cell, spanning: 3)
            XCTAssertEqual(T.Grid.span(ofCell: wider), 3, wider)
            XCTAssertTrue(wider.hasPrefix("\\multicolumn{3}"), wider)
            XCTAssertEqual(T.Grid.cell(cell, spanning: 1, columnSpec: "c"), "AB", cell)
        }
        XCTAssertEqual(T.Grid.cell("\\multicolumn2c{AB}", spanning: 3), "\\multicolumn{3}c{AB}", "the spec and content stay as written")
        XCTAssertEqual(T.Grid.cell("\\multicolumn{2}\\bfseries{AB}", spanning: 3), "\\multicolumn{3}\\bfseries{AB}", "a control sequence is one token")
        XCTAssertEqual(T.Grid.span(ofCell: "\\multicolumns{2}{c}{AB}"), 1, "another command")
        XCTAssertEqual(T.Grid.span(ofCell: "\\multicolumn{2}{c}"), 1, "missing its content: not a span")
        XCTAssertEqual(T.Grid.cell("\\multicolumn{2}{c}", spanning: 3), "\\multicolumn{2}{c}", "and left alone")

        // Column operations keep the table valid with the unbraced forms.
        var d = try XCTUnwrap(target("\\begin{tabular}{ccc}\n  \\multicolumn2c{AB} & C \\\\\n  a & \\multicolumn{2}c{BC} \\\\\n  a & b & ‸c\n\\end{tabular}")).document
        XCTAssertEqual(d.cells.map { T.Grid.span(of: $0) }, [3, 3, 3])
        d.addColumn(after: 0)
        XCTAssertEqual(d.cells, [["\\multicolumn{3}c{AB}", "C"], ["a", "", "\\multicolumn{2}c{BC}"], ["a", "", "b", "c"]])
        d.removeColumn(0)
        d.removeColumn(0)
        XCTAssertEqual(d.cells, [["AB", "C"], ["\\multicolumn{2}c{BC}"], ["b", "c"]])
        XCTAssertEqual(d.cells.map { T.Grid.span(of: $0) }, [2, 2, 2])
    }

    func testUntouchedRowsKeepTheirSpacingBeforeTheBreak() throws {
        let source = "\\begin{tabular}{ll}\n  a & b\\\\\n  c & d  \\\\[2pt]\n  e&‸f\t\\\\*\n  g & h\n\\end{tabular}"
        var t = try XCTUnwrap(target(source))
        let clean = source.replacingOccurrences(of: "‸", with: "")
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, clean, "round trip, byte for byte")
        t.document.setCell(3, 1, "x")
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, clean.replacingOccurrences(of: "g & h", with: "g & x"))
        t.document.setCell(1, 0, "y")
        XCTAssertTrue(t.document.render(indent: "", unit: "  ").text.contains("\n  y & d  \\\\[2pt]\n"), "an edited row keeps its gap too")
        t.document.addRow(after: 3)
        XCTAssertTrue(t.document.render(indent: "", unit: "  ").text.contains("\n  g & x \\\\\n  &\n"),
                      t.document.render(indent: "", unit: "  ").text)
    }

    func testANewEmptyOneColumnLastRowIsNotAnIndentOnlyLine() throws {
        var t = try XCTUnwrap(target("\\begin{tabular}{l}\n  a \\\\\n  ‸b\n\\end{tabular}"))
        t.document.addRow(after: 1)
        let out = t.document.render(indent: "", unit: "  ")
        XCTAssertEqual(out.text, "\\begin{tabular}{l}\n  a \\\\\n  b \\\\\n  \\\\\n\\end{tabular}")
        XCTAssertFalse(out.text.components(separatedBy: "\n").contains { !$0.isEmpty && $0.allSatisfy(\.isWhitespace) })
        XCTAssertTrue((out.text as NSString).substring(from: out.cellOffsets[2][0]).hasPrefix("\\\\\n\\end"), "the caret goes before its break")
        t.document.setCell(2, 0, "c")
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, "\\begin{tabular}{l}\n  a \\\\\n  b \\\\\n  c\n\\end{tabular}",
                       "once it has text, the table ends as it was written: no final break")
    }

    func testAFinalBreakWithASuffix() throws {
        let source = "\\begin{pmatrix}\n  a & b \\\\\n  c & ‸d \\\\[2pt]\n\\end{pmatrix}"
        var t = try XCTUnwrap(target(source))
        XCTAssertEqual(t.document.finalBreak, "\\\\[2pt]")
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, source.replacingOccurrences(of: "‸", with: ""), "round trip")
        t.document.setCell(1, 1, "x")
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, "\\begin{pmatrix}\n  a & b \\\\\n  c & x \\\\[2pt]\n\\end{pmatrix}")
        t.document.addRow(after: 1)
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text,
                       "\\begin{pmatrix}\n  a & b \\\\\n  c & x \\\\[2pt]\n  & \\\\[2pt]\n\\end{pmatrix}",
                       "the row keeps its own break; the new last row takes the final one")
        t.document.removeRow(1)
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, "\\begin{pmatrix}\n  a & b \\\\\n  & \\\\[2pt]\n\\end{pmatrix}")
    }

    /// The structure editor's Tab walks cells by index (the next row's first
    /// after a row's last); Return goes to the cell below covering the same
    /// first column (`focusColumn(r + 1, column(ofCell: c, inRow: r))`).
    func testTabAndReturnBetweenRowsOfDifferentShapes() throws {
        let d = try spans()
        let ret = { (r: Int, c: Int) in d.cellIndex(atColumn: d.column(ofCell: c, inRow: r), inRow: r + 1) }
        // Row 0: [AB(2) C D]; row 1: [a BC(2) d]; row 2: [a b CD(2)]; row 3: [a b c d].
        XCTAssertEqual((0..<3).map { ret(0, $0) }, [0, 1, 2], "AB→a, C (col 2)→BC, D (col 3)→d")
        XCTAssertEqual((0..<3).map { ret(1, $0) }, [0, 1, 2], "a→a, BC (col 1)→b, d (col 3)→CD")
        XCTAssertEqual((0..<3).map { ret(2, $0) }, [0, 1, 2], "CD (col 2)→c")
        XCTAssertEqual(d.cells.map(\.count), [3, 3, 3, 4], "Tab: a row's last cell index before the wrap")
        // Tab wraps after each row's last cell, which ends at the last column.
        for r in 0..<3 {
            let last = d.cells[r].count - 1
            XCTAssertEqual(d.column(ofCell: last, inRow: r) + d.span(ofCell: last, inRow: r), 4, "row \(r) ends at the last column")
        }
        // Past a short row's end Return lands on its last cell.
        var short = T.StructureDocument(environment: "tabular", leading: "", colspec: T.ColumnSpec("lll"),
                                        grid: T.Grid(rows: [.cells(["a", "b", "c"]), .cells(["\\multicolumn2c{x}"])]))
        XCTAssertEqual(short.cells[1], ["\\multicolumn2c{x}", ""], "an unbraced span pads by its width")
        XCTAssertEqual(short.cellIndex(atColumn: short.column(ofCell: 2, inRow: 0), inRow: 1), 1)
        XCTAssertEqual(short.cellIndex(atColumn: short.column(ofCell: 1, inRow: 0), inRow: 1), 0)
        short.setCell(1, 1, "z")
        XCTAssertEqual(short.cells[1], ["\\multicolumn2c{x}", "z"])
    }

    func testUnchangedEmptyCellOffsets() throws {
        var t = try XCTUnwrap(target("\\begin{tabular}{lll}\n  a&&c \\\\\n  a & & c \\\\\n  d & ‸e &\n\\end{tabular}"))
        t.document.setCell(2, 1, "x")
        let out = t.document.render(indent: "", unit: "  ")
        let text = out.text as NSString
        XCTAssertEqual(out.text, "\\begin{tabular}{lll}\n  a&&c \\\\\n  a & & c \\\\\n  d & x &\n\\end{tabular}")
        XCTAssertTrue(text.substring(from: out.cellOffsets[0][1]).hasPrefix("&c"), "between `&&`, not past the next one")
        XCTAssertTrue(text.substring(from: out.cellOffsets[1][1]).hasPrefix("& c"), "after the one space it has")
        XCTAssertTrue(text.substring(from: out.cellOffsets[2][2]).hasPrefix("\n\\end"), "a trailing `&`: right after it")
        XCTAssertTrue(text.substring(from: out.cellOffsets[2][1]).hasPrefix("x &"))
    }

    func testTheLastRowKeepsItsTrailingBreak() throws {
        let source = "\\begin{pmatrix}\n  a & b \\\\\n  c & ‸d \\\\\n\\end{pmatrix}"
        var t = try XCTUnwrap(target(source))
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, source.replacingOccurrences(of: "‸", with: ""), "round trip")
        t.document.setCell(1, 1, "x")
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, "\\begin{pmatrix}\n  a & b \\\\\n  c & x \\\\\n\\end{pmatrix}")
        t.document.addRow(after: 1)
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, "\\begin{pmatrix}\n  a & b \\\\\n  c & x \\\\\n  & \\\\\n\\end{pmatrix}",
                       "a new last row takes the trailing break")
        t.document.removeRow(2)
        t.document.removeRow(1)
        XCTAssertEqual(t.document.render(indent: "", unit: "  ").text, "\\begin{pmatrix}\n  a & b \\\\\n\\end{pmatrix}")
        var bare = try XCTUnwrap(target("\\begin{pmatrix}\n  a & ‸b\n\\end{pmatrix}"))
        bare.document.setCell(0, 0, "y")
        XCTAssertEqual(bare.document.render(indent: "", unit: "  ").text, "\\begin{pmatrix}\n  y & b\n\\end{pmatrix}", "none added")
    }

    func testProvidersFollowTheSettings() {
        var s = T.Settings()
        s.enabled = true
        XCTAssertEqual(T.StructureProvider.enabled(s).map(\.name), ["matrix", "tabular", "cases", "align"])
        s.disabled = ["structure:tabular"]
        XCTAssertEqual(T.StructureProvider.enabled(s).map(\.name), ["matrix", "cases", "align"])
        s.structureEditor = false
        XCTAssertEqual(T.StructureProvider.enabled(s), [])
    }
}
