import AppKit
import SwiftUI
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
import HostedWindows
@testable import FlashTeXMac

/// An included PDF page (`\includegraphics{logo.pdf}`) is drawn by the
/// engine-v3 pane, end to end through `flashtex-host` (lane INFDESC-APP: the
/// title-page logo of "Infinite Descent" was missing from the preview while
/// the exported PDF had it). Needs a built host (skipped otherwise).
@MainActor
final class EngineV3PDFImageTests: XCTestCase {
    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-pdfimage-tests-\(getpid())")
    override func setUp() { setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1) }
    override func tearDown() { unsetenv("FLASHTEX_V3_CACHE") }

    func waitUntil(_ what: String, timeout: TimeInterval = 120, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    /// A 70 × 140 bp page filled with pure magenta (the figure to find).
    static func writeFigure(_ url: URL) {
        var box = CGRect(x: 0, y: 0, width: 70, height: 140)
        guard let ctx = CGContext(url as CFURL, mediaBox: &box, nil) else { return }
        ctx.beginPDFPage(nil)
        ctx.setFillColor(CGColor(red: 1, green: 0, blue: 1, alpha: 1))
        ctx.fill(box)
        ctx.endPDFPage()
        ctx.closePDF()
    }

    func testIncludedPDFPageIsDrawn() async throws {
        try EngineV3TestHost.require()
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-pdfimage-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        Self.writeFigure(dir.appendingPathComponent("figure.pdf"))
        let main = dir.appendingPathComponent("main.tex")
        try Data("\\documentclass{article}\n\\usepackage{graphicx}\n\\begin{document}\nA figure:\n\n\\includegraphics{figure.pdf}\n\n\\includegraphics[width=35bp]{figure.pdf}\n\\end{document}\n".utf8).write(to: main)
        let model = ShellModel()
        _ = model.openTex(at: main)
        model.engineV3Enabled = true
        let s = model.engineV3
        s.start(model: model)
        defer { s.stop() }
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && s.pageCount == 1 && s.pages[0] != nil }
        let page = try XCTUnwrap(s.pages[0])
        let images = page.page.items.compactMap { item -> UInt32? in if case .image(let id, _) = item { id } else { nil } }
        XCTAssertEqual(images.count, 2, "two IMAGE items; items: \(page.page.items.count), forms \(s.forms.count), problems \(page.problems)")
        XCTAssertEqual(page.problems, [])
        // Light appearance, 2 px/pt: the figure's magenta is on the page.
        let img = try XCTUnwrap(DL3Renderer.rasterizeToSurface(page, forms: s.forms, scale: 2, appearance: .light).flatMap { DL3Renderer.image(of: $0) })
        let rep = NSBitmapImageRep(cgImage: img)
        var magenta = 0
        for y in stride(from: 0, to: rep.pixelsHigh, by: 2) {
            for x in stride(from: 0, to: rep.pixelsWide, by: 2) {
                guard let c = rep.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB) else { continue }
                if c.redComponent > 0.8, c.greenComponent < 0.5, c.blueComponent > 0.8 { magenta += 1 } // device magenta, colour-matched
            }
        }
        // 70 × 140 bp and 35 × 70 bp at 2 px/pt, sampled every 2 px: 9,800 + 2,450 samples.
        XCTAssertGreaterThan(magenta, 11_000, "the included PDF page is drawn at its size, twice")
        XCTAssertLessThan(magenta, 13_500, "and not larger")
        // Exactly as Core Graphics draws the compile's PDF (the preview parity rule, light mode).
        let pdf = URL(fileURLWithPath: try XCTUnwrap(s.lastDone?["pdf"]?.string))
        let pdfPage = try XCTUnwrap(CGPDFDocument(pdf as CFURL)?.page(at: 1))
        let ref = try XCTUnwrap(DL3Renderer.rasterizeToSurface(pdfPage: pdfPage, scale: 2, appearance: .light).flatMap { DL3Renderer.image(of: $0) })
        let d = DL3Parity.diff(DL3Parity.rgba(img), DL3Parity.rgba(ref))
        XCTAssertEqual(d.pixels, 0, "the pane's page equals the PDF's (max delta \(d.maxDelta))")
    }
}
