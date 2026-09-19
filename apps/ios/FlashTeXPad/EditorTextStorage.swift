import FlashTeXEditorCore
import UIKit

/// The editor's text storage: an `NSMutableAttributedString` plus the shared
/// `SyntaxHighlighter` line table. Every character edit re-lexes only the
/// lines the edit touched (until the line-start modes converge — the same
/// incremental model the Mac's `SyntaxPainter` drives) and recolours that
/// range; a whole-document lex happens once, when the document is loaded.
///
/// Colours are ordinary `.foregroundColor` attributes (iOS's layout manager
/// has no temporary attributes), applied inside `processEditing` before the
/// layout manager sees the edit, so a keystroke never paints twice. Undo is
/// unaffected: attribute-only changes are not registered with the undo
/// manager, and the font/colour of typed text is normalised on the way in.
final class EditorTextStorage: NSTextStorage {
    private let backing = NSMutableAttributedString()
    private(set) var highlighter = SyntaxHighlighter(language: .latex)
    var theme: EditorTheme
    /// `backing.string` bridges a mutable NSString by copying it, and
    /// Foundation asks a text storage for its `string` and `length` many
    /// times per edit; without this cache a keystroke on a 200 KB document
    /// cost 80 ms (measured). Invalidated by every character edit.
    private var cachedString: String?

    /// Character edits processed so far; the controller uses it to notice a
    /// change exactly once whether UIKit or a programmatic edit made it.
    private(set) var editSerial = 0
    /// The range the last character edit recoloured, and the lines it lexed
    /// (evidence for the incremental invariant in the tests).
    private(set) var lastHighlightedRange = NSRange(location: 0, length: 0)
    private(set) var lastLinesLexed = 0

    init(theme: EditorTheme = .standard) {
        self.theme = theme
        super.init()
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) is not supported") }

    // MARK: NSAttributedString primitives

    override var string: String {
        if let cachedString { return cachedString }
        let s = backing.string
        cachedString = s
        return s
    }

    override var length: Int { backing.length }

    override func attributes(at location: Int, effectiveRange range: NSRangePointer?) -> [NSAttributedString.Key: Any] {
        backing.attributes(at: location, effectiveRange: range)
    }

    override func replaceCharacters(in range: NSRange, with str: String) {
        beginEditing()
        backing.replaceCharacters(in: range, with: str)
        cachedString = nil
        edited(.editedCharacters, range: range, changeInLength: (str as NSString).length - range.length)
        endEditing()
    }

    override func setAttributes(_ attrs: [NSAttributedString.Key: Any]?, range: NSRange) {
        beginEditing()
        backing.setAttributes(attrs, range: range)
        edited(.editedAttributes, range: range, changeInLength: 0)
        endEditing()
    }

    // MARK: highlighting

    override func processEditing() {
        if editedMask.contains(.editedCharacters) {
            let edited = editedRange
            let old = NSRange(location: edited.location, length: edited.length - changeInLength)
            let text = string as NSString
            let dirty: NSRange
            if highlighter.length == old.length + (text.length - edited.length) {
                dirty = highlighter.edit(range: old, replacementLength: edited.length, text: text)
            } else {
                // Out of sync (a load, or an edit made behind the model's back):
                // rebuild the table and recolour everything.
                highlighter.reset(text)
                dirty = NSRange(location: 0, length: text.length)
            }
            recolor(dirty, text: text)
            lastHighlightedRange = dirty
            lastLinesLexed = highlighter.lastEditLinesLexed
            editSerial += 1
        }
        super.processEditing()
    }

    /// `NSTextStorage`'s own attribute fixing walks every attribute run of
    /// the whole string through `attributes(at:)` whatever range it is given
    /// (58,000 calls and 85 ms per keystroke on a 200 KB document, measured);
    /// the concrete backing store fixes just the range.
    override func fixAttributes(in range: NSRange) {
        backing.fixAttributes(in: range)
    }

    /// Rebuilds the line table for the whole text and recolours it (the
    /// theme changed, or the storage was filled behind the highlighter's back).
    func rehighlightAll() {
        let text = string as NSString
        highlighter.reset(text)
        beginEditing()
        recolor(NSRange(location: 0, length: text.length), text: text)
        endEditing()
    }

    private func recolor(_ range: NSRange, text: NSString) {
        guard range.length > 0 else { return }
        // Normalise: the base font and colour, no leftover match background.
        backing.addAttributes(theme.baseAttributes, range: range)
        backing.removeAttribute(.backgroundColor, range: range)
        for run in highlighter.runs(in: range, text: text) {
            guard let color = theme.color(for: run.kind) else { continue }
            backing.addAttribute(.foregroundColor, value: color, range: run.range)
        }
        edited(.editedAttributes, range: range, changeInLength: 0)
    }

    /// Whether the highlighter's line table describes the current text.
    var isInSync: Bool { highlighter.length == backing.length }

    /// The lexer's mode at `utf16` — one line of work, never a document.
    func mode(at utf16: Int) -> SyntaxHighlighter.Mode {
        highlighter.mode(at: utf16, text: string as NSString)
    }

    /// Colour of the run at `utf16` after the incremental pass (tests).
    func foregroundColor(at utf16: Int) -> UIColor? {
        guard utf16 >= 0, utf16 < backing.length else { return nil }
        return backing.attribute(.foregroundColor, at: utf16, effectiveRange: nil) as? UIColor
    }
}
