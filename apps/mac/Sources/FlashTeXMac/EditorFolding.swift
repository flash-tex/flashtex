import AppKit
import SwiftUI

/// Code folding for the Mac source editor (lane editor-folding).
///
/// Foldable regions are `\begin{x}…\end{x}` environments
/// (`EditorNavigation.environmentPairs`) and sectioning blocks
/// (`\part`…`\subparagraph` via `DocumentOutline`): from the heading line to
/// the line before the next heading of the same or higher level, or
/// `\end{document}`. An unbalanced `\begin` (`end == nil`) is not foldable.
///
/// A fold keeps the first line visible and hides the rest. The hidden
/// characters stay in the text storage — glyph hiding is TextKit 1
/// (`NSLayoutManagerDelegate.layoutManager(_:shouldGenerateGlyphs:…)` sets
/// `.null` on those glyphs; an inline “…” is drawn at the end of the header
/// line). Undo, compile input, save and source-mapping therefore see the
/// same bytes they always did.
///
/// Vim: a folded body is one line. `j`/`k` (logical-line motions) skip the
/// hidden range rather than landing inside it; `gj`/`gk` already skip it
/// because null glyphs produce no line fragments.
enum EditorFolding {
    enum Kind: Equatable {
        case environment(String)
        case section(command: String, level: Int)
    }

    /// One foldable span. `hidden` is the body after the header line's newline;
    /// empty `hidden` means the region is a single line and cannot fold.
    struct Region: Equatable {
        var kind: Kind
        /// Header line from its start through (not including) its newline.
        var header: NSRange
        /// Characters after the header newline through the end of the region.
        var hidden: NSRange

        /// Header line start through the end of `hidden`.
        var range: NSRange {
            NSRange(location: header.location, length: max(0, NSMaxRange(hidden) - header.location))
        }

        func contains(_ caret: Int) -> Bool {
            range.location <= caret && caret <= NSMaxRange(range)
        }
    }

    /// Line start of the UTF-16 offset `p` (the character after the previous `\n`, or 0).
    static func lineStart(_ p: Int, in text: NSString) -> Int {
        var i = min(max(0, p), text.length)
        while i > 0, text.character(at: i - 1) != 0x0A { i -= 1 }
        return i
    }

    /// Index of the `\n` on the line containing `p`, or nil when the line runs to EOF.
    static func newlineIndex(onLineContaining p: Int, in text: NSString) -> Int? {
        var i = min(max(0, p), text.length)
        while i < text.length {
            if text.character(at: i) == 0x0A { return i }
            i += 1
        }
        return nil
    }

    /// Foldable region covering `[lineStart, end)` when that span has a body after the first line.
    static func region(kind: Kind, lineStart: Int, end: Int, in text: NSString) -> Region? {
        guard lineStart >= 0, end > lineStart, end <= text.length else { return nil }
        guard let nl = newlineIndex(onLineContaining: lineStart, in: text), nl < end else { return nil }
        let hiddenStart = nl + 1
        guard hiddenStart < end else { return nil }
        let header = NSRange(location: lineStart, length: nl - lineStart)
        let hidden = NSRange(location: hiddenStart, length: end - hiddenStart)
        return Region(kind: kind, header: header, hidden: hidden)
    }

    /// Every foldable region of `text`. Pass already-computed `pairs` / `outline`
    /// to avoid a second whole-buffer scan (the editor's outline and pair
    /// highlight already have those); nil means compute them here.
    static func regions(in text: NSString,
                        pairs: [EditorNavigation.EnvironmentPair]? = nil,
                        outline: [DocumentOutline.Item]? = nil) -> [Region] {
        let pairs = pairs ?? EditorNavigation.environmentPairs(in: text)
        let outline = outline ?? DocumentOutline.scan(text as String)
        var out: [Region] = []

        for p in pairs {
            guard let endRange = p.end else { continue } // unbalanced \begin is not foldable
            let start = lineStart(p.begin.location, in: text)
            let end = NSMaxRange(endRange)
            if let r = region(kind: .environment(p.name), lineStart: start, end: end, in: text) {
                out.append(r)
            }
        }

        let sections = outline.filter { $0.kind == .section }
        let endDocument: Int? = {
            if let end = pairs.first(where: { $0.name == "document" })?.end {
                return lineStart(end.location, in: text)
            }
            // A stray `\\end{document}` still ends sectioning (the pair scan ignores unmatched \\end).
            let r = text.range(of: "\\end{document}")
            return r.location == NSNotFound ? nil : lineStart(r.location, in: text)
        }()
        for (i, item) in sections.enumerated() {
            let start = lineStart(item.utf16.location, in: text)
            var end = endDocument ?? text.length
            if let next = sections[(i + 1)...].first(where: { $0.level <= item.level }) {
                end = min(end, lineStart(next.utf16.location, in: text))
            } else if let endDocument {
                end = min(end, endDocument)
            }
            if let r = region(kind: .section(command: item.command, level: item.level), lineStart: start, end: end, in: text) {
                out.append(r)
            }
        }

        return out.sorted { $0.range.location < $1.range.location || ($0.range.location == $1.range.location && $0.range.length < $1.range.length) }
    }

    /// Innermost region containing `caret` (smallest span; a caret on the header counts).
    static func innermost(at caret: Int, in regions: [Region]) -> Region? {
        regions.filter { $0.contains(caret) }.min { a, b in
            if a.range.length != b.range.length { return a.range.length < b.range.length }
            return a.range.location > b.range.location
        }
    }

    /// Whether `edit` sits inside `hidden` or shares a boundary with it
    /// (an insertion at either end, or a deletion of the adjacent character).
    static func touches(_ edit: NSRange, hidden: NSRange) -> Bool {
        let e0 = edit.location, e1 = NSMaxRange(edit)
        let h0 = hidden.location, h1 = NSMaxRange(hidden)
        if edit.length == 0 { return e0 >= h0 && e0 <= h1 }
        return (e0 < h1 && e1 > h0) || e1 == h0 || e0 == h1
    }

    /// Shift `hidden` across `edit`, or nil when the fold should unfold.
    static func shift(_ hidden: NSRange, edit: NSRange, replacementLength: Int) -> NSRange? {
        if touches(edit, hidden: hidden) { return nil }
        if NSMaxRange(edit) <= hidden.location {
            let delta = replacementLength - edit.length
            return NSRange(location: hidden.location + delta, length: hidden.length)
        }
        return hidden
    }

    /// Merge overlapping/adjacent ranges so glyph-hiding can binary-search.
    static func merge(_ ranges: [NSRange]) -> [NSRange] {
        let sorted = ranges.filter { $0.length > 0 }.sorted { $0.location < $1.location }
        var out: [NSRange] = []
        for r in sorted {
            if let last = out.last, NSMaxRange(last) >= r.location {
                out[out.count - 1] = NSRange(location: last.location, length: max(NSMaxRange(last), NSMaxRange(r)) - last.location)
            } else {
                out.append(r)
            }
        }
        return out
    }

    static func contains(_ index: Int, inSortedRanges ranges: [NSRange]) -> Bool {
        var lo = 0, hi = ranges.count
        while lo < hi {
            let mid = (lo + hi) / 2
            if ranges[mid].location <= index { lo = mid + 1 } else { hi = mid }
        }
        guard lo > 0 else { return false }
        let r = ranges[lo - 1]
        return index < NSMaxRange(r)
    }
}

/// Active folds for one editor buffer. Region computation is cached against a
/// generation that bumps on every edit; a keystroke with no folds never scans.
@MainActor
final class EditorFoldStore {
    struct Fold: Equatable {
        var kind: EditorFolding.Kind
        var header: NSRange
        var hidden: NSRange
    }

    private(set) var folds: [Fold] = []
    /// Merged hidden ranges, sorted, for glyph hiding.
    private(set) var mergedHidden: [NSRange] = []
    /// Whole-buffer region scans performed (tests: a prose keystroke must not bump this).
    private(set) var regionComputeCount = 0
    private var cachedGeneration = -1
    private var cachedRegions: [EditorFolding.Region] = []
    private var generation = 0
    weak var textView: NSTextView?
    var onChange: (() -> Void)?

    var isEmpty: Bool { folds.isEmpty }
    var cacheIsWarm: Bool { cachedGeneration == generation }

    func isHidden(_ index: Int) -> Bool { EditorFolding.contains(index, inSortedRanges: mergedHidden) }

    func reset() {
        guard !folds.isEmpty || cachedGeneration != -1 else { return }
        folds = []
        mergedHidden = []
        cachedRegions = []
        cachedGeneration = -1
        generation += 1
        onChange?()
    }

    /// Cached regions of `text`. Computes at most once per generation.
    @discardableResult
    func regions(in text: NSString) -> [EditorFolding.Region] {
        if cachedGeneration == generation { return cachedRegions }
        regionComputeCount += 1
        cachedRegions = EditorFolding.regions(in: text)
        cachedGeneration = generation
        return cachedRegions
    }

    /// Seed the cache with already-computed outline/pairs (no extra scan of those).
    func replaceCache(regions: [EditorFolding.Region]) {
        cachedRegions = regions
        cachedGeneration = generation
    }

    @discardableResult
    func foldInnermost(at caret: Int, in text: NSString) -> Bool {
        let regions = regions(in: text)
        guard let region = EditorFolding.innermost(at: caret, in: regions) else { return false }
        if folds.contains(where: { $0.hidden == region.hidden }) { return false }
        folds.append(Fold(kind: region.kind, header: region.header, hidden: region.hidden))
        rebuild()
        onChange?()
        return true
    }

    @discardableResult
    func unfoldInnermost(at caret: Int) -> Bool {
        let containing = folds.enumerated().filter { $0.element.header.location <= caret && caret <= NSMaxRange($0.element.hidden) }
        guard let innermost = containing.min(by: { $0.element.hidden.length < $1.element.hidden.length }) else { return false }
        folds.remove(at: innermost.offset)
        rebuild()
        onChange?()
        return true
    }

    func foldAll(in text: NSString) {
        let regions = regions(in: text)
        folds = regions.map { Fold(kind: $0.kind, header: $0.header, hidden: $0.hidden) }
        rebuild()
        onChange?()
    }

    func unfoldAll() {
        guard !folds.isEmpty else { return }
        folds = []
        rebuild()
        onChange?()
    }

    /// Unfold every fold whose hidden range intersects `range` (find, go-to-definition,
    /// diagnostics, preview→source). Returns whether anything changed.
    @discardableResult
    func unfoldCovering(_ range: NSRange) -> Bool {
        let before = folds.count
        folds.removeAll { hidden in
            let h = hidden.hidden
            return range.location < NSMaxRange(h) && NSMaxRange(range) > h.location
                || (range.length == 0 && range.location >= h.location && range.location <= NSMaxRange(h))
        }
        guard folds.count != before else { return false }
        rebuild()
        onChange?()
        return true
    }

    /// Shift folds across `edit`; unfold those the edit touches. Does not rescan
    /// the buffer (call `revalidate(in:)` after the storage has the new text).
    func shiftFolds(edit: NSRange, replacementLength: Int) {
        generation += 1
        cachedGeneration = -1
        guard !folds.isEmpty else { return }
        folds = folds.compactMap { fold in
            guard let hidden = EditorFolding.shift(fold.hidden, edit: edit, replacementLength: replacementLength) else { return nil }
            var header = fold.header
            if NSMaxRange(edit) <= header.location {
                header.location += replacementLength - edit.length
            } else if EditorFolding.touches(edit, hidden: header) {
                return nil
            }
            return Fold(kind: fold.kind, header: header, hidden: hidden)
        }
        rebuild()
    }

    /// Drop folds whose hidden range is no longer a foldable region of `text`.
    func revalidate(in text: NSString) {
        guard !folds.isEmpty else { return }
        let current = Set(regions(in: text).map(\.hidden))
        let before = folds.count
        folds.removeAll { !current.contains($0.hidden) }
        guard folds.count != before else { return }
        rebuild()
        onChange?()
    }

    /// Shift folds across `edit`; unfold those the edit touches. When folds remain,
    /// recompute regions of `newText` and drop any that no longer parse.
    func applyEdit(_ edit: NSRange, replacementLength: Int, newText: NSString) {
        shiftFolds(edit: edit, replacementLength: replacementLength)
        revalidate(in: newText)
    }

    /// Toggle the fold whose header line is `lineStart`.
    @discardableResult
    func toggleHeader(at lineStart: Int, in text: NSString) -> Bool {
        if let i = folds.firstIndex(where: { $0.header.location == lineStart }) {
            folds.remove(at: i)
            rebuild()
            onChange?()
            return true
        }
        return foldInnermost(at: lineStart, in: text)
    }

    /// Header line-starts that currently have a foldable region (for gutter triangles).
    func foldableLineStarts(in text: NSString) -> Set<Int> {
        Set(regions(in: text).map(\.header.location))
    }

    /// Header line-starts that are currently folded.
    var foldedLineStarts: Set<Int> { Set(folds.map(\.header.location)) }

    /// Snap a linewise vim motion that would land inside a fold: moving down
    /// skips to the first character after it, moving up to the header line.
    func adjustLinewise(from origin: Int, to target: Int) -> Int {
        var t = target
        for _ in 0..<folds.count + 1 {
            guard let hidden = folds.map(\.hidden).first(where: { t >= $0.location && t < NSMaxRange($0) }) else { return t }
            if target >= origin {
                t = NSMaxRange(hidden)
            } else {
                t = max(0, hidden.location - 1)
            }
        }
        return t
    }

    private func rebuild() {
        mergedHidden = EditorFolding.merge(folds.map(\.hidden))
        folds.sort { $0.hidden.location < $1.hidden.location }
    }
}

// MARK: - menu / first-responder actions

enum EditorFoldAction {
    static func fold() { NSApp.sendAction(#selector(CompletingTextView.foldInnermost(_:)), to: nil, from: nil) }
    static func unfold() { NSApp.sendAction(#selector(CompletingTextView.unfoldInnermost(_:)), to: nil, from: nil) }
    static func foldAll() { NSApp.sendAction(#selector(CompletingTextView.foldAll(_:)), to: nil, from: nil) }
    static func unfoldAll() { NSApp.sendAction(#selector(CompletingTextView.unfoldAll(_:)), to: nil, from: nil) }
}

/// Editor ▸ Fold / Unfold / Fold All / Unfold All (⌥⌘← / ⌥⌘→ / ⌥⇧⌘← / ⌥⇧⌘→).
struct EditorFoldCommands: Commands {
    var body: some Commands {
        CommandMenu("Editor") {
            Button("Fold") { EditorFoldAction.fold() }
                .keyboardShortcut(.leftArrow, modifiers: [.command, .option])
            Button("Unfold") { EditorFoldAction.unfold() }
                .keyboardShortcut(.rightArrow, modifiers: [.command, .option])
            Button("Fold All") { EditorFoldAction.foldAll() }
                .keyboardShortcut(.leftArrow, modifiers: [.command, .option, .shift])
            Button("Unfold All") { EditorFoldAction.unfoldAll() }
                .keyboardShortcut(.rightArrow, modifiers: [.command, .option, .shift])
        }
    }
}

// MARK: - CompletingTextView folding

extension CompletingTextView {
    /// Installs TextKit-1 glyph hiding. Call once from `SourceEditorView.makeNSView`.
    func installFolding() {
        layoutManager?.delegate = self
        folds.textView = self
        folds.onChange = { [weak self] in self?.foldingDidChange() }
    }

    func foldingDidChange() {
        guard let lm = layoutManager, let storage = textStorage else { return }
        let whole = NSRange(location: 0, length: storage.length)
        lm.invalidateGlyphs(forCharacterRange: whole, changeInLength: 0, actualCharacterRange: nil)
        lm.invalidateLayout(forCharacterRange: whole, actualCharacterRange: nil)
        needsDisplay = true
        (delegate as? SourceEditorView.Coordinator)?.refreshFoldGutter(rescan: true)
        (enclosingScrollView?.verticalRulerView as? LineNumberGutter)?.needsDisplay = true
    }

    @objc func foldInnermost(_ sender: Any?) {
        let caret = selectedRange().location
        guard folds.foldInnermost(at: caret, in: string as NSString) else { return }
        snapCaretOutOfFolds()
    }

    @objc func unfoldInnermost(_ sender: Any?) {
        _ = folds.unfoldInnermost(at: selectedRange().location)
    }

    @objc func foldAll(_ sender: Any?) {
        folds.foldAll(in: string as NSString)
        snapCaretOutOfFolds()
    }

    @objc func unfoldAll(_ sender: Any?) {
        folds.unfoldAll()
    }

    func noteFoldEdit(_ range: NSRange, replacementLength: Int) {
        folds.shiftFolds(edit: range, replacementLength: replacementLength)
    }

    /// Folding hides the body; a caret left inside it would sit on a null glyph.
    func snapCaretOutOfFolds() {
        let caret = selectedRange().location
        guard folds.isHidden(caret) else { return }
        if let fold = folds.folds.filter({ caret >= $0.hidden.location && caret < NSMaxRange($0.hidden) })
            .min(by: { $0.hidden.length < $1.hidden.length }) {
            setSelectedRange(NSRange(location: fold.header.location, length: 0))
        }
    }
}

extension CompletingTextView: NSLayoutManagerDelegate {
    /// Hide folded characters with the null glyph property. Returning 0 leaves
    /// generation to AppKit (the common path: no folds, or this run has none).
    func layoutManager(_ layoutManager: NSLayoutManager,
                              shouldGenerateGlyphs glyphs: UnsafePointer<CGGlyph>,
                              properties props: UnsafePointer<NSLayoutManager.GlyphProperty>,
                              characterIndexes charIndexes: UnsafePointer<Int>,
                              font aFont: NSFont,
                              forGlyphRange glyphRange: NSRange) -> Int {
        guard !folds.mergedHidden.isEmpty else { return 0 }
        let count = glyphRange.length
        var any = false
        for i in 0..<count where folds.isHidden(charIndexes[i]) { any = true; break }
        guard any else { return 0 }
        var g = Array(UnsafeBufferPointer(start: glyphs, count: count))
        var p = Array(UnsafeBufferPointer(start: props, count: count))
        var c = Array(UnsafeBufferPointer(start: charIndexes, count: count))
        for i in 0..<count where folds.isHidden(c[i]) {
            p[i].insert(.null)
        }
        g.withUnsafeMutableBufferPointer { gp in
            p.withUnsafeMutableBufferPointer { pp in
                c.withUnsafeMutableBufferPointer { cp in
                    layoutManager.setGlyphs(gp.baseAddress!, properties: pp.baseAddress!,
                                            characterIndexes: cp.baseAddress!, font: aFont,
                                            forGlyphRange: glyphRange)
                }
            }
        }
        return count
    }
}

extension EditorFoldStore {
    /// Inline “…” at the end of each folded header line. Drawn over the text;
    /// never inserted into the storage.
    func drawPlaceholders(in rect: NSRect, textView tv: NSTextView) {
        guard !folds.isEmpty, let lm = tv.layoutManager, let container = tv.textContainer else { return }
        let font = tv.font ?? NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        let attrs: [NSAttributedString.Key: Any] = [
            .font: font,
            .foregroundColor: NSColor.tertiaryLabelColor,
        ]
        let ellipsis = "…" as NSString
        let inset = tv.textContainerInset
        for fold in folds {
            let nl = NSMaxRange(fold.header)
            guard nl < (tv.textStorage?.length ?? 0) else { continue }
            if isHidden(nl) { continue } // nested: the header itself is inside an outer fold
            let glyphCount = lm.numberOfGlyphs
            guard glyphCount > 0 else { continue }
            let g = min(lm.glyphIndexForCharacter(at: max(fold.header.location, 0)), glyphCount - 1)
            let frag = lm.lineFragmentUsedRect(forGlyphAt: g, effectiveRange: nil, withoutAdditionalLayout: true)
            var origin = frag.origin
            origin.x += inset.width + frag.width + 4
            origin.y += inset.height + max(0, (frag.height - font.boundingRectForFont.height) / 2)
            let box = NSRect(x: origin.x, y: origin.y, width: 16, height: frag.height)
            guard NSIntersectsRect(rect, box) else { continue }
            _ = container
            ellipsis.draw(at: origin, withAttributes: attrs)
        }
    }
}
