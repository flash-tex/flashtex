import XCTest
@testable import FlashTeXProtocol

/// `display-list-v2-compact` decoder gate (protocol/proposals/display-list-v2-compact.md):
/// a compact line and the full line of the same document decode to identical
/// `RenderingV2` values (clusters, carets, hit rects, provenance), every
/// override and default of the derivation is exercised, and the refusals are typed.
final class DisplayListCompactTests: XCTestCase {
    static let header = "\"color_space\":\"srgb\",\"coordinate_unit\":\"bp_2pow20\",\"diagnostics\":[],\"documents\":[{\"byte_length\":100,\"path\":\"main.tex\",\"revision\":1,\"sha256\":\"0000000000000000000000000000000000000000000000000000000000000000\"}],\"fonts\":[{\"byte_length\":10,\"face_index\":0,\"font_id\":\"f\",\"format\":\"opentype-cff\",\"glyph_count\":100,\"postscript_name\":\"F\",\"sha256\":\"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff\",\"units_per_em\":1000}]"
    static let footer = "\"project_id\":\"p\",\"render_format\":\"display-list-v2\",\"required_features\":[\"glyph_run\",\"rgba-srgb\",\"cluster-actualtext\"],\"revision\":1,\"text_extraction\":\"cluster-actualtext\"},\"protocol_version\":2,\"type\":\"display_list\"}"

    static func line(items: String, compact: Bool) -> Data {
        let enc = compact ? "\"cluster_encoding\":\"compact-1\"," : ""
        return Data(("{\"id\":\"x\",\"payload\":{" + enc + header + ",\"pages\":[{\"height\":830472192,\"items\":[" + items + "],\"number\":1,\"width\":641728512}]," + footer).utf8)
    }

    static let paint = "\"paint\":{\"a\":1,\"b\":0,\"g\":0,\"r\":0}"

    /// The full encoding of a word "ab" with an end caret, and of a ligature
    /// run "ﬁx" whose ligature covers two source bytes and whose second
    /// cluster starts a new source region.
    static let fullItems = """
    {"clusters":[{"carets":[{"height":20,"text_byte":0,"top":-10,"x":0}],"hit_rects":[{"height":20,"top":-10,"width":5,"x":0}],"sources":[{"end_byte":11,"path":"main.tex","start_byte":10}],"text_end_byte":1,"text_start_byte":0},{"carets":[{"height":20,"text_byte":1,"top":-10,"x":5},{"height":20,"text_byte":2,"top":-10,"x":11}],"hit_rects":[{"height":20,"top":-10,"width":6,"x":5}],"sources":[{"end_byte":12,"path":"main.tex","start_byte":11}],"text_end_byte":2,"text_start_byte":1}],"font_id":"f","font_size":12582912,"glyphs":[{"advance_x":5,"advance_y":0,"baseline_y":700,"cluster":0,"gid":1,"origin_x":0},{"advance_x":6,"advance_y":0,"baseline_y":700,"cluster":1,"gid":2,"origin_x":5}],"kind":"glyph_run",\(paint),"text":"ab"},\
    {"clusters":[{"carets":[{"height":20,"text_byte":0,"top":-10,"x":0}],"hit_rects":[{"height":20,"top":-10,"width":7,"x":0}],"sources":[{"end_byte":22,"path":"main.tex","start_byte":20}],"text_end_byte":3,"text_start_byte":0},{"carets":[{"height":24,"text_byte":3,"top":-12,"x":7}],"hit_rects":[{"height":24,"top":-12,"width":4,"x":7}],"sources":[{"end_byte":31,"path":"main.tex","start_byte":30}],"text_end_byte":4,"text_start_byte":3}],"font_id":"f","font_size":12582912,"glyphs":[{"advance_x":7,"advance_y":0,"baseline_y":700,"cluster":0,"gid":3,"origin_x":0},{"advance_x":4,"advance_y":0,"baseline_y":700,"cluster":1,"gid":6,"origin_x":7}],"kind":"glyph_run",\(paint),"text":"ﬁx"}
    """

    /// The same two runs in the compact encoding (the producer's bytes for this model).
    static let compactItems = """
    {"clusters":[{},{}],"end_caret":{"text_byte":2,"x":11},"font_id":"f","font_size":12582912,"glyphs":[[1,0,700,5,0,0],[2,5,700,6,0,1]],"hit_height":20,"hit_top":-10,"kind":"glyph_run",\(paint),"sources":[{"end_byte":12,"path":"main.tex","start_byte":10}],"text":"ab"},\
    {"clusters":[{"e":2},{"hv":[-12,24],"s":8}],"font_id":"f","font_size":12582912,"glyphs":[[3,0,700,7,0,0],[6,7,700,4,0,1]],"hit_height":20,"hit_top":-10,"kind":"glyph_run",\(paint),"sources":[{"end_byte":31,"path":"main.tex","start_byte":20}],"text":"ﬁx"}
    """

    func testCompactAndFullLinesDecodeToTheSameClusters() throws {
        let full = try RenderingV2.decode(Self.line(items: Self.fullItems, compact: false))
        let compact = try RenderingV2.decode(Self.line(items: Self.compactItems, compact: true))
        XCTAssertNil(full.payload.clusterEncoding)
        XCTAssertEqual(compact.payload.clusterEncoding, "compact-1")
        XCTAssertEqual(compact.payload.pages, full.payload.pages)
        // The slow (Codable) path cannot read compact clusters; the fast reader is the route.
        XCTAssertNoThrow(try RenderingV2Fast.envelope(Self.line(items: Self.compactItems, compact: true)))
        // Both decodes carry identical carets and hit rects within 1e-6 bp (they are integer ticks).
        for (pf, pc) in zip(full.payload.pages, compact.payload.pages) {
            for (a, b) in zip(pf.items, pc.items) {
                guard case .glyphRun(let ra) = a, case .glyphRun(let rb) = b else { XCTFail("run"); continue }
                for (ca, cb) in zip(ra.clusters, rb.clusters) {
                    XCTAssertEqual(ca.carets.count, cb.carets.count)
                    for (ka, kb) in zip(ca.carets, cb.carets) {
                        XCTAssertEqual(ka.textByte, kb.textByte)
                        XCTAssertEqual(RenderingV2.points(ka.x), RenderingV2.points(kb.x), accuracy: 1e-6)
                        XCTAssertEqual(RenderingV2.points(ka.top), RenderingV2.points(kb.top), accuracy: 1e-6)
                        XCTAssertEqual(RenderingV2.points(ka.height), RenderingV2.points(kb.height), accuracy: 1e-6)
                    }
                    XCTAssertEqual(ca.hitRects, cb.hitRects)
                    XCTAssertEqual(ca.sources, cb.sources)
                }
            }
        }
    }

    // MARK: derivation rules, one override at a time

    func run(_ clusters: [DisplayListCompact.RawCluster], header: DisplayListCompact.RunHeader, glyphs: [RenderingV2.Glyph], text: String) throws -> [RenderingV2.Cluster] {
        try DisplayListCompact.resolve(clusters, header: header, glyphs: glyphs, text: text)
    }

    static func glyph(_ gid: Int, _ ox: Int64, _ adv: Int64, _ cluster: Int) -> RenderingV2.Glyph {
        RenderingV2.Glyph(gid: gid, originX: ox, baselineY: 700, advanceX: adv, advanceY: 0, cluster: cluster)
    }
    static let span = [RenderingV2.SourceRange(path: "m", startByte: 100, endByte: 103)]

    func testDefaultsDeriveEveryField() throws {
        let raw = [DisplayListCompact.RawCluster(), DisplayListCompact.RawCluster(), DisplayListCompact.RawCluster()]
        let header = DisplayListCompact.RunHeader(hitTop: -10, hitHeight: 20, endCaret: (textByte: 4, x: 99), sources: Self.span)
        // "aé" + "b": cluster 1 has two glyphs (base + combining mark, zero advance), so its width is the sum.
        let out = try run(raw, header: header, glyphs: [Self.glyph(1, 0, 5, 0), Self.glyph(2, 5, 6, 1), Self.glyph(3, 5, 0, 1), Self.glyph(4, 11, 7, 2)], text: "aéb")
        XCTAssertEqual(out.map { [$0.textStartByte, $0.textEndByte] }, [[0, 1], [1, 3], [3, 4]])
        XCTAssertEqual(out.map(\.hitRects), [[.init(x: 0, top: -10, width: 5, height: 20)], [.init(x: 5, top: -10, width: 6, height: 20)], [.init(x: 11, top: -10, width: 7, height: 20)]])
        XCTAssertEqual(out[0].carets, [.init(textByte: 0, x: 0, top: -10, height: 20)])
        XCTAssertEqual(out[1].carets, [.init(textByte: 1, x: 5, top: -10, height: 20)])
        XCTAssertEqual(out[2].carets, [.init(textByte: 3, x: 11, top: -10, height: 20), .init(textByte: 4, x: 99, top: -10, height: 20)])
        // Source chain: lengths default to the text lengths, starts to the previous end.
        XCTAssertEqual(out.map { $0.sources! }, [[.init(path: "m", startByte: 100, endByte: 101)], [.init(path: "m", startByte: 101, endByte: 103)], [.init(path: "m", startByte: 103, endByte: 104)]])
        XCTAssertTrue(out.allSatisfy { $0.syntheticReason == nil })
    }

    func testEachOverrideReplacesItsDefault() throws {
        var c0 = DisplayListCompact.RawCluster(); c0.carets = [.init(textByte: 0, x: 1, top: -11, height: 21), .init(textByte: 1, x: 4, top: -10, height: 20)]
        var c1 = DisplayListCompact.RawCluster(); c1.hitRect = .init(x: 16, top: -12, width: 3, height: 24); c1.sourceDelta = 3; c1.sourceLength = 5
        var c2 = DisplayListCompact.RawCluster(); c2.hitVertical = (-30, 40); c2.textLength = 2; c2.textStart = 3
        var c3 = DisplayListCompact.RawCluster(); c3.hasExplicitProvenance = true; c3.sources = [.init(path: "other.tex", startByte: 0, endByte: 1), .init(path: "m", startByte: 5, endByte: 6)]
        var c4 = DisplayListCompact.RawCluster(); c4.hasExplicitProvenance = true; c4.syntheticReason = "heading number"
        let c5 = DisplayListCompact.RawCluster() // implicit again: the chain continues from c1's end
        let header = DisplayListCompact.RunHeader(hitTop: -10, hitHeight: 20, endCaret: (textByte: 7, x: 40), sources: Self.span)
        let glyphs = (0..<6).map { Self.glyph($0 + 1, Int64($0) * 5, 5, $0) }
        let out = try run([c0, c1, c2, c3, c4, c5], header: header, glyphs: glyphs, text: "pqrstuvw")
        XCTAssertEqual(out[0].carets, c0.carets)
        XCTAssertEqual(out[0].hitRects, [.init(x: 0, top: -10, width: 5, height: 20)])
        XCTAssertEqual(out[1].hitRects, [.init(x: 16, top: -12, width: 3, height: 24)])
        XCTAssertEqual(out[1].carets, [.init(textByte: 1, x: 16, top: -12, height: 24)], "the derived caret follows the overridden rect")
        XCTAssertEqual(out[1].sources, [.init(path: "m", startByte: 104, endByte: 109)], "s from the chain end 101, e explicit")
        XCTAssertEqual(out[2].textStartByte, 3); XCTAssertEqual(out[2].textEndByte, 5)
        XCTAssertEqual(out[2].hitRects, [.init(x: 10, top: -30, width: 5, height: 40)])
        XCTAssertEqual(out[2].sources, [.init(path: "m", startByte: 109, endByte: 111)], "e defaults to the (overridden) text length")
        XCTAssertEqual(out[3].sources, c3.sources); XCTAssertNil(out[3].syntheticReason)
        XCTAssertNil(out[4].sources); XCTAssertEqual(out[4].syntheticReason, "heading number")
        XCTAssertEqual(out[3].textStartByte, 5, "the text chain continues through overrides: 5 after c2's [3,5)")
        XCTAssertEqual(out[5].sources, [.init(path: "m", startByte: 111, endByte: 112)], "explicit clusters leave the source chain untouched")
        XCTAssertEqual(out[5].carets.count, 2, "the end caret sits on the last cluster")
        XCTAssertEqual(out[5].carets[1], .init(textByte: 7, x: 40, top: -10, height: 20))
    }

    func testRefusals() {
        // Implicit cluster without a run span.
        XCTAssertThrowsError(try run([DisplayListCompact.RawCluster()], header: .init(hitTop: 0, hitHeight: 1), glyphs: [Self.glyph(1, 0, 1, 0)], text: "a"))
        // No hit_top/hit_height and no override.
        XCTAssertThrowsError(try run([DisplayListCompact.RawCluster()], header: .init(sources: Self.span), glyphs: [Self.glyph(1, 0, 1, 0)], text: "a"))
        // Two spans on the run.
        XCTAssertThrowsError(try run([DisplayListCompact.RawCluster()], header: .init(hitTop: 0, hitHeight: 1, sources: Self.span + Self.span), glyphs: [Self.glyph(1, 0, 1, 0)], text: "a"))
        // An unknown encoding is refused by validation; a mixed run and a malformed glyph array by the reader.
        let unknown = Data(String(decoding: Self.line(items: Self.compactItems, compact: true), as: UTF8.self).replacingOccurrences(of: "compact-1", with: "compact-9").utf8)
        XCTAssertThrowsError(try RenderingV2.decode(unknown)) { e in XCTAssertEqual((e as? RenderingV2.ValidationError)?.code, "unsupported_feature") }
        let mixed = Self.line(items: Self.compactItems.replacingOccurrences(of: "{\"e\":2},{\"hv\":[-12,24],\"s\":8}", with: "{\"e\":2},{\"carets\":[],\"hit_rects\":[],\"text_end_byte\":4,\"text_start_byte\":3}"), compact: true)
        XCTAssertThrowsError(try RenderingV2Fast.envelope(mixed))
        let shortGlyph = Self.line(items: Self.compactItems.replacingOccurrences(of: "[1,0,700,5,0,0]", with: "[1,0,700,5,0]"), compact: true)
        XCTAssertThrowsError(try RenderingV2Fast.envelope(shortGlyph))
    }

    func testRunSpanAndImplicitPredicatesFollowTheModel() throws {
        let full = try RenderingV2.decode(Self.line(items: Self.fullItems, compact: false))
        guard case .glyphRun(let r0) = full.payload.pages[0].items[0], case .glyphRun(let r1) = full.payload.pages[0].items[1] else { return XCTFail() }
        XCTAssertEqual(DisplayListCompact.runSpan(r0).map { [$0.start, $0.end] }, [10, 12])
        XCTAssertEqual(DisplayListCompact.runSpan(r1).map { [$0.start, $0.end] }, [20, 31])
        XCTAssertEqual(DisplayListCompact.runPath(r0), "main.tex")
        XCTAssertTrue(DisplayListCompact.isImplicit(r0.clusters[0], runPath: "main.tex"))
        XCTAssertFalse(DisplayListCompact.isImplicit(r0.clusters[0], runPath: "other.tex"))
    }
}
