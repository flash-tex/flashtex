import AppKit
import QuartzCore
import FlashTeXPreviewV3

/// The caret on the page in the engine-v3 preview (DESIGN §10 app parity,
/// gap C12): the v2 pane's "you are here", always shown, not only as the
/// flash of an explicit forward search. A faint band over each row of the
/// caret's source line, the caret's glyph tinted, and a caret bar at the
/// caret's column (before the glyph that starts there, else after the last
/// glyph before it), in page points. Pure apart from the source index.
///
/// Kept out of SwiftUI: the caret moves on every keystroke, so the model
/// tells the pages view directly (`EngineV3Session.scheduleCaretMark`, once
/// per run-loop turn) instead of the pane reading the caret in its body
/// (which re-evaluated the pages' SwiftUI view per keystroke, P5-KEYSTROKE-MAIN).
struct EngineV3CaretMark: Equatable {
    var page: Int
    /// The caret bar's x and its row (nil when the engine knew no column).
    var bar: CGRect?
    var glyph: CGRect?
    /// One rect per row of the caret's line on that page.
    var band: [CGRect]

    /// Rows of `glyphs`: grouped by baseline (within half a point), each the union of its cells.
    static func rows(_ glyphs: [DL3GlyphRef]) -> [CGRect] {
        var rows: [(y: CGFloat, rect: CGRect)] = []
        for g in glyphs {
            if let i = rows.firstIndex(where: { abs($0.y - g.origin.y) <= 0.5 }) { rows[i].rect = rows[i].rect.union(g.cell) } else { rows.append((g.origin.y, g.cell)) }
        }
        return rows.sorted { $0.y < $1.y }.map(\.rect)
    }

    /// The mark from a page's glyphs of the caret's line and the caret's
    /// column, nil when the line has no glyph there.
    static func make(page: Int, line glyphs: [DL3GlyphRef], index: DL3SourceIndex, spans: Set<UInt32>, col: Int) -> EngineV3CaretMark? {
        guard !glyphs.isEmpty else { return nil }
        var mark = EngineV3CaretMark(page: page, bar: nil, glyph: nil, band: rows(glyphs))
        if let g = index.glyphs(of: spans, col: col).first, g.col != 0xFFFF {
            let x = Int(g.col) >= col ? g.cell.minX : g.cell.maxX
            mark.bar = CGRect(x: x, y: g.cell.minY, width: 0, height: g.cell.height)
            mark.glyph = g.cell
        }
        return mark
    }
}

/// Where the caret is in the text the pages were made from. The source
/// map's lines and byte columns are the compiled text's, and typing goes on
/// while the next compile runs, so the caret in the editor's text is moved
/// into the compiled one first: unchanged before the first difference,
/// shifted after the last, and inside the changed part held to it (typing
/// at the caret marks the place the typing began until the compile lands).
/// UTF-16 throughout, as the editor's text is stored: nothing is transcoded
/// per keystroke; the two texts are compared 2,048 units at a time with
/// `memcmp`, and the compiled text's line table is built once per compile.
enum EngineV3CaretPlace {
    static let chunk = 2048

    /// UTF-16 units equal at the start of both texts.
    static func commonPrefix(_ a: NSString, _ b: NSString) -> Int {
        if a === b { return min(a.length, b.length) }
        return commonPrefix(a, from: 0, b, from: 0, limit: min(a.length, b.length))
    }

    /// UTF-16 units equal from `ia` in `a` and `ib` in `b`, at most `limit`.
    static func commonPrefix(_ a: NSString, from ia: Int, _ b: NSString, from ib: Int, limit: Int) -> Int {
        guard limit > 0 else { return 0 }
        let buf = UnsafeMutablePointer<unichar>.allocate(capacity: 2 * chunk)
        defer { buf.deallocate() }
        let x = buf, y = buf + chunk
        var i = 0
        while i < limit {
            let n = min(chunk, limit - i)
            a.getCharacters(x, range: NSRange(location: ia + i, length: n))
            b.getCharacters(y, range: NSRange(location: ib + i, length: n))
            if memcmp(x, y, n * 2) != 0 {
                var k = 0
                while x[k] == y[k] { k += 1 }
                return i + k
            }
            i += n
        }
        return limit
    }

    /// UTF-16 units equal at the end of both texts, not reaching into the
    /// first `skip` units of either (the common prefix).
    static func commonSuffix(_ a: NSString, _ b: NSString, skip: Int) -> Int {
        let limit = min(a.length, b.length) - skip
        guard limit > 0 else { return 0 }
        if a === b { return limit }
        return commonSuffix(a, end: a.length, b, end: b.length, limit: limit)
    }

    /// UTF-16 units equal before `ea` in `a` and `eb` in `b`, at most `limit`.
    static func commonSuffix(_ a: NSString, end ea: Int, _ b: NSString, end eb: Int, limit: Int) -> Int {
        guard limit > 0 else { return 0 }
        let buf = UnsafeMutablePointer<unichar>.allocate(capacity: 2 * chunk)
        defer { buf.deallocate() }
        let x = buf, y = buf + chunk
        var n = 0
        while n < limit {
            let m = min(chunk, limit - n)
            a.getCharacters(x, range: NSRange(location: ea - n - m, length: m))
            b.getCharacters(y, range: NSRange(location: eb - n - m, length: m))
            if memcmp(x, y, m * 2) != 0 {
                var k = 0
                while x[m - 1 - k] == y[m - 1 - k] { k += 1 }
                return n + k
            }
            n += m
        }
        return limit
    }

    /// The caret at UTF-16 `caret` of `current`, moved into `compiled`.
    static func map(caret: Int, current: NSString, compiled: NSString) -> Int {
        let caret = min(max(0, caret), current.length)
        let p = commonPrefix(current, compiled)
        if caret <= p { return caret }
        let s = commonSuffix(current, compiled, skip: p)
        let currentEnd = current.length - s, compiledEnd = compiled.length - s
        if caret >= currentEnd { return caret - currentEnd + compiledEnd }
        return min(caret, compiledEnd)
    }

    /// Where the editor's text differs from a compiled text, kept from the
    /// editor's edits as they happen (`NSTextStorage` notifications), so a
    /// caret is moved into the compiled text without comparing the texts.
    /// A compile's window starts empty when it is sent (the editor's text is
    /// what it reads then) and grows with every edit after: one region,
    /// `[start, endCurrent)` of the editor's text for `[start, endCompiled)`
    /// of the compiled one; before it the texts are the same, after it the
    /// same shifted. UTF-16 offsets, as the editor's.
    struct Window: Equatable {
        var path: String
        var empty = true
        var start = 0, endCurrent = 0, endCompiled = 0
        /// The editor text's length after the last edit (checked against the
        /// editor before the window is trusted: a missed edit falls back to
        /// comparing the texts).
        var currentLength = 0
        /// An edit came in for another document.
        var invalid = false

        init(path: String) { self.path = path }

        /// The editor replaced `newRange.length - delta` units at
        /// `newRange.location` with `newRange.length` units; its text is now
        /// `length` units long.
        mutating func edit(newRange r: NSRange, delta: Int, length: Int) {
            let a = r.location, b = r.location + r.length - delta
            if empty {
                (start, endCurrent, endCompiled, empty) = (a, r.location + r.length, b, false)
            } else {
                // The region in the old text's terms grows to cover the edit;
                // the compiled end follows where the old text matched it.
                let end = max(endCurrent, b)
                if end > endCurrent { endCompiled += end - endCurrent }
                start = min(start, a)
                endCurrent = end + delta
            }
            currentLength = length
        }

        /// The region narrowed to where the texts really differ: the storage's
        /// `editedRange` can be wider than the characters changed (it covers
        /// attribute fixes too), so the edits give a region that may be too
        /// wide, never too narrow. Compares only inside the region: the cost
        /// follows the edits since the compile, not the document.
        mutating func narrow(current: NSString, compiled: NSString) {
            guard !empty, endCurrent <= current.length, endCompiled <= compiled.length else { return }
            let lc = endCurrent - start, lp = endCompiled - start
            let p = EngineV3CaretPlace.commonPrefix(current, from: start, compiled, from: start, limit: min(lc, lp))
            let s = EngineV3CaretPlace.commonSuffix(current, end: endCurrent, compiled, end: endCompiled, limit: min(lc, lp) - p)
            start += p; endCurrent -= s; endCompiled -= s
            if start == endCurrent, start == endCompiled { empty = true; start = 0; endCurrent = 0; endCompiled = 0 }
        }

        /// The editor's length this window expects, given the compiled text's.
        func expectedLength(compiled: Int) -> Int { empty ? compiled : currentLength }

        /// The caret at `c` of the editor's text, in the compiled text: the
        /// same before the region, shifted after it, and inside it held to
        /// the compiled part (typing marks the place the typing began).
        func map(_ c: Int) -> Int {
            if empty || c <= start { return c }
            if c >= endCurrent { return c - endCurrent + endCompiled }
            return min(c, endCompiled)
        }
    }

    /// The line starts of a text (UTF-16 offsets; a line ends at `\n`, as
    /// TeX's line numbers count them).
    struct LineTable: Equatable {
        let starts: [Int]

        init(_ t: NSString) {
            var starts = [0]
            let buf = UnsafeMutablePointer<unichar>.allocate(capacity: EngineV3CaretPlace.chunk)
            defer { buf.deallocate() }
            var i = 0
            while i < t.length {
                let n = min(EngineV3CaretPlace.chunk, t.length - i)
                t.getCharacters(buf, range: NSRange(location: i, length: n))
                for k in 0..<n where buf[k] == 0x0A { starts.append(i + k + 1) }
                i += n
            }
            self.starts = starts
        }

        /// The 1-based line and the byte column (UTF-8, from the line's
        /// start) of UTF-16 offset `u` of `t` (inside a surrogate pair: its start).
        func place(_ u: Int, in t: NSString) -> (line: Int, col: Int) {
            var u = min(max(0, u), t.length)
            if u > 0, u < t.length, UTF16.isTrailSurrogate(t.character(at: u)) { u -= 1 }
            var lo = 0, hi = starts.count - 1
            while lo < hi {
                let mid = (lo + hi + 1) / 2
                if starts[mid] <= u { lo = mid } else { hi = mid - 1 }
            }
            let start = starts[lo]
            let col = u > start ? t.substring(with: NSRange(location: start, length: u - start)).utf8.count : 0
            return (lo + 1, col)
        }
    }
}

extension EngineV3Session {
    /// Where the caret at UTF-16 `caret` of `path`'s `current` text is
    /// drawn: moved into `compiled` (the text the pages were made from) and
    /// looked up only on `pages` (the ones the pane holds: a per-keystroke
    /// cost bounded by the view).
    func caretMark(path: String, caret: Int, current: String, compiled: String, pages candidates: [Int]) -> EngineV3CaretMark? {
        let comp = compiled as NSString
        // The compiled text's line table and length: once per compile.
        let table: EngineV3CaretPlace.LineTable, compiledLength: Int
        if let c = caretLines, c.path == path, c.stamp == compiledStamp {
            (table, compiledLength) = (c.table, c.length)
        } else {
            (table, compiledLength) = (EngineV3CaretPlace.LineTable(comp), comp.length)
            caretLines = (path, compiledStamp, compiledLength, table)
        }
        let at: Int
        if var w = caretWindow, !w.invalid, w.path == path, let storage = caretStorage,
           storage.length == w.expectedLength(compiled: compiledLength) {
            // The edits since the compile: the texts are compared only inside them.
            w.narrow(current: storage.mutableString, compiled: comp)
            if w.empty { w.currentLength = 0 }
            caretWindow = w
            at = w.map(caret)
            caretMapsByWindow &+= 1
        } else {
            // No edit record for this text (a reopen, another document, an
            // editor not on screen): compare the texts.
            at = EngineV3CaretPlace.map(caret: caret, current: current as NSString, compiled: comp)
        }
        let (line, col) = table.place(at, in: comp)
        let spans = sourceMap.spans(line: line) { self.projectPath(ofEngineFile: $0) == path }
        guard !spans.isEmpty else { return nil }
        for i in candidates.sorted() {
            guard let ix = sourceIndex(page: i) else { continue }
            let gs = ix.glyphs(of: spans)
            if let m = EngineV3CaretMark.make(page: i, line: gs, index: ix, spans: spans, col: col) { return m }
        }
        return nil
    }

    /// The compiled text's line table, built off the main thread after a
    /// DONE (about 2 ms at 560 KB): the mark the compile schedules (after
    /// the settle) finds it ready. A caret mark needed before it lands
    /// builds it itself (`caretMark`), and a newer DONE makes it stale.
    func prepareCaretLines(path: String, text: String?) {
        guard let text, view != nil else { return }
        let stamp = compiledStamp, ref = EngineV3WeakRef(self)
        Self.caretLinesQueue.async {
            let ns = text as NSString
            let table = EngineV3CaretPlace.LineTable(ns), length = ns.length
            EngineV3Session.onMain {
                guard let self = ref.value, self.compiledStamp == stamp,
                      self.caretLines.map({ $0.path != path || $0.stamp != stamp }) ?? true else { return }
                self.caretLines = (path, stamp, length, table)
            }
        }
    }

    static let caretLinesQueue = DispatchQueue(label: "flashtex.engine-v3.caret-lines", qos: .userInitiated)

    /// The caret moved, the text changed or a compile landed: the pages
    /// view works the mark out again, without any SwiftUI view reading the
    /// caret. A caret move alone (arrows, a click) is marked on the next
    /// run-loop turn. While typing, and after a compile, the mark waits for
    /// `settle` (150 ms) and is worked out once for all that came in: each
    /// keystroke's compile replaces the caret's page, and its glyph index
    /// would otherwise be rebuilt on every key (0.9 ms per key, measured).
    func scheduleCaretMark(afterCompile: Bool = false) {
        guard view != nil else { return }
        let typing = afterCompile || (model?.editorRevision ?? 0) != caretMarkedRevision
        // Each path coalesces on its own: a settling edit never holds a caret move back.
        if typing { settleCaretMark(); return }
        guard !caretMarkScheduled else { return }
        caretMarkScheduled = true
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.caretMarkScheduled = false
            // Typing waits for the settle too: NSTextView moves the selection
            // before it reports the text change (insertText, deleteBackward),
            // so a typed key arrives here as a caret move, and the edit is
            // seen only now. So does a page installed since the last mark
            // (its glyph index is rebuilt on the next lookup): the key's
            // compile streams the caret's page in after nearly every key.
            let typed = (self.model?.editorRevision ?? 0) != self.caretMarkedRevision
            if typed || self.pageInstalls != self.caretMarkedInstalls { self.settleCaretMark() } else { self.markCaretNow() }
        }
    }

    /// A throttle, not a debounce: during continuous typing the mark moves
    /// once every `caretSettle`, where a debounce would leave it behind for
    /// as long as the typing goes on (the bar would stand at the place the
    /// typing began until a pause), and costs the same at most once per period.
    private func settleCaretMark() {
        guard !caretMarkSettling else { return }
        caretMarkSettling = true
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.caretSettle) { [weak self] in
            self?.caretMarkSettling = false
            self?.markCaretNow()
        }
    }

    private func markCaretNow() {
        caretMarkedRevision = model?.editorRevision ?? 0
        caretMarkedInstalls = pageInstalls
        view?.refreshCaret() // nothing when its inputs are unchanged (CaretKey)
    }

    static let caretSettle: TimeInterval = 0.15
}

extension EngineV3PagesView {
    /// The mark for the model's caret now (EngineV3Session.scheduleCaretMark).
    func refreshCaret() {
        guard let session, let model = session.model else { return }
        setCaret(path: model.engineV3Enabled ? model.activePath : nil, utf16: model.caretUTF16, stamp: session.contentStamp)
    }

    /// The caret moved, or the pages changed: works out the mark again
    /// (only when the inputs differ) and draws it.
    func setCaret(path: String?, utf16: Int, stamp: Int) {
        let model = session?.model
        let key = CaretKey(path: path, utf16: utf16, stamp: stamp, pages: heldPageIndexes, revision: model?.editorRevision ?? 0)
        guard key != caretKey else { return }
        caretKey = key
        var mark: EngineV3CaretMark?
        if let path, let session, let model, path == model.activePath {
            let current = model.activeText
            mark = session.caretMark(path: path, caret: utf16, current: current,
                                     compiled: model.compiledDocuments[path] ?? current, pages: heldPageIndexes)
        }
        if mark != caretMark { caretMark = mark; drawCaretMark() }
    }

    struct CaretKey: Equatable { var path: String?; var utf16: Int; var stamp: Int; var pages: [Int]; var revision: Int }

    /// Draws `caretMark` at the current layout (also after a relayout).
    func drawCaretMark() {
        wantsLayer = true
        CATransaction.begin(); CATransaction.setDisableActions(true)
        defer { CATransaction.commit() }
        let layer = caretMarkLayer
        if layer.superlayer == nil { self.layer?.addSublayer(layer) }
        guard let m = caretMark, let origin = viewPoint(page: m.page, .zero), let s = viewPoint(page: m.page, CGPoint(x: 1, y: 0)).map({ $0.x - origin.x }) else {
            layer.isHidden = true
            return
        }
        func view(_ r: CGRect) -> CGRect { CGRect(x: origin.x + r.minX * s, y: origin.y + r.minY * s, width: r.width * s, height: r.height * s) }
        let dark = pageAppearance == .dark
        let accent = NSColor.controlAccentColor
        let band = CGMutablePath()
        let overhang = DS.Preview.paragraphBandOverhang * s
        for row in m.band { band.addRoundedRect(in: view(row).insetBy(dx: -overhang, dy: -overhang / 2), cornerWidth: 2, cornerHeight: 2) }
        layer.band.path = band
        layer.band.fillColor = accent.withAlphaComponent(dark ? DS.Preview.paragraphBandOpacityDark : DS.Preview.paragraphBandOpacity).cgColor
        layer.glyph.path = m.glyph.map { CGPath(rect: view($0), transform: nil) }
        layer.glyph.fillColor = accent.withAlphaComponent(DS.Preview.hoverHighlightOpacity).cgColor
        layer.bar.path = m.bar.map { b in CGPath(rect: view(b).insetBy(dx: -0.75, dy: 0), transform: nil) }
        layer.bar.fillColor = accent.cgColor
        layer.frame = bounds
        for l in [layer.band, layer.glyph, layer.bar] { l.frame = layer.bounds }
        layer.isHidden = false
    }
}

/// The caret mark's layers, above the pages (document coordinates).
final class EngineV3CaretMarkLayer: CALayer {
    let band = CAShapeLayer(), glyph = CAShapeLayer(), bar = CAShapeLayer()
    override init() {
        super.init()
        zPosition = 8
        for l in [band, glyph, bar] { addSublayer(l) }
    }
    override init(layer: Any) { super.init(layer: layer) }
    required init?(coder: NSCoder) { fatalError() }
}
