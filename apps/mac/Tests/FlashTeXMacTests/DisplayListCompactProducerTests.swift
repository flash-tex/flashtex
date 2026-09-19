import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// `display-list-v2-compact` consumer gate against the REAL producer
/// (`FLASHTEX_RENDER`, a `flashtex-render` carrying the capability; skips
/// finitely otherwise, like `DisplayListDeltaTests`):
///
/// 1. The compact sibling and the full sibling of the same request decode to
///    the same `RenderingV2` values — every derived caret and hit rect equal
///    within 1e-6 bp (they are integer ticks: exactly equal) — and the compact
///    line is a fraction of the full one.
/// 2. The delta chain runs under the compact encoding: reconstruction equals
///    the fresh compact decode, `page_bytes` equal the fresh line's raw page
///    ranges (the run-level width rule), the exact target equals the fresh
///    line length (compact frame constant), and the parity gate holds.
/// 3. A delta in the other encoding than the installed base is refused typed.
final class DisplayListCompactProducerTests: XCTestCase {
    typealias Worker = DisplayListDeltaTests.Worker

    static func request(_ text: String, revision: Int, compact: Bool, delta: Bool, ack: RuntimeV1.CompileRequest.DisplayListBase?) -> RuntimeV1.CompileRequest {
        var caps = ["rules-v1", "font-hints-v1", "display-list-v2"]
        if compact { caps.append(DisplayListCompact.capability) }
        if delta { caps.append(DisplayListDelta.capability) }
        return RuntimeV1.CompileRequest(projectId: "compact-gate", revision: revision, entryPath: "main.tex",
                                        documents: [.init(path: "main.tex", text: text)], layoutCapabilities: caps, displayListBase: delta ? ack : nil)
    }

    /// A producer that speaks the capability, or a finite skip.
    func worker() throws -> Worker {
        let environment = ProcessInfo.processInfo.environment
        guard let path = environment["FLASHTEX_RENDER"] ?? environment["FLASHTEX_COMPILER"], FileManager.default.isExecutableFile(atPath: path) else { throw XCTSkip("set FLASHTEX_RENDER to a flashtex-render with display-list-v2-compact") }
        let url = URL(fileURLWithPath: path)
        if url.lastPathComponent == "flashtex-compiler" { throw XCTSkip("FLASHTEX_COMPILER is the plain compiler") }
        guard V2FontStore.shared.fonts.contains(where: { $0.url.lastPathComponent.hasPrefix("lmroman12") }) else { throw XCTSkip("Latin Modern not bundled") }
        let w = try Worker(url)
        try w.send(Self.request(DisplayListDeltaTests.text0, revision: 1, compact: true, delta: false, ack: nil), id: "probe")
        let line: Data
        do { line = try w.readLine(timeout: 20) } catch { throw XCTSkip("producer did not answer within 20 s") }
        guard let result = try? RuntimeV1.decodeCompileResult(line), result.id == "probe" else { throw XCTSkip("no compile_result") }
        let echo = result.payload.layoutCapabilities ?? []
        guard echo.contains("display-list-v2") else { throw XCTSkip("producer does not offer display-list-v2") }
        let sibling: Data
        do { sibling = try w.readLine(timeout: 20) } catch { throw XCTSkip("no sibling within 20 s") }
        guard echo.contains(DisplayListCompact.capability) else { throw XCTSkip("producer does not offer \(DisplayListCompact.capability) (echoed \(echo)); build flashtex-render from this branch") }
        XCTAssertEqual(RenderingV2Fast.header(sibling)?.type, "display_list")
        return w
    }

    func exchange(_ w: Worker, _ req: RuntimeV1.CompileRequest, id: String) throws -> (echo: [String], sibling: Data) {
        try w.send(req, id: id)
        let result = try w.readLine(timeout: 60)
        let echo = try RuntimeV1.decodeCompileResult(result).payload.layoutCapabilities ?? []
        guard echo.contains("display-list-v2") else { return (echo, Data()) }
        return (echo, try w.readLine(timeout: 60))
    }

    func testCompactAndFullSiblingsDecodeToTheSameFrame() throws {
        let w = try worker()
        var compactBytes = 0, fullBytes = 0, clusters = 0
        for (k, text) in ([DisplayListDeltaTests.text0] + DisplayListDeltaTests.edits()).enumerated() {
            let (echoC, compactLine) = try exchange(w, Self.request(text, revision: 10 + k, compact: true, delta: false, ack: nil), id: "c-\(k)")
            let (echoF, fullLine) = try exchange(w, Self.request(text, revision: 10 + k, compact: false, delta: false, ack: nil), id: "f-\(k)")
            XCTAssertTrue(echoC.contains(DisplayListCompact.capability)); XCTAssertFalse(echoF.contains(DisplayListCompact.capability))
            let compact = try RenderingV2.decode(compactLine), full = try RenderingV2.decode(fullLine)
            XCTAssertEqual(compact.payload.clusterEncoding, DisplayListCompact.encoding); XCTAssertNil(full.payload.clusterEncoding)
            var normalised = compact.payload; normalised.clusterEncoding = nil
            XCTAssertEqual(normalised, full.payload, "edit \(k): the compact decode is the full decode")
            for (pc, pf) in zip(compact.payload.pages, full.payload.pages) {
                for (a, b) in zip(pc.items, pf.items) {
                    guard case .glyphRun(let rc) = a, case .glyphRun(let rf) = b else { continue }
                    for (cc, cf) in zip(rc.clusters, rf.clusters) {
                        clusters += 1
                        XCTAssertEqual(cc.carets.count, cf.carets.count)
                        for (kc, kf) in zip(cc.carets, cf.carets) {
                            XCTAssertEqual(kc.textByte, kf.textByte)
                            XCTAssertEqual(RenderingV2.points(kc.x), RenderingV2.points(kf.x), accuracy: 1e-6)
                            XCTAssertEqual(RenderingV2.points(kc.top), RenderingV2.points(kf.top), accuracy: 1e-6)
                            XCTAssertEqual(RenderingV2.points(kc.height), RenderingV2.points(kf.height), accuracy: 1e-6)
                        }
                        XCTAssertEqual(cc.hitRects.count, cf.hitRects.count)
                        for (hc, hf) in zip(cc.hitRects, cf.hitRects) {
                            XCTAssertEqual(RenderingV2.points(hc.x), RenderingV2.points(hf.x), accuracy: 1e-6)
                            XCTAssertEqual(RenderingV2.points(hc.top), RenderingV2.points(hf.top), accuracy: 1e-6)
                            XCTAssertEqual(RenderingV2.points(hc.width), RenderingV2.points(hf.width), accuracy: 1e-6)
                            XCTAssertEqual(RenderingV2.points(hc.height), RenderingV2.points(hf.height), accuracy: 1e-6)
                        }
                    }
                }
            }
            // The compact frame prepares and paints exactly as the full one (page cache path included).
            let frame = try V2Frame.prepare(data: compactLine)
            XCTAssertEqual(frame.list.pages.count, full.payload.pages.count)
            XCTAssertEqual(V2Parity.compare(frame: frame, scale: 1).totalDifferingPixels, 0, "edit \(k): parity on the compact frame")
            compactBytes += compactLine.count; fullBytes += fullLine.count
        }
        XCTAssertGreaterThan(clusters, 1000)
        XCTAssertLessThan(compactBytes * 2, fullBytes, "compact \(compactBytes) B vs full \(fullBytes) B")
        print("compact-gate: \(clusters) clusters, compact \(compactBytes) B vs full \(fullBytes) B (\(100 * compactBytes / fullBytes)%)")
    }

    func testDeltaChainUnderTheCompactEncoding() throws {
        let a = try worker(), b = try worker()
        // Install a compact full frame on worker A.
        let (_, first) = try exchange(a, Self.request(DisplayListDeltaTests.text0, revision: 20, compact: true, delta: true, ack: nil), id: "d-0")
        let (env0, pb0) = try RenderingV2Fast.envelopeWithPageBytes(first)
        try RenderingV2.validate(env0.payload)
        XCTAssertEqual(env0.payload.clusterEncoding, DisplayListCompact.encoding)
        var installed = try XCTUnwrap(DisplayListDelta.installed(from: env0, pageBytes: pb0, lineBytes: first.count))
        var deltas = 0
        for (k, text) in DisplayListDeltaTests.edits().enumerated() {
            let id = "d-\(k + 1)"
            let (echo, sibling) = try exchange(a, Self.request(text, revision: 21 + k, compact: true, delta: true, ack: installed.acknowledgement), id: id)
            XCTAssertTrue(echo.contains(DisplayListDelta.capability), "\(id): delta echoed (\(echo))")
            XCTAssertTrue(echo.contains(DisplayListCompact.capability), "\(id): compact echoed (\(echo))")
            XCTAssertEqual(RenderingV2Fast.header(sibling)?.type, DisplayListDelta.messageType)
            let d = try RenderingV2Fast.delta(sibling, maxPages: DisplayListDelta.maxSnapshotPages)
            XCTAssertEqual(d.clusterEncoding, DisplayListCompact.encoding)
            let (_, freshLine) = try exchange(b, Self.request(text, revision: 21 + k, compact: true, delta: false, ack: nil), id: id)
            let (fresh, fpb) = try RenderingV2Fast.envelopeWithPageBytes(freshLine)
            let (env, pageBytes, target) = try DisplayListDelta.apply(d, to: installed)
            XCTAssertEqual(env.payload, fresh.payload, "\(id): reconstruction differs from the fresh compact decode")
            XCTAssertEqual(pageBytes, fpb, "\(id): page_bytes vs the fresh compact line's raw page ranges")
            XCTAssertEqual(target, freshLine.count, "\(id): exact target size vs the fresh compact line length")
            XCTAssertLessThan(sibling.count, freshLine.count)
            try RenderingV2.validate(env.payload)
            let frame = try V2Frame.prepare(env)
            XCTAssertEqual(V2Parity.compare(frame: frame, scale: 1).totalDifferingPixels, 0, "\(id): parity")
            XCTAssertGreaterThan(d.pageCount - d.changedPages.count, 0, "\(id): at least one page was relocated under the compact width rule")
            installed = try XCTUnwrap(DisplayListDelta.installed(from: env, pageBytes: pageBytes, lineBytes: target))
            deltas += 1
        }
        XCTAssertEqual(deltas, DisplayListDeltaTests.edits().count)
        // A full-encoding delta against a compact base is refused before any page is built.
        let (echo, sibling) = try exchange(a, Self.request(DisplayListDeltaTests.text0, revision: 40, compact: false, delta: true, ack: installed.acknowledgement), id: "d-x")
        if echo.contains(DisplayListDelta.capability) {
            // The producer's snapshot is keyed on the wire options, so it should answer full; if it ever
            // emitted a delta here, the consumer refuses it typed.
            let d = try RenderingV2Fast.delta(sibling, maxPages: DisplayListDelta.maxSnapshotPages)
            XCTAssertThrowsError(try DisplayListDelta.apply(d, to: installed)) { e in
                if case .encodingMismatch? = e as? DisplayListDelta.Refusal {} else { XCTFail("\(e)") }
            }
        } else {
            XCTAssertEqual(RenderingV2Fast.header(sibling)?.type, "display_list", "a changed wire option answers full")
        }
    }

    func testEncodingMismatchIsATypedRefusal() throws {
        let a = try worker()
        let (_, first) = try exchange(a, Self.request(DisplayListDeltaTests.text0, revision: 50, compact: true, delta: true, ack: nil), id: "e-0")
        let (env0, pb0) = try RenderingV2Fast.envelopeWithPageBytes(first)
        let installed = try XCTUnwrap(DisplayListDelta.installed(from: env0, pageBytes: pb0, lineBytes: first.count))
        let (_, sibling) = try exchange(a, Self.request(DisplayListDeltaTests.edits()[0], revision: 51, compact: true, delta: true, ack: installed.acknowledgement), id: "e-1")
        var d = try RenderingV2Fast.delta(sibling, maxPages: DisplayListDelta.maxSnapshotPages)
        XCTAssertNoThrow(try DisplayListDelta.apply(d, to: installed))
        d.clusterEncoding = nil
        XCTAssertThrowsError(try DisplayListDelta.apply(d, to: installed)) { e in
            if case .encodingMismatch? = e as? DisplayListDelta.Refusal {} else { XCTFail("\(e)") }
        }
        d.clusterEncoding = "compact-9"
        XCTAssertThrowsError(try DisplayListDelta.apply(d, to: installed))
    }
}
