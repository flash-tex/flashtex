import XCTest
import AppKit
import SwiftUI
import FlashTeXProtocol
import FlashTeXAccessibility
import HostedWindows
@testable import FlashTeXMac

/// Diagnostics panel follow-up (DiagnosticsPanel.swift): grouped rows speak
/// their count and current occurrence, ⌘⌥] / ⌘⌥[ step within a group,
/// Return jumps to the selected group's occurrence, Esc hands the keyboard
/// back to the editor, and "Copy as text" writes `path:line: message` lines.
///
/// The keyboard test hosts the real editor and the panel in one window that
/// is never made key (nothing steals focus). Key events are synthesized and
/// sent to that window; when SwiftUI's key handling does not run for a
/// non-key window the same shell action the key is wired to is exercised
/// directly, and the test says which path it measured.
@MainActor
final class DiagnosticsPanelTests: XCTestCase {
    static let text = "line one\n\\in here\nline three \\in again\n\\mathbb{R}\nlast \\in\n"

    /// 3× "\in …" (bytes 9, 29, 55: lines 2, 3, 5) + 1 \mathbb warning (line 4) + 1 unsourced.
    func diagnostics() -> [RuntimeV1.Diagnostic] {
        let message = "\\in is not supported in math mode"
        func at(_ s: Int, _ e: Int, _ m: String = message, _ sev: RuntimeV1.Severity = .error) -> RuntimeV1.Diagnostic {
            .init(severity: sev, message: m, source: .init(path: "main.tex", startByte: s, endByte: e), recovery: "rendered as text")
        }
        return [at(29, 32), at(9, 12), at(39, 49, "\\mathbb is not supported", .warning), at(55, 58),
                .init(severity: .warning, message: "Overfull line", source: nil, recovery: nil)]
    }

    /// Source order: main.tex gap, main.tex warning, main.tex typo, chapter.tex
    /// typo, chapter.tex gap, chapter.tex warning. Codes so `isGap` does not
    /// depend on message phrasing.
    func mixedBucketDiagnostics() -> [RuntimeV1.Diagnostic] {
        [
            .init(severity: .error, message: "packages tikz are recognised but not implemented",
                  source: .init(path: "main.tex", startByte: 0, endByte: 4), recovery: nil, code: "unsupported_feature"),
            .init(severity: .warning, message: "Overfull line",
                  source: .init(path: "main.tex", startByte: 10, endByte: 14), recovery: nil),
            .init(severity: .error, message: "unknown command \\alpah",
                  source: .init(path: "main.tex", startByte: 20, endByte: 26), recovery: nil, code: "unknown_command"),
            .init(severity: .error, message: "unknown command \\textbff",
                  source: .init(path: "chapter.tex", startByte: 5, endByte: 13), recovery: nil, code: "unknown_command"),
            .init(severity: .error, message: "environment 'tabbing' is not implemented",
                  source: .init(path: "chapter.tex", startByte: 0, endByte: 3), recovery: nil, code: "unsupported_feature"),
            .init(severity: .warning, message: "Underfull line",
                  source: .init(path: "chapter.tex", startByte: 40, endByte: 44), recovery: nil),
        ]
    }

    func model() -> ShellModel {
        let m = ShellModel()
        m.replaceProject(entryText: Self.text)
        m.result = RuntimeV1.CompileResult(projectId: "p", revision: m.editorRevision, status: .recovered, pages: [],
                                           diagnostics: diagnostics(), pdfPath: nil)
        m.resultID = "r1"
        m.setCompiledDocuments(["main.tex": Self.text])
        return m
    }

    // MARK: gap category

    /// The compiler's not-implemented / not-supported messages are gaps, not
    /// errors or warnings, whatever severity they carry (daniel-fable-ui-qa #1).
    func testNotImplementedDiagnosticsCountAsGapsNotErrors() {
        let mk = { (s: RuntimeV1.Severity, m: String) in RuntimeV1.Diagnostic(severity: s, message: m, source: nil, recovery: nil) }
        let diags = [
            mk(.warning, "packages fontenc are recognised but not implemented"),
            mk(.error, "\\setlength is not supported by this compiler version; unrestricted TeX math mode is not implemented"),
            mk(.error, "\\mathbb is not supported in math mode"),
            mk(.error, "environment 'tikzpicture' is not implemented; its body is typeset as plain text"),
            mk(.error, "\\includegraphics is unsupported; image loading is not implemented"),
            mk(.error, "\\setlength is not supported in the document preamble"),
            mk(.error, "missing } inserted"),
            mk(.warning, "Overfull line"),
        ]
        let c = EditorDiagnostics.counts(diags)
        XCTAssertEqual(c.errors, 1)
        XCTAssertEqual(c.warnings, 1)
        XCTAssertEqual(c.gaps, 6)
        XCTAssertFalse(EditorDiagnostics.isGap("missing } inserted"))
        XCTAssertEqual(EditorDiagnostics.counts([]).gaps, 0)
        // `code` wins over message wording; unknown codes keep the phrase fallback.
        let byCode: [RuntimeV1.Diagnostic] = [
            .init(severity: .error, message: "custom wording with no gap phrase", source: nil, recovery: nil,
                  code: "unsupported_feature"),
            .init(severity: .error, message: "\\foo is not implemented", source: nil, recovery: nil,
                  code: "unknown_command"),
            .init(severity: .error, message: "still a gap phrase: not implemented", source: nil, recovery: nil,
                  code: "made_up_code"),
        ]
        XCTAssertTrue(EditorDiagnostics.isGap(byCode[0]))
        XCTAssertFalse(EditorDiagnostics.isGap(byCode[1]), "unknown_command is an author error even if the message mentions unimplemented")
        XCTAssertTrue(EditorDiagnostics.isGap(byCode[2]), "unknown codes fall back to message phrases")
        let coded = EditorDiagnostics.counts(byCode)
        XCTAssertEqual(coded.gaps, 2)
        XCTAssertEqual(coded.errors, 1)
    }

    func testGroupsByCodeWhenPresentAndFallsBackToMessage() {
        let sameCode: [RuntimeV1.Diagnostic] = [
            .init(severity: .error, message: "\\in is not supported in math mode",
                  source: .init(path: "main.tex", startByte: 9, endByte: 12), recovery: nil, code: "unsupported_feature"),
            .init(severity: .error, message: "\\in is not supported in math mode",
                  source: .init(path: "main.tex", startByte: 29, endByte: 32), recovery: nil, code: "unsupported_feature"),
            .init(severity: .error, message: "\\mathbb is not supported",
                  source: .init(path: "main.tex", startByte: 39, endByte: 49), recovery: nil, code: "unsupported_feature"),
        ]
        let groups = EditorDiagnostics.groups(of: sameCode, documentOrder: ["main.tex"])
        XCTAssertEqual(groups.map(\.count), [2, 1], "same code still splits on different messages")
        XCTAssertEqual(groups[0].code, "unsupported_feature")
        XCTAssertEqual(groups[0].id, "error:unsupported_feature:\\in is not supported in math mode")
        let split: [RuntimeV1.Diagnostic] = [
            .init(severity: .error, message: "m", source: .init(path: "main.tex", startByte: 0, endByte: 1), recovery: nil,
                  code: "unknown_command"),
            .init(severity: .error, message: "m", source: .init(path: "main.tex", startByte: 2, endByte: 3), recovery: nil,
                  code: "unsupported_feature"),
            .init(severity: .error, message: "m", source: .init(path: "main.tex", startByte: 4, endByte: 5), recovery: nil),
        ]
        let byCode = EditorDiagnostics.groups(of: split, documentOrder: ["main.tex"])
        XCTAssertEqual(Set(byCode.map(\.id)), Set([
            "error:unknown_command:m",
            "error:unsupported_feature:m",
            "error:m",
        ]), "code in the key; missing code falls back to severity:message")
        XCTAssertEqual(byCode.map(\.id), [
            "error:unknown_command:m",
            "error:m",
            "error:unsupported_feature:m",
        ], "author errors (unknown_command, then the uncoded error) before the unsupported_feature gap")
    }

    func testCopyLineIncludesHelpAndNotes() {
        let texts = ["main.tex": Self.text]
        let d = RuntimeV1.Diagnostic(
            severity: .error, message: "\\in is not supported in math mode",
            source: .init(path: "main.tex", startByte: 9, endByte: 12), recovery: "rendered as text",
            code: "unsupported_feature",
            notes: ["math mode only"],
            help: .init(message: "wrap in $...$"))
        XCTAssertEqual(EditorDiagnostics.copyLine(d, texts: texts), """
            main.tex:2: error: \\in is not supported in math mode
            = note: math mode only
            = help: wrap in $...$
            """)
        XCTAssertEqual(EditorDiagnostics.secondaryLabelHelp(d), nil)
        let labeled = RuntimeV1.Diagnostic(
            severity: .error, message: "m", source: nil, recovery: nil,
            labels: [
                .init(source: .init(path: "main.tex", startByte: 0, endByte: 1), text: "primary", primary: true),
                .init(source: .init(path: "main.tex", startByte: 2, endByte: 3), text: "secondary one", primary: false),
                .init(source: .init(path: "ch.tex", startByte: 0, endByte: 1), text: "secondary two", primary: false),
            ])
        XCTAssertEqual(EditorDiagnostics.secondaryLabelHelp(labeled), "secondary one\nsecondary two")
    }

    // MARK: copy text

    func testCopyLineFormat() {
        let texts = ["main.tex": Self.text]
        let diags = diagnostics()
        XCTAssertEqual(EditorDiagnostics.copyLine(diags[1], texts: texts), "main.tex:2: error: \\in is not supported in math mode")
        XCTAssertEqual(EditorDiagnostics.copyLine(diags[2], texts: texts), "main.tex:4: warning: \\mathbb is not supported")
        XCTAssertEqual(EditorDiagnostics.copyLine(diags[4], texts: texts), "-:0: warning: Overfull line", "unsourced")
        XCTAssertEqual(EditorDiagnostics.copyLine(diags[1]), "main.tex:byte9: error: \\in is not supported in math mode",
                       "no compiled text: the byte offset stands in for the line")
    }

    func testCopyTextIsTheSelectedGroupInDocumentOrderOrEverything() throws {
        let diags = diagnostics()
        let texts = ["main.tex": Self.text]
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: ["main.tex"])
        XCTAssertEqual(groups.map(\.count), [1, 1, 3], "warnings (mathbb, Overfull) then the \\in gap group")
        let inGroup = try XCTUnwrap(groups.first { $0.count == 3 })
        XCTAssertEqual(EditorDiagnostics.copyText(groups: groups, selection: inGroup.id, in: diags, texts: texts),
                       """
                       main.tex:2: error: \\in is not supported in math mode
                       main.tex:3: error: \\in is not supported in math mode
                       main.tex:5: error: \\in is not supported in math mode
                       """, "every place of the grouped row, document order, no trailing newline")
        let overfull = try XCTUnwrap(groups.first { $0.message == "Overfull line" })
        XCTAssertEqual(EditorDiagnostics.copyText(groups: groups, selection: overfull.id, in: diags, texts: texts),
                       "-:0: warning: Overfull line")
        let all = EditorDiagnostics.copyText(groups: groups, selection: nil, in: diags, texts: texts)
        XCTAssertEqual(all.split(separator: "\n").count, 5, "no selection: all diagnostics, group by group")
        XCTAssertTrue(all.hasPrefix("main.tex:4: warning"), all)
        XCTAssertTrue(all.hasSuffix("main.tex:5: error: \\in is not supported in math mode"), all)
        XCTAssertEqual(EditorDiagnostics.copyText(groups: groups, selection: "error:not a group", in: diags, texts: texts), all,
                       "a stale selection copies everything rather than nothing")
        XCTAssertEqual(EditorDiagnostics.copyText(groups: [], selection: nil, in: [], texts: texts), "")
    }

    // MARK: #76 remainder — errors, then warnings, then FlashTeX gaps

    /// Gap at the start of main.tex, warnings in the middle, real typos later
    /// and in chapter.tex. Groups list author errors first, then warnings,
    /// then `isGap` rows; within a bucket, `documentOrder` then start byte.
    /// Occurrence lists stay document order. The diagnostics array itself is
    /// not reordered (its indices are identity keys for marks / explanations).
    func testGroupsListErrorsThenWarningsThenGapsAcrossTwoDocuments() {
        let diags = mixedBucketDiagnostics()
        let original = diags.map(\.message)
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: ["main.tex", "chapter.tex"])
        XCTAssertEqual(diags.map(\.message), original, "groups() must not reorder result.diagnostics")
        XCTAssertEqual(groups.map(\.message), [
            "unknown command \\alpah",
            "unknown command \\textbff",
            "Overfull line",
            "Underfull line",
            "packages tikz are recognised but not implemented",
            "environment 'tabbing' is not implemented",
        ])
        XCTAssertEqual(groups.map(\.first), [2, 3, 1, 5, 0, 4], "first occurrence index in document order within each bucket")
        XCTAssertEqual(groups.map(\.occurrences), [[2], [3], [1], [5], [0], [4]])
    }

    /// Acceptance: 3 gaps, then 1 warning, then 1 error, in that source order,
    /// must list the error first (30 gaps cannot bury the one real error).
    func testThreeGapsThenWarningThenErrorListsTheErrorFirst() {
        let diags: [RuntimeV1.Diagnostic] = [
            .init(severity: .error, message: "gap a", source: nil, recovery: nil, code: "unsupported_feature"),
            .init(severity: .error, message: "gap b", source: nil, recovery: nil, code: "unsupported_feature"),
            .init(severity: .error, message: "gap c", source: nil, recovery: nil, code: "unsupported_feature"),
            .init(severity: .warning, message: "Overfull line", source: nil, recovery: nil),
            .init(severity: .error, message: "unknown command \\alpah", source: nil, recovery: nil, code: "unknown_command"),
        ]
        XCTAssertEqual(EditorDiagnostics.groups(of: diags).map(\.message), [
            "unknown command \\alpah",
            "Overfull line",
            "gap a", "gap b", "gap c",
        ])
    }

    /// Copy Diagnostics (no selection) walks groups in the same bucket order
    /// as the panel, then each group's occurrences in document order.
    func testCopyTextFollowsBucketThenDocumentOrder() {
        let diags = mixedBucketDiagnostics()
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: ["main.tex", "chapter.tex"])
        let all = EditorDiagnostics.copyText(groups: groups, selection: nil, in: diags)
        XCTAssertEqual(all, """
            main.tex:byte20: error: unknown command \\alpah
            chapter.tex:byte5: error: unknown command \\textbff
            main.tex:byte10: warning: Overfull line
            chapter.tex:byte40: warning: Underfull line
            main.tex:byte0: error: packages tikz are recognised but not implemented
            chapter.tex:byte0: error: environment 'tabbing' is not implemented
            """)
        XCTAssertEqual(EditorDiagnostics.copyText(groups: groups, selection: groups[0].id, in: diags),
                       "main.tex:byte20: error: unknown command \\alpah")
    }

    /// ⌘⌥] with nothing selected starts on the first group of the list — the
    /// first author error, not the earlier gap — so "3 of 12" and stepping
    /// agree with what the panel shows.
    func testStepOccurrenceStartsOnFirstErrorGroupNotEarlierGap() {
        let m = ShellModel()
        m.replaceProject(entryText: "abcdefghijKLMNOPqr\\alpah")
        let diags: [RuntimeV1.Diagnostic] = [
            .init(severity: .error, message: "packages tikz are recognised but not implemented",
                  source: .init(path: "main.tex", startByte: 0, endByte: 4), recovery: nil, code: "unsupported_feature"),
            .init(severity: .warning, message: "Overfull line",
                  source: .init(path: "main.tex", startByte: 10, endByte: 14), recovery: nil),
            .init(severity: .error, message: "unknown command \\alpah",
                  source: .init(path: "main.tex", startByte: 18, endByte: 24), recovery: nil, code: "unknown_command"),
        ]
        m.result = RuntimeV1.CompileResult(projectId: "p", revision: m.editorRevision, status: .recovered,
                                           pages: [], diagnostics: diags, pdfPath: nil)
        m.resultID = "r-order"
        m.setCompiledDocuments(["main.tex": m.activeText])
        let panel = DiagnosticsPanelState()
        m.stepOccurrence(forward: true, panel: panel)
        XCTAssertEqual(panel.selection, EditorDiagnostics.groups(of: diags, documentOrder: ["main.tex"])[0].id)
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 18, length: 6), "lands on \\alpah, not the gap at byte 0")
        XCTAssertEqual(m.currentDiagnosticID, "r-order#2@main.tex:18..<24")
    }

    func testSummaryNamesPerBucketCounts() {
        var diags: [RuntimeV1.Diagnostic] = [
            .init(severity: .error, message: "e1", source: nil, recovery: nil, code: "unknown_command"),
            .init(severity: .error, message: "e2", source: nil, recovery: nil, code: "syntax_error"),
            .init(severity: .warning, message: "w1", source: nil, recovery: nil),
            .init(severity: .warning, message: "w2", source: nil, recovery: nil),
            .init(severity: .warning, message: "w3", source: nil, recovery: nil),
            .init(severity: .warning, message: "w4", source: nil, recovery: nil),
            .init(severity: .warning, message: "w5", source: nil, recovery: nil),
        ]
        diags += (0..<46).map { i in
            RuntimeV1.Diagnostic(severity: .error, message: "gap \(i)", source: nil, recovery: nil, code: "unsupported_feature")
        }
        XCTAssertEqual(EditorDiagnostics.summary(diags), "2 errors · 5 warnings · 46 FlashTeX gaps")
        XCTAssertEqual(EditorDiagnostics.summary([]), "0 errors · 0 warnings · 0 FlashTeX gaps")
        XCTAssertEqual(EditorDiagnostics.summary([
            .init(severity: .error, message: "e", source: nil, recovery: nil, code: "unknown_command"),
        ]), "1 error · 0 warnings · 0 FlashTeX gaps")
        XCTAssertEqual(EditorDiagnostics.summary([
            .init(severity: .warning, message: "w", source: nil, recovery: nil),
        ]), "0 errors · 1 warning · 0 FlashTeX gaps")
        XCTAssertEqual(EditorDiagnostics.summary([
            .init(severity: .error, message: "g", source: nil, recovery: nil, code: "unsupported_feature"),
        ]), "0 errors · 0 warnings · 1 FlashTeX gap")
        XCTAssertEqual(EditorDiagnostics.listBucket(diags[0]), 0)
        XCTAssertEqual(EditorDiagnostics.listBucket(diags[2]), 1)
        XCTAssertEqual(EditorDiagnostics.listBucket(diags[7]), 2)
    }

    func testCopyDiagnosticsAsTextWritesThePasteboardAndNotesTheCount() {
        let m = model()
        let panel = DiagnosticsPanelState()
        let pb = NSPasteboard(name: .init("flashtex.tests.diagnostics.\(UUID().uuidString)"))
        defer { pb.releaseGlobally() }
        let written = m.copyDiagnosticsAsText(panel: panel, pasteboard: pb)
        XCTAssertEqual(pb.string(forType: .string), written)
        XCTAssertEqual(written.split(separator: "\n").count, 5)
        XCTAssertEqual(m.navigationNote, "Copied 5 diagnostic lines as text.")
        panel.selection = EditorDiagnostics.groups(of: m.displayedDiagnostics).first { $0.message == "Overfull line" }?.id
        XCTAssertEqual(m.copyDiagnosticsAsText(panel: panel, pasteboard: pb), "-:0: warning: Overfull line")
        XCTAssertEqual(pb.string(forType: .string), "-:0: warning: Overfull line")
        XCTAssertEqual(m.navigationNote, "Copied 1 diagnostic line as text.")
        // Nothing to copy leaves the pasteboard alone.
        m.result = RuntimeV1.CompileResult(projectId: "p", revision: m.editorRevision, status: .ok, pages: [], diagnostics: [], pdfPath: nil)
        XCTAssertEqual(m.copyDiagnosticsAsText(panel: nil, pasteboard: pb), "")
        XCTAssertEqual(pb.string(forType: .string), "-:0: warning: Overfull line")
        XCTAssertEqual(m.navigationNote, "No diagnostics to copy.")
    }

    // MARK: group info and occurrence stepping

    func testGroupInfoNamesTheCurrentOccurrenceLine() throws {
        let diags = diagnostics()
        let texts = ["main.tex": Self.text]
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: ["main.tex"])
        let g = try XCTUnwrap(groups.first { $0.count > 1 })
        XCTAssertEqual(EditorDiagnostics.groupInfo(g, occurrence: 0, in: diags, texts: texts),
                       .init(count: 3, occurrence: 0, location: "main.tex line 2"))
        XCTAssertEqual(EditorDiagnostics.groupInfo(g, occurrence: 2, in: diags, texts: texts)?.spoken, "3 places, 3 of 3, main.tex line 5")
        XCTAssertEqual(EditorDiagnostics.groupInfo(g, occurrence: 9, in: diags, texts: texts)?.occurrence, 2, "clamped")
        XCTAssertEqual(EditorDiagnostics.groupInfo(g, occurrence: 1, in: diags)?.location, "main.tex bytes 29..<32", "no compiled text")
        let mathbb = try XCTUnwrap(groups.first { $0.message.contains("mathbb") })
        XCTAssertNil(EditorDiagnostics.groupInfo(mathbb, occurrence: 0, in: diags, texts: texts), "single place: not a group")
        let doubled = diags + [diags[4]]
        let overfull = try XCTUnwrap(EditorDiagnostics.groups(of: doubled).first { $0.message == "Overfull line" })
        XCTAssertEqual(EditorDiagnostics.groupInfo(overfull, occurrence: 1, in: doubled)?.spoken,
                       "2 places, 2 of 2, no source")
        // The row label the panel builds from it.
        let row = DiagnosticRowAccessibility(diags[g.first], index: g.first, total: diags.count, status: .recovered,
                                             group: EditorDiagnostics.groupInfo(g, occurrence: 1, in: diags, texts: texts))
        XCTAssertEqual(row.label, "Diagnostic 2 of 5: Error: \\in is not supported in math mode, 3 places, 2 of 3, main.tex line 3")
    }

    func testStepOccurrenceWrapsWithinTheGroupAndFeedsDiagnosticNavigation() throws {
        let m = model()
        let panel = DiagnosticsPanelState()
        let groups = EditorDiagnostics.groups(of: m.displayedDiagnostics, documentOrder: ["main.tex"])
        let g = try XCTUnwrap(groups.first { $0.count > 1 })
        // No selection, no current diagnostic: the first multi-place group, from its first place.
        m.stepOccurrence(forward: true, panel: panel)
        XCTAssertEqual(panel.selection, g.id)
        XCTAssertEqual(panel.currentOccurrence(of: g), 0)
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 9, length: 3))
        XCTAssertEqual(m.navigationNote?.hasPrefix("1 of 3: main.tex line 2 — \\in is not supported in math mode"), true, m.navigationNote ?? "nil")
        XCTAssertEqual(m.currentDiagnosticID, "r1#1@main.tex:9..<12", "⌘⇧] continues from this occurrence")
        m.stepOccurrence(forward: true, panel: panel)
        XCTAssertEqual(panel.currentOccurrence(of: g), 1)
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 29, length: 3))
        m.stepOccurrence(forward: true, panel: panel)
        XCTAssertEqual(panel.currentOccurrence(of: g), 2)
        m.stepOccurrence(forward: true, panel: panel)
        XCTAssertEqual(panel.currentOccurrence(of: g), 0, "wraps within the group")
        m.stepOccurrence(forward: false, panel: panel)
        XCTAssertEqual(panel.currentOccurrence(of: g), 2, "wraps backwards")
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 55, length: 3))
        // Backwards with nothing selected starts from the last place.
        let fresh = DiagnosticsPanelState()
        m.currentDiagnosticID = nil
        m.stepOccurrence(forward: false, panel: fresh)
        XCTAssertEqual(fresh.currentOccurrence(of: g), 2)
        // With ⌘⇧] having reached the \mathbb warning, the step stays in its (single) group.
        let single = DiagnosticsPanelState()
        m.currentDiagnosticID = "r1#2@main.tex:39..<49"
        m.stepOccurrence(forward: true, panel: single)
        XCTAssertEqual(single.selection, groups.first { $0.message.contains("mathbb") }?.id)
        XCTAssertEqual(m.navigationNote?.hasPrefix("Only one place: "), true, m.navigationNote ?? "nil")
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 39, length: 10))
        // The selected group wins over the caret's diagnostic (from its first place, so the step lands on 2 of 3).
        single.selection = g.id
        m.stepOccurrence(forward: true, panel: single)
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 29, length: 3))
        XCTAssertEqual(single.currentOccurrence(of: g), 1)
        // Selecting a diagnostic and pressing Return goes to its current occurrence; no selection explains.
        let ret = DiagnosticsPanelState()
        m.goToSelectedOccurrence(panel: ret)
        XCTAssertEqual(m.navigationNote, "Select a diagnostic first (↑/↓), then press Return.")
        ret.selection = g.id; ret.occurrence[g.id] = 1
        m.goToSelectedOccurrence(panel: ret)
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 29, length: 3))
        XCTAssertEqual(ShellModel.diagnosticIndex(ofIdentityKey: "r1#12@main.tex:1..<2"), 12)
        XCTAssertNil(ShellModel.diagnosticIndex(ofIdentityKey: "garbage"))
        // Without diagnostics the step explains.
        m.result = RuntimeV1.CompileResult(projectId: "p", revision: m.editorRevision, status: .ok, pages: [], diagnostics: [], pdfPath: nil)
        m.stepOccurrence(forward: true, panel: ret)
        XCTAssertEqual(m.navigationNote, "No diagnostics to step through.")
    }

    func testReturnKeyboardToEditorReappliesTheCaretWithANewToken() {
        let m = model()
        m.caretUTF16 = 12; m.caretLengthUTF16 = 0
        XCTAssertNil(m.selection)
        m.returnKeyboardToEditor()
        XCTAssertEqual(m.selection, .init(path: "main.tex", nsRange: NSRange(location: 12, length: 0), token: 1))
        m.returnKeyboardToEditor()
        XCTAssertEqual(m.selection?.token, 2, "a new token re-applies the same range (the editor takes first responder again)")
        m.caretUTF16 = 10_000; m.caretLengthUTF16 = 5
        m.returnKeyboardToEditor()
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: (Self.text as NSString).length, length: 0), "clamped to the buffer")
    }

    // MARK: off-screen keyboard traversal

    private struct Host: View {
        var model: ShellModel
        var panel: DiagnosticsPanelState
        var body: some View {
            VStack(spacing: 0) {
                SourceEditorView(
                    text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                    selection: model.selection, pendingEdit: model.pendingEdit, marks: model.editorMarks, result: model.result,
                    onCaretChange: { model.caretUTF16 = $0 },
                    onSelectionChange: { model.caretLengthUTF16 = $0.length },
                    onEditApplied: { edit, text in model.editApplied(edit, newText: text) })
                    .frame(height: 200)
                DiagnosticsListView(diagnostics: model.displayedDiagnostics, panel: panel)
            }
            .environment(model)
        }
    }

    static func descendants(_ root: NSView) -> [NSView] { root.subviews.flatMap { [$0] + descendants($0) } }

    func key(_ code: UInt16, _ chars: String, window: NSWindow) -> NSEvent? {
        NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                         windowNumber: window.windowNumber, context: nil, characters: chars, charactersIgnoringModifiers: chars,
                         isARepeat: false, keyCode: code)
    }

    func settle() async throws { for _ in 0..<5 { try await Task.sleep(nanoseconds: 50_000_000) } }

    /// Editor text view → diagnostics table in reading order, both take
    /// first responder in a never-key window; Return goes to the selected
    /// group's occurrence; Esc leaves the editor's text view as first responder.
    func testPanelKeyboardTraversalReturnJumpsAndEscReturnsTheEditor() async throws {
        let m = model()
        let panel = DiagnosticsPanelState()
        let hostView = NSHostingView(rootView: Host(model: m, panel: panel))
        hostView.frame = NSRect(x: 0, y: 0, width: 640, height: 420)
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: hostView.frame, styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = hostView
        window.orderFrontRegardless() // never makeKey
        defer { window.orderOut(nil) }
        hostView.layoutSubtreeIfNeeded()
        var tv: NSTextView?
        var table: NSTableView?
        let deadline = Date().addingTimeInterval(10)
        while (tv == nil || table == nil), Date() < deadline {
            try await Task.sleep(nanoseconds: 50_000_000)
            tv = TypingBenchDriver.findTextView(in: [hostView])
            table = Self.descendants(hostView).compactMap { $0 as? NSTableView }.first
        }
        let editor = try XCTUnwrap(tv, "editor text view hosted")
        guard let list = table else {
            throw XCTSkip("SwiftUI did not back the diagnostics List with an NSTableView in this in-process host (\(Self.descendants(hostView).count) views); the key actions are covered by testStepOccurrence… / testReturnKeyboardToEditor…")
        }
        // Reading order: the editor above the list.
        let fe = editor.convert(editor.bounds, to: nil), fl = list.convert(list.bounds, to: nil)
        XCTAssertGreaterThan(fe.midY, fl.midY, "editor is above the diagnostics list (Tab reaches it first)")
        XCTAssertTrue(window.makeFirstResponder(editor))
        XCTAssertTrue(list.acceptsFirstResponder, "the diagnostics table takes keyboard focus")
        XCTAssertTrue(window.makeFirstResponder(list))
        XCTAssertTrue(window.firstResponder === list)
        XCTAssertFalse(window.firstResponder === editor, "the editor gave the keyboard up")
        XCTAssertEqual(list.numberOfRows, 3, "one row per group")

        // Return: jump to the selected group's current occurrence.
        let groups = EditorDiagnostics.groups(of: m.displayedDiagnostics, documentOrder: ["main.tex"])
        let inGroup = try XCTUnwrap(groups.first { $0.count > 1 })
        panel.selection = inGroup.id
        panel.occurrence[inGroup.id] = 1
        try await settle()
        var returnPath = "key event"
        if let ev = key(36, "\r", window: window) { window.sendEvent(ev) }
        try await settle()
        if m.selection?.nsRange != NSRange(location: 29, length: 3) {
            returnPath = "direct action (Return key event not delivered to SwiftUI onKeyPress in a non-key window)"
            m.goToSelectedOccurrence(panel: panel)
            try await settle()
        }
        XCTAssertEqual(m.selection?.nsRange, NSRange(location: 29, length: 3), "Return jumps to 2 of 3 (\(returnPath))")
        XCTAssertEqual(editor.selectedRange(), NSRange(location: 29, length: 3), "the editor shows the occurrence")
        XCTAssertTrue(window.firstResponder === editor, "a jump gives the editor the keyboard (\(returnPath))")

        // Esc from the list: the editor's text view is first responder again at its caret.
        XCTAssertTrue(window.makeFirstResponder(list))
        m.caretUTF16 = 29; m.caretLengthUTF16 = 3
        var escPath = "key event"
        if let ev = key(53, "\u{1B}", window: window) { window.sendEvent(ev) }
        try await settle()
        if !(window.firstResponder === editor) {
            escPath = "direct action (Esc key event not delivered to SwiftUI onKeyPress in a non-key window)"
            m.returnKeyboardToEditor()
            try await settle()
        }
        XCTAssertTrue(window.firstResponder === editor, "Esc returns the keyboard to the editor (\(escPath))")
        XCTAssertEqual(editor.selectedRange(), NSRange(location: 29, length: 3), "the caret is where it was")
        print("DiagnosticsPanelTests: Return via \(returnPath); Esc via \(escPath)")
    }
}
