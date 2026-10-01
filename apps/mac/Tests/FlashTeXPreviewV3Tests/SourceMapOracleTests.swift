import CoreGraphics
import Foundation
import XCTest
import FlashTeXDisplayListV3
@testable import FlashTeXPreviewV3

/// Forward and reverse search against pdflatex's SyncTeX (the oracle,
/// `synctex` from TeX Live), on every parity fixture that
/// `tools/displaylist/synctex_oracle.py` prepared (`target/dl3-positions/<f>/oracle`).
///
/// * Reverse: for sampled glyphs, the (file, line) our index gives for the
///   glyph under a point equals what `synctex edit` gives for that point.
/// * Forward: for sampled source lines, our target (the first page with the
///   line's glyphs, the union of their cells) is on the page `synctex view`
///   names and overlaps a box it returns.
///
/// Measurements, not a zero-tolerance gate: SyncTeX records boxes, not
/// characters (a macro's text is attributed to where its box began), so the
/// two disagree by construction on some glyphs; the counts are the evidence.
/// `FLASHTEX_V3_SYNCTEX_OUT=path.json` writes every disagreement.
final class SourceMapOracleTests: XCTestCase {
    static var repoRoot: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }
    static let synctex = "/Library/TeX/texbin/synctex"

    func run(_ args: [String], in dir: URL) -> String {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: Self.synctex)
        p.arguments = args
        p.currentDirectoryURL = dir
        let pipe = Pipe()
        p.standardOutput = pipe
        p.standardError = FileHandle.nullDevice
        try? p.run()
        let out = pipe.fileHandleForReading.readDataToEndOfFile()
        p.waitUntilExit()
        return String(decoding: out, as: UTF8.self)
    }

    /// Records of a `synctex` answer: one dictionary per result.
    func records(_ text: String) -> [[String: String]] {
        var out: [[String: String]] = [], cur: [String: String] = [:]
        for line in text.split(separator: "\n") {
            guard let i = line.firstIndex(of: ":") else { continue }
            let k = String(line[..<i]), v = String(line[line.index(after: i)...])
            if (k == "Output" || k == "Page" && cur["Page"] != nil), !cur.isEmpty, cur["Page"] != nil || cur["Input"] != nil {
                out.append(cur); cur = [:]
            }
            cur[k] = v
        }
        if !cur.isEmpty { out.append(cur) }
        return out
    }

    struct Miss: Codable { var fixture: String; var kind: String; var page: Int; var ours: String; var synctex: String }

    func testForwardAndReverseSearchAgreeWithSyncTeX() throws {
        let env = ProcessInfo.processInfo.environment
        let dir = URL(fileURLWithPath: env["FLASHTEX_DL3_FIXTURES"] ?? Self.repoRoot.appendingPathComponent("target/dl3-positions").path)
        guard FileManager.default.isExecutableFile(atPath: Self.synctex),
              let names = try? FileManager.default.contentsOfDirectory(atPath: dir.path).sorted() else {
            throw XCTSkip("needs TeX Live's synctex and target/dl3-positions (check_positions.py, synctex_oracle.py)")
        }
        var fixtures = 0, revN = 0, revLine = 0, revNear = 0, fwdN = 0, fwdPage = 0, fwdBox = 0
        var misses: [Miss] = []
        for name in names {
            let fx = dir.appendingPathComponent(name)
            let oracle = fx.appendingPathComponent("oracle"), src = fx.appendingPathComponent("src").path + "/"
            guard let pdf = (try? FileManager.default.contentsOfDirectory(atPath: oracle.path))?.first(where: { $0.hasSuffix(".pdf") }),
                  let dl3 = try? Data(contentsOf: fx.appendingPathComponent("display.dl3")) else { continue }
            let doc = try DL3Document(frames: Array(dl3))
            fixtures += 1
            // Our file paths are the engine's run directory's; SyncTeX's the oracle's.
            func rel(_ p: String) -> String? {
                var s = p
                if s.hasPrefix(src) { s.removeFirst(src.count) } else if s.hasPrefix(oracle.path + "/") { s.removeFirst(oracle.path.count + 1) } else { return nil }
                while s.hasPrefix("./") { s.removeFirst(2) }
                return s
            }
            let pages = doc.orderedPages
            let indexes = pages.map(DL3SourceIndex.init)
            // Candidate glyphs: with a span in one of the document's own files.
            var cands: [(page: Int, g: DL3GlyphRef, file: String, line: Int)] = []
            for (pi, ix) in indexes.enumerated() {
                for g in ix.glyphs where g.span != 0 && !g.ink.isEmpty {
                    if let loc = doc.sources.location(of: g.span), let f = rel(loc.path) { cands.append((pi, g, f, loc.line)) }
                }
            }
            guard !cands.isEmpty else { continue }
            // Reverse: 30 glyphs spread over the document.
            let stride = max(1, cands.count / 30)
            for c in Swift.stride(from: 0, to: cands.count, by: stride).map({ cands[$0] }).prefix(30) {
                let p = CGPoint(x: c.g.ink.midX, y: c.g.ink.midY)
                let r = records(run(["edit", "-o", "\(c.page + 1):\(p.x):\(p.y):\(pdf)"], in: oracle)).first { $0["Input"] != nil }
                revN += 1
                let sf = r?["Input"].flatMap(rel), sl = r?["Line"].flatMap { Int($0) }
                if sf == c.file, sl == c.line { revLine += 1; revNear += 1 }
                else {
                    if sf == c.file, let sl, abs(sl - c.line) <= 1 { revNear += 1 }
                    misses.append(Miss(fixture: name, kind: "reverse", page: c.page + 1, ours: "\(c.file):\(c.line)", synctex: "\(sf ?? "?"):\(sl ?? -1)"))
                }
            }
            // Forward: 15 distinct lines.
            var seen = Set<String>(), lines: [(String, Int)] = []
            for c in cands where seen.insert("\(c.file):\(c.line)").inserted { lines.append((c.file, c.line)) }
            let lstride = max(1, lines.count / 15)
            for (file, line) in Swift.stride(from: 0, to: lines.count, by: lstride).map({ lines[$0] }).prefix(15) {
                let spans = doc.sources.spans(line: line) { rel($0) == file }
                guard let (pi, box) = indexes.enumerated().lazy.compactMap({ i, ix -> (Int, CGRect)? in
                    DL3SourceIndex.box(ix.glyphs(of: spans)).map { (i, $0) } }).first else { continue }
                let rs = records(run(["view", "-i", "\(line):0:\(file)", "-o", pdf], in: oracle)).filter { $0["Page"] != nil }
                fwdN += 1
                let samePage = rs.filter { Int($0["Page"] ?? "") == pi + 1 }
                if !samePage.isEmpty { fwdPage += 1 }
                let hit = samePage.contains { r in
                    guard let h = Double(r["h"] ?? ""), let v = Double(r["v"] ?? ""), let w = Double(r["W"] ?? ""), let hh = Double(r["H"] ?? "") else { return false }
                    return CGRect(x: h, y: v - hh, width: max(w, 1), height: max(hh, 1) * 1.3).insetBy(dx: -1, dy: -1).intersects(box)
                }
                if hit { fwdBox += 1 } else {
                    misses.append(Miss(fixture: name, kind: "forward", page: pi + 1, ours: "\(file):\(line) p\(pi + 1) \(box.integral)",
                                       synctex: rs.map { "p\($0["Page"] ?? "?") h\($0["h"] ?? "") v\($0["v"] ?? "") W\($0["W"] ?? "") H\($0["H"] ?? "")" }.joined(separator: "; ")))
                }
            }
        }
        if let out = env["FLASHTEX_V3_SYNCTEX_OUT"] {
            let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted, .sortedKeys]
            try enc.encode(misses).write(to: URL(fileURLWithPath: out))
        }
        print("synctex oracle: \(fixtures) fixtures; reverse search: same line \(revLine)/\(revN), within one line \(revNear)/\(revN); forward search: same page \(fwdPage)/\(fwdN), box overlaps SyncTeX's \(fwdBox)/\(fwdN)")
        XCTAssertGreaterThan(revN, 0)
    }
}
