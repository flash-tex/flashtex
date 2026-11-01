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
        // + NewCM Math + 10 typewriter + 11 non-upright roman + 14 sans
        // + 6 TeX Gyre (the Core 14 faces' programs).
        XCTAssertEqual(PreviewFonts.latinModernFaceFiles.count, 64)
        XCTAssertEqual(PreviewFonts.texGyreFaceFiles.count, 6)
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
        XCTAssertEqual(entries.count, 61)
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
        XCTAssertEqual(rows.filter { $0["tier"] as? String == "supplementary-face" }.count, 61)
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
}
