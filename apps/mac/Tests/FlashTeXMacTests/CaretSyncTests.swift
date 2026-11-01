import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXMac

/// Source → preview sync against `Samples/multipage-{request,result}.json`.
final class CaretSyncTests: XCTestCase {
    static let samples = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("Samples")
    static let requestURL = samples.appendingPathComponent("multipage-request.json")
    static let resultURL = samples.appendingPathComponent("multipage-result.json")

    private func load() throws -> (text: String, result: RuntimeV1.CompileResult) {
        let req = try RuntimeV1.decodeCompileRequest(Data(contentsOf: Self.requestURL))
        let res = try RuntimeV1.decodeCompileResult(Data(contentsOf: Self.resultURL))
        XCTAssertEqual(req.id, res.id)
        XCTAssertEqual(res.id, "sample-multipage-1")
        XCTAssertEqual(req.payload.revision, res.payload.revision)
        let doc = try XCTUnwrap(req.payload.documents.first { $0.path == req.payload.entryPath })
        return (doc.text, res.payload)
    }

    /// (a) Every item's source range slices the request text to exactly its `text`.
    func testEveryItemSourceSlicesToItsText() throws {
        let (text, result) = try load()
        XCTAssertEqual(result.pages.count, 2)
        XCTAssertEqual(result.diagnostics.count, 2)
        XCTAssertNotEqual(text.utf8.count, text.count, "sample must contain non-ASCII text")
        var checked = 0
        for page in result.pages {
            for item in page.items {
                guard case .text(let t) = item else { XCTFail("unexpected item kind"); continue }
                let source = try XCTUnwrap(t.source, "\(t.text) has no source")
                XCTAssertEqual(source.path, "main.tex")
                let range = try XCTUnwrap(text.range(utf8Bytes: source), "invalid range for \(t.text)")
                XCTAssertEqual(String(text[range]), t.text)
                checked += 1
            }
        }
        XCTAssertEqual(checked, 9)

        // Diagnostics: one error with a source range and recovery, one warning without.
        let error = try XCTUnwrap(result.diagnostics.first { $0.severity == .error })
        let errRange = try XCTUnwrap(text.range(utf8Bytes: try XCTUnwrap(error.source)))
        XCTAssertEqual(String(text[errRange]), "\\textbf{oops")
        XCTAssertNotNil(error.recovery)
        let warning = try XCTUnwrap(result.diagnostics.first { $0.severity == .warning })
        XCTAssertNil(warning.source)
        XCTAssertNil(warning.recovery)
    }

    /// (b) Byte inside the non-ASCII word hits it; whitespace between items hits nothing.
    func testItemsContainingFindsNonASCIIWordAndIgnoresGaps() throws {
        let (text, result) = try load()
        let naive = try XCTUnwrap(text.utf8ByteRange(of: (text as NSString).range(of: "naïve")))
        XCTAssertEqual(naive.start, 66)
        XCTAssertEqual(naive.end, 72) // "ï" is 2 bytes
        // Second byte of "ï" (not a scalar boundary) is still inside the item.
        let hits = CaretSync.itemsContaining(byte: naive.start + 3, path: "main.tex", in: result)
        XCTAssertEqual(hits.count, 1)
        XCTAssertEqual(hits.first?.page, 1)
        XCTAssertEqual(hits.first?.index, 2)
        guard case .text(let t) = result.pages[0].items[2] else { return XCTFail() }
        XCTAssertEqual(t.text, "naïve")

        // Boundaries: start is inclusive, end is exclusive.
        XCTAssertEqual(CaretSync.itemsContaining(byte: naive.start, path: "main.tex", in: result).first?.index, 2)
        XCTAssertTrue(CaretSync.itemsContaining(byte: naive.end, path: "main.tex", in: result).isEmpty) // the space
        XCTAssertEqual(CaretSync.itemsContaining(byte: naive.end + 1, path: "main.tex", in: result).first?.index, 3) // "approach"
        // Whitespace between "Introduction}" and "A" maps to nothing; wrong path maps to nothing.
        XCTAssertTrue(CaretSync.itemsContaining(byte: 63, path: "main.tex", in: result).isEmpty)
        XCTAssertTrue(CaretSync.itemsContaining(byte: naive.start, path: "other.tex", in: result).isEmpty)

        // Page 2: "Résumé".
        let resume = try XCTUnwrap(text.utf8ByteRange(of: (text as NSString).range(of: "Résumé")))
        let p2 = CaretSync.itemsContaining(byte: resume.start + 1, path: "main.tex", in: result)
        XCTAssertEqual(p2.first?.page, 2)
        XCTAssertEqual(p2.first?.index, 1)
        XCTAssertEqual(CaretSync.indicesByPage(byte: resume.start, path: "main.tex", in: result), [2: [1]])
    }

    /// Empty source ranges match only an exactly equal byte.
    func testEmptyRangeMatchesOnlyExactByte() {
        let item = RuntimeV1.PageItem.text(.init(text: "", xPt: 0, baselineYPt: 0, fontSizePt: 10,
                                                  source: .init(path: "main.tex", startByte: 5, endByte: 5)))
        let result = RuntimeV1.CompileResult(
            projectId: "demo", revision: 1, status: .ok,
            pages: [.init(number: 1, widthPt: 10, heightPt: 10, items: [item, .unknown(kind: "image")])],
            diagnostics: [], pdfPath: nil)
        XCTAssertEqual(CaretSync.itemsContaining(byte: 5, path: "main.tex", in: result).map(\.index), [0])
        XCTAssertTrue(CaretSync.itemsContaining(byte: 4, path: "main.tex", in: result).isEmpty)
        XCTAssertTrue(CaretSync.itemsContaining(byte: 6, path: "main.tex", in: result).isEmpty)
    }
}

/// (c) ShellModel: the UTF-16 caret maps to a UTF-8 byte and then to preview items.
@MainActor
final class ShellModelCaretSyncTests: XCTestCase {
    func testCaretMapsToByteAndPreviewItems() throws {
        let model = ShellModel()
        // Pass no request: the `<name>-request.json` sibling must be discovered.
        model.loadFixtures(request: nil, result: CaretSyncTests.resultURL)
        XCTAssertNil(model.loadError, model.loadError ?? "")
        XCTAssertEqual(model.resultID, "sample-multipage-1")
        XCTAssertEqual(model.activePath, "main.tex")
        XCTAssertTrue(model.activeText.contains("naïve"))
        XCTAssertFalse(model.previewIsStale)

        // Caret on the "v" of "naïve": UTF-16 offset 69, UTF-8 byte 70 (ï is 2 bytes).
        let ns = (model.activeText as NSString).range(of: "naïve")
        XCTAssertEqual(ns.location, 66)
        model.caretUTF16 = ns.location + 3
        XCTAssertEqual(model.caretByte, 70)
        XCTAssertEqual(model.caretItems, [1: [2]])
        XCTAssertEqual(CaretSync.itemsContaining(byte: 70, path: "main.tex", in: model.result!).first?.index, 2)

        // After the non-ASCII word, UTF-16 and UTF-8 offsets diverge by one.
        let resume = (model.activeText as NSString).range(of: "Résumé")
        XCTAssertEqual(resume.location, 114)
        model.caretUTF16 = resume.location
        XCTAssertEqual(model.caretByte, 115)
        XCTAssertEqual(model.caretItems, [2: [1]])

        // Gap and out-of-range carets highlight nothing.
        model.caretUTF16 = 63
        XCTAssertEqual(model.caretItems, [:])
        model.caretUTF16 = (model.activeText as NSString).length + 10
        XCTAssertNil(model.caretByte)
        XCTAssertEqual(model.caretItems, [:])

        // A different active document never matches items sourced from main.tex.
        model.caretUTF16 = 70
        model.activePath = "other.tex"
        XCTAssertEqual(model.caretItems, [:])
    }

    func testSiblingRequestURLDerivation() {
        let dir = URL(fileURLWithPath: "/tmp/x")
        XCTAssertEqual(ShellModel.siblingRequestURL(forResult: dir.appendingPathComponent("multipage-result.json")),
                       dir.appendingPathComponent("multipage-request.json"))
        XCTAssertEqual(ShellModel.siblingRequestURL(forResult: dir.appendingPathComponent("compile-result.json")),
                       dir.appendingPathComponent("compile-request.json"))
        XCTAssertNil(ShellModel.siblingRequestURL(forResult: dir.appendingPathComponent("something.json")))
    }
}

// MARK: - sorted index

final class CaretSyncIndexTests: XCTestCase {
    private func result(_ spans: [(page: Int, start: Int, end: Int, path: String)]) -> RuntimeV1.CompileResult {
        var pages: [Int: [RuntimeV1.PageItem]] = [:]
        for s in spans {
            pages[s.page, default: []].append(.text(.init(text: "t", xPt: 0, baselineYPt: 0, fontSizePt: 10,
                                                          source: .init(path: s.path, startByte: s.start, endByte: s.end))))
        }
        return .init(projectId: "p", revision: 1, status: .ok,
                     pages: pages.keys.sorted().map { .init(number: $0, widthPt: 10, heightPt: 10, items: pages[$0]!) },
                     diagnostics: [], pdfPath: nil)
    }

    private func same(_ index: CaretSync.Index, _ result: RuntimeV1.CompileResult, byte: Int, path: String,
                      file: StaticString = #filePath, line: UInt = #line) {
        let linear = CaretSync.itemsContaining(byte: byte, path: path, in: result).map { [$0.page, $0.index] }
        let fast = index.itemsContaining(byte: byte).map { [$0.page, $0.index] }
        XCTAssertEqual(fast, linear, "byte \(byte)", file: file, line: line)
        XCTAssertEqual(index.indicesByPage(byte: byte), CaretSync.indicesByPage(byte: byte, path: path, in: result), file: file, line: line)
    }

    /// Nested, overlapping, empty, duplicated (hyphenated across pages) and
    /// foreign-path spans: the index answers exactly like the linear scan at
    /// every byte, in document order.
    func testIndexMatchesLinearScanOnOverlappingAndCrossPageSpans() {
        let r = result([
            (1, 0, 10, "a.tex"), (1, 2, 4, "a.tex"), (1, 4, 4, "a.tex"), (1, 3, 8, "a.tex"), (1, 20, 30, "a.tex"),
            (2, 20, 30, "a.tex"),   // the same word continued on the next page
            (2, 30, 35, "a.tex"), (2, 0, 100, "b.tex"), (3, 40, 45, "a.tex"), (3, 44, 44, "a.tex"),
        ])
        let index = CaretSync.Index(result: r, path: "a.tex")
        XCTAssertEqual(index.count, 9)
        for byte in -1...50 { same(index, r, byte: byte, path: "a.tex") }
        XCTAssertEqual(index.indicesByPage(byte: 25), [1: [4], 2: [0]], "a span repeated on two pages reports both")
        XCTAssertEqual(index.itemsContaining(byte: 4).map(\.index), [0, 2, 3], "empty span at 4 plus the two covering it, in item order")
        XCTAssertEqual(index.itemsContaining(byte: 44).map { [$0.page, $0.index] }, [[3, 0], [3, 1]])
        XCTAssertTrue(index.itemsContaining(byte: 10).isEmpty, "end is exclusive")
        XCTAssertTrue(CaretSync.Index(result: r, path: "c.tex").itemsContaining(byte: 0).isEmpty)
        XCTAssertEqual(CaretSync.Index(result: r, path: "b.tex").itemsContaining(byte: 99).map(\.page), [2])
    }

    /// Random spans, seeded: every byte agrees with the linear scan.
    func testIndexMatchesLinearScanOnRandomSpans() {
        var g = SplitMix(seed: 0x5EED)
        for round in 0..<20 {
            var spans: [(page: Int, start: Int, end: Int, path: String)] = []
            for _ in 0..<(round * 7 + 1) {
                let start = Int(g.next() % 200)
                let len = round % 3 == 0 ? Int(g.next() % 40) : Int(g.next() % 4)
                spans.append((Int(g.next() % 3) + 1, start, start + len, g.next() % 5 == 0 ? "other.tex" : "a.tex"))
            }
            let r = result(spans)
            let index = CaretSync.Index(result: r, path: "a.tex")
            for byte in stride(from: -2, through: 245, by: 1) { same(index, r, byte: byte, path: "a.tex") }
        }
    }

    /// The multipage sample and the two-document fixture through the model:
    /// `exactCaretItems` (index) equals `caretItems` (scan) at every UTF-16 caret.
    @MainActor
    func testModelExactCaretItemsEqualScanAtEveryCaret() throws {
        let model = ShellModel()
        model.loadFixtures(request: nil, result: CaretSyncTests.resultURL)
        for path in ["main.tex", "other.tex"] {
            model.activePath = path
            let length = (model.activeText as NSString).length
            for caret in 0...(length + 1) {
                model.caretUTF16 = caret
                XCTAssertEqual(model.exactCaretItems, model.caretItems, "\(path) caret \(caret)")
            }
        }
        let multi = try MultiFileFixture.loadedModel()
        for path in ["main.tex", "chapter.tex"] {
            multi.activePath = path
            let text = multi.activeText as NSString
            for caret in 0...text.length {
                multi.caretUTF16 = caret
                XCTAssertEqual(multi.exactCaretItems, multi.caretItems, "\(path) caret \(caret)")
            }
        }
        // Inside the ZWJ family the highlight stays on the one item; the ligature glyph likewise.
        multi.activePath = "main.tex"
        let main = MultiFileFixture.main as NSString
        let family = main.range(of: "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}")
        for caret in family.location..<NSMaxRange(family) { multi.caretUTF16 = caret; XCTAssertEqual(multi.exactCaretItems, [1: [9]], "caret \(caret)") }
        multi.caretUTF16 = NSMaxRange(family)
        XCTAssertEqual(multi.exactCaretItems, [:], "the space after the emoji")
        multi.caretUTF16 = main.range(of: "\u{FB01}le").location
        XCTAssertEqual(multi.exactCaretItems, [1: [4]])
        multi.caretUTF16 = main.range(of: "CRLF").location - 1 // between "\r" and "\n"
        XCTAssertEqual(multi.exactCaretItems, [:])
    }

    func testCaretByteRoundsSurrogateHalvesAndRefusesOutOfRange() {
        let text = MultiFileFixture.main
        let ns = text as NSString
        let smile = ns.range(of: "\u{1F600}")
        XCTAssertEqual(CaretSync.byteOffset(ofCaretUTF16: smile.location, in: text), 93)
        XCTAssertEqual(CaretSync.byteOffset(ofCaretUTF16: smile.location + 1, in: text), 93, "between the surrogates: start of 😀")
        XCTAssertEqual(CaretSync.byteOffset(ofCaretUTF16: smile.location + 2, in: text), 97)
        // Between "e" and U+0301 is a real scalar boundary: kept exactly.
        let e = ns.range(of: "e\u{301}")
        XCTAssertEqual(CaretSync.byteOffset(ofCaretUTF16: e.location + 1, in: text), 84)
        XCTAssertEqual(CaretSync.byteOffset(ofCaretUTF16: 0, in: text), 0)
        XCTAssertEqual(CaretSync.byteOffset(ofCaretUTF16: ns.length, in: text), text.utf8.count)
        XCTAssertNil(CaretSync.byteOffset(ofCaretUTF16: ns.length + 1, in: text))
        XCTAssertNil(CaretSync.byteOffset(ofCaretUTF16: -1, in: text))
        XCTAssertEqual(CaretSync.byteOffset(ofCaretUTF16: 0, in: ""), 0)
    }

    /// 10 000 disjoint items over 200 pages: one caret move through the index
    /// is a binary search (O(log n)); the scan is O(n). Both are timed over
    /// the same 20 000 caret positions and the ratio is asserted loosely so
    /// the test is stable on a loaded machine; the absolute numbers are
    /// printed for the report.
    func testTenThousandItemsQueryIsLogarithmic() {
        var spans: [(page: Int, start: Int, end: Int, path: String)] = []
        var byte = 0
        for i in 0..<10_000 {
            let len = 3 + (i % 7)
            spans.append((i / 50 + 1, byte, byte + len, "main.tex"))
            byte += len + 1
        }
        let r = result(spans)
        let built = Date()
        let index = CaretSync.Index(result: r, path: "main.tex")
        let buildMs = Date().timeIntervalSince(built) * 1000
        XCTAssertEqual(index.count, 10_000)
        let carets = (0..<20_000).map { $0 * 3 % byte }
        var checksum = 0
        let t0 = Date()
        for c in carets { checksum += index.itemsContaining(byte: c).count }
        let indexed = Date().timeIntervalSince(t0)
        var scanChecksum = 0
        let t1 = Date()
        for c in carets.prefix(200) { scanChecksum += CaretSync.itemsContaining(byte: c, path: "main.tex", in: r).count }
        let scanned = Date().timeIntervalSince(t1) * 100 // extrapolate 200 → 20 000
        for c in carets.prefix(200) { XCTAssertEqual(index.itemsContaining(byte: c).count, CaretSync.itemsContaining(byte: c, path: "main.tex", in: r).count) }
        XCTAssertGreaterThan(checksum, 0)
        XCTAssertGreaterThan(scanChecksum, 0)
        let perQueryUs = indexed / Double(carets.count) * 1e6
        print(String(format: "CaretSync.Index 10k items: build %.2f ms; %.3f µs per caret query (20 000 queries); linear scan %.1f µs per query (extrapolated from 200)",
                     buildMs, perQueryUs, scanned / Double(carets.count) * 1e6))
        XCTAssertLessThan(indexed * 20, scanned, "index must be at least 20× faster than the scan for 10k items")
    }

    /// Repeated queries through the model rebuild the index only when the
    /// result or the active document changes.
    @MainActor
    func testModelCachesIndexPerResultAndDocument() throws {
        let model = try MultiFileFixture.loadedModel()
        let a = try XCTUnwrap(model.caretIndex())
        let b = try XCTUnwrap(model.caretIndex())
        XCTAssertEqual(a.entries, b.entries)
        XCTAssertEqual(a.path, "main.tex")
        model.activePath = "chapter.tex"
        XCTAssertEqual(model.caretIndex()?.path, "chapter.tex")
        XCTAssertEqual(model.caretIndex()?.count, 6)
        model.activePath = "main.tex"
        XCTAssertEqual(model.caretIndex()?.count, 17)
        // A new result (different id) is indexed afresh.
        model.result = RuntimeV1.CompileResult(projectId: "demo", revision: 2, status: .ok, pages: [], diagnostics: [], pdfPath: nil)
        model.resultID = "mf-2"
        XCTAssertEqual(model.caretIndex()?.count, 0)
        model.result = nil
        XCTAssertNil(model.caretIndex())
        XCTAssertEqual(model.exactCaretItems, [:])
    }
}

/// Small deterministic generator for the random-span test.
private struct SplitMix {
    var state: UInt64
    init(seed: UInt64) { state = seed }
    mutating func next() -> UInt64 {
        state &+= 0x9E3779B97F4A7C15
        var z = state
        z = (z ^ (z >> 30)) &* 0xBF58476D1CE4E5B9
        z = (z ^ (z >> 27)) &* 0x94D049BB133111EB
        return z ^ (z >> 31)
    }
}
