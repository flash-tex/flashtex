import CoreGraphics
import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// An included PDF page (`IMAGE` type `pdf`, protocol §5.2) is drawn by
/// mapping its box to the image's unit square. The box is in bp; a box in
/// pdfTeX's scaled points (what `flashtex-host` sent for the title-page
/// logo of "Infinite Descent", 69.738 × 138.331 bp) shrank the page 65,782
/// times and the logo vanished from the preview. Such a box is now replaced
/// by the file's own box.
final class PDFImageBoxTests: XCTestCase {
    /// A one-page PDF with the given media box (and crop box, when given).
    func makePDF(media: CGRect, crop: CGRect? = nil) throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("pdf-image-box-\(UUID().uuidString).pdf")
        var box = media
        let ctx = try XCTUnwrap(CGContext(url as CFURL, mediaBox: &box, nil))
        var info: [CFString: Any] = [kCGPDFContextMediaBox: NSData(bytes: &box, length: MemoryLayout<CGRect>.size)]
        if var c = crop { info[kCGPDFContextCropBox] = NSData(bytes: &c, length: MemoryLayout<CGRect>.size) }
        ctx.beginPDFPage(info as CFDictionary)
        ctx.setFillColor(CGColor(red: 0.5, green: 0, blue: 0.5, alpha: 1))
        ctx.fill(media.insetBy(dx: 5, dy: 5))
        ctx.endPDFPage()
        ctx.closePDF()
        addTeardownBlock { try? FileManager.default.removeItem(at: url) }
        return url
    }

    func info(_ url: URL, _ box: CGRect, pageBox: String = "crop") -> DL3JSON {
        .object(["id": .int(1), "key": .string(UUID().uuidString), "type": .string("pdf"), "file": .string(url.path), "page": .int(1),
                 "page_box": .string(pageBox), "width": .double(box.width), "height": .double(box.height),
                 "orig_x": .double(box.minX), "orig_y": .double(box.minY)])
    }

    func loadedBox(_ j: DL3JSON) throws -> CGRect {
        let img = try DL3RenderImage.load(j).get()
        guard case .pdf(_, let box) = img.payload else { XCTFail("not a PDF payload"); return .zero }
        return box
    }

    func testABoxInBigPointsIsUsedAsSent() throws {
        let media = CGRect(x: 0, y: 0, width: 69.738, height: 138.331)
        let url = try makePDF(media: media)
        let sent = CGRect(x: 2, y: 3, width: 60, height: 100) // e.g. a viewport selection: the host's box wins
        XCTAssertEqual(try loadedBox(info(url, sent)), sent)
    }

    func testABoxInScaledPointsFallsBackToTheFilesBox() throws {
        let media = CGRect(x: 0, y: 0, width: 69.738, height: 138.331)
        let url = try makePDF(media: media)
        let sp = 65_781.76 // scaled points per bp (pdfTeX's one_hundred_bp / 100)
        let sent = CGRect(x: 0, y: 0, width: (media.width * sp).rounded(), height: (media.height * sp).rounded())
        let box = try loadedBox(info(url, sent))
        XCTAssertEqual(box.width, media.width, accuracy: 0.01)
        XCTAssertEqual(box.height, media.height, accuracy: 0.01)
        XCTAssertEqual(box.minX, 0, accuracy: 0.01)
    }

    func testTheFallbackHonoursPageBox() throws {
        let media = CGRect(x: 0, y: 0, width: 200, height: 300)
        let crop = CGRect(x: 10, y: 20, width: 100, height: 150)
        let url = try makePDF(media: media, crop: crop)
        let missing = CGRect.zero
        let c = try loadedBox(info(url, missing, pageBox: "crop"))
        XCTAssertEqual(c.minX, 10, accuracy: 0.01); XCTAssertEqual(c.width, 100, accuracy: 0.01)
        let m = try loadedBox(info(url, missing, pageBox: "media"))
        XCTAssertEqual(m.width, 200, accuracy: 0.01); XCTAssertEqual(m.height, 300, accuracy: 0.01)
        let t = try loadedBox(info(url, missing, pageBox: "trim"))
        XCTAssertEqual(t.width, 100, accuracy: 0.01, "no trim box: the crop box, as pdfTeX falls back")
    }
}
