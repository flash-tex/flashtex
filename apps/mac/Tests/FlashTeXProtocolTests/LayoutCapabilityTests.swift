import XCTest
@testable import FlashTeXProtocol

/// runtime-v1-layout-capabilities.md: request/result capability fields, typed
/// rules, font hints, and the per-request negotiation checks.
final class LayoutCapabilityTests: XCTestCase {
    private func resultJSON(items: String, caps: String? = nil) -> Data {
        let capsField = caps.map { #","layout_capabilities":\#($0)"# } ?? ""
        return Data(#"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[{"number":1,"width_pt":612,"height_pt":792,"items":[\#(items)]}],"diagnostics":[],"pdf_path":null\#(capsField)}}"#.utf8)
    }

    private func decodeResult(items: String, caps: String? = nil) throws -> RuntimeV1.CompileResult {
        try RuntimeV1.decodeCompileResult(resultJSON(items: items, caps: caps)).payload
    }

    static let rule = #"{"kind":"rule","x_pt":72,"y_pt":84,"width_pt":24,"height_pt":0.5,"source":{"path":"main.tex","start_byte":0,"end_byte":11}}"#
    static let hintedText = #"{"kind":"text","text":"x","x_pt":72,"baseline_y_pt":84,"font_size_pt":10,"source":null,"font":{"family":"Latin Modern Roman","weight":"normal","style":"italic"}}"#

    // MARK: request field

    func testRequestOmitsCapabilitiesWhenNilAndEncodesSnakeCaseList() throws {
        let plain = RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "m", documents: [])
        let plainLine = String(decoding: try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "a", plain)), as: UTF8.self)
        XCTAssertFalse(plainLine.contains("layout_capabilities"), plainLine)

        let extended = RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "m", documents: [],
                                                layoutCapabilities: ["rules-v1", "font-hints-v1"])
        let line = try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "b", extended))
        XCTAssertTrue(String(decoding: line, as: UTF8.self).contains(#""layout_capabilities":["rules-v1","font-hints-v1"]"#))
        let back = try RuntimeV1.decodeCompileRequest(line)
        XCTAssertEqual(back.payload, extended)
        XCTAssertEqual(back.payload.layoutCapabilities, ["rules-v1", "font-hints-v1"])
        // An explicitly empty list is legal and round-trips as empty (= none).
        let empty = RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "m", documents: [], layoutCapabilities: [])
        XCTAssertEqual(try RuntimeV1.decodeCompileRequest(RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "c", empty))).payload.layoutCapabilities, [])
    }

    func testCapabilityListValidationRejectsTooManyDuplicateEmptyAndOversized() throws {
        func encode(_ caps: [String]) throws -> Data {
            try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "v", .init(projectId: "p", revision: 1, entryPath: "m", documents: [], layoutCapabilities: caps)))
        }
        XCTAssertNoThrow(try encode((0..<16).map { "cap-\($0)" }))
        XCTAssertThrowsError(try encode((0..<17).map { "cap-\($0)" })) { XCTAssertEqual($0 as? RuntimeV1.DecodeError, .invalidLayoutCapabilities("17 capabilities exceed the limit of 16")) }
        XCTAssertThrowsError(try encode(["rules-v1", "rules-v1"])) { XCTAssertEqual($0 as? RuntimeV1.DecodeError, .invalidLayoutCapabilities("duplicate capability rules-v1")) }
        XCTAssertThrowsError(try encode([""])) { XCTAssertEqual($0 as? RuntimeV1.DecodeError, .invalidLayoutCapabilities("empty capability string")) }
        XCTAssertNoThrow(try encode([String(repeating: "é", count: 32)])) // 64 bytes exactly
        XCTAssertThrowsError(try encode([String(repeating: "é", count: 33)])) // 66 bytes
        // The same limits apply to the accepted set in a result.
        XCTAssertThrowsError(try decodeResult(items: "", caps: #"["a","a"]"#))
        XCTAssertThrowsError(try decodeResult(items: "", caps: #"[""]"#))
        XCTAssertNoThrow(try decodeResult(items: "", caps: #"["rules-v1"]"#))
    }

    func testResultAcceptedSetDecodesAndRoundTrips() throws {
        let result = try decodeResult(items: Self.rule, caps: #"["rules-v1"]"#)
        XCTAssertEqual(result.layoutCapabilities, ["rules-v1"])
        let env = RuntimeV1.Envelope(protocolVersion: 1, id: "r", type: "compile_result", payload: result)
        let line = try RuntimeV1.encodeLine(env)
        XCTAssertTrue(String(decoding: line, as: UTF8.self).contains(#""layout_capabilities":["rules-v1"]"#))
        XCTAssertEqual(try RuntimeV1.decodeCompileResult(line).payload, result)
        // Absent field means none and stays absent on re-encode (legacy fixtures unchanged).
        let legacy = try decodeResult(items: "")
        XCTAssertNil(legacy.layoutCapabilities)
        let legacyLine = try RuntimeV1.encodeLine(RuntimeV1.Envelope(protocolVersion: 1, id: "l", type: "compile_result", payload: legacy))
        XCTAssertFalse(String(decoding: legacyLine, as: UTF8.self).contains("layout_capabilities"))
    }

    // MARK: typed rules

    func testTypedRuleDecodesAsTopLeftRectangleAndRoundTrips() throws {
        let result = try decodeResult(items: Self.rule, caps: #"["rules-v1"]"#)
        guard case .rule(let r) = result.pages[0].items[0] else { return XCTFail("expected .rule, got \(result.pages[0].items[0])") }
        XCTAssertEqual(r.xPt, 72); XCTAssertEqual(r.yPt, 84)
        XCTAssertEqual(r.widthPt, 24); XCTAssertEqual(r.heightPt, 0.5)
        XCTAssertEqual(r.source, .init(path: "main.tex", startByte: 0, endByte: 11))
        let line = try RuntimeV1.encodeLine(RuntimeV1.Envelope(protocolVersion: 1, id: "r", type: "compile_result", payload: result))
        let s = String(decoding: line, as: UTF8.self)
        XCTAssertTrue(s.contains(#""kind":"rule""#) && s.contains(#""y_pt":84"#) && s.contains(#""height_pt":0.5"#), s)
        XCTAssertEqual(try RuntimeV1.decodeCompileResult(line).payload, result)
    }

    func testTypedRuleRejectsNonPositiveNonFiniteAndOversizedGeometry() {
        func rule(_ fields: String) -> String { #"{"kind":"rule",\#(fields),"source":null}"# }
        XCTAssertThrowsError(try decodeResult(items: rule(#""x_pt":0,"y_pt":0,"width_pt":0,"height_pt":1"#)), "zero width")
        XCTAssertThrowsError(try decodeResult(items: rule(#""x_pt":0,"y_pt":0,"width_pt":1,"height_pt":-1"#)), "negative height")
        XCTAssertThrowsError(try decodeResult(items: rule(#""x_pt":0,"y_pt":0,"width_pt":1000001,"height_pt":1"#)), "over 1e6")
        XCTAssertThrowsError(try decodeResult(items: rule(#""x_pt":-1000001,"y_pt":0,"width_pt":1,"height_pt":1"#)), "magnitude over 1e6")
        XCTAssertThrowsError(try decodeResult(items: rule(#""x_pt":0,"y_pt":0,"width_pt":1e400,"height_pt":1"#)), "non-finite")
        XCTAssertThrowsError(try decodeResult(items: rule(#""x_pt":0,"y_pt":0,"width_pt":1"#)), "missing height")
        // Malformed rules fail the whole decode: they are never downgraded to `.unknown`.
        XCTAssertNoThrow(try decodeResult(items: rule(#""x_pt":-5,"y_pt":1000000,"width_pt":1000000,"height_pt":0.001"#)))
    }

    // MARK: font hints

    func testFontHintDecodesAndValidatesFamilyWeightStyle() throws {
        let result = try decodeResult(items: Self.hintedText, caps: #"["font-hints-v1"]"#)
        guard case .text(let t) = result.pages[0].items[0] else { return XCTFail() }
        XCTAssertEqual(t.font, .init(family: "Latin Modern Roman", weight: .normal, style: .italic))
        let line = try RuntimeV1.encodeLine(RuntimeV1.Envelope(protocolVersion: 1, id: "f", type: "compile_result", payload: result))
        XCTAssertTrue(String(decoding: line, as: UTF8.self).contains(#""font":{"family":"Latin Modern Roman","weight":"normal","style":"italic"}"#) ||
                      String(decoding: line, as: UTF8.self).contains(#""family":"Latin Modern Roman""#))
        XCTAssertEqual(try RuntimeV1.decodeCompileResult(line).payload, result)
        // Absent hint stays absent (no "font":null on the wire).
        let plain = try decodeResult(items: #"{"kind":"text","text":"x","x_pt":1,"baseline_y_pt":1,"font_size_pt":10,"source":null}"#)
        guard case .text(let p) = plain.pages[0].items[0] else { return XCTFail() }
        XCTAssertNil(p.font)
        XCTAssertFalse(String(decoding: try RuntimeV1.encodeLine(RuntimeV1.Envelope(protocolVersion: 1, id: "p", type: "compile_result", payload: plain)), as: UTF8.self).contains(#""font":"#))

        func text(font: String) -> String { #"{"kind":"text","text":"x","x_pt":1,"baseline_y_pt":1,"font_size_pt":10,"source":null,"font":\#(font)}"# }
        XCTAssertThrowsError(try decodeResult(items: text(font: #"{"family":"","weight":"normal","style":"normal"}"#)), "empty family")
        XCTAssertThrowsError(try decodeResult(items: text(font: #"{"family":"Bad\u0007Family","weight":"normal","style":"normal"}"#)), "control character")
        XCTAssertThrowsError(try decodeResult(items: text(font: #"{"family":"Tab\tHere","weight":"normal","style":"normal"}"#)), "tab is a control character")
        XCTAssertThrowsError(try decodeResult(items: text(font: #"{"family":"\#(String(repeating: "é", count: 65))","weight":"normal","style":"normal"}"#)), "130 bytes")
        XCTAssertNoThrow(try decodeResult(items: text(font: #"{"family":"\#(String(repeating: "é", count: 64))","weight":"bold","style":"italic"}"#)), "128 bytes")
        XCTAssertThrowsError(try decodeResult(items: text(font: #"{"family":"X","weight":"heavy","style":"normal"}"#)), "unknown weight")
        XCTAssertThrowsError(try decodeResult(items: text(font: #"{"family":"X","weight":"normal","style":"oblique"}"#)), "unknown style")
        XCTAssertThrowsError(try decodeResult(items: text(font: #"{"family":"X","weight":"normal"}"#)), "missing style")
    }

    // MARK: unknown kinds

    func testUnknownKindKeepsItsSourceAndRoundTrips() throws {
        let result = try decodeResult(items: #"{"kind":"blob","source":{"path":"main.tex","start_byte":3,"end_byte":9},"extra":1},{"kind":"image"}"#)
        XCTAssertEqual(result.pages[0].items, [.unknown(kind: "blob", source: .init(path: "main.tex", startByte: 3, endByte: 9)),
                                               .unknown(kind: "image")])
        let line = try RuntimeV1.encodeLine(RuntimeV1.Envelope(protocolVersion: 1, id: "u", type: "compile_result", payload: result))
        XCTAssertEqual(try RuntimeV1.decodeCompileResult(line).payload.pages[0].items, result.pages[0].items)
    }

    // MARK: negotiation

    func testViolationDetectsUnrequestedAcceptanceAndUnnegotiatedShapes() throws {
        // Accepted ⊆ requested, shapes match: no violation.
        let good = try decodeResult(items: "\(Self.rule),\(Self.hintedText)", caps: #"["rules-v1","font-hints-v1"]"#)
        XCTAssertNil(LayoutNegotiation.violation(in: good, requested: ["rules-v1", "font-hints-v1"]))
        XCTAssertNil(LayoutNegotiation.violation(in: good, requested: ["font-hints-v1", "rules-v1", "future-v9"]))
        // Accepted something never requested.
        XCTAssertEqual(LayoutNegotiation.violation(in: good, requested: ["rules-v1"]),
                       "accepted capability font-hints-v1 was not requested (requested: rules-v1)")
        XCTAssertEqual(LayoutNegotiation.violation(in: try decodeResult(items: "", caps: #"["rules-v1"]"#), requested: []),
                       "accepted capability rules-v1 was not requested (requested: none)")
        // A rule without accepted rules-v1 (request did not ask; or asked but the producer did not accept).
        let ruleOnly = try decodeResult(items: Self.rule)
        XCTAssertEqual(LayoutNegotiation.violation(in: ruleOnly, requested: []),
                       "page 1 contains a rule item but rules-v1 was not accepted (accepted: none)")
        XCTAssertNotNil(LayoutNegotiation.violation(in: ruleOnly, requested: ["rules-v1"]), "asked but not accepted still may not carry rules")
        XCTAssertNil(LayoutNegotiation.violation(in: try decodeResult(items: Self.rule, caps: #"["rules-v1"]"#), requested: ["rules-v1"]))
        // A font hint without accepted font-hints-v1.
        let hintOnly = try decodeResult(items: Self.hintedText, caps: #"["rules-v1"]"#)
        XCTAssertEqual(LayoutNegotiation.violation(in: hintOnly, requested: ["rules-v1", "font-hints-v1"]),
                       "page 1 text item carries a font hint but font-hints-v1 was not accepted (accepted: rules-v1)")
        // Legacy result on a legacy request: fine, and unknown kinds are not violations.
        XCTAssertNil(LayoutNegotiation.violation(in: try decodeResult(items: #"{"kind":"image"}"#), requested: []))
    }

    func testUnsupportedPrimitiveDiagnosticsOnlyOnTheNegotiatedRoute() throws {
        let result = try decodeResult(items: #"{"kind":"blob","source":{"path":"main.tex","start_byte":3,"end_byte":9}},{"kind":"image"}"#, caps: #"["rules-v1"]"#)
        let negotiated = LayoutNegotiation(requested: ["rules-v1"], accepted: ["rules-v1"])
        let diags = LayoutNegotiation.unsupportedPrimitiveDiagnostics(in: result, negotiation: negotiated)
        XCTAssertEqual(diags.count, 2)
        guard diags.count == 2 else { return XCTFail("expected two diagnostics, got \(diags.count)") }
        XCTAssertEqual(diags[0].severity, .error)
        XCTAssertEqual(diags[0].message, "unsupported layout primitive 'blob' at main.tex bytes 3..<9 on page 1")
        XCTAssertEqual(diags[0].source, .init(path: "main.tex", startByte: 3, endByte: 9))
        XCTAssertNotNil(diags[0].recovery)
        XCTAssertEqual(diags[1].message, "unsupported layout primitive 'image' (no source mapping) on page 1")
        XCTAssertNil(diags[1].source)
        // Legacy route: silently skipped, as before.
        XCTAssertEqual(LayoutNegotiation.unsupportedPrimitiveDiagnostics(in: result, negotiation: .legacy), [])
        XCTAssertEqual(LayoutNegotiation.unsupportedPrimitiveDiagnostics(in: result, negotiation: .init(requested: ["rules-v1"], accepted: [])), [],
                       "requested but not accepted is the legacy route")
        // Missing is explicit, in request order.
        XCTAssertEqual(LayoutNegotiation(requested: ["rules-v1", "font-hints-v1"], accepted: ["font-hints-v1"]).missing, ["rules-v1"])
        XCTAssertTrue(LayoutNegotiation.legacy.missing.isEmpty)
    }
}
