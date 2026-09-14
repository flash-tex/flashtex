import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Decoding and fail-closed validation of the experimental rendering-v2
/// display list, against real `flashtex-render --v2` output (fixtures
/// generated from origin/agent/mac-render-pipeline/unified 7094ef7 with the
/// repository's bundled Latin Modern as the only font directory).
final class RenderingV2Tests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static func fixture(_ name: String) throws -> Data { try Data(contentsOf: fixtures.appendingPathComponent(name)) }

    /// A minimal valid envelope as a mutable JSON object for negative cases.
    static func minimal() -> [String: Any] {
        [
            "protocol_version": 2, "id": "r1", "type": "display_list",
            "payload": [
                "render_format": "display-list-v2", "coordinate_unit": "bp_2pow20", "color_space": "srgb", "text_extraction": "cluster-actualtext",
                "project_id": "p", "revision": 1, "required_features": ["glyph_run", "rule", "rgba-srgb", "cluster-actualtext"],
                "documents": [["path": "main.tex", "revision": 1, "sha256": String(repeating: "0", count: 64), "byte_length": 3]],
                "fonts": [["font_id": "f", "sha256": String(repeating: "1", count: 64), "byte_length": 10, "format": "static-truetype",
                           "face_index": 0, "units_per_em": 1000, "glyph_count": 300, "postscript_name": "Demo"]],
                "pages": [[
                    "number": 1, "width": 641728512, "height": 830472192,
                    "items": [
                        ["kind": "glyph_run", "font_id": "f", "font_size": 12582912, "text": "fi",
                         "glyphs": [["gid": 125, "origin_x": 1048576, "baseline_y": 2097152, "advance_x": 5808301, "advance_y": 0, "cluster": 0]],
                         "clusters": [["text_start_byte": 0, "text_end_byte": 2, "hit_rects": [["x": 1048576, "top": 0, "width": 5808301, "height": 2097152]],
                                       "carets": [["text_byte": 0, "x": 1048576, "top": 0, "height": 2097152]],
                                       "sources": [["path": "main.tex", "start_byte": 0, "end_byte": 2]]]],
                         "paint": ["r": 0, "g": 0, "b": 0, "a": 1]],
                        ["kind": "rule", "x": 0, "top": 3145728, "width": 1048576, "height": 524288, "paint": ["r": 0, "g": 0, "b": 0, "a": 1],
                         "synthetic_reason": "fraction bar"],
                    ],
                ]],
                "diagnostics": [],
            ],
        ]
    }

    static func data(_ object: [String: Any]) -> Data { try! JSONSerialization.data(withJSONObject: object) }

    /// Applies `edit` to the minimal envelope and returns the validation error code.
    static func code(_ edit: (inout [String: Any]) -> Void) -> String? {
        var o = minimal(); edit(&o)
        do { _ = try RenderingV2.decode(data(o)); return nil } catch let e as RenderingV2.ValidationError { return e.code } catch { return "other:\(error)" }
    }

    static func setPayload(_ o: inout [String: Any], _ key: String, _ value: Any?) {
        var p = o["payload"] as! [String: Any]; p[key] = value; o["payload"] = p
    }
    static func setItem(_ o: inout [String: Any], _ index: Int, _ edit: (inout [String: Any]) -> Void) {
        var p = o["payload"] as! [String: Any]; var pages = p["pages"] as! [[String: Any]]; var items = pages[0]["items"] as! [[String: Any]]
        edit(&items[index]); pages[0]["items"] = items; p["pages"] = pages; o["payload"] = p
    }
    static func setFont(_ o: inout [String: Any], _ edit: (inout [String: Any]) -> Void) {
        var p = o["payload"] as! [String: Any]; var fonts = p["fonts"] as! [[String: Any]]; edit(&fonts[0]); p["fonts"] = fonts; o["payload"] = p
    }
    static func setCluster(_ o: inout [String: Any], _ edit: (inout [String: Any]) -> Void) {
        setItem(&o, 0) { run in var cs = run["clusters"] as! [[String: Any]]; edit(&cs[0]); run["clusters"] = cs }
    }

    // MARK: decode of real pipeline output

    func testDecodesRealPipelineDisplayListAndConsumesItsFields() throws {
        let env = try RenderingV2.decode(try Self.fixture("display-list-v2-text.json"))
        XCTAssertEqual(env.protocolVersion, 2)
        XCTAssertEqual(env.type, "display_list")
        XCTAssertEqual(env.id, "req-1")
        let list = env.payload
        XCTAssertEqual(list.renderFormat, "display-list-v2")
        XCTAssertEqual(list.coordinateUnit, "bp_2pow20")
        XCTAssertEqual(list.colorSpace, "srgb")
        XCTAssertEqual(list.textExtraction, "cluster-actualtext")
        XCTAssertEqual(list.projectId, "demo")
        XCTAssertEqual(list.revision, 3)
        XCTAssertEqual(list.requiredFeatures, ["glyph_run", "rgba-srgb", "cluster-actualtext"])
        XCTAssertEqual(list.documents.map(\.path), ["main.tex"])
        XCTAssertEqual(list.documents[0].byteLength, 161)
        // Pipeline deviation from the schema: CFF Latin Modern is `opentype-cff`, not `static-truetype`.
        XCTAssertEqual(Set(list.fonts.map(\.format)), ["opentype-cff"])
        XCTAssertEqual(list.fonts.count, 4)
        XCTAssertTrue(list.fonts.allSatisfy { $0.fontId == $0.sha256 && $0.faceIndex == 0 && $0.unitsPerEm == 1000 && $0.glyphCount == 821 })
        XCTAssertEqual(list.pages.count, 1)
        XCTAssertEqual(list.pages[0].widthPt, 612, accuracy: 0.0001)
        XCTAssertEqual(list.pages[0].heightPt, 792, accuracy: 0.0001)
        XCTAssertEqual(list.pages[0].items.count, 15)
        // "office": the ffi ligature is one glyph (original GID 123) over three source bytes.
        guard case .glyphRun(let office) = list.pages[0].items[4] else { return XCTFail("item 4 is not a glyph run") }
        XCTAssertEqual(office.text, "office")
        XCTAssertEqual(office.glyphs.map(\.gid), [81, 123, 43, 50])
        XCTAssertEqual(office.clusters[1].textStartByte, 1)
        XCTAssertEqual(office.clusters[1].textEndByte, 4)
        XCTAssertEqual(office.clusterText(1), "ffi")
        XCTAssertEqual(office.clusters[1].sources, [RenderingV2.SourceRange(path: "main.tex", startByte: 75, endByte: 78)])
        XCTAssertEqual(office.clusters[1].hitRects.count, 1)
        XCTAssertEqual(office.clusters[1].carets.count, 1)
        XCTAssertEqual(office.clusters[1].carets[0].textByte, 1)
        XCTAssertEqual(office.paint, .black)
        // "café.": é is two text bytes from three source bytes (\'e).
        guard case .glyphRun(let cafe) = list.pages[0].items[14] else { return XCTFail() }
        XCTAssertEqual(cafe.text, "café.")
        XCTAssertEqual(cafe.clusterText(3), "é")
        XCTAssertEqual(cafe.clusters[3].sources?[0].endByte, 144)
        XCTAssertEqual(cafe.clusters[3].sources?[0].startByte, 141)
        XCTAssertEqual(list.diagnostics, [])
        // The document digest is the SHA-256 of the fixture's .tex bytes.
        let tex = try Self.fixture("display-list-v2-text.tex")
        XCTAssertEqual(list.documents[0].sha256, SourceDigest.sha256Hex(String(decoding: tex, as: UTF8.self)))
    }

    func testDecodesRealPipelineRulesAndMathFontManifest() throws {
        let env = try RenderingV2.decode(try Self.fixture("display-list-v2-math.json"))
        let list = env.payload
        XCTAssertEqual(list.requiredFeatures, ["glyph_run", "rule", "rgba-srgb", "cluster-actualtext"])
        let rules = list.pages[0].items.compactMap { if case .rule(let r) = $0 { return r } else { return nil } }
        XCTAssertEqual(rules.count, 1)
        XCTAssertEqual(rules[0].sources, [RenderingV2.SourceRange(path: "main.tex", startByte: 67, endByte: 80)])
        XCTAssertGreaterThan(rules[0].height, 0)
        XCTAssertEqual(RenderingV2.points(rules[0].height), 0.3985, accuracy: 0.0005, "fraction rule thickness is 0.4 TeX pt")
        XCTAssertTrue(list.fonts.contains { $0.postscriptName == "LatinModernMath-Regular" && $0.glyphCount == 4802 })
        // Round trip through the model keeps every consumed field.
        let again = try RenderingV2.decode(try JSONEncoder().encode(env))
        XCTAssertEqual(again, env)
    }

    func testMathFixtureFailsClosedWithoutTheMathFontBundled() throws {
        // display-list-v2-missing-font.json is the math fixture with the math font's
        // content hash replaced by one no bundled font has (latinmodern-math.otf is
        // vendored now): the WHOLE frame is refused with the missing hash, never
        // partially drawn.
        let env = try RenderingV2.decode(try Self.fixture("display-list-v2-missing-font.json"))
        let math = try XCTUnwrap(env.payload.fonts.first { $0.postscriptName == "LatinModernMath-Regular" })
        XCTAssertThrowsError(try V2Frame.prepare(env, store: PreviewV2Tests.store)) { error in
            let e = error as? RenderingV2.ValidationError
            XCTAssertEqual(e?.code, "font_resource_unavailable")
            XCTAssertTrue(e?.message.contains("font resource \(math.sha256)") == true, e?.message ?? "")
            XCTAssertTrue(e?.message.contains("unavailable") == true)
        }
    }

    func testRealTextFixturePreparesAgainstBundledFontsByRawByteHash() throws {
        let env = try RenderingV2.decode(try Self.fixture("display-list-v2-text.json"))
        let frame = try V2Frame.prepare(env, store: PreviewV2Tests.store)
        XCTAssertEqual(frame.fonts.count, 4)
        for (_, f) in frame.fonts {
            XCTAssertEqual(f.resource.sha256, f.file.bytesSha256, "flashtex-render names fonts by the raw byte SHA-256 (D3)")
            XCTAssertEqual(f.cgFont.postScriptName as String?, f.resource.postscriptName)
        }
        XCTAssertEqual(Set(frame.fonts.values.map { $0.file.url.lastPathComponent }),
                       ["lmroman10-regular.otf", "lmroman10-italic.otf", "lmroman12-bold.otf", "lmroman10-bold.otf"])
        let image = try XCTUnwrap(GlyphRunRenderer.rasterize(page: env.payload.pages[0], frame: frame, scale: 1))
        XCTAssertEqual(image.width, 612)
        XCTAssertGreaterThan(PreviewV2Tests.inkPixels(image), 500)
    }

    // MARK: fail closed

    func testMinimalEnvelopeIsValid() throws {
        let env = try RenderingV2.decode(Self.data(Self.minimal()))
        XCTAssertEqual(env.payload.pages[0].items.count, 2)
        if case .rule(let r) = env.payload.pages[0].items[1] { XCTAssertEqual(r.syntheticReason, "fraction bar") } else { XCTFail() }
    }

    func testUnknownProtocolVersionAndTypeAreRefused() {
        XCTAssertEqual(Self.code { $0["protocol_version"] = 1 }, "unsupported_protocol_version")
        XCTAssertEqual(Self.code { $0["protocol_version"] = 3 }, "unsupported_protocol_version")
        XCTAssertEqual(Self.code { $0["protocol_version"] = nil }, "missing_protocol_version")
        XCTAssertEqual(Self.code { $0["type"] = "compile_result" }, "unsupported_message_type")
        XCTAssertEqual(Self.code { $0["type"] = "render_format_rejected" }, "unsupported_message_type")
        XCTAssertEqual(Self.code { $0["type"] = nil }, "missing_type")
        XCTAssertEqual(Self.code { $0["payload"] = nil }, "malformed_payload")
        XCTAssertThrowsError(try RenderingV2.decode(Data("not json".utf8))) { XCTAssertEqual(($0 as? RenderingV2.ValidationError)?.code, "malformed_json") }
        // A runtime-v1 compile_result is never mistaken for a display list.
        let v1 = try! Data(contentsOf: URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("protocol/fixtures/compile-result.json"))
        XCTAssertThrowsError(try RenderingV2.decode(v1)) { XCTAssertEqual(($0 as? RenderingV2.ValidationError)?.code, "unsupported_protocol_version") }
    }

    func testUnknownItemKindIsRefusedNotSkipped() {
        XCTAssertEqual(Self.code { Self.setItem(&$0, 1) { $0["kind"] = "shading" } }, "unknown_item_kind") // `image` is a known kind since display-list-v2-images (V2ImageTests)
        XCTAssertEqual(Self.code { Self.setItem(&$0, 1) { $0["kind"] = nil } }, "malformed_payload")
    }

    func testEnvelopeConstantsAndFeaturesAreChecked() {
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "render_format", "runtime-v1") }, "unsupported_format")
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "coordinate_unit", "pt") }, "unsupported_coordinate_unit")
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "color_space", "cmyk") }, "unsupported_color_space")
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "text_extraction", "tounicode") }, "unsupported_text_extraction")
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "required_features", ["glyph_run", "transparency-group"]) }, "unsupported_feature")
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "required_features", []) }, "invalid_display_list")
        // Used features must be declared (rendering-core "undeclared rendering feature").
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "required_features", ["glyph_run", "rgba-srgb", "cluster-actualtext"]) }, "invalid_display_list", "a rule is painted but 'rule' is undeclared")
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "required_features", ["rule", "rgba-srgb", "cluster-actualtext"]) }, "invalid_display_list", "a glyph run is painted but 'glyph_run' is undeclared")
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "required_features", ["glyph_run", "rule"]) }, "invalid_display_list", "rgba-srgb and cluster-actualtext are always used")
        XCTAssertNil(Self.code { Self.setPayload(&$0, "required_features", ["glyph_run", "rule", "static-truetype", "rgba-srgb", "cluster-actualtext"]) }, "declaring more than is used is fine")
        XCTAssertNil(Self.code { o in
            // No rule on the page → 'rule' need not be declared.
            Self.setPayload(&o, "required_features", ["glyph_run", "rgba-srgb", "cluster-actualtext"])
            var p = o["payload"] as! [String: Any]; var pages = p["pages"] as! [[String: Any]]; var items = pages[0]["items"] as! [[String: Any]]
            items.removeLast(); pages[0]["items"] = items; p["pages"] = pages; o["payload"] = p
        })
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "documents", []) }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "revision", -1) }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "revision", 1.5) }, "malformed_payload", "floating-point geometry/ids are rejected")
    }

    func testDocumentPathsAndByteLengthsAreChecked() {
        func setDoc(_ o: inout [String: Any], _ edit: (inout [String: Any]) -> Void) {
            var p = o["payload"] as! [String: Any]; var docs = p["documents"] as! [[String: Any]]; edit(&docs[0]); p["documents"] = docs; o["payload"] = p
        }
        // Source ranges name main.tex; a renamed document makes them undeclared,
        // so these cases rename the range too and probe only the path rule.
        func withPath(_ path: String) -> String? {
            Self.code { o in
                setDoc(&o) { $0["path"] = path }
                Self.setCluster(&o) { $0["sources"] = [["path": path, "start_byte": 0, "end_byte": 2]] }
            }
        }
        XCTAssertNil(withPath("chapters/one.tex"))
        XCTAssertEqual(withPath("/abs.tex"), "invalid_resource")
        XCTAssertEqual(withPath("a/../b.tex"), "invalid_resource")
        XCTAssertEqual(withPath("./b.tex"), "invalid_resource")
        XCTAssertEqual(withPath("a//b.tex"), "invalid_resource")
        XCTAssertEqual(withPath("a\\b.tex"), "invalid_resource")
        XCTAssertEqual(withPath("c:b.tex"), "invalid_resource")
        XCTAssertEqual(withPath(""), "invalid_resource")
        XCTAssertEqual(Self.code { setDoc(&$0) { $0["byte_length"] = 8_388_609 } }, "invalid_resource", "source over 8 MiB")
        XCTAssertEqual(Self.code { setDoc(&$0) { $0["byte_length"] = -1 } }, "invalid_resource")
        XCTAssertEqual(Self.code { setDoc(&$0) { $0["revision"] = -1 } }, "invalid_resource")
        XCTAssertEqual(Self.code { o in
            var p = o["payload"] as! [String: Any]; var docs = p["documents"] as! [[String: Any]]; docs.append(docs[0]); p["documents"] = docs; o["payload"] = p
        }, "invalid_resource", "duplicate path")
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "diagnostics", [["code": "x", "message": "m", "severity": "warning", "sources": [["path": "main.tex", "start_byte": 0, "end_byte": 9]]]]) }, "invalid_display_list", "diagnostic source beyond the document")
        XCTAssertNil(Self.code { Self.setPayload(&$0, "diagnostics", [["code": "x", "message": "m", "severity": "warning", "sources": [["path": "main.tex", "start_byte": 0, "end_byte": 3]]]]) })
    }

    func testFontManifestIsChecked() {
        XCTAssertEqual(Self.code { Self.setFont(&$0) { $0["format"] = "type1" } }, "unsupported_feature")
        XCTAssertEqual(Self.code { Self.setFont(&$0) { $0["face_index"] = 1 } }, "unsupported_feature")
        XCTAssertEqual(Self.code { Self.setFont(&$0) { $0["sha256"] = "ABC" } }, "invalid_resource")
        XCTAssertEqual(Self.code { Self.setFont(&$0) { $0["byte_length"] = 0 } }, "invalid_resource")
        XCTAssertEqual(Self.code { Self.setFont(&$0) { $0["glyph_count"] = 1 } }, "invalid_resource")
        XCTAssertEqual(Self.code { Self.setFont(&$0) { $0["units_per_em"] = 8 } }, "invalid_resource")
        XCTAssertEqual(Self.code { Self.setFont(&$0) { $0["font_id"] = "other" } }, "invalid_resource", "run references an undeclared font")
        XCTAssertNil(Self.code { Self.setFont(&$0) { $0["format"] = "opentype-cff" } }, "pipeline CFF format is accepted")
        XCTAssertNil(Self.code { Self.setFont(&$0) { $0["format"] = "core14-afm"; $0["byte_length"] = 0 } }, "metrics-only manifest decodes; painting it fails later")
    }

    func testGlyphAndClusterGeometryIsChecked() {
        XCTAssertEqual(Self.code { Self.setItem(&$0, 0) { $0["glyphs"] = [["gid": 0, "origin_x": 0, "baseline_y": 0, "advance_x": 0, "advance_y": 0, "cluster": 0]] } }, "invalid_display_list", "gid 0 is the missing-glyph diagnostic")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 0) { $0["glyphs"] = [["gid": 300, "origin_x": 0, "baseline_y": 0, "advance_x": 0, "advance_y": 0, "cluster": 0]] } }, "invalid_display_list", "gid beyond glyph_count")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 0) { $0["glyphs"] = [["gid": 5, "origin_x": 0, "baseline_y": 0, "advance_x": 0, "advance_y": 0, "cluster": 7]] } }, "invalid_display_list", "cluster index out of range")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 0) { $0["glyphs"] = [] } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 0) { $0["text"] = "" } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 0) { $0["font_size"] = 0 } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["text_end_byte"] = 1 } }, "invalid_display_list", "clusters must cover the whole run text")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["text_end_byte"] = 3 } }, "invalid_display_list", "cluster beyond the text")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["hit_rects"] = [] } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["carets"] = [["text_byte": 5, "x": 0, "top": 0, "height": 1]] } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["carets"] = [["text_byte": 0, "x": 0, "top": 0, "height": 0]] } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["sources"] = nil } }, "invalid_display_list", "no provenance")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["synthetic_reason"] = "x" } }, "invalid_display_list", "both provenance kinds")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["sources"] = [["path": "other.tex", "start_byte": 0, "end_byte": 1]] } }, "invalid_display_list", "undeclared document")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["sources"] = [["path": "main.tex", "start_byte": 2, "end_byte": 1]] } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["sources"] = [["path": "main.tex", "start_byte": 2, "end_byte": 4]] } }, "invalid_display_list", "source range beyond the document's declared byte_length (3)")
        XCTAssertNil(Self.code { Self.setCluster(&$0) { $0["sources"] = [["path": "main.tex", "start_byte": 3, "end_byte": 3]] } }, "an empty range at the end of the document is allowed")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["sources"] = [] } }, "invalid_display_list", "sources must not be empty")
        // Every cluster needs at least one glyph (rendering-core "cluster without glyph mapping").
        XCTAssertEqual(Self.code {
            Self.setItem(&$0, 0) { run in
                run["clusters"] = [["text_start_byte": 0, "text_end_byte": 1, "hit_rects": [["x": 0, "top": 0, "width": 1, "height": 1]], "carets": [], "sources": [["path": "main.tex", "start_byte": 0, "end_byte": 1]]],
                                   ["text_start_byte": 1, "text_end_byte": 2, "hit_rects": [["x": 0, "top": 0, "width": 1, "height": 1]], "carets": [], "sources": [["path": "main.tex", "start_byte": 1, "end_byte": 2]]]]
            }
        }, "invalid_display_list", "two clusters, one glyph")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["text_end_byte"] = 0 } }, "invalid_display_list", "empty cluster")
        // Exact-integer range: a tick beyond ±(2^53−1), or a sum that leaves it, is refused.
        let huge = Int64(1) << 53
        XCTAssertEqual(Self.code { Self.setItem(&$0, 0) { $0["glyphs"] = [["gid": 5, "origin_x": huge, "baseline_y": 0, "advance_x": 0, "advance_y": 0, "cluster": 0]] } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 0) { $0["glyphs"] = [["gid": 5, "origin_x": huge - 1, "baseline_y": 0, "advance_x": 1, "advance_y": 0, "cluster": 0]] } }, "invalid_display_list", "origin + advance overflows the exact range")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["hit_rects"] = [["x": 0, "top": 0, "width": -1, "height": 1]] } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["hit_rects"] = Array(repeating: ["x": 0, "top": 0, "width": 1, "height": 1], count: 129) } }, "invalid_display_list", "hit rect count bound")
        XCTAssertEqual(Self.code { Self.setCluster(&$0) { $0["carets"] = Array(repeating: ["text_byte": 0, "x": 0, "top": 0, "height": 1], count: 129) } }, "invalid_display_list", "caret count bound")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 0) { $0["paint"] = ["r": 0, "g": 0, "b": 0, "a": 2] } }, "invalid_display_list")
        // UTF-8 boundary inside a multi-byte scalar.
        XCTAssertEqual(Self.code {
            Self.setItem(&$0, 0) { run in
                run["text"] = "é"
                run["clusters"] = [["text_start_byte": 0, "text_end_byte": 1, "hit_rects": [["x": 0, "top": 0, "width": 1, "height": 1]], "carets": [], "sources": [["path": "main.tex", "start_byte": 0, "end_byte": 1]]],
                                   ["text_start_byte": 1, "text_end_byte": 2, "hit_rects": [["x": 0, "top": 0, "width": 1, "height": 1]], "carets": [], "sources": [["path": "main.tex", "start_byte": 1, "end_byte": 2]]]]
            }
        }, "invalid_display_list")
    }

    func testRuleAndPageGeometryIsChecked() {
        XCTAssertEqual(Self.code { Self.setItem(&$0, 1) { $0["width"] = 0 } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 1) { $0["height"] = -1 } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 1) { $0["synthetic_reason"] = nil } }, "invalid_display_list")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 1) { $0["synthetic_reason"] = nil; $0["sources"] = [["path": "main.tex", "start_byte": 0, "end_byte": 3]] } }, nil)
        XCTAssertEqual(Self.code { Self.setItem(&$0, 1) { $0["synthetic_reason"] = nil; $0["sources"] = [["path": "main.tex", "start_byte": 0, "end_byte": 4]] } }, "invalid_display_list", "rule source beyond the document")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 1) { $0["synthetic_reason"] = "" } }, "invalid_display_list", "empty synthetic reason")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 1) { $0["x"] = Int64(1) << 53 } }, "invalid_display_list", "rule x outside the exact range")
        XCTAssertEqual(Self.code { Self.setItem(&$0, 1) { $0["x"] = (Int64(1) << 53) - 1 } }, "invalid_display_list", "rule x + width overflows the exact range")
        XCTAssertEqual(Self.code { o in
            var p = o["payload"] as! [String: Any]; var pages = p["pages"] as! [[String: Any]]; pages[0]["number"] = 2; p["pages"] = pages; o["payload"] = p
        }, "invalid_display_list")
        XCTAssertEqual(Self.code { o in
            var p = o["payload"] as! [String: Any]; var pages = p["pages"] as! [[String: Any]]; pages[0]["width"] = 0; p["pages"] = pages; o["payload"] = p
        }, "invalid_display_list")
    }

    /// The typed fast reader yields exactly the `Codable` values for every
    /// real fixture and for escapes, and refuses to guess: anything it does
    /// not accept goes to `JSONDecoder`, whose diagnostics stand.
    func testFastReaderMatchesCodableAndFallsBack() throws {
        for name in ["display-list-v2-text.json", "display-list-v2-math.json", "display-list-v2-math-rules.json", "display-list-v2-diagnostics.json"] {
            let data = try Self.fixture(name)
            XCTAssertEqual(try RenderingV2Fast.envelope(data), try JSONDecoder().decode(RenderingV2.Envelope.self, from: data), name)
        }
        // Escapes, surrogate pairs, unicode and null optionals decode identically.
        var o = Self.minimal()
        Self.setItem(&o, 0) { run in
            run["text"] = "a\"b\\c/é😀\n" // JSONSerialization escapes these; the reader must decode \\uXXXX pairs

            run["clusters"] = [["text_start_byte": 0, "text_end_byte": 13, "hit_rects": [["x": 0, "top": 0, "width": 1, "height": 1]], "carets": [], "sources": [["path": "main.tex", "start_byte": 0, "end_byte": 2]], "synthetic_reason": NSNull()]]
        }
        let data = Self.data(o)
        let fast = try RenderingV2Fast.envelope(data)
        XCTAssertEqual(fast, try JSONDecoder().decode(RenderingV2.Envelope.self, from: data))
        guard case .glyphRun(let run) = fast.payload.pages[0].items[0] else { return XCTFail() }
        XCTAssertEqual(run.text, "a\"b\\c/é😀\n")
        XCTAssertNil(run.clusters[0].syntheticReason)
        // Not accepted by the fast reader → JSONDecoder decides (same public errors as before).
        XCTAssertThrowsError(try RenderingV2Fast.envelope(Data("{\"protocol_version\": 2.0}".utf8))) { XCTAssertTrue($0 is RenderingV2Fast.Error) }
        XCTAssertThrowsError(try RenderingV2Fast.envelope(Data("[1]".utf8))) { XCTAssertTrue($0 is RenderingV2Fast.Error) }
        XCTAssertEqual(Self.code { Self.setItem(&$0, 0) { $0["kind"] = "shading" } }, "unknown_item_kind")
        XCTAssertEqual(Self.code { Self.setPayload(&$0, "revision", 1.5) }, "malformed_payload")
        // Exponent form: not an integer literal for the fast reader (falls back);
        // the Codable path decides, and `decode` returns whatever it returns.
        let exponent = Data(String(decoding: data, as: UTF8.self).replacingOccurrences(of: "\"origin_x\":1048576", with: "\"origin_x\":1.048576e6").utf8)
        XCTAssertNotEqual(exponent, data, "the fixture carries the literal to rewrite")
        XCTAssertThrowsError(try RenderingV2Fast.envelope(exponent)) { XCTAssertTrue($0 is RenderingV2Fast.Error) }
        XCTAssertEqual(try? RenderingV2.decode(exponent), try? JSONDecoder().decode(RenderingV2.Envelope.self, from: exponent))
        // Explicit \\u escapes with a surrogate pair decode to the same scalar.
        let escaped = String(decoding: data, as: UTF8.self).replacingOccurrences(of: "é😀", with: "\\u00e9\\ud83d\\ude00")
        XCTAssertEqual(try RenderingV2Fast.envelope(Data(escaped.utf8)), fast)
        // Whitespace and key order do not matter.
        let reordered = Data("{ \"payload\": \(String(decoding: Self.data(o["payload"] as! [String: Any]), as: UTF8.self)) , \"type\":\"display_list\", \"id\":\"r1\", \"protocol_version\" : 2 }".utf8)
        XCTAssertEqual(try RenderingV2Fast.envelope(reordered), fast)
    }

    /// GH-277: `display-list-v2-diagnostics` optional keys round-trip on both
    /// readers; a diagnostic that omits them stays the frozen four-key object.
    func testDisplayListV2DiagnosticsFixtureKeepsStructuredFields() throws {
        let data = try Self.fixture("display-list-v2-diagnostics.json")
        let env = try RenderingV2.decode(data)
        XCTAssertEqual(env.payload.diagnostics.count, 2)
        let plain = env.payload.diagnostics[0]
        XCTAssertNil(plain.suggestion)
        XCTAssertNil(plain.labels)
        XCTAssertNil(plain.notes)
        XCTAssertNil(plain.help)
        let d = env.payload.diagnostics[1]
        XCTAssertEqual(d.suggestion, "\\alpha")
        XCTAssertEqual(d.notes, ["\\alpah looks like a misspelling of \\alpha"])
        XCTAssertEqual(d.labels?.count, 2)
        XCTAssertEqual(d.labels?[0].text, "this command")
        XCTAssertEqual(d.labels?[0].primary, true)
        XCTAssertEqual(d.help?.message, "did you mean \\alpha?")
        XCTAssertEqual(d.help?.replacement?.text, "\\alpha")
        XCTAssertEqual(d.help?.replacement?.source, .init(path: "notes.tex", startByte: 0, endByte: 6))
        let v1 = d.asRuntimeV1
        XCTAssertEqual(v1.code, "unknown_command")
        XCTAssertEqual(v1.suggestion, "\\alpha")
        XCTAssertEqual(v1.notes, d.notes)
        XCTAssertEqual(v1.help?.message, d.help?.message)
        XCTAssertEqual(v1.help?.replacement?.text, "\\alpha")
        XCTAssertEqual(v1.help?.replacement?.path, "notes.tex")
        XCTAssertEqual(v1.help?.replacement?.startByte, 0)
        XCTAssertEqual(v1.help?.replacement?.endByte, 6)
        XCTAssertEqual(v1.source, d.sources.first)
        let fast = try RenderingV2Fast.envelope(data)
        XCTAssertEqual(fast, try JSONDecoder().decode(RenderingV2.Envelope.self, from: data))
        XCTAssertEqual(fast.payload.diagnostics[1].suggestion, d.suggestion)
        XCTAssertEqual(fast.payload.diagnostics[1].help, d.help)
    }

    /// The fast diagnostic reader used to skip unknown keys by ignoring the rest
    /// of the object; a future key must not drop `suggestion` / `help`.
    func testFastDiagnosticDecoderKeepsStructuredFieldsWhenUnknownKeysArePresent() throws {
        var obj = try JSONSerialization.jsonObject(with: try Self.fixture("display-list-v2-diagnostics.json")) as! [String: Any]
        var payload = obj["payload"] as! [String: Any]
        var diags = payload["diagnostics"] as! [[String: Any]]
        diags[1]["future_extra"] = ["nested": true]
        payload["diagnostics"] = diags
        obj["payload"] = payload
        let data = Self.data(obj)
        let fast = try RenderingV2Fast.envelope(data)
        XCTAssertEqual(fast, try JSONDecoder().decode(RenderingV2.Envelope.self, from: data))
        XCTAssertEqual(fast.payload.diagnostics[1].suggestion, "\\alpha")
        XCTAssertEqual(fast.payload.diagnostics[1].help?.message, "did you mean \\alpha?")
    }

    /// `delta.rs` hashes `suggestion` only when it is `Some`; labels/notes/help
    /// stay out of the header digest. Shared golden bytes are a follow-up.
    func testHeaderDigestHashesSuggestionOnlyWhenPresent() throws {
        let list = try RenderingV2.decode(try Self.fixture("display-list-v2-diagnostics.json")).payload
        XCTAssertNotNil(list.diagnostics[1].suggestion)
        var without = list
        without.diagnostics = without.diagnostics.map { d in
            var d = d; d.suggestion = nil; return d
        }
        XCTAssertNotEqual(DisplayListDelta.headerDigest(list), DisplayListDelta.headerDigest(without),
                          "a present suggestion must change the header digest")
        var extra = list
        extra.diagnostics = extra.diagnostics.map { d in
            var d = d
            d.notes = ["ignored in the digest"]
            d.help = .init(message: "ignored in the digest")
            return d
        }
        XCTAssertEqual(DisplayListDelta.headerDigest(list), DisplayListDelta.headerDigest(extra),
                       "notes/help are not part of the header digest")
    }

    func testValidationErrorCarriesADiagnostic() {
        var o = Self.minimal()
        Self.setCluster(&o) { $0["sources"] = [["path": "missing.tex", "start_byte": 4, "end_byte": 9]] }
        do {
            _ = try RenderingV2.decode(Self.data(o)); XCTFail("expected refusal")
        } catch let e as RenderingV2.ValidationError {
            let d = e.asRuntimeV1
            XCTAssertEqual(d.severity, .error)
            XCTAssertTrue(d.message.hasPrefix("[invalid_display_list]"))
            XCTAssertEqual(d.source, RenderingV2.SourceRange(path: "missing.tex", startByte: 4, endByte: 9))
        } catch { XCTFail("\(error)") }
    }
}
