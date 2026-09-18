import XCTest
import PDFKit
import FlashTeXProtocol
@testable import FlashTeXMac

// Consumer side of runtime-v1-layout-capabilities.md (migration gate 2):
// typed-rule geometry, font-hint face mapping with substitution reporting,
// unknown-kind diagnostics on the negotiated route, and the shell's
// per-request binding of the accepted set.

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

@MainActor
final class LayoutCapabilityPDFExportTests: XCTestCase {
    private func result(items: [RuntimeV1.PageItem], caps: [String]?) -> RuntimeV1.CompileResult {
        .init(projectId: "demo", revision: 1, status: .ok,
              pages: [.init(number: 1, widthPt: 612, heightPt: 792, items: items)],
              diagnostics: [], pdfPath: nil, layoutCapabilities: caps)
    }

    func testTypedRuleAndFontHintExportToAValidPDFWithTheTextIntact() throws {
        let items: [RuntimeV1.PageItem] = [
            .text(.init(text: "Hinted heading", xPt: 72, baselineYPt: 80, fontSizePt: 14, source: nil,
                        font: .init(family: "Times", weight: .bold, style: .normal))),
            .rule(.init(xPt: 72, yPt: 84, widthPt: 24, heightPt: 0.5, source: nil)),
            .text(.init(text: "Comic body", xPt: 72, baselineYPt: 120, fontSizePt: 10, source: nil,
                        font: .init(family: "Comic Sans", weight: .normal, style: .italic))),
            .unknown(kind: "blob", source: nil),
        ]
        let data = PDFExport.render(result(items: items, caps: ["rules-v1", "font-hints-v1"]))
        XCTAssertTrue(data.starts(with: Array("%PDF".utf8)))
        let doc = try XCTUnwrap(PDFDocument(data: data))
        XCTAssertEqual(doc.pageCount, 1)
        let text = try XCTUnwrap(doc.page(at: 0)?.string)
        XCTAssertTrue(text.contains("Hinted heading") && text.contains("Comic body"), text)
        // The typed rule is geometry, not a glyph: no box-drawing character lands in the text.
        XCTAssertFalse(text.contains("─"))
        // Dark export is a color change only.
        XCTAssertEqual(PDFDocument(data: PDFExport.render(result(items: items, caps: ["rules-v1", "font-hints-v1"]), dark: true))?.pageCount, 1)
    }

    func testLegacyRouteStillApproximatesBoxDrawingRunsAsBars() throws {
        // Without rules-v1 a U+2500 run is drawn as a bar (approximation); with
        // rules-v1 accepted it is left to the font like any other text run.
        let bar = RuntimeV1.PageItem.text(.init(text: "──", xPt: 72, baselineYPt: 131.52, fontSizePt: 8.4, source: nil))
        let legacy = try XCTUnwrap(PDFDocument(data: PDFExport.render(result(items: [bar], caps: nil))))
        XCTAssertFalse(legacy.page(at: 0)?.string?.contains("─") == true, "legacy bar is filled geometry, not glyphs")
        let negotiated = try XCTUnwrap(PDFDocument(data: PDFExport.render(result(items: [bar], caps: ["rules-v1"]))))
        XCTAssertEqual(negotiated.pageCount, 1)
    }
}

@MainActor
final class ShellLayoutNegotiationTests: XCTestCase {
    static let extended = ["rules-v1", "font-hints-v1"]

    private func attachedModel(capabilities: [String]) -> ShellModel {
        let model = ShellModel()
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        model.autoCompile = false
        model.requestedLayoutCapabilities = capabilities
        return model
    }

    private func compileAndWait(_ model: ShellModel, _ text: String) async throws {
        model.updateActiveText(text)
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
    }

    func testDefaultCapabilitiesComeFromTheEnvironmentOverride() {
        XCTAssertEqual(ShellModel.defaultLayoutCapabilities(environment: [:]), ShellModel.builtInLayoutCapabilities)
        XCTAssertEqual(ShellModel.defaultLayoutCapabilities(environment: ["FLASHTEX_LAYOUT_CAPABILITIES": ""]), [])
        XCTAssertEqual(ShellModel.defaultLayoutCapabilities(environment: ["FLASHTEX_LAYOUT_CAPABILITIES": "rules-v1"]), ["rules-v1"])
        XCTAssertEqual(ShellModel.defaultLayoutCapabilities(environment: ["FLASHTEX_LAYOUT_CAPABILITIES": " rules-v1 , font-hints-v1,"]), ["rules-v1", "font-hints-v1"])
        // Fixtures never negotiate: a fresh model is on the legacy route.
        let model = ShellModel()
        XCTAssertEqual(model.negotiation, .legacy)
        XCTAssertTrue(model.capabilityNotes.isEmpty)
        XCTAssertEqual(model.displayedDiagnostics, model.result?.diagnostics ?? [])
    }

    func testNegotiatedCompileBindsAcceptedSetDrawsRuleAndHonorsFontHint() async throws {
        let model = attachedModel(capabilities: Self.extended + ["future-v9"])
        try await compileAndWait(model, "%caps\nHello negotiated\n")
        let result = try XCTUnwrap(model.result)
        XCTAssertFalse(model.isFixture)
        XCTAssertEqual(model.negotiation, .init(requested: Self.extended + ["future-v9"], accepted: Self.extended))
        XCTAssertEqual(model.acceptedLayoutCapabilities, Self.extended)
        // The unknown capability is reported, never guessed. Latin Modern is
        // bundled (apps/mac/Fonts), so the LM bold hint is honored, not substituted.
        XCTAssertEqual(model.capabilityNotes, ["capability future-v9 not accepted by the worker"])
        XCTAssertEqual(model.fontSubstitutions, [], "bundled LM must not be reported as substituted")
        guard let firstPage = result.pages.first, firstPage.items.count >= 2 else {
            return XCTFail("expected at least one page with at least two items")
        }
        guard case .text(let t) = firstPage.items[0], case .rule(let r) = firstPage.items[1] else { return XCTFail("\(firstPage.items)") }
        XCTAssertEqual(t.font, .init(family: "Latin Modern Roman", weight: .bold, style: .normal))
        XCTAssertEqual(r.yPt, 90); XCTAssertEqual(r.widthPt, 24)
        // The rule's own source range navigates like a text item's.
        model.navigate(to: r.source)
        XCTAssertEqual((model.activeText as NSString).substring(with: try XCTUnwrap(model.selection).nsRange), "Hello negotiated")
        XCTAssertEqual(model.layoutDiagnostics, [], "no unknown kinds → no layout diagnostics")
        XCTAssertTrue(model.workerLog.contains { $0.contains("future-v9 not accepted") })
        model.detachWorker()
    }

    func testRequestedButNotAcceptedIsReportedExplicitlyAndRendersLegacy() async throws {
        // Without %caps the double ignores layout_capabilities entirely (like a pre-contract worker).
        let model = attachedModel(capabilities: Self.extended)
        try await compileAndWait(model, "Plain worker\n")
        XCTAssertEqual(model.negotiation, .init(requested: Self.extended, accepted: []))
        XCTAssertFalse(model.negotiation.isNegotiated)
        XCTAssertEqual(model.capabilityNotes, ["capability rules-v1 not accepted by the worker",
                                               "capability font-hints-v1 not accepted by the worker"])
        XCTAssertEqual(model.result?.pages[0].items.count, 1, "legacy output still applied")
        XCTAssertFalse(model.workerStatus.contains("violation"), model.workerStatus)
        model.detachWorker()
    }

    func testFontSubstitutionIsReportedNotHidden() async throws {
        let model = attachedModel(capabilities: ["font-hints-v1"])
        try await compileAndWait(model, "%caps:sub\nComic\n")
        XCTAssertEqual(model.negotiation.accepted, ["font-hints-v1"])
        XCTAssertEqual(model.fontSubstitutions.map(\.description), ["font substituted: Comic Sans bold → Times-Bold"])
        XCTAssertTrue(model.capabilityNotes.contains("font substituted: Comic Sans bold → Times-Bold"))
        XCTAssertTrue(model.workerLog.contains { $0.contains("font substituted: Comic Sans bold") })
        model.detachWorker()
    }

    func testUnknownKindIsDiagnosedOnTheNegotiatedRouteAndSkippedSilentlyOnLegacy() async throws {
        let model = attachedModel(capabilities: Self.extended)
        try await compileAndWait(model, "%caps:blob\nBlobbed\n")
        XCTAssertTrue(model.negotiation.isNegotiated)
        let d = try XCTUnwrap(model.layoutDiagnostics.first)
        XCTAssertEqual(d.severity, .error)
        XCTAssertEqual(d.message, "unsupported layout primitive 'blob' at main.tex bytes 11..<18 on page 1")
        XCTAssertEqual(model.displayedDiagnostics, model.result!.diagnostics + model.layoutDiagnostics)
        model.navigate(to: d.source)
        XCTAssertEqual((model.activeText as NSString).substring(with: try XCTUnwrap(model.selection).nsRange), "Blobbed")

        // Same reply shape on the legacy route (nothing requested → nothing accepted, the
        // double emits no rule/font and the blob is skipped as before).
        model.requestedLayoutCapabilities = []
        try await compileAndWait(model, "%caps:blob\nBlobbed again\n")
        XCTAssertEqual(model.negotiation, .legacy)
        XCTAssertEqual(model.layoutDiagnostics, [])
        XCTAssertEqual(model.displayedDiagnostics, [])
        XCTAssertEqual(model.result?.pages[0].items.last, .unknown(kind: "blob", source: .init(path: "main.tex", startByte: 11, endByte: 24)))
        model.detachWorker()
    }

    func testUnrequestedRuleOrFontHintInResultIsAProtocolViolation() async throws {
        let model = attachedModel(capabilities: [])
        let before = model.result
        try await compileAndWait(model, "%caps:unrequested\nSneaky\n")
        XCTAssertEqual(model.result, before, "result with unnegotiated shapes must not be applied")
        XCTAssertTrue(model.isFixture)
        XCTAssertTrue(model.workerStatus.hasPrefix("protocol violation:"), model.workerStatus)
        XCTAssertTrue(model.workerStatus.contains("carries a font hint but font-hints-v1 was not accepted (accepted: none)"), model.workerStatus)
        XCTAssertTrue(model.inFlightRequests.isEmpty)
        XCTAssertEqual(model.negotiation, .legacy)

        // Claiming acceptance of something never requested is rejected the same way.
        try await compileAndWait(model, "%caps:claim\nLiar\n")
        XCTAssertTrue(model.isFixture)
        XCTAssertTrue(model.workerStatus.contains("accepted capability rules-v1 was not requested"), model.workerStatus)

        // Asking for only font hints and receiving a rule too is a violation as well.
        model.requestedLayoutCapabilities = ["font-hints-v1"]
        try await compileAndWait(model, "%caps:unrequested\nHalf\n")
        XCTAssertTrue(model.isFixture)
        XCTAssertTrue(model.workerStatus.contains("rules-v1 was not accepted (accepted: font-hints-v1)"), model.workerStatus)

        // And a well-formed reply afterwards is applied normally.
        model.requestedLayoutCapabilities = Self.extended
        try await compileAndWait(model, "%caps\nFine\n")
        XCTAssertFalse(model.isFixture)
        XCTAssertEqual(model.negotiation.accepted, Self.extended)
        model.detachWorker()
    }

    func testRapidLegacyExtendedSwitchingBindsEachResultToItsOwnRequest() async throws {
        let model = attachedModel(capabilities: [])
        var seen: [(revision: Int, negotiation: LayoutNegotiation)] = []
        for i in 0..<6 {
            model.requestedLayoutCapabilities = i % 2 == 0 ? [] : Self.extended
            try await compileAndWait(model, "%caps\nswitch \(i)\n")
            let result = try XCTUnwrap(model.result)
            seen.append((result.revision, model.negotiation))
            XCTAssertEqual(model.negotiation.requested, model.requestedLayoutCapabilities)
            XCTAssertEqual(model.negotiation.accepted, i % 2 == 0 ? [] : Self.extended, "iteration \(i)")
            XCTAssertEqual(result.layoutCapabilities ?? [], model.negotiation.accepted)
            XCTAssertEqual(result.pages[0].items.contains { if case .rule = $0 { true } else { false } }, i % 2 == 1)
        }
        XCTAssertEqual(seen.map(\.revision), seen.map(\.revision).sorted())

        // Mode switch while a request is in flight: a legacy (slow) request is
        // pending when the setting flips to extended. The switch does not wait
        // behind the in-flight request — it goes out at once under a new id —
        // and the legacy reply, arriving later, is superseded: valid, logged,
        // never applied (check_runtime.py classifies it `stale_ignore`).
        // Every applied result changes `resultID` (ids are unique per reply);
        // Observation does not report a value-equal set of `negotiation`, so the
        // id is the applied-result signal and both fields are read one turn later.
        let applied = AppliedResults(model: model)
        defer { applied.stop() }
        model.requestedLayoutCapabilities = []
        model.updateActiveText("%caps\n%slow\nin flight legacy\n")
        model.compile()
        let legacyRevision = model.editorRevision
        let legacyID = try XCTUnwrap(model.latestRequestID)
        XCTAssertEqual(model.inFlightRevision, legacyRevision)
        model.requestedLayoutCapabilities = Self.extended
        model.updateActiveText("%caps\nqueued extended\n")
        let extendedRevision = model.editorRevision
        model.compile() // capability switch: sent immediately, not coalesced
        let extendedID = try XCTUnwrap(model.latestRequestID)
        XCTAssertNotEqual(extendedID, legacyID)
        XCTAssertEqual(model.inFlightRequests.count, 2, "both requests are in flight")
        XCTAssertEqual(model.inFlightRevision, extendedRevision)
        try await waitUntil { model.inFlightRequests.isEmpty }
        XCTAssertEqual(model.result?.revision, extendedRevision)
        XCTAssertNil(model.inFlightRevision)
        try await waitUntil { !applied.entries.isEmpty }
        XCTAssertEqual(applied.entries.map(\.revision), [extendedRevision], "the superseded legacy reply is never applied")
        XCTAssertEqual(applied.entries.map(\.negotiation), [.init(requested: Self.extended, accepted: Self.extended)])
        XCTAssertTrue(model.workerLog.contains { $0.contains("ignored stale compile_result \(legacyID)") && $0.contains("superseded by \(extendedID)") }, "\(model.workerLog.suffix(4))")
        model.detachWorker()
    }

    func testCapabilitySwitchReRequestsTheSameRevisionAndSupersedesTheOlderReply() async throws {
        // No edit between the two requests: the revision must not change (the
        // buffers did not), yet a new id goes out and the reply is bound to it.
        let model = attachedModel(capabilities: [])
        try await compileAndWait(model, "%caps\nsame revision\n")
        let revision = model.editorRevision
        let legacyID = try XCTUnwrap(model.resultID)
        XCTAssertEqual(model.negotiation, .legacy)
        model.requestedLayoutCapabilities = Self.extended
        model.compile()
        XCTAssertEqual(model.inFlightRevision, revision, "mode switch re-requests the same revision")
        XCTAssertNotEqual(model.latestRequestID, legacyID)
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertEqual(model.result?.revision, revision)
        XCTAssertEqual(model.negotiation, .init(requested: Self.extended, accepted: Self.extended))
        XCTAssertNotEqual(model.resultID, legacyID)
        // Unchanged buffers and set: nothing is sent.
        let before = model.latestRequestID
        model.compile()
        XCTAssertEqual(model.latestRequestID, before)
        XCTAssertNil(model.inFlightRevision)

        // Auto-compile treats the switch like an edit: the request goes out by itself.
        model.autoCompile = true
        model.requestedLayoutCapabilities = ["rules-v1"]
        XCTAssertEqual(model.inFlightRevision, revision)
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertEqual(model.negotiation, .init(requested: ["rules-v1"], accepted: ["rules-v1"]))
        model.autoCompile = false

        // Async same-revision switch: the slow legacy reply for the older id
        // arrives after the extended request was sent → superseded, not applied.
        model.requestedLayoutCapabilities = []
        model.updateActiveText("%caps\n%slow\nasync switch\n")
        model.compile()
        let slowID = try XCTUnwrap(model.latestRequestID)
        model.requestedLayoutCapabilities = Self.extended
        model.compile()
        let switchID = try XCTUnwrap(model.latestRequestID)
        XCTAssertEqual(model.inFlightRequests[slowID]?.revision, model.inFlightRequests[switchID]?.revision)
        try await waitUntil { model.inFlightRequests.isEmpty }
        XCTAssertEqual(model.resultID, switchID)
        XCTAssertEqual(model.negotiation, .init(requested: Self.extended, accepted: Self.extended))
        XCTAssertTrue(model.workerLog.contains { $0.contains("ignored stale compile_result \(slowID)") })
        XCTAssertFalse(model.workerStatus.contains("violation"), model.workerStatus)
        model.detachWorker()
    }

    func testLateReplyIsBoundToTheRequestItAnswersNotTheCurrentSetting() async throws {
        // `%wrongid` makes the double answer with an id nobody sent, so the real
        // request stays in flight and we can deliver its reply by hand.
        let model = attachedModel(capabilities: Self.extended)
        model.updateActiveText("%wrongid extended in flight\n")
        model.compile()
        let id = try XCTUnwrap(model.inFlightRequests.keys.first)
        let sent = try XCTUnwrap(model.inFlightRequests[id])
        XCTAssertEqual(sent.layoutCapabilities, Self.extended)
        try await Task.sleep(nanoseconds: 300_000_000)
        XCTAssertTrue(model.isFixture)

        // The user flips to legacy meanwhile. The late extended reply is still valid
        // for ITS request: applied, bound to the extended set.
        model.requestedLayoutCapabilities = []
        let rule = RuntimeV1.PageItem.rule(.init(xPt: 72, yPt: 90, widthPt: 24, heightPt: 0.5, source: nil))
        let late = RuntimeV1.Envelope(protocolVersion: 1, id: id, type: "compile_result",
            payload: RuntimeV1.CompileResult(projectId: sent.projectId, revision: sent.revision, status: .ok,
                pages: [.init(number: 1, widthPt: 612, heightPt: 792, items: [rule])],
                diagnostics: [], pdfPath: nil, layoutCapabilities: Self.extended))
        model.handleForTesting(.result(late))
        XCTAssertFalse(model.isFixture)
        XCTAssertEqual(model.negotiation, .init(requested: Self.extended, accepted: Self.extended))
        XCTAssertEqual(model.requestedLayoutCapabilities, [], "setting untouched")

        // Reverse: legacy request in flight, setting flips to extended, and the reply
        // carries a rule → rejected; the renderer mode stays what the applied result had.
        model.updateActiveText("%wrongid legacy in flight\n")
        model.compile()
        let id2 = try XCTUnwrap(model.inFlightRequests.keys.first)
        let sent2 = try XCTUnwrap(model.inFlightRequests[id2])
        XCTAssertEqual(sent2.layoutCapabilities, [])
        model.requestedLayoutCapabilities = Self.extended
        try await Task.sleep(nanoseconds: 300_000_000)
        let bad = RuntimeV1.Envelope(protocolVersion: 1, id: id2, type: "compile_result",
            payload: RuntimeV1.CompileResult(projectId: sent2.projectId, revision: sent2.revision, status: .ok,
                pages: [.init(number: 1, widthPt: 612, heightPt: 792, items: [rule])],
                diagnostics: [], pdfPath: nil, layoutCapabilities: Self.extended))
        model.handleForTesting(.result(bad))
        XCTAssertEqual(model.result?.revision, sent.revision, "violating reply not applied")
        XCTAssertEqual(model.negotiation, .init(requested: Self.extended, accepted: Self.extended))
        XCTAssertTrue(model.workerStatus.contains("was not requested"), model.workerStatus)
        XCTAssertNil(model.inFlightRevision)

        // A reply for an id that is no longer in flight never changes the mode either.
        model.handleForTesting(.result(late))
        XCTAssertTrue(model.workerLog.last?.contains("unknown id") == true)
        XCTAssertEqual(model.negotiation, .init(requested: Self.extended, accepted: Self.extended))
        model.detachWorker()
    }

    func testFixtureCarryingUnnegotiatedShapesIsRejectedOnLoad() throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-caps-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let rule = #"{"kind":"rule","x_pt":72,"y_pt":84,"width_pt":24,"height_pt":0.5,"source":null}"#
        func write(_ name: String, caps: String) throws -> URL {
            let url = dir.appendingPathComponent(name)
            try Data(#"{"protocol_version":1,"id":"fx","type":"compile_result","payload":{"project_id":"p","revision":1,"status":"ok","pages":[{"number":1,"width_pt":612,"height_pt":792,"items":[\#(rule)]}],"diagnostics":[],"pdf_path":null\#(caps)}}"#.utf8).write(to: url)
            return url
        }
        let model = ShellModel()
        let good = model.result
        model.loadFixtures(request: nil, result: try write("bad-result.json", caps: ""))
        XCTAssertTrue(model.loadError?.contains("rule item but rules-v1 was not accepted") == true, model.loadError ?? "nil")
        XCTAssertEqual(model.result, good, "rejected fixture leaves the previous result")

        // A self-consistent fixture (declares rules-v1) loads on the negotiated route.
        model.loadFixtures(request: nil, result: try write("good-result.json", caps: #","layout_capabilities":["rules-v1"]"#))
        XCTAssertNil(model.loadError)
        XCTAssertEqual(model.negotiation, .init(requested: ["rules-v1"], accepted: ["rules-v1"]))
        XCTAssertTrue(model.isFixture)
    }

    private func waitUntil(timeout: TimeInterval = 10, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { throw XCTSkip("timeout") }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }
}

/// Records `(result revision, negotiation)` once per applied result, keyed on
/// `ShellModel.resultID` changing (unique per reply), re-arming Observation
/// tracking after each change. Values are read on the following main-actor
/// turn, after the whole result has been bound.
@MainActor
private final class AppliedResults {
    private(set) var entries: [(revision: Int?, negotiation: LayoutNegotiation)] = []
    private var stopped = false
    private let model: ShellModel

    init(model: ShellModel) { self.model = model; arm() }
    func stop() { stopped = true }

    private func arm() {
        withObservationTracking { _ = model.resultID } onChange: { [weak self] in
            MainActor.assumeIsolated {
                guard let self, !self.stopped else { return }
                self.arm()
                Task { @MainActor [weak self] in
                    guard let self, self.model.resultID != nil else { return }
                    self.entries.append((self.model.result?.revision, self.model.negotiation))
                }
            }
        }
    }
}
