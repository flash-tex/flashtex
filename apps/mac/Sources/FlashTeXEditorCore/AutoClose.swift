import Foundation

/// The auto-close discipline of the source editor as pure decisions, so the
/// iPad's `UITextViewDelegate` and the Mac's `NSTextViewDelegate` agree on
/// what a keystroke does (the Mac coordinator in SourceEditorView.swift is
/// the reference: `autoClose(after:)`, `pendingCloserCompleted`,
/// `deleteBackward` and `shiftPendingClosers`).
///
/// The caller keeps the list of *pending closers*: UTF-16 offsets of the
/// units an auto-close inserted and the user has not typed over or edited
/// away. That list, not the character under the caret, is the authority for
/// type-over, so a `}` the user wrote by hand is never stepped over.
public enum AutoClose {
    /// Openers that get their closer inserted after the caret. `(` also
    /// enables the math pairs `\(`…`\)`, `\[`…`\]` and `\left(`…`\right)`.
    public static let conventionalPairs: Set<Character> = ["{", "[", "(", "$"]

    /// The closer to insert after the caret once `opener` — a single unit the
    /// user just typed — sits right before `caretUTF16` in `text` (the text
    /// *after* the insertion). Nil when nothing should be paired: the opener
    /// is escaped, in a comment or `\verb`, followed by a word, or a `$` that
    /// closes math already open in its paragraph.
    ///
    /// Order matters and matches the Mac: `\left(` (math only) before `\(`,
    /// before the plain pair.
    public static func closer(afterTyping opener: Character, in text: String, caretUTF16: Int, mathMode: Bool,
                              pairs: Set<Character> = conventionalPairs) -> String? {
        if "([{|.".contains(opener), pairs.contains("("), mathMode,
           let leftRight = BraceMatcher.leftRightCloser(in: text, caretUTF16: caretUTF16) {
            return leftRight
        }
        if opener == "(" || opener == "[", pairs.contains("("),
           let math = BraceMatcher.mathCloser(in: text, caretUTF16: caretUTF16) {
            return math
        }
        guard pairs.contains(opener), let closer = BraceMatcher.closer(for: opener) else { return nil }
        guard BraceMatcher.autoCloseAllowed(in: text, caretUTF16: caretUTF16) else { return nil }
        return String(closer)
    }

    /// Type-over: the number of hand-typed units before `caret` that, with
    /// `typed`, complete the auto-inserted closer whose units are pending
    /// from `caret`; nil when `typed` completes nothing. `0` is the plain
    /// case — `typed` is a single-unit closer (`}`) sitting at the caret.
    /// For `\]`, `\)` or `\right)` the keystroke must be the terminal unit
    /// and the units before it must already precede the caret: `\[\alpha`
    /// + `\` inserts a backslash, then `]` steps over the whole `\]`.
    public static func overtypePrefix(typing typed: String, at caret: Int, in text: NSString, pending: [Int]) -> Int? {
        guard typed.count == 1, let unit = typed.first, !BraceMatcher.isClosingUnitOpener(unit) else { return nil }
        var k = 0
        while pending.contains(caret + k), caret + k < text.length {
            if text.substring(with: NSRange(location: caret + k, length: 1)) == typed {
                guard caret >= k else { return nil }
                let handTyped = text.substring(with: NSRange(location: caret - k, length: k))
                let units = text.substring(with: NSRange(location: caret, length: k))
                return handTyped == units ? k : nil
            }
            k += 1
        }
        return nil
    }

    /// Whether Backspace at `caret` (an empty selection) removes both halves
    /// of an auto-closed pair: the unit at the caret is a pending closer and
    /// the unit before it is its opener.
    public static func backspaceRemovesPair(at caret: Int, in text: NSString, pending: [Int]) -> Bool {
        guard caret >= 1, pending.contains(caret), text.length > caret else { return false }
        let pair = text.substring(with: NSRange(location: caret - 1, length: 2))
        guard pair.count == 2, let opener = pair.first else { return false }
        return BraceMatcher.closer(for: opener) == pair.last
    }

    /// The pending offsets after `range` was replaced by `replacementLength`
    /// units: an edit before a closer shifts it, one after leaves it, one
    /// overlapping it removes it.
    public static func shifted(_ pending: [Int], edit range: NSRange, replacementLength: Int) -> [Int] {
        guard !pending.isEmpty else { return pending }
        let delta = replacementLength - range.length
        return pending.compactMap { closer in
            if NSMaxRange(range) <= closer { return closer + delta }
            if range.location > closer { return closer }
            return nil
        }
    }
}
