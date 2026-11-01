import XCTest
import CoreText
import FlashTeXProtocol
@testable import FlashTeXMac

// Consumer side of runtime-v1-layout-capabilities.md that the app still
// carries: typed-rule geometry and font-hint face mapping with substitution
// reporting.

final class RuleGeometryTests: XCTestCase {
    private let page = RuntimeV1.Page(number: 1, widthPt: 612, heightPt: 792, items: [])
    private let rule = RuntimeV1.PageItem.RuleItem(xPt: 72, yPt: 84, widthPt: 24, heightPt: 0.5, source: nil)

    func testTypedRuleMapsTopLeftCornerNotBaseline() {
        let pdf = RuleGeometry.pdfRect(page: page, rule: rule)
        // PDF space is y-up: the rule's TOP edge is 84 pt below the page top → 708 from the bottom.
        XCTAssertEqual(pdf.maxY, 792 - 84, accuracy: 1e-9)
        XCTAssertEqual(pdf.minY, 792 - 84 - 0.5, accuracy: 1e-9)
        XCTAssertEqual(pdf.minX, 72); XCTAssertEqual(pdf.width, 24); XCTAssertEqual(pdf.height, 0.5, accuracy: 1e-9)

        let view = RuleGeometry.previewRect(rule, scale: 1)
        XCTAssertEqual(view, CGRect(x: 72, y: 84, width: 24, height: 0.5))
        let scaled = RuleGeometry.previewRect(rule, scale: 2)
        XCTAssertEqual(scaled, CGRect(x: 144, y: 168, width: 48, height: 1))
        // Hairlines never vanish on screen (0.5 pt floor), but export keeps the exact height.
        let hair = RuntimeV1.PageItem.RuleItem(xPt: 0, yPt: 0, widthPt: 10, heightPt: 0.1, source: nil)
        XCTAssertEqual(RuleGeometry.previewRect(hair, scale: 1).height, 0.5)
        XCTAssertEqual(RuleGeometry.pdfRect(page: page, rule: hair).height, 0.1, accuracy: 1e-12)
        // Per-page height: a shorter page flips differently.
        let small = RuntimeV1.Page(number: 2, widthPt: 200, heightPt: 100, items: [])
        XCTAssertEqual(RuleGeometry.pdfRect(page: small, rule: rule).maxY, 16, accuracy: 1e-9)
    }

    func testLegacyBarStillHugsBaselineOnlyThroughRuleConvention() throws {
        // Legacy route: a U+2500 run at baseline 131.52 is a bar whose BOTTOM is the baseline.
        let text = RuntimeV1.PageItem.TextItem(text: "──", xPt: 72, baselineYPt: 131.52, fontSizePt: 8.4, source: nil)
        let r = try XCTUnwrap(RuleGeometry.legacyPDFRect(page: page, text: text))
        XCTAssertEqual(r.minY, 792 - 131.52, accuracy: 1e-9)
        XCTAssertEqual(r.width, 8.4, accuracy: 1e-9)
        XCTAssertNil(RuleGeometry.legacyPDFRect(page: page, text: .init(text: "x", xPt: 0, baselineYPt: 0, fontSizePt: 10, source: nil)))
    }
}

final class FontHintResolutionTests: XCTestCase {
    typealias Hint = RuntimeV1.PageItem.FontHint

    func testTimesFamiliesMapToCore14FacesWithWeightAndStyleAndNoSubstitution() {
        for family in ["Times", "Times New Roman", "Times-Roman", "times roman"] {
            XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: family), size: 10), .init(postScriptName: "Times-Roman", substitution: nil), family)
            XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: family, weight: .bold), size: 10).postScriptName, "Times-Bold")
            XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: family, style: .italic), size: 10).postScriptName, "Times-Italic")
            XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: family, weight: .bold, style: .italic), size: 10).postScriptName, "Times-BoldItalic")
        }
    }

    /// Latin Modern is vendored in apps/mac/Fonts (registered from the repo copy
    /// in tests, from the bundle's Fonts/ in the app), so an LM hint resolves to
    /// the real LM master at the requested weight/style with NO substitution.
    func testBundledLatinModernBoldHintResolvesToLMRoman12BoldWithoutSubstitution() {
        XCTAssertTrue(PreviewFonts.latinModernRegistered, "apps/mac/Fonts ships lmroman*.otf; registration must not depend on TeX")
        let bold = PreviewFonts.resolve(hint: Hint(family: "Latin Modern Roman", weight: .bold), size: 12)
        XCTAssertEqual(bold, .init(postScriptName: "LMRoman12-Bold", substitution: nil))
        // CoreText really has that face (not a fallback that merely keeps the name).
        let font = CTFontCreateWithName(bold.postScriptName as CFString, 12, nil)
        XCTAssertEqual(CTFontCopyPostScriptName(font) as String, "LMRoman12-Bold")
        XCTAssertEqual(CTFontCopyFamilyName(font) as String, "Latin Modern Roman")
        // Latin Modern has one bold-italic master (10); the producer scales it at 12 pt.
        XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: "Latin Modern Roman", weight: .bold, style: .italic), size: 12),
                       .init(postScriptName: "LMRoman10-BoldItalic", substitution: nil))
        XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: "Latin Modern Roman"), size: 10).postScriptName, "LMRoman10-Regular")
        XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: "Latin Modern Roman", weight: .bold), size: 24).postScriptName, "LMRoman12-Bold", "17 master has only Regular")
        XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: "Latin Modern Roman"), size: 24).postScriptName, "LMRoman17-Regular")
        for family in ["lmroman10-regular", "Computer Modern", "LM Roman", "latin modern roman"] {
            XCTAssertNil(PreviewFonts.resolve(hint: Hint(family: family, weight: .bold), size: 12).substitution, family)
        }
        // A result made only of LM hints therefore carries no substitution notes.
        let result = RuntimeV1.CompileResult(projectId: "p", revision: 1, status: .ok, pages: [
            .init(number: 1, widthPt: 100, heightPt: 100, items: [
                .text(.init(text: "x", xPt: 0, baselineYPt: 10, fontSizePt: 12, source: nil, font: Hint(family: "Latin Modern Roman", weight: .bold)))])],
            diagnostics: [], pdfPath: nil, layoutCapabilities: ["font-hints-v1"])
        XCTAssertEqual(PreviewFonts.substitutions(in: result), [])
    }

    /// The Rust compiler names hint families after the Core-14 face it measured
    /// with (`Times-Bold`, `Times-Italic`, `Helvetica`, `Courier`); those are
    /// honored with the hint's own weight/style and are not substitutions.
    func testCompilerCore14FaceNamesResolveWithoutSubstitution() {
        XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: "Times-Bold", weight: .bold), size: 17), .init(postScriptName: "Times-Bold", substitution: nil))
        XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: "Times-Italic", style: .italic), size: 12).postScriptName, "Times-Italic")
        XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: "Times-BoldItalic", weight: .bold, style: .italic), size: 12).postScriptName, "Times-BoldItalic")
        XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: "Helvetica"), size: 12), .init(postScriptName: "Helvetica", substitution: nil))
        XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: "Helvetica", weight: .bold, style: .italic), size: 12).postScriptName, "Helvetica-BoldOblique")
        XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: "Courier", weight: .bold), size: 12), .init(postScriptName: "Courier-Bold", substitution: nil))
        XCTAssertEqual(PreviewFonts.resolve(hint: Hint(family: "Courier-Oblique", style: .italic), size: 12).postScriptName, "Courier-Oblique")
        // Every Core-14 face named here exists in CoreText on macOS.
        for name in ["Times-Roman", "Times-Bold", "Times-Italic", "Times-BoldItalic", "Helvetica", "Helvetica-Bold",
                     "Helvetica-Oblique", "Helvetica-BoldOblique", "Courier", "Courier-Bold", "Courier-Oblique", "Courier-BoldOblique"] {
            let font = CTFontCreateWithName(name as CFString, 12, nil)
            XCTAssertEqual(CTFontCopyPostScriptName(font) as String, name)
        }
    }

    func testUnknownFamilyIsAnExplicitSubstitutionAndNilHintIsLegacySelection() {
        let comic = PreviewFonts.resolve(hint: Hint(family: "Comic Sans", weight: .bold), size: 10)
        XCTAssertEqual(comic.postScriptName, "Times-Bold")
        XCTAssertEqual(comic.substitution, .init(family: "Comic Sans", weight: .bold, style: .normal, usedFace: "Times-Bold"))
        XCTAssertEqual(comic.substitution?.description, "font substituted: Comic Sans bold → Times-Bold")
        // No hint → whatever the legacy producer heuristic picks, never reported as a substitution.
        let legacy = PreviewFonts.resolve(hint: nil, size: 10)
        XCTAssertNil(legacy.substitution)
        XCTAssertEqual(legacy.postScriptName, PreviewFonts.postScriptName(size: 10))
    }

    func testSubstitutionsInResultAreDeduplicatedInFirstSeenOrder() {
        func text(_ family: String?, weight: Hint.Weight = .normal) -> RuntimeV1.PageItem {
            .text(.init(text: "x", xPt: 0, baselineYPt: 10, fontSizePt: 10, source: nil, font: family.map { Hint(family: $0, weight: weight) }))
        }
        let result = RuntimeV1.CompileResult(projectId: "p", revision: 1, status: .ok, pages: [
            .init(number: 1, widthPt: 100, heightPt: 100, items: [text("Comic Sans"), text("Times"), text(nil), text("Comic Sans"), text("Papyrus", weight: .bold)]),
            .init(number: 2, widthPt: 100, heightPt: 100, items: [text("Comic Sans", weight: .bold)]),
        ], diagnostics: [], pdfPath: nil, layoutCapabilities: ["font-hints-v1"])
        let subs = PreviewFonts.substitutions(in: result)
        XCTAssertEqual(subs.map(\.requested), ["Comic Sans", "Papyrus bold", "Comic Sans bold"])
        XCTAssertEqual(subs.map(\.usedFace), ["Times-Roman", "Times-Bold", "Times-Bold"])
    }
}
