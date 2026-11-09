import CoreGraphics
import Foundation
import XCTest
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
@testable import FlashTeXMac

/// `loadFallbacks` reads the compile's PDF off main and installs its pages on
/// main later (REVIEW-E on #1971): a newer run's page that arrives in between
/// must not get the older run's PDF page drawn over it.
@MainActor
final class EngineV3FallbackSupersedeTests: XCTestCase {
    static let dir = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-fallback-supersede-\(getpid())")
    private var env = EnvironmentOverride()

    override func setUpWithError() throws {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.dir.appendingPathComponent("cache").path)
        env.set("FLASHTEX_HOST", "none")
        env.set("FLASHTEX_BUNDLE_LOCK", "/nonexistent/flashtex-bundle.lock")
        env.set("FLASHTEX_BUNDLE_DIGEST", "")
        try FileManager.default.createDirectory(at: Self.dir, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        env.restore()
        try? FileManager.default.removeItem(at: Self.dir)
    }

    /// A whole one-page PDF (ends in `%%EOF`).
    private func writePDF() throws -> String {
        let url = Self.dir.appendingPathComponent("main.pdf")
        var box = CGRect(x: 0, y: 0, width: 612, height: 792)
        let ctx = try XCTUnwrap(CGContext(url as CFURL, mediaBox: &box, nil))
        ctx.beginPDFPage(nil)
        ctx.fill(CGRect(x: 72, y: 72, width: 100, height: 100))
        ctx.endPDFPage()
        ctx.closePDF()
        XCTAssertTrue(DL3Renderer.endsWhole(try Data(contentsOf: url)))
        return url.path
    }

    /// An INCOMPLETE page: drawn from the compile's PDF.
    private static func page(_ tag: UInt8) -> DL3PreparedPage {
        var p = DL3Page(kind: .page, index: 0)
        p.width = 612 * 65536; p.height = 792 * 65536; p.box = [0, 0, 612, 792]
        p.hash = [UInt8](repeating: tag, count: 32)
        p.flags = DL3Page.flagIncomplete
        return DL3PreparedPage(page: p, fonts: [:], images: [:])
    }

    /// Lets the PDF read finish and its main-thread apply run.
    private func drainFallbackLoad() {
        EngineV3Session.readQueue.sync {}
        RunLoop.main.run(until: Date().addingTimeInterval(0.1))
    }

    func testAnOlderRunsPDFPageIsNotInstalledOverANewerRunsPage() throws {
        let pdf = try writePDF()
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n", named: "main.tex")
        model.engineV3Enabled = true
        defer { model.engineV3Enabled = false }
        let s = model.engineV3

        // Run 1's DONE loads its PDF page for page 0 (the control: the load reaches main).
        s.handle(.started(.object(["id": .int(1)])))
        s.handle(.page(Self.page(0xA0), compileID: 1, timing: .init(), image: nil))
        s.handle(.done(.object(["id": .int(1), "status": .string("ok"), "pages": .int(1), "pdf": .string(pdf)]), compileID: 1))
        drainFallbackLoad()
        XCTAssertEqual(s.pdfFallbackCount, 1, "run 1's page is drawn from run 1's PDF")

        // Run 2's DONE starts a load; before it lands, run 3 sends page 0 (also INCOMPLETE).
        s.handle(.started(.object(["id": .int(2)])))
        s.handle(.page(Self.page(0xB0), compileID: 2, timing: .init(), image: nil))
        s.handle(.done(.object(["id": .int(2), "status": .string("ok"), "pages": .int(1), "pdf": .string(pdf)]), compileID: 2))
        s.handle(.started(.object(["id": .int(3)])))
        s.handle(.page(Self.page(0xC0), compileID: 3, timing: .init(), image: nil))
        XCTAssertEqual(s.pdfFallbackCount, 0, "run 3's page cleared the old fallback")
        drainFallbackLoad()
        XCTAssertEqual(s.pdfFallbackCount, 0, "run 2's PDF page was installed over run 3's page 0")

        // Run 3's own DONE loads its PDF page.
        s.handle(.done(.object(["id": .int(3), "status": .string("ok"), "pages": .int(1), "pdf": .string(pdf)]), compileID: 3))
        drainFallbackLoad()
        XCTAssertEqual(s.pdfFallbackCount, 1)
    }
}
