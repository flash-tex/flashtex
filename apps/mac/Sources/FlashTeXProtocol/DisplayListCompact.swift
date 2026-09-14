import Foundation

/// `display-list-v2-compact` (protocol/proposals/display-list-v2-compact.md r1):
/// the consumer side of the compact cluster encoding. A payload that carries
/// `"cluster_encoding":"compact-1"` writes every glyph run with
///
/// - `glyphs` as `[gid, origin_x, baseline_y, advance_x, advance_y, cluster]`
///   arrays,
/// - run-level `hit_top` / `hit_height` (the default hit-rect vertical
///   extent), an optional `end_caret` `{text_byte, x}` and an optional
///   run-level `sources` span `[{path, start_byte, end_byte}]`,
/// - clusters as small objects whose every field is an override of a value
///   derived from the glyphs and the run chain (`resolve` below).
///
/// `resolve` reconstructs exactly the `RenderingV2.Cluster` values the full
/// encoding carries (the producer writes an override wherever the derivation
/// would differ from its model; `DisplayListCompactTests` checks the two
/// decodes against the real producer), so hit-testing, caret following,
/// source mapping and the `dl2-canon-1` digests see identical values.
public enum DisplayListCompact {
    public static let capability = "display-list-v2-compact"
    public static let encoding = "compact-1"
    /// The payload key exactly as the writer places it (sorted between
    /// `changed_pages`/`payload{` and `color_space`); part of the fixed
    /// framing of a compact full line (`DisplayListDelta.frameConstantBytes`).
    public static let encodingKeyBytes = "\"cluster_encoding\":\"compact-1\",".utf8.count
    /// Encodings this consumer can decode; any other value is refused.
    public static let knownEncodings: Set<String> = [encoding]

    public struct Error: Swift.Error, CustomStringConvertible, Equatable {
        public var message: String
        public var description: String { "compact cluster encoding: \(message)" }
    }

    /// One compact cluster object as read off the wire (every field optional).
    public struct RawCluster: Equatable {
        /// `c`: explicit carets `[[text_byte, x, top, height], …]`.
        public var carets: [RenderingV2.Caret]?
        /// `e`: source range length when not the text length.
        public var sourceLength: Int?
        /// `h`: explicit hit rect `[x, top, width, height]`.
        public var hitRect: RenderingV2.Rect?
        /// `hv`: `[top, height]` override of the run default.
        public var hitVertical: (top: Int64, height: Int64)?
        /// `l`: text byte length when not one UTF-8 scalar.
        public var textLength: Int?
        /// `s`: source start delta from the run chain.
        public var sourceDelta: Int?
        /// `sources` / `synthetic_reason`: explicit provenance.
        public var sources: [RenderingV2.SourceRange]?
        public var syntheticReason: String?
        public var hasExplicitProvenance = false
        /// `ts`: text start when not the previous cluster's end.
        public var textStart: Int?
        public init() {}
        public static func == (a: RawCluster, b: RawCluster) -> Bool {
            a.carets == b.carets && a.sourceLength == b.sourceLength && a.hitRect == b.hitRect
                && a.hitVertical?.top == b.hitVertical?.top && a.hitVertical?.height == b.hitVertical?.height
                && a.textLength == b.textLength && a.sourceDelta == b.sourceDelta && a.sources == b.sources
                && a.syntheticReason == b.syntheticReason && a.hasExplicitProvenance == b.hasExplicitProvenance && a.textStart == b.textStart
        }
    }

    /// The run-level compact fields.
    public struct RunHeader: Equatable {
        public var hitTop: Int64?
        public var hitHeight: Int64?
        public var endCaret: (textByte: Int, x: Int64)?
        public var sources: [RenderingV2.SourceRange]?
        public init(hitTop: Int64? = nil, hitHeight: Int64? = nil, endCaret: (textByte: Int, x: Int64)? = nil, sources: [RenderingV2.SourceRange]? = nil) {
            self.hitTop = hitTop; self.hitHeight = hitHeight; self.endCaret = endCaret; self.sources = sources
        }
        public static func == (a: RunHeader, b: RunHeader) -> Bool {
            a.hitTop == b.hitTop && a.hitHeight == b.hitHeight && a.endCaret?.textByte == b.endCaret?.textByte && a.endCaret?.x == b.endCaret?.x && a.sources == b.sources
        }
    }

    /// The UTF-8 length of the scalar starting at byte `at` of `text`
    /// (0 when `at` is outside the text or not on a scalar boundary).
    public static func scalarLength(_ text: [UInt8], at: Int) -> Int {
        guard at >= 0, at < text.count else { return 0 }
        let b = text[at]
        if b < 0x80 { return 1 }
        if b & 0xE0 == 0xC0 { return 2 }
        if b & 0xF0 == 0xE0 { return 3 }
        if b & 0xF8 == 0xF0 { return 4 }
        return 0 // a continuation byte: not a boundary
    }

    /// Derives the full clusters of one run from its compact form. The
    /// derivation (per cluster, in order; `prevTextEnd` starts at 0 and
    /// `prevSourceEnd` at the run span's `start_byte`):
    ///
    ///     text_start = ts ?? prevTextEnd;  text_end = text_start + (l ?? scalarLength(text, text_start))
    ///     hit_rect   = h ?? { x: first glyph of the cluster's origin_x (0 without glyphs),
    ///                         top: hv.top ?? hit_top, width: Σ advance_x of the cluster's glyphs,
    ///                         height: hv.height ?? hit_height }
    ///     carets     = c ?? [ {text_start, hit.x, hit.top, hit.height} ]
    ///                       + (last cluster && end_caret ? [ {end_caret.text_byte, end_caret.x, hit.top, hit.height} ] : [])
    ///     provenance = sources / synthetic_reason if present (chain untouched), else
    ///                  one range {run path, start: prevSourceEnd + (s ?? 0), end: start + (e ?? text length)};
    ///                  prevSourceEnd = end
    public static func resolve(_ raw: [RawCluster], header: RunHeader, glyphs: [RenderingV2.Glyph], text: String) throws -> [RenderingV2.Cluster] {
        let bytes = Array(text.utf8)
        // Cluster → (first glyph origin, advance sum), in glyph-array order.
        var firstX: [Int64?] = Array(repeating: nil, count: raw.count)
        var widths: [Int64] = Array(repeating: 0, count: raw.count)
        for g in glyphs where g.cluster >= 0 && g.cluster < raw.count {
            if firstX[g.cluster] == nil { firstX[g.cluster] = g.originX }
            let (w, o) = widths[g.cluster].addingReportingOverflow(g.advanceX)
            guard !o else { throw Error(message: "cluster \(g.cluster): advance sum overflows") }
            widths[g.cluster] = w
        }
        let span = header.sources?.first
        if let s = header.sources, s.count != 1 { throw Error(message: "run sources must carry exactly one span (found \(s.count))") }
        var out: [RenderingV2.Cluster] = []
        out.reserveCapacity(raw.count)
        var prevTextEnd = 0
        var prevSourceEnd = span?.startByte ?? 0
        for (i, c) in raw.enumerated() {
            let textStart = c.textStart ?? prevTextEnd
            let textLength = c.textLength ?? scalarLength(bytes, at: textStart)
            guard textStart >= 0, textLength >= 0 else { throw Error(message: "cluster \(i): negative text range") }
            let (textEnd, o1) = textStart.addingReportingOverflow(textLength)
            guard !o1 else { throw Error(message: "cluster \(i): text range overflows") }
            prevTextEnd = textEnd
            let rect: RenderingV2.Rect
            if let h = c.hitRect {
                rect = h
            } else {
                let top: Int64, height: Int64
                if let hv = c.hitVertical { top = hv.top; height = hv.height } else {
                    guard let ht = header.hitTop, let hh = header.hitHeight else { throw Error(message: "cluster \(i): no hit_top/hit_height on the run and no override") }
                    top = ht; height = hh
                }
                rect = RenderingV2.Rect(x: firstX[i] ?? 0, top: top, width: widths[i], height: height)
            }
            let carets: [RenderingV2.Caret]
            if let explicit = c.carets {
                carets = explicit
            } else {
                var ks = [RenderingV2.Caret(textByte: textStart, x: rect.x, top: rect.top, height: rect.height)]
                if i + 1 == raw.count, let e = header.endCaret {
                    ks.append(RenderingV2.Caret(textByte: e.textByte, x: e.x, top: rect.top, height: rect.height))
                }
                carets = ks
            }
            var sources: [RenderingV2.SourceRange]? = nil
            var synthetic: String? = nil
            if c.hasExplicitProvenance {
                sources = c.sources
                synthetic = c.syntheticReason
            } else {
                guard let span else { throw Error(message: "cluster \(i): implicit provenance in a run without a sources span") }
                let (start, o2) = prevSourceEnd.addingReportingOverflow(c.sourceDelta ?? 0)
                let (end, o3) = start.addingReportingOverflow(c.sourceLength ?? textLength)
                guard !o2, !o3, start >= 0, end >= start else { throw Error(message: "cluster \(i): source range out of range") }
                prevSourceEnd = end
                sources = [RenderingV2.SourceRange(path: span.path, startByte: start, endByte: end)]
            }
            out.append(RenderingV2.Cluster(textStartByte: textStart, textEndByte: textEnd, hitRects: [rect], carets: carets, sources: sources, syntheticReason: synthetic))
        }
        return out
    }

    // MARK: model predicates shared with the delta accounting

    /// The run's source path: that of the first range of the first cluster
    /// that has one.
    public static func runPath(_ run: RenderingV2.GlyphRun) -> String? {
        for c in run.clusters { if let s = c.sources?.first { return s.path } }
        return nil
    }

    /// Whether a cluster is encoded implicitly against the run chain (exactly
    /// one range, on the run path).
    public static func isImplicit(_ cluster: RenderingV2.Cluster, runPath: String) -> Bool {
        guard let s = cluster.sources, s.count == 1 else { return false }
        return s[0].path == runPath
    }

    /// The run-level `sources` span `(start, end)` over the implicit clusters.
    public static func runSpan(_ run: RenderingV2.GlyphRun) -> (path: String, start: Int, end: Int)? {
        guard let path = runPath(run) else { return nil }
        var span: (Int, Int)? = nil
        for c in run.clusters where isImplicit(c, runPath: path) {
            let s = c.sources![0]
            span = span.map { (min($0.0, s.startByte), max($0.1, s.endByte)) } ?? (s.startByte, s.endByte)
        }
        return span.map { (path, $0.0, $0.1) }
    }
}
