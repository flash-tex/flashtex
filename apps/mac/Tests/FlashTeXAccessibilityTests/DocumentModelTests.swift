import XCTest
import FlashTeXProtocol
@testable import FlashTeXAccessibility

/// Reading sequence, labels, and offsets of `AccessibleDocumentModel` against
/// `Samples/multipage-{request,result}.json` and synthetic math lines.
final class DocumentModelTests: XCTestCase {
    static let samples = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("Samples")

    func loadMultipage() throws -> (result: RuntimeV1.CompileResult, text: String) {
        let result = try RuntimeV1.decodeCompileResult(Data(contentsOf: Self.samples.appendingPathComponent("multipage-result.json"))).payload
        let request = try RuntimeV1.decodeCompileRequest(Data(contentsOf: Self.samples.appendingPathComponent("multipage-request.json"))).payload
        return (result, request.documents[0].text)
    }

    func text(_ s: String, x: Double, y: Double, size: Double, source: (Int, Int)? = nil) -> RuntimeV1.PageItem {
        let json = """
        {"kind":"text","text":\(String(data: try! JSONEncoder().encode(s), encoding: .utf8)!),"x_pt":\(x),"baseline_y_pt":\(y),"font_size_pt":\(size)\
        \(source.map { ",\"source\":{\"path\":\"main.tex\",\"start_byte\":\($0.0),\"end_byte\":\($0.1)}" } ?? "")}
        """
        return try! JSONDecoder().decode(RuntimeV1.PageItem.self, from: Data(json.utf8))
    }

    func page(_ number: Int, _ items: [RuntimeV1.PageItem]) -> RuntimeV1.Page {
        var p = try! JSONDecoder().decode(RuntimeV1.Page.self, from: Data("{\"number\":\(number),\"width_pt\":612,\"height_pt\":792,\"items\":[]}".utf8))
        p.items = items
        return p
    }

    func result(_ pages: [RuntimeV1.Page], diagnostics: [RuntimeV1.Diagnostic] = [], status: RuntimeV1.Status = .ok) -> RuntimeV1.CompileResult {
        RuntimeV1.CompileResult(projectId: "t", revision: 1, status: status, pages: pages, diagnostics: diagnostics, pdfPath: nil)
    }

    // MARK: multipage sample

    func testReadingSequenceAcrossPagesAndLines() throws {
        let (res, text) = try loadMultipage()
        let model = AccessibleDocumentModel(result: res, documents: ["main.tex": text])
        XCTAssertEqual(model.pages.count, 2)
        XCTAssertEqual(model.lines.map(\.label), [
            "Page 1, line 1: Introduction",
            "Page 1, line 2: A naïve approach fails.",
            "Page 2, line 1: Method",
            "Page 2, line 2: Résumé of the steps.",
            "Page 2, line 3: oops",
        ])
        XCTAssertEqual(model.readingSequence.map(\.text),
                       ["Introduction", "A", "naïve", "approach", "fails.", "Method", "Résumé", "of the steps.", "oops"])
        XCTAssertEqual(model.readingSequence.map(\.role), Array(repeating: .text, count: 9))
        guard model.pages.count == 2 else { return XCTFail("expected two pages, got \(model.pages.count)") }
        XCTAssertEqual(model.pages[0].label, "Page 1 of 2, 2 lines")
        XCTAssertEqual(model.pages[1].label, "Page 2 of 2, 3 lines")
        XCTAssertEqual(model.summary, "Compile result: recovered, 2 pages, 9 items, 1 error, 1 warning")
        // Heading (17 pt) and body (12 pt) 34 pt apart stay separate lines.
        guard model.lines.count >= 2 else { return XCTFail("expected at least two lines, got \(model.lines.count)") }
        XCTAssertEqual(model.lines[0].fontSizePt, 17)
        XCTAssertEqual(model.lines[1].fontSizePt, 12)
    }

    func testUTF16RangesOnNonASCII() throws {
        let (res, text) = try loadMultipage()
        let model = AccessibleDocumentModel(result: res, documents: ["main.tex": text])
        let byText = Dictionary(uniqueKeysWithValues: model.readingSequence.map { ($0.text, $0) })
        // "naïve" is 6 UTF-8 bytes but 5 UTF-16 units; nothing before it is multi-byte.
        XCTAssertEqual(byText["naïve"]?.source, .init(path: "main.tex", startByte: 66, endByte: 72))
        XCTAssertEqual(byText["naïve"]?.utf16Range, NSRange(location: 66, length: 5))
        // After "naïve" (+1) the UTF-16 offsets lag the bytes by one; "Résumé" has two 2-byte scalars.
        XCTAssertEqual(byText["Résumé"]?.utf16Range, NSRange(location: 114, length: 6))
        XCTAssertEqual(byText["of the steps."]?.utf16Range, NSRange(location: 121, length: 13))
        XCTAssertEqual(byText["oops"]?.utf16Range, NSRange(location: 162, length: 4))
        // Every mapped range slices back to the item's text.
        for e in model.readingSequence {
            let r = try XCTUnwrap(Range(try XCTUnwrap(e.utf16Range), in: text))
            XCTAssertEqual(String(text[r]), e.text)
        }
        XCTAssertEqual(byText["naïve"]?.value, "12 point")
        XCTAssertEqual(byText["naïve"]?.actions, ["Go to source"])
        // Without document text no UTF-16 ranges exist, but the sequence is the same.
        let bare = AccessibleDocumentModel(result: res)
        XCTAssertEqual(bare.readingSequence.map(\.text), model.readingSequence.map(\.text))
        XCTAssertTrue(bare.readingSequence.allSatisfy { $0.utf16Range == nil })
        guard bare.readingSequence.count > 2 else { return XCTFail("expected more than two reading-sequence items, got \(bare.readingSequence.count)") }
        XCTAssertEqual(bare.readingSequence[2].value, "12 point", "no text supplied: no staleness claim")
    }

    func testRangesRebaseAcrossEditsOrDropOut() throws {
        let (res, compiled) = try loadMultipage()
        // Insert before everything: every range shifts by the inserted UTF-16 length.
        let shifted = "% é\n" + compiled
        let m1 = AccessibleDocumentModel(result: res, documents: ["main.tex": shifted], compiledDocuments: ["main.tex": compiled])
        let naive = try XCTUnwrap(m1.readingSequence.first { $0.text == "naïve" })
        XCTAssertEqual(naive.utf16Range, NSRange(location: 66 + 4, length: 5))
        let naiveUTF16Range = try XCTUnwrap(naive.utf16Range)
        let naiveRange = try XCTUnwrap(Range(naiveUTF16Range, in: shifted))
        XCTAssertEqual(String(shifted[naiveRange]), "naïve")
        // Edit inside "naïve": that element loses its range; later ones still map.
        let edited = compiled.replacingOccurrences(of: "naïve", with: "naive")
        let m2 = AccessibleDocumentModel(result: res, documents: ["main.tex": edited], compiledDocuments: ["main.tex": compiled])
        XCTAssertNil(m2.readingSequence.first { $0.text == "naïve" }?.utf16Range)
        let oops = try XCTUnwrap(m2.readingSequence.first { $0.text == "oops" })
        XCTAssertEqual(String(edited[Range(try XCTUnwrap(oops.utf16Range), in: edited)!]), "oops")
        XCTAssertEqual(m2.readingSequence.first { $0.text == "naïve" }?.value, "12 point, source not mapped to the current text")
    }

    func testDiagnosticsAreActionable() throws {
        let (res, text) = try loadMultipage()
        let model = AccessibleDocumentModel(result: res, documents: ["main.tex": text])
        XCTAssertEqual(model.diagnostics.count, 2)
        guard model.diagnostics.count == 2 else { return XCTFail("expected two diagnostics, got \(model.diagnostics.count)") }
        let err = model.diagnostics[0]
        XCTAssertEqual(err.label, "Error: Missing } inserted for \\textbf.")
        XCTAssertEqual(err.value, "recovery: Closed the group at end of paragraph and rendered its contents in bold.; in page 2 line 3")
        XCTAssertEqual(err.actions, ["Go to source"])
        XCTAssertEqual(err.announcement, "Error: Missing } inserted for \\textbf. — recovery: Closed the group at end of paragraph and rendered its contents in bold.; in page 2 line 3; Go to source")
        XCTAssertEqual(err.utf16Range, NSRange(location: 154, length: 12))
        XCTAssertEqual(err.lines, [.init(page: 2, line: 3)])
        let warn = model.diagnostics[1]
        XCTAssertEqual(warn.label, "Warning: Overfull \\hbox on page 2.")
        XCTAssertEqual(warn.value, "no provisional rendering; no source mapping")
        XCTAssertEqual(warn.actions, [])
        XCTAssertNil(warn.utf16Range)
        XCTAssertEqual(warn.announcement, "Warning: Overfull \\hbox on page 2. — no provisional rendering; no source mapping")
    }

    // MARK: math lines (compiler geometry: script 0.7×, raise 0.45 em, lower 0.2 em)

    func testScriptsFractionBarAndSecondOrderScriptsJoinTheirLine() {
        let items: [RuntimeV1.PageItem] = [
            text("x", x: 72, y: 100, size: 12, source: (0, 1)),
            text("2", x: 78, y: 100 - 0.45 * 12, size: 8.4, source: (2, 3)),        // superscript
            text("3", x: 84, y: 100 - 0.45 * 12 - 0.45 * 8.4, size: 6, source: (4, 5)), // second-order superscript
            text("+", x: 90, y: 100, size: 12),
            text("b", x: 102, y: 100 + 0.64 * 12, size: 8.4, source: (7, 8)),      // denominator (listed first on purpose)
            text("──", x: 100, y: 100 - 0.22 * 12, size: 8.4, source: (6, 9)),     // fraction bar, 8.4 pt wide
            text("a", x: 102, y: 100 - 0.58 * 12, size: 8.4, source: (6, 7)),      // numerator
            text("y", x: 114, y: 100, size: 12),
            text("i", x: 120, y: 100 + 0.2 * 12, size: 8.4),                       // subscript
        ]
        let model = AccessibleDocumentModel(result: result([page(1, items)]))
        XCTAssertEqual(model.lines.count, 1, "scripts and fraction parts must not become their own lines")
        guard let line = model.lines.first else { return XCTFail("expected one line") }
        XCTAssertEqual(line.summary, "x superscript 2 superscript 3 + numerator a fraction bar denominator b y subscript i")
        XCTAssertEqual(line.elements.map(\.role), [.text, .superscript, .superscript, .text, .numerator, .rule, .denominator, .text, .subscript])
        guard line.elements.count == 9 else { return XCTFail("expected nine elements, got \(line.elements.count)") }
        XCTAssertEqual(line.elements[1].label, "superscript 2")
        XCTAssertEqual(line.elements[1].value, "8.4 point, 70% of the 12 point line")
        XCTAssertEqual(line.elements[2].value, "6 point, 50% of the 12 point line")
        XCTAssertEqual(line.elements[5].label, "fraction bar")
        XCTAssertEqual(line.elements[5].value, "8.4 point, 2 segments")
        XCTAssertEqual(line.elements[8].label, "subscript i")
        XCTAssertEqual(line.elements[8].value, "8.4 point, 70% of the 12 point line, no source mapping")
        XCTAssertEqual(line.elements[8].actions, [])
        XCTAssertEqual(line.label, "Page 1, line 1: " + line.summary)
    }

    func testHeadingDirectlyAboveBodyIsNotAScript() {
        // 17 pt heading with a 12 pt body line 14.4 pt (one body leading) below:
        // size ratio 0.71 looks like a script, but the reach (0.85 em) does not.
        let items = [text("Title", x: 72, y: 96, size: 17), text("Body", x: 72, y: 96 + 14.4, size: 12)]
        let model = AccessibleDocumentModel(result: result([page(1, items)]))
        XCTAssertEqual(model.lines.map(\.summary), ["Title", "Body"])
        XCTAssertEqual(model.readingSequence.map(\.role), [.text, .text])
    }

    func testUnknownItemsAreSkippedAndPagesKeepTheirNumbers() throws {
        let unknown = try JSONDecoder().decode(RuntimeV1.PageItem.self, from: Data("{\"kind\":\"image\"}".utf8))
        let p3 = page(3, [unknown, text("late", x: 72, y: 100, size: 12)])
        let p7 = page(7, [text("later", x: 72, y: 100, size: 12)])
        let model = AccessibleDocumentModel(result: result([p3, p7]))
        XCTAssertEqual(model.lines.map(\.label), ["Page 3, line 1: late", "Page 7, line 1: later"])
        XCTAssertEqual(model.readingSequence.map(\.itemIndex), [1, 0])
        XCTAssertEqual(model.pages[0].label, "Page 3 of 2, 1 line")
        XCTAssertEqual(model.summary, "Compile result: ok, 2 pages, 2 items, 0 errors, 0 warnings")
    }

    func testDeterministicAndIndependentOfItemOrder() throws {
        let (res, text) = try loadMultipage()
        let a = AccessibleDocumentModel(result: res, documents: ["main.tex": text])
        let b = AccessibleDocumentModel(result: res, documents: ["main.tex": text])
        XCTAssertEqual(a, b)
        var shuffled = res
        for i in shuffled.pages.indices { shuffled.pages[i].items.reverse() }
        let c = AccessibleDocumentModel(result: shuffled, documents: ["main.tex": text])
        XCTAssertEqual(c.lines.map(\.label), a.lines.map(\.label))
        XCTAssertEqual(c.readingSequence.map(\.text), a.readingSequence.map(\.text))
        XCTAssertEqual(c.readingSequence.map(\.utf16Range), a.readingSequence.map(\.utf16Range))
    }
}
