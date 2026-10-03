import CoreGraphics
import Foundation
import XCTest
@testable import FlashTeXPreviewV3

/// The pane draws the pages it shows from the compile's PDF (INCOMPLETE pages:
/// beamer's shaded balls, headlines, frame titles) on a concurrent raster
/// queue and the tile queue at once, all from one `CGPDFDocument`. Core
/// Graphics draws one document's shadings differently when two threads draw
/// from it at the same time (lane BEAMER-V3: Madrid and allowframebreaks
/// pages up to 75 levels off, different pages on every run). Every PDF page
/// `DL3Renderer` draws holds its document's lock (`drawPDFPage(_:in:)`), so
/// pages drawn from many threads equal pages drawn one at a time.
final class PDFConcurrentDrawTests: XCTestCase {
    static var repoRoot: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }

    func testOneDocumentsPagesDrawTheSameFromManyThreads() throws {
        // pdflatex's Madrid deck: radial shadings (the balls) in Form XObjects, axial ones in the headline.
        let url = Self.repoRoot.appendingPathComponent("fixtures/real-world/beamer-madrid/reference.pdf") as CFURL
        let pages = try XCTUnwrap(CGPDFDocument(url)).numberOfPages
        XCTAssertGreaterThan(pages, 3)
        let scales = [2.0, 4.25] // the backdrop and a tiled pane's scale
        // One at a time, each page from its own document.
        var reference: [[UInt8]] = []
        for k in 1 ... pages {
            for s in scales {
                let page = try XCTUnwrap(CGPDFDocument(url)?.page(at: k))
                reference.append(DL3Parity.rgba(try XCTUnwrap(DL3Renderer.rasterize(pdfPage: page, scale: s))))
            }
        }
        for round in 1 ... 3 {
            let doc = try XCTUnwrap(CGPDFDocument(url))
            var drawn = [[UInt8]](repeating: [], count: reference.count)
            let lock = NSLock()
            DispatchQueue.concurrentPerform(iterations: reference.count) { j in
                guard let page = doc.page(at: j / scales.count + 1) else { return }
                let s = scales[j % scales.count]
                // The two ways the pane draws a fallback page: a CGImage, and an IOSurface (its layer contents).
                let image = j % 3 == 0 ? DL3Renderer.rasterize(pdfPage: page, scale: s)
                    : DL3Renderer.rasterizeToSurface(pdfPage: page, scale: s).flatMap { DL3Renderer.image(of: $0) }
                let px = image.map(DL3Parity.rgba) ?? []
                lock.lock(); drawn[j] = px; lock.unlock()
            }
            for j in reference.indices {
                let d = DL3Parity.diff(drawn[j], reference[j])
                XCTAssertEqual(d.pixels, 0, "round \(round), page \(j / scales.count + 1) at \(scales[j % scales.count]) px/pt: max delta \(d.maxDelta)")
            }
        }
    }

    func testOneDocumentAlwaysTakesTheSameLock() throws {
        let url = Self.repoRoot.appendingPathComponent("fixtures/real-world/beamer-madrid/reference.pdf") as CFURL
        let doc = try XCTUnwrap(CGPDFDocument(url))
        XCTAssertEqual(DL3Renderer.pdfStripe(doc.page(at: 1)?.document), DL3Renderer.pdfStripe(doc.page(at: 2)?.document))
        XCTAssertTrue((0 ..< DL3Renderer.pdfLocks.count).contains(DL3Renderer.pdfStripe(doc)))
        XCTAssertEqual(DL3Renderer.pdfStripe(nil), 0)
    }
}
