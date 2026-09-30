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
        XCTAssertEqual(T.ColumnSpec("*{3}{c}|l")?.text, "ccc|l", "*{n}{…} expands to edit")
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
        XCTAssertEqual(out.text, "\\begin{bmatrix}\n    a & x &  \\\\\n    c & d &  \\\\\n    e &  & \n  \\end{bmatrix}")
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
