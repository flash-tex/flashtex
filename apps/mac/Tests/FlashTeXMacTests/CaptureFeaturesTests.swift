import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// `supported_features` sent with every `capture_convert` (CaptureFeatures.swift).
@MainActor
final class CaptureFeaturesTests: XCTestCase {
    func testFeatureListFitsTheBridgeLimitsAndNamesEveryGlyph() {
        let features = CaptureFeatures.supportedFeatures()
        XCTAssertLessThanOrEqual(features.count, 64, "bridge context.rs: at most 64 entries")
        for f in features { XCTAssertLessThanOrEqual(f.utf8.count, 128, f) }
        let symbolText = CaptureFeatures.symbolLines().joined(separator: " ")
        for name in CaptureFeatures.commandGlyphs { XCTAssertTrue(symbolText.contains(" \\" + name), name) }
        XCTAssertEqual(CaptureFeatures.commandGlyphs.count, 60)
        XCTAssertEqual(Set(CaptureFeatures.commandGlyphs).count, 60, "no duplicates")
        XCTAssertTrue(features.contains { $0.hasPrefix("RULE:") && $0.contains("never substituted") })
        XCTAssertTrue(features.contains { $0.contains("gather") && $0.hasPrefix("NOT supported") })
        XCTAssertTrue(features.contains { $0.contains("\\mathbb") && $0.hasPrefix("NOT supported") })
        XCTAssertTrue(features.contains { $0.contains("\\frac") })
        XCTAssertTrue(CaptureFeatures.defaultInstructions.contains("report anything else in ambiguities"))
    }

    /// `capture_convert` names the document's engine; the bridge picks the
    /// provider's list for it (crates/bridge tests `the_documents_engine_picks…`).
    /// The field is additive: omitted when nil, so an old payload is unchanged.
    func testCaptureConvertCarriesTheEngine() throws {
        func json(_ c: TransferV1.CaptureConvert) throws -> [String: Any] {
            try XCTUnwrap(JSONSerialization.jsonObject(with: JSONEncoder().encode(c)) as? [String: Any])
        }
        let new = try json(.init(captureId: "c", supportedFeatures: [], engine: "new"))
        XCTAssertEqual(new["engine"] as? String, "new")
        XCTAssertEqual(new["capture_id"] as? String, "c")
        XCTAssertNil(try json(.init(captureId: "c", supportedFeatures: []))["engine"])
    }

    /// Re-derives the glyph names from the pinned compiler commit when this
    /// clone has it (`git show <sha>:crates/compiler/src/math.rs`).
    func testGlyphListMatchesThePinnedCompilerTable() throws {
        let git = Process()
        git.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        git.currentDirectoryURL = BridgeClientTests.repoRoot
        git.arguments = ["show", "\(CaptureFeatures.compilerSHA):crates/compiler/src/math.rs"]
        let out = Pipe()
        git.standardOutput = out; git.standardError = FileHandle.nullDevice
        try git.run()
        let source = String(decoding: out.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
        git.waitUntilExit()
        guard git.terminationStatus == 0, let start = source.range(of: "pub const COMMAND_GLYPHS: &[(&str, &str)] = &[") else {
            throw XCTSkip("commit \(CaptureFeatures.compilerSHA) is not in this clone; fetch origin/agent/claude/compiler-foundation")
        }
        let table = source[start.upperBound...].prefix { $0 != "]" }
        let names = table.split(separator: "\n").compactMap { line -> String? in
            guard let open = line.range(of: "(\""), let close = line[open.upperBound...].range(of: "\"") else { return nil }
            return String(line[open.upperBound..<close.lowerBound])
        }
        XCTAssertEqual(names, CaptureFeatures.commandGlyphs)
    }
}
