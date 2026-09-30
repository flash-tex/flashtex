import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac
@testable import FlashTeXEditorCore

/// PLAN M3 acceptance in the real text view: `;enum3` Tab expands, Esc
/// leaves the literal, one undo restores it; Tab precedence; placeholders
/// selected; the completion list stays shut while capturing. Hosted
/// windows are non-activating and parked off-screen: the app never comes
/// forward and focus is never taken.
@MainActor
final class TeXpandEditorTests: XCTestCase {
    private var window: NSWindow!
    private var tv: CompletingTextView!

    override func setUp() async throws {
        var on = TeXpand.Settings()
        on.enabled = true
        TeXpandPreferences.override = on
        // Never the user's own texpand.toml.
        configDir = FileManager.default.temporaryDirectory.appendingPathComponent("texpand-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: configDir, withIntermediateDirectories: true)
        TeXpandPreferences.userConfigURL = configDir.appendingPathComponent("user/texpand.toml")
        HostedWindowSupport.prepare()
        window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless()
        window.makeFirstResponder(tv)
        tv.allowsUndo = true
        tv.projectDocumentClass = { "article" }
    }

    override func tearDown() async throws {
        window.orderOut(nil)
        TeXpandPreferences.override = nil
        try? FileManager.default.removeItem(at: configDir)
    }

    private var configDir: URL!

    /// M8: a project `texpand.toml` overrides a built-in and reloads on save;
    /// a user file sits under it; a magic comment tops both; a malformed
    /// definition only produces a diagnostic naming its layer.
    func testConfigLayersAndHotReload() throws {
        let project = configDir.appendingPathComponent("project")
        try FileManager.default.createDirectory(at: project, withIntermediateDirectories: true)
        tv.texpandProject = { [project] in TeXpandProject(activePath: "main.tex", entryPath: "main.tex", root: project, text: { _ in nil }) }
        try FileManager.default.createDirectory(at: TeXpandPreferences.userConfigURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        try "[[abbr]]\nname = \"hi\"\nbody = 'user hi'\n[[abbr]]\nname = \"sec\"\nbody = 'user sec'\n"
            .write(to: TeXpandPreferences.userConfigURL, atomically: true, encoding: .utf8)
        let projectFile = project.appendingPathComponent("texpand.toml")
        try "[[abbr]]\nname = \"sec\"\nbody = '\\section{<<arg.1>>} % project'\n".write(to: projectFile, atomically: true, encoding: .utf8)

        start("")
        type(";sec{A}")
        tab()
        XCTAssertEqual(tv.string, "\\section{A} % project", "the project file overrides the built-in and the user file")
        start("")
        type(";hi")
        tab()
        XCTAssertEqual(tv.string, "user hi", "the user file")

        // Save a new version: the next use picks it up.
        try "[[abbr]]\nname = \"sec\"\nbody = 'reloaded'\n".write(to: projectFile, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.modificationDate: Date().addingTimeInterval(5)], ofItemAtPath: projectFile.path)
        start("")
        type(";sec")
        tab()
        XCTAssertEqual(tv.string, "reloaded", "hot reload")

        // A malformed definition: a diagnostic naming the layer, nothing else breaks.
        try "[[abbr]]\nname = \"sec\"\nbody = '<<oops>>'\n".write(to: projectFile, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.modificationDate: Date().addingTimeInterval(10)], ofItemAtPath: projectFile.path)
        start("")
        type(";")
        XCTAssertEqual(tv.texpand.configDiagnostics.map(\.description), ["error: texpand.toml (project):1 [sec]: `body`: unknown hole `<<oops>>`"])
        XCTAssertEqual(tv.texpand.notice, "error: texpand.toml (project):1 [sec]: `body`: unknown hole `<<oops>>`")
        type("sec")
        tab()
        XCTAssertEqual(tv.string, "user sec", "the next layer down still works")

        // A magic comment tops the files.
        start("% !texpand leader=, disable=hi\n")
        type(";hi")
        tab()
        XCTAssertTrue(tv.string.hasSuffix(";hi\t"), "`;` is no longer the leader, and `hi` is disabled: \(tv.string.debugDescription)")
        start("% !texpand leader=,\n")
        type(",sec")
        tab()
        XCTAssertTrue(tv.string.hasSuffix("user sec"), tv.string)
    }

    private func key(_ chars: String, code: UInt16, flags: NSEvent.ModifierFlags = []) {
        let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
                                 windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: chars,
                                 charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        tv.keyDown(with: e)
    }

    private func type(_ s: String) { for ch in s { key(String(ch), code: 0) } }
    private func tab() { key("\t", code: 48) }
    private func esc() { key("\u{1B}", code: 53) }

    /// Ends the run-loop turn, as separate key events do: the undo manager
    /// groups by event, so keystrokes sent back to back in one turn would
    /// otherwise share an undo group with what follows.
    private func endEvent() {
        RunLoop.main.run(until: Date().addingTimeInterval(0.02))
    }

    private func start(_ text: String) {
        tv.string = text
        tv.setSelectedRange(NSRange(location: (text as NSString).length, length: 0))
        tv.undoManager?.removeAllActions()
    }

    func testEnum3TabExpandsAndOneUndoRestoresTheLiteral() throws {
        start("")
        type(";enum3")
        XCTAssertTrue(tv.texpand.isCapturing)
        XCTAssertEqual(tv.texpand.region, NSRange(location: 0, length: 6))
        XCTAssertNotNil(tv.texpand.preview, "a live preview while capturing")
        endEvent()
        tab()
        endEvent()
        let unit = EditorPreferences.shared.indentString
        XCTAssertEqual(tv.string, "\\begin{enumerate}\n\(unit)\\item \n\(unit)\\item \n\(unit)\\item \n\\end{enumerate}")
        XCTAssertEqual(tv.selectedRange().location, ("\\begin{enumerate}\n\(unit)\\item " as NSString).length, "the caret in the first item")
        XCTAssertTrue(tv.isSnippetActive, "Tab visits the other items")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Expand Abbreviation")
        XCTAssertNil(tv.texpand.region)
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, ";enum3", "one undo restores the literal")
        XCTAssertFalse(tv.texpand.isCapturing, "and capture does not re-arm")
        tv.setSelectedRange(NSRange(location: 6, length: 0))
        tab()
        XCTAssertEqual(tv.string, ";enum3\t", "the undone literal is marked: Tab is a Tab again (a bare view inserts \\t)")
    }

    func testEscLeavesTheLiteralText() {
        start("See ")
        type(";sec{Intro}")
        XCTAssertTrue(tv.texpand.isCapturing)
        esc()
        XCTAssertEqual(tv.string, "See ;sec{Intro}")
        XCTAssertFalse(tv.texpand.isCapturing)
        XCTAssertNil(tv.session, "Esc ended the capture; it did not open the completion list")
        tab()
        XCTAssertNotEqual(tv.string, "See \\section{Intro}", "the Esc'd literal is not expanded by Tab")
    }

    func testPlaceholdersAreSelectedAndTabWalksThem() {
        start("")
        type(";fig")
        tab()
        let sel = tv.selectedRange()
        XCTAssertEqual((tv.string as NSString).substring(with: sel), "width=0.8\\linewidth", "the first placeholder is selected")
        XCTAssertTrue(tv.isSnippetActive, "a selected placeholder keeps the snippet")
        type("scale=1")
        XCTAssertTrue(tv.string.contains("\\includegraphics[scale=1]{}"), "typing replaced the placeholder")
        tab()
        XCTAssertEqual(tv.selectedRange().length, 0, "an empty field is a caret")
        type("a.pdf")
        key("\t", code: 48, flags: .shift)
        XCTAssertEqual((tv.string as NSString).substring(with: tv.selectedRange()), "scale=1", "⇧Tab reselects what was typed there")
    }

    func testTabPrecedence() {
        // Incomplete capture: Tab is swallowed (no Tab character) and explained.
        start("")
        type(";sec{Intro")
        tab()
        XCTAssertEqual(tv.string, ";sec{Intro")
        XCTAssertEqual(tv.texpand.diagnostic, "expected `}`")
        // Capture inside an active snippet: the capture wins over the stop.
        start("")
        type(";items2")
        tab()
        XCTAssertTrue(tv.isSnippetActive)
        type(";sec")
        tab()
        XCTAssertTrue(tv.string.contains("\\item \\section{}"), tv.string)
        // No capture: snippet stops, then indentation, as before.
        start("x")
        tab()
        XCTAssertNotEqual(tv.string, "x", "plain Tab still reaches the editor")
    }

    func testCompletionListStaysShutWhileCapturing() async throws {
        tv.automaticCompletionDelay = 0
        start("")
        type(";sec")
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertNil(tv.session)
        XCTAssertFalse(tv.hasPendingAutomaticCompletion)
    }

    private final class TextBox { var text = "" }

    private struct Host: View {
        let box: TextBox
        var body: some View {
            SourceEditorView(text: Binding(get: { box.text }, set: { box.text = $0 }), autoClosePairs: ["{", "["],
                             projectDocumentClass: { "article" })
        }
    }

    /// Through the hosted `SourceEditorView`: its coordinator auto-closes
    /// `{` (inserting `{}`) and types over the `}`, and the capture follows.
    func testThroughTheRealEditorWithAutoClose() throws {
        let box = TextBox()
        let host = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        host.contentView = NSHostingView(rootView: Host(box: box))
        host.orderFrontRegardless()
        defer { host.orderOut(nil) }
        var found: NSTextView?
        let deadline = Date().addingTimeInterval(10)
        while found == nil, Date() < deadline {
            RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.005))
            found = TypingBenchDriver.findTextView(in: [host.contentView!])
        }
        tv = try XCTUnwrap(found as? CompletingTextView)
        XCTAssertTrue(host.makeFirstResponder(tv))
        tv.allowsUndo = true
        type(";sec{")
        XCTAssertEqual(tv.string, ";sec{}", "the editor auto-closed the brace")
        type("Intro}")
        XCTAssertEqual(tv.string, ";sec{Intro}", "and the } was typed over")
        XCTAssertEqual(tv.texpand.preview, "\\section{Intro}")
        endEvent()
        tab()
        XCTAssertEqual(tv.string, "\\section{Intro}")
        endEvent()
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, ";sec{Intro}", "one undo restores the literal")
        RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.05))
        XCTAssertEqual(box.text, ";sec{Intro}", "the binding follows")
    }

    /// M4/M5 in the real text view: instant atoms, ligatures, postfix and
    /// the `//` fraction, each its own undo step.
    func testInstantAtomsLigaturesAndPostfixInTheEditor() {
        var on = TeXpand.Settings()
        on.enabled = true
        on.ligatures = true
        TeXpandPreferences.override = on
        start("$$")
        tv.setSelectedRange(NSRange(location: 1, length: 0))
        type(";a")
        endEvent()
        type("^")
        endEvent()
        type("2")
        XCTAssertEqual(tv.string, "$\\alpha^2$", "`;a^2` → `\\alpha^2` without Tab")
        XCTAssertEqual(tv.selectedRange().location, 9, "the caret stays after what was typed")
        endEvent()
        type(" ->")
        endEvent()
        XCTAssertEqual(tv.string, "$\\alpha^2 \\to $", "a ligature")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "$\\alpha^2 ->$", "one undo restores the trigger")

        start("$$")
        tv.setSelectedRange(NSRange(location: 1, length: 0))
        type("(x+1)//2")
        endEvent()
        tab()
        XCTAssertEqual(tv.string, "$\\frac{x+1}{2}$", "`(x+1)//2` Tab")
        start("$$")
        tv.setSelectedRange(NSRange(location: 1, length: 0))
        type("a/b")
        endEvent()
        tab()
        XCTAssertEqual(tv.string, "$a/b\t$", "a single `/` never expands")
        start("$$")
        tv.setSelectedRange(NSRange(location: 1, length: 0))
        type("\\alpha_i.hat")
        endEvent()
        tab()
        XCTAssertEqual(tv.string, "$\\hat{\\alpha_i}$")
        start("$$")
        tv.setSelectedRange(NSRange(location: 1, length: 0))
        type("3.14")
        endEvent()
        tab()
        XCTAssertEqual(tv.string, "$3.14\t$", "`3.14` never triggers postfix")
    }

    /// M6: `;cd:2x2` in a document without tikz-cd adds the package in the
    /// expansion's undo step; a second expansion adds nothing.
    func testAutoPreambleInsertsOnceInOneUndoStep() {
        let preamble = "\\documentclass{article}\n\\usepackage{amsmath}\n"
        start(preamble + "\\begin{document}\nA\n")
        type(";cd:2x2")
        endEvent()
        tab()
        endEvent()
        XCTAssertTrue(tv.string.hasPrefix(preamble + "\\usepackage{tikz-cd}\n\\begin{document}\nA\n\\begin{tikzcd}"), tv.string)
        XCTAssertEqual((tv.string as NSString).substring(with: tv.selectedRange()), "", "the caret in the first cell")
        XCTAssertTrue(tv.isSnippetActive, "the snippet's stops follow the insertion")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, preamble + "\\begin{document}\nA\n;cd:2x2", "one undo removes both")
        tv.undoManager?.redo()
        endEvent()
        tv.setSelectedRange(NSRange(location: (tv.string as NSString).length, length: 0))
        type("\n;cd:2x2")
        endEvent()
        tab()
        XCTAssertEqual(tv.string.components(separatedBy: "\\usepackage{tikz-cd}").count, 2, "inserted once")
    }

    func testAutoPreamblePromptAndOff() {
        var prompt = TeXpand.Settings()
        prompt.enabled = true
        prompt.autoPreamble = .prompt
        TeXpandPreferences.override = prompt
        start("\\documentclass{article}\n\\begin{document}\n")
        var asked: [String] = []
        tv.texpand.promptHandler = { missing, reply in asked = missing.map(\.name); reply(true) }
        type(";btab")
        endEvent()
        tab()
        XCTAssertEqual(asked, ["booktabs"])
        XCTAssertTrue(tv.string.contains("\\usepackage{booktabs}\n\\begin{document}"))
        var off = TeXpand.Settings()
        off.enabled = true
        off.autoPreamble = .off
        TeXpandPreferences.override = off
        start("\\documentclass{article}\n\\begin{document}\n")
        type(";btab")
        endEvent()
        tab()
        XCTAssertFalse(tv.string.contains("booktabs}\n\\begin"), "off: the preamble is left alone")
    }

    func testRootElsewhereLeavesANotice() {
        start("% !TEX root = main.tex\n")
        tv.texpandProject = { TeXpandProject(activePath: "chapter.tex", entryPath: "main.tex", root: nil,
                                             text: { $0 == "main.tex" ? "\\documentclass{article}\n\\usepackage{physics}\n\\begin{document}\n" : nil }) }
        type(";btab")
        endEvent()
        tab()
        XCTAssertTrue(tv.string.hasPrefix("% !TEX root = main.tex\n\\begin{tabular}"))
        XCTAssertEqual(tv.texpand.notice, "Needs \\usepackage{booktabs} in main.tex")
        start("$$")
        tv.setSelectedRange(NSRange(location: 1, length: 0))
        type(";dd:y/x")
        endEvent()
        tab()
        XCTAssertEqual(tv.string, "$\\dv{y}{x}$", "the root's packages pick the variant")
    }

    /// M7: the prompt expands at the caret, wraps a selection, distributes
    /// lines over a bare `*`, and turns CSV into a booktabs table.
    func testPromptAndWrap() {
        start("\\begin{document}\n")
        XCTAssertTrue(tv.texpand.expandFromPrompt("sec{Intro}#"))
        XCTAssertEqual(tv.string, "\\begin{document}\n\\section{Intro}\\label{sec:intro}")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Expand Abbreviation")

        start("apples\npears\nplums")
        tv.setSelectedRange(NSRange(location: 0, length: (tv.string as NSString).length))
        XCTAssertEqual(tv.texpand.promptPreview("enum>item*").text.components(separatedBy: "\\item").count, 4, "the preview")
        XCTAssertTrue(tv.texpand.expandFromPrompt("enum>item*"))
        let unit = EditorPreferences.shared.indentString
        XCTAssertEqual(tv.string, "\\begin{enumerate}\n\(unit)\\item apples\n\(unit)\\item pears\n\(unit)\\item plums\n\\end{enumerate}",
                       "three lines + enum>item* → three items")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "apples\npears\nplums", "one undo restores the selection")

        start("Name,Score\nAda,10")
        tv.setSelectedRange(NSRange(location: 0, length: (tv.string as NSString).length))
        XCTAssertTrue(tv.texpand.expandFromPrompt("btab"))
        XCTAssertTrue(tv.string.hasPrefix("\\begin{tabular}{ll}\n\(unit)\\toprule\n\(unit)Name & Score \\\\\n\(unit)\\midrule\n\(unit)Ada & 10"), tv.string)

        start("x")
        XCTAssertFalse(tv.texpand.expandFromPrompt("zzz"), "an unknown abbreviation changes nothing")
        XCTAssertEqual(tv.string, "x")
    }

    func testPromptPanelExpandsOnReturn() {
        start("A ")
        tv.texpandCommand(nil)
        let panel = TeXpandPromptPanel.shared
        XCTAssertTrue(panel.isVisible, "outside a structure the command opens the prompt")
        let field = panel.firstResponder as? NSTextView
        field?.insertText("eq", replacementRange: field?.selectedRange() ?? NSRange(location: 0, length: 0))
        field?.doCommand(by: #selector(NSResponder.insertNewline(_:)))
        XCTAssertFalse(panel.isVisible)
        XCTAssertTrue(tv.string.hasPrefix("A \\begin{equation}"), tv.string)
        TeXpandPreferences.override = TeXpand.Settings()
        tv.texpandCommand(nil)
        XCTAssertFalse(panel.isVisible, "off: the command does nothing")
    }

    /// M10b: ⌃⌘T inside a matrix opens the grid; edits, a new column and a
    /// type switch go back as one undo step with the caret in the cell;
    /// outside a grid the same command opens the prompt.
    func testStructureEditorRoundTrip() throws {
        let unit = EditorPreferences.shared.indentString
        let source = "$\\begin{pmatrix}\n\(unit)a & b \\\\\n\(unit)c & d\n\\end{pmatrix}$"
        start(source)
        tv.setSelectedRange(NSRange(location: (source as NSString).range(of: "d").location, length: 0))
        tv.texpandCommand(nil)
        let editor = try XCTUnwrap(tv.texpand.structureEditor, "the grid opens inside the matrix")
        XCTAssertTrue(editor.superview === tv, "a subview of the text view")
        XCTAssertEqual(editor.focus.row, 1)
        XCTAssertEqual(editor.focus.col, 1, "the caret's cell has focus")
        XCTAssertFalse(TeXpandPromptPanel.shared.isVisible)
        editor.setCell(1, 1, "x")
        editor.addColumn(nil)
        editor.switchEnvironment(to: "bmatrix")
        tv.texpandCommand(nil) // ⌃⌘T again: back to the source
        XCTAssertNil(tv.texpand.structureEditor)
        XCTAssertEqual(tv.string, "$\\begin{bmatrix}\n\(unit)a & b &  \\\\\n\(unit)c & x & \n\\end{bmatrix}$")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Edit Structure")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, source, "one undo step")

        // A tabular: Esc writes back, the spec follows the columns.
        start("\\begin{tabular}{l|r}\n\(unit)\\hline\n\(unit)A & B \\\\\n\\end{tabular}")
        tv.setSelectedRange(NSRange(location: 25, length: 0))
        tv.texpandCommand(nil)
        let tab = try XCTUnwrap(tv.texpand.structureEditor)
        tab.addRow(nil)
        tab.setCell(1, 0, "1")
        tab.addColumn(nil)
        tab.close(apply: true)
        XCTAssertTrue(tv.string.hasPrefix("\\begin{tabular}{l|l|r}\n\(unit)\\hline\n\(unit)A &  & B \\\\\n\(unit)1 & "),
                      "a column after the focused one, the spec in step: \(tv.string)")

        // Revert changes nothing; the source changing under it closes it.
        start("\\begin{cases}\n\(unit)1 & x>0\n\\end{cases}")
        tv.setSelectedRange(NSRange(location: 20, length: 0))
        tv.texpandCommand(nil)
        try XCTUnwrap(tv.texpand.structureEditor).setCell(0, 0, "2")
        tv.texpand.structureEditor?.revert(nil)
        XCTAssertEqual(tv.string, "\\begin{cases}\n\(unit)1 & x>0\n\\end{cases}")

        // Outside a grid, or with the structure editor off: the prompt.
        start("text")
        tv.texpandCommand(nil)
        XCTAssertNil(tv.texpand.structureEditor)
        XCTAssertTrue(TeXpandPromptPanel.shared.isVisible)
        TeXpandPromptPanel.shared.close(expanding: false)
        var noGrid = TeXpand.Settings()
        noGrid.enabled = true
        noGrid.disabled = ["structure:matrix"]
        TeXpandPreferences.override = noGrid
        start(source)
        tv.setSelectedRange(NSRange(location: 20, length: 0))
        tv.texpandCommand(nil)
        XCTAssertNil(tv.texpand.structureEditor, "the matrix provider is off")
        XCTAssertTrue(TeXpandPromptPanel.shared.isVisible)
        TeXpandPromptPanel.shared.close(expanding: false)
    }

    func testOffByDefaultDoesNothing() {
        TeXpandPreferences.override = TeXpand.Settings()
        start("")
        type(";enum3")
        XCTAssertFalse(tv.texpand.isCapturing)
        tab()
        XCTAssertTrue(tv.string.hasPrefix(";enum3"), "with the master switch off, Tab is the editor's: \(tv.string.debugDescription)")
    }

    func testTheSettingsSwitchTakesEffectLive() {
        TeXpandPreferences.override = TeXpand.Settings()
        start("")
        type(";sec")
        XCTAssertFalse(tv.texpand.isCapturing)
        var on = TeXpand.Settings()
        on.enabled = true
        on.leader = ","
        TeXpandPreferences.override = on
        start("")
        type(",sec{A}")
        tab()
        XCTAssertEqual(tv.string, "\\section{A}")
    }
}
