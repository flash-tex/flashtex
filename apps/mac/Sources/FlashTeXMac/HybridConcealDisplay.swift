import AppKit
import ObjectiveC

/// Hybrid conceal on the Mac editor (lane HYBRID-CONCEAL): draws the spans
/// `HybridConceal` (FlashTeXEditorCore) finds — `\alpha` as α, `\textbf{x}`
/// as a bold x — without ever changing the text storage. Copy, Find, undo,
/// saving, compiling and the caret all see the real source.
///
/// ## How it draws (TextKit 1, the editor's text system)
///
/// - **Hidden** source (`\textbf{`, `}`): the glyphs get the `.null`
///   property in `NSLayoutManagerDelegate.layoutManager(_:shouldGenerateGlyphs:…)`
///   (EditorFolding.swift hides folded bodies the same way). They take no
///   space and draw nothing; the characters are untouched.
/// - **Replaced** source (`\alpha` → α): the first glyph becomes a control
///   glyph laid out as whitespace exactly as wide as the replacement
///   (`layoutManager(_:boundingBoxForControlGlyphAt:…)`), the rest `.null`,
///   and the replacement is drawn into that box after the text
///   (`draw(in:)`, chained after the error lens). Characters no font of the
///   editor has (ℝ, 𝒜) use the system's font fallback.
/// - **Styled** kept text (`\textbf{x}` → **x**): temporary attributes
///   (a hard one-point-wide `.shadow` in the text's colour for bold, `.obliqueness` for italic) painted with the
///   syntax colours over the same window, so nothing lays out differently.
/// - **Dimmed comments**: the theme's comment colour at reduced alpha
///   (`EditorThemeRuntime.commentAlpha`): a redraw, nothing else.
///
/// ## Reveal
///
/// A span shows its source when the caret or a selection is on its line
/// (`.line`), or touches the span (`.construct`). The revealed character
/// ranges are kept in storage coordinates and shifted by every edit, so a
/// keystroke on the caret's line never re-lays anything out; a caret move
/// to another line invalidates the glyphs of the old and new line only, and
/// only when they have something concealed. A selection longer than
/// `maxRevealedLines` lines reveals the lines at its two ends only.
///
/// ## Cost
///
/// Spans are computed per line from `SyntaxHighlighter`'s per-line lexing
/// (one line's runs), cached by line relative to the line's start, and
/// dropped at each edit for the edited lines and any whose mode the edit
/// changed; every other line keeps its entry. Nothing runs while conceal is
/// off (the delegate returns before looking at a character).
///
/// ## Caret and VoiceOver
///
/// The caret moves over real characters. In line mode the caret's line is
/// always revealed, so arrow keys never step through hidden text; a vertical
/// move that lands inside a construct that was concealed is snapped to the
/// construct's nearer end. VoiceOver reads the text view's value, which is
/// the source: `\alpha` is read as "backslash alpha", never as a symbol that
/// is not in the document (apps/mac/docs/hybrid-conceal.md).
@MainActor
final class ConcealController {
    typealias Span = HybridConceal.Span

    /// Test seam, like `LineNumberGutter.relativeOverride`: a hosted editor
    /// reads the shared preferences, which a test cannot inject into.
    nonisolated(unsafe) static var settingsOverride: HybridConceal.Settings?

    static let maxRevealedLines = 400
    static let styleKeys: [NSAttributedString.Key] = [.shadow, .obliqueness]

    private(set) var settings = HybridConceal.Settings()
    private weak var textView: NSTextView?
    private weak var syntax: SyntaxPainter?
    /// Spans per line index, with ranges relative to the line's start, so an
    /// edit leaves every other line's entry valid (only a line break typed or
    /// removed renumbers the lines after it).
    private var cache: [Int: [Span]] = [:]
    /// Revealed character ranges (line mode: whole lines; construct mode: spans).
    private(set) var revealed: [NSRange] = []
    /// Whether the edited lines had something concealed before the last edit.
    private var editedLinesHadConceal = false
    /// End of the last edit's replacement (new coordinates).
    private var lastEditEnd = 0
    private var widthCache: [String: CGFloat] = [:]
    /// Appearance and theme the bold strokes were resolved for.
    fileprivate(set) var styleStamp: StyleStamp?
    fileprivate var styleRepaintScheduled = false
    private var widthFont: NSFont?

    // Evidence for tests and the bench.
    private(set) var linesComputed = 0
    private(set) var glyphInvalidations = 0
    private(set) var charactersInvalidated = 0

    nonisolated(unsafe) private static var key: UInt8 = 0

    /// The controller attached to `textView`, if any (the layout-manager
    /// delegate and `EditorPreferences.apply(to:)` find it here).
    static func attached(to textView: NSTextView) -> ConcealController? {
        (objc_getAssociatedObject(textView, &key) as? WeakBox)?.controller
    }

    /// The text view can outlive its coordinator (and this controller): the
    /// association holds it weakly, never as an unretained pointer.
    private final class WeakBox: NSObject {
        weak var controller: ConcealController?
        init(_ controller: ConcealController) { self.controller = controller }
    }

    func attach(_ tv: NSTextView, syntax: SyntaxPainter) {
        textView = tv
        self.syntax = syntax
        syntax.conceal = self
        objc_setAssociatedObject(tv, &Self.key, WeakBox(self), .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        if let completing = tv as? CompletingTextView {
            let lens = completing.foregroundDecorator
            completing.foregroundDecorator = { [weak self] rect in
                lens?(rect)
                self?.draw(in: rect)
            }
        }
        update(settings: Self.settingsOverride ?? EditorPreferences.shared.conceal)
    }

    /// Whether anything can be concealed in this buffer now.
    var isActive: Bool {
        guard settings.concealsAnything, let syntax else { return false }
        return syntax.language == .latex || syntax.language == .package
    }

    // MARK: settings

    /// New settings: drop every cached span and re-lay out the text (a rare,
    /// explicit user action, like folding or a font change).
    func update(settings new: HybridConceal.Settings) {
        EditorThemeRuntime.setCommentsDimmed(new.dimsComments)
        guard new != settings else { return }
        let wasActive = isActive
        settings = new
        cache.removeAll()
        guard let tv = textView else { return }
        refreshReveal(in: tv, relayout: false)
        if wasActive || isActive { invalidateEverything(in: tv) }
    }

    private func invalidateEverything(in tv: NSTextView) {
        guard let lm = tv.layoutManager, let storage = tv.textStorage else { return }
        let whole = NSRange(location: 0, length: storage.length)
        lm.invalidateGlyphs(forCharacterRange: whole, changeInLength: 0, actualCharacterRange: nil)
        lm.invalidateLayout(forCharacterRange: whole, actualCharacterRange: nil)
        glyphInvalidations += 1
        charactersInvalidated += whole.length
        for range in syntax?.painted ?? [] { paintStyles(in: NSIntersectionRange(range, whole), layoutManager: lm) }
        tv.needsDisplay = true
    }

    // MARK: spans

    private var table: SyntaxHighlighter? {
        guard let syntax, let text = textView?.textStorage?.string as NSString?, syntax.inSync(with: text) else { return nil }
        return syntax.highlighter
    }

    /// Spans of line `line` (all of them, revealed or not), in storage coordinates.
    func spans(line: Int) -> [Span] {
        guard isActive, let table, line < table.lineCount else { return [] }
        let start = table.lineStarts[line]
        if let cached = cache[line] { return Self.shift(cached, by: start) }
        guard let text = textView?.textStorage?.string as NSString? else { return [] }
        let range = table.lineRange(line)
        let spans = range.length > 0
            ? HybridConceal.spans(in: text, lineRange: range, runs: table.runs(in: range, text: text), settings: settings) : []
        linesComputed += 1
        if cache.count > 20_000 { cache.removeAll() }
        cache[line] = Self.shift(spans, by: -start)
        return spans
    }

    static func shift(_ spans: [Span], by delta: Int) -> [Span] {
        guard delta != 0, !spans.isEmpty else { return spans }
        return spans.map { span in
            var s = span
            s.range.location += delta
            s.pieces = s.pieces.map { HybridConceal.Piece(NSRange(location: $0.range.location + delta, length: $0.range.length), $0.action) }
            return s
        }
    }

    /// Spans on the lines covering `range`.
    func spans(in range: NSRange) -> [Span] {
        guard let table else { return [] }
        let first = table.line(at: range.location)
        let last = table.line(at: max(range.location, NSMaxRange(range) - 1))
        return (first...last).flatMap { spans(line: $0) }
    }

    func isRevealed(_ span: Span) -> Bool {
        switch settings.reveal {
        case .never: return true
        case .line: return revealed.contains { NSLocationInRange(span.range.location, $0) }
        case .construct: return revealed.contains { $0 == span.range }
        }
    }

    /// Concealed (not revealed) spans on the lines covering `range`.
    func concealedSpans(in range: NSRange) -> [Span] {
        spans(in: range).filter { !isRevealed($0) }
    }

    // MARK: edits (from SyntaxPainter)

    /// The storage replaced `range` (old coordinates) with `replacementLength`
    /// units; called synchronously from the storage notification, *before*
    /// the highlighter's edit (its line table is still the old one). No layout here.
    func textEdited(range: NSRange, replacementLength: Int) {
        lastEditEnd = range.location + replacementLength
        guard isActive, let table = syntax?.highlighter, table.length > 0,
              let text = textView?.textStorage?.string as NSString? else { cache.removeAll(); return }
        let first = table.line(at: min(range.location, table.length))
        let lastOld = table.line(at: min(NSMaxRange(range), table.length))
        // Did the edited lines draw anything concealed? (Not cached: assume so.)
        editedLinesHadConceal = (first...lastOld).contains { line in
            guard let spans = cache[line], line < table.lineCount else { return true }
            return Self.shift(spans, by: table.lineStarts[line]).contains { !isRevealed($0) }
        }
        for line in first...lastOld { cache.removeValue(forKey: line) }
        // Line breaks typed or removed renumber the lines after the edit.
        var added = 0
        let inserted = NSRange(location: range.location, length: min(replacementLength, max(0, text.length - range.location)))
        if inserted.length > 0 {
            var i = inserted.location
            while i < NSMaxRange(inserted) { if text.character(at: i) == 0x0A { added += 1 }; i += 1 }
        }
        let lineDelta = added - (lastOld - first)
        if lineDelta != 0 {
            var moved: [Int: [Span]] = [:]
            moved.reserveCapacity(cache.count)
            for (line, spans) in cache { moved[line > lastOld ? line + lineDelta : line] = spans }
            cache = moved
        }
        revealed = revealed.map { SyntaxPainter.shifted($0, edit: range, replacementLength: replacementLength) }
    }

    /// The highlighter re-lexed `dirty` (whole lines) after an edit; painting
    /// is allowed again. Re-lays out the edited lines only when they draw
    /// something concealed now or did before, and the lines after them only
    /// when the edit changed their mode (a `$` typed, an `\end{…}` removed).
    func linesRelexed(_ dirty: NSRange) {
        guard isActive, let tv = textView, let lm = tv.layoutManager, let table, let storage = tv.textStorage else { return }
        let whole = NSRange(location: 0, length: storage.length)
        let d = NSIntersectionRange(dirty, whole)
        guard d.length > 0 else { return }
        let first = table.line(at: d.location)
        let last = table.line(at: max(d.location, NSMaxRange(d) - 1))
        let editedLast = min(last, max(first, table.line(at: min(lastEditEnd, whole.length))))
        let edited = NSUnionRange(table.lineRange(first), table.lineRange(editedLast))
        var target: NSRange?
        if editedLinesHadConceal || !concealedSpans(in: edited).isEmpty { target = edited }
        editedLinesHadConceal = false
        if last > editedLast { // the edit changed the mode of the lines below it
            for line in (editedLast + 1)...last { cache.removeValue(forKey: line) }
            let tail = NSRange(location: NSMaxRange(edited), length: NSMaxRange(d) - NSMaxRange(edited))
            if tail.length > 0 { target = target.map { NSUnionRange($0, tail) } ?? tail }
        }
        guard let target else { return }
        invalidate(NSIntersectionRange(target, whole), in: tv, layoutManager: lm)
    }

    func didReset() {
        cache.removeAll()
        revealed = []
        guard let tv = textView, isActive else { return }
        refreshReveal(in: tv, relayout: false)
        invalidateEverything(in: tv)
    }

    private func invalidate(_ range: NSRange, in tv: NSTextView, layoutManager lm: NSLayoutManager) {
        guard range.length > 0 else { return }
        lm.invalidateGlyphs(forCharacterRange: range, changeInLength: 0, actualCharacterRange: nil)
        lm.invalidateLayout(forCharacterRange: range, actualCharacterRange: nil)
        paintStyles(in: range, layoutManager: lm)
        glyphInvalidations += 1
        charactersInvalidated += range.length
        tv.needsDisplay = true
    }

    // MARK: reveal (selection changes)

    /// The selection moved: reveal what it is on now, conceal what it left.
    /// Returns the ranges re-laid out (evidence).
    @discardableResult
    func selectionChanged(in tv: NSTextView) -> [NSRange] {
        guard isActive else { return [] }
        return refreshReveal(in: tv, relayout: true)
    }

    @discardableResult
    private func refreshReveal(in tv: NSTextView, relayout: Bool) -> [NSRange] {
        guard let table, let lm = tv.layoutManager, let storage = tv.textStorage else { return [] }
        let selection = tv.selectedRange()
        let new = revealTargets(for: selection, table: table)
        guard new != revealed else { return [] }
        let old = revealed
        revealed = new
        guard relayout, !tv.hasMarkedText() else { return [] }
        // Lines whose reveal state changed and that have something to conceal.
        let whole = NSRange(location: 0, length: storage.length)
        var changed: [NSRange] = []
        for r in Set(old.map(\.hashKey)).symmetricDifference(Set(new.map(\.hashKey))).map(\.range) where r.location <= whole.length {
            let clipped = NSIntersectionRange(r, whole)
            let lines = lineRanges(covering: clipped.length > 0 ? clipped : NSRange(location: min(r.location, whole.length), length: 0), table: table)
            if !spans(in: lines).isEmpty { changed.append(lines) }
        }
        // Defer while the storage is mid-edit (GH#681: no layout inside processEditing).
        if !storage.editedMask.isEmpty {
            DispatchQueue.main.async { [weak self, weak tv] in
                guard let self, let tv, let lm = tv.layoutManager else { return }
                for r in SourceEditorView.MarkPainter.merged(changed) { self.invalidate(r, in: tv, layoutManager: lm) }
            }
            return changed
        }
        for r in SourceEditorView.MarkPainter.merged(changed) { invalidate(r, in: tv, layoutManager: lm) }
        return changed
    }

    private func lineRanges(covering range: NSRange, table: SyntaxHighlighter) -> NSRange {
        let first = table.line(at: range.location)
        let last = table.line(at: max(range.location, NSMaxRange(range) - 1))
        return NSUnionRange(table.lineRange(first), table.lineRange(last))
    }

    private func revealTargets(for selection: NSRange, table: SyntaxHighlighter) -> [NSRange] {
        guard table.length > 0 else { return [] }
        let loc = min(selection.location, table.length)
        let endLoc = min(NSMaxRange(selection), table.length)
        switch settings.reveal {
        case .never: return []
        case .line:
            let first = table.line(at: loc)
            let last = table.line(at: max(loc, endLoc - (selection.length > 0 && endLoc > loc ? 1 : 0)))
            if last - first + 1 > Self.maxRevealedLines {
                return [table.lineRange(first), table.lineRange(last)]
            }
            return [NSUnionRange(table.lineRange(first), table.lineRange(last))]
        case .construct:
            let lines = selection.length == 0 ? [table.line(at: loc)]
                : Array(Set([table.line(at: loc), table.line(at: max(loc, endLoc - 1))]))
            var result: [NSRange] = []
            for line in lines {
                for span in spans(line: line)
                where HybridConceal.isRevealed(span, reveal: .construct, selection: selection, revealedLines: []) {
                    result.append(span.range)
                }
            }
            return result.sorted { $0.location < $1.location }
        }
    }

    /// A caret that moved onto a line that was concealed (arrow keys, a
    /// click) and landed strictly inside source that was hidden there — a
    /// hidden `\textbf{`, a replaced `\alpha` — snaps to that piece's nearer
    /// end, so it never sits in text that was invisible when it got there.
    /// Kept text (the bold word) was visible, so a caret there stays.
    /// Returns the snapped location, or nil when nothing to do.
    func snappedCaret(_ caret: Int, previouslyConcealed: [Span]) -> Int? {
        for span in previouslyConcealed {
            for piece in span.pieces {
                if case .style = piece.action { continue }
                let r = piece.range
                guard caret > r.location, caret < NSMaxRange(r) else { continue }
                return caret - r.location <= NSMaxRange(r) - caret ? r.location : NSMaxRange(r)
            }
        }
        return nil
    }

    // MARK: glyph generation (the layout-manager delegate)

    /// Pieces that change glyphs (hide / replace) on concealed spans between
    /// two character indexes, sorted. Empty when nothing to do.
    func glyphPieces(from firstChar: Int, to lastChar: Int) -> [HybridConceal.Piece] {
        guard isActive else { return [] }
        let range = NSRange(location: firstChar, length: max(1, lastChar - firstChar + 1))
        var pieces: [HybridConceal.Piece] = []
        for span in concealedSpans(in: range) {
            for piece in span.pieces {
                if case .style = piece.action { continue }
                pieces.append(piece)
            }
        }
        return pieces.sorted { $0.range.location < $1.range.location }
    }

    /// Applies `pieces` to one glyph run's properties: replaced source's first
    /// glyph becomes a control glyph, the rest of the source null. Folded
    /// (already null) glyphs stay null.
    static func apply(_ pieces: [HybridConceal.Piece], props: inout [NSLayoutManager.GlyphProperty], chars: [Int]) {
        var k = 0
        for i in 0..<chars.count {
            let c = chars[i]
            while k < pieces.count, NSMaxRange(pieces[k].range) <= c { k += 1 }
            guard k < pieces.count, NSLocationInRange(c, pieces[k].range), !props[i].contains(.null) else { continue }
            switch pieces[k].action {
            case .hide: props[i] = .null
            case .replace: props[i] = c == pieces[k].range.location ? .controlCharacter : .null
            case .style: break
            }
        }
    }

    /// The replacement drawn at `charIndex` (a concealed replacement's first character).
    func replacement(at charIndex: Int) -> (text: String, style: HybridConceal.Style)? {
        guard isActive, let table, charIndex < table.length else { return nil }
        for span in spans(line: table.line(at: charIndex)) where NSLocationInRange(charIndex, span.range) && !isRevealed(span) {
            for piece in span.pieces where piece.range.location == charIndex {
                if case .replace(let text, let style) = piece.action { return (text, style) }
            }
        }
        return nil
    }

    // MARK: drawing

    private func font(for style: HybridConceal.Style, base: NSFont) -> NSFont {
        switch style {
        case .superscript, .subscript: return NSFontManager.shared.convert(base, toSize: (base.pointSize * 0.72).rounded())
        case .heading: return NSFontManager.shared.convert(base, toHaveTrait: .boldFontMask)
        default: return base
        }
    }

    private func color(for style: HybridConceal.Style) -> NSColor {
        switch style {
        case .text, .bold, .italic: return SyntaxTheme.foreground
        case .heading: return SyntaxTheme.command
        case .math, .superscript, .subscript: return SyntaxTheme.conceal
        }
    }

    /// Width of the box a replacement takes.
    func width(of text: String, style: HybridConceal.Style) -> CGFloat {
        guard let base = textView?.font else { return 0 }
        if widthFont != base { widthCache.removeAll(); widthFont = base }
        let key = "\(style.rawValue)|\(text)"
        if let w = widthCache[key] { return w }
        let w = ceil((text as NSString).size(withAttributes: [.font: font(for: style, base: base)]).width)
        widthCache[key] = w
        return w
    }

    /// Draws every concealed replacement whose box intersects `rect`.
    func draw(in rect: NSRect) {
        guard isActive, let tv = textView, let lm = tv.layoutManager, let container = tv.textContainer,
              let base = tv.font, lm.numberOfGlyphs > 0 else { return }
        // Bold strokes carry concrete colours (see `embolden`): an appearance
        // or theme change since they were painted repaints them after this draw.
        if styleStamp != nil, styleStamp != currentStyleStamp(tv), !styleRepaintScheduled {
            styleRepaintScheduled = true
            DispatchQueue.main.async { [weak self] in self?.repaintStylesInWindow() }
        }
        let origin = tv.textContainerOrigin
        let local = rect.offsetBy(dx: -origin.x, dy: -origin.y)
        let glyphs = lm.glyphRange(forBoundingRectWithoutAdditionalLayout: local, in: container)
        guard glyphs.length > 0 else { return }
        let chars = lm.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil)
        for span in concealedSpans(in: chars) {
            for piece in span.pieces {
                guard case .replace(let text, let style) = piece.action else { continue }
                let g = lm.glyphIndexForCharacter(at: piece.range.location)
                guard g < lm.numberOfGlyphs, lm.propertyForGlyph(at: g).contains(.controlCharacter) else { continue }
                let fragment = lm.lineFragmentRect(forGlyphAt: g, effectiveRange: nil, withoutAdditionalLayout: true)
                let location = lm.location(forGlyphAt: g)
                let drawFont = font(for: style, base: base)
                var baseline = fragment.minY + location.y + origin.y
                if style == .superscript { baseline -= base.pointSize * 0.36 }
                if style == .subscript { baseline += base.pointSize * 0.14 }
                let point = NSPoint(x: fragment.minX + location.x + origin.x, y: baseline - drawFont.ascender)
                (text as NSString).draw(at: point, withAttributes: [.font: drawFont, .foregroundColor: color(for: style)])
            }
        }
    }

    /// Bold/italic kept text as temporary attributes over `range` (whole
    /// lines are painted by the callers): cleared first, so a revealed span
    /// loses its style.
    func paintStyles(in range: NSRange, layoutManager lm: NSLayoutManager) {
        guard range.length > 0 else { return }
        if let tv = textView { styleStamp = currentStyleStamp(tv) }
        for key in Self.styleKeys { lm.removeTemporaryAttribute(key, forCharacterRange: range) }
        guard isActive else { return }
        for span in concealedSpans(in: range) {
            for piece in span.pieces {
                guard case .style(let style) = piece.action else { continue }
                let r = NSIntersectionRange(piece.range, range)
                guard r.length > 0 else { continue }
                switch style {
                case .bold, .heading: embolden(r, layoutManager: lm)
                case .italic: lm.addTemporaryAttribute(.obliqueness, value: 0.2, forCharacterRange: r)
                default: break
                }
            }
        }
    }
}

extension ConcealController {
    /// Faux bold: each run drawn twice, the second copy a hard shadow a
    /// fraction of a point to the right in the run's own colour (the syntax
    /// colour painted there, else the text colour), resolved for the view's
    /// appearance now. A temporary `.strokeWidth` looked right in light
    /// windows but drew nothing extra on dark grounds in the layer-backed
    /// editor, so it is not used; `draw(in:)` repaints these shadows after an
    /// appearance or theme change.
    func embolden(_ r: NSRange, layoutManager lm: NSLayoutManager) {
        let appearance = textView?.effectiveAppearance ?? NSAppearance.currentDrawing()
        let offset = max(0.5, ((textView?.font?.pointSize ?? 13) / 22).rounded(.toNearestOrEven))
        var i = r.location
        while i < NSMaxRange(r) {
            var effective = NSRange(location: 0, length: 0)
            let color = lm.temporaryAttribute(.foregroundColor, atCharacterIndex: i, longestEffectiveRange: &effective, in: r) as? NSColor
            let run = NSIntersectionRange(effective.length > 0 ? effective : NSRange(location: i, length: 1), r)
            var concrete = color ?? SyntaxTheme.foreground
            appearance.performAsCurrentDrawingAppearance { concrete = concrete.usingColorSpace(.sRGB) ?? concrete }
            let shadow = NSShadow()
            shadow.shadowOffset = NSSize(width: offset, height: 0)
            shadow.shadowBlurRadius = 0
            shadow.shadowColor = concrete
            lm.addTemporaryAttribute(.shadow, value: shadow, forCharacterRange: run)
            i = max(i + 1, NSMaxRange(run))
        }
    }

    struct StyleStamp: Equatable {
        var appearance: NSAppearance.Name?
        var themeGeneration: Int
    }

    func currentStyleStamp(_ tv: NSTextView) -> StyleStamp {
        StyleStamp(appearance: tv.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]), themeGeneration: EditorThemeRuntime.generation)
    }

    /// Repaints the bold/italic attributes over the syntax painter's window.
    func repaintStylesInWindow() {
        styleRepaintScheduled = false
        guard let tv = textView, let lm = tv.layoutManager, let length = tv.textStorage?.length else { return }
        let whole = NSRange(location: 0, length: length)
        for range in syntax?.painted ?? [] { paintStyles(in: NSIntersectionRange(range, whole), layoutManager: lm) }
        if let tv = textView { styleStamp = currentStyleStamp(tv) }
    }
}

private extension NSRange {
    /// Hashable stand-in (NSRange is not Hashable).
    var hashKey: RangeKey { RangeKey(location: location, length: length) }
}

private struct RangeKey: Hashable {
    var location: Int
    var length: Int
    var range: NSRange { NSRange(location: location, length: length) }
}
