import AppKit
import CoreText
import FlashTeXProtocol

/// Cached CoreText lines for the v1 preview. Resolving a SwiftUI `Text` per
/// item inside `Canvas` cost ~0.3 ms each (demo.tex: 968 items ≈ 300 ms per
/// keystroke on the main thread); a `CTLine` keyed by (string, face, size,
/// font-resource generation) is built once per distinct run and survives across
/// compile revisions, so an unchanged paragraph costs only its draw call. Colour
/// is taken from the graphics context (`kCTForegroundColorFromContextAttributeName`),
/// so light and dark previews share one entry. Main-thread only (the draw
/// closure runs on the main thread and the cache is not synchronized).
///
/// Identity and invalidation: a key is immutable and complete. The same
/// PostScript name can map to a different font once the resource set changes
/// (`LMRoman10-Regular` before and after Latin Modern registration; the legacy
/// face after a producer switch between `flashtex-compiler` and
/// `flashtex-render`), so every key carries `PreviewFonts.resourceGeneration`
/// and both stores retire wholesale when the generation moves — never ad hoc
/// `clear()` calls scattered through the shell. Within a generation retention
/// is least-recently-used with independent bounds for lines and fonts.
final class PreviewTextCache {
    nonisolated(unsafe) static let shared = PreviewTextCache()

    struct Key: Hashable {
        var text: String
        var postScriptName: String
        /// Exact font size in screen points (bit pattern, so 10.000 and 10.004 pt
        /// are distinct entries and the CTFont is built at the size asked for;
        /// no quantization — geometry must stay exact for oracle comparison).
        var sizeBits: UInt64
        /// `PreviewFonts.resourceGeneration` the key was made at. Two keys that
        /// differ only here are different entries; the cache never serves an
        /// entry built at another generation.
        var generation: UInt64
        var size: CGFloat { CGFloat(Double(bitPattern: sizeBits)) }

        init(text: String, postScriptName: String, sizeBits: UInt64,
             generation: UInt64 = PreviewFonts.resourceGeneration) {
            self.text = text; self.postScriptName = postScriptName
            self.sizeBits = sizeBits; self.generation = generation
        }
    }

    struct Line {
        var line: CTLine
        var ascent: CGFloat
        var descent: CGFloat
        var width: CGFloat
    }

    struct FontKey: Hashable { var postScriptName: String; var sizeBits: UInt64 }

    /// Bound on cached lines (least-recently-used eviction past this).
    var capacity: Int {
        get { lines.capacity }
        set { lines.capacity = newValue }
    }
    /// Bound on cached fonts (one per distinct face × exact size), evicted
    /// independently of lines: a `CTLine` retains its own font, so dropping a
    /// font entry never invalidates a line.
    var fontCapacity: Int {
        get { fonts.capacity }
        set { fonts.capacity = newValue }
    }

    private let lines: LRUCache<Key, Line>
    private let fonts: LRUCache<FontKey, CTFont>
    /// Generation the stores currently hold entries for.
    private(set) var generation: UInt64

    private(set) var hits = 0, misses = 0
    /// Lines dropped by the LRU bound (not by generation change).
    private(set) var lineEvictions = 0
    /// Fonts dropped by the LRU bound.
    private(set) var fontEvictions = 0
    /// Number of times the stores were retired because the resource generation moved.
    private(set) var generationRetirements = 0
    /// Fonts actually created with `CTFontCreateWithName` (font-cache misses).
    private(set) var fontsBuilt = 0

    init(capacity: Int = 20_000, fontCapacity: Int = 512) {
        lines = LRUCache(capacity: capacity)
        fonts = LRUCache(capacity: fontCapacity)
        generation = PreviewFonts.resourceGeneration
    }

    var count: Int { lines.count }
    var fontCount: Int { fonts.count }

    /// Drops every entry and resets counters. Kept for callers that want an
    /// explicit reset (tests, bench); resource changes do not need it.
    func clear() {
        lines.removeAll(); fonts.removeAll()
        hits = 0; misses = 0; lineEvictions = 0; fontEvictions = 0; fontsBuilt = 0
        generation = PreviewFonts.resourceGeneration
    }

    static func key(text: String, postScriptName: String, size: CGFloat) -> Key {
        Key(text: text, postScriptName: postScriptName, sizeBits: Double(size).bitPattern)
    }

    /// Retires both stores if the font resource set moved since they were
    /// filled. Every held entry was built against the old set, so none is
    /// reachable through a current key; keeping them would only occupy the
    /// bound. One integer compare per lookup.
    @discardableResult
    func syncGeneration() -> Bool {
        let current = PreviewFonts.resourceGeneration
        guard current != generation else { return false }
        lines.removeAll(); fonts.removeAll()
        generation = current
        generationRetirements += 1
        return true
    }

    /// The CTFont for a face at an exact size (`CTFontGetSize` == `size`),
    /// resolved against the current resource generation.
    func font(_ name: String, size: CGFloat) -> CTFont {
        syncGeneration()
        let id = FontKey(postScriptName: name, sizeBits: Double(size).bitPattern)
        if let f = fonts.value(for: id) { return f }
        let f = CTFontCreateWithName(name as CFString, size, nil)
        fontsBuilt += 1
        if fonts.insert(f, for: id) != nil { fontEvictions += 1 }
        return f
    }

    /// The cached line for `key`. A key from another generation is a miss that
    /// builds against the current resource set and is stored under the
    /// current generation; the stale key itself is never inserted.
    func line(for key: Key) -> Line {
        syncGeneration()
        var key = key
        if key.generation != generation { key.generation = generation }
        if let l = lines.value(for: key) { hits += 1; return l }
        misses += 1
        let l = Self.build(text: key.text, font: font(key.postScriptName, size: key.size))
        if lines.insert(l, for: key) != nil { lineEvictions += 1 }
        return l
    }

    /// The full construction cost of one line: attributed string, `CTLine`,
    /// typographic bounds. Shared by the cache miss path and the uncached
    /// baseline of `measure` so both pay exactly the same work.
    static func build(text: String, font: CTFont) -> Line {
        let attributed = NSAttributedString(string: text, attributes: [
            .font: font,
            kCTForegroundColorFromContextAttributeName as NSAttributedString.Key: true,
        ])
        let ct = CTLineCreateWithAttributedString(attributed)
        var ascent: CGFloat = 0, descent: CGFloat = 0
        let width = CGFloat(CTLineGetTypographicBounds(ct, &ascent, &descent, nil))
        return Line(line: ct, ascent: ascent, descent: descent, width: width)
    }

    /// Screen rect of a text item drawn at `scale`, from the cached metrics.
    func rect(for t: RuntimeV1.PageItem.TextItem, scale: CGFloat) -> CGRect {
        let name = PreviewFonts.resolve(hint: t.font, size: t.fontSizePt).postScriptName
        let l = line(for: Self.key(text: t.text, postScriptName: name, size: t.fontSizePt * scale))
        return CGRect(x: t.xPt * scale, y: t.baselineYPt * scale - l.ascent, width: l.width, height: l.ascent + l.descent)
    }

    // MARK: Test/evidence access

    /// Whether `key` is held right now, without touching recency.
    func contains(_ key: Key) -> Bool { lines.peek(key) != nil }
    /// Cached line keys, most recently used first.
    var lineKeysMostRecentFirst: [Key] { lines.keysMostRecentFirst }
    /// Cached font keys, most recently used first.
    var fontKeysMostRecentFirst: [FontKey] { fonts.keysMostRecentFirst }

    // MARK: Workload measurement

    /// Line-construction work the paint loop pays for a set of pages, measured
    /// three ways with no approximation of the work itself:
    /// - `uncached`: every text item built from scratch (`CTFontCreateWithName`
    ///   + attributed string + `CTLineCreateWithAttributedString` + bounds),
    ///   what the draw closure paid before the cache;
    /// - `cold`: a fresh cache filled by one pass over the items (misses for
    ///   every distinct key, hits for repeats within the pass);
    /// - `warm`: the second pass over the same items (all hits).
    /// Times are monotonic wall-clock nanoseconds of this process; counts are
    /// deterministic for a given result and scale.
    struct Workload: CustomStringConvertible {
        var items = 0
        var distinctKeys = 0
        var fontsBuilt = 0
        var coldHits = 0, coldMisses = 0
        var warmHits = 0, warmMisses = 0
        var uncachedNs: UInt64 = 0
        var coldNs: UInt64 = 0
        var warmNs: UInt64 = 0
        /// Whether the summed widths of the three passes were bit-identical
        /// (same construction, same fonts, same order of summation).
        var metricsIdentical = false

        func perItemUs(_ ns: UInt64) -> Double { items == 0 ? 0 : Double(ns) / Double(items) / 1_000 }
        var description: String {
            let f = { (ns: UInt64) in String(format: "%.3f ms (%.2f µs/item)", Double(ns) / 1e6, self.perItemUs(ns)) }
            return "items=\(items) distinct=\(distinctKeys) fonts=\(fontsBuilt) "
                + "uncached=\(f(uncachedNs)) cold=\(f(coldNs)) [\(coldHits) hits/\(coldMisses) misses] "
                + "warm=\(f(warmNs)) [\(warmHits) hits/\(warmMisses) misses]"
        }
    }

    /// Measures `pages` at `scale` on a fresh cache with the given bounds. The
    /// cache is returned filled so callers can check the lines it holds against
    /// the uncached construction (`Workload` does not hide any of the work).
    static func measure(pages: [RuntimeV1.Page], scale: CGFloat,
                        capacity: Int = 20_000, fontCapacity: Int = 512) -> (workload: Workload, cache: PreviewTextCache) {
        var keys: [Key] = []
        for page in pages {
            for case .text(let t) in page.items {
                let name = PreviewFonts.resolve(hint: t.font, size: t.fontSizePt).postScriptName
                keys.append(key(text: t.text, postScriptName: name, size: t.fontSizePt * scale))
            }
        }
        var w = Workload()
        w.items = keys.count
        w.distinctKeys = Set(keys).count

        let uncachedStart = DispatchTime.now().uptimeNanoseconds
        var uncachedWidth: CGFloat = 0
        for k in keys {
            let l = build(text: k.text, font: CTFontCreateWithName(k.postScriptName as CFString, k.size, nil))
            uncachedWidth += l.width
        }
        w.uncachedNs = DispatchTime.now().uptimeNanoseconds - uncachedStart

        let cache = PreviewTextCache(capacity: capacity, fontCapacity: fontCapacity)
        let coldStart = DispatchTime.now().uptimeNanoseconds
        var coldWidth: CGFloat = 0
        for k in keys { coldWidth += cache.line(for: k).width }
        w.coldNs = DispatchTime.now().uptimeNanoseconds - coldStart
        w.coldHits = cache.hits; w.coldMisses = cache.misses; w.fontsBuilt = cache.fontsBuilt

        let warmStart = DispatchTime.now().uptimeNanoseconds
        var warmWidth: CGFloat = 0
        for k in keys { warmWidth += cache.line(for: k).width }
        w.warmNs = DispatchTime.now().uptimeNanoseconds - warmStart
        w.warmHits = cache.hits - w.coldHits; w.warmMisses = cache.misses - w.coldMisses
        w.metricsIdentical = uncachedWidth == coldWidth && coldWidth == warmWidth
        return (w, cache)
    }
}

/// Bounded least-recently-used map: a dictionary onto nodes of a doubly linked
/// list ordered most-recent first. Lookup, insert and eviction are O(1). Not
/// synchronized. Node links are strong and unlinked on removal (and in
/// `deinit`), so the list never leaks through its own cycles.
final class LRUCache<Key: Hashable, Value> {
    final class Node {
        let key: Key
        var value: Value
        var prev: Node?
        var next: Node?
        init(key: Key, value: Value) { self.key = key; self.value = value }
    }

    private var map: [Key: Node] = [:]
    private var head: Node? // most recently used
    private var tail: Node? // least recently used

    /// Maximum entries held. Lowering it evicts immediately.
    var capacity: Int {
        didSet { trim() }
    }

    init(capacity: Int) {
        precondition(capacity >= 1, "LRUCache needs room for at least one entry")
        self.capacity = capacity
    }

    deinit { removeAll() }

    var count: Int { map.count }

    /// The value for `key`, marking it most recently used.
    func value(for key: Key) -> Value? {
        guard let node = map[key] else { return nil }
        moveToFront(node)
        return node.value
    }

    /// The value for `key` without touching recency.
    func peek(_ key: Key) -> Value? { map[key]?.value }

    /// Inserts or replaces `value` as most recently used. Returns the key of the
    /// least-recently-used entry that was evicted to stay within `capacity`.
    @discardableResult
    func insert(_ value: Value, for key: Key) -> Key? {
        if let node = map[key] {
            node.value = value
            moveToFront(node)
            return nil
        }
        let node = Node(key: key, value: value)
        map[key] = node
        pushFront(node)
        guard map.count > capacity, let lru = tail else { return nil }
        unlink(lru)
        map[lru.key] = nil
        return lru.key
    }

    func removeAll() {
        var node = head
        while let n = node {
            node = n.next
            n.prev = nil; n.next = nil
        }
        head = nil; tail = nil
        map.removeAll(keepingCapacity: true)
    }

    /// Keys from most to least recently used.
    var keysMostRecentFirst: [Key] {
        var out: [Key] = []
        out.reserveCapacity(map.count)
        var node = head
        while let n = node { out.append(n.key); node = n.next }
        return out
    }

    private func trim() {
        while map.count > capacity, let lru = tail {
            unlink(lru)
            map[lru.key] = nil
        }
    }

    private func pushFront(_ node: Node) {
        node.next = head
        node.prev = nil
        head?.prev = node
        head = node
        if tail == nil { tail = node }
    }

    private func unlink(_ node: Node) {
        if let p = node.prev { p.next = node.next } else { head = node.next }
        if let n = node.next { n.prev = node.prev } else { tail = node.prev }
        node.prev = nil; node.next = nil
    }

    private func moveToFront(_ node: Node) {
        guard head !== node else { return }
        unlink(node)
        pushFront(node)
    }
}

/// Where each item of a page lands on screen, for hover, click and caret
/// highlights. A pure function of the page and scale (through the cache), so
/// the draw closure never has to publish state back into the view.
struct PreviewHitRects {
    struct Hit { var index: Int; var rect: CGRect; var source: RuntimeV1.SourceRange?; var text: String? }

    static func compute(page: RuntimeV1.Page, scale: CGFloat, rulesNegotiated: Bool,
                        cache: PreviewTextCache = .shared) -> [Hit] {
        var out: [Hit] = []
        out.reserveCapacity(page.items.count)
        for (index, item) in page.items.enumerated() {
            switch item {
            case .rule(let rule):
                out.append(Hit(index: index, rect: RuleGeometry.previewRect(rule, scale: scale), source: rule.source, text: nil))
            case .text(let t):
                if !rulesNegotiated, let r = RuleConvention.rect(for: t) {
                    let rect = CGRect(x: r.x * scale, y: r.y * scale, width: r.width * scale, height: max(0.5, r.height * scale))
                    out.append(Hit(index: index, rect: rect, source: t.source, text: t.text))
                } else {
                    out.append(Hit(index: index, rect: cache.rect(for: t, scale: scale), source: t.source, text: t.text))
                }
            default:
                continue
            }
        }
        return out
    }

    static func hit(_ hits: [Hit], at p: CGPoint) -> Hit? { hits.first { $0.rect.contains(p) } }
}
