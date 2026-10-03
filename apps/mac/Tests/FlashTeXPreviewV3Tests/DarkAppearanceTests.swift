import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// The dark reading appearance (DL3Appearance.dark): a dark page, ink with
/// its lightness inverted and hue kept, images untouched. Light stays the
/// PDF exactly (PreviewParityTests). `FLASHTEX_V3_DARK_PNG=dir` writes
/// light/dark pairs rendered by the app's renderer (evidence).
final class DarkAppearanceTests: XCTestCase {
    static var repoRoot: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }

    func testLightnessInversionKeepsHue() {
        func close(_ a: (Double, Double, Double), _ b: (Double, Double, Double)) -> Bool {
            abs(a.0 - b.0) < 1e-9 && abs(a.1 - b.1) < 1e-9 && abs(a.2 - b.2) < 1e-9
        }
        XCTAssertTrue(close(DL3Appearance.darken(r: 0, g: 0, b: 0), (1, 1, 1)))
        let g = DL3Appearance.groundLightness
        XCTAssertTrue(close(DL3Appearance.darken(r: 1, g: 1, b: 1), (g, g, g)), "white ink is the ground")
        let blue = DL3Appearance.darken(r: 0, g: 0, b: 1)
        XCTAssertEqual(blue.0, blue.1, accuracy: 1e-9); XCTAssertGreaterThan(blue.2, 0.99, "pure blue stays blue")
        let d = DL3Appearance.darken(r: 0.2, g: 0.2, b: 0.7) // beamer's structure blue
        XCTAssertGreaterThan(d.0 + d.1 + d.2, 0.2 + 0.2 + 0.7, "lighter")
        XCTAssertGreaterThan(d.2, d.0, "still blue")
        XCTAssertEqual(d.0, d.1, accuracy: 1e-9)
    }

    func rgba(_ img: CGImage) -> [UInt8] { DL3Parity.rgba(img) }

    func pixel(_ px: [UInt8], _ img: CGImage, x: Int, y: Int) -> [UInt8] {
        let i = (y * img.width + x) * 4
        return Array(px[i ..< i + 4])
    }

    func save(_ img: CGImage, _ name: String) {
        guard let dir = ProcessInfo.processInfo.environment["FLASHTEX_V3_DARK_PNG"] else { return }
        try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        let url = URL(fileURLWithPath: dir).appendingPathComponent(name + ".png")
        if let d = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil) {
            CGImageDestinationAddImage(d, img, nil); CGImageDestinationFinalize(d)
        }
    }

    func testDarkPagesAreDarkWithLightInk() throws {
        let dir = Self.repoRoot.appendingPathComponent("apps/mac/Tests/FlashTeXDisplayListV3Tests/Fixtures")
        let doc = try DL3Document(frames: Array(try Data(contentsOf: dir.appendingPathComponent("beamer-overlays.dl3"))))
        let page = try XCTUnwrap(doc.orderedPages.first)
        let light = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: 2))
        let dark = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: 2, appearance: .dark))
        save(light, "beamer-overlays-p1-light"); save(dark, "beamer-overlays-p1-dark")
        let lp = rgba(light), dp = rgba(dark)
        // Mean luminance: the dark page is dark overall.
        func mean(_ p: [UInt8]) -> Double {
            var sum = 0.0
            for i in stride(from: 0, to: p.count, by: 4) {
                let r = Double(p[i]), g = Double(p[i + 1]), b = Double(p[i + 2])
                sum += r + g + b
            }
            return sum / Double(p.count / 4 * 3)
        }
        XCTAssertGreaterThan(mean(lp), 180); XCTAssertLessThan(mean(dp), 90)
        // Where the light page has dark ink, the dark page has light ink.
        var inkPixels = 0, lightInk = 0
        for i in stride(from: 0, to: lp.count, by: 4) where lp[i] < 60 && lp[i + 1] < 60 && lp[i + 2] < 60 {
            inkPixels += 1
            if dp[i] > 190, dp[i + 1] > 190, dp[i + 2] > 190 { lightInk += 1 }
        }
        XCTAssertGreaterThan(inkPixels, 100)
        XCTAssertGreaterThan(Double(lightInk) / Double(inkPixels), 0.95)
    }

    /// Images keep their own pixels in the dark appearance (only the page
    /// items' own colours change), on the parity fixture with a PNG figure.
    func testImagesAreNotInverted() throws {
        let fx = Self.repoRoot.appendingPathComponent("target/dl3-positions/real-world__beamer-blocks-columns/display.dl3")
        guard let data = try? Data(contentsOf: fx) else { throw XCTSkip("run tools/displaylist/check_positions.py first") }
        let doc = try DL3Document(frames: Array(data))
        guard let page = doc.orderedPages.first(where: { p in p.page.items.contains { if case .image = $0 { true } else { false } } }),
              case .image(_, let m)? = page.page.items.first(where: { if case .image = $0 { true } else { false } }) else { return XCTFail("no image page") }
        let scale = 2.0
        let light = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: scale))
        let dark = try XCTUnwrap(DL3Renderer.rasterize(page, forms: doc.forms, scale: scale, appearance: .dark))
        save(light, "beamer-blocks-columns-p\(page.page.index + 1)-light"); save(dark, "beamer-blocks-columns-p\(page.page.index + 1)-dark")
        // The image's unit square in stream space → bitmap pixels (y up in stream, y down in the bitmap rows).
        let t = DL3Renderer.affine(page.page.matrix(m))
        let r = CGRect(x: 0, y: 0, width: 1, height: 1).applying(t).insetBy(dx: 2, dy: 2)
        let lp = rgba(light), dp = rgba(dark)
        var same = 0, n = 0
        for yy in stride(from: r.minY, to: r.maxY, by: 1.5) {
            for xx in stride(from: r.minX, to: r.maxX, by: 1.5) {
                let px = Int(xx * scale), py = light.height - 1 - Int(yy * scale)
                guard px >= 0, py >= 0, px < light.width, py < light.height else { continue }
                n += 1
                if pixel(lp, light, x: px, y: py) == pixel(dp, dark, x: px, y: py) { same += 1 }
            }
        }
        XCTAssertGreaterThan(n, 100)
        XCTAssertGreaterThan(Double(same) / Double(n), 0.99, "the figure's pixels are the same in both appearances")
    }

    /// Evidence pairs of a few parity fixtures, when they exist.
    func testWriteEvidencePairs() throws {
        guard ProcessInfo.processInfo.environment["FLASHTEX_V3_DARK_PNG"] != nil else { throw XCTSkip("set FLASHTEX_V3_DARK_PNG") }
        for name in ["real-world__plain-article", "real-world__thesis-chapter", "real-world__beamer-madrid", "real-world__hyperref-toc"] {
            let fx = Self.repoRoot.appendingPathComponent("target/dl3-positions/\(name)/display.dl3")
            guard let data = try? Data(contentsOf: fx) else { continue }
            let doc = try DL3Document(frames: Array(data))
            guard let page = doc.orderedPages.first(where: { !$0.needsPDFFallback(forms: doc.forms) }) else { continue }
            if let l = DL3Renderer.rasterize(page, forms: doc.forms, scale: 1.5) { save(l, "\(name)-p\(page.page.index + 1)-light") }
            if let d = DL3Renderer.rasterize(page, forms: doc.forms, scale: 1.5, appearance: .dark) { save(d, "\(name)-p\(page.page.index + 1)-dark") }
        }
    }
}
