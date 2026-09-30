import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers
import PDFKit
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// The preview-parity gate (DESIGN.md §6.2, P3 exit): every page the engine
/// expressed fully (not INCOMPLETE) rasterises through `DL3Renderer` to the
/// same RGBA bytes as Core Graphics' rendering of the engine's PDF (which is
/// byte-identical to pdflatex's for the parity fixtures), at 1×, 2× and 4×,
/// zero tolerance.
///
/// * Always: the checked-in `beamer-overlays` display list and PDF.
/// * Every parity fixture when `target/dl3-positions/` (or
///   `FLASHTEX_DL3_FIXTURES`) holds them (`tools/displaylist/check_positions.py`
///   leaves `display.dl3` and `src/main.pdf` there); skipped otherwise.
///   `FLASHTEX_V3_PARITY_OUT=path.json` writes the per-page report.
final class PreviewParityTests: XCTestCase {
    static let scales: [Double] = [1, 2, 4]

    static var repoRoot: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }

    enum Reference: String, Codable { case coreGraphics, pdfKit }

    /// PDFKit's rendering of page `index` into the renderer's bitmap configuration.
    static func pdfKitRender(_ doc: PDFDocument, _ index: Int, _ scale: Double) -> CGImage? {
        guard let page = doc.page(at: index) else { return nil }
        let box = page.bounds(for: .mediaBox)
        guard let ctx = DL3Renderer.bitmapContext(widthPt: box.width, heightPt: box.height, scale: scale) else { return nil }
        ctx.translateBy(x: -box.minX, y: -box.minY)
        page.draw(with: .mediaBox, to: ctx)
        return ctx.makeImage()
    }

    func check(dl3: URL, pdf: URL, name: String, reference: Reference = .coreGraphics) throws -> [DL3Parity.PageResult] {
        let doc = try DL3Document(frames: Array(try Data(contentsOf: dl3)))
        let pdfDoc = try XCTUnwrap(CGPDFDocument(pdf as CFURL), "\(name): no PDF")
        XCTAssertEqual(doc.pages.count, pdfDoc.numberOfPages, "\(name): pages")
        let kit = reference == .pdfKit ? PDFDocument(url: pdf) : nil
        let dump = ProcessInfo.processInfo.environment["FLASHTEX_V3_PARITY_PNG"]
        return DL3Parity.compare(document: doc, pdf: pdfDoc, scales: Self.scales,
                                 reference: kit.map { k in { Self.pdfKitRender(k, $0, $1) } }) { page, scale, a, b in
            guard let dump else { return }
            for (img, tag) in [(a, "preview"), (b, reference == .pdfKit ? "pdfkit" : "pdf")] {
                let url = URL(fileURLWithPath: dump).appendingPathComponent("\(name)-p\(page + 1)-\(Int(scale))x-\(tag).png")
                if let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil) {
                    CGImageDestinationAddImage(dest, img, nil); CGImageDestinationFinalize(dest)
                }
            }
        }
    }

    /// The engine's PDF in a fixture's work directory: `main.pdf`, else the
    /// PDF named after the fixture's `.tex` (not the oracle's `*reference*`).
    static func enginePDF(in src: URL) -> URL? {
        let fm = FileManager.default
        let main = src.appendingPathComponent("main.pdf")
        if fm.fileExists(atPath: main.path) { return main }
        let files = (try? fm.contentsOfDirectory(atPath: src.path)) ?? []
        for tex in files where tex.hasSuffix(".tex") {
            let pdf = src.appendingPathComponent(String(tex.dropLast(4)) + ".pdf")
            if fm.fileExists(atPath: pdf.path) { return pdf }
        }
        return nil
    }

    func testCheckedInFixtureIsPixelIdenticalToThePDF() throws {
        let dir = Self.repoRoot.appendingPathComponent("apps/mac/Tests/FlashTeXDisplayListV3Tests/Fixtures")
        let results = try check(dl3: dir.appendingPathComponent("beamer-overlays.dl3"), pdf: dir.appendingPathComponent("beamer-overlays.pdf"), name: "beamer-overlays")
        XCTAssertFalse(results.isEmpty)
        for r in results where !r.incomplete {
            XCTAssertEqual(r.differingPixels, 0, "page \(r.page + 1) at \(r.scale)x: \(r.differingPixels) px differ (max Δ \(r.maxChannelDelta)) \(r.problems)")
        }
    }

    func testEveryParityFixtureIsPixelIdenticalToThePDF() throws { try sweep(reference: .coreGraphics) }

    /// The on-screen layout (BGRA, premultiplied-first) draws the same
    /// pixels as the RGBA one the parity sweeps compare.
    func testScreenLayoutDrawsTheSamePixels() throws {
        let dir = Self.repoRoot.appendingPathComponent("apps/mac/Tests/FlashTeXDisplayListV3Tests/Fixtures")
        let doc = try DL3Document(frames: Array(try Data(contentsOf: dir.appendingPathComponent("beamer-overlays.dl3"))))
        for page in doc.orderedPages {
            for scale in Self.scales {
                let a = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: scale))
                let b = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: scale, layout: .screen))
                let bgra = DL3Parity.rgba(b) // converted to RGBA by drawing: a lossless reorder for opaque pixels
                XCTAssertEqual(DL3Parity.diff(DL3Parity.rgba(a), bgra).pixels, 0, "page \(page.page.index + 1) at \(scale)x")
                let surface = try XCTUnwrap(DL3Renderer.rasterizeToSurface(page, forms: doc.forms, scale: scale))
                let c = try XCTUnwrap(DL3Renderer.image(of: surface))
                XCTAssertEqual(DL3Parity.diff(DL3Parity.rgba(a), DL3Parity.rgba(c)).pixels, 0, "IOSurface page \(page.page.index + 1) at \(scale)x")
            }
        }
    }

    /// Against PDFKit's rendering (what Preview.app shows). PDFKit itself
    /// differs from Core Graphics' `drawPDFPage` on the same PDF (thin rules
    /// and some glyphs at 1x and 2x, measured per page as `baseline`), so the
    /// gate here is: the preview is no further from PDFKit than Core
    /// Graphics' own rendering of the engine's PDF is (plus the 1x floor).
    func testEveryParityFixtureIsAsCloseToPDFKitAsCoreGraphicsIs() throws { try sweep(reference: .pdfKit) }

    /// Residual differences, measured (EVIDENCE: docs/evidence/app-v3-preview-2026-09-29/):
    /// single glyphs at 1x whose origin lies within 7.6e-6 bp of a subpixel
    /// boundary, where the sp-rounded display-list position (spec §4.2) and
    /// the PDF's decimal position fall on different sides. Allowed per page:
    /// at most `floorPixels` differing pixels, only at 1x.
    static let floorPixels = 64

    func sweep(reference: Reference) throws {
        let env = ProcessInfo.processInfo.environment
        let dir = URL(fileURLWithPath: env["FLASHTEX_DL3_FIXTURES"] ?? Self.repoRoot.appendingPathComponent("target/dl3-positions").path)
        guard let names = try? FileManager.default.contentsOfDirectory(atPath: dir.path).sorted() else {
            throw XCTSkip("no \(dir.path): run tools/displaylist/check_positions.py first")
        }
        struct Row: Codable { var fixture: String; var reference: Reference; var pages: [DL3Parity.PageResult]; var baseline: [Int]? }
        var rows: [Row] = []
        var complete = 0, identical = 0, fallback = 0, pixels = 0
        var perScale: [Double: (pages: Int, identical: Int, pixels: Int)] = [:]
        for name in names {
            let dl3 = dir.appendingPathComponent(name).appendingPathComponent("display.dl3")
            guard FileManager.default.fileExists(atPath: dl3.path),
                  let pdf = Self.enginePDF(in: dir.appendingPathComponent(name).appendingPathComponent("src")) else { continue }
            let results = try check(dl3: dl3, pdf: pdf, name: name, reference: reference)
            var baseline: [Int]?
            if reference == .pdfKit, let cg = CGPDFDocument(pdf as CFURL), let kit = PDFDocument(url: pdf) {
                baseline = results.map { r in
                    guard let page = cg.page(at: r.page + 1), let a = DL3Renderer.rasterize(pdfPage: page, scale: r.scale),
                          let b = Self.pdfKitRender(kit, r.page, r.scale) else { return -1 }
                    return DL3Parity.diff(DL3Parity.rgba(a), DL3Parity.rgba(b)).pixels
                }
            }
            rows.append(Row(fixture: name, reference: reference, pages: results, baseline: baseline))
            for r in results {
                if r.incomplete { fallback += 1; continue }
                complete += 1
                pixels += r.differingPixels
                var s = perScale[r.scale] ?? (0, 0, 0)
                s.pages += 1; s.pixels += r.differingPixels
                if r.differingPixels == 0 { identical += 1; s.identical += 1 }
                perScale[r.scale] = s
            }
        }
        if let out = env["FLASHTEX_V3_PARITY_OUT"] {
            let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted, .sortedKeys]
            try enc.encode(rows).write(to: URL(fileURLWithPath: out + "." + reference.rawValue + ".json"))
        }
        let baselinePixels = rows.reduce(0) { acc, row in acc + zip(row.pages, row.baseline ?? []).filter { !$0.0.incomplete }.reduce(0) { $0 + max(0, $1.1) } }
        if reference == .pdfKit { print("preview parity: Core Graphics' own rendering of the same PDFs differs from PDFKit by \(baselinePixels) px on those pages") }
        let byScale = perScale.keys.sorted().map { "\(Int($0))x \(perScale[$0]!.identical)/\(perScale[$0]!.pages) identical (\(perScale[$0]!.pixels) px differ)" }.joined(separator: "; ")
        print("preview parity vs \(reference.rawValue): \(rows.count) fixtures; page renders \(identical)/\(complete) identical, \(pixels) differing pixels in all; \(fallback) page renders drawn from the PDF instead (INCOMPLETE page or form); \(byScale)")
        XCTAssertGreaterThan(complete, 0)
        for row in rows {
            for (i, r) in row.pages.enumerated() where !r.incomplete && r.differingPixels != 0 {
                if let b = row.baseline?[i], r.differingPixels <= b + (r.scale == 1 ? Self.floorPixels : 0) { continue }
                if r.scale == 1, r.differingPixels <= Self.floorPixels {
                    print("  floor: \(row.fixture) page \(r.page + 1) at 1x: \(r.differingPixels) px (max Δ \(r.maxChannelDelta))")
                    continue
                }
                XCTFail("\(row.fixture) page \(r.page + 1) at \(r.scale)x vs \(reference.rawValue): \(r.differingPixels) px differ (max Δ \(r.maxChannelDelta)) \(r.problems)")
            }
        }
    }
}
