import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
import HostedWindows
@testable import FlashTeXMac

/// GH-363 review: v2 frame diagnostics must reach `problemsList` (and therefore
/// Problems, editor marks, and `caretFix`) through `asRuntimeV1`. The debug
/// raw list under the v2 pages is not that path.
@MainActor
final class ProblemsPanelV2Tests: XCTestCase {
    /// `Text \alpah here.` — `\alpah` is bytes 5..<11 (ASCII, so UTF-16 matches).
    private let text = "Text \\alpah here."

    private func v2Diagnostic(suggestion: String? = "\\alpha") -> RenderingV2.Diagnostic {
        .init(code: "unknown_command",
              message: "\\alpah is not supported by this compiler version",
              severity: .error,
              sources: [.init(path: "main.tex", startByte: 5, endByte: 11)],
              suggestion: suggestion)
    }

    private func frame(diagnostics: [RenderingV2.Diagnostic], revision: Int = 1) throws -> V2Frame {
        let list = RenderingV2.DisplayList(
            projectId: "p", revision: revision,
            requiredFeatures: ["glyph_run", "rgba-srgb", "cluster-actualtext"],
            documents: [], fonts: [], pages: [], diagnostics: diagnostics)
        return try V2Frame.prepare(.init(id: "r1", payload: list))
    }

    private func model(v1: [RuntimeV1.Diagnostic], v2: [RenderingV2.Diagnostic]) throws -> ShellModel {
        let m = ShellModel()
        m.autoCompile = false
        m.replaceProject(entryText: text)
        let revision = m.editorRevision
        m.result = .init(projectId: "p", revision: revision, status: .recovered, pages: [],
                         diagnostics: v1, pdfPath: nil)
        m.resultID = "r1"
        m.setCompiledDocuments(["main.tex": text])
        m.displayListV2 = .loaded(try frame(diagnostics: v2, revision: revision),
                                  .worker(requestID: "r1", projectId: "p", revision: revision, line: Data()))
        return m
    }

    /// Review gap: empty runtime-v1 list + a suggestion-bearing v2 diagnostic
    /// used to leave Problems empty because the frame only fed the debug raw list.
    func testV2SuggestionWithEmptyV1GivesOneProblemsRowAndCaretFix() throws {
        let m = try model(v1: [], v2: [v2Diagnostic()])
        XCTAssertEqual(m.problemsList.count, 1, "v2 diagnostics map through asRuntimeV1 into problemsList")
        XCTAssertEqual(m.displayedDiagnostics.count, 1)
        let row = try XCTUnwrap(m.problemsList.first, "empty problemsList is the review gap")
        XCTAssertEqual(row.code, "unknown_command")
        XCTAssertEqual(row.suggestion, "\\alpha")
        XCTAssertEqual(row.source, .init(path: "main.tex", startByte: 5, endByte: 11))
        m.caretUTF16 = 5
        XCTAssertEqual(m.caretFix?.replacement, "\\alpha", "Tab's caretFix reads the same list")
        XCTAssertEqual(m.caretFix?.diagnosticIndex, 0)
        XCTAssertEqual(m.caretFix?.startByte, 5)
        XCTAssertEqual(m.caretFix?.endByte, 11)
        let marks = m.editorMarks
        XCTAssertEqual(marks.count, 1, "EditorDiagnostics underlines the v2 span")
        XCTAssertEqual((text as NSString).substring(with: marks[0].nsRange), "\\alpah")
    }

    /// Prefer runtime-v1 when it already lists diagnostics; never concatenate
    /// the v2 sibling of the same compile (duplicate rows).
    func testV1DiagnosticsWinAndAreNotDuplicatedByTheV2Frame() throws {
        let v1 = RuntimeV1.Diagnostic(
            severity: .error, message: "from compile_result",
            source: .init(path: "main.tex", startByte: 5, endByte: 11), recovery: nil,
            code: "unknown_command")
        let m = try model(v1: [v1], v2: [v2Diagnostic()])
        XCTAssertEqual(m.problemsList.count, 1, "one source: never duplicate v1+v2")
        XCTAssertEqual(m.problemsList[0].message, "from compile_result")
        XCTAssertNil(m.problemsList[0].suggestion, "v1 wins even when the v2 sibling carries a suggestion")
        m.caretUTF16 = 5
        XCTAssertNil(m.caretFix, "the chosen v1 row has no suggestion")
    }

    /// Hosted Problems panel: the v2-only row is a real list row, not "No problems".
    func testHostedProblemsPanelShowsTheV2Row() async throws {
        let m = try model(v1: [], v2: [v2Diagnostic()])
        XCTAssertEqual(m.problemsList.count, 1)
        guard m.problemsList.count == 1 else { return }
        let host = NSHostingView(rootView: ProblemsPanel().environment(m))
        host.frame = NSRect(x: 0, y: 0, width: 640, height: 280)
        let window = HostedWindowSupport.window(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = host
        window.orderFrontRegardless()
        defer { window.orderOut(nil) }
        host.layoutSubtreeIfNeeded()
        var table: NSTableView?
        let deadline = Date().addingTimeInterval(5)
        while table == nil, Date() < deadline {
            try await Task.sleep(nanoseconds: 50_000_000)
            table = descendants(host).compactMap { $0 as? NSTableView }.first
        }
        guard let list = table else {
            throw XCTSkip("SwiftUI did not back Problems with an NSTableView in this host; problemsList is asserted above")
        }
        XCTAssertGreaterThanOrEqual(list.numberOfRows, 1, "Problems shows the v2 diagnostic")
    }

    /// Requested `display-list-v2-diagnostics` that the producer did not echo
    /// is logged, never an error. Overflow that drops `display-list-v2` also
    /// drops the dependent diagnostics cap (whole family).
    func testDeclineLoggingForDiagnosticsCapabilityAndWholeFamilyOverflow() {
        let m = ShellModel()
        m.autoCompile = false
        m.setLiveV2(true)
        XCTAssertTrue(m.requestedLayoutCapabilities.contains(RenderingV2.diagnosticsCapability))
        XCTAssertTrue(m.requestedLayoutCapabilities.contains(V2Live.capability))

        let v2Only = RuntimeV1.CompileResult(
            projectId: "p", revision: 1, status: .ok, pages: [], diagnostics: [], pdfPath: nil,
            layoutCapabilities: [V2Live.capability])
        m.bindLayout(of: v2Only, requested: m.requestedLayoutCapabilities)
        XCTAssertFalse(m.negotiation.missing.contains(V2Live.capability), "display-list-v2 was accepted")
        XCTAssertTrue(m.negotiation.missing.contains(RenderingV2.diagnosticsCapability))
        XCTAssertTrue(m.workerLog.contains { $0.contains("capability \(RenderingV2.diagnosticsCapability) not accepted") },
                      "diagnostics cap decline is logged: \(m.workerLog)")

        m.workerLog = []
        let overflow = RuntimeV1.CompileResult(
            projectId: "p", revision: 1, status: .ok, pages: [], diagnostics: [], pdfPath: nil,
            layoutCapabilities: [])
        m.bindLayout(of: overflow, requested: m.requestedLayoutCapabilities)
        XCTAssertTrue(m.negotiation.missing.contains(V2Live.capability))
        XCTAssertTrue(m.negotiation.missing.contains(RenderingV2.diagnosticsCapability),
                      "dependent diagnostics cap drops with the family")
        XCTAssertTrue(m.workerLog.contains { $0.contains("capability \(V2Live.capability) not accepted") },
                      "overflow declines display-list-v2: \(m.workerLog)")
        XCTAssertTrue(m.workerLog.contains { $0.contains("capability \(RenderingV2.diagnosticsCapability) not accepted") },
                      "dependent diagnostics cap is logged too: \(m.workerLog)")
    }

    private func descendants(_ root: NSView) -> [NSView] { root.subviews.flatMap { [$0] + descendants($0) } }
}
