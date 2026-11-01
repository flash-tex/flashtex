import XCTest
import CoreText
import FlashTeXProtocol
@testable import FlashTeXMac

/// The preview line cache must not change geometry: keys are exact sizes and
/// the fonts are built at exactly the requested size. Retention is bounded by
/// a least-recently-used policy for lines and fonts independently, and every
/// entry is tied to the font resource generation it was built at.
final class PreviewTextCacheTests: XCTestCase {
    /// The font CoreText actually used for the line's first run.
    static func runFont(_ line: CTLine) -> CTFont {
        let runs = CTLineGetGlyphRuns(line) as! [CTRun]
        let attrs = CTRunGetAttributes(runs[0]) as! [NSAttributedString.Key: Any]
        return attrs[.font] as! CTFont
    }

    /// PostScript name of the font CoreText actually used for the line's first run.
    static func runFontName(_ line: CTLine) -> String { CTFontCopyPostScriptName(runFont(line)) as String }

    static let samples = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        .appendingPathComponent("Samples")

    // MARK: exact geometry

    func testKeysDistinguishNearlyEqualSizesAndFontsAreExact() {
        let cache = PreviewTextCache()
        let a = PreviewTextCache.key(text: "x", postScriptName: "Times-Roman", size: 10.0)
        let b = PreviewTextCache.key(text: "x", postScriptName: "Times-Roman", size: 10.004)
        XCTAssertNotEqual(a, b, "10.000 and 10.004 pt are different entries")
        XCTAssertEqual(a.size, 10.0)
        XCTAssertEqual(b.size, 10.004)
        _ = cache.line(for: a); _ = cache.line(for: b)
        XCTAssertEqual(cache.count, 2)
        XCTAssertEqual(cache.fontCount, 2)
        XCTAssertEqual(CTFontGetSize(cache.font("Times-Roman", size: 10.004)), 10.004, accuracy: 0)
        XCTAssertEqual(CTFontGetSize(cache.font("Times-Roman", size: 9.999999)), 9.999999, accuracy: 0)
        // Metrics scale with the exact size, not a quantized one.
        let w10 = cache.line(for: a).width, w10004 = cache.line(for: b).width
        XCTAssertGreaterThan(w10004, w10)
        XCTAssertEqual(w10004 / w10, 10.004 / 10.0, accuracy: 1e-9)
    }

    func testIdenticalRequestsHitAndCacheIsNotQuantized() {
        let cache = PreviewTextCache()
        let k = PreviewTextCache.key(text: "Hello", postScriptName: "Times-Roman", size: 12.25)
        _ = cache.line(for: k); _ = cache.line(for: k)
        XCTAssertEqual(cache.hits, 1); XCTAssertEqual(cache.misses, 1)
        let k2 = PreviewTextCache.key(text: "Hello", postScriptName: "Times-Roman", size: 12.25.nextUp)
        _ = cache.line(for: k2)
        XCTAssertEqual(cache.misses, 2, "one ulp apart is a distinct entry")
    }

    func testCachedLineIsBitIdenticalToUncachedConstruction() {
        let cache = PreviewTextCache()
        for (text, name, size) in [("Résumé — naïve", "Times-Roman", 17.0), ("fi fl", "Times-Italic", 10.004), ("x", "Times-Bold", 2.5 * 12.0)] {
            let cached = cache.line(for: PreviewTextCache.key(text: text, postScriptName: name, size: size))
            let fresh = PreviewTextCache.build(text: text, font: CTFontCreateWithName(name as CFString, size, nil))
            XCTAssertEqual(cached.ascent, fresh.ascent, accuracy: 0, text)
            XCTAssertEqual(cached.descent, fresh.descent, accuracy: 0, text)
            XCTAssertEqual(cached.width, fresh.width, accuracy: 0, text)
            XCTAssertEqual(CTFontGetSize(cache.font(name, size: size)), size, accuracy: 0)
        }
    }

    // MARK: bounded retention (LRU)

    func testLineRetentionIsLeastRecentlyUsedAtTheBound() {
        let cache = PreviewTextCache(capacity: 8, fontCapacity: 4)
        let keys = (0..<20).map { PreviewTextCache.key(text: "t\($0)", postScriptName: "Times-Roman", size: 10) }
        for k in keys[0..<8] { _ = cache.line(for: k) }
        XCTAssertEqual(cache.count, 8); XCTAssertEqual(cache.lineEvictions, 0)
        // Touch t0 so it becomes most recent; t1 is now the LRU entry.
        _ = cache.line(for: keys[0])
        XCTAssertEqual(cache.hits, 1)
        _ = cache.line(for: keys[8])
        XCTAssertEqual(cache.count, 8)
        XCTAssertEqual(cache.lineEvictions, 1)
        XCTAssertFalse(cache.contains(keys[1]), "t1 was least recently used")
        XCTAssertTrue(cache.contains(keys[0]), "a touched entry survives")
        XCTAssertEqual(cache.lineKeysMostRecentFirst.map(\.text), ["t8", "t0", "t7", "t6", "t5", "t4", "t3", "t2"])
        for k in keys[9...] { _ = cache.line(for: k) }
        XCTAssertEqual(cache.count, 8, "never above capacity")
        XCTAssertEqual(cache.lineEvictions, 12)
        XCTAssertEqual(cache.lineKeysMostRecentFirst.map(\.text), (12..<20).reversed().map { "t\($0)" })
        XCTAssertEqual(cache.generationRetirements, 0, "the bound is not a drop-all")
        // Hits keep working on what is retained; a retained hit rebuilds nothing.
        let misses = cache.misses
        _ = cache.line(for: keys[19]); _ = cache.line(for: keys[12])
        XCTAssertEqual(cache.misses, misses)
    }

    func testEvictionBoundsBothLinesAndFonts() {
        let cache = PreviewTextCache()
        cache.capacity = 8
        cache.fontCapacity = 4
        for i in 0..<20 {
            _ = cache.line(for: PreviewTextCache.key(text: "t\(i)", postScriptName: "Times-Roman", size: 10))
        }
        XCTAssertLessThanOrEqual(cache.count, 8)
        for i in 0..<20 {
            _ = cache.font("Times-Roman", size: CGFloat(10 + i))
        }
        XCTAssertEqual(cache.fontCount, 4, "fonts are bounded independently of lines")
        XCTAssertEqual(cache.fontEvictions, 16)
        XCTAssertEqual(cache.fontKeysMostRecentFirst.map { Double(bitPattern: $0.sizeBits) }, [29, 28, 27, 26])
        XCTAssertEqual(cache.count, 8, "font eviction never drops lines")
        XCTAssertEqual(cache.lineEvictions, 12)
        // Lines built with an evicted font are still served: a CTLine retains its font.
        let hits = cache.hits
        let l = cache.line(for: PreviewTextCache.key(text: "t19", postScriptName: "Times-Roman", size: 10))
        XCTAssertEqual(cache.hits, hits + 1)
        XCTAssertEqual(CTFontGetSize(Self.runFont(l.line)), 10)
    }

    func testLoweringCapacityEvictsImmediatelyAndEveryOperationStaysO1Shaped() {
        let lru = LRUCache<Int, Int>(capacity: 100)
        for i in 0..<100 { lru.insert(i, for: i) }
        XCTAssertEqual(lru.value(for: 0), 0)
        lru.capacity = 3
        XCTAssertEqual(lru.count, 3)
        XCTAssertEqual(lru.keysMostRecentFirst, [0, 99, 98])
        XCTAssertNil(lru.peek(97))
        XCTAssertEqual(lru.insert(7, for: 7), 98)
        XCTAssertEqual(lru.keysMostRecentFirst, [7, 0, 99])
        XCTAssertNil(lru.insert(70, for: 7), "replacing an existing key evicts nothing")
        XCTAssertEqual(lru.peek(7), 70)
        lru.removeAll()
        XCTAssertEqual(lru.count, 0); XCTAssertEqual(lru.keysMostRecentFirst, [])
        XCTAssertNil(lru.insert(1, for: 1))
        XCTAssertEqual(lru.keysMostRecentFirst, [1])
    }

    // MARK: resource generation

    func testKeysCarryTheResourceGenerationAndTheStoreRetiresWhenItMoves() {
        let cache = PreviewTextCache()
        let g0 = PreviewFonts.resourceGeneration
        let stale = PreviewTextCache.key(text: "gen", postScriptName: "Times-Roman", size: 10)
        XCTAssertEqual(stale.generation, g0)
        let before = cache.line(for: stale)
        XCTAssertTrue(cache.contains(stale))
        XCTAssertEqual(cache.generation, g0)

        PreviewFonts.invalidateResources()
        XCTAssertEqual(PreviewFonts.resourceGeneration, g0 + 1)
        let fresh = PreviewTextCache.key(text: "gen", postScriptName: "Times-Roman", size: 10)
        XCTAssertNotEqual(fresh, stale, "the generation is part of the identity")
        XCTAssertEqual(fresh.generation, g0 + 1)

        let after = cache.line(for: fresh)
        XCTAssertEqual(cache.generationRetirements, 1)
        XCTAssertEqual(cache.generation, g0 + 1)
        XCTAssertEqual(cache.misses, 2, "a moved generation is a miss even for the same text/face/size")
        XCTAssertFalse(before.line === after.line, "never the line built against the old resource set")
        XCTAssertFalse(cache.contains(stale))
        XCTAssertEqual(cache.count, 1); XCTAssertEqual(cache.fontCount, 1)

        // A caller holding a stale key is served the current generation's entry
        // (a hit now that it exists), and the stale key is never inserted.
        let viaStale = cache.line(for: stale)
        XCTAssertEqual(cache.hits, 1)
        XCTAssertTrue(viaStale.line === after.line)
        XCTAssertEqual(cache.count, 1)
        XCTAssertEqual(cache.lineKeysMostRecentFirst.map(\.generation), [g0 + 1])

        // Fonts retire with the lines: the same name may denote another font now.
        let f0 = cache.font("Times-Roman", size: 10)
        PreviewFonts.invalidateResources()
        XCTAssertEqual(cache.fontCount, 1, "not yet observed")
        let f1 = cache.font("Times-Roman", size: 10)
        XCTAssertEqual(cache.generationRetirements, 2)
        XCTAssertFalse(f0 === f1)
        XCTAssertEqual(cache.count, 0)
    }

    func testProducerFaceSwitchNeverServesALineBuiltWithTheOldFace() throws {
        let original = PreviewFonts.producerFace, env = PreviewFonts.environmentFace
        defer { PreviewFonts.producerFace = original; PreviewFonts.overrideEnvironmentFace(env) }
        PreviewFonts.overrideEnvironmentFace(nil)
        guard PreviewFonts.latinModernRegistered else { throw XCTSkip("Latin Modern not registered on this machine") }

        let cache = PreviewTextCache()
        func lineForLegacyItem() -> (key: PreviewTextCache.Key, line: PreviewTextCache.Line) {
            // What PreviewView does per item: resolve the face, then key at the exact size.
            let name = PreviewFonts.resolve(hint: nil, size: 10).postScriptName
            let key = PreviewTextCache.key(text: "Résumé", postScriptName: name, size: 10 * 2.0)
            return (key, cache.line(for: key))
        }

        PreviewFonts.producerFace = .times // flashtex-compiler
        let g = PreviewFonts.resourceGeneration
        let times = lineForLegacyItem()
        XCTAssertEqual(times.key.postScriptName, "Times-Roman")
        XCTAssertEqual(Self.runFontName(times.line.line), "Times-Roman")
        XCTAssertEqual(cache.line(for: times.key).line === times.line.line, true, "same face, same generation: a hit")

        PreviewFonts.producerFace = .latinModern // flashtex-render
        XCTAssertEqual(PreviewFonts.resourceGeneration, g + 1, "the switch is a resource change")
        let lm = lineForLegacyItem()
        XCTAssertEqual(lm.key.postScriptName, "LMRoman10-Regular")
        XCTAssertEqual(lm.key.generation, g + 1)
        XCTAssertEqual(Self.runFontName(lm.line.line), "LMRoman10-Regular")
        XCTAssertFalse(lm.line.line === times.line.line)
        XCTAssertNotEqual(lm.line.width, times.line.width, "Latin Modern and Times measure differently")
        XCTAssertFalse(cache.contains(times.key), "the Times entry retired with its generation")
        XCTAssertEqual(cache.lineKeysMostRecentFirst.map(\.postScriptName), ["LMRoman10-Regular"])

        PreviewFonts.producerFace = .latinModern
        XCTAssertEqual(PreviewFonts.resourceGeneration, g + 1, "setting the same face is not a change")
        XCTAssertTrue(cache.line(for: lm.key).line === lm.line.line)

        PreviewFonts.producerFace = .times
        XCTAssertEqual(PreviewFonts.resourceGeneration, g + 2)
        let back = lineForLegacyItem()
        XCTAssertEqual(Self.runFontName(back.line.line), "Times-Roman")
        XCTAssertFalse(back.line.line === times.line.line, "rebuilt at the new generation, not the retired one")
        XCTAssertEqual(back.line.width, times.line.width, accuracy: 0, "same face → bit-identical metrics")

        // FLASHTEX_PREVIEW_FACE override behaves the same way.
        PreviewFonts.overrideEnvironmentFace(.latinModern)
        XCTAssertEqual(PreviewFonts.resourceGeneration, g + 3)
        XCTAssertEqual(lineForLegacyItem().key.postScriptName, "LMRoman10-Regular")
        XCTAssertFalse(cache.contains(back.key))
        XCTAssertEqual(cache.generationRetirements, 3)
    }

    /// The same PostScript name denotes a different font before and after the
    /// Latin Modern masters are registered; a hit across that change would draw
    /// the fallback face. Unregisters the masters for the duration of the test
    /// and restores them no matter what.
    func testSameNameAfterFontRegistrationIsNotServedFromTheFallbackLine() throws {
        guard PreviewFonts.latinModernRegistered, let dir = PreviewFonts.latinModernDirectory else {
            throw XCTSkip("Latin Modern not registered on this machine")
        }
        let urls = try FileManager.default.contentsOfDirectory(at: URL(fileURLWithPath: dir), includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "otf" && $0.lastPathComponent.hasPrefix("lmroman") }
        var failures = 0
        CTFontManagerUnregisterFontURLs(urls as CFArray, .process) { errors, _ in failures += CFArrayGetCount(errors); return true }
        var registered = false
        func register() {
            CTFontManagerRegisterFontURLs(urls as CFArray, .process, true) { errors, _ in failures += CFArrayGetCount(errors); return true }
            registered = true
        }
        defer { if !registered { register(); PreviewFonts.invalidateResources() } }
        guard failures == 0, CTFontCopyPostScriptName(CTFontCreateWithName("LMRoman10-Regular" as CFString, 10, nil)) as String != "LMRoman10-Regular" else {
            throw XCTSkip("CoreText did not unregister the Latin Modern masters (\(failures) errors)")
        }
        PreviewFonts.invalidateResources() // the caller that changed the set reports it
        let cache = PreviewTextCache()
        let name = "LMRoman10-Regular"
        let fallback = cache.line(for: PreviewTextCache.key(text: "fi", postScriptName: name, size: 10))
        let fallbackName = Self.runFontName(fallback.line)
        XCTAssertNotEqual(fallbackName, name, "unregistered: CoreText substitutes another face")

        register(); XCTAssertEqual(failures, 0)
        PreviewFonts.invalidateResources()
        let real = cache.line(for: PreviewTextCache.key(text: "fi", postScriptName: name, size: 10))
        XCTAssertEqual(Self.runFontName(real.line), name)
        XCTAssertFalse(real.line === fallback.line)
        XCTAssertEqual(cache.misses, 2); XCTAssertEqual(cache.hits, 0)
        XCTAssertEqual(cache.generationRetirements, 1)
        XCTAssertEqual(CTFontCopyPostScriptName(cache.font(name, size: 10)) as String, name)
        XCTAssertNotEqual(real.width, fallback.width, "the two faces measure differently, so a stale hit would move glyphs")
    }

    // MARK: workload measurement (demo.tex)

    /// Cold vs warm line construction for every text item of `Samples/demo-result.json`
    /// (the compiled `demo.tex`), at the 1× and 2× (Retina) scales the preview
    /// uses. Counts are asserted; timings are printed for the handoff and never
    /// asserted against a tolerance.
    func testDemoWorkloadColdVersusWarmIsExactAndFullyHit() throws {
        let data = try Data(contentsOf: Self.samples.appendingPathComponent("demo-result.json"))
        let result = try RuntimeV1.decodeCompileResult(data).payload
        let items = result.pages.reduce(0) { $0 + $1.items.filter { if case .text = $0 { return true } else { return false } }.count }
        XCTAssertGreaterThan(items, 0)
        for scale in [1.0, 2.0] as [CGFloat] {
            let (w, cache) = PreviewTextCache.measure(pages: result.pages, scale: scale)
            print("preview-text-cache demo.tex scale=\(scale) face=\(PreviewFonts.active.rawValue) gen=\(PreviewFonts.resourceGeneration): \(w)")
            XCTAssertEqual(w.items, items)
            XCTAssertEqual(w.coldMisses, w.distinctKeys)
            XCTAssertEqual(w.coldHits, items - w.distinctKeys)
            XCTAssertEqual(w.warmMisses, 0, "the warm pass must be hits only")
            XCTAssertEqual(w.warmHits, items)
            XCTAssertTrue(w.metricsIdentical, "cached metrics must be bit-identical to uncached construction")
            XCTAssertEqual(cache.count, w.distinctKeys)
            XCTAssertEqual(cache.fontsBuilt, w.fontsBuilt)
            XCTAssertEqual(cache.generationRetirements, 0)
            XCTAssertLessThanOrEqual(cache.fontCount, cache.fontCapacity)
            // Every cached font is at the exact size the item asked for.
            for fk in cache.fontKeysMostRecentFirst {
                let size = CGFloat(Double(bitPattern: fk.sizeBits))
                XCTAssertEqual(CTFontGetSize(cache.font(fk.postScriptName, size: size)), size, accuracy: 0)
            }
            // Per item: the cached line equals a fresh uncached construction bit for bit.
            var checked = 0
            for page in result.pages {
                for case .text(let t) in page.items {
                    let name = PreviewFonts.resolve(hint: t.font, size: t.fontSizePt).postScriptName
                    let key = PreviewTextCache.key(text: t.text, postScriptName: name, size: t.fontSizePt * scale)
                    let cached = cache.line(for: key)
                    let fresh = PreviewTextCache.build(text: t.text, font: CTFontCreateWithName(name as CFString, key.size, nil))
                    XCTAssertEqual(cached.width, fresh.width, accuracy: 0, t.text)
                    XCTAssertEqual(cached.ascent, fresh.ascent, accuracy: 0, t.text)
                    XCTAssertEqual(cached.descent, fresh.descent, accuracy: 0, t.text)
                    checked += 1
                }
            }
            XCTAssertEqual(checked, items)
            XCTAssertEqual(cache.misses, w.coldMisses, "the verification pass added no misses")
        }
    }

    /// A cache smaller than the document still bounds retention and stays exact;
    /// the warm pass then thrashes rather than silently keeping more than allowed.
    func testDemoWorkloadUnderABoundSmallerThanTheDocument() throws {
        let data = try Data(contentsOf: Self.samples.appendingPathComponent("demo-result.json"))
        let result = try RuntimeV1.decodeCompileResult(data).payload
        let (full, _) = PreviewTextCache.measure(pages: result.pages, scale: 1)
        let bound = max(1, full.distinctKeys / 2)
        let (w, cache) = PreviewTextCache.measure(pages: result.pages, scale: 1, capacity: bound, fontCapacity: 2)
        print("preview-text-cache demo.tex scale=1 capacity=\(bound) fontCapacity=2: \(w)")
        XCTAssertEqual(cache.count, bound)
        XCTAssertLessThanOrEqual(cache.fontCount, 2)
        XCTAssertTrue(w.metricsIdentical)
        XCTAssertGreaterThan(w.warmMisses, 0, "half the document cannot all hit")
        XCTAssertEqual(w.warmHits + w.warmMisses, w.items)
        XCTAssertGreaterThan(cache.lineEvictions, 0)
        XCTAssertEqual(cache.generationRetirements, 0)
    }
}
