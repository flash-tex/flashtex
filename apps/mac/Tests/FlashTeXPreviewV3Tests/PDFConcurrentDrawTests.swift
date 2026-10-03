import CoreGraphics
import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// The pane draws the pages it shows from the compile's PDF (INCOMPLETE pages:
/// beamer's shaded balls, headlines, frame titles) on a concurrent raster
/// queue, the tile queue and `concurrentPerform` tile batches at once, all
/// from one opened PDF. Core Graphics draws one `CGPDFDocument`'s shadings
/// wrongly when two threads use it at the same time (lane BEAMER-V3: Madrid
/// and allowframebreaks pages up to 75 levels off, different pages on every
/// run; a lock around the draw alone still failed 3 of ~52 runs). A PDF
/// opened with `DL3Renderer.openPDF` is drawn from each thread's own
/// document of the same bytes (`drawPDFPage(_:in:)`), so pages drawn from
/// many threads equal pages drawn one at a time.
///
/// `FLASHTEX_PDF_CONCURRENT_ROUNDS` (default 3) sets the rounds;
/// `FLASHTEX_PDF_CONCURRENT_FILES` (paths, `:`-separated) draws those PDFs
/// instead of the Madrid deck, at `FLASHTEX_PDF_CONCURRENT_SCALES`
/// (`,`-separated px/pt; default 2, 4.25), at most
/// `FLASHTEX_PDF_CONCURRENT_PAGES` pages of each (evidence runs).
final class PDFConcurrentDrawTests: XCTestCase {
    static var repoRoot: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }

    /// pdflatex's Madrid deck: radial shadings (the balls) in Form XObjects, axial ones in the headline.
    static var madrid: URL { repoRoot.appendingPathComponent("fixtures/real-world/beamer-madrid/reference.pdf") }

    func testOneDocumentsPagesDrawTheSameFromManyThreads() throws {
        let env = ProcessInfo.processInfo.environment
        let files = env["FLASHTEX_PDF_CONCURRENT_FILES"].map { $0.split(separator: ":").map { URL(fileURLWithPath: String($0)) } } ?? [Self.madrid]
        // the backdrop and a tiled pane's scale
        let scales = env["FLASHTEX_PDF_CONCURRENT_SCALES"].map { $0.split(separator: ",").compactMap { Double($0) } } ?? [2.0, 4.25]
        let maxPages = Int(env["FLASHTEX_PDF_CONCURRENT_PAGES"] ?? "") ?? .max
        let rounds = Int(env["FLASHTEX_PDF_CONCURRENT_ROUNDS"] ?? "") ?? 3
        for url in files {
            try drawsTheSameFromManyThreads(url, scales: scales, maxPages: maxPages, rounds: rounds)
        }
    }

    func drawsTheSameFromManyThreads(_ url: URL, scales: [Double], maxPages: Int, rounds: Int) throws {
        let name = url.deletingLastPathComponent().lastPathComponent
        let pages = min(maxPages, try XCTUnwrap(CGPDFDocument(url as CFURL), name).numberOfPages)
        if url == Self.madrid { XCTAssertGreaterThan(pages, 3) }
        // One at a time, each page from its own document.
        var reference: [[UInt8]] = []
        for k in 1 ... pages {
            for s in scales {
                let page = try XCTUnwrap(CGPDFDocument(url as CFURL)?.page(at: k))
                reference.append(DL3Parity.rgba(try XCTUnwrap(DL3Renderer.rasterize(pdfPage: page, scale: s))))
            }
        }
        for round in 1 ... rounds {
            // As the session holds it: one document, its pages looked up and drawn from any thread.
            let doc = try XCTUnwrap(DL3Renderer.openPDF(url))
            var drawn = [[UInt8]](repeating: [], count: reference.count)
            let lock = NSLock()
            DispatchQueue.concurrentPerform(iterations: reference.count) { j in
                guard let page = doc.page(at: j / scales.count + 1) else { return }
                let s = scales[j % scales.count]
                // The ways the pane draws a fallback page: a CGImage, an IOSurface (layer contents), a kept tile raster.
                let image: CGImage?
                switch j % 3 {
                case 0: image = DL3Renderer.rasterize(pdfPage: page, scale: s)
                case 1: image = DL3Renderer.rasterizeToSurface(pdfPage: page, scale: s).flatMap { DL3Renderer.image(of: $0) }
                default:
                    let (w, h) = DL3PageRaster.gridSize(pdfPage: page, scale: s)
                    let whole = DL3PageRaster(pdfPage: page, scale: s)?.cut([DL3PixelRect(x: 0, y: 0, width: w, height: h)])?.first ?? nil
                    image = whole.flatMap { DL3Renderer.image(of: $0) }
                }
                let px = image.map(DL3Parity.rgba) ?? []
                lock.lock(); drawn[j] = px; lock.unlock()
            }
            for j in reference.indices {
                let d = DL3Parity.diff(drawn[j], reference[j])
                XCTAssertEqual(d.pixels, 0, "\(name): round \(round), page \(j / scales.count + 1) at \(scales[j % scales.count]) px/pt: max delta \(d.maxDelta)")
            }
        }
    }

    /// Each thread opens its own document of a PDF once and keeps the last
    /// few; a document not opened with `openPDF` draws as given.
    func testEachThreadOpensItsOwnDocumentOnce() throws {
        let a = try XCTUnwrap(DL3Renderer.openPDF(Self.madrid))
        let b = try XCTUnwrap(DL3Renderer.openPDF(Self.madrid))
        let ba = try XCTUnwrap(DL3PDFBytes.of(a)), bb = try XCTUnwrap(DL3PDFBytes.of(b))
        XCTAssertNotEqual(ba.id, bb.id, "each opened PDF is its own")
        XCTAssertNil(DL3PDFBytes.of(try XCTUnwrap(CGPDFDocument(Self.madrid as CFURL))))
        let cache = PDFThreadDocuments()
        let first = cache.document(for: ba)
        XCTAssertTrue(first === cache.document(for: ba), "kept")
        XCTAssertFalse(first === a, "a private document, not the shared one")
        XCTAssertEqual(cache.opened, 1)
        let more = (0 ..< PDFThreadDocuments.limit).map { _ in DL3PDFBytes(ba.data) }
        for m in more { _ = cache.document(for: m) }
        XCTAssertEqual(cache.opened, 1 + PDFThreadDocuments.limit)
        _ = cache.document(for: ba)
        XCTAssertEqual(cache.opened, 2 + PDFThreadDocuments.limit, "the least recently used went")
        XCTAssertTrue(PDFThreadDocuments.current === PDFThreadDocuments.current, "one per thread")
    }

    /// The bytes are read when the PDF is opened: the compile rewriting the
    /// file later does not change the pages already held.
    func testARewrittenFileDoesNotChangeThePagesHeld() throws {
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("pdf-bytes-\(UUID().uuidString).pdf")
        addTeardownBlock { try? FileManager.default.removeItem(at: tmp) }
        try FileManager.default.copyItem(at: Self.madrid, to: tmp)
        let doc = try XCTUnwrap(DL3Renderer.openPDF(tmp))
        let page = try XCTUnwrap(doc.page(at: 3))
        let before = DL3Parity.rgba(try XCTUnwrap(DL3Renderer.rasterize(pdfPage: page, scale: 1)))
        let other = Self.repoRoot.appendingPathComponent("fixtures/real-world/beamer-default/reference.pdf")
        try FileManager.default.removeItem(at: tmp)
        try FileManager.default.copyItem(at: other, to: tmp)
        let after = DL3Parity.rgba(try XCTUnwrap(DL3Renderer.rasterize(pdfPage: page, scale: 1)))
        XCTAssertEqual(DL3Parity.diff(before, after).pixels, 0)
    }

    /// An included PDF image is opened the same way (drawn per tile in `concurrentPerform`).
    func testIncludedPDFImagesAreOpenedForPerThreadDrawing() throws {
        let info: DL3JSON = .object(["id": .int(1), "key": .string("madrid"), "type": .string("pdf"), "file": .string(Self.madrid.path),
                                     "page": .int(1), "page_box": .string("media")])
        let image = try DL3RenderImage.load(info).get()
        XCTAssertNotNil(image.document.flatMap { DL3PDFBytes.of($0) })
    }
}
