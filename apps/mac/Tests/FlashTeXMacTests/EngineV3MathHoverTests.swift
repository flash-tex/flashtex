import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac

/// The inline math hover preview under the engine-v3 preview (DESIGN §10
/// app parity, gap C24): as on the v2 pane, hovering a formula shows its
/// crop from the page bitmap on screen (padded by
/// `MathHoverPreview.padding`), and nothing while the preview is older than
/// the editor text. Needs a built `flashtex-host` and TeX Live (skipped
/// otherwise).
@MainActor
final class EngineV3MathHoverTests: XCTestCase {
    struct Host: View {
        let model: ShellModel
        var body: some View { EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom) }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-math-hover-\(getpid())")
    private var env = EnvironmentOverride()
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore() }

    func waitUntil(_ what: String, timeout: TimeInterval = 120, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    static let doc = "\\documentclass{article}\n\\begin{document}\nBefore words $a^2+b^2=c^2$ after words.\n\\end{document}\n"

    func testHoveringAFormulaShowsItsCropFromThePage() async throws {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: Self.doc, named: "main.tex")
        model.engineV3Enabled = true
        let s = model.engineV3
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 800), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        s.start(model: model)
        defer { s.stop(); window.contentView = nil }
        try await EngineV3TestHost.awaitReady(s)
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && !s.compiling && s.pages[0] != nil }
        window.layoutIfNeeded()
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: 1)
        pages.relayout()
        try await waitUntil("the page bitmap on screen") { pages.installedImage(0) != nil }

        let ns = Self.doc as NSString
        let formula = ns.range(of: "$a^2+b^2=c^2$")
        let text = Self.doc
        func byte(_ needle: String) -> Int { text.utf8.distance(from: text.startIndex, to: text.range(of: needle)!.lowerBound) }
        // The formula's box holds its glyphs and not the words around it.
        let (page, box) = try XCTUnwrap(s.formulaBox(path: "main.tex", start: byte("$a^2"), end: byte("$a^2") + "$a^2+b^2=c^2$".utf8.count,
                                                     in: text, pages: [0]))
        XCTAssertEqual(page, 0)
        // TeX typesets an inline formula when it reads the closing `$`: its
        // glyphs carry that column. The box is everything between the word
        // before the formula and the word after it, on that line.
        let words = try XCTUnwrap(s.place(path: "main.tex", byte: byte("s $a"), in: text)) // the "s" of "words"
        let after = try XCTUnwrap(s.place(path: "main.tex", byte: byte("after"), in: text))
        XCTAssertGreaterThan(box.minX, words.rect.maxX - 0.5, "starts after 'words': \(box) vs \(words.rect)")
        XCTAssertLessThan(box.maxX, after.rect.minX + 0.5, "ends before 'after': \(box) vs \(after.rect)")
        XCTAssertGreaterThan(box.width, 40, "a^2+b^2=c^2 is about 50 pt wide: \(box)")

        // The hover image: the padded box at the bitmap's scale, with ink in it.
        let image = try XCTUnwrap(s.mathPreviewImage(span: formula))
        let page0 = try XCTUnwrap(pages.installedImage(0))
        let ppp = Double(page0.width) / Double(try XCTUnwrap(s.pageSize(0)).width)
        XCTAssertEqual(Double(image.width), (box.width + 2 * MathHoverPreview.padding) * ppp, accuracy: 2)
        XCTAssertEqual(Double(image.height), (box.height + 2 * MathHoverPreview.padding) * ppp, accuracy: 2)
        XCTAssertTrue(Self.hasInk(image), "the crop shows the formula")

        // Stale: an edit the preview has not caught up with shows nothing.
        model.autoCompile = false
        model.updateActiveText(Self.doc.replacingOccurrences(of: "Before", with: "Earlier"))
        let moved = (model.activeText as NSString).range(of: "$a^2+b^2=c^2$")
        XCTAssertNil(s.mathPreviewImage(span: moved), "no crop from a preview older than the text")
    }

    /// Any pixel darker than mid grey.
    static func hasInk(_ image: CGImage) -> Bool {
        let w = image.width, h = image.height
        var px = [UInt8](repeating: 255, count: w * h)
        guard let ctx = CGContext(data: &px, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w,
                                  space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGImageAlphaInfo.none.rawValue) else { return false }
        ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
        return px.contains { $0 < 128 }
    }
}
