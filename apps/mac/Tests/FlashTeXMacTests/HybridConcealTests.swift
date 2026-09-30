import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac
@testable import FlashTeXEditorCore

/// Hybrid conceal (lane HYBRID-CONCEAL): the pure mapping and classes
/// (`HybridConcealTests`), and the display on a real hosted editor
/// (`HybridConcealEditorTests`): storage unchanged, reveal on the caret's line
/// and conceal again when it leaves, construct mode, toggles and the deny
/// list, undo and copy returning the source, and the caret never left inside
/// hidden source.
final class HybridConcealTests: XCTestCase {
    typealias Settings = HybridConceal.Settings
    typealias Span = HybridConceal.Span

    static let on = Settings(enabled: true, reveal: .line, classes: Set(HybridConceal.Class.allCases))

    /// Spans of the whole of `text` (every line), lexed like the editor does.
    static func spans(_ text: String, _ settings: Settings = on) -> [Span] {
        let ns = text as NSString
        var h = SyntaxHighlighter()
        h.reset(ns)
        return (0..<h.lineCount).flatMap { line -> [Span] in
            let r = h.lineRange(line)
            return r.length > 0 ? HybridConceal.spans(in: ns, lineRange: r, runs: h.runs(in: r, text: ns), settings: settings) : []
        }
    }

    /// What the line would show: replacements in, hidden source out, kept text as is.
    static func display(_ text: String, _ settings: Settings = on) -> String {
        let ns = text as NSString
        var pieces = spans(text, settings).flatMap(\.pieces).filter { if case .style = $0.action { return false }; return true }
        pieces.sort { $0.range.location < $1.range.location }
        var out = ""
        var i = 0
        for p in pieces where p.range.location >= i {
            out += ns.substring(with: NSRange(location: i, length: p.range.location - i))
            if case .replace(let s, let style) = p.action {
                out += style == .superscript ? "^(\(s))" : style == .subscript ? "_(\(s))" : s
            }
            i = NSMaxRange(p.range)
        }
        return out + ns.substring(from: i)
    }

    // MARK: classes

    func testGreekOnlyInMath() {
        XCTAssertEqual(Self.display("$\\alpha + \\Gamma = \\varepsilon$"), "$α + Γ = ε$")
        XCTAssertEqual(Self.display("\\alpha in text"), "\\alpha in text", "Greek commands in text are an error in LaTeX: left alone")
        XCTAssertEqual(Self.display("\\begin{align}\\phi\\end{align}"), "\\begin{align}ϕ\\end{align}")
        XCTAssertEqual(Self.display("$\\alphabet$"), "$\\alphabet$", "a longer command is not \\alpha")
    }

    func testSymbolsAndOperators() {
        XCTAssertEqual(Self.display("$a \\leq b \\to \\infty$, $x \\in A \\subseteq B$"), "$a ≤ b → ∞$, $x ∈ A ⊆ B$")
        XCTAssertEqual(Self.display("$\\sum_{i} \\int f \\neq 0$"), "$∑ᵢ ∫ f ≠ 0$")
    }

    func testScriptsUseUnicodeWhereItExists() {
        XCTAssertEqual(Self.display("$x^2 + a_i + e^{n+1} + y^T$"), "$x² + aᵢ + eⁿ⁺¹ + yᵀ$")
        XCTAssertEqual(Self.display("$x_{max}$"), "$xₘₐₓ$")
        XCTAssertEqual(Self.display("$x_{cy}$"), "$x_(cy)$", "no Unicode subscript c or y: a smaller lowered rendering")
        XCTAssertEqual(Self.display("$x^{\\alpha}$"), "$x^{α}$", "a script with a command keeps its markup; the command itself conceals")
        XCTAssertEqual(Self.display("a^2 in text"), "a^2 in text", "scripts are math only")
    }

    func testFontsHideTheCommandAndKeepOrMapTheText() {
        let text = "A \\textbf{bold} and \\emph{soft} $\\mathbb{R} \\mathcal{A} \\mathfrak{g} \\mathbf{v}$ \\texttt{code}"
        XCTAssertEqual(Self.display(text), "A bold and soft $ℝ 𝒜 𝔤 v$ code")
        let spans = Self.spans(text)
        let bold = try! XCTUnwrap(spans.first { $0.name == "textbf" })
        XCTAssertEqual(bold.pieces.map(\.action), [.hide, .style(.bold), .hide])
        XCTAssertEqual(bold.range, (text as NSString).range(of: "\\textbf{bold}"))
        XCTAssertEqual(spans.first { $0.name == "emph" }?.pieces[1].action, .style(.italic))
        XCTAssertEqual(Self.display("$\\mathbb{R}^n$"), "$ℝⁿ$")
        XCTAssertEqual(Self.display("\\textbf{unclosed"), "\\textbf{unclosed", "an argument must close on the line")
    }

    func testFractionsAreOffByDefault() {
        XCTAssertEqual(Self.display("$\\frac{a}{b}$"), "$a⁄b$")
        XCTAssertEqual(Self.display("$\\frac{a}{b}$", Settings(enabled: true)), "$\\frac{a}{b}$")
        XCTAssertFalse(Settings.defaultClasses.contains(.fractions))
        XCTAssertFalse(Settings.defaultClasses.contains(.sectioning))
    }

    func testQuotesAndDashesOnlyInText() {
        XCTAssertEqual(Self.display("``quoted'' -- and --- too"), "“quoted” – and — too")
        XCTAssertEqual(Self.display("$a--b$"), "$a--b$", "in math -- is two minus signs")
        XCTAssertEqual(Self.display("% a -- comment"), "% a -- comment", "comments are left alone")
        XCTAssertEqual(Self.display("\\ref{a--b}"), "\\ref{a--b}", "arguments that are keys are left alone")
    }

    func testItemsAndSections() {
        XCTAssertEqual(Self.display("\\item one"), "• one")
        XCTAssertEqual(Self.display("\\section{Intro} and \\subsection*{More}"), "§ Intro and §§ More")
        let heading = try! XCTUnwrap(Self.spans("\\section{Intro}").first)
        XCTAssertEqual(heading.pieces.map(\.action), [.replace("§ ", .heading), .style(.heading), .hide])
    }

    // MARK: switches

    func testMasterSwitchRevealNeverAndEmptyClassesConcealNothing() {
        let text = "$\\alpha$ \\textbf{b}"
        XCTAssertTrue(Self.spans(text, Settings(enabled: false)).isEmpty)
        XCTAssertTrue(Self.spans(text, Settings(enabled: true, reveal: .never)).isEmpty)
        XCTAssertTrue(Self.spans(text, Settings(enabled: true, classes: [.comments])).isEmpty, "dimming alone conceals nothing")
        XCTAssertTrue(Settings(enabled: true, classes: [.comments]).dimsComments)
        XCTAssertFalse(Settings(enabled: false).dimsComments)
    }

    func testEachClassTogglesOnItsOwn() {
        let text = "$\\alpha \\leq x^2$ \\textbf{b} -- \\item x"
        var s = Self.on
        s.classes.remove(.greek)
        XCTAssertEqual(Self.display(text, s), "$\\alpha ≤ x²$ b – • x")
        s = Self.on; s.classes.remove(.fonts)
        XCTAssertEqual(Self.display(text, s), "$α ≤ x²$ \\textbf{b} – • x")
        s = Self.on; s.classes.remove(.scripts)
        XCTAssertEqual(Self.display(text, s), "$α ≤ x^2$ b – • x")
    }

    /// The owner's example: conceal Greek but not `\textbf`; all Greek but `\phi`.
    func testDenyListKeepsNamedCommands() {
        var s = Self.on
        s.denied = Settings.parseDenyList("\\textbf, phi   --\n^")
        XCTAssertEqual(s.denied, ["textbf", "phi", "--", "^"])
        XCTAssertEqual(Self.display("$\\alpha \\phi x^2$ \\textbf{b} \\emph{e} a -- b", s), "$α \\phi x^2$ \\textbf{b} e a -- b")
        s.denied = ["section"]
        XCTAssertEqual(Self.display("\\section*{A}", s), "\\section*{A}", "the unstarred name denies the starred form")
        XCTAssertEqual(Settings.formatDenyList(["textbf", "phi", "--"]), "--, \\phi, \\textbf")
    }

    func testSettingsRoundTripAndToleratePartialOrNewerJSON() throws {
        var s = Self.on
        s.reveal = .construct
        s.denied = ["phi"]
        XCTAssertEqual(Settings.decoded(try XCTUnwrap(s.encoded())), s)
        let partial = try XCTUnwrap(Settings.decoded(Data(#"{"enabled": true, "classes": ["greek", "holograms"]}"#.utf8)))
        XCTAssertEqual(partial.classes, [.greek], "an unknown class from a newer app is dropped")
        XCTAssertEqual(partial.reveal, .line)
        XCTAssertTrue(partial.denied.isEmpty)
        XCTAssertEqual(Settings.decoded(Data("{}".utf8)), Settings())
        XCTAssertFalse(Settings().enabled, "off until the user turns it on")
    }

    // MARK: structure

    func testSpansStayOnTheirLineAndPiecesAreDisjoint() {
        let text = "\\textbf{a\nb} $\\alpha$\n$x^{2\n}$ ``q\n''"
        for span in Self.spans(text) {
            let ns = text as NSString
            XCTAssertFalse(ns.substring(with: span.range).contains("\n"), "\(span)")
            for (a, b) in zip(span.pieces, span.pieces.dropFirst()) {
                XCTAssertLessThanOrEqual(NSMaxRange(a.range), b.range.location)
            }
            for p in span.pieces { XCTAssertTrue(NSLocationInRange(p.range.location, span.range) && NSMaxRange(p.range) <= NSMaxRange(span.range)) }
        }
        XCTAssertEqual(Self.display(text), "\\textbf{a\nb} $α$\n$x^{2\n}$ “q\n”", "nothing spans a line break; each quote conceals on its own line")
    }

    func testRevealRules() {
        let span = Span(range: NSRange(location: 10, length: 6), cls: .greek, name: "alpha", pieces: [])
        let line = [NSRange(location: 0, length: 30)]
        XCTAssertTrue(HybridConceal.isRevealed(span, reveal: .line, selection: NSRange(location: 2, length: 0), revealedLines: line))
        XCTAssertFalse(HybridConceal.isRevealed(span, reveal: .line, selection: NSRange(location: 40, length: 0), revealedLines: [NSRange(location: 30, length: 20)]))
        XCTAssertTrue(HybridConceal.isRevealed(span, reveal: .construct, selection: NSRange(location: 10, length: 0), revealedLines: []), "touching the start")
        XCTAssertTrue(HybridConceal.isRevealed(span, reveal: .construct, selection: NSRange(location: 16, length: 0), revealedLines: []), "touching the end")
        XCTAssertFalse(HybridConceal.isRevealed(span, reveal: .construct, selection: NSRange(location: 17, length: 0), revealedLines: []))
        XCTAssertTrue(HybridConceal.isRevealed(span, reveal: .construct, selection: NSRange(location: 0, length: 12), revealedLines: []), "a selection over it")
        XCTAssertTrue(HybridConceal.isRevealed(span, reveal: .never, selection: NSRange(location: 99, length: 0), revealedLines: []))
    }

    func testEveryTableEntryIsASingleVisibleCharacterOrShortString() {
        for (name, s) in HybridConceal.greek.merging(HybridConceal.mathSymbols, uniquingKeysWith: { a, _ in a }) {
            XCTAssertFalse(s.isEmpty, name)
            XCTAssertLessThanOrEqual(s.count, 2, name)
        }
    }
}

// MARK: - on a hosted editor

@MainActor
final class HybridConcealEditorTests: XCTestCase {
    static let text = """
    Let $\\alpha \\leq \\beta^2$ and \\textbf{bold} text.
    Second ``quoted'' line -- with $x_i \\in \\mathbb{R}$.
    \\begin{itemize}
    \\item one
    \\end{itemize}

    """

    private var window: NSWindow?

    override func setUp() {
        super.setUp()
        ConcealController.settingsOverride = HybridConceal.Settings(enabled: true, reveal: .line)
    }

    override func tearDown() {
        window?.orderOut(nil)
        window = nil
        ConcealController.settingsOverride = nil
        EditorThemeRuntime.setCommentsDimmed(EditorPreferences.shared.conceal.dimsComments)
        super.tearDown()
    }

    func host(_ text: String = text, size: NSSize = NSSize(width: 700, height: 300)) async throws -> (NSTextView, SourceEditorView.Coordinator, ShellModel) {
        let model = ShellModel()
        model.replaceProject(entryText: text)
        let probe = LargeDocumentEditorTests.Probe()
        HostedWindowSupport.prepare() // non-activating: hosted windows never pull the app forward
        let w = HostedWindowSupport.window(contentRect: NSRect(origin: .zero, size: size), styleMask: [.titled])
        w.contentView = NSHostingView(rootView: LargeDocumentEditorTests.Host(model: model, probe: probe, marks: []))
        w.orderFrontRegardless() // never makeKey
        window = w
        var found: NSTextView?
        let deadline = Date().addingTimeInterval(20)
        while found == nil, Date() < deadline {
            found = TypingBenchDriver.findTextView(in: [w.contentView!])
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        let tv = try XCTUnwrap(found)
        let co = try XCTUnwrap(tv.delegate as? SourceEditorView.Coordinator)
        XCTAssertTrue(w.makeFirstResponder(tv))
        try await settle()
        return (tv, co, model)
    }

    func settle() async throws {
        try await Task.sleep(nanoseconds: 40_000_000)
        RunLoop.main.run(until: Date().addingTimeInterval(0.02))
    }

    func caret(_ tv: NSTextView, at location: Int) async throws {
        tv.setSelectedRange(NSRange(location: location, length: 0))
        try await settle()
    }

    /// Glyph property of the character at `index` after layout.
    func property(_ tv: NSTextView, at index: Int) -> NSLayoutManager.GlyphProperty {
        let lm = tv.layoutManager!
        lm.ensureLayout(forCharacterRange: NSRange(location: 0, length: tv.textStorage!.length))
        return lm.propertyForGlyph(at: lm.glyphIndexForCharacter(at: index))
    }

    func concealed(_ tv: NSTextView, _ needle: String) -> Bool {
        let at = (tv.string as NSString).range(of: needle).location
        return property(tv, at: at).contains(.controlCharacter) || property(tv, at: at).contains(.null)
    }

    func lineWidth(_ tv: NSTextView, line: Int) -> CGFloat {
        let lm = tv.layoutManager!
        let starts = [0] + (tv.string as NSString).components(separatedBy: "\n").dropLast().reduce(into: [Int]()) { a, l in a.append((a.last ?? 0) + (l as NSString).length + 1) }
        lm.ensureLayout(forCharacterRange: NSRange(location: 0, length: tv.textStorage!.length))
        return lm.lineFragmentUsedRect(forGlyphAt: lm.glyphIndexForCharacter(at: starts[line]), effectiveRange: nil).width
    }

    func testStorageIsNeverChangedAndOnlyOtherLinesAreConcealed() async throws {
        let (tv, co, model) = try await host()
        try await caret(tv, at: (Self.text as NSString).range(of: "\\item").location) // line 3
        XCTAssertTrue(tv.string == Self.text, "the storage is the source")
        XCTAssertTrue(model.activeText == Self.text)
        XCTAssertTrue(co.conceal.isActive)
        XCTAssertTrue(concealed(tv, "\\alpha"))
        XCTAssertTrue(concealed(tv, "\\textbf"))
        XCTAssertTrue(concealed(tv, "``"))
        XCTAssertFalse(concealed(tv, "\\item"), "the caret's line shows its source")
        XCTAssertFalse(concealed(tv, "bold"), "kept text keeps its glyphs")
        // The replacement box is as wide as the replacement: α is one column, not six.
        let font = try XCTUnwrap(tv.font)
        let lm = try XCTUnwrap(tv.layoutManager)
        let alpha = (Self.text as NSString).range(of: "\\alpha").location
        let box = lm.boundingRect(forGlyphRange: NSRange(location: lm.glyphIndexForCharacter(at: alpha), length: 1), in: tv.textContainer!)
        XCTAssertEqual(box.width, ceil(("α" as NSString).size(withAttributes: [.font: font]).width), accuracy: 0.5)
        // Bold kept text is drawn bold through a temporary attribute only.
        let boldAt = (Self.text as NSString).range(of: "bold").location
        XCTAssertNotNil(lm.temporaryAttribute(.shadow, atCharacterIndex: boldAt, effectiveRange: nil) as? NSShadow)
        XCTAssertNil(tv.textStorage?.attribute(.shadow, at: boldAt, effectiveRange: nil), "never a storage attribute")
    }

    func testRevealsOnEnteringTheLineAndConcealsOnLeaving() async throws {
        let (tv, co, _) = try await host()
        try await caret(tv, at: (Self.text as NSString).range(of: "\\item").location)
        let concealedWidth = lineWidth(tv, line: 0)
        XCTAssertTrue(concealed(tv, "\\alpha"))
        try await caret(tv, at: 3) // line 0
        XCTAssertFalse(concealed(tv, "\\alpha"), "revealed on entry")
        XCTAssertFalse(concealed(tv, "\\textbf"))
        XCTAssertGreaterThan(lineWidth(tv, line: 0), concealedWidth + 20, "the source is wider than what it means")
        XCTAssertTrue(concealed(tv, "``"), "line 1 still concealed")
        let boldAt = (Self.text as NSString).range(of: "bold").location
        XCTAssertNil(tv.layoutManager?.temporaryAttribute(.shadow, atCharacterIndex: boldAt, effectiveRange: nil), "revealed text is plain")
        let invalidations = co.conceal.glyphInvalidations
        // Typing on the revealed line re-lays out nothing extra.
        tv.insertText("x", replacementRange: tv.selectedRange())
        try await settle()
        XCTAssertEqual(co.conceal.glyphInvalidations, invalidations, "a keystroke on the caret's line never re-lays out for conceal")
        tv.moveDown(nil)
        try await settle()
        XCTAssertTrue(concealed(tv, "\\alpha"), "concealed again on leaving")
        XCTAssertFalse(concealed(tv, "``"), "and the new line is revealed")
        XCTAssertEqual(lineWidth(tv, line: 0), concealedWidth + ("x" as NSString).size(withAttributes: [.font: tv.font!]).width, accuracy: 1)
    }

    func testSelectionRevealsEveryLineItTouches() async throws {
        let (tv, _, _) = try await host()
        let second = (Self.text as NSString).range(of: "quoted").location
        tv.setSelectedRange(NSRange(location: 2, length: second - 2))
        try await settle()
        XCTAssertFalse(concealed(tv, "\\alpha"))
        XCTAssertFalse(concealed(tv, "``"))
    }

    func testConstructModeRevealsOnlyWhatTheCaretTouches() async throws {
        ConcealController.settingsOverride = HybridConceal.Settings(enabled: true, reveal: .construct)
        let (tv, co, _) = try await host()
        co.conceal.update(settings: ConcealController.settingsOverride!)
        let alpha = (Self.text as NSString).range(of: "\\alpha")
        try await caret(tv, at: NSMaxRange(alpha))
        XCTAssertFalse(concealed(tv, "\\alpha"), "the caret touches it")
        XCTAssertTrue(concealed(tv, "\\leq"), "its neighbour on the same line stays concealed")
        tv.moveLeft(nil) // one character at a time inside the revealed source
        try await settle()
        XCTAssertEqual(tv.selectedRange().location, NSMaxRange(alpha) - 1)
        XCTAssertFalse(concealed(tv, "\\alpha"))
        try await caret(tv, at: NSMaxRange((Self.text as NSString).range(of: "\\leq")) + 2)
        XCTAssertTrue(concealed(tv, "\\alpha"))
    }

    func testTogglesAndDenyListApplyLive() async throws {
        let (tv, co, _) = try await host()
        try await caret(tv, at: (Self.text as NSString).range(of: "\\item").location)
        var s = HybridConceal.Settings(enabled: true, reveal: .line)
        s.classes.remove(.greek)
        co.conceal.update(settings: s)
        try await settle()
        XCTAssertFalse(concealed(tv, "\\alpha"), "Greek off")
        XCTAssertTrue(concealed(tv, "\\leq"), "symbols still on")
        s = HybridConceal.Settings(enabled: true, reveal: .line, denied: ["textbf", "leq"])
        co.conceal.update(settings: s)
        try await settle()
        XCTAssertTrue(concealed(tv, "\\alpha"))
        XCTAssertFalse(concealed(tv, "\\leq"), "denied by name")
        XCTAssertFalse(concealed(tv, "\\textbf"), "conceal Greek but not \\textbf")
        co.conceal.update(settings: HybridConceal.Settings(enabled: false))
        try await settle()
        XCTAssertFalse(concealed(tv, "\\alpha"), "master switch off")
        XCTAssertFalse(co.conceal.isActive)
        XCTAssertTrue(tv.string == Self.text)
    }

    func testUndoAndCopyReturnTheSource() async throws {
        let (tv, _, model) = try await host()
        try await caret(tv, at: (Self.text as NSString).range(of: "\\item").location)
        // Copy a concealed line: the pasteboard gets the source.
        let line0 = (Self.text as NSString).lineRange(for: NSRange(location: 0, length: 0))
        tv.setSelectedRange(line0)
        let pasteboard = NSPasteboard.withUniqueName() // never the user's clipboard
        defer { pasteboard.releaseGlobally() }
        XCTAssertEqual(tv.selectedRange(), line0)
        XCTAssertTrue(tv.writeSelection(to: pasteboard, types: tv.writablePasteboardTypes), "\(tv.writablePasteboardTypes)")
        XCTAssertEqual(pasteboard.string(forType: .string), (Self.text as NSString).substring(with: line0))
        // Edit, move away (concealed again), undo: the source comes back exactly.
        try await caret(tv, at: 4)
        tv.insertText("Z", replacementRange: NSRange(location: 4, length: 0))
        try await settle()
        tv.breakUndoCoalescing()
        try await caret(tv, at: (Self.text as NSString).range(of: "\\item").location + 1)
        XCTAssertTrue(concealed(tv, "\\alpha"))
        XCTAssertTrue(tv.undoManager?.canUndo == true)
        tv.undoManager?.undo()
        try await settle()
        XCTAssertTrue(tv.string == Self.text, "undo restores the exact source")
        XCTAssertTrue(model.activeText == Self.text)
    }

    func testCaretNeverLandsInsideHiddenSource() async throws {
        let text = "\\textbf{bold} and $\\alpha\\beta\\gamma$ tail\n0123456789 0123456789 0123456789 0123\n"
        let (tv, co, _) = try await host(text)
        let spans = HybridConceal.spans(in: text as NSString, lineRange: (text as NSString).lineRange(for: NSRange(location: 0, length: 0)),
                                        runs: SyntaxHighlighter.runs(of: text as NSString), settings: co.conceal.settings)
        let hidden = spans.flatMap(\.pieces).filter { if case .style = $0.action { return false }; return true }.map(\.range)
        XCTAssertFalse(hidden.isEmpty)
        let secondLine = (text as NSString).range(of: "0123").location
        for column in 0..<36 {
            try await caret(tv, at: secondLine + column)
            tv.moveUp(nil)
            try await settle()
            let c = tv.selectedRange().location
            for r in hidden {
                XCTAssertFalse(c > r.location && c < NSMaxRange(r), "column \(column): caret \(c) inside hidden \(r)")
            }
        }
        // Left/right on the revealed line go one character at a time.
        try await caret(tv, at: 0)
        for expected in 1...10 {
            tv.moveRight(nil)
            XCTAssertEqual(tv.selectedRange().location, expected)
        }
    }

    func testCommentsDimWithoutRepaint() async throws {
        let (tv, co, _) = try await host("% a comment\ntext\n")
        let paints = co.syntax.paints
        EditorThemeRuntime.setCommentsDimmed(true)
        let comment = try XCTUnwrap(tv.layoutManager?.temporaryAttribute(.foregroundColor, atCharacterIndex: 2, effectiveRange: nil) as? NSColor)
        var alpha: CGFloat = 1
        NSAppearance(named: .aqua)!.performAsCurrentDrawingAppearance { alpha = comment.usingColorSpace(.sRGB)!.alphaComponent }
        XCTAssertEqual(alpha, EditorThemeRuntime.commentAlpha, accuracy: 0.01)
        EditorThemeRuntime.setCommentsDimmed(false)
        NSAppearance(named: .aqua)!.performAsCurrentDrawingAppearance { alpha = comment.usingColorSpace(.sRGB)!.alphaComponent }
        XCTAssertEqual(alpha, 1, accuracy: 0.01)
        XCTAssertEqual(co.syntax.paints, paints, "dimming is a redraw, not a repaint")
    }

    func testAccessibilityValueIsTheSource() async throws {
        let (tv, _, _) = try await host()
        try await caret(tv, at: (Self.text as NSString).range(of: "\\item").location)
        XCTAssertEqual(tv.accessibilityValue() as? String, Self.text, "VoiceOver reads the source, never the concealed display")
    }

    /// Settings > Conceal: master switch, reveal mode, one switch per class,
    /// the deny list; changes land in the preferences.
    func testConcealPaneHostsAndWritesThePreferences() throws {
        let suite = "flashtex.tests.HybridConceal.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let p = EditorPreferences(defaults: defaults)
        XCTAssertFalse(p.conceal.enabled, "off by default")
        let host = NSHostingView(rootView: HybridConcealSettingsView(preferences: p))
        host.frame = NSRect(x: 0, y: 0, width: 480, height: 1_000)
        let w = HostedWindowSupport.window(contentRect: host.frame, styleMask: [.titled])
        w.contentView = host
        window = w
        host.layoutSubtreeIfNeeded()
        RunLoop.main.run(until: Date().addingTimeInterval(0.2))
        func controls(_ v: NSView) -> [NSControl] { (v as? NSControl).map { [$0] } ?? [] + v.subviews.flatMap(controls) }
        let kinds = controls(host).map { String(describing: type(of: $0)) }
        XCTAssertEqual(kinds.filter { $0.contains("Switch") }.count, 1 + HybridConceal.Class.allCases.count, "master + one per class: \(kinds)")
        XCTAssertTrue(kinds.contains { $0.contains("TextField") }, "the deny list: \(kinds)")
        var c = p.conceal
        c.enabled = true
        c.classes.remove(.greek)
        c.denied = ["textbf"]
        p.conceal = c
        XCTAssertEqual(EditorPreferences(defaults: defaults).conceal, c, "persisted")
    }

    /// The large-document keystroke bench with conceal on (line mode) and off,
    /// in one process: typing near the start and the end of a 560 KB document
    /// (every paragraph has math, `\\textbf` and `\\emph` to conceal), and
    /// arrow-down ×100, which reveals and conceals a line per step. Thread
    /// CPU, as in `LargeDocumentEditorTests`; the machine's load is printed.
    func testLargeDocumentKeystrokesWithConcealOnAndOff() async throws {
        let text = LargeDocumentEditorTests.proseDocument(bytes: 560_000)
        let (tv, co, model) = try await host(text, size: NSSize(width: 600, height: 400))
        let ns = text as NSString
        let lm = try XCTUnwrap(tv.layoutManager)
        lm.ensureLayout(forCharacterRange: NSRange(location: 0, length: ns.length))
        func keys(at needle: String, backwards: Bool = false) async throws -> [LargeDocumentEditorTests.Sample] {
            let at = (model.activeText as NSString).range(of: needle, options: backwards ? .backwards : []).location
            try await caret(tv, at: at)
            tv.scrollRangeToVisible(tv.selectedRange())
            try await settle()
            var samples: [LargeDocumentEditorTests.Sample] = []
            for _ in 0..<20 { samples.append(LargeDocumentEditorTests.timedWithTurn { tv.insertText("z", replacementRange: tv.selectedRange()) }) }
            return samples
        }
        func arrows() async throws -> [LargeDocumentEditorTests.Sample] {
            try await caret(tv, at: 0)
            tv.scrollRangeToVisible(NSRange(location: 0, length: 0))
            try await settle()
            return (0..<100).map { _ in LargeDocumentEditorTests.timedWithTurn { tv.moveDown(nil) } }
        }
        var report = ["560 KB, conceal on (line mode), \(IMEHarness.uptime())"]
        XCTAssertTrue(co.conceal.isActive)
        let onStart = try await keys(at: "The quick brown fox")
        let onEnd = try await keys(at: "\\end{document}", backwards: true)
        let onArrows = try await arrows()
        report.append("on:  keystrokes at the start \(LargeDocumentEditorTests.stats(onStart))")
        report.append("on:  keystrokes at the end \(LargeDocumentEditorTests.stats(onEnd))")
        report.append("on:  arrow down ×100 (reveal + conceal per step) \(LargeDocumentEditorTests.stats(onArrows))")
        report.append("on:  lines computed \(co.conceal.linesComputed), glyph invalidations \(co.conceal.glyphInvalidations), characters re-laid out \(co.conceal.charactersInvalidated)")
        co.conceal.update(settings: HybridConceal.Settings(enabled: false))
        let offStart = try await keys(at: "The quick brown fox")
        let offEnd = try await keys(at: "\\end{document}", backwards: true)
        let offArrows = try await arrows()
        report.append("off: keystrokes at the start \(LargeDocumentEditorTests.stats(offStart))")
        report.append("off: keystrokes at the end \(LargeDocumentEditorTests.stats(offEnd))")
        report.append("off: arrow down ×100 \(LargeDocumentEditorTests.stats(offArrows))")
        print("hybrid-conceal bench " + report.joined(separator: "\n  "))
        XCTAssertTrue(model.activeText.sameBytes(as: tv.string))
        // Budgets on the median CPU (this machine runs other agents' builds):
        // conceal adds at most a millisecond or half again to a keystroke.
        func p50(_ s: [LargeDocumentEditorTests.Sample]) -> Double { LatencyStats(s.map(\.cpu)).p50Ms ?? 0 }
        XCTAssertLessThan(p50(onStart), p50(offStart) * 1.5 + 1, "keystroke near the start")
        XCTAssertLessThan(p50(onEnd), p50(offEnd) * 1.5 + 1, "keystroke near the end")
    }

    /// Evidence (docs/evidence/hybrid-conceal-2026-09-30/): window-ID
    /// screenshots of the hosted editor with `FLASHTEX_CONCEAL_EVIDENCE=<dir>`.
    func testWritesEvidenceScreenshotsWhenRequested() async throws {
        guard let dir = ProcessInfo.processInfo.environment["FLASHTEX_CONCEAL_EVIDENCE"] else {
            throw XCTSkip("set FLASHTEX_CONCEAL_EVIDENCE=<dir> to write the screenshots")
        }
        let sample = """
        \\section{Results}
        Let $\\alpha \\leq \\beta^2$ and $x_i \\in \\mathbb{R}$, so $\\sum_{n} a_n \\to \\infty$.
        A \\textbf{bold} claim and an \\emph{emphasised} ``quoted'' reply -- 1990--2000 --- done.
        \\begin{itemize}
        \\item first point with $\\Gamma(z) \\neq 0$
        \\item second point % a remark
        \\end{itemize}

        """
        ConcealController.settingsOverride = HybridConceal.Settings(enabled: true, reveal: .line,
                                                                    classes: HybridConceal.Settings.defaultClasses.union([.sectioning]))
        let (tv, co, _) = try await host(sample, size: NSSize(width: 820, height: 260))
        co.conceal.update(settings: ConcealController.settingsOverride!)
        let ns = sample as NSString
        func shot(_ name: String) async throws {
            try await settle()
            tv.display() // one draw: an appearance change repaints the bold strokes after it
            try await settle()
            tv.window?.displayIfNeeded()
            let path = (dir as NSString).appendingPathComponent(name)
            try? FileManager.default.removeItem(atPath: path)
            let p = Process()
            p.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
            p.arguments = ["-x", "-o", "-l", String(tv.window!.windowNumber), path]
            try p.run(); p.waitUntilExit()
            // A hosted window sits off every display (HostedWindowSupport), where
            // `screencapture -l` cannot read it ("could not create image from
            // window"); the window's own drawing is the fallback.
            print("conceal evidence \(name): screencapture -l exit \(p.terminationStatus)")
            if p.terminationStatus != 0 || !FileManager.default.fileExists(atPath: path) {
                try? FileManager.default.removeItem(atPath: path)
                let host = tv.window!.contentView!
                let rep = try XCTUnwrap(host.bitmapImageRepForCachingDisplay(in: host.bounds))
                host.cacheDisplay(in: host.bounds, to: rep)
                try rep.representation(using: .png, properties: [:])?.write(to: URL(fileURLWithPath: path))
            }
        }
        for (appearance, suffix) in [(NSAppearance.Name.aqua, "light"), (.darkAqua, "dark")] {
            tv.window?.appearance = NSAppearance(named: appearance)
            try await caret(tv, at: ns.range(of: "\\end{itemize}").location)
            try await shot("1-concealed-caret-on-last-line-\(suffix).png")
            let boldAt = ns.range(of: "bold").location
            let lm = try XCTUnwrap(tv.layoutManager)
            let shadow = try XCTUnwrap(lm.temporaryAttribute(.shadow, atCharacterIndex: boldAt, effectiveRange: nil) as? NSShadow, suffix)
            let luminance = shadow.shadowColor?.usingColorSpace(.sRGB)?.redComponent ?? -1
            XCTAssertEqual(luminance > 0.5, suffix == "dark", "the bold copy is drawn in the text colour of the appearance")
            try await caret(tv, at: ns.range(of: "\\alpha").location + 2)
            try await shot("2-caret-line-revealed-\(suffix).png")
        }
        tv.window?.appearance = NSAppearance(named: .aqua)
        co.conceal.update(settings: HybridConceal.Settings(enabled: true, reveal: .construct))
        try await caret(tv, at: NSMaxRange(ns.range(of: "\\alpha")))
        try await shot("3-construct-mode-caret-after-alpha.png")
        co.conceal.update(settings: HybridConceal.Settings(enabled: true, reveal: .line, denied: ["textbf", "phi", "leq"]))
        try await caret(tv, at: ns.range(of: "\\end{itemize}").location)
        try await shot("4-deny-textbf-and-leq.png")
        co.conceal.update(settings: HybridConceal.Settings(enabled: false))
        try await shot("5-off-raw-source.png")
        XCTAssertTrue(tv.string == sample)
    }
}
