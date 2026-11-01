import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// Unicode mode's preview against the PDF (lane UNICODE-PREVIEW-QA).
///
/// `tools/displaylist/unicode_preview_parity.py` compiles every case of
/// `tools/displaylist/unicode-cases/` through `flashtex-host-unicode` as the
/// app does and leaves, per case, the frames the app received
/// (`display.dl3`), FlashTeX's own PDF of that compile (`flashtex.pdf`) and,
/// when TeX Live's xelatex is installed, xelatex's PDF (`oracle.pdf`). For
/// each page drawn through `DL3Renderer`:
///
/// * every font and image resolves (no `problems`), and the page and every
///   form it draws are complete (not drawn from the PDF instead), unless
///   the case says `% preview: pdf-fallback` (transparency and shadings,
///   which the app's client does not draw: the app shows FlashTeX's PDF,
///   so that PDF is what is compared with xelatex's);
/// * against FlashTeX's PDF: Core Graphics' rendering of it, pixel for
///   pixel (`maxOwnPixels`, 0 by default: the same glyphs of the same
///   programs at the same positions);
/// * against xelatex's PDF: the pages match after a 1 px/pt box filter
///   (xdvipdfmx rounds positions to 0.01 bp and subsets differently, so
///   exact pixels are not expected): the share of inked cells whose grey
///   differs by more than a quarter stays under `maxOracleMismatch`.
///   Wrong glyphs (#1759's glyph ids drawn as characters), a wrong face, a
///   missing image or a misplaced line all exceed it by far.
///
/// Input: `FLASHTEX_DL3_UNICODE` (default `target/dl3-unicode`); skipped
/// without it, failed when `FLASHTEX_UNICODE_PARITY_REQUIRE=1` (CI:
/// .github/workflows/unicode-preview-parity.yml). Without `oracle.pdf` the
/// xelatex comparison of that case is skipped (no xelatex installed).
/// `FLASHTEX_V3_PARITY_PNG=dir` writes the renders, `FLASHTEX_V3_PARITY_OUT=path`
/// the per-page report (`path.unicode.json`).
final class UnicodePreviewParityTests: XCTestCase {
    static var repoRoot: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }

    static let env = ProcessInfo.processInfo.environment
    static let scales: [Double] = env["FLASHTEX_V3_PARITY_SCALES"]
        .map { $0.split(separator: ",").compactMap { Double($0.trimmingCharacters(in: .whitespaces)) } } ?? [1, 2, 4]
    static let maxOwnPixels = Int(env["FLASHTEX_UNICODE_PARITY_OWN_PX"] ?? "") ?? 0
    static let maxOracleMismatch = Double(env["FLASHTEX_UNICODE_PARITY_ORACLE"] ?? "") ?? 0.02

    struct PageRow: Codable {
        var page: Int
        var problems: [String]
        var incomplete: Bool
        /// Against FlashTeX's PDF, per scale: differing pixels.
        var ownPixels: [Double: Int]
        /// Against xelatex's PDF (nil without it): mismatched / inked cells.
        var oracleMismatch: Double?
        var oracleInk: Int?
    }

    struct CaseRow: Codable {
        var name: String
        var pages: Int
        var pdfPages: Int
        var oraclePages: Int?
        var rows: [PageRow]
    }

    /// The tolerant comparison: grey at 1 px/pt cells (mean over each
    /// `scale`×`scale` block of the 2 px/pt render), then the share of cells
    /// inked on either side whose grey differs by more than 0.25.
    static func mismatch(_ a: [UInt8], _ b: [UInt8], width: Int, height: Int, block: Int) -> (share: Double, ink: Int) {
        guard a.count == b.count, a.count == width * height * 4 else { return (1, 0) }
        let cw = width / block, ch = height / block
        var ink = 0, bad = 0
        for cy in 0 ..< ch {
            for cx in 0 ..< cw {
                var ga = 0, gb = 0
                for y in cy * block ..< (cy + 1) * block {
                    var i = (y * width + cx * block) * 4
                    for _ in 0 ..< block {
                        ga += 2 * Int(a[i]) + 5 * Int(a[i + 1]) + Int(a[i + 2])
                        gb += 2 * Int(b[i]) + 5 * Int(b[i + 1]) + Int(b[i + 2])
                        i += 4
                    }
                }
                let n = Double(block * block * 8 * 255)
                let fa = Double(ga) / n, fb = Double(gb) / n
                guard fa < 0.9 || fb < 0.9 else { continue }
                ink += 1
                if abs(fa - fb) > 0.25 { bad += 1 }
            }
        }
        return (ink == 0 ? 0 : Double(bad) / Double(ink), ink)
    }

    static func write(_ img: CGImage, _ name: String) {
        guard let dir = env["FLASHTEX_V3_PARITY_PNG"] else { return }
        let url = URL(fileURLWithPath: dir).appendingPathComponent(name + ".png")
        if let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil) {
            CGImageDestinationAddImage(dest, img, nil); CGImageDestinationFinalize(dest)
        }
    }

    func check(_ dir: URL, name: String) throws -> CaseRow {
        let doc = try DL3Document(frames: Array(try Data(contentsOf: dir.appendingPathComponent("display.dl3"))))
        let own = try XCTUnwrap(CGPDFDocument(dir.appendingPathComponent("flashtex.pdf") as CFURL), "\(name): no flashtex.pdf")
        let oracleURL = dir.appendingPathComponent("oracle.pdf")
        let oracle = FileManager.default.fileExists(atPath: oracleURL.path) ? CGPDFDocument(oracleURL as CFURL) : nil
        var row = CaseRow(name: name, pages: doc.pages.count, pdfPages: own.numberOfPages, oraclePages: oracle?.numberOfPages, rows: [])
        XCTAssertEqual(doc.pages.count, own.numberOfPages, "\(name): display-list pages vs FlashTeX's PDF")
        if let oracle { XCTAssertEqual(doc.pages.count, oracle.numberOfPages, "\(name): pages vs xelatex's PDF") }
        // A case whose pages the display list cannot express (`% preview:
        // pdf-fallback` in its source) is shown from FlashTeX's PDF, as the
        // app shows it; that PDF is then what xelatex's is compared with.
        let source = (try? String(contentsOf: dir.appendingPathComponent("src/main.tex"), encoding: .utf8)) ?? ""
        let fallbackExpected = source.contains("% preview: pdf-fallback")
        // `% oracle: known-difference (why, #issue)`: FlashTeX's own
        // typesetting differs from xelatex's there (not the preview); the
        // mismatch is reported, and the mark must go once it no longer differs.
        let knownDifference = source.contains("% oracle: known-difference")
        // `% preview: tolerance N px (why)`: up to N pixels, each one
        // coverage level (delta 1) off, against FlashTeX's PDF.
        let tolerance = source.split(separator: "\n").lazy.compactMap { line -> Int? in
            guard let r = line.range(of: "% preview: tolerance ") else { return nil }
            return Int(line[r.upperBound...].prefix { $0.isNumber })
        }.first ?? Self.maxOwnPixels
        func judge(_ i: Int, _ m: (share: Double, ink: Int), _ what: String) {
            let text = "\(name) p\(i + 1)\(what) vs xelatex's PDF: \(String(format: "%.2f", m.share * 100))% of \(m.ink) inked cells differ"
            XCTAssertGreaterThan(m.ink, 0, "\(name) p\(i + 1): nothing drawn")
            if knownDifference {
                print("unicode parity (known difference) \(text)")
                XCTAssertGreaterThan(m.share, Self.maxOracleMismatch, "\(text): marked known-difference, but it matches now (remove the mark)")
            } else {
                XCTAssertLessThanOrEqual(m.share, Self.maxOracleMismatch, text)
            }
        }
        for prepared in doc.orderedPages {
            let i = Int(prepared.page.index)
            let fallback = prepared.needsPDFFallback(forms: doc.forms)
            var r = PageRow(page: i + 1, problems: prepared.problems, incomplete: fallback, ownPixels: [:])
            XCTAssertEqual(prepared.problems, [], "\(name) p\(i + 1): resources that do not resolve")
            if fallback {
                XCTAssertTrue(fallbackExpected, "\(name) p\(i + 1): the page (or a form) is drawn from the PDF instead")
                if let oracle, let page = oracle.page(at: i + 1), let mine = own.page(at: i + 1),
                   let a = DL3Renderer.rasterize(pdfPage: mine, scale: 2), let b = DL3Renderer.rasterize(pdfPage: page, scale: 2),
                   a.width == b.width, a.height == b.height {
                    let m = Self.mismatch(DL3Parity.rgba(a), DL3Parity.rgba(b), width: a.width, height: a.height, block: 2)
                    r.oracleMismatch = m.share; r.oracleInk = m.ink
                    judge(i, m, " (from FlashTeX's PDF)")
                    if m.share > 0 { Self.write(a, "\(name)-p\(i + 1)-flashtexpdf"); Self.write(b, "\(name)-p\(i + 1)-xelatex") }
                }
                row.rows.append(r)
                continue
            }
            XCTAssertFalse(fallbackExpected, "\(name): marked pdf-fallback, but p\(i + 1) draws from the display list (remove the mark)")
            for scale in Self.scales {
                guard let page = own.page(at: i + 1),
                      let a = DL3Renderer.rasterize(prepared, forms: doc.forms, scale: scale),
                      let b = DL3Renderer.rasterize(pdfPage: page, scale: scale) else { XCTFail("\(name) p\(i + 1): render"); continue }
                let d = DL3Parity.diff(DL3Parity.rgba(a), DL3Parity.rgba(b))
                r.ownPixels[scale] = d.pixels
                let within = d.pixels <= Self.maxOwnPixels || (d.pixels <= tolerance && d.maxDelta <= 1)
                XCTAssertTrue(within, "\(name) p\(i + 1) at \(scale)x vs FlashTeX's PDF: \(d.pixels) px differ (max Δ \(d.maxDelta))")
                if d.pixels > 0 { Self.write(a, "\(name)-p\(i + 1)-\(scale)x-preview"); Self.write(b, "\(name)-p\(i + 1)-\(scale)x-flashtex") }
            }
            if let oracle, let page = oracle.page(at: i + 1),
               let a = DL3Renderer.rasterize(prepared, forms: doc.forms, scale: 2),
               let b = DL3Renderer.rasterize(pdfPage: page, scale: 2) {
                if a.width == b.width, a.height == b.height {
                    let m = Self.mismatch(DL3Parity.rgba(a), DL3Parity.rgba(b), width: a.width, height: a.height, block: 2)
                    r.oracleMismatch = m.share; r.oracleInk = m.ink
                    judge(i, m, "")
                } else {
                    XCTFail("\(name) p\(i + 1): page size \(a.width)x\(a.height) vs xelatex's \(b.width)x\(b.height)")
                }
                if (r.oracleMismatch ?? 1) > 0 { Self.write(a, "\(name)-p\(i + 1)-preview"); Self.write(b, "\(name)-p\(i + 1)-xelatex") }
            }
            row.rows.append(r)
        }
        return row
    }

    func testUnicodeModePreviewMatchesThePDFs() throws {
        let dir = URL(fileURLWithPath: Self.env["FLASHTEX_DL3_UNICODE"] ?? Self.repoRoot.appendingPathComponent("target/dl3-unicode").path)
        let names = ((try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? []).sorted().filter {
            FileManager.default.fileExists(atPath: dir.appendingPathComponent($0).appendingPathComponent("display.dl3").path)
        }
        if names.isEmpty {
            if Self.env["FLASHTEX_UNICODE_PARITY_REQUIRE"] == "1" { XCTFail("no display lists under \(dir.path)"); return }
            throw XCTSkip("no \(dir.path): run tools/displaylist/unicode_preview_parity.py first")
        }
        var rows: [CaseRow] = []
        for name in names { rows.append(try check(dir.appendingPathComponent(name), name: name)) }
        if let out = Self.env["FLASHTEX_V3_PARITY_OUT"] {
            let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted, .sortedKeys]
            try enc.encode(rows).write(to: URL(fileURLWithPath: out + ".unicode.json"))
        }
        for c in rows {
            let own = c.rows.map { r in Self.scales.map { "\(r.ownPixels[$0] ?? -1)" }.joined(separator: "/") }.joined(separator: " ")
            let ora = c.rows.map { $0.oracleMismatch.map { String(format: "%.2f%%", $0 * 100) } ?? "-" }.joined(separator: " ")
            print("unicode parity \(c.name): \(c.pages) pages; vs FlashTeX PDF px \(own); vs xelatex \(ora)")
        }
    }
}
