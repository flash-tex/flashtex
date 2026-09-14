import AppKit
import XCTest
@testable import FlashTeXMac

/// Signature help (SignatureHelp.swift): the pure model and the panel on the
/// real hosted text view (`{` after a command opens it, `}`/Esc/caret leave
/// close it, ⌘⇧Space opens it on demand).
final class SignatureHelpTests: XCTestCase {
    func testInfoFindsTheCommandAndTheActiveArgument() {
        let frac = "$\\frac{a}{b"
        let i = SignatureHelp.info(in: frac, caretUTF16: (frac as NSString).length)
        XCTAssertEqual(i?.command, "frac")
        XCTAssertEqual(i?.arguments, "{num}{den}")
        XCTAssertEqual(i?.activeArgument, 1)
        XCTAssertEqual(i?.display.text, "\\frac{num}{den}")
        XCTAssertEqual(i?.display.active, NSRange(location: 10, length: 5))
        XCTAssertEqual(i?.description, "\\frac{num}{den}: a fraction.")
        XCTAssertEqual(i?.openerUTF16, 9)

        // First argument, caret between auto-closed braces; nested braces are balanced.
        let first = SignatureHelp.info(in: "\\frac{}{}", caretUTF16: 6)
        XCTAssertEqual(first?.activeArgument, 0)
        XCTAssertEqual(first?.display.active, NSRange(location: 5, length: 5))
        let nested = SignatureHelp.info(in: "\\section{The \\emph{best} id", caretUTF16: 27)
        XCTAssertEqual(nested?.command, "section")
        XCTAssertEqual(nested?.activeArgument, 0)
        // An optional argument counts as a group: `\sqrt[3]{` is argument 1 of `[index]{x}`.
        let opt = SignatureHelp.info(in: "\\sqrt[3]{", caretUTF16: 9)
        XCTAssertEqual(opt?.activeArgument, 1)
        XCTAssertEqual(opt?.groups, ["[index]", "{x}"])
        // Past the pattern: no active group, the help still names the command.
        let extra = SignatureHelp.info(in: "\\section{a}{", caretUTF16: 12)
        XCTAssertEqual(extra?.activeArgument, 1)
        XCTAssertNil(extra?.display.active)
        // Only CommandDocs knows it (the compiler does not render \chapter):
        // the pattern is empty, the doc line shows.
        let doc = SignatureHelp.info(in: "\\chapter{", caretUTF16: 9)
        XCTAssertNil(Completion.Vocabulary.byName["chapter"])
        XCTAssertEqual(doc?.arguments, "")
        XCTAssertEqual(doc?.description, "\\chapter{title}: a chapter heading (report and book classes).")
        // In the vocabulary and CommandDocs: the compiler's shape, CommandDocs' line.
        let foot = SignatureHelp.info(in: "\\footnote{", caretUTF16: 10)
        XCTAssertEqual(foot?.arguments, "[n]{...}")
        XCTAssertEqual(foot?.description, "\\footnote{text}: a numbered footnote.")

        // Nothing: outside braces, after the closing brace, escaped braces,
        // a comment, an unknown command, a brace with no command, a bad caret.
        XCTAssertNil(SignatureHelp.info(in: "\\frac{a}{b}", caretUTF16: 11))
        XCTAssertNil(SignatureHelp.info(in: "plain text", caretUTF16: 5))
        XCTAssertNil(SignatureHelp.info(in: "\\{ x", caretUTF16: 4))
        XCTAssertNil(SignatureHelp.info(in: "% \\frac{", caretUTF16: 8))
        XCTAssertNil(SignatureHelp.info(in: "\\zzzq{", caretUTF16: 6))
        XCTAssertNil(SignatureHelp.info(in: "a {b", caretUTF16: 4))
        XCTAssertNil(SignatureHelp.info(in: "\\frac{", caretUTF16: 99))
        XCTAssertNil(SignatureHelp.info(in: "\\frac{", caretUTF16: -1))
        // Bounded to the caret's line.
        XCTAssertNil(SignatureHelp.info(in: "\\frac{\nx", caretUTF16: 8))
    }

    @MainActor
    private func key(_ tv: NSTextView, _ chars: String, code: UInt16, flags: NSEvent.ModifierFlags = []) {
        let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
                                 windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: chars,
                                 charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        tv.keyDown(with: e)
    }

    @MainActor
    func testPanelOpensOnBraceFollowsTheArgumentAndCloses() throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless()
        window.makeFirstResponder(tv)
        defer { window.orderOut(nil) }
        tv.string = "$"
        tv.setSelectedRange(NSRange(location: 1, length: 0))
        for ch in "\\frac" { key(tv, String(ch), code: 0) }
        XCTAssertFalse(tv.isSignatureHelpVisible)
        key(tv, "{", code: 0)
        XCTAssertTrue(tv.isSignatureHelpVisible, "`{` after a command opens the help")
        let panel = tv.signatureHelpPanel
        XCTAssertEqual(panel.info?.command, "frac")
        XCTAssertEqual(panel.info?.activeArgument, 0)
        XCTAssertEqual(panel.patternText, "\\frac{num}{den}")
        XCTAssertEqual(panel.docText, "the numerator, above the rule", "the doc line is the active argument's own help")
        XCTAssertTrue(panel.isVisible)
        XCTAssertTrue(panel.parent === window)
        for ch in "a}{" { key(tv, String(ch), code: 0) }
        XCTAssertEqual(panel.info?.activeArgument, 1, "the second brace moves the highlight")
        XCTAssertEqual(SignatureHelpPanel.attributed(try XCTUnwrap(panel.info)).string, "\\frac{num}{den}")
        key(tv, "b", code: 0)
        XCTAssertTrue(tv.isSignatureHelpVisible)
        key(tv, "}", code: 0)
        XCTAssertFalse(tv.isSignatureHelpVisible, "the closing brace leaves the argument")
        XCTAssertFalse(panel.isVisible)
        XCTAssertEqual(tv.string, "$\\frac{a}{b}")

        // ⌘⇧Space inside an argument opens it on demand; Esc closes it without opening the completion list.
        tv.setSelectedRange(NSRange(location: 7, length: 0))
        key(tv, " ", code: 49, flags: [.command, .shift])
        XCTAssertTrue(tv.isSignatureHelpVisible)
        XCTAssertEqual(panel.info?.activeArgument, 0)
        key(tv, "\u{1B}", code: 53)
        XCTAssertFalse(tv.isSignatureHelpVisible)
        XCTAssertNil(tv.session)
        // The caret leaving the argument closes it; moving within it keeps it.
        key(tv, " ", code: 49, flags: [.command, .shift])
        XCTAssertTrue(tv.isSignatureHelpVisible)
        tv.setSelectedRange(NSRange(location: 8, length: 0))
        XCTAssertTrue(tv.isSignatureHelpVisible, "still inside the first argument")
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        XCTAssertFalse(tv.isSignatureHelpVisible)
        // ⌘⇧Space outside any argument shows nothing.
        key(tv, " ", code: 49, flags: [.command, .shift])
        XCTAssertFalse(tv.isSignatureHelpVisible)
        // A plain brace opens nothing.
        tv.string = "x "
        tv.setSelectedRange(NSRange(location: 2, length: 0))
        key(tv, "{", code: 0)
        XCTAssertFalse(tv.isSignatureHelpVisible)
    }

    // MARK: optional-argument awareness

    func testTheActiveGroupIsMatchedByBracketKindNotByCount() {
        // `\includegraphics`'s shape is `*[keys]{file}`. Writing no optional
        // must still land on `{file}` — counting groups would say `[keys]`.
        let plain = SignatureHelp.info(in: "\\includegraphics{", caretUTF16: 17)
        XCTAssertEqual(plain?.groups, ["[keys]", "{file}"])
        XCTAssertEqual(plain?.activeArgument, 1)
        XCTAssertEqual(plain?.display.active, NSRange(location: 22, length: 6))
        XCTAssertTrue(plain?.argumentHint?.hasPrefix("the image, without an extension") == true)
        // Writing it lands on the same group.
        let withOption = SignatureHelp.info(in: "\\includegraphics[width=2cm]{", caretUTF16: 28)
        XCTAssertEqual(withOption?.activeArgument, 1)
        // The caret inside the optional is argument 0.
        let inOption = SignatureHelp.info(in: "\\includegraphics[", caretUTF16: 17)
        XCTAssertEqual(inOption?.activeArgument, 0)
        XCTAssertTrue(inOption?.argumentHint?.contains("keepaspectratio") == true)
        // The starred form is the same command, not an extra group.
        XCTAssertEqual(SignatureHelp.info(in: "\\DeclareMathOperator*{", caretUTF16: 22)?.command, "DeclareMathOperator")
    }

    func testActiveIndexRules() {
        let pattern = ["[opt]", "{a}", "{b}"]
        XCTAssertEqual(SignatureHelp.activeIndex(groups: pattern, written: ["{"]), 1)          // optional skipped
        XCTAssertEqual(SignatureHelp.activeIndex(groups: pattern, written: ["["]), 0)
        XCTAssertEqual(SignatureHelp.activeIndex(groups: pattern, written: ["[", "{"]), 1)
        XCTAssertEqual(SignatureHelp.activeIndex(groups: pattern, written: ["{", "{"]), 2)
        XCTAssertEqual(SignatureHelp.activeIndex(groups: pattern, written: ["{", "{", "{"]), 3) // past the pattern
        XCTAssertEqual(SignatureHelp.activeIndex(groups: [], written: ["{"]), 0)
    }

    func testMiddleOptionalOfNewcommandIsSkippedWhenNotWritten() {
        // `{\name}[n]{body}`: `\newcommand{\x}{` is the body, not the arity.
        let body = SignatureHelp.info(in: "\\newcommand{\\x}{", caretUTF16: 16)
        XCTAssertEqual(body?.groups, ["{\\name}", "[n]", "{body}"])
        XCTAssertEqual(body?.activeArgument, 2)
        XCTAssertEqual(body?.argumentHint, "the body; #1 … #n are the arguments")
        let arity = SignatureHelp.info(in: "\\newcommand{\\x}[", caretUTF16: 16)
        XCTAssertEqual(arity?.activeArgument, 1)
        XCTAssertEqual(arity?.argumentHint, "how many arguments it takes, 0 to 9")
    }

    // MARK: per-argument hints

    func testHintsNameTheArgumentAndFallBackToTheDescription() {
        XCTAssertEqual(SignatureHelp.info(in: "\\frac{", caretUTF16: 6)?.detail, "the numerator, above the rule")
        XCTAssertEqual(SignatureHelp.info(in: "\\frac{a}{", caretUTF16: 9)?.detail, "the denominator, below the rule")
        XCTAssertEqual(SignatureHelp.info(in: "\\rule{1pt}{", caretUTF16: 11)?.argumentHint, "the height")
        // No hints for this command: the doc line is its documentation.
        let section = SignatureHelp.info(in: "\\section{", caretUTF16: 9)
        XCTAssertNil(section?.argumentHint)
        XCTAssertEqual(section?.detail, "\\section{title}: a numbered section heading. \\section* is unnumbered.")
        // Past the pattern there is no active group, so no hint.
        XCTAssertNil(SignatureHelp.info(in: "\\frac{a}{b}{", caretUTF16: 12)?.argumentHint)
    }

    /// Drift gate: every hint list must have exactly one entry per group of the
    /// compiler inventory's own pattern for that command. A shape that changes
    /// upstream fails here instead of mislabelling an argument in the panel.
    func testArgumentHintsMatchTheInventoryPatterns() {
        for (name, hints) in SignatureHelp.argumentHints {
            let entry = Completion.Vocabulary.byName[name]
            XCTAssertNotNil(entry, "\\\(name) is not in the compiler inventory")
            let groups = SignatureHelp.groups(of: entry?.arguments ?? "")
            XCTAssertEqual(hints.count, groups.count, "\\\(name): \(hints.count) hints for \(groups.count) groups \(groups)")
            XCTAssertFalse(hints.contains(where: \.isEmpty), "\\\(name) has an empty hint")
        }
    }

    func testEnvironmentSignatureHintsMatchTheirOwnPatterns() {
        for (name, signature) in SignatureHelp.environmentSignatures {
            let groups = SignatureHelp.groups(of: signature.arguments)
            XCTAssertEqual(signature.hints.count, groups.count, "\(name): \(signature.hints.count) hints for \(groups.count) groups")
            XCTAssertFalse(signature.description.isEmpty, "\(name) has no description")
        }
    }

    // MARK: environments

    func testTabularColumnSpecGetsHelp() throws {
        let info = try XCTUnwrap(SignatureHelp.info(in: "\\begin{tabular}{", caretUTF16: 16))
        XCTAssertEqual(info.kind, .environment)
        XCTAssertEqual(info.command, "tabular")
        XCTAssertEqual(info.head, "\\begin{tabular}")
        XCTAssertEqual(info.display.text, "\\begin{tabular}[pos]{cols}")
        XCTAssertEqual(info.activeArgument, 1, "the column spec, not the optional position")
        XCTAssertEqual(info.display.active, NSRange(location: 20, length: 6))
        XCTAssertTrue(info.argumentHint?.contains("p{width} wraps") == true)
        // The optional position, when written.
        XCTAssertEqual(SignatureHelp.info(in: "\\begin{tabular}[", caretUTF16: 16)?.activeArgument, 0)
    }

    func testFloatPlacementGetsHelp() throws {
        let figure = try XCTUnwrap(SignatureHelp.info(in: "\\begin{figure}[", caretUTF16: 15))
        XCTAssertEqual(figure.command, "figure")
        XCTAssertEqual(figure.activeArgument, 0)
        XCTAssertTrue(figure.argumentHint?.contains("h here") == true)
        // Starred environments share their base signature.
        XCTAssertEqual(SignatureHelp.info(in: "\\begin{table*}[", caretUTF16: 15)?.command, "table*")
        XCTAssertEqual(SignatureHelp.info(in: "\\begin{table*}[", caretUTF16: 15)?.arguments, "[placement]")
    }

    func testMinipageWalksItsThreeOptionals() {
        XCTAssertEqual(SignatureHelp.info(in: "\\begin{minipage}{", caretUTF16: 17)?.activeArgument, 3, "the width")
        XCTAssertEqual(SignatureHelp.info(in: "\\begin{minipage}[t]{", caretUTF16: 20)?.activeArgument, 3)
        XCTAssertEqual(SignatureHelp.info(in: "\\begin{minipage}[t][5cm][", caretUTF16: 25)?.activeArgument, 2, "the inner position")
    }

    func testTheEnvironmentNameGroupItselfIsStillTheBeginArgument() {
        // Inside `\begin{|}` the caret is naming the environment, not passing
        // it an argument, so the help is `\begin`'s own.
        let info = SignatureHelp.info(in: "\\begin{", caretUTF16: 7)
        XCTAssertEqual(info?.kind, .command)
        XCTAssertEqual(info?.command, "begin")
        XCTAssertEqual(info?.activeArgument, 0)
        // An environment with no signature of its own falls back to `\begin`.
        let unknown = SignatureHelp.info(in: "\\begin{center}{", caretUTF16: 15)
        XCTAssertEqual(unknown?.kind, .command)
        XCTAssertEqual(unknown?.command, "begin")
    }

    @MainActor
    func testPanelGrowsForAWrappedHelpLine() {
        let short = SignatureHelpPanel.height(for: "the width")
        let long = SignatureHelpPanel.height(for: SignatureHelp.columnSpecHint)
        XCTAssertEqual(short, SignatureHelpPanel.height(for: "x"))
        XCTAssertGreaterThan(long, short, "a sentence that wraps needs a taller panel")
        XCTAssertLessThanOrEqual(long, SignatureHelpPanel.height(for: String(repeating: "word ", count: 200)), "capped at docLines")
    }
}
