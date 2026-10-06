import XCTest
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

    /// The previous engine's list is the pinned inventory, unchanged, and is
    /// still the default (retirement plan #1236, S3r).
    func testTheOldEngineListIsUnchanged() {
        let old = CaptureFeatures.structures + CaptureFeatures.symbolLines() + CaptureFeatures.unsupported
        XCTAssertEqual(CaptureFeatures.supportedFeatures(for: .previous), old)
        XCTAssertEqual(CaptureFeatures.supportedFeatures(), old)
    }

    /// The new engine typesets what pdfLaTeX does with the document's own
    /// packages: no closed symbol list and none of the old compiler's
    /// "NOT supported" lines, the output and reporting rules kept, within the
    /// bridge's limits.
    func testTheNewEngineListOmitsTheOldLimits() {
        let features = CaptureFeatures.supportedFeatures(for: .new)
        XCTAssertLessThanOrEqual(features.count, 64)
        for f in features { XCTAssertLessThanOrEqual(f.utf8.count, 128, f) }
        XCTAssertEqual(features.first, CaptureFeatures.structures.first, "the output rule comes first")
        XCTAssertFalse(features.contains { $0.hasPrefix("NOT supported") })
        XCTAssertFalse(features.contains { $0.hasPrefix("symbols (only these render)") })
        XCTAssertTrue(features.contains { $0.hasPrefix("pdfLaTeX-compatible:") })
        XCTAssertTrue(features.contains { $0.contains("amsmath") })
        for rule in CaptureFeatures.unsupported where rule.hasPrefix("RULE:") {
            XCTAssertTrue(features.contains(rule), rule)
        }
        XCTAssertTrue(features.contains { $0.hasPrefix("RULE:") && $0.contains("\\usepackage") })
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
