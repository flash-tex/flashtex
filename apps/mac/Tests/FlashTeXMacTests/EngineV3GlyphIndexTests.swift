import CoreGraphics
import Foundation
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
@testable import FlashTeXMac

/// The bounded glyph indexes of forward and reverse search
/// (EngineV3GlyphIndexes.swift; mem-research §2.9 #10), and the
/// memory-pressure handler (EngineV3MemoryPressure.swift, #11).
///
/// Pages are a fixture page's glyphs, given their own spans (one per 40
/// glyphs, distinct per page unless a test shares them), installed through
/// `handle(.page)` as the reader's pages are. No host runs: the pressure
/// tests talk to a stand-in host on a Unix socket.
@MainActor
final class EngineV3GlyphIndexTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-glyph-index-\(getpid())")
    private var env = EnvironmentOverride()
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore(); try? FileManager.default.removeItem(at: Self.cache) }

    // MARK: pages

    /// The fixture page with the most glyphs (tile-text: Type 1 text).
    static let base: DL3PreparedPage = {
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("FlashTeXDisplayListV3Tests/Fixtures/tile-text.dl3")
        let pages = (try? DL3Document(frames: Array(try Data(contentsOf: url))).orderedPages) ?? []
        return pages.max { DL3SourceIndex($0).glyphs.count < DL3SourceIndex($1).glyphs.count }!
    }()

    /// Page `index`: the base page's items, repeated until there are at
    /// least 600 glyphs (each copy 1 sp lower), with spans `spanBase`,
    /// `spanBase + 1`, … (one per 40 glyphs), moved right by `shift` sp.
    static func page(_ index: Int, spanBase: UInt32, shift: Int32 = 0) -> DL3PreparedPage {
        var p = base
        p.page.index = UInt32(index)
        withUnsafeBytes(of: UInt64(index) << 32 | UInt64(spanBase)) { p.page.hash.replaceSubrange(0 ..< 8, with: $0) }
        p.page.hash[8] = UInt8(truncatingIfNeeded: shift)
        var items: [DL3Item] = []
        var n = 0, copy: Int32 = 0
        while n < 600 {
            let before = n
            for it in base.page.items {
                switch it {
                case .span: continue
                case .glyph(let f, let code, let x, let y, _):
                    if n % 40 == 0 { items.append(.span(spanBase + UInt32(n / 40))) }
                    items.append(.glyph(font: f, code: code, x: x + shift, y: y - copy, col: UInt16(n % 40)))
                    n += 1
                default: items.append(it)
                }
            }
            precondition(n > before, "the fixture page has glyphs")
            copy += 1
        }
        p.page.items = items
        return p
    }

    /// A session holding `n` pages; page i's spans start at `spanBase(i)`.
    func session(pages n: Int, spanBase: (Int) -> UInt32 = { UInt32($0 + 1) * 10_000 }) -> EngineV3Session {
        let s = EngineV3Session(smoothFonts: false)
        for i in 0 ..< n { s.handle(.page(Self.page(i, spanBase: spanBase(i)), compileID: 1, timing: .init(), image: nil)) }
        return s
    }

    /// What forward search answered before (every page's index, first page with a glyph of the spans).
    static func fullScan(_ s: EngineV3Session, _ spans: Set<UInt32>, col: Int?) -> EngineV3Place? {
        for i in 0 ..< s.pageCount {
            guard let p = s.pages[i] else { continue }
            if let box = DL3SourceIndex.box(DL3SourceIndex(p).glyphs(of: spans, col: col)) { return EngineV3Place(page: i, rect: box) }
        }
        return nil
    }

    func testTheTestPagesHaveIndexedGlyphs() {
        let ix = DL3SourceIndex(Self.page(0, spanBase: 1))
        XCTAssertGreaterThanOrEqual(ix.glyphs.count, 600, "every glyph's font resolved")
        XCTAssertEqual(EngineV3GlyphIndexes.glyphSpans(Self.page(0, spanBase: 1).page).count, (ix.glyphs.count + 39) / 40)
    }

    // MARK: the bound

    /// Caret follow on the last page builds that page's index only, and a
    /// search on every page in turn holds at most `capacity` indexes.
    func testForwardSearchIndexesOnlyThePageThatShowsTheLine() throws {
        let n = 120
        let s = session(pages: n)
        let last = try XCTUnwrap(s.place(spans: [UInt32(n) * 10_000], col: nil))
        XCTAssertEqual(last.page, n - 1)
        XCTAssertEqual(s.glyphIndexes.builds, 1, "one index, not one per page before it")
        XCTAssertEqual(s.glyphIndexes.count, 1)
        XCTAssertEqual(s.glyphIndexes.summarized, n)
        for i in 0 ..< n {
            let place = try XCTUnwrap(s.place(spans: [UInt32(i + 1) * 10_000 + 2], col: 5))
            XCTAssertEqual(place.page, i)
            XCTAssertLessThanOrEqual(s.glyphIndexes.count, EngineV3GlyphIndexes.defaultCapacity)
        }
        XCTAssertLessThanOrEqual(s.glyphIndexes.builds, n + 1)
        XCTAssertEqual(s.glyphIndexes.count, EngineV3GlyphIndexes.defaultCapacity)
    }

    /// The answers are the full scan's: the first page with the line's
    /// glyphs (a line over a page break: the first), at the column.
    func testPlaceAgreesWithAFullScan() throws {
        // Page 11 starts with page 10's spans (a paragraph over the break).
        let s = session(pages: 24) { i in i == 11 ? 11 * 10_000 : UInt32(i + 1) * 10_000 }
        var checked = 0
        for i in [0, 3, 10, 11, 12, 23] {
            for k: UInt32 in [0, 1, 3] {
                for col in [nil, 0, 7, 39, 200] as [Int?] {
                    let spans: Set<UInt32> = [UInt32(i + 1) * 10_000 + k]
                    let want = Self.fullScan(s, spans, col: col)
                    XCTAssertEqual(s.place(spans: spans, col: col), want, "page \(i) span +\(k) col \(String(describing: col))")
                    checked += want == nil ? 0 : 1
                }
            }
        }
        XCTAssertGreaterThan(checked, 60)
        XCTAssertEqual(s.place(spans: [11 * 10_000], col: nil)?.page, 10, "the first page that shows the line")
        XCTAssertNil(s.place(spans: [999], col: nil))
        // Two spans of one line, on two pages.
        XCTAssertEqual(s.place(spans: [5 * 10_000 + 1, 20 * 10_000], col: nil), Self.fullScan(s, [5 * 10_000 + 1, 20 * 10_000], col: nil))
    }

    /// A page sent again is indexed again: its old spans and places are gone.
    func testAChangedPageIsIndexedAgain() throws {
        let s = session(pages: 6)
        let before = try XCTUnwrap(s.place(spans: [40_000], col: 3))
        XCTAssertEqual(before.page, 3)
        s.handle(.page(Self.page(3, spanBase: 77_000, shift: 657_818), compileID: 2, timing: .init(), image: nil))
        XCTAssertNil(s.place(spans: [40_000], col: 3), "the old spans are not on the new page")
        let after = try XCTUnwrap(s.place(spans: [77_000], col: 3))
        XCTAssertEqual(after.page, 3)
        XCTAssertEqual(after.rect.minX, before.rect.minX + 10, accuracy: 0.01, "the new page's glyphs, 10 bp to the right")
        XCTAssertEqual(after, Self.fullScan(s, [77_000], col: 3))
    }

    /// A shorter document (a complete count at DONE) drops the indexes of the pages that went.
    func testAShorterDocumentDropsItsIndexes() throws {
        let s = session(pages: 30)
        XCTAssertEqual(s.place(spans: [26 * 10_000], col: nil)?.page, 25)
        XCTAssertEqual(s.place(spans: [3 * 10_000], col: nil)?.page, 2)
        XCTAssertNotNil(s.sourceIndex(page: 25))
        s.handle(.done(.object(["status": .string("ok"), "pages": .int(10)]), compileID: 1))
        XCTAssertEqual(s.pageCount, 10)
        XCTAssertEqual(s.glyphIndexes.indexedPages, [2])
        XCTAssertLessThanOrEqual(s.glyphIndexes.summarized, 10)
        XCTAssertNil(s.sourceIndex(page: 25))
        XCTAssertNil(s.place(spans: [26 * 10_000], col: nil))
    }

    /// Another project in the window (or a stop): no lookup answers with
    /// the last project's glyphs, even for span ids the new one reuses (ids
    /// restart with each connection's resources).
    func testAnotherProjectNeverAnswersWithTheOldProjectsGlyphs() throws {
        for how in ["another project", "stop"] {
            let s = session(pages: 5) { UInt32($0 + 1) * 100 }
            let old = try XCTUnwrap(s.place(spans: [100], col: 2))
            for i in 0 ..< 5 { XCTAssertNotNil(s.sourceIndex(page: i)) }
            let mid = CGPoint(x: old.rect.midX, y: old.rect.midY)
            if how == "stop" { s.stop() } else { s.dropPagesForTesting() }
            XCTAssertEqual(s.glyphIndexes.count, 0, how)
            XCTAssertEqual(s.glyphIndexes.summarized, 0, how)
            for i in 0 ..< 5 { XCTAssertNil(s.sourceIndex(page: i), "\(how): page \(i)") }
            XCTAssertNil(s.source(page: 0, at: mid), how)
            XCTAssertNil(s.place(spans: [100], col: 2), how)
            // The new project's first page, with the same span ids, 20 bp to the right.
            s.handle(.page(Self.page(0, spanBase: 100, shift: 1_315_635), compileID: 1, timing: .init(), image: nil))
            let new = try XCTUnwrap(s.place(spans: [100], col: 2))
            XCTAssertEqual(new.page, 0)
            XCTAssertEqual(new.rect.minX, old.rect.minX + 20, accuracy: 0.01, "\(how): the new project's glyphs")
            for i in 1 ..< 5 { XCTAssertNil(s.sourceIndex(page: i), "\(how): page \(i) is not the new project's") }
            s.stop()
        }
    }

    func testTheCacheIsLeastRecentlyUsedFirstOut() {
        var c = EngineV3GlyphIndexes(capacity: 3)
        let p = Self.page(0, spanBase: 1)
        for i in 0 ..< 3 { _ = c.index(i, of: p) }
        _ = c.cached(0) // 0 used most recently: 1 goes next
        _ = c.index(3, of: p)
        XCTAssertEqual(c.indexedPages, [0, 2, 3])
        XCTAssertEqual(c.builds, 4)
        c.invalidate(2)
        XCTAssertEqual(c.indexedPages, [0, 3])
        for i in 4 ..< 9 { _ = c.index(i, of: p) }
        XCTAssertEqual(c.indexedPages, [6, 7, 8])
        c.trim(keeping: [7, 42])
        XCTAssertEqual(c.indexedPages, [7])
        c.removePages(from: 7)
        XCTAssertEqual(c.count, 0)
        XCTAssertEqual(EngineV3GlyphIndexes.glyphSpans(p.page).first, 1)
        XCTAssertTrue(EngineV3GlyphIndexes.contains([1, 5, 9], 9))
        XCTAssertFalse(EngineV3GlyphIndexes.contains([1, 5, 9], 6))
        XCTAssertFalse(EngineV3GlyphIndexes.contains([], 0))
    }

    // MARK: cost (A/B in one run, the previous algorithm reimplemented here)

    /// Forward search to the last of 300 pages: the cold search (after a
    /// recompile) and a warm one, against the unbounded cache it replaces,
    /// and the bytes each holds. Thread CPU time, interleaved; a debug
    /// build, so the ratio is the evidence, not the microseconds.
    func testPlaceCostAgainstTheUnboundedCache() throws {
        let n = 300
        let s = session(pages: n)
        let target: Set<UInt32> = [UInt32(n) * 10_000 + 3]
        var old: [Int: DL3SourceIndex] = [:]
        func oldPlace(_ spans: Set<UInt32>, col: Int?) -> EngineV3Place? {
            for i in 0 ..< s.pageCount {
                let ix: DL3SourceIndex
                if let hit = old[i] { ix = hit } else {
                    guard let p = s.pages[i] else { continue }
                    ix = DL3SourceIndex(p); old[i] = ix
                }
                if let box = DL3SourceIndex.box(ix.glyphs(of: spans, col: col)) { return EngineV3Place(page: i, rect: box) }
            }
            return nil
        }
        func cpu(_ f: () -> EngineV3Place?) -> (Double, EngineV3Place?) {
            let t0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            let r = f()
            return (Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - t0) / 1_000, r)
        }
        let (oldCold, a) = cpu { oldPlace(target, col: 9) }
        let (newCold, b) = cpu { s.place(spans: target, col: 9) }
        XCTAssertEqual(a, b)
        XCTAssertEqual(b?.page, n - 1)
        var oldWarm: [Double] = [], newWarm: [Double] = []
        for k in 0 ..< 101 {
            let col = k % 40
            let (o, ra) = cpu { oldPlace(target, col: col) }
            let (w, rb) = cpu { s.place(spans: target, col: col) }
            XCTAssertEqual(ra, rb)
            oldWarm.append(o); newWarm.append(w)
        }
        oldWarm.sort(); newWarm.sort()
        let glyphBytes = MemoryLayout<DL3GlyphRef>.stride + MemoryLayout<Int32>.stride
        let oldBytes = old.values.reduce(0) { $0 + $1.glyphs.count * glyphBytes }
        let heldBytes = (0 ..< n).compactMap { s.glyphIndexes.indexedPages.contains($0) ? s.sourceIndex(page: $0) : nil }
            .reduce(0) { $0 + $1.glyphs.count * glyphBytes }
        let summaryBytes = (0 ..< n).compactMap { s.pages[$0] }.reduce(0) { $0 + EngineV3GlyphIndexes.glyphSpans($1.page).count * 4 }
        let glyphs = DL3SourceIndex(Self.page(0, spanBase: 1)).glyphs.count
        print(String(format: "glyph-index bench (%ld pages, %ld glyphs a page, %ld B a glyph): cold place %.0f µs (unbounded cache %.0f µs); "
                     + "warm p50 %.1f µs (%.1f µs), p95 %.1f µs (%.1f µs); held %ld indexes %.2f MB + summaries %.3f MB (unbounded %ld indexes %.2f MB)",
                     n, glyphs, glyphBytes, newCold, oldCold, newWarm[50], oldWarm[50], newWarm[95], oldWarm[95],
                     s.glyphIndexes.count, Double(heldBytes) / 1e6, Double(summaryBytes) / 1e6, old.count, Double(oldBytes) / 1e6))
        XCTAssertEqual(s.glyphIndexes.count, 1)
        XCTAssertEqual(old.count, n)
        XCTAssertLessThan(newCold, oldCold, "a cold search builds one index, not \(n)")
        XCTAssertLessThan(newWarm[50], oldWarm[50] * 2 + 20, "a warm search is no slower (loose: a shared runner)")
    }

    // MARK: memory pressure

    func testMemoryPressureTrimsTheIndexesAndAsksTheHostToTrim() throws {
        let host = try StandInHost(capabilities: [DL3.trimCapability, "progress-v1"])
        let c = try DL3Connection(socketPath: host.path, client: "test")
        let s = session(pages: 20)
        s.adoptForTesting(c)
        XCTAssertTrue(s.hostOffersTrim)
        for i in [3, 9, 15] { XCTAssertEqual(s.place(spans: [UInt32(i + 1) * 10_000], col: nil)?.page, i) }
        XCTAssertEqual(s.glyphIndexes.count, 3)
        let pressure = EngineV3MemoryPressure(listens: false)
        pressure.register(s)
        pressure.register(s) // once only
        XCTAssertEqual(pressure.registered, 1)
        pressure.handle(.warning)
        XCTAssertEqual(s.pressureEvents, 1)
        XCTAssertEqual(s.glyphIndexes.count, 0, "no pane holds a page: every index goes")
        XCTAssertEqual(s.glyphIndexes.summarized, 16, "the span summaries (small) of pages 0-15, searched, stay")
        XCTAssertEqual(DL3ResourceCache.shared.heldImages.count, 0)
        XCTAssertEqual(s.place(spans: [10 * 10_000], col: nil)?.page, 9, "built again on the next lookup")
        pressure.handle(.critical)
        XCTAssertEqual(s.trimsSent, 2)
        let frames = host.wait(for: 2)
        XCTAssertEqual(frames.map { $0.kind }, [DL3.Kind.trim, DL3.Kind.trim])
        XCTAssertEqual(frames.map { (try? DL3JSON.parse($0.body))?["level"]?.string }, ["warning", "critical"])
        XCTAssertEqual(DL3.Kind.name(DL3.Kind.trim), "trim")
        XCTAssertEqual(DL3.Kind.trim, 0x08)
        pressure.unregister(s)
        pressure.handle(.warning)
        XCTAssertEqual(s.pressureEvents, 2, "an unregistered session is not trimmed")
        s.stop()
        XCTAssertEqual(host.wait(for: 3).map { $0.kind }, [DL3.Kind.trim, DL3.Kind.trim, DL3.Kind.bye])
    }

    func testNoTrimToAHostWithoutTrimV1() throws {
        let host = try StandInHost(capabilities: ["progress-v1"])
        let c = try DL3Connection(socketPath: host.path, client: "test")
        let s = session(pages: 2)
        s.adoptForTesting(c)
        XCTAssertFalse(s.hostOffersTrim)
        let pressure = EngineV3MemoryPressure(listens: false)
        pressure.register(s)
        pressure.handle(.warning)
        pressure.handle(.critical)
        XCTAssertEqual(s.pressureEvents, 2)
        XCTAssertEqual(s.trimsSent, 0)
        s.stop()
        XCTAssertEqual(host.wait(for: 1).map { $0.kind }, [DL3.Kind.bye], "nothing but the BYE")
    }

    /// The app-wide handler follows the sessions that run.
    func testRunningSessionsRegisterWithTheAppWideHandler() {
        let s = EngineV3Session(smoothFonts: false)
        let before = EngineV3MemoryPressure.shared.registered
        EngineV3MemoryPressure.shared.register(s)
        XCTAssertEqual(EngineV3MemoryPressure.shared.registered, before + 1)
        s.stop()
        XCTAssertEqual(EngineV3MemoryPressure.shared.registered, before)
    }
}

/// A display-list-v3 host stand-in on a Unix socket: answers the client's
/// HELLO with `capabilities` and records every later frame.
final class StandInHost: @unchecked Sendable {
    let path: String
    private let listener: Int32
    private let lock = NSLock()
    private var got: [(kind: UInt8, body: [UInt8])] = []

    init(capabilities: [String]) throws {
        path = "/tmp/ftx-standin-\(getpid())-\(UInt32.random(in: 0 ... .max)).sock"
        unlink(path)
        let fd = socket(AF_UNIX, Int32(SOCK_STREAM), 0)
        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        let bytes = Array(path.utf8)
        withUnsafeMutableBytes(of: &addr.sun_path) { raw in raw.copyBytes(from: bytes); raw[bytes.count] = 0 }
        let bound = withUnsafePointer(to: &addr) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) { bind(fd, $0, socklen_t(MemoryLayout<sockaddr_un>.size)) }
        }
        guard fd >= 0, bound == 0, listen(fd, 1) == 0 else { close(fd); throw DL3Error("stand-in host: bind/listen errno \(errno)") }
        listener = fd
        let hello: DL3JSON = .object(["protocol": .string(DL3.protocolName),
                                      "version": .array([.int(Int64(DL3.versionMajor)), .int(Int64(DL3.versionMinor))]),
                                      "server": .string("stand-in"), "engine": .string("none"),
                                      "capabilities": .array(capabilities.map(DL3JSON.string))])
        let t = Thread { [self] in
            let c = accept(fd, nil, nil)
            guard c >= 0 else { return }
            defer { close(c) }
            guard Self.readFrame(c) != nil else { return } // the client's HELLO
            let out = DL3Frames.encode(kind: DL3.Kind.hello, body: hello.data())
            _ = out.withUnsafeBytes { write(c, $0.baseAddress!, $0.count) }
            while let f = Self.readFrame(c) { lock.lock(); got.append(f); lock.unlock() }
        }
        t.start()
    }

    deinit { close(listener); unlink(path) }

    /// The frames after HELLO once there are `n` (or after 5 s).
    func wait(for n: Int) -> [(kind: UInt8, body: [UInt8])] {
        let end = Date().addingTimeInterval(5)
        while Date() < end {
            lock.lock(); let have = got; lock.unlock()
            if have.count >= n { return have }
            Thread.sleep(forTimeInterval: 0.01)
        }
        lock.lock(); defer { lock.unlock() }
        return got
    }

    private static func readFrame(_ fd: Int32) -> (kind: UInt8, body: [UInt8])? {
        func read(_ n: Int) -> [UInt8]? {
            var buf = [UInt8](repeating: 0, count: n), off = 0
            while off < n {
                let r = buf.withUnsafeMutableBytes { Darwin.read(fd, $0.baseAddress! + off, n - off) }
                if r < 0, errno == EINTR { continue }
                guard r > 0 else { return nil }
                off += r
            }
            return buf
        }
        guard let head = read(5) else { return nil }
        let len = Int(UInt32(head[0]) | UInt32(head[1]) << 8 | UInt32(head[2]) << 16 | UInt32(head[3]) << 24)
        guard len >= 1 else { return nil }
        if len == 1 { return (head[4], []) }
        guard let body = read(len - 1) else { return nil }
        return (head[4], body)
    }
}
