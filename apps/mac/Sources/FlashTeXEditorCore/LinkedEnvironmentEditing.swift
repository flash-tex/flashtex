import Foundation

// Shared by the Mac and iPad editors (FlashTeXEditorCore); platform-free.

/// Linked `\begin{name}` / `\end{name}` editing: typing, deleting, pasting
/// or accepting a completion inside one name of a balanced pair rewrites the
/// other name to match. Pure over UTF-16 `NSString`s; each editor adds only
/// its text-view glue (the Mac's `SourceEditorView.Coordinator`, the iPad's
/// `EditorController`):
///
/// 1. before a user edit, `isOnEnvironmentName` (O(line)) gates the
///    `session(for:in:continuing:)` capture (one O(n) lexical scan), so
///    ordinary typing elsewhere never pays for a document scan;
/// 2. after the edit, `partnerEdit(for:in:)` (O(name)) says what to write
///    over the partner, in the same undo group as the edit, and refuses when
///    the partner no longer reads the old name where the session expects it.
///
/// Unbalanced pairs never link; verbatim *bodies* and `\verb` never link,
/// but the names of a verbatim environment itself do.
public enum LinkedEnvironmentEditing {
    /// Caret is inside (or at the end of) a `\begin{name}` / `\end{name}` name
    /// of a balanced pair: the name span it is on, the other end's name span,
    /// and the name. An emptied name (`\begin{}` … `\end{}`) still links, so a
    /// name deleted letter by letter can be typed anew. One lexical scan
    /// (`LaTeXScan.uses`); same-name nesting resolves by a per-name stack,
    /// innermost pair first.
    public static func linkedNames(at caret: Int, in text: NSString) -> (active: NSRange, partner: NSRange, name: String)? {
        let uses = LaTeXScan.uses(in: text)
        for u in uses where u.name == "verb" || u.name == "verb*" {
            if NSLocationInRange(caret, u.range) || caret == NSMaxRange(u.range) { return nil }
        }
        var open: [String: [(use: LaTeXScan.Use, arg: NSRange)]] = [:]
        var best: (active: NSRange, partner: NSRange, name: String, begin: Int)?
        for u in uses where u.name == "begin" || u.name == "end" {
            guard let name = u.arg, let arg = u.argRange else { continue }
            if u.name == "begin" {
                open[name, default: []].append((u, arg))
                continue
            }
            guard let b = open[name]?.popLast() else { continue } // a stray `\end` never links
            let verbatim = SyntaxHighlighter.verbatimEnvironments.contains(name) || name == "comment"
            if verbatim, caret >= NSMaxRange(b.use.range), caret < u.range.location { return nil }
            let link: (NSRange, NSRange)? = containsCaret(caret, b.arg) ? (b.arg, arg)
                : containsCaret(caret, arg) ? (arg, b.arg) : nil
            if let link, best == nil || b.use.range.location > best!.begin {
                best = (link.0, link.1, name, b.use.range.location)
            }
        }
        return best.map { ($0.active, $0.partner, $0.name) }
    }

    /// A linked-editing session: the name span the user is editing, its
    /// partner and the partner's name, captured *before* the edit (in
    /// `shouldChangeTextIn`) at storage length `length`. Once the names
    /// diverge a fresh pair scan would refuse to link, so the spans are carried
    /// forward by the length change instead of rediscovered afterwards.
    public struct LinkedSession: Equatable, Sendable {
        public var active: NSRange
        public var partner: NSRange
        public var name: String
        public var length: Int

        public init(active: NSRange, partner: NSRange, name: String, length: Int) {
            self.active = active
            self.partner = partner
            self.name = name
            self.length = length
        }

        /// The spans after the storage grew or shrank to `newLength`, every
        /// change having landed inside `active` (the typed edit and an
        /// auto-inserted closer at its caret).
        public func advanced(to newLength: Int) -> LinkedSession {
            let delta = newLength - length
            var s = self
            s.active.length = max(0, active.length + delta)
            if partner.location >= NSMaxRange(active) { s.partner.location += delta }
            s.length = newLength
            return s
        }

        /// True when `edit` (current coordinates) lies inside the active span.
        public func covers(_ edit: NSRange) -> Bool {
            edit.location >= active.location && NSMaxRange(edit) <= NSMaxRange(active)
        }
    }

    /// Starts (or, during an IME composition, continues) a linked session for
    /// a user edit of `range`. `text` is the pre-edit buffer; `continuing` is
    /// the open session when marked text is being replaced. Nil when the edit
    /// is not inside one name span of a balanced pair.
    public static func session(for range: NSRange, in text: NSString, continuing: LinkedSession?) -> LinkedSession? {
        if let s = continuing?.advanced(to: text.length), s.covers(range) { return s }
        guard let link = linkedNames(at: range.location, in: text) else { return nil }
        let s = LinkedSession(active: link.active, partner: link.partner, name: link.name, length: text.length)
        return s.covers(range) ? s : nil
    }

    /// The partner rewrite once the edits of `session` are in `now`, in `now`'s
    /// coordinates. Nil when the names already match, or when `now` no longer
    /// shows the partner where the session expects it (something else changed
    /// the buffer: never write into a guessed range).
    public static func partnerEdit(for session: LinkedSession, in now: NSString) -> (range: NSRange, replacement: String)? {
        let s = session.advanced(to: now.length)
        guard s.active.location >= 1, NSMaxRange(s.active) <= now.length,
              s.partner.location >= 1, NSMaxRange(s.partner) < now.length,
              now.character(at: s.active.location - 1) == 0x7B,
              now.character(at: s.partner.location - 1) == 0x7B,
              now.character(at: NSMaxRange(s.partner)) == 0x7D,
              now.substring(with: s.partner) == s.name else { return nil }
        let newName = now.substring(with: s.active)
        guard newName != s.name else { return nil }
        return (s.partner, newName)
    }

    /// Partner name-span rewrite for a user edit inside a linked name, in
    /// post-edit coordinates, from the pre-edit buffer `old`. Nil when the
    /// edit is not in a balanced name span or the partner already matches.
    public static func partnerEdit(old: NSString, edit: (range: NSRange, replacement: String)) -> (range: NSRange, replacement: String)? {
        guard let session = session(for: edit.range, in: old, continuing: nil) else { return nil }
        let applied = old.replacingCharacters(in: edit.range, with: edit.replacement) as NSString
        return partnerEdit(for: session, in: applied)
    }

    /// The name span of a balanced `\begin{name}` … `\end{name}` pair that
    /// holds the completion range `range`, when there is one: accepting an
    /// environment there renames it (the accepted name replaces the whole
    /// name, not only the typed prefix, with no body skeleton and no second
    /// `}`) rather than opening a new one. A name whose `}` the editor
    /// auto-inserted (`closerIsPending` at the span's end) is being typed right
    /// now (a nested `\begin{itemize` can pair with the outer `\end{itemize}`):
    /// that one still gets its skeleton.
    public static func completionRenameSpan(for range: NSRange, in text: NSString,
                                            closerIsPending: (Int) -> Bool) -> NSRange? {
        guard let link = linkedNames(at: range.location, in: text),
              range.location >= link.active.location, NSMaxRange(range) <= NSMaxRange(link.active),
              !closerIsPending(NSMaxRange(link.active)) else { return nil }
        return link.active
    }

    /// True when `caret` sits in (or at the end of) the `{name}` of `\begin` /
    /// `\end` on its line. O(line), no document pair scan: ordinary typing
    /// next to `\end{document}` must not pay the lexical scan.
    public static func isOnEnvironmentName(in text: NSString, at caret: Int) -> Bool {
        guard text.length > 0 else { return false }
        let i = min(max(0, caret), text.length)
        var j = i
        var open = -1
        while j > 0 {
            let c = text.character(at: j - 1)
            if c == 0x0A { break }
            if c == 0x7D { return false }
            if c == 0x7B { open = j - 1; break }
            j -= 1
        }
        guard open >= 0 else { return false }
        var close = open + 1
        while close < text.length {
            let c = text.character(at: close)
            if c == 0x7D || c == 0x0A { break }
            close += 1
        }
        if caret < open + 1 || caret > close { return false }
        var k = open
        while k > 0, text.character(at: k - 1) == 0x20 { k -= 1 }
        func token(_ s: String) -> Bool {
            let n = (s as NSString).length
            return k >= n && text.substring(with: NSRange(location: k - n, length: n)) == s
        }
        return token("\\begin") || token("\\end")
    }

    private static func containsCaret(_ caret: Int, _ range: NSRange) -> Bool {
        NSLocationInRange(caret, range) || caret == NSMaxRange(range)
    }
}
