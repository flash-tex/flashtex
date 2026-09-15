import XCTest
@testable import FlashTeXProtocol

final class ProtocolTests: XCTestCase {
    static let fixtureDir: URL = {
        // Tests/FlashTeXProtocolTests/ProtocolTests.swift -> repo root
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { url.deleteLastPathComponent() }
        return url.appendingPathComponent("protocol/fixtures")
    }()

    func testDecodesCompileResultFixture() throws {
        let data = try Data(contentsOf: Self.fixtureDir.appendingPathComponent("compile-result.json"))
        let env = try RuntimeV1.decodeCompileResult(data)
        XCTAssertEqual(env.id, "fixture-compile-1")
        XCTAssertEqual(env.payload.status, .ok)
        XCTAssertEqual(env.payload.revision, 1)
        XCTAssertNil(env.payload.pdfPath)
        XCTAssertEqual(env.payload.pages.count, 1)
        guard case .text(let item) = env.payload.pages[0].items[0] else { return XCTFail("expected text item") }
        XCTAssertEqual(item.text, "Hello FlashTeX.")
        XCTAssertEqual(item.source, .init(path: "main.tex", startByte: 0, endByte: 14))
    }

    func testDecodesCompileRequestFixtureAndRangeMatchesText() throws {
        let req = try RuntimeV1.decodeCompileRequest(Data(contentsOf: Self.fixtureDir.appendingPathComponent("compile-request.json")))
        let res = try RuntimeV1.decodeCompileResult(Data(contentsOf: Self.fixtureDir.appendingPathComponent("compile-result.json")))
        let text = req.payload.documents[0].text
        guard case .text(let item) = res.payload.pages[0].items[0], let src = item.source else { return XCTFail() }
        let r = try XCTUnwrap(text.range(utf8Bytes: src))
        XCTAssertEqual(String(text[r]), "Hello FlashTeX")
    }

    func testRejectsWrongVersionAndType() {
        let bad = #"{"protocol_version":2,"id":"x","type":"compile_result","payload":{}}"#.data(using: .utf8)!
        XCTAssertThrowsError(try RuntimeV1.decodeCompileResult(bad))
        let wrongType = #"{"protocol_version":1,"id":"x","type":"compile","payload":{"project_id":"p","revision":1,"entry_path":"m","documents":[]}}"#.data(using: .utf8)!
        XCTAssertThrowsError(try RuntimeV1.decodeCompileResult(wrongType))
    }

    func testUnknownItemKindDoesNotFail() throws {
        let json = #"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[{"number":1,"width_pt":1,"height_pt":1,"items":[{"kind":"image"}]}],"diagnostics":[],"pdf_path":null}}"#
        let env = try RuntimeV1.decodeCompileResult(json.data(using: .utf8)!)
        XCTAssertEqual(env.payload.pages[0].items, [.unknown(kind: "image")])
    }

    func testUTF8OffsetsConvertToUTF16Selection() throws {
        // "é" is 2 UTF-8 bytes / 1 UTF-16 unit; "😀" is 4 UTF-8 bytes / 2 UTF-16 units.
        let text = "aé😀b"
        let ns = try XCTUnwrap(text.nsRange(utf8Bytes: .init(path: "m", startByte: 3, endByte: 8)))
        XCTAssertEqual(ns, NSRange(location: 2, length: 3))
        XCTAssertEqual((text as NSString).substring(with: ns), "😀b")
        // Inside the 4-byte scalar: rejected.
        XCTAssertNil(text.rangeOfUTF8(start: 4, end: 8))
        // Out of bounds / reversed: rejected.
        XCTAssertNil(text.rangeOfUTF8(start: 0, end: 99))
        XCTAssertNil(text.rangeOfUTF8(start: 5, end: 2))
        // Round trip.
        let back = try XCTUnwrap(text.utf8ByteRange(of: ns))
        XCTAssertEqual(back.start, 3); XCTAssertEqual(back.end, 8)
    }
}

final class RuleConventionTests: XCTestCase {
    private func item(_ text: String, size: Double = 8.4) -> RuntimeV1.PageItem.TextItem {
        let json = #"{"kind":"text","text":"\#(text)","x_pt":72,"baseline_y_pt":131.52,"font_size_pt":\#(size),"source":null}"#
        guard case .text(let t) = try! JSONDecoder().decode(RuntimeV1.PageItem.self, from: json.data(using: .utf8)!) else { fatalError() }
        return t
    }

    func testPureRuleRunsBecomeRectangles() throws {
        let r = try XCTUnwrap(RuleConvention.rect(for: item("──")))
        XCTAssertEqual(r.x, 72)
        XCTAssertEqual(r.width, 8.4, accuracy: 1e-9)          // 2 × 0.5 em × 8.4
        XCTAssertEqual(r.height, 0.0857 * 8.4, accuracy: 1e-9)
        XCTAssertEqual(r.y + r.height, 131.52, accuracy: 1e-9) // hugs the baseline from above
    }

    func testTextContainingOtherCharactersIsNotARule() {
        XCTAssertNil(RuleConvention.rect(for: item("─a")))
        XCTAssertNil(RuleConvention.rect(for: item("")))
        XCTAssertNil(RuleConvention.rect(for: item("-")))
    }
}

final class StructuredDiagnosticTests: XCTestCase {
    static let macFixtures: URL = {
        var url = URL(fileURLWithPath: #filePath)
        url.deleteLastPathComponent() // FlashTeXProtocolTests
        url.deleteLastPathComponent() // Tests
        return url.appendingPathComponent("FlashTeXMacTests/Fixtures")
    }()

    func testLegacyDiagnosticUnchangedWhenNewFieldsAbsent() throws {
        let json = #"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[],"diagnostics":[{"severity":"error","message":"m","source":{"path":"main.tex","start_byte":0,"end_byte":1},"recovery":"skipped"}],"pdf_path":null}}"#
        let env = try RuntimeV1.decodeCompileResult(Data(json.utf8))
        let d = try XCTUnwrap(env.payload.diagnostics.first)
        XCTAssertEqual(d.severity, .error)
        XCTAssertEqual(d.message, "m")
        XCTAssertEqual(d.source, .init(path: "main.tex", startByte: 0, endByte: 1))
        XCTAssertEqual(d.recovery, "skipped")
        XCTAssertNil(d.code)
        XCTAssertNil(d.suggestion)
        XCTAssertNil(d.labels)
        XCTAssertNil(d.notes)
        XCTAssertNil(d.help)
        let encoded = try JSONEncoder().encode(d)
        let obj = try XCTUnwrap(JSONSerialization.jsonObject(with: encoded) as? [String: Any])
        XCTAssertNil(obj["code"])
        XCTAssertNil(obj["suggestion"])
        XCTAssertNil(obj["labels"])
        XCTAssertNil(obj["notes"])
        XCTAssertNil(obj["help"])
        XCTAssertEqual(obj["severity"] as? String, "error")
        XCTAssertEqual(obj["message"] as? String, "m")
    }

    func testFixtureDecodesStructuredFieldsAndIgnoresUnknownKeys() throws {
        let data = try Data(contentsOf: Self.macFixtures.appendingPathComponent("structured-diagnostics.json"))
        let fast = try FastJSON.compileResultEnvelope(data)
        let reference = try RuntimeV1.decodeCompileResultReference(data)
        XCTAssertEqual(fast.payload, reference.payload)
        XCTAssertEqual(fast.payload.diagnostics.count, 2)
        let legacy = fast.payload.diagnostics[0]
        XCTAssertNil(legacy.code)
        XCTAssertNil(legacy.help)
        XCTAssertNil(legacy.labels)
        let d = fast.payload.diagnostics[1]
        XCTAssertEqual(d.code, "unsupported_feature")
        XCTAssertEqual(d.suggestion, "\\(...\\)")
        XCTAssertEqual(d.labels?.count, 2)
        XCTAssertEqual(d.labels?[0].primary, true)
        XCTAssertEqual(d.labels?[0].text, "this command")
        XCTAssertEqual(d.labels?[1].primary, false)
        XCTAssertEqual(d.labels?[1].text, "in this item")
        XCTAssertEqual(d.notes, ["\\tilde is a math accent; here it is outside math mode"])
        XCTAssertEqual(d.help?.message, "wrap it in math: \\(\\tilde{c}_t\\)")
        XCTAssertEqual(d.help?.replacement?.startByte, 10)
        XCTAssertEqual(d.help?.replacement?.endByte, 16)
        XCTAssertEqual(d.help?.replacement?.text, "\\(\\tilde{c}_t\\)")
        XCTAssertNil(d.help?.replacement?.path)
        XCTAssertEqual(d.path(of: try XCTUnwrap(d.help?.replacement)), "notes.tex")
        let encoded = try JSONEncoder().encode(d)
        let obj = try XCTUnwrap(JSONSerialization.jsonObject(with: encoded) as? [String: Any])
        XCTAssertEqual(obj["code"] as? String, "unsupported_feature")
        XCTAssertNotNil(obj["help"])
        XCTAssertNil(obj["future_extra"], "unknown keys are decode-only; they are not re-encoded")
    }

    func testHelpReplacementDecodesBareOrWithPath() throws {
        let bare = #"{"message":"h","replacement":{"start_byte":1,"end_byte":2,"text":"x"}}"#
        let withPath = #"{"message":"h","replacement":{"start_byte":1,"end_byte":2,"text":"x","path":"other.tex"}}"#
        let withSource = #"{"message":"h","replacement":{"start_byte":1,"end_byte":2,"text":"x","source":{"path":"src.tex","start_byte":9,"end_byte":10}}}"#
        let a = try JSONDecoder().decode(RuntimeV1.Diagnostic.Help.self, from: Data(bare.utf8))
        let b = try JSONDecoder().decode(RuntimeV1.Diagnostic.Help.self, from: Data(withPath.utf8))
        let c = try JSONDecoder().decode(RuntimeV1.Diagnostic.Help.self, from: Data(withSource.utf8))
        XCTAssertNil(a.replacement?.path)
        XCTAssertEqual(b.replacement?.path, "other.tex")
        XCTAssertEqual(c.replacement?.path, "src.tex", "nested source.path is accepted as the replacement path")
        let encoded = try JSONEncoder().encode(a)
        let obj = try XCTUnwrap(JSONSerialization.jsonObject(with: encoded) as? [String: Any])
        XCTAssertNil((obj["replacement"] as? [String: Any])?["path"])
        XCTAssertEqual((obj["replacement"] as? [String: Any])?["text"] as? String, "x")
    }

    /// The compiler nests the edit range inside `source` and emits no flat
    /// `start_byte`. Both decoders used to require the flat pair, so every
    /// diagnostic carrying a fix made the whole `compile_result` frame fail to
    /// decode -- and the Mac dropped the preview update as a protocol
    /// violation. These bytes are the shape `crates/compiler/src/diagnostics.rs`
    /// actually writes (its own unit tests assert this string).
    func testHelpReplacementAcceptsTheCompilersNestedSourceRange() throws {
        let emitted = #"{"message":"did you mean \\alpha?","replacement":{"source":{"end_byte":11,"path":"main.tex","start_byte":5},"text":"\\alpha"}}"#
        let help = try JSONDecoder().decode(RuntimeV1.Diagnostic.Help.self, from: Data(emitted.utf8))
        let r = try XCTUnwrap(help.replacement)
        XCTAssertEqual(r.startByte, 5)
        XCTAssertEqual(r.endByte, 11)
        XCTAssertEqual(r.text, "\\alpha")
        XCTAssertEqual(r.path, "main.tex")
    }

    /// A flat pair still wins over a nested one, so a producer that sends both
    /// is read the way it was before nested ranges were accepted.
    func testFlatReplacementOffsetsWinOverNestedOnes() throws {
        let both = #"{"message":"h","replacement":{"start_byte":1,"end_byte":2,"text":"x","source":{"path":"src.tex","start_byte":9,"end_byte":10}}}"#
        let help = try JSONDecoder().decode(RuntimeV1.Diagnostic.Help.self, from: Data(both.utf8))
        XCTAssertEqual(help.replacement?.startByte, 1)
        XCTAssertEqual(help.replacement?.endByte, 2)
    }

    /// Neither form present is still an error, and it names both spellings.
    func testReplacementWithNoRangeAtAllIsRejected() {
        let none = #"{"message":"h","replacement":{"text":"x"}}"#
        XCTAssertThrowsError(try JSONDecoder().decode(RuntimeV1.Diagnostic.Help.self, from: Data(none.utf8)))
    }
}
