import XCTest
import CoreGraphics
import CoreText
import CryptoKit
import FlashTeXProtocol
@testable import FlashTeXMac

final class PreviewFontsTests: XCTestCase {
    func testLatinModernRegistersFromBasicTeXWhenPresent() throws {
        guard PreviewFonts.latinModernRegistered else {
            throw XCTSkip("Latin Modern not found in any search path on this machine")
        }
        // The registered face must resolve to the real font, not a fallback.
        let font = CTFontCreateWithName("LMRoman10-Regular" as CFString, 10, nil)
        let name = CTFontCopyPostScriptName(font) as String
        XCTAssertEqual(name, "LMRoman10-Regular")
        XCTAssertEqual(PreviewFonts.postScriptName(size: 12), PreviewFonts.active == .latinModern ? "LMRoman12-Regular" : "Times-Roman")
        XCTAssertEqual(PreviewFonts.postScriptName(size: 17.28, bold: true), PreviewFonts.active == .latinModern ? "LMRoman12-Bold" : "Times-Bold")
    }

    /// The compiler's `Latin Modern Math` hint draws from the bundled
    /// `lm.math` program itself, never from a roman master or a fallback.
    func testLatinModernMathHintDrawsTheBundledMathFont() throws {
        guard PreviewFonts.latinModernMathRegistered else {
            throw XCTSkip("latinmodern-math.otf not found in any search path on this machine")
        }
        let hint = RuntimeV1.PageItem.FontHint(family: "Latin Modern Math")
        let resolved = PreviewFonts.resolve(hint: hint, size: 12)
        XCTAssertEqual(resolved.postScriptName, "LatinModernMath-Regular")
        XCTAssertNil(resolved.substitution)
        let font = CTFontCreateWithName(resolved.postScriptName as CFString, 12, nil)
        XCTAssertEqual(CTFontCopyPostScriptName(font) as String, "LatinModernMath-Regular")
        for scalar in ["ℤ", "ℝ", "ℚ", "ℕ", "∖", "⟹", "𝔸"] {
            let units = Array(scalar.utf16)
            var glyphs = [CGGlyph](repeating: 0, count: units.count)
            XCTAssertTrue(CTFontGetGlyphsForCharacters(font, units, &glyphs, units.count), "\(scalar) has a glyph")
            XCTAssertNotEqual(glyphs[0], 0, "\(scalar) is not .notdef")
        }
        // The roman prefix match must not capture the math family.
        XCTAssertFalse(PreviewFonts.resolve(hint: .init(family: "Latin Modern Roman"), size: 12).postScriptName.hasPrefix("LatinModernMath"))
    }

    /// The secondary double-struck math face (`NewCMMath-Regular.otf`, msbm's
    /// `\mathbb` design) is vendored next to Latin Modern and resolves through
    /// `V2FontStore` by raw bytes only (GH31): the producer's `fonts[]` entry
    /// for it names sha256 60394d35…, and the loaded program is the real
    /// NewCM face with double-struck glyphs, never a CoreText substitute.
    func testNewComputerModernMathResolvesByRawBytesForMathbb() throws {
        let fontsDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().appendingPathComponent("Fonts")
        let file = fontsDir.appendingPathComponent(PreviewFonts.newComputerModernMathFile)
        guard let data = try? Data(contentsOf: file) else {
            throw XCTSkip("\(PreviewFonts.newComputerModernMathFile) not vendored in apps/mac/Fonts")
        }
        XCTAssertTrue(PreviewFonts.latinModernFaceFiles.contains(PreviewFonts.newComputerModernMathFile))
        XCTAssertEqual(V2FontStore.hex(SHA256.hash(data: data)), PreviewFonts.newComputerModernMathSHA256, "vendored bytes match the pin")
        XCTAssertEqual(data.count, 1_187_476)

        let store = V2FontStore(directories: [fontsDir.path])
        let discovered = try XCTUnwrap(store.fonts.first { $0.bytesSha256 == PreviewFonts.newComputerModernMathSHA256 })
        XCTAssertEqual(discovered.url.lastPathComponent, PreviewFonts.newComputerModernMathFile)
        // What flashtex-render publishes for the face (raw-byte sha256 as the id).
        let program = try XCTUnwrap(CGFont(CGDataProvider(data: data as CFData)!))
        let resource = RenderingV2.FontResource(fontId: PreviewFonts.newComputerModernMathSHA256,
                                                sha256: PreviewFonts.newComputerModernMathSHA256,
                                                byteLength: Int64(data.count), format: "opentype-cff", faceIndex: 0,
                                                unitsPerEm: Int(program.unitsPerEm), glyphCount: Int(program.numberOfGlyphs),
                                                postscriptName: PreviewFonts.newComputerModernMathPostScriptName)
        let resolved = try store.resolve(resource)
        XCTAssertEqual(resolved.file.bytesSha256, PreviewFonts.newComputerModernMathSHA256)
        XCTAssertEqual((resolved.cgFont.postScriptName as String?) ?? "", "NewCMMath-Regular")
        let font = resolved.ctFont(size: 12)
        for scalar in ["ℤ", "ℝ", "ℚ", "ℕ", "ℂ", "𝔸"] {
            let units = Array(scalar.utf16)
            var glyphs = [CGGlyph](repeating: 0, count: units.count)
            XCTAssertTrue(CTFontGetGlyphsForCharacters(font, units, &glyphs, units.count), "\(scalar) has a glyph")
            XCTAssertNotEqual(glyphs[0], 0, "\(scalar) is not .notdef")
        }
        // A wrong hash under the same name is not admitted (identity is the bytes).
        var wrong = resource
        wrong.sha256 = String(repeating: "0", count: 64)
        wrong.fontId = wrong.sha256
        XCTAssertThrowsError(try store.resolve(wrong))
    }

    func testTimesFallbackNamesAreBase14() {
        // Independent of what is installed, the Times mapping must be the Core-14 names.
        XCTAssertEqual(["Times-Roman", "Times-Bold", "Times-Italic", "Times-BoldItalic"],
                       [(false, false), (true, false), (false, true), (true, true)].map { b, i in
                           PreviewFonts.active == .times ? PreviewFonts.postScriptName(size: 12, bold: b, italic: i)
                               : ["Times-Roman", "Times-Bold", "Times-Italic", "Times-BoldItalic"][(b ? 1 : 0) + (i ? 2 : 0)]
                       })
    }

    // MARK: resource generation

    func testResourceGenerationMovesOnlyWhenAnInputOfResolutionChanges() {
        let original = PreviewFonts.producerFace, env = PreviewFonts.environmentFace
        defer { PreviewFonts.producerFace = original; PreviewFonts.overrideEnvironmentFace(env) }

        let g = PreviewFonts.resourceGeneration
        PreviewFonts.producerFace = original
        XCTAssertEqual(PreviewFonts.resourceGeneration, g, "re-setting the same producer face is not a change")
        PreviewFonts.overrideEnvironmentFace(env)
        XCTAssertEqual(PreviewFonts.resourceGeneration, g, "re-setting the same override is not a change")

        let other: PreviewFonts.Face = original == .times ? .latinModern : .times
        PreviewFonts.producerFace = other
        XCTAssertEqual(PreviewFonts.resourceGeneration, g + 1)
        PreviewFonts.producerFace = original
        XCTAssertEqual(PreviewFonts.resourceGeneration, g + 2)

        PreviewFonts.overrideEnvironmentFace(.times)
        let afterTimes = PreviewFonts.resourceGeneration
        XCTAssertEqual(afterTimes, env == .times ? g + 2 : g + 3)
        XCTAssertEqual(PreviewFonts.requested, .times, "the override wins over the producer face")
        PreviewFonts.overrideEnvironmentFace(.latinModern)
        XCTAssertEqual(PreviewFonts.resourceGeneration, afterTimes + 1)
        XCTAssertEqual(PreviewFonts.requested, .latinModern)
        PreviewFonts.overrideEnvironmentFace(nil)
        XCTAssertEqual(PreviewFonts.resourceGeneration, afterTimes + 2)
        XCTAssertEqual(PreviewFonts.requested, original, "no override: the producer face")

        PreviewFonts.invalidateResources()
        XCTAssertEqual(PreviewFonts.resourceGeneration, afterTimes + 3, "explicit invalidation always moves it")
    }

    func testLatinModernRegistrationIsAGenerationAndRecordsItsDirectory() throws {
        guard PreviewFonts.latinModernRegistered else {
            XCTAssertNil(PreviewFonts.latinModernDirectory)
            throw XCTSkip("Latin Modern not found in any search path on this machine")
        }
        let dir = try XCTUnwrap(PreviewFonts.latinModernDirectory)
        XCTAssertTrue(PreviewFonts.latinModernSearchPaths.contains(dir), dir)
        XCTAssertTrue(FileManager.default.fileExists(atPath: dir + "/lmroman10-regular.otf"), dir)
        XCTAssertGreaterThanOrEqual(PreviewFonts.resourceGeneration, 1, "registration moved the generation from 0")
        // A key made now is stamped with a generation that includes the registration.
        let key = PreviewTextCache.key(text: "x", postScriptName: "LMRoman10-Regular", size: 10)
        XCTAssertEqual(key.generation, PreviewFonts.resourceGeneration)
        XCTAssertGreaterThanOrEqual(key.generation, 1)
    }
}
