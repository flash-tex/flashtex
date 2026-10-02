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
        XCTAssertEqual(T.Grid.cell("\\multicolumn{2}{|c|}{\\textbf{x}}", spanning: 1), "\\textbf{x}", "one column: a plain cell")
        XCTAssertEqual(T.Grid.cell("x", spanning: 1), "x")
    }

    func testRemoveColumnNarrowsSpans() throws {
        var d = try spans()
        d.removeColumn(1)
        XCTAssertEqual(d.colspec?.text, "ccc")
        XCTAssertEqual(d.cells, [["AB", "C", "D"], ["a", "\\textbf{BC}", "d"], ["a", "\\multicolumn{2}{c}{CD}"], ["a", "c", "d"]],
                       "a span at the start or middle narrows (to a plain cell at one column); a plain cell goes")
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
