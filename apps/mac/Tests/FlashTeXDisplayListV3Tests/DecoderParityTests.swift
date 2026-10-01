import Foundation
import XCTest
@testable import FlashTeXDisplayListV3

/// The Swift decoder against the Rust reference decoder
/// (`crates/display-list-v3`): equal canonical texts (`dl3-dump --canonical`,
/// every decoded field, floats bit for bit) on
///
/// * the checked-in fixture (always): `Fixtures/beamer-overlays.dl3`, the
///   engine's display list of `fixtures/real-world/beamer-overlays`, whose
///   Rust canonical text has the SHA-256 below;
/// * every parity fixture's display list, when they exist: the directories
///   `tools/displaylist/check_positions.py` leaves in `target/dl3-positions/`
///   (or `FLASHTEX_DL3_FIXTURES`), each compared with `target/release/dl3-dump
///   --canonical` (or `FLASHTEX_DL3_DUMP`) run on the same file. Skipped, not
///   failed, when neither is built.
final class DecoderParityTests: XCTestCase {
    static let beamerOverlaysCanonicalSHA256 = "b5ec8800a20c2cc771fb266c92db720a0bc05979c9c2f99df0c2da200636dcd1"

    static var repoRoot: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }

    func fixture(_ name: String) throws -> [UInt8] {
        let url = try XCTUnwrap(Bundle.module.url(forResource: "Fixtures/\(name)", withExtension: nil))
        return Array(try Data(contentsOf: url))
    }

    func testCheckedInFixtureMatchesTheReferenceDecoder() throws {
        let text = try DL3Canonical.text(ofFile: fixture("beamer-overlays.dl3"))
        XCTAssertEqual(DL3Hex.string(DL3Canonical.sha256(Array(text.utf8))), Self.beamerOverlaysCanonicalSHA256)
        // Spot checks that the canonical text is what it says.
        let frames = try DL3Frames.split(fixture("beamer-overlays.dl3"))
        let pages = try frames.compactMap { f -> DL3Page? in
            if case .page(let p) = try DL3Event.decode(kind: f.kind, body: f.body) { return p }
            return nil
        }
        XCTAssertEqual(pages.map(\.index), Array(0 ..< UInt32(pages.count)))
        XCTAssertGreaterThan(pages.first?.items.count ?? 0, 10)
    }

    func testEveryParityFixtureMatchesTheReferenceDecoder() throws {
        let env = ProcessInfo.processInfo.environment
        let dir = URL(fileURLWithPath: env["FLASHTEX_DL3_FIXTURES"] ?? Self.repoRoot.appendingPathComponent("target/dl3-positions").path)
        let dump = env["FLASHTEX_DL3_DUMP"] ?? Self.repoRoot.appendingPathComponent("target/release/dl3-dump").path
        let fm = FileManager.default
        guard fm.isExecutableFile(atPath: dump),
              let names = try? fm.contentsOfDirectory(atPath: dir.path).sorted() else {
            throw XCTSkip("no \(dir.path) or \(dump): run tools/displaylist/check_positions.py and cargo build --release -p flashtex-display-list")
        }
        var files = 0, pages = 0, items = 0, glyphs = 0
        for name in names {
            let file = dir.appendingPathComponent(name).appendingPathComponent("display.dl3")
            guard fm.fileExists(atPath: file.path) else { continue }
            let p = Process()
            p.executableURL = URL(fileURLWithPath: dump)
            p.arguments = ["--canonical", file.path]
            let pipe = Pipe()
            p.standardOutput = pipe
            try p.run()
            let rust = pipe.fileHandleForReading.readDataToEndOfFile()
            p.waitUntilExit()
            XCTAssertEqual(p.terminationStatus, 0, name)
            let bytes = Array(try Data(contentsOf: file))
            let swift = try DL3Canonical.text(ofFile: bytes)
            if Data(swift.utf8) != rust {
                let r = String(decoding: rust, as: UTF8.self).split(separator: "\n", omittingEmptySubsequences: false)
                let s = swift.split(separator: "\n", omittingEmptySubsequences: false)
                let i = (0 ..< min(r.count, s.count)).first { r[$0] != s[$0] } ?? min(r.count, s.count)
                XCTFail("\(name): canonical texts differ at line \(i + 1):\n rust:  \(i < r.count ? String(r[i].prefix(200)) : "<end>")\n swift: \(i < s.count ? String(s[i].prefix(200)) : "<end>")")
                continue
            }
            files += 1
            for f in try DL3Frames.split(bytes) where f.kind == DL3.Kind.page {
                let page = try DL3Page.decode(kind: .page, body: f.body)
                pages += 1
                items += page.items.count
                glyphs += page.items.reduce(0) { if case .glyph = $1 { $0 + 1 } else { $0 } }
            }
        }
        XCTAssertGreaterThan(files, 0)
        print("dl3 decoder parity: \(files) fixtures identical to dl3-dump --canonical; \(pages) pages, \(items) items, \(glyphs) glyphs")
    }

    func testDecodingFailsClosed() throws {
        let bytes = try fixture("beamer-overlays.dl3")
        let frames = try DL3Frames.split(bytes)
        let page = try XCTUnwrap(frames.first { $0.kind == DL3.Kind.page })
        // Truncated body.
        XCTAssertThrowsError(try DL3Page.decode(kind: .page, body: Array(page.body.dropLast(3))))
        // Truncated frame.
        XCTAssertThrowsError(try DL3Frames.split(Array(bytes.dropLast(1))))
        // An unknown item opcode: header (124) + n + ITEMS section with 0xFF.
        var body = Array(page.body.prefix(120))
        body += [1, 0, 0, 0] // one section
        body += [3, 0, 0, 0, 1, 0, 0, 0, 0xFF]
        XCTAssertThrowsError(try DL3Page.decode(kind: .page, body: body))
        // A path reference that does not exist.
        var bad = Array(page.body.prefix(120))
        bad += [1, 0, 0, 0, 3, 0, 0, 0, 5, 0, 0, 0, 0x03, 7, 0, 0, 0]
        XCTAssertThrowsError(try DL3Page.decode(kind: .page, body: bad))
        // An unknown section is skipped.
        var skip = Array(page.body.prefix(120))
        skip += [1, 0, 0, 0, 99, 0, 0, 0, 2, 0, 0, 0, 0xAA, 0xBB]
        XCTAssertEqual(try DL3Page.decode(kind: .page, body: skip).items.count, 0)
    }

    func testCompileRequestJSON() throws {
        var r = DL3CompileRequest(id: 7, root: "/p", main: "main.tex")
        r.viewport = 3
        r.edits = [.init(path: "main.tex", offset: 10, delete: 1, insert: "é")]
        let j = try DL3JSON.parse(Array(r.json.data()))
        XCTAssertEqual(j["id"]?.int, 7)
        XCTAssertEqual(j["incremental"]?.bool, true)
        XCTAssertEqual(j["viewport"]?.int, 3)
        XCTAssertEqual(j["edits"]?.array?.first?["insert"]?.string, "é")
        XCTAssertEqual(j["edits"]?.array?.first?["offset"]?.int, 10)
    }
}
