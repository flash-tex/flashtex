import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// Prose tokenizer and the hosted spell checker (issue #67, lane daniel-ux-spellcheck).
@MainActor
final class LaTeXSpellCheckTests: XCTestCase {
    /// Words (letter runs) the spell checker would see.
    private func words(_ s: String) -> [String] {
        LaTeXProse.maskedText(s as NSString)
            .components(separatedBy: CharacterSet.letters.inverted)
            .filter { !$0.isEmpty }
    }

    func testCommandsMathAndCommentsAreNotProse() {
        XCTAssertEqual(words(#"\section{Introduction} We use $x^{abc}$ and \(yy\) here."#), ["Introduction", "We", "use", "and", "here"])
        XCTAssertEqual(words("Display $$ qqq $$ and \\[ zzz \\] done % a commment\nnext"), ["Display", "and", "done", "next"])
        XCTAssertEqual(words("An \\textbf{important} \\emph{word}\\\\[3pt] end"), ["An", "important", "word", "end"])
        XCTAssertEqual(words("100\\% of \\$5 \\& more"), ["of", "more"])
        // An unterminated inline $ ends at a blank line.
        XCTAssertEqual(words("broken $x + y\n\nNew paragraph"), ["broken", "New", "paragraph"])
    }

    func testMathAndVerbatimEnvironmentBodiesAreSkipped() {
        let s = """
        Before \\begin{equation}\\label{eq:one} qwerty \\text{asdf} \\end{equation} middle
        \\begin{align*} a &= bbb \\\\ \\end{align*}
        \\begin{verbatim}
        raw textt

        more rawt
        \\end{verbatim}
        after \\verb|inline codde| tail
        \\begin{itemize}[leftmargin=*] \\item Point \\end{itemize}
        """
        XCTAssertEqual(words(s), ["Before", "middle", "after", "tail", "Point"])
    }

    func testReferenceFileAndOptionArgumentsAreSkipped() {
        let s = #"""
        \documentclass[11pt]{article}
        \usepackage[margin=1in]{geometry}
        \usepackage{amsmath,amssymb}
        \newcommand{\R}{\mathbb{R}}
        \def\foo#1{barbaz #1}
        See \ref{sec:intro}, \eqref{eq:mainn} and \cite[p.~3]{knuth84}.
        \includegraphics[width=0.5\textwidth]{figs/plott.png}
        \href{https://exmaple.com}{the site} \url{https://x.y/zz}
        \setlength{\parindent}{0pt} \vspace{0.6em}
        \section[short=yes]{Title}
        """#
        XCTAssertEqual(words(s), ["See", "and", "the", "site", "Title"])
    }

    func testAccentedWordsAreSkippedWhole() {
        XCTAssertEqual(words(#"na\"ive caf\'e and Erd\H{o}s wrote"#), ["and", "wrote"])
    }

    func testMaskKeepsOffsetsAndRangesAreMaximal() {
        let s = "ab \\cmd{cd} $e$"
        let masked = LaTeXProse.maskedText(s as NSString)
        XCTAssertEqual((masked as NSString).length, (s as NSString).length)
        XCTAssertEqual(LaTeXProse.proseRanges(in: s as NSString), [NSRange(location: 0, length: 3), NSRange(location: 8, length: 2), NSRange(location: 11, length: 1)])
    }

    func testRealWorldFixtureTokenizesQuickly() throws {
        // The HW1 fixture: preamble and math produce no prose words beyond real text.
        let url = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("../../../../fixtures/real-world/hw1/HW1.tex").standardized
        let text = try String(contentsOf: url, encoding: .utf8)
        let t0 = Date()
        let w = Set(words(text))
        XCTAssertLessThan(Date().timeIntervalSince(t0), 0.5)
        for code in ["documentclass", "usepackage", "amsmath", "mathbb", "leftmargin", "shortlabels", "mid", "hfill", "bfseries"] {
            XCTAssertFalse(w.contains(code), code)
        }
        XCTAssertTrue(w.contains("Solutions"))
    }

    // MARK: hosted editor

    struct Host: View {
        var model: ShellModel
        var body: some View {
            SourceEditorView(text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                             selection: model.selection, pendingEdit: model.pendingEdit, result: model.result)
        }
    }

    func testHostedEditorFlagsProseOnlyAndCorrectsThroughTheEditPath() async throws {
        let text = "Teh cat \\begin{equation} xqzv \\end{equation} \\label{sec:qqzz} sleeps.\n"
        let model = ShellModel()
        model.updateActiveText(text)
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model))
        window.orderFrontRegardless()
        defer { window.orderOut(nil) }
        var found: NSTextView?
        var deadline = Date().addingTimeInterval(10)
        while Date() < deadline, found == nil {
            found = TypingBenchDriver.findTextView(in: [window.contentView!])
            if found == nil { try await Task.sleep(nanoseconds: 10_000_000) }
        }
        let tv = try XCTUnwrap(found)
        let co = try XCTUnwrap(tv.delegate as? SourceEditorView.Coordinator)
        XCTAssertFalse(tv.isContinuousSpellCheckingEnabled)
        guard EditorPreferences.shared.spellCheck else { throw XCTSkip("spell check preference is off in this user's defaults") }
        deadline = Date().addingTimeInterval(10)
        while Date() < deadline, co.spelling.lastPainted.isEmpty { try await Task.sleep(nanoseconds: 20_000_000) }
        let flagged = co.spelling.lastPainted.map { (text as NSString).substring(with: $0) }
        XCTAssertEqual(flagged, ["Teh"], "math bodies and label keys are not checked")
        XCTAssertNotNil(co.spelling.misspelledRange(at: 1))
        XCTAssertNil(co.spelling.misspelledRange(at: 26))

        let menu = co.textView(tv, menu: NSMenu(), for: NSEvent(), at: 1) ?? NSMenu()
        let fix = try XCTUnwrap(menu.items.first { $0.title == "The" }, menu.items.map(\.title).description)
        XCTAssertTrue(menu.items.contains { $0.title == "Learn Spelling" })
        tv.window?.makeFirstResponder(tv)
        co.spelling.replaceWord(fix)
        XCTAssertTrue(tv.string.hasPrefix("The cat"))
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertTrue(model.activeText.hasPrefix("The cat"), "the correction reaches the model through the binding")
        tv.undoManager?.undo()
        XCTAssertTrue(tv.string.hasPrefix("Teh cat"), "the correction is one undo step")
    }
}
