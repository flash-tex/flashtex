import AppKit
import XCTest
import FlashTeXProtocol
import FlashTeXAccessibility
@testable import FlashTeXMac

/// The redesigned main window's pure pieces (lane mac-ui-redesign): the
/// sidebar's outline scan (DocumentOutline.swift), outline navigation on the
/// model, the command palette's filter and runnable set (CommandPalette.swift),
/// and the workspace flags the View menu toggles.
@MainActor
final class WorkspaceShellTests: XCTestCase {

    // MARK: outline

    static let sample = """
    \\documentclass{article}
    \\begin{document}
    \\chapter{Intro} % \\section{not this}
    \\section*{Setup}\\label{sec:setup}
    Text with 100\\% and \\ref{sec:setup}.
    \\subsection[short]{Long title}
    \\begin{theorem}\\label{thm:main}
    \\begin{equation}
    x = 1
    \\end{equation}
    \\end{theorem}
    % \\label{commented}
    \\end{document}
    """

    func testOutlineScanFindsSectionsEnvironmentsAndLabelsInOrder() {
        let items = DocumentOutline.scan(Self.sample)
        XCTAssertEqual(items.map { "\($0.kind.rawValue):\($0.title)" },
                       ["section:Intro", "section:Setup", "label:sec:setup", "section:Long title",
                        "environment:theorem", "label:thm:main", "environment:equation"])
        XCTAssertEqual(items.map(\.line), [3, 4, 4, 6, 7, 7, 8])
        XCTAssertEqual(items.map(\.level), [0, 1, 0, 2, 0, 0, 1], "chapter 0, section 1, subsection 2; equation nests inside theorem")
        XCTAssertEqual(items[0].command, "chapter")
        XCTAssertEqual(items[1].command, "section", "the starred form keeps its name")
        // The command's own range is selectable text.
        let ns = Self.sample as NSString
        XCTAssertEqual(ns.substring(with: items[3].utf16), "\\subsection[short]{Long title}")
        XCTAssertEqual(ns.substring(with: items[5].utf16), "\\label{thm:main}")
        XCTAssertEqual(DocumentOutline.counts(items), [.section: 3, .environment: 2, .label: 2])
        XCTAssertEqual(DocumentOutline.items(.label, in: items).map(\.title), ["sec:setup", "thm:main"])
    }

    func testOutlineSkipsCommentsAndTheDocumentEnvironment() {
        let items = DocumentOutline.scan("% \\section{a}\n\\begin{document}\n\\section{b} %\\label{x}\n\\end{document}\n")
        XCTAssertEqual(items.map(\.title), ["b"])
        XCTAssertEqual(DocumentOutline.scan("").count, 0)
        XCTAssertEqual(DocumentOutline.scan("plain prose without commands").count, 0)
        // An escaped percent does not start a comment.
        XCTAssertEqual(DocumentOutline.scan("50\\% \\label{after}").map(\.title), ["after"])
    }

    func testOutlineRefusesOversizedBuffers() {
        let big = String(repeating: "\\section{x}\n", count: 1) + String(repeating: "a", count: DocumentOutline.maxScannedUTF16)
        XCTAssertEqual(DocumentOutline.scan(big), [], "beyond the bound the sidebar shows nothing rather than paying for the scan")
    }

    func testRevealOutlineItemSelectsTheCommandInTheEditor() {
        let model = ShellModel()
        model.replaceProject(entryText: Self.sample)
        let items = model.outline
        XCTAssertEqual(items.count, 7)
        model.reveal(outlineItem: items[3])
        XCTAssertEqual(model.selection?.path, "main.tex")
        XCTAssertEqual(model.selection?.nsRange, items[3].utf16)
        XCTAssertEqual(model.caretUTF16, items[3].utf16.location)
        XCTAssertEqual(model.caretLengthUTF16, items[3].utf16.length)
        XCTAssertEqual(model.navigationNote, "\\subsection at line 6.")
        let token = model.selection?.token
        // A stale item (buffer shrank) is refused with a note, never applied.
        model.updateActiveText("short")
        model.reveal(outlineItem: items[3])
        XCTAssertEqual(model.selection?.token, token, "no new selection")
        XCTAssertEqual(model.navigationNote, "Long title moved: the outline is older than the buffer.")
    }

    func testSwitchOrNoteReportsRefusals() {
        let model = ShellModel()
        model.replaceProject(entryText: "x")
        model.switchOrNote("missing.tex")
        XCTAssertEqual(model.navigationNote, "missing.tex is not open")
        XCTAssertEqual(model.activePath, "main.tex")
    }

    // MARK: command palette

    func testPaletteListsEveryCommandOfTheTableOnce() {
        let rows = CommandPaletteModel.rows(matching: "")
        XCTAssertEqual(rows.map(\.id), AccessibilityCommand.allCases, "table order, every command")
        XCTAssertEqual(Set(rows.map(\.id)).count, rows.count)
        let notRunnable = rows.filter { !$0.runnable }.map(\.id)
        XCTAssertEqual(Set(notRunnable), CommandPaletteModel.notRunnable)
        // Only editor keys, the preview click and the search window's own key are hints.
        XCTAssertEqual(Set(notRunnable), [.completion, .completionList, .toggleComment, .duplicateLine, .signatureHelp, .selectPreviewItemSource, .nextSearchMatch])
        for c in notRunnable { XCTAssertNil(c.entry.menuItem, "\(c) is not a menu item") }
        // Every menu item of the table is runnable from the palette.
        for e in AccessibilityCommand.entries where e.menuItem != nil { XCTAssertTrue(CommandPaletteModel.isRunnable(e.command), e.title) }
    }

    func testPaletteFilterMatchesTitleMenuShortcutAndRanksTitlesFirst() {
        XCTAssertEqual(CommandPaletteModel.rows(matching: "palette").map(\.id), [.commandPalette])
        XCTAssertEqual(CommandPaletteModel.rows(matching: "⌘⇧P").map(\.id), [.commandPalette])
        XCTAssertEqual(CommandPaletteModel.rows(matching: "⌘⌥P").map(\.id), [.pinInsertionPoint])
        // Multi-term: every term must match; case-insensitive.
        XCTAssertEqual(CommandPaletteModel.rows(matching: "EXPORT rust").map(\.id), [.exportPDFViaRust])
        XCTAssertEqual(CommandPaletteModel.rows(matching: "zzz-nothing"), [])
        // A menu name lists that menu's commands; title matches rank first.
        let navigate = CommandPaletteModel.rows(matching: "navigate").map(\.id)
        XCTAssertTrue(navigate.contains(.nextDiagnostic) && navigate.contains(.goToMatching))
        let problems = CommandPaletteModel.rows(matching: "problems").map(\.id)
        XCTAssertEqual(problems.first, .toggleProblems, "title match before description matches")
        XCTAssertTrue(problems.count >= 1)
        // Deterministic.
        XCTAssertEqual(CommandPaletteModel.rows(matching: "diagnostic"), CommandPaletteModel.rows(matching: "diagnostic"))
    }

    // MARK: completion popup documentation pane

    func testCompletionDocumentationNamesKindAndSyntax() {
        let cmd = Completion.Suggestion(label: "\\section", insertText: "\\section", kind: .command, detail: "supported by this compiler")
        let doc = CompletionPopup.documentationPane(for: cmd)
        XCTAssertEqual(doc.title, "\\section{…}")
        XCTAssertTrue(doc.body.hasPrefix("Command · supported by this compiler"))
        let ref = Completion.Suggestion(label: "sec:setup", insertText: "sec:setup", kind: .reference, detail: "label in main.tex")
        XCTAssertEqual(CompletionPopup.documentationPane(for: ref).title, "\\ref{sec:setup}")
        XCTAssertTrue(CompletionPopup.documentationPane(for: ref).body.contains("⌘⇧D"))
        for kind in [Completion.Kind.command, .environment, .reference, .citation, .word] {
            XCTAssertFalse(kind.symbolName.isEmpty)
            XCTAssertNotNil(NSImage(systemSymbolName: kind.symbolName, accessibilityDescription: nil), kind.badge)
        }
        // Panel bounds: the documentation pane is part of the popup's height.
        XCTAssertGreaterThan(CompletionPopup.docHeight, 40)
        XCTAssertEqual(ProblemsPanel.minHeight, 120)
        XCTAssertGreaterThan(ProblemsPanel.idealHeight, ProblemsPanel.minHeight)
    }

    // MARK: workspace flags

    func testWorkspaceFlagsDefaultToVisiblePanelAndClosedPalette() {
        let model = ShellModel()
        XCTAssertTrue(model.problemsVisible)
        XCTAssertFalse(model.commandPaletteShown)
        XCTAssertNil(model.problemsSeverityFilter)
        XCTAssertNil(model.problemsPanel.selection)
    }

    func testViewMenuCommandsAreInTheTableWithTheirShortcuts() {
        XCTAssertEqual(AccessibilityCommand.commandPalette.entry.shortcuts, ["⌘⇧P"])
        XCTAssertEqual(AccessibilityCommand.commandPalette.entry.menu, "View")
        XCTAssertEqual(AccessibilityCommand.toggleProblems.entry.shortcuts, ["⌘⇧M"])
        XCTAssertEqual(AccessibilityCommand.pinInsertionPoint.entry.shortcuts, ["⌘⌥P"], "the palette took ⌘⇧P")
        XCTAssertEqual(AccessibilityCommand.renameCitation.entry.menuItem, "Rename Citation…")
    }
}
