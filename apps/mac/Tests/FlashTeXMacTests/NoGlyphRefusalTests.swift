import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The producer finding from lane mac-navigation-2, reproduced and pinned:
/// flashtex-render 9aaec57a lays out `e\u{301} \u{1F600}shuffle` (an emoji with
/// no Latin Modern glyph glued to a word) as ONE glyph run `😀shuffle` whose
/// cluster 0 (`😀`, chapter.tex bytes 13..<17) is referenced by no glyph
/// (`display-list-v2-no-glyph-cluster.json`, produced directly from the two
/// `display-list-v2-no-glyph-cluster-*.tex` documents). The fail-closed
/// validator refuses the whole list; this lane makes the refusal name the
/// cluster, its text and its source span, carry that span typed, and — on the
/// helper's candidate route — record it so the pane can point at the source
/// and offer the v1 preview of the same revision as the fallback frame.
@MainActor
final class NoGlyphRefusalTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Fonts")

    private func list() throws -> Data { try Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-no-glyph-cluster.json")) }

    func testValidatorNamesTheOrphanedClusterTextAndSourceSpan() throws {
        XCTAssertThrowsError(try RenderingV2.decode(try list())) { error in
            guard let e = error as? RenderingV2.ValidationError else { return XCTFail("\(error)") }
            XCTAssertEqual(e.code, "invalid_display_list")
            XCTAssertTrue(e.message.hasPrefix("page 1 item 4: 1 cluster(s) have no glyph (every cluster needs at least one glyph): "), e.message)
            XCTAssertTrue(e.message.contains("cluster 0 “\u{1F600}” (chapter.tex bytes 13..<17)"), e.message)
            XCTAssertEqual(e.source, RuntimeV1.SourceRange(path: "chapter.tex", startByte: 13, endByte: 17))
            XCTAssertEqual(e.asRuntimeV1.source, e.source, "the typed span reaches the v1 diagnostic form")
        }
        // The span names the emoji in the document the list was produced from.
        let chapter = try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-no-glyph-cluster-chapter.tex"), encoding: .utf8)
        XCTAssertEqual(String(decoding: Array(chapter.utf8)[13..<17], as: UTF8.self), "\u{1F600}")
        // The run's other clusters are the letters of "shuffle" with exact spans:
        // the producer dropped only the glyph, not the cluster (the reproduction).
        let json = try XCTUnwrap(try JSONSerialization.jsonObject(with: try list()) as? [String: Any])
        let pages = try XCTUnwrap((json["payload"] as? [String: Any])?["pages"] as? [[String: Any]])
        let item = try XCTUnwrap((pages[0]["items"] as? [[String: Any]])?[4])
        XCTAssertEqual(item["text"] as? String, "\u{1F600}shuffle")
        let referenced = Set(try XCTUnwrap(item["glyphs"] as? [[String: Any]]).compactMap { $0["cluster"] as? Int })
        XCTAssertEqual(referenced, [1, 2, 3, 4, 5], "5 glyphs (s h u ffl e) for 6 clusters")
    }

    func testCandidateRouteCarriesTheSpanAndOffersTheV1Preview() throws {
        let store = V2FontStore(directories: [Self.fontsDir.path])
        guard store.fonts.contains(where: { $0.url.lastPathComponent == "lmroman10-regular.otf" }) else { throw XCTSkip("bundled fonts missing") }
        // The helper's candidate for an applied v1 preview: validated off the model.
        let payload: [String: Any] = ["kind": "display_candidate", "untrusted": true, "source_actions_enabled": false,
                                      "request_id": "pc-9", "project_id": "p", "compile_revision": 1,
                                      "source_versions": ["main.tex": 1, "chapter.tex": 1], "membership_generation": 1, "display_list": "@@LIST@@"]
        let frame: [String: Any] = ["protocol_version": 1, "session_id": "s1", "id": NSNull(), "type": "update", "payload": payload]
        var text = String(decoding: try JSONSerialization.data(withJSONObject: frame, options: [.sortedKeys]), as: UTF8.self)
        text = text.replacingOccurrences(of: "\"@@LIST@@\"", with: String(decoding: try list(), as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines))
        guard case .displayCandidate(let c) = PreviewControllerClient.decode(Data(text.utf8), sessionID: "s1") else { return XCTFail("expected a candidate") }
        let main = try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-no-glyph-cluster-main.tex"), encoding: .utf8)
        let chapter = try String(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-no-glyph-cluster-chapter.tex"), encoding: .utf8)
        guard case .refused(let why, let source) = DisplayCandidateValidator.validate(c, texts: ["main.tex": main, "chapter.tex": chapter], store: store) else {
            return XCTFail("expected the validator refusal")
        }
        XCTAssertTrue(why.hasPrefix("[invalid_display_list] page 1 item 4: 1 cluster(s) have no glyph"), why)
        XCTAssertEqual(source, RuntimeV1.SourceRange(path: "chapter.tex", startByte: 13, endByte: 17))
        // What the pane records for the refusal: the span and the revision whose v1 preview is the fallback.
        let refusal = DisplayCandidateRefusal(requestID: c.requestID, editorRevision: 7, why: why, source: source)
        XCTAssertEqual(refusal.source?.path, "chapter.tex")
        XCTAssertEqual(refusal.editorRevision, 7)
        // Painting a later candidate clears it; a fresh state has none.
        let state = DisplayCandidateState()
        XCTAssertNil(state.lastInvalidCandidate)
        state.lastInvalidCandidate = refusal
        XCTAssertEqual(state.lastInvalidCandidate, refusal)
    }
}
