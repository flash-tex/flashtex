import AppKit

/// Tab / Shift-Tab indentation (GH74) kept separate from SourceEditorView.swift
/// (owned by another lane): the pure decision logic lives here, and only a
/// couple of small hooks connect it — one line in the Coordinator's existing
/// `doCommandBy` dispatch (SourceEditorView.swift), and one closure property
/// plus a single call site in Completion.swift's snippet insertion, so a
/// completion's placeholder closer (`\section{}`) is tracked for overtype the
/// same way a hand-typed `{` already is. LaTeX-aware Re-indent Lines / Document
/// (⌃I) lives in EditorIndentation.swift so this file stays Tab/Shift-Tab only.
enum EditorKeyHandling {
    /// One text-storage edit in the *original* text's coordinates.
    struct LineEdit: Equatable {
        var range: NSRange
        var replacement: String
    }

    // MARK: line ranges

    /// UTF-16 offsets of the start of every line `range` touches (at least
    /// one — the line a zero-length range/caret sits on). A range that ends
    /// exactly at the start of a following line does not pull that line in
    /// (selecting through the end of line 1 only touches line 1).
    static func lineStarts(in text: String, range: NSRange) -> [Int] {
        let ns = text as NSString
        guard range.location >= 0, NSMaxRange(range) <= ns.length else { return [] }
        var end = NSMaxRange(range)
        if range.length > 0, end < ns.length, ns.lineRange(for: NSRange(location: end, length: 0)).location == end {
            end -= 1 // ends exactly at the start of a following line: exclude that line
        }
        var starts: [Int] = []
        var loc = ns.lineRange(for: NSRange(location: range.location, length: 0)).location
        while true {
            starts.append(loc)
            let lineEnd = NSMaxRange(ns.lineRange(for: NSRange(location: loc, length: 0)))
            if lineEnd > end || lineEnd >= ns.length || lineEnd == loc { break }
            loc = lineEnd
        }
        return starts
    }

    /// Whether `range` spans more than one line (a single-line, or empty,
    /// range does not — Tab then inserts/replaces at the caret instead of
    /// indenting the whole line).
    static func isMultiLine(_ text: String, range: NSRange) -> Bool {
        range.length > 0 && lineStarts(in: text, range: range).count > 1
    }

    /// New selection after applying `edits` (each at or before `range`'s
    /// original end, true for every line-start edit this file produces). A
    /// caret (no selection) stays a caret, simply shifted by whatever the
    /// (at most one) edit on its line changed at or before it — indenting at
    /// a caret must not turn it into a selection. An actual selection snaps
    /// to cover the touched lines in full, growing/shrinking with them.
    private static func selectionAfter(_ edits: [LineEdit], range: NSRange, firstLineStart: Int) -> NSRange {
        guard range.length > 0 else {
            let delta = edits.reduce(0) { acc, edit in
                guard NSMaxRange(edit.range) <= range.location else { return acc }
                return acc + ((edit.replacement as NSString).length - edit.range.length)
            }
            return NSRange(location: range.location + delta, length: 0)
        }
        let newEnd = edits.reduce(NSMaxRange(range)) { acc, edit in
            guard NSMaxRange(edit.range) <= NSMaxRange(range) else { return acc }
            return acc + ((edit.replacement as NSString).length - edit.range.length)
        }
        return NSRange(location: firstLineStart, length: max(0, newEnd - firstLineStart))
    }

    // MARK: indent / outdent

    /// Prefixes `unit` to the start of every line `range` touches (Tab over a
    /// multi-line selection, or ⌘]). Nil when `range` or `unit` is unusable.
    static func indentEdits(in text: String, range: NSRange, unit: String) -> (edits: [LineEdit], selection: NSRange)? {
        guard !unit.isEmpty else { return nil }
        let starts = lineStarts(in: text, range: range)
        guard !starts.isEmpty else { return nil }
        let edits = starts.map { LineEdit(range: NSRange(location: $0, length: 0), replacement: unit) }
        return (edits, selectionAfter(edits, range: range, firstLineStart: starts[0]))
    }

    /// Removes up to one indent unit's worth of leading whitespace from the
    /// start of every line `range` touches (Shift-Tab, or ⌘[): the longest
    /// run (up to `unit.count`) of `unit`'s repeated character found at that
    /// line's start. A line with no matching leading whitespace contributes
    /// no edit. Nil only when `range`/`unit` is unusable; an empty (but
    /// non-nil) `edits` array means nothing was there to remove.
    static func outdentEdits(in text: String, range: NSRange, unit: String) -> (edits: [LineEdit], selection: NSRange)? {
        guard let first = unit.first else { return nil }
        let starts = lineStarts(in: text, range: range)
        guard !starts.isEmpty else { return nil }
        let ns = text as NSString
        var edits: [LineEdit] = []
        for start in starts {
            var count = 0
            while count < unit.count, start + count < ns.length, ns.character(at: start + count) == first.utf16.first {
                count += 1
            }
            guard count > 0 else { continue }
            edits.append(LineEdit(range: NSRange(location: start, length: count), replacement: ""))
        }
        return (edits, selectionAfter(edits, range: range, firstLineStart: starts[0]))
    }

    // MARK: duplicate line (⌥⇧↓ / ⌥⇧↑)

    /// Copies every line `range` touches, placing the copy below (`below`) or
    /// above the block, as one edit. The returned `selection` lands on the
    /// **copy** in both directions, so holding the key stacks copies and the
    /// caret keeps its column — which is the point of the shortcut when you
    /// are working an equation down the page a line at a time.
    ///
    /// Duplicating downward inserts after the block and shifts the selection
    /// by the inserted length; duplicating upward inserts before it, which
    /// leaves the original offsets describing the copy, so the selection does
    /// not move at all. The last line of a document has no newline to copy,
    /// so one is supplied on whichever side the copy is joined.
    static func duplicateLinesEdit(in text: String, range: NSRange, below: Bool) -> (edit: LineEdit, selection: NSRange)? {
        let ns = text as NSString
        guard range.location >= 0, NSMaxRange(range) <= ns.length else { return nil }
        let starts = lineStarts(in: text, range: range)
        guard let first = starts.first, let last = starts.last else { return nil }
        let blockStart = first
        let blockEnd = NSMaxRange(ns.lineRange(for: NSRange(location: last, length: 0)))
        let block = ns.substring(with: NSRange(location: blockStart, length: blockEnd - blockStart))
        // `lineRange(for:)` includes the terminator when there is one; the
        // document's last line has none.
        let terminated = block.hasSuffix("\n") || block.hasSuffix("\r") || block.hasSuffix("\r\n")
        let replacement = terminated ? block : (below ? "\n" + block : block + "\n")
        let at = below ? blockEnd : blockStart
        let edit = LineEdit(range: NSRange(location: at, length: 0), replacement: replacement)
        let shift = below ? (replacement as NSString).length : 0
        return (edit, NSRange(location: range.location + shift, length: range.length))
    }

    // MARK: completion-inserted closers (Completion.swift's small hook)

    /// A programmatic multi-character insertion (a completion snippet like
    /// `\section{}`, `\begin{env}` closing itself with `\end{env}`, …) can
    /// leave the caret directly before a closing delimiter that insertion
    /// itself just placed there — exactly the situation `autoClose(after:)`
    /// in SourceEditorView.swift handles for a single hand-typed opener.
    /// Returns the UTF-16 offset to track as a pending closer (the same list
    /// `autoClose` populates), or nil when the caret is not right before one.
    static func programmaticCloser(in insertedText: String, insertedAt: Int, caretUTF16: Int) -> Int? {
        let ns = insertedText as NSString
        let local = caretUTF16 - insertedAt
        guard local >= 0, local < ns.length else { return nil }
        let unit = ns.substring(with: NSRange(location: local, length: 1))
        guard let ch = unit.first, SourceEditorView.BraceMatcher.isCloser(ch) else { return nil }
        return caretUTF16
    }

    /// Brackets whose two halves differ, so nesting can be counted. `$` (and
    /// the `\\` of an auto-inserted `\\)`) are deliberately absent: they are
    /// their own partner, so an unmatched one cannot be recognised.
    private static let bracketPairs: [Character: Character] = ["{": "}", "[": "]", "(": ")"]

    /// Whether `insertedText` itself closes a delimiter that was opened
    /// *before* it: its first bracket left unmatched by the text is `closer`.
    ///
    /// Completion snippets are written to continue an argument the user has
    /// already opened — accepting `proof` after `\begin{` inserts
    /// `proof}\n…\n\end{proof}`, which supplies the `}` for that `{`. When the
    /// editor auto-closed the same `{` the buffer already holds a `}` right
    /// after the replaced range, and inserting leaves it stranded
    /// (`\end{proof}}`). The caller consumes the tracked closer when this
    /// returns true. `\frac{}{}` and friends are balanced, so nothing is eaten.
    static func supersedesTrackedCloser(_ insertedText: String, closer: Character) -> Bool {
        guard bracketPairs.values.contains(closer) else { return false }
        var stack: [Character] = []
        var i = insertedText.startIndex
        while i < insertedText.endIndex {
            let c = insertedText[i]
            if c == "\\" { // `\{`, `\}`, `\$`: an escaped literal, never a delimiter (a control word is harmless to skip too)
                i = insertedText.index(after: i)
                if i < insertedText.endIndex { i = insertedText.index(after: i) }
                continue
            }
            if let partner = bracketPairs[c] {
                stack.append(partner)
            } else if bracketPairs.values.contains(c) {
                guard let expected = stack.last else { return c == closer } // unmatched: it belongs to an opener before this text
                if expected != c { return false } // crossed brackets: not a shape we understand
                stack.removeLast()
            }
            i = insertedText.index(after: i)
        }
        return false
    }
}

extension SourceEditorView.Coordinator {
    /// Tab: a selection spanning more than one line indents every touched
    /// line; otherwise the indent unit (`EditorPreferences.indentString`,
    /// spaces × width or one tab) replaces the current selection (a caret is
    /// a zero-length selection, so this is a plain insert there). Shift-Tab
    /// always outdents the touched line(s), whether or not there is a
    /// selection — the conventional editor behaviour (dedent acts on whole
    /// lines; indent-at-a-caret does not, so typing lines up mid-line text).
    func handleTab(reverse: Bool, in tv: NSTextView) -> Bool {
        guard programmaticChanges == 0, !tv.hasMarkedText() else { return false }
        if let completing = tv as? CompletingTextView, completing.isCompletionActive { return false }
        let text = currentText(of: tv)
        let range = tv.selectedRange()
        // The fix at the caret sits between the modal lanes and indentation.
        //
        // Completion and snippets come first and are already handled above and
        // in `CompletingTextView.keyDown`: both are mid-interaction, the author
        // is looking at them, and Tab plainly belongs to them. Indentation
        // comes after, but only loses the key on three conditions, so Tab never
        // does something invisible:
        //   - a fix is actually offered (`parent.caretFix` is non-nil, which is
        //     the same value that draws the hint, and Esc has not taken it down);
        //   - the selection is empty — Tab on a multi-line selection is
        //     unambiguously "indent this block", never "fix a word in it";
        //   - it is Tab, not ⇧Tab, which always means outdent.
        // With no fix offered this falls straight through and Tab indents
        // exactly as it did before.
        if !reverse, range.length == 0, parent.caretFix != nil {
            parent.onAcceptCaretFix()
            return true
        }
        let unit = EditorPreferences.shared.indentString
        if reverse {
            guard let (edits, selection) = EditorKeyHandling.outdentEdits(in: text, range: range, unit: unit) else { return true }
            applyLineEdits(edits, to: tv, actionName: "Outdent", selection: selection)
            return true
        }
        if EditorKeyHandling.isMultiLine(text, range: range) {
            guard let (edits, selection) = EditorKeyHandling.indentEdits(in: text, range: range, unit: unit) else { return true }
            applyLineEdits(edits, to: tv, actionName: "Indent", selection: selection)
        } else {
            tv.insertText(unit, replacementRange: range)
        }
        return true
    }

    /// Applies pre-computed line edits (already in original-text coordinates)
    /// as one undo step, last line first so earlier offsets stay valid while
    /// later ones are applied. `programmaticChanges` suppresses the normal
    /// per-keystroke path (autoClose, announcements) exactly as the single-edit
    /// paths (`applyPendingEdit`, capture insertion) already do; the buffer and
    /// selection are then pushed through once, like a committed user edit.
    func applyLineEdits(_ edits: [EditorKeyHandling.LineEdit], to tv: NSTextView, actionName: String, selection: NSRange, pushBinding: Bool = true) {
        guard !edits.isEmpty else { return }
        tv.breakUndoCoalescing()
        tv.undoManager?.beginUndoGrouping()
        programmaticChanges += 1
        for edit in edits.sorted(by: { $0.range.location > $1.range.location }) {
            guard tv.shouldChangeText(in: edit.range, replacementString: edit.replacement) else { continue }
            tv.textStorage?.replaceCharacters(in: edit.range, with: edit.replacement)
            tv.didChangeText()
        }
        programmaticChanges -= 1
        tv.undoManager?.setActionName(actionName)
        tv.undoManager?.endUndoGrouping()
        tv.breakUndoCoalescing()
        tv.setSelectedRange(selection)
        let s = SourceEditorView.nativeText(of: tv)
        lastKnownText = s
        if pushBinding { parent.text = s }
        refreshBraceHighlight(tv)
    }
}
