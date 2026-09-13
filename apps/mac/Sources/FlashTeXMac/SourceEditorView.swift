import AppKit
import SwiftUI
import FlashTeXProtocol

/// NSTextView wrapper. Uses a monospaced font, reports edits, applies UTF-16
/// selections requested by preview navigation, and underlines diagnostic marks
/// with layout-manager temporary attributes (never touching the text storage,
/// so undo and the `text` binding are unaffected).
///
/// Responsiveness and accessibility rules (owner: mac-editor-accessibility):
/// - Marks are painted only over a window around the visible text
///   (`MarkPainter`), so 200 marks on a 60 KB buffer cost a fraction of a
///   millisecond per change; scrolling paints the newly exposed window.
/// - A navigation selection never fights typing: it waits while the text view
///   has marked (IME) text, and while the user is actively typing it is
///   deferred instead of moving the caret backwards. It is applied as soon as
///   typing pauses (`typingGuardNs`), and only if it is still the newest token.
/// - VoiceOver: the view is labelled, and every selection change that is not
///   a plain typing step announces "Line L, column C" (or the selection
///   extent), coalesced to one announcement per run-loop turn.
/// - A capture insertion is exactly one undo step: typing coalescing is
///   broken on both sides, the change goes through `shouldChangeText` /
///   `didChangeText`, and the model learns about it once through
///   `onEditApplied` (the binding is not written during the view update).
/// - Input methods: while the view has marked text (IME composition, dead
///   keys) nothing reaches the binding — AppKit does not post a text change
///   for `setMarkedText`, and a commit that still had marked text is held
///   back — so no revision/compile per composition step; the caret is
///   reported at the composition start (a position of the model's text);
///   composition steps are not announced; the completion list is closed and
///   its pending scan cancelled; the view is not re-synced from the model
///   until the composition ends.
struct SourceEditorView: NSViewRepresentable {
    @Binding var text: String
    var selection: ShellModel.Selection?
    var pendingEdit: ShellModel.PendingEdit?
    var marks: [EditorDiagnostics.Mark] = []
    var result: RuntimeV1.CompileResult? // for completion (Completion.swift)
    /// Editor revision the buffer is at; completion metadata binds to it.
    var editorRevision: Int?
    var projectIndexMetadata: Completion.Metadata?
    /// Project document paths for `\input{`/`\include{` completion (Completion.swift).
    var projectFiles: [String] = []
    var onCaretChange: (Int) -> Void = { _ in }
    var onSelectionChange: (NSRange) -> Void = { _ in }
    var onEditApplied: (ShellModel.PendingEdit, String) -> Void = { _, _ in }
    /// A pending edit the view could not apply (the buffer moved on since it
    /// was prepared, or its range no longer fits); never reported as applied.
    var onEditRefused: (ShellModel.PendingEdit, String) -> Void = { _, _ in }
    /// Openers typed at the caret that get their closer inserted after it
    /// (`{`, `[`, `$`). Default: braces only; the owner passes its setting.
    /// Auto-close, type-over and empty-pair backspace never run while marked
    /// text (an input-method composition) exists, and never inside a comment,
    /// a `\verb` argument or after an escaping backslash.
    ///
    /// Boundary: `\begin{env}` completed by hand (Return after the closing
    /// brace) offers nothing here — the `\end{env}` snippet belongs to the
    /// completion lane (`Completion.swift`, "\end{X} for every open \begin{X}").
    var autoClosePairs: Set<Character> = ["{"]
    /// LaTeX syntax colouring (SyntaxHighlighter.swift); off paints nothing.
    var syntaxHighlighting = true
    /// Line-number gutter with diagnostic markers (EditorIntelligence.swift).
    var showLineNumbers = true
    /// ⌘-click on a `\ref`/`\cite` key, an `\input` path or an environment
    /// name: the owner routes the target to the model's navigation
    /// (`goToMatching`, `project.openDocument`). The caret is placed on the
    /// clicked token first, so `goToMatching` sees it.
    var onDefinitionRequest: (EditorIntelligence.DefinitionTarget) -> Void = { _ in }
    /// The user's own definition of a command name for the hover peek
    /// (`ShellModel.definitionSummary`; EditorNavigation.swift).
    var userDefinition: (String) -> String? = { _ in nil }
    /// Vim mode (VimMode.swift), inert unless `VimModeFeature.isEnabled`.
    /// `onVimStatus` feeds the mode indicator; `onVimRequest` carries the
    /// commands only the model can serve (`:w`, `:q`, and `%` falling through
    /// to Go to Matching). Neither is called while the feature is off.
    var onVimStatus: (VimModeStatus) -> Void = { _ in }
    var onVimRequest: (VimRequest) -> Void = { _ in }

    /// A navigation selection that would move the caret backwards is deferred
    /// while the last user edit is younger than this.
    static let typingGuardNs: UInt64 = 350_000_000

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = CompletingTextView.scrollable() // Completion.swift
        let tv = scroll.documentView as! NSTextView
        // Marks are TextKit 1 temporary attributes. Touching `layoutManager` on
        // an empty view selects TextKit 1 now, instead of a full re-layout of a
        // large document the first time a diagnostic arrives.
        _ = tv.layoutManager
        tv.delegate = context.coordinator
        // Font, tab interval, wrapping and appearance follow EditorPreferences (applied now and on every change).
        context.coordinator.preferencesToken = EditorPreferences.shared.observeApplying(to: tv)
        context.coordinator.magnifyMonitor = EditorFontMagnifier.install(on: scroll) // pinch changes the font size (PreviewZoom.swift)
        tv.isRichText = false
        tv.isAutomaticQuoteSubstitutionEnabled = false
        tv.isAutomaticDashSubstitutionEnabled = false
        tv.isAutomaticTextReplacementEnabled = false
        tv.allowsUndo = true
        // Standard Find bar (EditorFind.swift routes Edit ▸ Find to it via
        // performTextFinderAction); incremental search highlights matches as
        // the query is typed.
        tv.usesFindBar = true
        tv.isIncrementalSearchingEnabled = true
        tv.textContainerInset = NSSize(width: 8, height: 8)
        tv.setAccessibilityLabel("LaTeX source") // FlashTeXAccessibility: VoiceOver names the editor
        tv.setAccessibilityHelp("LaTeX source editor. Moving the selection announces the line and column.")
        tv.string = text
        context.coordinator.syntax.enabled = syntaxHighlighting
        context.coordinator.syntax.attach(tv) // follows the storage from here on; paints the visible window
        context.coordinator.attach(scroll)
        context.coordinator.spelling.attach(tv) // LaTeX-aware spell checking (LaTeXSpellCheck.swift)
        context.coordinator.installIntelligence(on: scroll, lineNumbers: showLineNumbers)
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        let tv = scroll.documentView as! NSTextView
        let co = context.coordinator
        co.parent = self
        if co.syntax.enabled != syntaxHighlighting {
            co.syntax.enabled = syntaxHighlighting
            if syntaxHighlighting { co.syntax.reset() }
        }
        co.setLineNumbers(showLineNumbers, on: scroll)
        if let completing = tv as? CompletingTextView {
            // Change-only: the setter rebuilds the completion metadata, and this
            // update runs on every keystroke, not only when a result arrives.
            if completing.compileResult?.revision != result?.revision || completing.compileResult != result { completing.compileResult = result }
            if completing.editorRevision != editorRevision { completing.editorRevision = editorRevision }
        }
        if let completing = tv as? CompletingTextView, completing.projectFiles != projectFiles { completing.projectFiles = projectFiles }
        if let m = projectIndexMetadata { _ = (tv as? CompletingTextView)?.accept(projectIndex: m) }
        if let edit = pendingEdit, edit.token != co.appliedEditToken {
            // While marked text exists the storage is ahead of the model by the
            // composition: the edit's range would land inside it. Wait for the
            // commit (which updates the model and brings the next update here;
            // the edit is then refused if it was prepared for the old text).
            guard !tv.hasMarkedText() else { return }
            co.appliedEditToken = edit.token
            co.applyPendingEdit(edit, to: tv)
            return
        }
        // Until `onEditApplied` has delivered an applied edit, the binding still
        // holds the pre-edit text; resetting the view from it would undo the edit.
        // While marked text exists the storage is ahead of the model by the
        // composition; every sync waits for the commit (which updates the model
        // and brings the next update here).
        guard !co.awaitingEditDelivery, !tv.hasMarkedText() else { return }
        // `tv.string` bridges a fresh copy and compares it character by character
        // (Unicode-normalized) on every update. The coordinator keeps the exact
        // String instance last exchanged with the text view; when the binding
        // still holds that instance the comparison is a pointer check.
        var textReset = false
        if text != co.lastKnownText {
            co.programmaticChanges += 1
            tv.string = text // drops temporary attributes; repaint marks below
            co.programmaticChanges -= 1
            co.lastKnownText = text
            co.textWasReset()
            textReset = true
        }
        co.marks.update(marks, in: tv, reset: textReset)
        co.gutter?.update(marks: marks)
        co.errorLens.update(marks: marks)
        if textReset { co.refreshBraceHighlight(tv) }
        if let selection, selection.token != co.appliedToken {
            co.appliedToken = selection.token
            co.applySelection(selection, to: tv)
        }
    }

    /// Replaces underline/tooltip temporary attributes over the whole text.
    /// Ranges outside the current string are skipped; errors are applied after
    /// warnings so an error wins where they overlap.
    @MainActor
    static func applyMarks(_ marks: [EditorDiagnostics.Mark], to tv: NSTextView) {
        guard let lm = tv.layoutManager else { return }
        let whole = NSRange(location: 0, length: (tv.string as NSString).length)
        MarkPainter.paint(marks, window: whole, length: whole.length, layoutManager: lm)
    }

    // MARK: accessibility

    /// Announcement for a selection: "Line 3, column 5" for a caret, or
    /// "Selected N characters, line 3 column 5 to line 4 column 2" for a range.
    /// Lines and columns are 1-based; the column counts user-perceived
    /// characters (grapheme clusters) from the start of the line.
    /// Nil when `range` is not a valid UTF-16 range of `text`.
    static func selectionAnnouncement(text: String, range: NSRange) -> String? {
        guard range.location >= 0, range.length >= 0,
              let start = lineColumn(text: text, utf16: range.location) else { return nil }
        if range.length == 0 { return "Line \(start.line), column \(start.column)" }
        guard let end = lineColumn(text: text, utf16: NSMaxRange(range)),
              let r = Range(range, in: text) else { return nil }
        let count = text[r].count
        let chars = count == 1 ? "1 character" : "\(count) characters"
        if start.line == end.line {
            return "Selected \(chars), line \(start.line) column \(start.column) to \(end.column)"
        }
        return "Selected \(chars), line \(start.line) column \(start.column) to line \(end.line) column \(end.column)"
    }

    /// 1-based line and column for a UTF-16 offset, nil when out of range or
    /// inside a surrogate pair. `\n`, `\r` and `\r\n` each end a line (NSString
    /// line semantics for the breaks LaTeX sources contain). Newlines in the
    /// prefix are counted with `memchr` over the UTF-8 storage.
    static func lineColumn(text: String, utf16: Int) -> (line: Int, column: Int)? {
        guard let index = scalarIndex(text: text, utf16: utf16) else { return nil }
        let byte = text.utf8.distance(from: text.utf8.startIndex, to: index)
        var line = 1
        var lineStartByte = 0
        var scanned = false
        var copy = text
        copy.withUTF8 { buffer in
            guard let base = buffer.baseAddress else { return }
            scanned = true
            var p = 0
            while p < byte, let hit = memchr(base + p, 0x0A, byte - p) {
                p = UnsafePointer<UInt8>(hit.assumingMemoryBound(to: UInt8.self)) - base + 1
                line += 1; lineStartByte = p
            }
            p = 0
            while p < byte, let hit = memchr(base + p, 0x0D, byte - p) {
                p = UnsafePointer<UInt8>(hit.assumingMemoryBound(to: UInt8.self)) - base + 1
                if p < buffer.count, buffer[p] == 0x0A { continue } // CRLF: counted by the LF pass
                line += 1; lineStartByte = max(lineStartByte, p)
            }
        }
        guard scanned else { return nil }
        let lineStart = text.utf8.index(text.utf8.startIndex, offsetBy: lineStartByte)
        return (line, text[lineStart..<index].count + 1)
    }

    /// `String.Index` for a UTF-16 offset on a scalar boundary; nil when out of
    /// range or inside a surrogate pair (`Range(NSRange, in:)` would snap that).
    static func scalarIndex(text: String, utf16: Int) -> String.Index? {
        guard utf16 >= 0, utf16 <= text.utf16.count,
              let index = text.utf16.index(text.utf16.startIndex, offsetBy: utf16, limitedBy: text.utf16.endIndex),
              index.samePosition(in: text.unicodeScalars) != nil else { return nil }
        return index
    }

    /// UTF-8 byte offset of a UTF-16 caret position in `text` (nil when it is
    /// out of range or inside a surrogate pair): the contract coordinate for
    /// the caret.
    static func caretByte(text: String, utf16: Int) -> Int? {
        scalarIndex(text: text, utf16: utf16).map { text.utf8.distance(from: text.utf8.startIndex, to: $0) }
    }

    /// The text view's contents as a native (contiguous UTF-8) `String`.
    /// `tv.string` bridges the storage lazily: every later byte-wise use of
    /// it — the model's `sameBytes`, UTF-8 caret offsets, `==` — transcodes
    /// the whole NSString again (measured 2–4 ms per keystroke on a 60 KB
    /// buffer with non-ASCII text). One `getBytes` pass costs ~0.2 ms and
    /// makes everything downstream a pointer check or O(1).
    static func nativeText(of tv: NSTextView) -> String {
        let ns = (tv.textStorage?.string ?? tv.string) as NSString
        let length = ns.length
        guard length > 0 else { return "" }
        let capacity = length * 3 // a UTF-16 unit never needs more than 3 UTF-8 bytes
        var complete = false
        let native = String(unsafeUninitializedCapacity: capacity) { buffer in
            var used = 0
            var remaining = NSRange(location: 0, length: 0)
            let ok = ns.getBytes(buffer.baseAddress, maxLength: capacity, usedLength: &used,
                                 encoding: String.Encoding.utf8.rawValue, options: [],
                                 range: NSRange(location: 0, length: length), remaining: &remaining)
            complete = ok && remaining.length == 0
            return complete ? used : 0
        }
        if complete { return native }
        // Unpaired surrogates cannot be encoded; the bridge replaces them.
        var bridged = tv.string
        bridged.makeContiguousUTF8()
        return bridged
    }

    // MARK: marks

    /// Paints diagnostic marks as temporary attributes over a window around the
    /// visible text only, and extends the painted range as the view scrolls.
    /// Marks outside the window cost nothing until they scroll into view.
    @MainActor
    final class MarkPainter {
        static let keys: [NSAttributedString.Key] = [.underlineStyle, .underlineColor, .toolTip]
        /// Extra characters painted on each side of the visible range so short
        /// scrolls need no repaint.
        static let padding = 4_000

        private(set) var marks: [EditorDiagnostics.Mark] = []
        /// Disjoint, sorted ranges whose temporary attributes match `marks`
        /// (over-approximated across edits, see `noteEdit`).
        private(set) var painted: [NSRange] = []
        /// Count of passes that painted something (tests and evidence).
        private(set) var paints = 0

        /// New marks (or a text reset, which drops all temporary attributes).
        func update(_ new: [EditorDiagnostics.Mark], in tv: NSTextView, reset: Bool) {
            guard reset || new != marks else { return }
            guard let lm = tv.layoutManager else { return }
            if !reset {
                // Clear only what was painted; the layout manager kept these
                // ranges aligned with edits made since.
                let whole = NSRange(location: 0, length: tv.textStorage?.length ?? 0)
                for range in painted {
                    let stale = NSIntersectionRange(range, whole)
                    guard stale.length > 0 else { continue }
                    for key in Self.keys { lm.removeTemporaryAttribute(key, forCharacterRange: stale) }
                }
            }
            painted = []
            marks = new
            extend(to: Self.window(for: tv), in: tv, layoutManager: lm)
        }

        /// The view scrolled: paint marks over any part of the new window not yet painted.
        func scrolled(_ tv: NSTextView) {
            guard !marks.isEmpty, let lm = tv.layoutManager else { return }
            let window = Self.window(for: tv)
            guard window.length > 0, !gaps(in: window).isEmpty else { return }
            extend(to: window, in: tv, layoutManager: lm)
        }

        /// The text view is about to replace `range` with `replacementLength`
        /// UTF-16 units: grow the painted ranges so they still cover every
        /// attribute the layout manager shifts.
        func noteEdit(range: NSRange, replacementLength: Int) {
            let growth = max(0, replacementLength - range.length)
            for i in painted.indices where NSMaxRange(painted[i]) >= range.location {
                painted[i] = NSUnionRange(painted[i], NSRange(location: range.location, length: 0))
                painted[i].length += growth
            }
            painted = Self.merged(painted)
        }

        /// Parts of `window` not covered by `painted`.
        func gaps(in window: NSRange) -> [NSRange] {
            var result: [NSRange] = []
            var cursor = window.location
            for range in painted where NSMaxRange(range) > cursor {
                if range.location >= NSMaxRange(window) { break }
                if range.location > cursor { result.append(NSRange(location: cursor, length: range.location - cursor)) }
                cursor = max(cursor, NSMaxRange(range))
            }
            if cursor < NSMaxRange(window) { result.append(NSRange(location: cursor, length: NSMaxRange(window) - cursor)) }
            return result
        }

        private func extend(to window: NSRange, in tv: NSTextView, layoutManager lm: NSLayoutManager) {
            let length = tv.textStorage?.length ?? 0
            let gaps = gaps(in: window)
            if !marks.isEmpty, !gaps.isEmpty {
                // Only the uncovered parts are painted; a mark straddling a
                // boundary is clipped to each part with identical attributes.
                for gap in gaps {
                    Self.paint(marks, window: NSIntersectionRange(gap, NSRange(location: 0, length: length)),
                               length: length, layoutManager: lm)
                }
                paints += 1
            }
            painted = Self.merged(painted + [window])
        }

        /// Sorted union of `ranges` (overlapping or adjacent ranges merged).
        static func merged(_ ranges: [NSRange]) -> [NSRange] {
            var result: [NSRange] = []
            for range in ranges.sorted(by: { $0.location < $1.location }) where range.length > 0 {
                if let last = result.last, range.location <= NSMaxRange(last) {
                    result[result.count - 1] = NSUnionRange(last, range)
                } else {
                    result.append(range)
                }
            }
            return result
        }

        /// Characters in the visible rect plus padding; the whole text when
        /// the view is not laid out in a window (unit tests, offscreen views).
        static func window(for tv: NSTextView) -> NSRange {
            let length = (tv.string as NSString).length
            guard let lm = tv.layoutManager, let container = tv.textContainer, tv.window != nil,
                  !tv.visibleRect.isEmpty else { return NSRange(location: 0, length: length) }
            let glyphs = lm.glyphRange(forBoundingRect: tv.visibleRect, in: container)
            let visible = lm.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil)
            let start = max(0, visible.location - padding)
            let end = min(length, NSMaxRange(visible) + padding)
            return NSRange(location: start, length: max(0, end - start))
        }

        /// Clears this painter's keys over `window` and paints every mark that
        /// intersects it (clipped to the text). Errors are applied after
        /// warnings so an error wins where they overlap.
        static func paint(_ marks: [EditorDiagnostics.Mark], window: NSRange, length: Int, layoutManager lm: NSLayoutManager) {
            guard window.length > 0 else { return }
            for key in keys { lm.removeTemporaryAttribute(key, forCharacterRange: window) }
            let warningAttrs = attributes(for: .warning), errorAttrs = attributes(for: .error)
            // Gaps (FlashTeX does not implement this) go first so a real
            // error or warning wins over their grey underline.
            let gapAttrs: [NSAttributedString.Key: Any] = [.underlineStyle: NSUnderlineStyle.single.rawValue | NSUnderlineStyle.patternDot.rawValue,
                                                            .underlineColor: NSColor.tertiaryLabelColor]
            for severity in [RuntimeV1.Severity?.none, .warning, .error] {
                let base = severity == .error ? errorAttrs : severity == .warning ? warningAttrs : gapAttrs
                for mark in marks where (severity == nil) == EditorDiagnostics.isGap(mark.message) && (severity == nil || mark.severity == severity) {
                    let r = mark.nsRange
                    guard r.location >= 0, r.length > 0, NSMaxRange(r) <= length else { continue }
                    let clipped = NSIntersectionRange(r, window)
                    guard clipped.length > 0 else { continue }
                    var attrs = base
                    attrs[.toolTip] = mark.toolTip
                    lm.addTemporaryAttributes(attrs, forCharacterRange: clipped)
                }
            }
        }

        private static func attributes(for severity: RuntimeV1.Severity) -> [NSAttributedString.Key: Any] {
            [.underlineStyle: NSUnderlineStyle.thick.rawValue | NSUnderlineStyle.patternDot.rawValue,
             .underlineColor: severity == .error ? NSColor.systemRed : NSColor.systemOrange]
        }
    }

    // MARK: brace matching (pure)

    /// Finds the partner of the `{}`, `[]` or `$…$` delimiter next to the
    /// caret. Works on the UTF-8 bytes of the (native) text, line by line:
    /// an escaping backslash hides the next byte (`\{`, `\$`, `\\`), an
    /// unescaped `%` hides the rest of the line, and a `\verb<d>…<d>` argument
    /// is skipped. `$$` is one token that pairs only with `$$`; a `$` is an
    /// opener when an even number of `$` tokens precede it in its paragraph
    /// (back to the last blank line) and a closer otherwise; inline math never
    /// crosses a blank line. Brackets are matched by kind with depth counting,
    /// within `budgetBytes` of the anchor in the search direction.
    enum BraceMatcher {
        struct Token: Equatable { var kind: UInt8; var byte: Int; var length: Int }
        struct Line { var start: Int; var end: Int; var tokens: [Token]; var commentAt: Int?; var verb: [Range<Int>] }
        struct Match: Equatable { var open: NSRange; var close: NSRange }

        static let budgetBytes = 32_768

        /// Delimiters and their partners.
        static func closer(for opener: Character) -> Character? {
            switch opener { case "{": return "}"; case "[": return "]"; case "(": return ")"; case "$": return "$"; default: return nil }
        }
        /// `)` and `\` are closers only as the halves of an auto-inserted `\)`/`\]`
        /// (type-over checks the pending-closer list before the character).
        static func isCloser(_ c: Character) -> Bool { c == "}" || c == "]" || c == "$" || c == ")" || c == "\\" }

        /// The math closer for a `(` or `[` just typed before `caretUTF16`
        /// right after a single backslash (`\(` → `\)`, `\[` → `\]`), when
        /// that opener is code and followed by nothing or whitespace; nil otherwise.
        static func mathCloser(in text: String, caretUTF16: Int) -> String? {
            guard caretUTF16 >= 2, let index = SourceEditorView.scalarIndex(text: text, utf16: caretUTF16) else { return nil }
            let p = text.utf8.distance(from: text.utf8.startIndex, to: index)
            var copy = text
            return copy.withUTF8 { b -> String? in
                let opener = p - 1
                guard opener >= 1, b[opener - 1] == UInt8(ascii: "\\"), !escaped(b, at: opener - 1), isCode(b, at: opener - 1) else { return nil }
                let closer: String
                switch b[opener] { case UInt8(ascii: "("): closer = "\\)"; case UInt8(ascii: "["): closer = "\\]"; default: return nil }
                if p < b.count {
                    let next = b[p]
                    guard next == 0x20 || next == 0x09 || next == 0x0A || next == 0x0D else { return nil }
                }
                return closer
            }
        }

        static func match(in text: String, caretUTF16: Int) -> Match? {
            guard let index = SourceEditorView.scalarIndex(text: text, utf16: caretUTF16) else { return nil }
            let p = text.utf8.distance(from: text.utf8.startIndex, to: index)
            var copy = text
            let bytes: (open: Range<Int>, close: Range<Int>)? = copy.withUTF8 { b in match(bytes: b, caret: p) }
            guard let bytes,
                  let open = text.nsRange(utf8Bytes: .init(path: "", startByte: bytes.open.lowerBound, endByte: bytes.open.upperBound)),
                  let close = text.nsRange(utf8Bytes: .init(path: "", startByte: bytes.close.lowerBound, endByte: bytes.close.upperBound))
            else { return nil }
            return Match(open: open, close: close)
        }

        /// Whether the (ASCII) delimiter just typed before `caretUTF16` is code
        /// (not escaped, not in a comment or `\verb`) and is followed by nothing,
        /// whitespace or a closing delimiter — the cases where auto-closing helps.
        static func autoCloseAllowed(in text: String, caretUTF16: Int) -> Bool {
            guard caretUTF16 >= 1, let index = SourceEditorView.scalarIndex(text: text, utf16: caretUTF16) else { return false }
            let p = text.utf8.distance(from: text.utf8.startIndex, to: index)
            var copy = text
            return copy.withUTF8 { b in
                let opener = p - 1
                guard opener >= 0, isCode(b, at: opener), !escaped(b, at: opener) else { return false }
                if p < b.count {
                    let next = b[p]
                    let allowedNext = next == 0x20 || next == 0x09 || next == 0x0A || next == 0x0D
                        || next == UInt8(ascii: "}") || next == UInt8(ascii: "]") || next == UInt8(ascii: ")") || next == UInt8(ascii: "$")
                    guard allowedNext else { return false }
                }
                // `$` only opens new inline math: an odd count of single `$`
                // tokens earlier in the paragraph means this one closes math
                // already open there, so it must not get a second `$` paired
                // onto it (GH74: "$ not inside math already opened by $").
                if b[opener] == UInt8(ascii: "$") {
                    return dollarsBefore(b, beforeByte: opener, length: 1) % 2 == 0
                }
                return true
            }
        }

        // MARK: byte-level scanning

        static func isCode(_ b: UnsafeBufferPointer<UInt8>, at p: Int) -> Bool {
            let line = parse(b, containing: p)
            if let c = line.commentAt, p > c { return false }
            return !line.verb.contains { $0.contains(p) }
        }

        /// An odd run of backslashes ends right before `p`.
        static func escaped(_ b: UnsafeBufferPointer<UInt8>, at p: Int) -> Bool {
            var n = 0
            var i = p - 1
            while i >= 0, b[i] == UInt8(ascii: "\\") { n += 1; i -= 1 }
            return n % 2 == 1
        }

        static func lineBounds(_ b: UnsafeBufferPointer<UInt8>, containing p: Int) -> (start: Int, end: Int) {
            var start = min(p, b.count)
            while start > 0, b[start - 1] != 0x0A { start -= 1 }
            var end = min(p, b.count)
            while end < b.count, b[end] != 0x0A { end += 1 }
            return (start, end)
        }

        static func parse(_ b: UnsafeBufferPointer<UInt8>, containing p: Int) -> Line {
            let (start, end) = lineBounds(b, containing: p)
            return parse(b, start: start, end: end)
        }

        static func parse(_ b: UnsafeBufferPointer<UInt8>, start: Int, end: Int) -> Line {
            var line = Line(start: start, end: end, tokens: [], commentAt: nil, verb: [])
            var i = start
            while i < end {
                let c = b[i]
                if c == UInt8(ascii: "\\") {
                    // \verb<d>…<d> (also \verb*): the argument is not code.
                    if i + 5 < end, b[i + 1] == UInt8(ascii: "v"), b[i + 2] == UInt8(ascii: "e"), b[i + 3] == UInt8(ascii: "r"), b[i + 4] == UInt8(ascii: "b") {
                        var j = i + 5
                        if j < end, b[j] == UInt8(ascii: "*") { j += 1 }
                        if j < end, !isLetter(b[j]) {
                            let d = b[j]
                            var k = j + 1
                            while k < end, b[k] != d { k += 1 }
                            line.verb.append(i..<min(k + 1, end))
                            i = min(k + 1, end)
                            continue
                        }
                    }
                    i += 2 // the escaped byte is literal (\{ \} \$ \% \\)
                    continue
                }
                if c == UInt8(ascii: "%") { line.commentAt = i; break }
                if c == UInt8(ascii: "$") {
                    if i + 1 < end, b[i + 1] == UInt8(ascii: "$") { line.tokens.append(Token(kind: c, byte: i, length: 2)); i += 2 }
                    else { line.tokens.append(Token(kind: c, byte: i, length: 1)); i += 1 }
                    continue
                }
                if c == UInt8(ascii: "{") || c == UInt8(ascii: "}") || c == UInt8(ascii: "[") || c == UInt8(ascii: "]") {
                    line.tokens.append(Token(kind: c, byte: i, length: 1))
                }
                i += 1
            }
            return line
        }

        static func isLetter(_ c: UInt8) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
        static func isBlank(_ b: UnsafeBufferPointer<UInt8>, _ line: Line) -> Bool {
            var i = line.start
            while i < line.end { if b[i] != 0x20 && b[i] != 0x09 && b[i] != 0x0D { return false }; i += 1 }
            return true
        }

        static func match(bytes b: UnsafeBufferPointer<UInt8>, caret p: Int) -> (open: Range<Int>, close: Range<Int>)? {
            let line = parse(b, containing: p)
            if let c = line.commentAt, p > c { return nil }
            if line.verb.contains(where: { $0.contains(p) }) { return nil }
            guard let anchorIndex = line.tokens.firstIndex(where: { $0.byte + $0.length == p })
                    ?? line.tokens.firstIndex(where: { $0.byte == p }) else { return nil }
            let anchor = line.tokens[anchorIndex]
            let range = anchor.byte..<(anchor.byte + anchor.length)
            switch anchor.kind {
            case UInt8(ascii: "{"), UInt8(ascii: "["):
                let close = anchor.kind == UInt8(ascii: "{") ? UInt8(ascii: "}") : UInt8(ascii: "]")
                return search(b, from: line, tokenIndex: anchorIndex, forward: true, open: anchor.kind, close: close, math: nil)
                    .map { (range, $0) }
            case UInt8(ascii: "}"), UInt8(ascii: "]"):
                let open = anchor.kind == UInt8(ascii: "}") ? UInt8(ascii: "{") : UInt8(ascii: "[")
                return search(b, from: line, tokenIndex: anchorIndex, forward: false, open: open, close: anchor.kind, math: nil)
                    .map { ($0, range) }
            default: // $ or $$
                let forward = dollarsBefore(b, line: line, tokenIndex: anchorIndex, length: anchor.length) % 2 == 0
                guard let partner = search(b, from: line, tokenIndex: anchorIndex, forward: forward, open: anchor.kind, close: anchor.kind, math: anchor.length) else { return nil }
                return forward ? (range, partner) : (partner, range)
            }
        }

        /// `$` tokens of `length` before the anchor in its paragraph.
        static func dollarsBefore(_ b: UnsafeBufferPointer<UInt8>, line: Line, tokenIndex: Int, length: Int) -> Int {
            var count = line.tokens[..<tokenIndex].filter { $0.kind == UInt8(ascii: "$") && $0.length == length }.count
            var current = line
            var scanned = 0
            while current.start > 0, scanned < budgetBytes {
                let previous = parse(b, containing: current.start - 1)
                if isBlank(b, previous) { break }
                count += previous.tokens.filter { $0.kind == UInt8(ascii: "$") && $0.length == length }.count
                scanned += current.start - previous.start
                current = previous
            }
            return count
        }

        /// Single (`length` 1) `$` tokens strictly before byte offset `p` in
        /// `p`'s paragraph (back to the last blank line), bounded like the
        /// token-index overload above. Even means a `$` typed at `p` would
        /// open new inline math; odd means it closes math already open
        /// earlier in the paragraph (`autoCloseAllowed`'s `$` gate).
        static func dollarsBefore(_ b: UnsafeBufferPointer<UInt8>, beforeByte p: Int, length: Int) -> Int {
            let line = parse(b, containing: p)
            var count = line.tokens.filter { $0.kind == UInt8(ascii: "$") && $0.length == length && $0.byte < p }.count
            var current = line
            var scanned = 0
            while current.start > 0, scanned < budgetBytes {
                let previous = parse(b, containing: current.start - 1)
                if isBlank(b, previous) { break }
                count += previous.tokens.filter { $0.kind == UInt8(ascii: "$") && $0.length == length }.count
                scanned += current.start - previous.start
                current = previous
            }
            return count
        }

        /// Depth-counted search for the partner token; `math` is the `$` token
        /// length to pair (no nesting, stops at a blank line).
        static func search(_ b: UnsafeBufferPointer<UInt8>, from line: Line, tokenIndex: Int, forward: Bool,
                           open: UInt8, close: UInt8, math: Int?) -> Range<Int>? {
            var depth = 1
            var current = line
            var tokens = forward ? Array(line.tokens[(tokenIndex + 1)...]) : Array(line.tokens[..<tokenIndex].reversed())
            var scanned = 0
            while true {
                for t in tokens {
                    if let math {
                        if t.kind == UInt8(ascii: "$"), t.length == math { return t.byte..<(t.byte + t.length) }
                        continue
                    }
                    if t.kind == (forward ? open : close) { depth += 1 }
                    else if t.kind == (forward ? close : open) {
                        depth -= 1
                        if depth == 0 { return t.byte..<(t.byte + t.length) }
                    }
                }
                if forward {
                    guard current.end < b.count, scanned < budgetBytes else { return nil }
                    let next = parse(b, containing: current.end + 1)
                    scanned += next.end - current.end
                    current = next
                } else {
                    guard current.start > 0, scanned < budgetBytes else { return nil }
                    let previous = parse(b, containing: current.start - 1)
                    scanned += current.start - previous.start
                    current = previous
                }
                if math != nil, isBlank(b, current) { return nil }
                tokens = forward ? current.tokens : current.tokens.reversed()
            }
        }
    }

    // MARK: coordinator

    @MainActor
    final class Coordinator: NSObject, NSTextViewDelegate {
        var parent: SourceEditorView
        var appliedToken: Int
        var appliedEditToken = 0
        /// Keeps EditorPreferences applied to the text view (EditorPreferences.swift).
        var preferencesToken: EditorPreferences.ObservationToken?
        var magnifyMonitor: Any? { didSet { if let old = oldValue { NSEvent.removeMonitor(old) } } }
        let marks = MarkPainter()
        /// Syntax colours as temporary attributes (SyntaxHighlighter.swift).
        let syntax = SyntaxPainter()
        /// Prose-only spelling underlines and right-click suggestions (LaTeXSpellCheck.swift).
        let spelling = LaTeXSpellChecker()
        /// Line numbers + diagnostic markers (nil while hidden).
        private(set) var gutter: LineNumberGutter?
        /// Hover quick-info popover.
        let hover = HoverController()
        /// Inline diagnostic text at line ends (ErrorLens.swift).
        let errorLens = ErrorLensPainter()
        /// Index of the caret's line, for the current-line band and the gutter.
        private(set) var currentLine: Int?
        /// Definition targets routed to the owner (evidence for tests).
        private(set) var definitionRequests: [EditorIntelligence.DefinitionTarget] = []
        /// Vim mode's state machine (VimMode.swift); never touched while the
        /// feature is off — `handleVimKey` returns before reaching it.
        let vim = VimMachine()
        /// Last status pushed to `parent.onVimStatus` (only changes are sent).
        var lastVimStatus = VimModeStatus.off
        /// The String instance last set on, or read from, the text view.
        var lastKnownText: String
        /// > 0 while this coordinator itself edits the text view (string reset,
        /// pending edit, navigation selection); the delegate then neither writes
        /// the binding nor announces.
        var programmaticChanges = 0
        /// A pending edit was applied to the view but `onEditApplied` has not run yet.
        private(set) var awaitingEditDelivery = false
        /// Monotonic time of the last text change the user made (0 = never).
        private(set) var lastUserEditNs: UInt64 = 0
        /// Thread CPU time at the last user text change (benchmark evidence:
        /// the delegate -> binding round trip net of preemption).
        private(set) var lastUserEditCpuNs: UInt64 = 0
        /// Navigation selection waiting for a typing pause or the end of an IME composition.
        private(set) var deferredSelection: ShellModel.Selection?
        private var deferredTimer: Timer?
        /// True from a text change until the end of the run-loop turn: selection
        /// changes in that turn are typing steps, not caret moves.
        private var textChangedThisTurn = false
        private var announcementPending = false
        /// True while the view has marked text (an IME composition or dead key).
        var composing: Bool { textView?.hasMarkedText() ?? false }
        /// Composition selection changes observed (tests and evidence).
        private(set) var compositionSteps = 0
        /// VoiceOver sink; tests replace it to observe announcements.
        var announce: (String) -> Void = { _ in }
        /// Delimiter pair highlighted around the caret (temporary background).
        private(set) var braceHighlight: BraceMatcher.Match?
        /// UTF-16 offsets of auto-inserted closers not yet typed over or edited
        /// away; kept aligned with edits by `shouldChangeTextIn`.
        private(set) var pendingClosers: [Int] = []
        /// Registers `offset` the same way `autoClose(after:)` does for a
        /// hand-typed opener's closer (EditorKeyHandling.swift's hook from
        /// Completion.swift's snippet insertion).
        func registerPendingCloser(_ offset: Int) { pendingClosers.append(offset) }
        /// The user edit AppKit is applying (from `shouldChangeTextIn` to `textDidChange`).
        private var lastEdit: (range: NSRange, replacement: String)?
        /// True while the coordinator inserts a closer or deletes a pair itself.
        private var pairing = false
        /// Marked text was seen since the last committed text change: that
        /// change came from an input method, never auto-closed.
        private var commitFromComposition = false
        static let highlightKey = NSAttributedString.Key.backgroundColor
        static let highlightColor = NSColor.selectedTextBackgroundColor.withAlphaComponent(0.45)
        /// Announcements posted (tests and evidence).
        private(set) var announcements: [String] = []
        private weak var scrollView: NSScrollView?
        private var boundsObserver: NSObjectProtocol?

        init(_ parent: SourceEditorView) {
            self.parent = parent
            lastKnownText = parent.text
            // A recreated view must not replay the last navigation (that would
            // move the caret back to an old target); a pending edit is applied
            // because the model is still waiting for `onEditApplied`.
            appliedToken = parent.selection?.token ?? 0
            super.init()
            announce = { [weak self] message in self?.post(message) }
        }

        deinit {
            if let boundsObserver { NotificationCenter.default.removeObserver(boundsObserver) }
            if let magnifyMonitor { NSEvent.removeMonitor(magnifyMonitor) }
            deferredTimer?.invalidate()
        }

        func attach(_ scroll: NSScrollView) {
            scrollView = scroll
            scroll.contentView.postsBoundsChangedNotifications = true
            boundsObserver = NotificationCenter.default.addObserver(
                forName: NSView.boundsDidChangeNotification, object: scroll.contentView, queue: nil
            ) { [weak self, weak scroll] _ in
                MainActor.assumeIsolated {
                    guard let self, let tv = scroll?.documentView as? NSTextView else { return }
                    self.marks.scrolled(tv)
                    self.syntax.scrolled()
                    self.hover.dismiss()
                    self.gutter?.needsDisplay = true
                }
            }
        }

        var textView: NSTextView? { scrollView?.documentView as? NSTextView }

        // MARK: editor intelligence (EditorIntelligence.swift)

        func installIntelligence(on scroll: NSScrollView, lineNumbers: Bool) {
            guard let tv = scroll.documentView as? NSTextView else { return }
            hover.install(on: tv)
            hover.info = { [weak self] index in self?.quickInfo(at: index) }
            if let completing = tv as? CompletingTextView {
                completing.commandClickHandler = { [weak self] index in self?.commandClick(at: index) ?? false }
                completing.backgroundDecorator = { [weak self] rect in self?.drawCurrentLine(in: rect) }
                // GH74: a completion snippet's placeholder closer (`\section{}`)
                // overtypes like a hand-typed `{` instead of doubling
                // (EditorKeyHandling.swift computes the offset; Completion.swift
                // calls this hook once, right after it places the caret).
                completing.onCloserInserted = { [weak self] offset in self?.registerPendingCloser(offset) }
            }
            errorLens.lineTable = { [weak self] in self?.syntax.highlighter ?? SyntaxHighlighter() }
            errorLens.attach(tv)
            errorLens.update(marks: parent.marks)
            setLineNumbers(lineNumbers, on: scroll)
            updateCurrentLine(tv)
        }

        func setLineNumbers(_ on: Bool, on scroll: NSScrollView) {
            if on, gutter == nil {
                let g = LineNumberGutter(scrollView: scroll)
                g.lineTable = { [weak self] in self?.syntax.highlighter ?? SyntaxHighlighter() }
                scroll.verticalRulerView = g
                scroll.hasVerticalRuler = true
                scroll.rulersVisible = true
                gutter = g
                g.layoutIfNeeded(lineCount: syntax.highlighter.lineCount)
                g.update(marks: parent.marks)
                g.currentLine = currentLine
            } else if !on, gutter != nil {
                scroll.rulersVisible = false
                scroll.hasVerticalRuler = false
                scroll.verticalRulerView = nil
                gutter = nil
            }
        }

        /// Hover data for a character index: token documentation plus the diagnostics there.
        func quickInfo(at index: Int) -> EditorIntelligence.QuickInfo? {
            guard let tv = textView else { return nil }
            let text = tv.textStorage?.string as NSString? ?? ""
            let h = syntax.highlighter.length == text.length ? syntax.highlighter : nil
            return EditorIntelligence.quickInfo(in: text, at: index, marks: marks.marks, highlighter: h, userDefinition: parent.userDefinition)
        }

        /// ⌘-click: place the caret on the token and hand its target to the owner.
        @discardableResult
        func commandClick(at index: Int) -> Bool {
            guard let tv = textView else { return false }
            let text = tv.textStorage?.string as NSString? ?? ""
            let h = syntax.highlighter.length == text.length ? syntax.highlighter : nil
            guard let target = EditorIntelligence.definitionTarget(in: text, at: index, highlighter: h) else { return false }
            hover.dismiss()
            tv.setSelectedRange(NSRange(location: index, length: 0)) // onCaretChange → model.caretUTF16
            definitionRequests.append(target)
            if definitionRequests.count > 32 { definitionRequests.removeFirst(definitionRequests.count - 32) }
            parent.onDefinitionRequest(target)
            return true
        }

        /// Return: auto-indent, one level deeper after `\begin{env}`, closing
        /// it with `\end{env}` when brace auto-closing is on. One typing-
        /// coalesced insertion through `insertText` (undo removes it whole).
        func insertNewline(in tv: NSTextView) -> Bool {
            guard programmaticChanges == 0, !tv.hasMarkedText() else { return false }
            let sel = tv.selectedRange()
            guard sel.length == 0 else { return false }
            let text = tv.textStorage?.string as NSString? ?? ""
            let insertion = EditorIntelligence.newline(in: text, caret: sel.location, indentUnit: EditorPreferences.shared.indentString,
                                                       closeEnvironments: parent.autoClosePairs.contains("{"))
            guard insertion.text != "\n" else { return false } // plain Return: AppKit's own path
            tv.insertText(insertion.text, replacementRange: sel)
            tv.setSelectedRange(NSRange(location: sel.location + insertion.caretOffset, length: 0))
            return true
        }

        func updateCurrentLine(_ tv: NSTextView) {
            let caret = tv.selectedRange().location
            let table = syntax.highlighter
            let line = table.length == (tv.textStorage?.length ?? 0) ? table.line(at: caret) : nil
            guard line != currentLine else { return }
            let old = currentLine
            currentLine = line
            gutter?.currentLine = line
            for l in [old, line].compactMap({ $0 }) { tv.setNeedsDisplay(currentLineRect(l, in: tv)) }
        }

        private func currentLineRect(_ line: Int, in tv: NSTextView) -> NSRect {
            guard let lm = tv.layoutManager else { return tv.bounds }
            let table = syntax.highlighter
            guard line < table.lineCount, table.length == (tv.textStorage?.length ?? 0) else { return .zero }
            let r = table.lineRange(line)
            let rect: NSRect
            if r.length == 0 || r.location >= table.length {
                rect = lm.extraLineFragmentRect
            } else {
                let glyphs = lm.glyphRange(forCharacterRange: r, actualCharacterRange: nil)
                var union = NSRect.null
                lm.enumerateLineFragments(forGlyphRange: glyphs) { fragment, _, _, _, _ in union = union.union(fragment) }
                rect = union.isNull ? .zero : union
            }
            var band = rect.offsetBy(dx: 0, dy: tv.textContainerInset.height)
            band.origin.x = 0
            band.size.width = tv.bounds.width
            return band
        }

        /// The current-line band, drawn under the text (only when no selection).
        func drawCurrentLine(in rect: NSRect) {
            guard let tv = textView, let line = currentLine, tv.selectedRange().length == 0,
                  tv.window?.firstResponder === tv else { return }
            let band = currentLineRect(line, in: tv)
            guard !band.isEmpty, band.intersects(rect) else { return }
            SyntaxTheme.currentLine.setFill()
            band.fill()
        }

        // MARK: pending edit (one undo step)

        func applyPendingEdit(_ edit: ShellModel.PendingEdit, to tv: NSTextView) {
            let ns = edit.nsRange
            // The range was computed for one editor revision: a buffer that
            // moved on since (an IME commit, a keystroke) makes it meaningless.
            // Refuse explicitly rather than insert at a shifted offset or report
            // an unapplied edit as applied.
            let refuse: (String) -> Void = { [weak self] reason in
                let onEditRefused = self?.parent.onEditRefused ?? { _, _ in }
                DispatchQueue.main.async { onEditRefused(edit, reason) }
            }
            if let prepared = edit.revision, let current = parent.editorRevision, prepared != current {
                refuse("the document changed since the edit was prepared (revision \(prepared), now \(current))")
                return
            }
            guard ns.location >= 0, NSMaxRange(ns) <= (tv.textStorage?.length ?? 0) else {
                refuse("range \(ns.location)..<\(NSMaxRange(ns)) is outside the buffer (\(tv.textStorage?.length ?? 0) UTF-16 units)")
                return
            }
            var applied = false
            do {
                tv.breakUndoCoalescing() // preceding typing stays its own undo step
                if tv.shouldChangeText(in: ns, replacementString: edit.text) {
                    programmaticChanges += 1
                    tv.textStorage?.replaceCharacters(in: ns, with: edit.text)
                    tv.didChangeText() // registers undo, fires textDidChange
                    tv.undoManager?.setActionName("Insert Capture")
                    tv.breakUndoCoalescing() // following typing starts a new step
                    let inserted = NSRange(location: ns.location, length: (edit.text as NSString).length)
                    tv.setSelectedRange(inserted)
                    tv.scrollRangeToVisible(inserted)
                    tv.showFindIndicator(for: inserted)
                    tv.window?.makeFirstResponder(tv)
                    programmaticChanges -= 1
                    applied = true
                }
            }
            guard applied else { refuse("the text view declined the change"); return }
            let s = SourceEditorView.nativeText(of: tv)
            lastKnownText = s
            pendingClosers = []
            refreshBraceHighlight(tv)
            announceNow(text: s, range: tv.selectedRange(), prefix: "Inserted capture. ")
            // The model is updated outside the SwiftUI view update; `editApplied`
            // bumps the revision and reaches the bridge/ledger through
            // `updateActiveText` exactly once.
            awaitingEditDelivery = true
            let onEditApplied = parent.onEditApplied
            DispatchQueue.main.async { [weak self] in
                self?.awaitingEditDelivery = false
                onEditApplied(edit, s)
            }
        }

        // MARK: navigation selection

        func applySelection(_ selection: ShellModel.Selection, to tv: NSTextView) {
            let range = selection.nsRange
            guard range.location >= 0, range.length >= 0, NSMaxRange(range) <= (tv.textStorage?.length ?? 0) else {
                deferredSelection = nil; return // no longer a range of the buffer
            }
            if tv.hasMarkedText() {
                deferSelection(selection, for: 0.1); return
            }
            let sinceEdit = MonotonicClock.nowNs() &- lastUserEditNs
            if lastUserEditNs != 0, sinceEdit < SourceEditorView.typingGuardNs,
               range.location < tv.selectedRange().location {
                deferSelection(selection, for: Double(SourceEditorView.typingGuardNs &- sinceEdit) / 1e9); return
            }
            deferredSelection = nil
            deferredTimer?.invalidate()
            programmaticChanges += 1
            tv.setSelectedRange(range)
            tv.scrollRangeToVisible(range) // scrolls only when the range is off screen
            tv.showFindIndicator(for: range)
            tv.window?.makeFirstResponder(tv)
            programmaticChanges -= 1
            announceNow(text: currentText(of: tv), range: range, prefix: "", suffix: matchSuffix(in: tv))
        }

        private func deferSelection(_ selection: ShellModel.Selection, for seconds: TimeInterval) {
            deferredSelection = selection
            deferredTimer?.invalidate()
            let timer = Timer(timeInterval: max(0.01, seconds), repeats: false) { [weak self] _ in
                MainActor.assumeIsolated { self?.retryDeferredSelection() }
            }
            RunLoop.main.add(timer, forMode: .common)
            deferredTimer = timer
        }

        private func retryDeferredSelection() {
            guard let selection = deferredSelection, parent.selection?.token == selection.token,
                  let tv = textView else { deferredSelection = nil; return }
            applySelection(selection, to: tv)
        }

        // MARK: delegate

        func textView(_ textView: NSTextView, shouldChangeTextIn range: NSRange, replacementString: String?) -> Bool {
            let replacementLength = (replacementString as NSString?)?.length ?? 0
            marks.noteEdit(range: range, replacementLength: replacementLength)
            // Type-over: the closer the user types is the one that was auto-inserted here.
            if !pairing, programmaticChanges == 0, let replacementString, range.length == 0, replacementString.count == 1,
               let ch = replacementString.first, BraceMatcher.isCloser(ch), !textView.hasMarkedText(),
               let i = pendingClosers.firstIndex(of: range.location),
               (textView.textStorage?.length ?? 0) > range.location,
               (textView.string as NSString).substring(with: NSRange(location: range.location, length: 1)) == replacementString {
                pendingClosers.remove(at: i)
                noteTypingStep()
                textView.setSelectedRange(NSRange(location: range.location + 1, length: 0))
                announceMatch(in: textView)
                return false // nothing changes: the caret stepped over the closer
            }
            shiftPendingClosers(edit: range, replacementLength: replacementLength)
            if !pairing, programmaticChanges == 0 {
                lastEdit = replacementString.map { (range, $0) }
                noteTypingStep() // the selection change AppKit posts before textDidChange is a typing step: no highlight refresh, no announcement
            }
            return true
        }

        /// Backspace between an auto-closed pair removes both characters;
        /// Return auto-indents (EditorIntelligence.swift).
        func textView(_ textView: NSTextView, doCommandBy commandSelector: Selector) -> Bool {
            if commandSelector == #selector(NSResponder.insertNewline(_:)) { return insertNewline(in: textView) }
            // Tab / Shift-Tab (EditorKeyHandling.swift). When the completion
            // popup is open, `CompletingTextView.keyDown` intercepts Tab itself
            // (moves the list selection) and never calls through to here.
            if commandSelector == #selector(NSResponder.insertTab(_:)) { return handleTab(reverse: false, in: textView) }
            if commandSelector == #selector(NSResponder.insertBacktab(_:)) { return handleTab(reverse: true, in: textView) }
            guard commandSelector == #selector(NSResponder.deleteBackward(_:)), !pairing, programmaticChanges == 0,
                  !textView.hasMarkedText() else { return false }
            let caret = textView.selectedRange()
            guard caret.length == 0, caret.location >= 1, pendingClosers.contains(caret.location),
                  (textView.textStorage?.length ?? 0) > caret.location else { return false }
            let pair = (textView.string as NSString).substring(with: NSRange(location: caret.location - 1, length: 2))
            guard pair.count == 2, let opener = pair.first, BraceMatcher.closer(for: opener) == pair.last else { return false }
            // Its own undo step: without breaking coalescing AppKit folds a
            // programmatic range deletion into the open typing group and undoes
            // more than the pair (observed: the preceding text vanished too).
            pairing = true
            textView.breakUndoCoalescing()
            textView.insertText("", replacementRange: NSRange(location: caret.location - 1, length: 2))
            textView.breakUndoCoalescing()
            pairing = false
            lastEdit = nil
            commitUserChange(textView, edit: nil)
            return true
        }

        private func shiftPendingClosers(edit range: NSRange, replacementLength: Int) {
            guard !pendingClosers.isEmpty else { return }
            let delta = replacementLength - range.length
            pendingClosers = pendingClosers.compactMap { closer in
                if NSMaxRange(range) <= closer { return closer + delta } // edit before it: shifts
                if range.location > closer { return closer } // edit after it: unchanged
                return nil // overlapped: the closer is gone
            }
        }

        private func noteTypingStep() {
            lastUserEditNs = MonotonicClock.nowNs()
            if !textChangedThisTurn {
                textChangedThisTurn = true
                DispatchQueue.main.async { [weak self] in self?.textChangedThisTurn = false }
            }
        }

        func textDidChange(_ notification: Notification) {
            guard let tv = notification.object as? NSTextView else { return }
            TypingBench.shared.textViewDidChange() // stamps the delegate time for keystroke -> paint
            syntax.flush() // the storage notification updated the line model; colours the changed lines now (deferred while composing)
            hover.dismiss()
            gutter?.layoutIfNeeded(lineCount: syntax.highlighter.lineCount)
            gutter?.needsDisplay = true
            if !textChangedThisTurn {
                textChangedThisTurn = true
                DispatchQueue.main.async { [weak self] in self?.textChangedThisTurn = false }
            }
            guard programmaticChanges == 0, !pairing else { return }
            let edit = lastEdit
            lastEdit = nil
            commitUserChange(tv, edit: edit)
        }

        /// A user edit is in the storage: auto-close, push the buffer to the
        /// binding once, move the delimiter highlight, announce a typed closer.
        private func commitUserChange(_ tv: NSTextView, edit: (range: NSRange, replacement: String)?) {
            lastUserEditNs = MonotonicClock.nowNs()
            lastUserEditCpuNs = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            // A change that leaves marked text behind is a composition step:
            // the model sees the buffer once the composition is committed.
            guard !tv.hasMarkedText() else { return }
            if commitFromComposition { commitFromComposition = false } else { autoClose(after: edit, in: tv) }
            let s = SourceEditorView.nativeText(of: tv)
            lastKnownText = s
            parent.text = s
            refreshBraceHighlight(tv)
            if let edit, edit.range.length == 0, edit.replacement.count == 1, let ch = edit.replacement.first, BraceMatcher.isCloser(ch) {
                announceMatch(in: tv)
            }
        }

        func textViewDidChangeSelection(_ notification: Notification) {
            guard let tv = notification.object as? NSTextView else { return }
            if tv.hasMarkedText() { compositionStep(tv); return }
            let range = tv.selectedRange()
            parent.onCaretChange(range.location)
            parent.onSelectionChange(range)
            updateCurrentLine(tv)
            if !textChangedThisTurn { refreshBraceHighlight(tv) } // a typing turn refreshes from textDidChange
            // A typing step already reads as typed text in VoiceOver; only
            // caret/selection moves are announced, once per run-loop turn.
            // (`textChangedThisTurn` is checked again when the turn ends because
            // AppKit may post the selection change before the text change.)
            guard programmaticChanges == 0, !textChangedThisTurn, !announcementPending else { return }
            announcementPending = true
            DispatchQueue.main.async { [weak self] in
                guard let self else { return }
                announcementPending = false
                guard !textChangedThisTurn, let tv = textView, tv.window?.firstResponder === tv else { return }
                announceNow(text: currentText(of: tv), range: tv.selectedRange(), prefix: "", suffix: matchSuffix(in: tv))
            }
        }

        // MARK: delimiter pairs

        /// Inserts the closer after an opener the user just typed (when the
        /// owner enabled it for that opener) and puts the caret between them.
        /// The closer is inserted through `insertText`, so it coalesces with
        /// the opener into one typing undo step.
        private func autoClose(after edit: (range: NSRange, replacement: String)?, in tv: NSTextView) {
            guard let edit, edit.range.length == 0, edit.replacement.count == 1, let opener = edit.replacement.first else { return }
            let caret = NSRange(location: edit.range.location + (edit.replacement as NSString).length, length: 0)
            guard tv.selectedRange() == caret else { return }
            // `\(` → `\)`, `\[` → `\]` (owner-enabled by `(`): both halves of the closer are typed over.
            if opener == "(" || opener == "[", parent.autoClosePairs.contains("("),
               let math = BraceMatcher.mathCloser(in: SourceEditorView.nativeText(of: tv), caretUTF16: caret.location) {
                pairing = true
                tv.insertText(math, replacementRange: caret)
                tv.setSelectedRange(caret)
                pairing = false
                pendingClosers += [caret.location, caret.location + 1]
                return
            }
            guard parent.autoClosePairs.contains(opener), let closer = BraceMatcher.closer(for: opener) else { return }
            guard BraceMatcher.autoCloseAllowed(in: SourceEditorView.nativeText(of: tv), caretUTF16: caret.location) else { return }
            pairing = true
            tv.insertText(String(closer), replacementRange: caret)
            tv.setSelectedRange(caret)
            pairing = false
            pendingClosers.append(caret.location)
        }

        func textWasReset() {
            braceHighlight = nil // the reset dropped every temporary attribute
            pendingClosers = []
            syntax.reset()
            hover.dismiss()
            gutter?.layoutIfNeeded(lineCount: syntax.highlighter.lineCount)
            gutter?.needsDisplay = true
        }

        /// Recomputes the pair around the caret and moves the highlight.
        func refreshBraceHighlight(_ tv: NSTextView) {
            guard let lm = tv.layoutManager else { return }
            let caret = tv.selectedRange()
            // O(1) look at the storage first: the native-text conversion and the
            // UTF-16 breadcrumbs behind `match` are O(n) per fresh buffer (1.2 +
            // 1.3 ms at 560 KB, LargeDocumentEditorTests) and a prose keystroke
            // is almost never next to a delimiter.
            var new = caret.length == 0 && !tv.hasMarkedText()
                && BraceMatcher.delimiterAdjacent(in: tv.textStorage, caretUTF16: caret.location)
                ? BraceMatcher.match(in: currentText(of: tv), caretUTF16: caret.location) : nil
            // `\begin{X}` ↔ `\end{X}` pair (EditorNavigation.swift), only when the
            // caret's line has one (the pair scan is linear in the buffer).
            if new == nil, caret.length == 0, !tv.hasMarkedText(), let pair = environmentPair(at: caret.location, in: tv), let end = pair.end {
                new = BraceMatcher.Match(open: pair.begin, close: end)
            }
            guard new != braceHighlight else { return }
            let length = tv.textStorage?.length ?? 0
            if let old = braceHighlight {
                for r in [old.open, old.close] where NSMaxRange(r) <= length {
                    lm.removeTemporaryAttribute(Self.highlightKey, forCharacterRange: r)
                }
            }
            if let new {
                for r in [new.open, new.close] where NSMaxRange(r) <= length {
                    lm.addTemporaryAttribute(Self.highlightKey, value: Self.highlightColor, forCharacterRange: r)
                }
            }
            braceHighlight = new
        }

        /// The environment pair whose `\begin`/`\end` the caret is on, or nil
        /// (also when the caret's line has no `\begin{`/`\end{`, without a scan).
        func environmentPair(at caret: Int, in tv: NSTextView) -> EditorNavigation.EnvironmentPair? {
            let table = syntax.highlighter
            guard let storage = tv.textStorage, table.length == storage.length, caret <= table.length else { return nil }
            let line = table.lineRange(table.line(at: caret))
            let lineText = (storage.string as NSString).substring(with: line)
            guard lineText.contains("\\begin{") || lineText.contains("\\end{") || lineText.contains("\\begin {") || lineText.contains("\\end {") else { return nil }
            return EditorNavigation.environmentPair(at: caret, in: currentText(of: tv) as NSString)
        }

        /// ", matches line L column C" for the delimiter partner farthest from the caret.
        func matchSuffix(in tv: NSTextView) -> String {
            guard let h = braceHighlight else { return "" }
            let caret = tv.selectedRange().location
            let adjacentToClose = NSLocationInRange(caret, h.close) || NSMaxRange(h.close) == caret // inside `\end{…}` counts too
            let partner = adjacentToClose ? h.open : h.close
            guard let lc = SourceEditorView.lineColumn(text: currentText(of: tv), utf16: partner.location) else { return "" }
            return ", matches line \(lc.line) column \(lc.column)"
        }

        /// A closer was typed (or typed over): say where its opener is.
        private func announceMatch(in tv: NSTextView) {
            refreshBraceHighlight(tv)
            let suffix = matchSuffix(in: tv)
            guard !suffix.isEmpty else { return }
            let message = String(suffix.dropFirst(2)) // "matches line L column C"
            announcements.append(message)
            if announcements.count > 64 { announcements.removeFirst(announcements.count - 64) }
            announce(message)
        }

        // MARK: input method composition

        /// `setMarkedText` posts no text change, only selection changes. The
        /// caret the model hears is the composition start — a position of the
        /// text it holds — nothing is announced, and the completion list (whose
        /// key path re-scans after every keystroke) is closed with its pending
        /// scan cancelled at the head of the next run-loop turn, i.e. after the
        /// keystroke that started the step has enqueued that scan.
        private func compositionStep(_ tv: NSTextView) {
            compositionSteps += 1
            commitFromComposition = true
            lastUserEditNs = MonotonicClock.nowNs() // composing is typing for the navigation guard
            let start = tv.markedRange().location
            if start != NSNotFound {
                parent.onCaretChange(start)
                parent.onSelectionChange(NSRange(location: start, length: 0))
            }
            guard let completing = tv as? CompletingTextView else { return }
            TypingBench.nextRunLoopTurn { [weak completing] in
                guard let completing, completing.hasMarkedText() else { return }
                completing.scheduler.cancel()
                completing.close(.textChanged)
            }
        }

        // MARK: announcements

        /// The native text last exchanged with the view when it still matches
        /// the storage length, else a fresh native copy (never the lazy bridge).
        func currentText(of tv: NSTextView) -> String {
            if let length = tv.textStorage?.length, lastKnownText.utf16.count == length { return lastKnownText }
            return SourceEditorView.nativeText(of: tv)
        }

        func announceNow(text: String, range: NSRange, prefix: String, suffix: String = "") {
            guard let message = SourceEditorView.boundedSelectionAnnouncement(text: text, range: range) else { return }
            announcements.append(prefix + message + suffix)
            if announcements.count > 64 { announcements.removeFirst(announcements.count - 64) }
            announce(prefix + message + suffix)
        }

        private func post(_ message: String) {
            guard let tv = textView else { return }
            NSAccessibility.post(element: tv, notification: .announcementRequested,
                                 userInfo: [.announcement: message, .priority: NSAccessibilityPriorityLevel.low.rawValue])
        }
    }
}
