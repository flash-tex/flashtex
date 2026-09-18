import XCTest
import CoreGraphics
import FlashTeXProtocol
@testable import FlashTeXMac

/// Page-level reuse of the v2 route (V2PageCache.swift): a frame whose page
/// bytes did not change keeps that page's decoded value, prepared glyph runs
/// and bitmap, and everything painted still equals a fresh decode.
final class PreviewLatencyTests: XCTestCase {
    static let fixtures = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("Fixtures")
    static let store = PreviewV2Tests.store

    /// The real one-page text fixture as a mutable JSON object.
    static func fixtureObject() throws -> [String: Any] {
        let data = try Data(contentsOf: fixtures.appendingPathComponent("display-list-v2-text.json"))
        return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
    }

    /// Deterministic bytes (sorted keys): the same object serializes to the same
    /// bytes, so page objects that were not edited are byte-identical between
    /// two envelopes built from the same base.
    static func bytes(_ o: [String: Any]) throws -> Data { try JSONSerialization.data(withJSONObject: o, options: [.sortedKeys]) }

    /// A two-page envelope: the fixture's page duplicated as page 2.
    static func twoPages() throws -> [String: Any] {
        var o = try fixtureObject()
        var payload = o["payload"] as! [String: Any]
        var pages = payload["pages"] as! [[String: Any]]
        var second = pages[0]; second["number"] = 2
        pages.append(second)
        payload["pages"] = pages
        o["payload"] = payload
        return o
    }

    static func edit(_ o: [String: Any], _ f: (inout [String: Any]) -> Void) -> [String: Any] {
        var payload = o["payload"] as! [String: Any]
        f(&payload)
        var out = o; out["payload"] = payload
        return out
    }

    /// The same document at the next revision with page 2 repainted (page 1 untouched).
    static func typed(_ o: [String: Any]) -> [String: Any] {
        edit(o) { payload in
            payload["revision"] = (payload["revision"] as! Int) + 1
            var docs = payload["documents"] as! [[String: Any]]
            docs[0]["sha256"] = String(repeating: "ab", count: 32)
            payload["documents"] = docs
            var pages = payload["pages"] as! [[String: Any]]
            var items = pages[1]["items"] as! [[String: Any]]
            items[0]["paint"] = ["r": 0.5, "g": 0, "b": 0, "a": 1]
            pages[1]["items"] = items
            payload["pages"] = pages
        }
    }

    func testUnchangedPageBytesAreReusedAndThePaintedFrameEqualsAFreshDecode() throws {
        _ = try PreviewV2Tests.lmRoman10()
        let cache = V2PageCache()
        let a = try Self.bytes(try Self.twoPages())
        let b = try Self.bytes(Self.typed(try Self.twoPages()))
        let first = try V2Frame.prepare(data: a, store: Self.store, cache: cache)
        XCTAssertEqual(first.reusedPages, 0)
        XCTAssertEqual(cache.count, 2)
        XCTAssertEqual(first.pageTokens.count, 2)
        guard first.pageTokens.count == 2 else { return XCTFail("expected two page tokens, got \(first.pageTokens.count)") }
        // Page 1 and page 2 of `first` differ only by their number, which is part of the bytes.
        XCTAssertNotEqual(first.pageTokens[0], first.pageTokens[1])
        let second = try V2Frame.prepare(data: b, store: Self.store, cache: cache)
        XCTAssertEqual(second.reusedPages, 1, "page 1 arrived as the same bytes; page 2 changed")
        guard second.pageTokens.count == 2 else { return XCTFail("expected two page tokens, got \(second.pageTokens.count)") }
        XCTAssertEqual(second.pageTokens[0], first.pageTokens[0], "the unchanged page keeps its content token")
        XCTAssertNotEqual(second.pageTokens[1], first.pageTokens[1])
        // Byte-exact page content: what the frame holds equals a fresh, reuse-free decode.
        let fresh = try RenderingV2.decode(b)
        XCTAssertEqual(second.list, fresh.payload)
        XCTAssertEqual(second.list.revision, (first.list.revision) + 1)
        let secondDocument = try XCTUnwrap(second.list.documents.first)
        XCTAssertEqual(secondDocument.sha256, String(repeating: "ab", count: 32))
        // The reused prepared page paints exactly what a fresh preparation paints (0 differing pixels).
        let freshFrame = try V2Frame.prepare(fresh, store: Self.store)
        guard second.prepared.count == 2, freshFrame.prepared.count == 2, first.prepared.count == 2 else {
            return XCTFail("expected two prepared pages in each of second (\(second.prepared.count)), freshFrame (\(freshFrame.prepared.count)), first (\(first.prepared.count))")
        }
        for index in 0..<2 {
            let reused = try XCTUnwrap(GlyphRunRenderer.rasterize(second.prepared[index], scale: 1))
            let direct = try XCTUnwrap(GlyphRunRenderer.rasterize(freshFrame.prepared[index], scale: 1))
            XCTAssertEqual(V2Parity.differingPixels(V2Parity.rgba(reused), V2Parity.rgba(direct)), 0, "page \(index + 1)")
            XCTAssertEqual(second.prepared[index].glyphCount, freshFrame.prepared[index].glyphCount)
            XCTAssertEqual(second.prepared[index].sourceBounds, freshFrame.prepared[index].sourceBounds)
        }
        // Page 2 of `second` was repainted red; page 2 of `first` was black.
        let before = try XCTUnwrap(GlyphRunRenderer.rasterize(first.prepared[1], scale: 1))
        let after = try XCTUnwrap(GlyphRunRenderer.rasterize(second.prepared[1], scale: 1))
        XCTAssertGreaterThan(V2Parity.differingPixels(V2Parity.rgba(before), V2Parity.rgba(after)), 0)
        // Warm again: both pages are now known.
        let third = try V2Frame.prepare(data: b, store: Self.store, cache: cache)
        XCTAssertEqual(third.reusedPages, 2)
        XCTAssertEqual(third.list, fresh.payload)
        XCTAssertEqual(cache.count, 3)
    }

    func testFontManifestBytesArePartOfThePageIdentity() throws {
        _ = try PreviewV2Tests.lmRoman10()
        let cache = V2PageCache()
        let base = try Self.twoPages()
        _ = try V2Frame.prepare(data: try Self.bytes(base), store: Self.store, cache: cache)
        // Same page bytes under another manifest (an extra, unreferenced entry): not reused, decoded afresh.
        let extra = Self.edit(base) { payload in
            var fonts = payload["fonts"] as! [[String: Any]]
            var more = fonts[0]; more["font_id"] = "extra"
            fonts.append(more)
            payload["fonts"] = fonts
        }
        let data = try Self.bytes(extra)
        let frame = try V2Frame.prepare(data: data, store: Self.store, cache: cache)
        XCTAssertEqual(frame.reusedPages, 0)
        XCTAssertEqual(frame.list, try RenderingV2.decode(data).payload)
        XCTAssertEqual(cache.count, 2, "the entry for those page bytes now carries the new manifest digest")
        XCTAssertEqual(try V2Frame.prepare(data: data, store: Self.store, cache: cache).reusedPages, 2)
        XCTAssertEqual(try V2Frame.prepare(data: try Self.bytes(base), store: Self.store, cache: cache).reusedPages, 0, "back under the first manifest: prepared afresh again")
        // A manifest whose bytes for a referenced font no longer match the store
        // fails resolution exactly as on the plain path, cached page or not.
        let swapped = Self.edit(base) { payload in
            var fonts = payload["fonts"] as! [[String: Any]]
            let s0 = fonts[0]["sha256"]!, s1 = fonts[1]["sha256"]!
            fonts[0]["sha256"] = s1; fonts[1]["sha256"] = s0
            payload["fonts"] = fonts
        }
        XCTAssertThrowsError(try V2Frame.prepare(data: try Self.bytes(swapped), store: Self.store, cache: cache)) { error in
            XCTAssertEqual((error as? RenderingV2.ValidationError)?.code, "font_resource_mismatch")
        }
        XCTAssertThrowsError(try V2Frame.prepare(try RenderingV2.decode(try Self.bytes(swapped)), store: Self.store)) { error in
            XCTAssertEqual((error as? RenderingV2.ValidationError)?.code, "font_resource_mismatch")
        }
    }

    func testPageCacheIsBoundedAndLeastRecentlyUsedGoesFirst() throws {
        _ = try PreviewV2Tests.lmRoman10()
        let cache = V2PageCache(maxEntries: 2)
        let one = try V2Frame.prepare(data: try Self.bytes(try Self.twoPages()), store: Self.store, cache: cache)
        XCTAssertEqual(cache.count, 2)
        XCTAssertEqual(cache.retainedBytes, one.pageTokens.map { Int($0.split(separator: "-")[1])! }.reduce(0, +))
        // The typed frame reuses page 1 (touched: most recent) and stores page 2':
        // the third entry evicts the least recently used, page 2 of `one`.
        let typed = try V2Frame.prepare(data: try Self.bytes(Self.typed(try Self.twoPages())), store: Self.store, cache: cache)
        XCTAssertEqual(typed.reusedPages, 1)
        XCTAssertEqual(cache.count, 2)
        // Back to the original: page 1 is still held, page 2 must be decoded again (and evicts page 2').
        let again = try V2Frame.prepare(data: try Self.bytes(try Self.twoPages()), store: Self.store, cache: cache)
        XCTAssertEqual(again.reusedPages, 1)
        XCTAssertEqual(again.list, one.list)
        XCTAssertEqual(cache.count, 2)
        XCTAssertEqual(cache.hits, 2)
        XCTAssertEqual(cache.misses, 4)
        // A tiny byte bound keeps at most one page.
        let small = V2PageCache(maxEntries: 64, maxBytes: 1)
        _ = try V2Frame.prepare(data: try Self.bytes(try Self.twoPages()), store: Self.store, cache: small)
        XCTAssertEqual(small.count, 1)
        // Without a cache nothing is retained and tokens are per preparation.
        let plain = try V2Frame.prepare(data: try Self.bytes(try Self.twoPages()), store: Self.store, cache: nil)
        XCTAssertEqual(plain.reusedPages, 0)
        XCTAssertTrue(plain.pageTokens[0].hasPrefix("page1#"))
    }

    @MainActor
    func testBitmapsOfUnchangedPagesSurviveTheNextFrame() throws {
        _ = try PreviewV2Tests.lmRoman10()
        let cache = V2PageCache()
        let first = try V2Frame.prepare(data: try Self.bytes(try Self.twoPages()), store: Self.store, cache: cache)
        let rasterizer = V2PageRasterizer(maxBytes: 64 << 20)
        let full = V2Loader.preraster(first, pixelsPerPoint: 1, dark: false)
        XCTAssertEqual(full.images.count, 2)
        rasterizer.preinstall(full, frame: first)
        XCTAssertEqual(rasterizer.images.count, 2)
        let page1 = try XCTUnwrap(rasterizer.image(for: first.prepared[0], pageToken: first.pageToken(at: 0), pixelsPerPoint: 1, dark: false))
        // The next frame changed page 2 only: the loader skips page 1 (already
        // held at this scale/appearance) and the rasterizer keeps its bitmap.
        let second = try V2Frame.prepare(data: try Self.bytes(Self.typed(try Self.twoPages())), store: Self.store, cache: cache)
        let hint = try XCTUnwrap(rasterizer.rasterHint)
        XCTAssertEqual(hint.cachedTokens, Set(first.pageTokens))
        let partial = V2Loader.preraster(second, hint: hint)
        XCTAssertEqual(partial.images.map(\.token), [second.pageToken(at: 1)])
        rasterizer.preinstall(partial, frame: second)
        XCTAssertEqual(rasterizer.images.count, 2)
        XCTAssertEqual(rasterizer.staleBitmapsDropped, 0)
        XCTAssertTrue(rasterizer.image(for: second.prepared[0], pageToken: second.pageToken(at: 0), pixelsPerPoint: 1, dark: false) === page1, "same bitmap object for the same page bytes")
        XCTAssertNotNil(rasterizer.image(for: second.prepared[1], pageToken: second.pageToken(at: 1), pixelsPerPoint: 1, dark: false))
        XCTAssertNil(rasterizer.images[V2PageRasterizer.Key(pageToken: first.pageToken(at: 1), pixelsPerPoint: 1, dark: false)], "the replaced page's bitmap was evicted")
        // Render tracking for the bench: 2 new pages, then 1, then 0 for the same revision.
        let tracker = V2RenderTracker()
        XCTAssertEqual(tracker.changedPages(frame: first), 2)
        XCTAssertEqual(tracker.changedPages(frame: second), 1)
        XCTAssertEqual(tracker.changedPages(frame: second), 1)
        let same = try V2Frame.prepare(data: try Self.bytes(Self.typed(try Self.twoPages())), store: Self.store, cache: cache)
        XCTAssertEqual(tracker.changedPages(frame: same), 1, "same revision: the first evaluation's count")
    }

    func testFastReaderReportsExactPageRangesAndHonoursTheReuseHook() throws {
        let data = try Self.bytes(try Self.twoPages())
        let plain = try RenderingV2Fast.envelope(data)
        var offered: [(Int, Int)] = []
        let decoded = try RenderingV2Fast.envelope(data) { index, bytes in
            offered.append((index, bytes.count))
            XCTAssertEqual(bytes.first, UInt8(ascii: "{")); XCTAssertEqual(bytes.last, UInt8(ascii: "}"))
            return nil
        }
        XCTAssertEqual(decoded.envelope, plain, "no reuse: identical to the plain reader")
        XCTAssertEqual(decoded.reusedPages, [])
        XCTAssertEqual(decoded.pageRanges.count, 2)
        XCTAssertEqual(offered.map(\.0), [0, 1])
        XCTAssertEqual(offered.map(\.1), decoded.pageRanges.map(\.count))
        for (index, range) in decoded.pageRanges.enumerated() {
            XCTAssertEqual(try RenderingV2Fast.page(data, range: range), plain.payload.pages[index], "a recorded range decodes to the same page")
            XCTAssertEqual(try JSONDecoder().decode(RenderingV2.Page.self, from: data.subdata(in: range)), plain.payload.pages[index], "the range is exactly one page object")
        }
        let fonts = try XCTUnwrap(decoded.fontsRange)
        XCTAssertEqual(try JSONDecoder().decode([RenderingV2.FontResource].self, from: data.subdata(in: fonts)), plain.payload.fonts)
        // A reused page is taken as offered and listed; the rest is decoded.
        let substitute = RenderingV2.Page(number: 1, width: plain.payload.pages[0].width, height: plain.payload.pages[0].height, items: [])
        let reused = try RenderingV2Fast.envelope(data) { index, _ in index == 0 ? substitute : nil }
        XCTAssertEqual(reused.reusedPages, [0])
        XCTAssertEqual(reused.envelope.payload.pages[0], substitute)
        XCTAssertEqual(reused.envelope.payload.pages[1], plain.payload.pages[1])
        XCTAssertEqual(reused.pageRanges, decoded.pageRanges)
        // Malformed input is refused by the reuse reader too (the caller falls back to JSONDecoder).
        XCTAssertThrowsError(try RenderingV2Fast.envelope(Data("{\"protocol_version\":2,\"payload\":{\"pages\":[{\"number\":1,".utf8)) { _, _ in nil })
    }

    func testPaneShowsTheLoadedFrameAndThePreviousFrameStaleFromOnePosition() throws {
        _ = try PreviewV2Tests.lmRoman10()
        let frame = try V2Frame.prepare(data: try Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.json")), store: Self.store, cache: nil)
        let source = V2Source.worker(requestID: "r1", projectId: frame.list.projectId, revision: frame.list.revision, line: Data())
        let loaded = try XCTUnwrap(PreviewV2Pane.shownFrame(.loaded(frame, source)))
        XCTAssertEqual(loaded.frame.pageTokens, frame.pageTokens)
        XCTAssertFalse(loaded.stale)
        // A load in flight keeps the previous frame on screen, marked stale.
        let stale = try XCTUnwrap(PreviewV2Pane.shownFrame(.loading(source, ticket: 2, previous: frame)))
        XCTAssertEqual(stale.frame.pageTokens, frame.pageTokens)
        XCTAssertTrue(stale.stale)
        // Nothing to show: first load, a refusal, no list.
        XCTAssertNil(PreviewV2Pane.shownFrame(.loading(source, ticket: 1, previous: nil)))
        XCTAssertNil(PreviewV2Pane.shownFrame(.failed(RenderingV2.ValidationError(code: "x", message: "y"), source)))
        XCTAssertNil(PreviewV2Pane.shownFrame(nil))
    }

    @MainActor
    func testPageBitmapLayerInstallsContentsOncePerBitmapAndRecordsThePaint() throws {
        _ = try PreviewV2Tests.lmRoman10()
        let frame = try V2Frame.prepare(data: try Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.json")), store: Self.store, cache: nil)
        let bitmap = try XCTUnwrap(GlyphRunRenderer.rasterize(frame.prepared[0], scale: 1))
        let view = PageBitmapView(frame: NSRect(x: 0, y: 0, width: 612, height: 792))
        let bench = TypingBench.shared
        bench.reset()
        let revision = 1_000_000 + Int.random(in: 0..<1000)
        // No bitmap yet: nothing installed, nothing recorded.
        XCTAssertFalse(view.show(nil, pageToken: "t", pageNumber: 1, frameRevision: revision, expectedDraws: 1, background: CGColor(gray: 1, alpha: 1)))
        XCTAssertNil(view.layer?.contents)
        XCTAssertEqual(view.installs, 0)
        // The bitmap: installed as layer contents once; the paint point is recorded on the next run-loop turn.
        XCTAssertTrue(view.show(bitmap, pageToken: "t", pageNumber: 1, frameRevision: revision, expectedDraws: 1, background: CGColor(gray: 1, alpha: 1)))
        XCTAssertTrue(view.layer?.contents as! CGImage === bitmap)
        XCTAssertEqual(view.installs, 1)
        XCTAssertFalse(view.show(bitmap, pageToken: "t", pageNumber: 1, frameRevision: revision, expectedDraws: 1, background: CGColor(gray: 0.16, alpha: 1)), "the same bitmap object is not re-installed")
        XCTAssertEqual(view.installs, 1)
        XCTAssertEqual(view.layer?.backgroundColor, CGColor(gray: 0.16, alpha: 1), "appearance still applies")
        let deadline = Date().addingTimeInterval(5)
        while bench.recorder.lastPaintedRevision < revision, Date() < deadline { RunLoop.main.run(until: Date().addingTimeInterval(0.01)) }
        XCTAssertEqual(bench.recorder.lastPaintedRevision, revision)
        XCTAssertEqual(bench.recorder.paints.last?.redrawn, true)
        // A different bitmap object (another scale) replaces the contents.
        let other = try XCTUnwrap(GlyphRunRenderer.rasterize(frame.prepared[0], scale: 0.5))
        XCTAssertTrue(view.show(other, pageToken: "t", pageNumber: 1, frameRevision: revision + 1, expectedDraws: 1, background: CGColor(gray: 1, alpha: 1)))
        XCTAssertEqual(view.installs, 2)
        XCTAssertTrue(view.layer?.contents as! CGImage === other)
        bench.reset()
    }

    func testCaretBoundsNeverExcludeAMatchingCluster() throws {
        _ = try PreviewV2Tests.lmRoman10()
        let frame = try V2Frame.prepare(data: try Data(contentsOf: Self.fixtures.appendingPathComponent("display-list-v2-text.json")), store: Self.store, cache: nil)
        let page = frame.list.pages[0], prepared = frame.prepared[0]
        let length = Int(frame.list.documents[0].byteLength)
        var matched = 0
        for byte in -1...(length + 1) {
            let matches = V2Geometry.clusters(containing: byte, path: "main.tex", in: page)
            if !matches.isEmpty { matched += 1; XCTAssertTrue(prepared.mayContain(byte: byte, path: "main.tex"), "byte \(byte)") }
        }
        XCTAssertGreaterThan(matched, 50)
        XCTAssertFalse(prepared.mayContain(byte: 0, path: "other.tex"))
        XCTAssertFalse(prepared.mayContain(byte: length + 1000, path: "main.tex"))
    }

    /// In-process stage attribution on real producer siblings: set
    /// FLASHTEX_V2_BENCH_DIR to a directory holding `<seed>.v2.json` and
    /// `<seed>-typed.v2.json` (tools: docs/evidence/preview-latency-*/siblings.py)
    /// for seeds p3 and pmax. Prints cold/warm prepare, decode, validate, hash
    /// and preraster times; skipped otherwise.
    func testStageAttributionOnRealSiblings() throws {
        guard let dir = ProcessInfo.processInfo.environment["FLASHTEX_V2_BENCH_DIR"], !dir.isEmpty else { throw XCTSkip("FLASHTEX_V2_BENCH_DIR not set") }
        let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Fonts")
        let store = V2FontStore(directories: [fontsDir.path])
        func ms(_ block: () throws -> Void) rethrows -> Double {
            let t0 = MonotonicClock.nowNs(); try block(); return Double(MonotonicClock.nowNs() &- t0) / 1e6
        }
        func median(_ xs: [Double]) -> Double { let s = xs.sorted(); return s[s.count / 2] }
        var rows: [String] = ["| seed | pages | bytes | decode | validate | prepare cold | sha256 all pages | prepare warm (1 page new) | preraster 2px full | preraster 2px 1 page |", "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|"]
        for seed in ["p3", "pmax"] {
            let base = URL(fileURLWithPath: dir)
            guard let a = try? Data(contentsOf: base.appendingPathComponent("\(seed).v2.json")),
                  let b = try? Data(contentsOf: base.appendingPathComponent("\(seed)-typed.v2.json")) else { continue }
            let n = 5
            var decode: [Double] = [], validate: [Double] = [], cold: [Double] = [], hash: [Double] = [], warm: [Double] = [], full: [Double] = [], one: [Double] = []
            var pages = 0
            for _ in 0..<n {
                var env: RenderingV2.Envelope?
                decode.append(try ms { env = try RenderingV2Fast.envelope(a) })
                validate.append(try ms { try RenderingV2.validate(env!.payload) })
                let cache = V2PageCache()
                var frameA: V2Frame?
                cold.append(try ms { frameA = try V2Frame.prepare(data: a, store: store, cache: cache) })
                pages = frameA!.prepared.count
                hash.append(ms { _ = try? RenderingV2Fast.envelope(a) { _, bytes in _ = V2PageCache.sha256(bytes); return nil } })
                var frameB: V2Frame?
                warm.append(try ms { frameB = try V2Frame.prepare(data: b, store: store, cache: cache) })
                XCTAssertEqual(frameB!.reusedPages, pages - 1, seed)
                XCTAssertEqual(frameB!.list, try RenderingV2.decode(b).payload, "\(seed): reused pages equal a fresh decode")
                full.append(ms { _ = V2Loader.preraster(frameB!, pixelsPerPoint: 2, dark: false) })
                one.append(ms { _ = V2Loader.preraster(frameB!, pixelsPerPoint: 2, dark: false, skipping: Set(frameA!.pageTokens)) })
            }
            rows.append(String(format: "| %@ | %d | %d | %.1f | %.1f | %.1f | %.1f | %.1f | %.1f | %.1f |", seed, pages, a.count, median(decode), median(validate), median(cold), median(hash), median(warm), median(full), median(one)))
        }
        let table = rows.joined(separator: "\n")
        print("STAGE-ATTRIBUTION (median of 5, ms)\n" + table)
        if let out = ProcessInfo.processInfo.environment["FLASHTEX_V2_BENCH_OUT"] { try table.write(toFile: out, atomically: true, encoding: .utf8) }
    }
}
