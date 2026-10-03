import AppKit

// Less redrawing per keystroke in the editor (P5-KEYSTROKE-MAIN,
// docs/evidence/keystroke-main-2026-10-03).
//
// 1. The text below an edit is not redrawn when it did not move.
//    TextKit 1 invalidates the display of everything from the edited line to
//    the end of the document on every keystroke (a plain NSTextView does the
//    same; the evidence has the stand-alone check). When an edit stays inside
//    one paragraph (no line break typed or deleted) and that paragraph's last
//    line fragment ends at the same y afterwards, every later line fragment is
//    where it was and shows the same glyphs: the text after the paragraph did
//    not change, and temporary attributes move with their characters. That
//    redraw, and the line-number gutter's, is then dropped, unless during the
//    edit (from `shouldChangeText` to the end of `didChangeText`):
//    - a temporary attribute was added or removed at or after the paragraph's
//      end (syntax colours, marks, spelling, the brace and environment-pair
//      highlight), or glyphs or layout there were invalidated; the layout
//      manager below records the ranges;
//    - or the invalidation did not come from the layout manager
//      (`needsDisplay`, `setNeedsDisplay(_:)`, a frame change): never deferred.
//    Invalidations outside an edit are never deferred either.
// 2. Long line fragments draw only the glyphs near the clip rect
//    (`EditTailLayoutManager.drawGlyphs`).
// `FLASHTEX_EDIT_TAIL=0` and `FLASHTEX_LONG_LINE_CLIP=0` turn each off.

/// A layout manager that records which characters' drawing app code changed
/// while an edit is in progress (`CompletingTextView`'s edit tail).
final class EditTailLayoutManager: NSLayoutManager {
    /// Set by the text view for the duration of an edit.
    var recording = false
    /// Character ranges touched while recording, before the storage processed
    /// the edit (old coordinates) and after it (new coordinates).
    private(set) var touchedBefore: [NSRange] = []
    private(set) var touchedAfter: [NSRange] = []
    private var edited = false
    private var inProcessEditing = false
    /// Line fragments drawn only near the clip rect (tests, evidence).
    private(set) var clippedLineDraws = 0

    func startRecording() { recording = true; edited = false; touchedBefore.removeAll(keepingCapacity: true); touchedAfter.removeAll(keepingCapacity: true) }
    func stopRecording() { recording = false; edited = false; touchedBefore.removeAll(keepingCapacity: true); touchedAfter.removeAll(keepingCapacity: true) }

    private func note(_ range: NSRange) {
        guard recording, !inProcessEditing else { return }
        if edited { touchedAfter.append(range) } else { touchedBefore.append(range) }
    }

    override func processEditing(for textStorage: NSTextStorage, edited editMask: NSTextStorageEditActions,
                                 range newCharRange: NSRange, changeInLength delta: Int,
                                 invalidatedRange invalidatedCharRange: NSRange) {
        // The storage's own edited range (the typed characters, or attributes
        // fixed over a larger range) counts; TextKit's internal invalidation
        // of the edited paragraph inside this call does not.
        if recording { edited = true }
        note(newCharRange)
        inProcessEditing = true
        defer { inProcessEditing = false }
        super.processEditing(for: textStorage, edited: editMask, range: newCharRange,
                             changeInLength: delta, invalidatedRange: invalidatedCharRange)
    }

    override func addTemporaryAttribute(_ attrName: NSAttributedString.Key, value: Any, forCharacterRange charRange: NSRange) {
        note(charRange)
        super.addTemporaryAttribute(attrName, value: value, forCharacterRange: charRange)
    }

    override func addTemporaryAttributes(_ attrs: [NSAttributedString.Key: Any], forCharacterRange charRange: NSRange) {
        note(charRange)
        super.addTemporaryAttributes(attrs, forCharacterRange: charRange)
    }

    override func setTemporaryAttributes(_ attrs: [NSAttributedString.Key: Any], forCharacterRange charRange: NSRange) {
        note(charRange)
        super.setTemporaryAttributes(attrs, forCharacterRange: charRange)
    }

    override func removeTemporaryAttribute(_ attrName: NSAttributedString.Key, forCharacterRange charRange: NSRange) {
        note(charRange)
        super.removeTemporaryAttribute(attrName, forCharacterRange: charRange)
    }

    override func invalidateGlyphs(forCharacterRange charRange: NSRange, changeInLength delta: Int,
                                   actualCharacterRange actualCharRange: NSRangePointer?) {
        note(charRange)
        super.invalidateGlyphs(forCharacterRange: charRange, changeInLength: delta, actualCharacterRange: actualCharRange)
    }

    override func invalidateLayout(forCharacterRange charRange: NSRange, actualCharacterRange actualCharRange: NSRangePointer?) {
        note(charRange)
        super.invalidateLayout(forCharacterRange: charRange, actualCharacterRange: actualCharRange)
    }

    // MARK: long lines

    /// Line fragments with at least this many glyphs draw only the glyphs
    /// near the clip rect (`drawGlyphs`).
    static let longLineGlyphs = 256
    /// Points drawn beyond each side of the clip rect: covers glyph overhang
    /// (italic, swashes) and marks positioned outside their base glyph.
    static let longLineMargin: CGFloat = 96

    /// With wrapping off, a paragraph is one line fragment that can hold
    /// thousands of glyphs, and TextKit draws every glyph (and every spelling
    /// underline) of each line fragment the dirty rect touches, although the
    /// view shows a few hundred points of it: a keystroke at the end of a long
    /// line redrew the whole line, 3–8 ms on a 10-page document. Long line
    /// fragments are drawn from the glyph at the clip rect's left edge to the
    /// one at its right edge, with `longLineMargin` on both sides; what is
    /// drawn inside the clip rect is unchanged. Lines with right-to-left or
    /// bidi-control characters, where glyph order is not left-to-right
    /// position, are drawn whole.
    override func drawGlyphs(forGlyphRange glyphsToShow: NSRange, at origin: NSPoint) {
        guard EditTailState.longLineClipEnabled, glyphsToShow.length >= Self.longLineGlyphs,
              let clip = NSGraphicsContext.current?.cgContext.boundingBoxOfClipPath, !clip.isNull, !clip.isInfinite,
              let storage = textStorage else {
            super.drawGlyphs(forGlyphRange: glyphsToShow, at: origin)
            return
        }
        let text = storage.string as NSString
        var pieces: [NSRange] = []
        var whole = false
        enumerateLineFragments(forGlyphRange: glyphsToShow) { rect, _, _, lineGlyphs, stop in
            let line = NSIntersectionRange(lineGlyphs, glyphsToShow)
            guard line.length >= Self.longLineGlyphs,
                  rect.width > clip.width + 2 * Self.longLineMargin,
                  !Self.hasBidi(text, self.characterRange(forGlyphRange: line, actualGlyphRange: nil)) else {
                pieces.append(line)
                return
            }
            let left = clip.minX - origin.x - Self.longLineMargin, right = clip.maxX - origin.x + Self.longLineMargin
            // Glyph x positions only grow along a left-to-right line, so the
            // two ends are binary searches (`glyphIndex(for:in:)` walks the
            // line glyph by glyph).
            let lo = self.firstGlyph(in: line, atOrAfterX: left - rect.minX) // positions are relative to the fragment
            let hi = self.firstGlyph(in: line, atOrAfterX: right - rect.minX)
            let from = max(line.location, lo - 1), to = min(NSMaxRange(line), hi + 1)
            guard to > from else { whole = true; stop.pointee = true; return }
            pieces.append(NSRange(location: from, length: to - from))
            if to - from < line.length { self.clippedLineDraws += 1 }
        }
        if whole || pieces.isEmpty { super.drawGlyphs(forGlyphRange: glyphsToShow, at: origin); return }
        for piece in pieces { super.drawGlyphs(forGlyphRange: piece, at: origin) }
    }

    /// The first glyph of `line` whose position is at or after `x` (relative
    /// to its line fragment), or the line's end.
    private func firstGlyph(in line: NSRange, atOrAfterX x: CGFloat) -> Int {
        var lo = line.location, hi = NSMaxRange(line)
        while lo < hi {
            let mid = (lo + hi) / 2
            if location(forGlyphAt: mid).x < x { lo = mid + 1 } else { hi = mid }
        }
        return lo
    }

    /// Right-to-left letters or bidi controls in `range`.
    static func hasBidi(_ text: NSString, _ range: NSRange) -> Bool {
        let r = NSIntersectionRange(range, NSRange(location: 0, length: text.length))
        guard r.length > 0 else { return false }
        var units = [unichar](repeating: 0, count: r.length)
        text.getCharacters(&units, range: r)
        for c in units {
            if c < 0x0590 { continue }
            if c <= 0x08FF { return true } // Hebrew, Arabic, Syriac, Thaana, NKo…
            if c >= 0xFB1D && c <= 0xFDFF { return true } // Hebrew and Arabic presentation forms
            if c >= 0xFE70 && c <= 0xFEFF { return true } // Arabic presentation forms B
            if c == 0x200E || c == 0x200F || (c >= 0x202A && c <= 0x202E) || (c >= 0x2066 && c <= 0x2069) { return true }
            if c >= 0xD800 && c <= 0xDBFF { return true } // astral planes (some right-to-left scripts): drawn whole
        }
        return false
    }

    /// Moves `textView`'s text system onto an `EditTailLayoutManager`,
    /// keeping the replaced manager's settings. Before any text or delegate
    /// is attached (`CompletingTextView.scrollable`).
    static func install(in textView: NSTextView) {
        guard let container = textView.textContainer, let old = container.layoutManager, !(old is EditTailLayoutManager) else { return }
        let lm = EditTailLayoutManager()
        lm.allowsNonContiguousLayout = old.allowsNonContiguousLayout
        lm.backgroundLayoutEnabled = old.backgroundLayoutEnabled
        lm.showsInvisibleCharacters = old.showsInvisibleCharacters
        lm.showsControlCharacters = old.showsControlCharacters
        lm.usesFontLeading = old.usesFontLeading
        lm.limitsLayoutForSuspiciousContents = old.limitsLayoutForSuspiciousContents
        lm.typesetterBehavior = old.typesetterBehavior
        lm.delegate = old.delegate
        container.replaceLayoutManager(lm)
    }
}

/// One edit in progress (`CompletingTextView.editTail`).
struct EditTailState {
    /// The edited paragraph's bottom (its last line fragment's maxY) before the edit.
    var paragraphMaxY: CGFloat
    /// The edited paragraph's end, before the edit (UTF-16).
    var oldParagraphEnd: Int
    var editLocation: Int
    var replacedLength: Int
    var replacementLength: Int
    /// What the layout manager asked to redraw below `paragraphMaxY` meanwhile.
    var deferred: NSRect = .null
}

extension CompletingTextView {
    /// Line and paragraph separators: an edit that adds or removes one moves
    /// every later line.
    private static let breaks = CharacterSet(charactersIn: "\n\r\u{85}\u{2028}\u{2029}")

    /// `shouldChangeText` accepted an edit: note where the edited paragraph
    /// ends, if this edit qualifies.
    func editTailWillChange(_ range: NSRange, replacement: String?) {
        if editTail != nil { editTailFlush() } // a nested edit: no deferral for either
        guard EditTailState.enabled, let replacement, let lm = layoutManager as? EditTailLayoutManager,
              let storage = textStorage, let container = textContainer else { return }
        let text = storage.string as NSString
        guard NSMaxRange(range) <= text.length,
              replacement.rangeOfCharacter(from: Self.breaks) == nil,
              text.substring(with: range).rangeOfCharacter(from: Self.breaks) == nil else { return }
        let paragraph = text.paragraphRange(for: range)
        // Nothing after the paragraph (the last one), or not laid out yet (an
        // edit far below what is on screen): nothing to save.
        guard NSMaxRange(paragraph) < text.length, lm.firstUnlaidCharacterIndex() > NSMaxRange(paragraph) else { return }
        let lastGlyph = lm.glyphIndexForCharacter(at: NSMaxRange(paragraph) - 1)
        let maxY = lm.lineFragmentRect(forGlyphAt: lastGlyph, effectiveRange: nil, withoutAdditionalLayout: true).maxY
        guard maxY > 0, lm.textContainer(forGlyphAt: lastGlyph, effectiveRange: nil, withoutAdditionalLayout: true) === container else { return }
        editTail = EditTailState(paragraphMaxY: maxY + textContainerOrigin.y, oldParagraphEnd: NSMaxRange(paragraph),
                                 editLocation: range.location, replacedLength: range.length,
                                 replacementLength: (replacement as NSString).length)
        lm.startRecording()
    }

    /// A layout-manager invalidation during the edit: the part below the
    /// edited paragraph waits for `editTailDidChange`. Returns what to pass on.
    func editTailFilter(_ rect: NSRect) -> NSRect {
        guard var tail = editTail, externalInvalidations == 0, rect.maxY > tail.paragraphMaxY else { return rect }
        let below = NSRect(x: rect.minX, y: max(rect.minY, tail.paragraphMaxY), width: rect.width,
                           height: rect.maxY - max(rect.minY, tail.paragraphMaxY))
        tail.deferred = tail.deferred.union(below)
        editTail = tail
        guard rect.minY < tail.paragraphMaxY else { return .null }
        return NSRect(x: rect.minX, y: rect.minY, width: rect.width, height: tail.paragraphMaxY - rect.minY)
    }

    /// The edit is done (end of `didChangeText`): drop the deferred redraw if
    /// the paragraph still ends at the same y and nothing else changed below
    /// it; otherwise redraw it as TextKit asked.
    func editTailDidChange() {
        guard let tail = editTail, let lm = layoutManager as? EditTailLayoutManager, let storage = textStorage else { editTailFlush(); return }
        let text = storage.string as NSString
        let delta = tail.replacementLength - tail.replacedLength
        let location = min(tail.editLocation, text.length)
        let paragraph = text.paragraphRange(for: NSRange(location: location, length: min(tail.replacementLength, text.length - location)))
        var unchanged = NSMaxRange(paragraph) == tail.oldParagraphEnd + delta && NSMaxRange(paragraph) < text.length
        if unchanged {
            let lastGlyph = lm.glyphIndexForCharacter(at: NSMaxRange(paragraph) - 1)
            let maxY = lm.lineFragmentRect(forGlyphAt: lastGlyph, effectiveRange: nil).maxY + textContainerOrigin.y
            unchanged = abs(maxY - tail.paragraphMaxY) < 0.01
        }
        if unchanged {
            // Anything app code touched at or after the paragraph's end keeps
            // the redraw (each range against the paragraph end of its time).
            func below(_ r: NSRange, _ end: Int) -> Bool { NSMaxRange(r) > end || r.location >= end }
            unchanged = !lm.touchedBefore.contains { below($0, tail.oldParagraphEnd) }
                && !lm.touchedAfter.contains { below($0, NSMaxRange(paragraph)) }
        }
        editTail = nil
        lm.stopRecording()
        editTailDroppedCount += unchanged && !tail.deferred.isNull ? 1 : 0
        if !unchanged, !tail.deferred.isNull { setNeedsDisplay(tail.deferred, avoidAdditionalLayout: false) }
        if !unchanged { gutterAfterEdit?.redrawAfterEdit() }
        gutterAfterEdit = nil
    }

    /// Redraws whatever was deferred and ends the edit (a nested edit, an
    /// edit that never finished before the next display).
    func editTailFlush() {
        gutterAfterEdit?.redrawAfterEdit()
        gutterAfterEdit = nil
        guard let tail = editTail else { return }
        editTail = nil
        (layoutManager as? EditTailLayoutManager)?.stopRecording()
        if !tail.deferred.isNull { setNeedsDisplay(tail.deferred, avoidAdditionalLayout: false) }
    }
}

extension EditTailState {
    /// `FLASHTEX_EDIT_TAIL=0` turns the deferral off (A/B evidence and a
    /// field escape hatch); on by default.
    static let enabled = ProcessInfo.processInfo.environment["FLASHTEX_EDIT_TAIL"] != "0"
    /// `FLASHTEX_LONG_LINE_CLIP=0` draws long line fragments whole again
    /// (`EditTailLayoutManager.drawGlyphs`); on by default.
    nonisolated(unsafe) static var longLineClipEnabled = ProcessInfo.processInfo.environment["FLASHTEX_LONG_LINE_CLIP"] != "0"
}
