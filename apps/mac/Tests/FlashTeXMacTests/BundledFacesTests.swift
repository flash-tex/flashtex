import CoreText
import CryptoKit
import Foundation
import XCTest
@testable import FlashTeXMac

/// Every Latin Modern face the layout producer can request is vendored in
/// `apps/mac/Fonts`, pinned (`SUPPLEMENTARY-FACES.json` + the Commander
/// manifest), and resolves in CoreText to the real master — so a Mac without
/// MacTeX never substitutes a text face (pdf-2 finding: HW1 needs
/// `lmroman8-regular.otf`, footnotes need `lmroman6`/`lmroman5`).
final class BundledFacesTests: XCTestCase {
    private static let macDir = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    private static let fontsDir = macDir.appendingPathComponent("Fonts")
    private static let repoRoot = macDir.deletingLastPathComponent().deletingLastPathComponent()

    /// The Commander's pinned OTFs (`native-assets/manifest.json`).
    private static let commanderPinned: [String: (sha256: String, bytes: Int)] = [
        "lmroman10-regular.otf": ("1aa18cfefa58132c52ce5de70db1fd1154201c19cd2b2cdaffba4906a33e6852", 111536),
        "lmroman12-regular.otf": ("e6be218ae83e61aa8a29990d3cdc401c678c1962188cb9a4a8b6359e4f5e5870", 110400),
        "latinmodern-math.otf": ("6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f", 733736),
    ]

    private static func sha256Hex(_ data: Data) -> String {
        SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }

    // MARK: vendored faces

    /// The producer's enumeration (`fonts.rs` @ 9aaec57a) is exactly the set
    /// of OTFs in the directory, each file matches its pin, and no OTF is
    /// unpinned.
    func testEveryProducerRequestableFaceIsVendoredAndPinned() throws {
        let files = try FileManager.default.contentsOfDirectory(atPath: Self.fontsDir.path)
        let onDisk = Set(files.filter { $0.hasSuffix(".otf") })
        XCTAssertEqual(onDisk, Set(PreviewFonts.latinModernFaceFiles), "vendored OTFs must be exactly the requestable faces")
        // 21 roman text masters + bold-italic is counted among them + LM Math
        // + NewCM Math + 10 typewriter + 11 non-upright roman + 14 sans.
        XCTAssertEqual(PreviewFonts.latinModernFaceFiles.count, 58)
        XCTAssertEqual(PreviewFonts.latinModernMonoFaceFiles.count, 10)
        XCTAssertEqual(PreviewFonts.latinModernRomanShapeFaceFiles.count, 11)
        XCTAssertEqual(PreviewFonts.latinModernSansFaceFiles.count, 14)
        XCTAssertEqual(PreviewFonts.latinModernMissingFaces(in: Self.fontsDir.path), [])

        for (name, pin) in Self.commanderPinned {
            let data = try Data(contentsOf: Self.fontsDir.appendingPathComponent(name))
            XCTAssertEqual(data.count, pin.bytes, name)
            XCTAssertEqual(Self.sha256Hex(data), pin.sha256, name)
        }

        let pinURL = Self.fontsDir.appendingPathComponent("SUPPLEMENTARY-FACES.json")
        let doc = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf: pinURL)) as? [String: Any])
        XCTAssertEqual(doc["schema_version"] as? Int, 1)
        XCTAssertNotNil((doc["provenance"] as? [String: Any])?["copied_from"])
        let entries = try XCTUnwrap(doc["entries"] as? [[String: Any]])
        var listed = Set<String>()
        for e in entries {
            let path = try XCTUnwrap(e["path"] as? String)
            XCTAssertFalse(path.contains("/"), "flat file name expected: \(path)")
            XCTAssertNil(Self.commanderPinned[path], "\(path) is Commander-pinned, not supplementary")
            XCTAssertTrue(listed.insert(path).inserted, "duplicate \(path)")
            let data = try Data(contentsOf: Self.fontsDir.appendingPathComponent(path))
            XCTAssertEqual(data.count, e["byte_length"] as? Int, path)
            XCTAssertEqual(Self.sha256Hex(data), e["sha256"] as? String, path)
        }
        XCTAssertEqual(listed.union(Self.commanderPinned.keys), onDisk, "every vendored OTF is pinned by one tier")
        XCTAssertEqual(entries.count, 55)
    }

    /// `bundle-texmf.py check` with the fonts directory verifies every tier
    /// and refuses a directory holding an unpinned or altered face.
    func testPackagingCheckVerifiesFacesAndRefusesDrift() throws {
        let tool = Self.macDir.appendingPathComponent("scripts/bundle-texmf.py").path
        let texmf = Self.fontsDir.appendingPathComponent("texmf").path
        let verified = try Self.runPython(tool, ["check", texmf, Self.fontsDir.path])
        XCTAssertEqual(verified.status, 0, verified.output)
        let report = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(verified.output.utf8)) as? [String: Any])
        let rows = try XCTUnwrap(report["entries"] as? [[String: Any]])
        XCTAssertEqual(rows.filter { $0["tier"] as? String == "supplementary-face" }.count, 55)
        XCTAssertEqual(rows.filter { $0["tier"] as? String == "pinned" && ($0["bundle_path"] as? String ?? "").hasPrefix("Fonts/") }.count, 3)
        XCTAssertTrue(rows.allSatisfy { $0["status"] as? String == "verified" })

        let copy = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-faces-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: copy, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: copy) }
        for f in try FileManager.default.contentsOfDirectory(atPath: Self.fontsDir.path) where f.hasSuffix(".otf") || f.hasSuffix(".json") || f.hasSuffix(".TXT") {
            try FileManager.default.copyItem(at: Self.fontsDir.appendingPathComponent(f), to: copy.appendingPathComponent(f))
        }
        // One byte appended to the 8 pt face: refused, and the row names it.
        let handle = try FileHandle(forWritingTo: copy.appendingPathComponent("lmroman8-regular.otf"))
        try handle.seekToEnd(); try handle.write(contentsOf: Data([0])); try handle.close()
        let drifted = try Self.runPython(tool, ["check", texmf, copy.path])
        XCTAssertEqual(drifted.status, 1)
        XCTAssertTrue(drifted.output.contains("\"path\": \"lmroman8-regular.otf\""), drifted.output)
        XCTAssertTrue(drifted.output.contains("length_mismatch"), drifted.output)
        // An unpinned extra face: refused as `unpinned`. The name must be one
        // no tier can ever pin — `latin_modern_outline` has no arm returning a
        // dunhill design, so `lmdunh10-regular.otf` is never vendored.
        try FileManager.default.copyItem(at: Self.fontsDir.appendingPathComponent("lmroman9-regular.otf"),
                                         to: copy.appendingPathComponent("lmdunh10-regular.otf"))
        let stray = try Self.runPython(tool, ["check", texmf, copy.path])
        XCTAssertEqual(stray.status, 1)
        XCTAssertTrue(stray.output.contains("\"status\": \"unpinned\""), stray.output)
    }

    private static func runPython(_ script: String, _ args: [String]) throws -> (status: Int32, output: String) {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/python3")
        p.arguments = [script] + args
        let out = Pipe()
        p.standardOutput = out
        p.standardError = FileHandle.nullDevice
        try p.run()
        let data = out.fileHandleForReading.readDataToEndOfFile()
        p.waitUntilExit()
        return (p.terminationStatus, String(decoding: data, as: UTF8.self))
    }

    // MARK: preview resolution

    /// The preview's master boundaries are the producer's (`t1lmr.fd`):
    /// the same file the producer loads is the face the preview draws with.
    func testMasterBoundariesMirrorTheProducer() {
        let cases: [(size: Double, bold: Bool, italic: Bool, expect: String)] = [
            (12, false, false, "LMRoman12-Regular"), (10, false, false, "LMRoman10-Regular"),
            (17.28, false, false, "LMRoman17-Regular"), (14.4, true, false, "LMRoman12-Bold"),
            (8, false, false, "LMRoman8-Regular"), (6, false, false, "LMRoman6-Regular"),
            (5, false, false, "LMRoman5-Regular"), (11, false, false, "LMRoman12-Regular"),
            (9, true, false, "LMRoman9-Bold"), (5, true, false, "LMRoman5-Bold"),
            (24, true, false, "LMRoman12-Bold"), (7, false, true, "LMRoman7-Italic"),
            (6, false, true, "LMRoman7-Italic"), (8, false, true, "LMRoman8-Italic"),
            (12, true, true, "LMRoman10-BoldItalic"), (8, true, true, "LMRoman10-BoldItalic"),
        ]
        for c in cases {
            XCTAssertEqual(PreviewFonts.postScriptName(face: .latinModern, size: c.size, bold: c.bold, italic: c.italic), c.expect,
                           "\(c.size) pt bold=\(c.bold) italic=\(c.italic)")
        }
    }

    /// Every master/style name the preview can ask for is a registered
    /// CoreText face from the vendored directory — never a fallback that
    /// merely keeps the name.
    func testEveryPreviewMasterResolvesToARealCoreTextFace() throws {
        XCTAssertTrue(PreviewFonts.latinModernRegistered, "apps/mac/Fonts ships lmroman*.otf")
        let dir = try XCTUnwrap(PreviewFonts.latinModernDirectory)
        guard dir == Self.fontsDir.path else {
            throw XCTSkip("Latin Modern registered from \(dir), not the repository copy (FLASHTEX_LM_DIR/bundle); its gaps: \(PreviewFonts.latinModernMissingFaces)")
        }
        XCTAssertEqual(PreviewFonts.latinModernMissingFaces, [])
        var seen = Set<String>()
        for size in [4.0, 5, 5.5, 6, 6.5, 7, 7.5, 8, 8.5, 9, 9.5, 10, 10.95, 11, 12, 14.4, 15, 17.28, 20.74, 24.88] {
            for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
                let name = PreviewFonts.postScriptName(face: .latinModern, size: size, bold: bold, italic: italic)
                let font = CTFontCreateWithName(name as CFString, size, nil)
                XCTAssertEqual(CTFontCopyPostScriptName(font) as String, name, "\(size) pt bold=\(bold) italic=\(italic)")
                XCTAssertEqual(CTFontCopyFamilyName(font) as String, "Latin Modern Roman", name)
                let file = name.lowercased() + ".otf"
                XCTAssertTrue(PreviewFonts.latinModernFaceFiles.contains(file), "\(name) is not a producer-requestable file")
                seen.insert(file)
            }
        }
        XCTAssertEqual(seen.count, 21, "all 21 text masters reachable: \(seen.sorted())")
    }

    // MARK: real producer

    private static let missingCodes: Set<String> = ["tfm_missing", "required_metrics_unavailable", "font_unavailable", "math_font_unavailable"]

    private struct ProducerRun {
        var results = 0
        var faces = Set<String>()
        var missing: [(id: String, code: String, message: String)] = []
    }

    /// Runs `flashtex-render` with host TeX denied by sandbox-exec and an
    /// `env -i`-style environment, `FLASHTEX_FONT_DIRS` = `fontDir`.
    private func runProducer(_ executable: String, fontDir: String, home: URL, requests: [String]) throws -> ProducerRun {
        let profile = home.appendingPathComponent("no-host-tex.sb")
        try """
        (version 1)
        (allow default)
        (deny file-read* (subpath "/usr/local/texlive"))
        (deny file-read* (subpath "/Library/TeX"))
        (deny file-read* (subpath "/usr/share/texmf"))
        (deny file-read* (subpath "/usr/share/texlive"))

        """.write(to: profile, atomically: true, encoding: .utf8)
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/sandbox-exec")
        process.arguments = ["-f", profile.path, executable]
        process.environment = ["PATH": "/usr/bin:/bin", "HOME": home.path, "FLASHTEX_FONT_DIRS": fontDir,
                               "FLASHTEX_TFM_DIRS": Self.fontsDir.appendingPathComponent("texmf/fonts/tfm/public/lm").path]
        process.currentDirectoryURL = home
        let stdin = Pipe(), stdout = Pipe()
        process.standardInput = stdin
        process.standardOutput = stdout
        process.standardError = FileHandle.nullDevice
        try process.run()
        try stdin.fileHandleForWriting.write(contentsOf: Data((requests.joined(separator: "\n") + "\n").utf8))
        try stdin.fileHandleForWriting.close()
        let output = stdout.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        var run = ProducerRun()
        for line in output.split(separator: UInt8(ascii: "\n")) {
            guard let obj = try JSONSerialization.jsonObject(with: Data(line)) as? [String: Any],
                  let kind = obj["type"] as? String else { continue }
            if kind == "display_list" {
                for font in (obj["payload"] as? [String: Any])?["fonts"] as? [[String: Any]] ?? [] {
                    if let name = font["postscript_name"] as? String { run.faces.insert(name) }
                }
                continue
            }
            guard kind == "compile_result" else { continue }
            run.results += 1
            let id = obj["id"] as? String ?? "?"
            for d in (obj["payload"] as? [String: Any])?["diagnostics"] as? [[String: Any]] ?? [] {
                let code = d["code"] as? String ?? ""
                let message = d["message"] as? String ?? ""
                // f762f82a reports optical Roman fallback as a profile warning.
                // Italic/symbol profile limitations are separate from bundling.
                let romanFallback = code == "math_resource_profile" && message.hasPrefix("lmr")
                if Self.missingCodes.contains(code) || romanFallback { run.missing.append((id, code, message)) }
            }
        }
        return run
    }

    private static func request(id: String, revision: Int, tex: String) throws -> String {
        let payload: [String: Any] = ["project_id": "p", "revision": revision, "entry_path": "main.tex",
                                      "documents": [["path": "main.tex", "text": tex]],
                                      "layout_capabilities": ["display-list-v2"]]
        let data = try JSONSerialization.data(withJSONObject: ["protocol_version": 1, "id": id, "type": "compile", "payload": payload])
        return String(decoding: data, as: UTF8.self)
    }

    /// Exercises nested 12 pt math scripts and HW1 with an optical-capable
    /// producer. Requires actual emitted Roman8/Roman6 faces before testing
    /// removal; profiles lacking optical support are explicitly skipped.
    /// Removing Roman8 must produce a missing-face or lmr8 profile warning.
    func testRealProducerNeedsNoSubstitutionForSmallMastersWithHostTeXExcluded() throws {
        guard let render = ProcessInfo.processInfo.environment["FLASHTEX_RENDER"],
              FileManager.default.isExecutableFile(atPath: render) else {
            throw XCTSkip("FLASHTEX_RENDER not set to a built flashtex-render")
        }
        guard FileManager.default.isExecutableFile(atPath: "/usr/bin/sandbox-exec") else { throw XCTSkip("sandbox-exec unavailable") }
        let home = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-faces-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: home, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: home) }

        var requests = [
            try Self.request(id: "small-10pt", revision: 1, tex: "\\documentclass{article}\n\\begin{document}\nBody. {\\footnotesize footnote \\textbf{bold} \\textit{italic}} {\\scriptsize script \\textbf{bold} \\textit{italic}} {\\tiny tiny \\textbf{bold}} {\\small small \\textit{italic}}\n\\end{document}\n"),
            try Self.request(id: "small-12pt", revision: 2, tex: "\\documentclass[12pt]{article}\n\\begin{document}\nBody $x_{1_{2}} + \\frac{1}{2}$.\n\\end{document}\n"),
            try Self.request(id: "small-11pt", revision: 3, tex: "\\documentclass[11pt]{article}\n\\begin{document}\nBody. {\\footnotesize footnote \\textbf{bold} \\textit{italic}} {\\scriptsize script} {\\tiny tiny \\textbf{bold}}\n\\end{document}\n"),
        ]
        // The committed homework (11 pt article: sub/superscripts set in 8 pt)
        // is what exercises Roman8; a checkout whose HW1.tex is a stub cannot,
        // so its shape goes into the message rather than a misleading verdict.
        let hw1 = Self.repoRoot.appendingPathComponent("fixtures/real-world/hw1/HW1.tex")
        var hw1Shape = "HW1.tex not readable at \(hw1.path)"
        if let tex = try? String(contentsOf: hw1, encoding: .utf8) {
            requests.append(try Self.request(id: "hw1", revision: 4, tex: tex))
            let lines = tex.split(separator: "\n", omittingEmptySubsequences: false).count
            hw1Shape = "HW1.tex is \(lines) lines / \(tex.utf8.count) bytes" + (tex.contains("\\documentclass[11pt]{article}") ? "" : " and is NOT the committed 11 pt homework")
        }

        let vendored = try runProducer(render, fontDir: Self.fontsDir.path, home: home, requests: requests)
        XCTAssertEqual(vendored.results, requests.count)
        XCTAssertTrue(vendored.missing.isEmpty, "vendored faces must leave no missing-font/metric diagnostics: \(vendored.missing)")
        guard vendored.faces.contains("LMRoman8-Regular"), vendored.faces.contains("LMRoman6-Regular") else {
            throw XCTSkip("Configured producer does not emit both Roman8/Roman6 faces; optical gate requires f762f82a support. Emitted: \(vendored.faces.sorted())")
        }

        // The same requests against a copy without the 8 pt regular face: the
        // fixtures really exercise it, and the producer says so explicitly.
        let copy = home.appendingPathComponent("Fonts")
        try FileManager.default.createDirectory(at: copy, withIntermediateDirectories: true)
        for f in PreviewFonts.latinModernFaceFiles where f != "lmroman8-regular.otf" {
            try FileManager.default.copyItem(at: Self.fontsDir.appendingPathComponent(f), to: copy.appendingPathComponent(f))
        }
        let removed = try runProducer(render, fontDir: copy.path, home: home, requests: requests)
        XCTAssertEqual(removed.results, requests.count)
        XCTAssertTrue(removed.missing.contains { $0.id == "small-12pt" && ($0.message.contains("lmroman8-regular.otf") || $0.message.hasPrefix("lmr8:")) },
                      "12 pt optical script must report missing Roman8: \(removed.missing)")
        if requests.count == 4 {
            XCTAssertTrue(removed.missing.contains { $0.id == "hw1" && ($0.message.contains("lmroman8-regular.otf") || $0.message.hasPrefix("lmr8:")) },
                          "HW1 must report missing Roman8 (the pdf-2 finding): \(removed.missing); \(hw1Shape)")
        }
    }
}
