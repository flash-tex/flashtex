import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// End-to-end check against the actual Rust worker (FT-002). Skipped unless
/// FLASHTEX_COMPILER points at a built `flashtex-compiler` binary, so `swift test`
/// stays hermetic on machines without the crate built.
@MainActor
final class RealCompilerTests: XCTestCase {
    static var binary: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"].map { URL(fileURLWithPath: $0) }
    }

    func testShellCompilesThroughRealWorkerAndNavigatesEverySpan() async throws {
        guard let binary = Self.binary, FileManager.default.isExecutableFile(atPath: binary.path) else {
            throw XCTSkip("set FLASHTEX_COMPILER to the built flashtex-compiler binary")
        }
        let model = ShellModel()
        model.attachWorker(at: binary)
        XCTAssertTrue(model.workerAttached)

        let source = "\\section{Intro}\nHello naïve FlashTeX — café. $x$ \\foo{bar}\n\nSecond paragraph.\n"
        model.updateActiveText(source)
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        let result = try XCTUnwrap(model.result)
        XCTAssertFalse(model.isFixture)
        XCTAssertEqual(result.revision, model.editorRevision)
        XCTAssertEqual(result.projectId, "demo")
        XCTAssertFalse(result.pages.isEmpty)
        XCTAssertFalse(result.diagnostics.isEmpty, "math and \\foo should be diagnosed, not silently dropped")

        // Every item's UTF-8 span must convert to a UTF-16 selection that slices
        // back to its text — or, for text the compiler generated (the section
        // number "1"), to the command that generated it (`\section`).
        var checked = 0
        for page in result.pages {
            for case .text(let item) in page.items {
                let src = try XCTUnwrap(item.source)
                model.navigate(to: src)
                let sel = try XCTUnwrap(model.selection, "navigation failed for \(item.text): \(model.navigationNote ?? "")")
                let selected = (model.activeText as NSString).substring(with: sel.nsRange)
                if selected != item.text {
                    XCTAssertTrue(selected.hasPrefix("\\"), "generated text '\(item.text)' must map to its generating command, got '\(selected)'")
                }
                checked += 1
            }
        }
        XCTAssertGreaterThan(checked, 5)
        // Diagnostics with a source range must navigate too.
        for d in result.diagnostics where d.source != nil {
            model.navigate(to: d.source)
            XCTAssertNotNil(model.selection)
            XCTAssertFalse(model.navigationNote?.contains("not a valid range") == true, model.navigationNote ?? "")
        }

        // Edit after the compiled text: spans before the edit still navigate (rebased and
        // verified against item text); an edit inside a span refuses navigation.
        let firstItems: [RuntimeV1.PageItem.TextItem] = result.pages.flatMap { $0.items }.compactMap { if case .text(let t) = $0 { t } else { nil } }
        let hello = try XCTUnwrap(firstItems.first { $0.text == "Hello" })
        let second = try XCTUnwrap(firstItems.first { $0.text == "Second" })
        model.autoCompile = false
        model.updateActiveText(source.replacingOccurrences(of: "Second paragraph.", with: "Edited paragraph."))
        model.navigate(to: hello.source!, expectedText: hello.text)
        XCTAssertEqual((model.activeText as NSString).substring(with: model.selection!.nsRange), "Hello")
        let before = model.selection
        model.navigate(to: second.source!, expectedText: second.text)
        XCTAssertEqual(model.selection, before, "span inside the edited region must not navigate")
        XCTAssertTrue(model.navigationNote?.contains("recompile") == true)
        model.updateActiveText("PREFIX " + source)
        model.navigate(to: second.source!, expectedText: second.text)
        XCTAssertEqual((model.activeText as NSString).substring(with: model.selection!.nsRange), "Second", "rebased across a prefix insertion")

        // Latency sample for the report (auto-compile burst, coalesced).
        model.autoCompile = true
        for i in 0..<10 {
            model.updateActiveText(source + "\nline \(i)\n")
            try await Task.sleep(nanoseconds: 60_000_000)
        }
        try await waitUntil { model.inFlightRevision == nil && model.result?.revision == model.editorRevision }
        let lat = model.latenciesMs.sorted()
        print("REAL-COMPILER LATENCY ms: n=\(lat.count) min=\(lat.first ?? 0) median=\(model.medianLatencyMs ?? 0) max=\(lat.last ?? 0)")
        XCTAssertFalse(lat.isEmpty)
        model.autoCompile = false

        // A second revision replaces the first; a repeat of the older one would be ignored.
        model.updateActiveText("Only one word\n")
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        XCTAssertEqual(model.result?.revision, model.editorRevision)
        XCTAssertEqual(model.result?.status, .ok)
        model.detachWorker()
    }

    /// Gate 3 evidence: the shell requests `rules-v1`/`font-hints-v1` by default.
    /// Whether the real compiler accepts them is observed, never assumed — if it
    /// does not, the shell must say so explicitly and still render legacy output;
    /// if it does, typed rules and font hints are checked end to end.
    func testLayoutCapabilityNegotiationAgainstRealWorkerIsExplicit() async throws {
        guard let binary = Self.binary, FileManager.default.isExecutableFile(atPath: binary.path) else {
            throw XCTSkip("set FLASHTEX_COMPILER to the built flashtex-compiler binary")
        }
        let model = ShellModel()
        model.attachWorker(at: binary)
        model.autoCompile = false
        let requested = model.requestedLayoutCapabilities
        XCTAssertEqual(requested, ShellModel.defaultLayoutCapabilities())
        XCTAssertFalse(requested.isEmpty, "gate 3: capabilities are requested by default (override with FLASHTEX_LAYOUT_CAPABILITIES)")

        let source = "\\section{Caps}\nInline $\\frac{1}{2}$ fraction and \\textbf{bold}.\n"
        model.updateActiveText(source)
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        let result = try XCTUnwrap(model.result)
        XCTAssertFalse(model.isFixture)
        XCTAssertFalse(model.workerStatus.contains("protocol violation"), model.workerStatus)
        XCTAssertEqual(model.negotiation.requested, requested, "bound to the request that produced it")
        let accepted = model.negotiation.accepted
        print("REAL-COMPILER CAPABILITIES: requested=\(requested) accepted=\(accepted.isEmpty ? ["<none>"] : accepted) notes=\(model.capabilityNotes)")

        let items = result.pages.flatMap(\.items)
        let rules: [RuntimeV1.PageItem.RuleItem] = items.compactMap { if case .rule(let r) = $0 { r } else { nil } }
        let texts: [RuntimeV1.PageItem.TextItem] = items.compactMap { if case .text(let t) = $0 { t } else { nil } }
        for cap in requested {
            if accepted.contains(cap) {
                XCTAssertFalse(model.capabilityNotes.contains("capability \(cap) not accepted by the worker"))
            } else {
                XCTAssertTrue(model.capabilityNotes.contains("capability \(cap) not accepted by the worker"),
                              "non-acceptance of \(cap) must be reported explicitly: \(model.capabilityNotes)")
            }
        }
        if accepted.contains(RuntimeV1.LayoutCapabilities.rulesV1) {
            // Typed fraction bar: top-left geometry from the contract, mapped to
            // the \frac source; no U+2500 stand-in and no export warning.
            let bar = try XCTUnwrap(rules.first, "rules-v1 accepted but no rule for \\frac: \(items)")
            XCTAssertGreaterThan(bar.widthPt, 0); XCTAssertGreaterThan(bar.heightPt, 0)
            let src = try XCTUnwrap(bar.source)
            model.navigate(to: src)
            XCTAssertEqual((model.activeText as NSString).substring(with: try XCTUnwrap(model.selection).nsRange), "\\frac")
            XCTAssertFalse(texts.contains { $0.text.contains("\u{2500}") }, "typed rule replaces the box-drawing approximation")
            XCTAssertFalse(result.diagnostics.contains { $0.message.contains("U+2500") }, "\(result.diagnostics.map(\.message))")
            let numerator = try XCTUnwrap(texts.first { $0.text == "1" && $0.fontSizePt < 12 })
            XCTAssertLessThan(numerator.baselineYPt, bar.yPt, "numerator baseline sits above the bar's top edge")
            let denominator = try XCTUnwrap(texts.first { $0.text == "2" && $0.fontSizePt < 12 })
            XCTAssertGreaterThan(denominator.baselineYPt, bar.yPt + bar.heightPt)
            XCTAssertEqual(RuleGeometry.pdfRect(page: result.pages[0], rule: bar).minY, result.pages[0].heightPt - bar.yPt - bar.heightPt, accuracy: 1e-9)
        } else {
            XCTAssertTrue(rules.isEmpty, "a worker that did not accept rules-v1 may not emit rules (the shell would have rejected it)")
        }
        if accepted.contains(RuntimeV1.LayoutCapabilities.fontHintsV1) {
            // Every text item carries a hint; the compiler names Core-14 faces
            // (Times-Roman/Times-Bold/…) which resolve without substitution.
            XCTAssertTrue(texts.allSatisfy { $0.font != nil }, "\(texts.filter { $0.font == nil }.map(\.text))")
            let heading = try XCTUnwrap(texts.first { $0.text == "Caps" })
            XCTAssertEqual(heading.font?.weight, .bold, "\\section heading is bold: \(String(describing: heading.font))")
            XCTAssertEqual(PreviewFonts.resolve(hint: heading.font, size: heading.fontSizePt).postScriptName, "Times-Bold")
            XCTAssertEqual(model.fontSubstitutions, [], "Core-14 families are honored, not substituted")
            print("REAL-COMPILER FONT HINTS: \(Set(texts.compactMap { $0.font.map { "\($0.family)/\($0.weight.rawValue)/\($0.style.rawValue)" } }).sorted())")
        } else {
            XCTAssertTrue(texts.allSatisfy { $0.font == nil })
        }
        XCTAssertEqual(model.layoutDiagnostics, [], "no unknown primitive kinds from the real compiler")
        if accepted.isEmpty { XCTAssertFalse(model.negotiation.isNegotiated) }
        // Either way the result exports through the CoreGraphics writer.
        let pdf = PDFExport.render(result)
        XCTAssertTrue(pdf.starts(with: Array("%PDF".utf8)))

        // Opting out sends no field and yields the legacy negotiation with no
        // notes; the fraction bar then comes back as the U+2500 approximation,
        // and the compiler says so.
        model.requestedLayoutCapabilities = []
        model.compile() // same revision, new id: capability switch only
        XCTAssertEqual(model.inFlightRevision, result.revision)
        try await waitUntil { model.inFlightRevision == nil }
        let legacy = try XCTUnwrap(model.result)
        XCTAssertEqual(legacy.revision, result.revision)
        XCTAssertEqual(model.negotiation, .legacy)
        XCTAssertTrue(model.capabilityNotes.isEmpty, "\(model.capabilityNotes)")
        XCTAssertTrue(legacy.pages.flatMap(\.items).contains { if case .text(let t) = $0 { t.text.contains("\u{2500}") } else { false } })
        XCTAssertFalse(legacy.pages.flatMap(\.items).contains { if case .rule = $0 { true } else { false } })
        model.detachWorker()
    }

    /// The real compiler's exchange with the shell — including the same-revision
    /// capability switch above — passes main's `scripts/check_runtime.py`.
    func testRealWorkerTranscriptPassesRuntimeValidator() async throws {
        guard let binary = Self.binary, FileManager.default.isExecutableFile(atPath: binary.path) else {
            throw XCTSkip("set FLASHTEX_COMPILER to the built flashtex-compiler binary")
        }
        let checker = RuntimeTranscriptTests.checker
        guard FileManager.default.isReadableFile(atPath: checker.path) else { throw XCTSkip("no scripts/check_runtime.py") }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-real-transcript-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: dir) }
        let transcript = dir.appendingPathComponent("real.jsonl")
        let model = ShellModel()
        model.transcript = try RuntimeTranscript(url: transcript)
        model.attachWorker(at: binary)
        model.autoCompile = false
        model.updateActiveText("Real $\\frac{a}{b}$ \\section{S} café.\n")
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        model.requestedLayoutCapabilities = ["rules-v1"]
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        model.requestedLayoutCapabilities = []
        model.updateActiveText("Legacy $\\frac{a}{b}$\n")
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        model.requestedLayoutCapabilities = RuntimeV1.LayoutCapabilities.supported + ["future-v9"]
        model.compile()
        try await waitUntil { model.inFlightRevision == nil }
        model.detachWorker()

        let p = Process()
        p.executableURL = WorkerClientTests.python
        p.arguments = [checker.path, transcript.path]
        let out = Pipe()
        p.standardOutput = out
        try p.run()
        let data = out.fileHandleForReading.readDataToEndOfFile()
        p.waitUntilExit()
        let report = try JSONDecoder().decode(RuntimeTranscriptTests.CheckerReport.self, from: data)
        XCTAssertEqual(p.terminationStatus, 0, "\(report.errors)")
        XCTAssertTrue(report.valid, "\(report.errors)")
        XCTAssertEqual(report.responses.map(\.preview), ["current", "current", "current", "current"])
        XCTAssertEqual(report.responses.map { $0.accepted_layout_capabilities ?? [] },
                       [["font-hints-v1", "rules-v1"], ["rules-v1"], [], ["font-hints-v1", "rules-v1"]])
        XCTAssertEqual(report.responses.last?.missing_layout_capabilities, ["future-v9"])
        print("REAL-COMPILER TRANSCRIPT: \(report.responses.count) responses validated by check_runtime.py at \(transcript.path)")
    }

    /// Waits on a helper process (the real compiler); generous because a loaded runner
    /// delays process start-up and replies by seconds, not because anything is timed.
    private func waitUntil(timeout: TimeInterval = 60, _ cond: () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timed out after \(Int(timeout)) s waiting for the real compiler"); return }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }
}
