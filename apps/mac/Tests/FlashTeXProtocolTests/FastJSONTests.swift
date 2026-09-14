import XCTest
import Foundation
@testable import FlashTeXProtocol

/// The fast compile_result reader must produce exactly what JSONDecoder
/// produces for every valid frame, and must never accept a frame JSONDecoder
/// rejects (it falls back, so the reference error stands).
final class FastJSONTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("protocol/fixtures")

    /// Large synthetic result exercising every shape: text items with and
    /// without source/font, rules, unknown kinds, diagnostics with/without
    /// source and recovery, escapes, surrogate pairs, exponents, negatives.
    static func synthetic(pages: Int, itemsPerPage: Int) -> Data {
        var items: [String] = []
        for i in 0..<itemsPerPage {
            switch i % 6 {
            case 0: items.append(#"{"kind":"text","text":"naïve \#(i)","x_pt":\#(72.0 + Double(i) * 0.125),"baseline_y_pt":1.5e2,"font_size_pt":10,"source":{"path":"main.tex","start_byte":\#(i),"end_byte":\#(i + 4)}}"#)
            case 1: items.append(#"{"kind":"text","text":"bold \\\" quote \/ slash \#(i)","x_pt":-1.25,"baseline_y_pt":200.0,"font_size_pt":12.0,"source":null,"font":{"family":"Latin Modern Roman","weight":"bold","style":"italic"}}"#)
            case 2: items.append(#"{"kind":"rule","x_pt":10,"y_pt":20.5,"width_pt":100,"height_pt":0.4,"source":{"path":"main.tex","start_byte":1,"end_byte":2},"extra":[1,2,{"a":null}]}"#)
            case 3: items.append(#"{"kind":"glyph-run-v9","gid":[1,2,3],"source":{"path":"main.tex","start_byte":5,"end_byte":6}}"#)
            case 4: items.append(#"{"kind":"unknown-broken-source","source":{"path":"main.tex"}}"#)
            default: items.append(#"{"kind":"text","text":"😀 emoji \t tab","x_pt":1E1,"baseline_y_pt":3.0e-1,"font_size_pt":9.5}"#)
            }
        }
        let page = { (n: Int) in #"{"number":\#(n),"width_pt":612,"height_pt":792.0,"items":[\#(items.joined(separator: ","))]}"# }
        let pagesJSON = (1...pages).map(page).joined(separator: ",")
        let diags = #"[{"severity":"error","message":"Undefined control sequence \\foo","source":{"path":"main.tex","start_byte":3,"end_byte":7},"recovery":"skipped"},{"severity":"warning","message":"loose","source":null,"recovery":null},{"severity":"warning","message":"no source"},{"severity":"error","code":"unknown_command","message":"`\\foo` is unknown","source":{"path":"main.tex","start_byte":3,"end_byte":7},"suggestion":"\\alpha","labels":[{"source":{"path":"main.tex","start_byte":3,"end_byte":7},"text":"this command","primary":true},{"source":{"path":"main.tex","start_byte":0,"end_byte":1},"text":"here","primary":false}],"notes":["a note"],"help":{"message":"did you mean \\alpha","replacement":{"start_byte":3,"end_byte":7,"text":"\\alpha"}},"recovery":"skipped","extra_future":{"nested":true}}]"#
        let json = #"{"protocol_version":1,"id":"r-1","type":"compile_result","payload":{"project_id":"demo","revision":42,"status":"recovered","pages":[\#(pagesJSON)],"diagnostics":\#(diags),"pdf_path":null,"layout_capabilities":["rules-v1","font-hints-v1"]}}"#
        return Data(json.utf8)
    }

    func testFixtureDecodesIdentically() throws {
        let data = try Data(contentsOf: Self.fixtures.appendingPathComponent("compile-result.json"))
        let fast = try FastJSON.compileResultEnvelope(data)
        let reference = try RuntimeV1.decodeCompileResultReference(data)
        XCTAssertEqual(fast.payload, reference.payload)
        XCTAssertEqual(fast.id, reference.id); XCTAssertEqual(fast.type, reference.type)
        XCTAssertEqual(fast.protocolVersion, reference.protocolVersion)
    }

    func testSyntheticShapesDecodeIdenticallyAndRoundTrip() throws {
        let data = Self.synthetic(pages: 3, itemsPerPage: 24)
        let fast = try FastJSON.compileResultEnvelope(data)
        let reference = try RuntimeV1.decodeCompileResultReference(data)
        XCTAssertEqual(fast.payload, reference.payload)
        // Every shape present.
        let kinds = fast.payload.pages[0].items.map { item -> String in
            switch item { case .text: "text"; case .rule: "rule"; case .unknown(let k, _): k }
        }
        XCTAssertTrue(kinds.contains("rule") && kinds.contains("glyph-run-v9") && kinds.contains("unknown-broken-source"))
        if case .unknown(_, let src) = fast.payload.pages[0].items[4] { XCTAssertNil(src, "malformed source on an unknown kind decodes as nil, like try?") } else { XCTFail() }
        if case .text(let t) = fast.payload.pages[0].items[5] { XCTAssertEqual(t.text, "😀 emoji \t tab"); XCTAssertEqual(t.xPt, 10); XCTAssertEqual(t.baselineYPt, 0.3) } else { XCTFail() }
        // Re-encoded by JSONEncoder (different whitespace/escaping) → same values both ways.
        let encoded = try JSONEncoder().encode(reference)
        XCTAssertEqual(try FastJSON.compileResultEnvelope(encoded).payload, reference.payload)
        XCTAssertEqual(try RuntimeV1.decodeCompileResult(encoded).payload, reference.payload)
    }

    func testRejectionsMatchReferenceOrFallBack() throws {
        let cases: [(String, String)] = [
            ("truncated", #"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[],"diagnostics":[]}"#),
            ("trailing", #"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[],"diagnostics":[]}} x"#),
            ("bad status", #"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"meh","pages":[],"diagnostics":[]}}"#),
            ("text missing x", #"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[{"number":1,"width_pt":1,"height_pt":1,"items":[{"kind":"text","text":"a","baseline_y_pt":1,"font_size_pt":1}]}],"diagnostics":[]}}"#),
            ("rule zero width", #"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[{"number":1,"width_pt":1,"height_pt":1,"items":[{"kind":"rule","x_pt":0,"y_pt":0,"width_pt":0,"height_pt":1}]}],"diagnostics":[]}}"#),
            ("font family empty", #"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[{"number":1,"width_pt":1,"height_pt":1,"items":[{"kind":"text","text":"a","x_pt":0,"baseline_y_pt":1,"font_size_pt":1,"font":{"family":"","weight":"bold","style":"normal"}}]}],"diagnostics":[]}}"#),
            ("text source malformed", #"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[{"number":1,"width_pt":1,"height_pt":1,"items":[{"kind":"text","text":"a","x_pt":0,"baseline_y_pt":1,"font_size_pt":1,"source":{"path":"m"}}]}],"diagnostics":[]}}"#),
            ("revision float", #"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1.5,"status":"ok","pages":[],"diagnostics":[]}}"#),
            ("duplicate capability", #"{"protocol_version":1,"id":"x","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[],"diagnostics":[],"layout_capabilities":["a","a"]}}"#),
            ("lone surrogate", #"{"protocol_version":1,"id":"\ud800","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[],"diagnostics":[]}}"#),
            ("control char", "{\"protocol_version\":1,\"id\":\"a\u{01}b\",\"type\":\"compile_result\",\"payload\":{\"project_id\":\"p\",\"revision\":1,\"status\":\"ok\",\"pages\":[],\"diagnostics\":[]}}"),
        ]
        for (name, json) in cases {
            let data = Data(json.utf8)
            let fast = try? FastJSON.compileResultEnvelope(data)
            let reference = try? RuntimeV1.decodeCompileResultReference(data)
            if let reference {
                XCTAssertEqual(fast?.payload, reference.payload, "\(name): the reference accepts it, so the fast path must agree")
            } else {
                XCTAssertNil(fast, "\(name): the fast path must not accept what the reference rejects")
                XCTAssertThrowsError(try RuntimeV1.decodeCompileResult(data), name)
            }
        }
    }

    func testGenericParseAndRawRanges() throws {
        let inner = String(decoding: Self.synthetic(pages: 1, itemsPerPage: 6), as: UTF8.self)
        let frame = Data(#"{"protocol_version":1,"session_id":"s","id":null,"type":"update","payload":{"kind":"preview","compile_revision":7,"source_versions":{"main.tex":3},"missing_layout_capabilities":[],"controller_total_ms":12.5,"request_id":"preview-7","result":\#(inner)}}"#.utf8)
        let v = try FastJSON.parse(frame, rawKeys: ["result"])
        let payload = try XCTUnwrap(v.object?["payload"]?.object)
        XCTAssertEqual(payload["compile_revision"]?.int, 7)
        XCTAssertEqual(payload["source_versions"]?.object?["main.tex"]?.int, 3)
        XCTAssertEqual(payload["controller_total_ms"]?.double, 12.5)
        XCTAssertTrue(v.object?["id"]?.isNull == true)
        guard case .raw(let range)? = payload["result"] else { return XCTFail("result kept raw") }
        let env = try FastJSON.compileResultEnvelope(frame, range: range)
        XCTAssertEqual(env.payload, try RuntimeV1.decodeCompileResultReference(Data(inner.utf8)).payload)
    }

    func testLargeResultIsFasterThanJSONDecoder() throws {
        let data = Self.synthetic(pages: 24, itemsPerPage: 450) // ≈ 10.8k items, like a 60 KB document
        let reference = try RuntimeV1.decodeCompileResultReference(data)
        XCTAssertEqual(try FastJSON.compileResultEnvelope(data).payload, reference.payload)
        func time(_ f: () throws -> Void) rethrows -> Double {
            var best = Double.infinity
            for _ in 0..<3 { let t = Date(); try f(); best = min(best, Date().timeIntervalSince(t) * 1000) }
            return best
        }
        let fast = try time { _ = try FastJSON.compileResultEnvelope(data) }
        let slow = try time { _ = try RuntimeV1.decodeCompileResultReference(data) }
        print("FASTJSON \(data.count) bytes, \(reference.payload.pages.reduce(0) { $0 + $1.items.count }) items: fast \(String(format: "%.1f", fast)) ms, JSONDecoder \(String(format: "%.1f", slow)) ms")
        // Only an optimized build is a fair comparison (Foundation is always
        // optimized; a debug FastJSON bounds-checks every byte): measured
        // release 16.8 ms vs 63.7 ms for 10.8k items.
        if !_isDebugAssertConfiguration() {
            XCTAssertLessThan(fast, slow, "the fast path must not be slower than JSONDecoder")
        }
    }
}
