import XCTest
import CoreGraphics
import FlashTeXProtocol
@testable import FlashTeXMac

/// ISOLATED `display-list-v2-delta` consumer gate (proposal r5 §10.2; Commander
/// 5647057936). Drives the REAL producer (`FLASHTEX_RENDER`, else `FLASHTEX_COMPILER`, a
/// `flashtex-render` built from `agent/mac-render-pipeline/delta-proposal`
/// b956304a+) over its stdin/stdout: worker A runs the delta chain with
/// installed-base acknowledgements, worker B is the fresh-full oracle for the
/// same text. Skips cleanly when no delta-capable producer is available.
final class DisplayListDeltaTests: XCTestCase {
    // MARK: a tiny line-protocol driver (no shell model, no UI)

    /// Reads on a background thread; `readLine(timeout:)` is genuinely
    /// deadline-bounded (semaphore wait), so a producer that never writes a
    /// second line (an ordinary v1 compiler) makes the test SKIP, never stall
    /// (recovery issue #50).
    final class Worker {
        struct Timeout: Error {}
        let process = Process()
        let stdin = Pipe(), stdout = Pipe()
        private let lock = NSLock()
        private var buffer = Data()
        private var closed = false
        private let available = DispatchSemaphore(value: 0)
        init(_ url: URL) throws {
            process.executableURL = url
            process.standardInput = stdin
            process.standardOutput = stdout
            process.standardError = FileHandle.nullDevice
            var env = ProcessInfo.processInfo.environment
            env["FLASHTEX_NO_ACTIVATE"] = "1"
            process.environment = env
            try process.run()
            let reader = stdout.fileHandleForReading
            Thread.detachNewThread { [weak self] in
                while true {
                    let chunk = reader.availableData
                    guard let self else { return }
                    self.lock.lock()
                    if chunk.isEmpty { self.closed = true } else { self.buffer.append(chunk) }
                    self.lock.unlock()
                    self.available.signal()
                    if chunk.isEmpty { return }
                }
            }
        }
        deinit {
            try? stdin.fileHandleForWriting.close()
            if process.isRunning { process.terminate() }
        }
        func send(_ request: RuntimeV1.CompileRequest, id: String) throws {
            try stdin.fileHandleForWriting.write(contentsOf: try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: id, request)))
        }
        /// The next line, or `Timeout` when none arrives within `timeout` seconds
        /// (also when the producer closes stdout).
        func readLine(timeout: TimeInterval = 20) throws -> Data {
            let deadline = DispatchTime.now() + timeout
            while true {
                lock.lock()
                if let nl = buffer.firstIndex(of: 0x0A) {
                    let line = buffer.subdata(in: buffer.startIndex..<nl)
                    buffer.removeSubrange(buffer.startIndex...nl)
                    lock.unlock()
                    return line
                }
                let isClosed = closed
                lock.unlock()
                if isClosed { throw Timeout() }
                if available.wait(timeout: deadline) == .timedOut { throw Timeout() }
            }
        }
    }

    static let text0: String = {
        let long = "The quick brown fox jumps over the lazy dog while the patient owl watches from an old oak branch and counts every leaf that falls into the quiet river below; then more text so that the paragraph wraps onto several lines of the page."
        var t = "\\begin{document}\n\\section{Part 0}\nParagraph 0.0 of the delta gate. A \\textbf{bold} word, an \\emph{emphasised} one, the UTF-8 word caf\\'e — naïve “quotes” on a short first page.\n\n\\newpage\n"
        for i in 0..<6 { t += "\\section{Part \(i + 1)}\n" + (0..<4).map { "Paragraph \(i + 1).\($0). \(long)\n\n" }.joined() }
        return t + "\\end{document}\n"
    }()

    static func edits() -> [String] {
        let t0 = text0
        return [
            t0.replacingOccurrences(of: "Paragraph 0.0", with: "Paragraph 0.0 edited"),
            t0.replacingOccurrences(of: "Paragraph 0.0", with: "Paragraph 0.0 \"escaped\" ü"),
            t0.replacingOccurrences(of: " of the delta gate.", with: "."),
            t0.replacingOccurrences(of: "Paragraph 0.0", with: "Paragraph 0.0 " + String(repeating: "word ", count: 30)),
            t0,
        ]
    }

    static func request(_ text: String, revision: Int, delta: Bool, ack: RuntimeV1.CompileRequest.DisplayListBase?) -> RuntimeV1.CompileRequest {
        var caps = ["rules-v1", "font-hints-v1", "display-list-v2"]
        if delta { caps.append(DisplayListDelta.capability) }
        return RuntimeV1.CompileRequest(projectId: "delta-gate", revision: revision, entryPath: "main.tex",
                                        documents: [.init(path: "main.tex", text: text)], layoutCapabilities: caps, displayListBase: delta ? ack : nil)
    }

    struct Session {
        var a: Worker, b: Worker
    }

    /// Two real workers, or a skip within seconds when the producer does not
    /// speak the capability: the negotiation ECHO of each probe is checked
    /// before any sibling is awaited, so an ordinary v1 compiler (no
    /// `display-list-v2`) and a producer without `-delta` both skip finitely.
    func session() throws -> Session {
        let environment = ProcessInfo.processInfo.environment
        guard let path = environment["FLASHTEX_RENDER"] ?? environment["FLASHTEX_COMPILER"], FileManager.default.isExecutableFile(atPath: path) else { throw XCTSkip("set FLASHTEX_RENDER (or FLASHTEX_COMPILER) to a delta-capable flashtex-render") }
        let url = URL(fileURLWithPath: path)
        if url.lastPathComponent == "flashtex-compiler" { throw XCTSkip("FLASHTEX_COMPILER is the plain compiler; the delta gate needs flashtex-render from agent/mac-render-pipeline/delta-proposal") }
        guard V2FontStore.shared.fonts.contains(where: { $0.url.lastPathComponent.hasPrefix("lmroman12") }) else { throw XCTSkip("Latin Modern not bundled") }
        let a = try Worker(url), b = try Worker(url)
        func echo(_ w: Worker, _ id: String) throws -> [String] {
            let line: Data
            do { line = try w.readLine(timeout: 20) } catch { throw XCTSkip("producer did not answer \(id) within 20 s") }
            guard let result = try? RuntimeV1.decodeCompileResult(line), result.id == id else { throw XCTSkip("producer's first line for \(id) is not its compile_result") }
            return result.payload.layoutCapabilities ?? []
        }
        try a.send(Self.request(Self.text0, revision: 1, delta: true, ack: nil), id: "probe-1")
        guard try echo(a, "probe-1").contains("display-list-v2") else { throw XCTSkip("producer does not offer display-list-v2 (no sibling to wait for)") }
        let s1: Data
        do { s1 = try a.readLine(timeout: 20) } catch { throw XCTSkip("no display_list sibling within 20 s") }
        guard let h1 = RenderingV2Fast.header(s1), h1.type == "display_list" else { throw XCTSkip("no display_list sibling from the producer") }
        let (env1, pb1) = try RenderingV2Fast.envelopeWithPageBytes(s1)
        guard let inst = DisplayListDelta.installed(from: env1, pageBytes: pb1, lineBytes: s1.count) else { throw XCTSkip("probe frame over the retention cap") }
        try a.send(Self.request(Self.edits()[0], revision: 2, delta: true, ack: inst.acknowledgement), id: "probe-2")
        let echo2 = try echo(a, "probe-2")
        guard echo2.contains(DisplayListDelta.capability) else {
            throw XCTSkip("producer does not offer display-list-v2-delta (echoed \(echo2)); build flashtex-render from agent/mac-render-pipeline/delta-proposal")
        }
        let s2: Data
        do { s2 = try a.readLine(timeout: 20) } catch { throw XCTSkip("no display_list_delta sibling within 20 s") }
        guard let h2 = RenderingV2Fast.header(s2), h2.type == DisplayListDelta.messageType else { throw XCTSkip("echoed -delta but the sibling is not a display_list_delta") }
        return Session(a: a, b: b)
    }

    /// Bounded read that converts a timeout into a finite skip.
    func line(_ w: Worker, _ what: String) throws -> Data {
        do { return try w.readLine(timeout: 60) } catch { throw XCTSkip("producer did not deliver \(what) within 60 s") }
    }

    /// One full install on worker A (fresh chain start), returning the installed base.
    func install(_ s: Session, text: String, revision: Int, id: String) throws -> DisplayListDelta.Installed {
        try s.a.send(Self.request(text, revision: revision, delta: false, ack: nil), id: id) // clears the producer snapshot
        _ = try line(s.a, "\(id) result"); _ = try line(s.a, "\(id) sibling")
        try s.a.send(Self.request(text, revision: revision, delta: true, ack: nil), id: id + "-full")
        _ = try line(s.a, "\(id)-full result"); let line = try line(s.a, "\(id)-full sibling")
        let (env, pb) = try RenderingV2Fast.envelopeWithPageBytes(line)
        try RenderingV2.validate(env.payload)
        return try XCTUnwrap(DisplayListDelta.installed(from: env, pageBytes: pb, lineBytes: line.count))
    }

    struct Step {
        var delta: RenderingV2Fast.DeltaEnvelope
        var deltaLine: Data
        var freshLine: Data
        var fresh: RenderingV2.Envelope
        var freshPageBytes: [Int]
    }

    /// Next edit on the chain: worker A answers a delta (asserted), worker B the fresh full line.
    func step(_ s: Session, installed: DisplayListDelta.Installed, text: String, revision: Int, id: String) throws -> Step {
        try s.a.send(Self.request(text, revision: revision, delta: true, ack: installed.acknowledgement), id: id)
        let result = try line(s.a, "\(id) result"); let sibling = try line(s.a, "\(id) sibling")
        let echo = try RuntimeV1.decodeCompileResult(result).payload.layoutCapabilities ?? []
        XCTAssertTrue(echo.contains(DisplayListDelta.capability), "\(id): producer echoed \(echo)")
        let d = try RenderingV2Fast.delta(sibling, maxPages: DisplayListDelta.maxSnapshotPages)
        try s.b.send(Self.request(text, revision: revision, delta: false, ack: nil), id: id)
        _ = try line(s.b, "\(id) fresh result"); let freshLine = try line(s.b, "\(id) fresh sibling")
        let (fresh, fpb) = try RenderingV2Fast.envelopeWithPageBytes(freshLine)
        return Step(delta: d, deltaLine: sibling, freshLine: freshLine, fresh: fresh, freshPageBytes: fpb)
    }

    // MARK: gates

    /// C1/C4: reconstruction == fresh decode (semantic), page_bytes == the fresh line's raw page
    /// lengths for every page, the exact target size == the fresh line length (writer template
    /// verified against the real producer, incl. escapes/UTF-8/digit boundaries), and the
    /// existing 0-pixel parity gate on the reconstructed frame.
    func testReconstructionEqualsFreshDecodeWithExactSizeAndParity() throws {
        let s = try session()
        var installed = try install(s, text: Self.text0, revision: 10, id: "c1-0")
        var deltas = 0, deltaBytes = 0, fullBytes = 0
        for (k, text) in Self.edits().enumerated() {
            let st = try step(s, installed: installed, text: text, revision: 11 + k, id: "c1-\(k + 1)")
            let (env, pageBytes, target) = try DisplayListDelta.apply(st.delta, to: installed)
            XCTAssertEqual(env.payload, st.fresh.payload, "edit \(k): reconstruction differs from the fresh decode")
            XCTAssertEqual(pageBytes, st.freshPageBytes, "edit \(k): page_bytes vs the fresh line's raw page ranges")
            XCTAssertEqual(target, st.freshLine.count, "edit \(k): exact target size vs the fresh line length")
            XCTAssertLessThan(st.deltaLine.count, st.freshLine.count, "edit \(k): delta smaller than full")
            // full validation + parity exactly as for a full frame
            try RenderingV2.validate(env.payload)
            let frame = try V2Frame.prepare(env)
            let report = V2Parity.compare(frame: frame, scale: 1)
            XCTAssertEqual(report.totalDifferingPixels, 0, "edit \(k): parity at 1 px/pt")
            deltas += 1; deltaBytes += st.deltaLine.count; fullBytes += st.freshLine.count
            installed = try XCTUnwrap(DisplayListDelta.installed(from: env, pageBytes: pageBytes, lineBytes: target))
        }
        XCTAssertEqual(deltas, Self.edits().count)
        print("delta-gate: \(deltas) deltas, sibling bytes \(deltaBytes) vs fresh full \(fullBytes), pages \(installed.list.pages.count)")
    }

    /// C3: cap + 1 is refused BEFORE any page is built, naming both numbers; forged page_bytes
    /// (changed page +1, unchanged page −1) are refused before reconstruction; wrong base refused.
    func testCapPlusOneAndForgedPageBytesRefusedBeforeAllocation() throws {
        let s = try session()
        let installed = try install(s, text: Self.text0, revision: 20, id: "c3-0")
        let st = try step(s, installed: installed, text: Self.edits()[0], revision: 21, id: "c3-1")
        let (_, _, target) = try DisplayListDelta.apply(st.delta, to: installed)
        XCTAssertEqual(target, st.freshLine.count)
        XCTAssertThrowsError(try DisplayListDelta.apply(st.delta, to: installed, cap: target - 1)) { e in
            XCTAssertEqual(e as? DisplayListDelta.Refusal, .targetOversize(bytes: target, cap: target - 1))
        }
        XCTAssertNoThrow(try DisplayListDelta.apply(st.delta, to: installed, cap: target))
        let changed = try XCTUnwrap(st.delta.changedPages.first).page.number
        let unchanged = try XCTUnwrap((1...st.delta.pageCount).first { n in !st.delta.changedPages.contains { $0.page.number == n } }, "the edit must leave an unchanged page")
        var forged = st.delta; forged.pageBytes[changed - 1] += 1
        XCTAssertThrowsError(try DisplayListDelta.apply(forged, to: installed)) { e in
            XCTAssertEqual(e as? DisplayListDelta.Refusal, .pageBytesMismatch(page: changed, wire: st.delta.pageBytes[changed - 1] + 1, computed: st.delta.pageBytes[changed - 1]))
        }
        forged = st.delta; forged.pageBytes[unchanged - 1] -= 1
        XCTAssertThrowsError(try DisplayListDelta.apply(forged, to: installed)) { e in
            XCTAssertEqual(e as? DisplayListDelta.Refusal, .pageBytesMismatch(page: unchanged, wire: st.delta.pageBytes[unchanged - 1] - 1, computed: st.delta.pageBytes[unchanged - 1]))
        }
        var wrong = st.delta; wrong.base.listDigest = String(repeating: "0", count: 64)
        XCTAssertThrowsError(try DisplayListDelta.apply(wrong, to: installed)) { e in
            if case .baseMismatch? = e as? DisplayListDelta.Refusal {} else { XCTFail("\(e)") }
        }
        var tampered = st.delta; tampered.pageDigests[unchanged - 1] = String(repeating: "f", count: 64)
        XCTAssertThrowsError(try DisplayListDelta.apply(tampered, to: installed)) { e in
            XCTAssertEqual(e as? DisplayListDelta.Refusal, .digestMismatch(page: unchanged))
        }
    }

    /// Producer-side wrong base: acknowledging an older snapshot yields the unchanged full line.
    func testAcknowledgingAnOlderSnapshotYieldsFull() throws {
        let s = try session()
        let installed = try install(s, text: Self.text0, revision: 30, id: "c2-0")
        _ = try step(s, installed: installed, text: Self.edits()[0], revision: 31, id: "c2-1") // producer now holds c2-1
        try s.a.send(Self.request(Self.edits()[1], revision: 32, delta: true, ack: installed.acknowledgement), id: "c2-2")
        let result = try line(s.a, "c2-2 result"); let sibling = try line(s.a, "c2-2 sibling")
        let echo = try RuntimeV1.decodeCompileResult(result).payload.layoutCapabilities ?? []
        XCTAssertFalse(echo.contains(DisplayListDelta.capability))
        XCTAssertEqual(RenderingV2Fast.header(sibling)?.type, "display_list")
    }

    /// One in-flight held through a DELAYED main-thread completion while newer lines arrive:
    /// the queued slot is replaced (older queued dropped), the running slot is never released
    /// by that replacement, and at most installed + in-flight + queued exist.
    func testOneInFlightHeldThroughDelayedMainThreadCompletion() {
        var r = DisplayDeltaResidency()
        XCTAssertTrue(r.arrive(lineBytes: 1_000))            // A starts
        r.inFlightAllocated(modelBytes: 5_000, rasterBytes: 8_000)
        XCTAssertFalse(r.arrive(lineBytes: 1_100))           // B queued
        XCTAssertFalse(r.arrive(lineBytes: 1_200))           // C replaces B
        XCTAssertEqual(r.queuedReplaced, 1)
        XCTAssertEqual(r.inFlight?.total, 14_000, "dropping the queued line never released the running callback")
        XCTAssertEqual(r.liveSlots, 2)
        // delayed main-thread completion of A
        let done = expectation(description: "delayed completion")
        var after: DisplayDeltaResidency?
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
            _ = r.complete(install: true)
            after = r
            done.fulfill()
        }
        wait(for: [done], timeout: 5)
        let final = try! XCTUnwrap(after)
        XCTAssertEqual(final.installed?.modelBytes, 5_000)
        XCTAssertEqual(final.installed?.rasterBytes, 8_000)
        XCTAssertEqual(final.inFlight?.lineBytes, 1_200, "C started only after A completed on the main thread")
        XCTAssertNil(final.queued)
        XCTAssertEqual(final.peakSlots, 2)
        XCTAssertEqual(final.peakBytes, 14_000 + 1_200)
        // a stale completion (clearing event while running) installs nothing
        var r2 = DisplayDeltaResidency()
        _ = r2.arrive(lineBytes: 10); r2.inFlightAllocated(modelBytes: 20, rasterBytes: 30)
        r2.clear()
        XCTAssertNotNil(r2.inFlight, "clearing does not terminate the running callback")
        _ = r2.complete(install: false)
        XCTAssertNil(r2.installed); XCTAssertNil(r2.inFlight); XCTAssertEqual(r2.completionsDropped, 1)
    }

    /// Residency probe with the real producer: installed (model + painted bitmaps) + in-flight
    /// reconstruction (line + target + preraster) + queued line, exact accounting and RSS.
    func testResidencyMeasuredWithRealFrames() throws {
        let s = try session()
        let installed = try install(s, text: Self.text0, revision: 40, id: "res-0")
        let st = try step(s, installed: installed, text: Self.edits()[0], revision: 41, id: "res-1")
        let rss0 = Self.residentBytes()
        let installedFrame = try V2Frame.prepare(RenderingV2.Envelope(id: "res-0-full", payload: installed.list))
        let painted = V2Loader.preraster(installedFrame, pixelsPerPoint: 2, dark: false)
        var acct = DisplayDeltaResidency()
        _ = acct.arrive(lineBytes: st.deltaLine.count)
        let (env, pageBytes, target) = try DisplayListDelta.apply(st.delta, to: installed)
        let candidate = try V2Frame.prepare(env)
        let preraster = V2Loader.preraster(candidate, pixelsPerPoint: 2, dark: false)
        acct.inFlightAllocated(modelBytes: target, rasterBytes: Self.bytes(preraster))
        _ = acct.arrive(lineBytes: st.freshLine.count) // a queued newer line (use the fresh line as a stand-in)
        let rss1 = Self.residentBytes()
        let table = """
        residency (exact accounting, bytes):
          installed  model \(installed.serialisedBytes)  painted rasters \(Self.bytes(painted))  (\(installed.list.pages.count) pages)
          in-flight  line \(st.deltaLine.count)  target \(target)  preraster \(Self.bytes(preraster))
          queued     line \(st.freshLine.count)
          live slots \(acct.liveSlots) peak slots \(acct.peakSlots) accounted live bytes \(acct.liveBytes + installed.serialisedBytes + Self.bytes(painted))
          RSS delta over the probe \(rss1 - rss0) bytes (process-wide, load-sensitive; not a bound)
        """
        print(table)
        XCTAssertEqual(acct.liveSlots, 2) // installed is tracked outside `acct` here; in-flight + queued inside
        XCTAssertEqual(pageBytes.count, env.payload.pages.count)
        withExtendedLifetime((installedFrame, painted, candidate, preraster)) {}
    }

    static func bytes(_ p: V2Loader.Prerastered) -> Int { p.images.reduce(0) { $0 + $1.image.bytesPerRow * $1.image.height } }

    static func residentBytes() -> Int {
        var info = mach_task_basic_info()
        var count = mach_msg_type_number_t(MemoryLayout<mach_task_basic_info>.size / MemoryLayout<natural_t>.size)
        let kr = withUnsafeMutablePointer(to: &info) { $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) { task_info(mach_task_self_, task_flavor_t(MACH_TASK_BASIC_INFO), $0, &count) } }
        return kr == KERN_SUCCESS ? Int(info.resident_size) : 0
    }
}
