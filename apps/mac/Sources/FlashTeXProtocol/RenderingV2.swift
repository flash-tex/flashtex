import Foundation

/// Codable model plus a fail-closed validator for the EXPERIMENTAL rendering-v2
/// `display_list` envelope (`protocol/rendering-v2.schema.json` and
/// `docs/contracts/rendering-v2-proposal.md` on main; not a negotiated production
/// wire — runtime-v1 remains authoritative). Shapes follow the schema; the fields
/// below are what `flashtex-render --v2` (crates/render-pipeline) actually emits.
///
/// Coordinates are `bp_2pow20` ticks: signed integers, 1,048,576 per PDF point,
/// origin at the page's top-left, y down. Glyph origins are absolute baseline
/// origins; advances are never re-added. Glyph IDs index the ORIGINAL font's
/// glyph order. Every cluster is an end-exclusive UTF-8 byte range of the run's
/// logical `text` with its own hit rectangles, carets and source provenance.
///
/// Documented deviations between the schema and the pipeline that this model
/// accepts (each one is explicit here, never silently widened):
/// - `fonts[].format`: schema allows only `static-truetype`; the pipeline emits
///   `opentype-cff` (Latin Modern) and `core14-afm` (Times metrics, no program
///   bytes). The model decodes all three; only `static-truetype`/`opentype-cff`
///   are paintable — a run that references a `core14-afm` font fails resolution.
/// - `fonts[].byte_length`: schema minimum 1; `core14-afm` entries carry 0.
/// - `fonts[].sha256`/`font_id`: SHA-256 of the raw program bytes, exactly as
///   the schema and the draft contract (runtime-v1-display-list-v2.md L55–57)
///   say; the producer emits raw-byte digests for OTF faces. The historical
///   SHA-256(bytes ‖ face_index) engine identifier is NOT a resource digest
///   and is refused as an unknown hash by `GlyphRunRenderer`'s font store
///   (never tolerated as an alias); the model only checks the format.
/// - Unknown JSON keys are ignored by the decoder (Swift `Codable`), where the
///   schema says `additionalProperties: false`. Unknown `kind` values, unknown
///   `required_features`, unknown protocol versions and message types are
///   rejected.
/// - `path_fill` / `path_stroke` items (proposal `path-v0`, TikZ) are accepted
///   with `clips`; see `pathFeatures`.
/// - The schema does not require clusters to partition the run text or source
///   paths to name a declared document; this validator requires both (as
///   crates/rendering-core does) because hit-testing depends on them. It also
///   applies rendering-core's other structural rules: every cluster is
///   referenced by at least one glyph, source ranges lie within the declared
///   document byte length, used features (`glyph_run`, `rule`, `rgba-srgb`,
///   `cluster-actualtext`) are declared in `required_features`, all ticks and
///   tick sums stay within ±(2^53−1), and collection sizes are bounded
///   (`Bounds`). `static-truetype` is not derived from glyph runs (see above).
public enum RenderingV2 {
    public static let protocolVersion = 2
    public static let messageType = "display_list"
    public static let renderFormat = "display-list-v2"
    public static let coordinateUnit = "bp_2pow20"
    public static let colorSpace = "srgb"
    public static let textExtraction = "cluster-actualtext"
    public static let ticksPerPoint: Int64 = 1 << 20
    /// Largest integer JSON carries exactly (2^53 − 1); every tick, revision and
    /// byte count must stay within ±this, and tick sums are checked, as
    /// crates/rendering-core's `Tick::validate`/`checked_add` do.
    public static let maxExactInteger: Int64 = (1 << 53) - 1
    /// Bounded collection sizes (crates/rendering-core `validate`): a list that
    /// exceeds one is refused, never truncated.
    public enum Bounds {
        public static let documents = 1...4096
        public static let fonts = 0...256
        public static let pages = 0...10000
        public static let pageItems = 0...100000
        public static let diagnostics = 0...10000
        public static let runTextBytes = 1...1_048_576
        public static let glyphs = 1...65536
        public static let clusters = 1...65536
        public static let hitRects = 1...128
        public static let carets = 0...128
        public static let sourceRanges = 1...128
        public static let diagnosticSources = 0...128
        public static let diagnosticMessageBytes = 1...4096
        public static let syntheticReasonBytes = 1...1024
        public static let postscriptNameBytes = 1...256
        public static let fontByteLength: ClosedRange<Int64> = 1...67_108_864
        public static let documentByteLength: ClosedRange<Int64> = 0...8_388_608
        public static let pathCommands = 1...65536
        public static let clips = 0...16
        public static let dashEntries = 1...32
    }
    /// Features this consumer understands (schema `feature` enum, plus
    /// `image` from the negotiated `display-list-v2-images` proposal —
    /// protocol/proposals/display-list-v2-image.md; a producer only emits it
    /// when the request listed `imagesCapability`).
    public static let knownFeatures: Set<String> = ["glyph_run", "rule", "static-truetype", "rgba-srgb", "cluster-actualtext", "image", "path_fill", "path_stroke", "clip"]
    /// Vector path items (proposal `path-v0`, crates/render-pipeline `display.rs`
    /// `PathItem`; TikZ pictures). Emitted whenever `display-list-v2` is
    /// negotiated — there is no separate capability — as `kind: "path_fill"`
    /// (with `fill_rule`) or `kind: "path_stroke"` (with `stroke`); each may
    /// carry `clips` (`kind: "path"` entries with their own `fill_rule`), which
    /// adds the `clip` feature. Commands are `["m",x,y]`, `["l",x,y]`,
    /// `["c",x1,y1,x2,y2,x,y]`, `["z"]` in page ticks (top-left, y down); no
    /// transform is on the wire. Bounds: `Bounds.pathCommands` per path or
    /// clip, `Bounds.clips` per item, `Bounds.dashEntries` per stroke.
    public static let pathFeatures: Set<String> = ["path_fill", "path_stroke", "clip"]
    /// Layout capability that lets the `display_list` line carry `image`
    /// items (accepted only alongside `display-list-v2`).
    public static let imagesCapability = "display-list-v2-images"
    /// Layout capability that lets each diagnostic carry optional
    /// `suggestion` / `labels` / `notes` / `help` (proposal
    /// `display-list-v2-diagnostics`; accepted only alongside `display-list-v2`).
    public static let diagnosticsCapability = "display-list-v2-diagnostics"
    /// Layout capability that lets the `display_list` line carry a top-level
    /// `navigation` object (protocol/proposals/display-list-v2-links.md).
    /// Accepted only alongside `display-list-v2`. `required_features` is not
    /// extended: a consumer that ignores `navigation` still paints the page.
    public static let linksCapability = "display-list-v2-links"
    /// Image formats the consumer can paint (proposal §3).
    public static let imageFormats: Set<String> = ["png", "jpeg", "pdf"]
    /// Upper bound on an image resource's byte length (bytes are read from
    /// the project, so this bounds one read and one cache entry).
    public static let maxImageByteLength: Int64 = 256 << 20
    /// Font formats whose bytes this consumer can paint from.
    public static let paintableFontFormats: Set<String> = ["static-truetype", "opentype-cff"]
    /// Formats the pipeline may declare that carry no program (never paintable).
    public static let metricsOnlyFontFormats: Set<String> = ["core14-afm"]

    public typealias SourceRange = RuntimeV1.SourceRange

    /// Ticks → PDF points (exact for |ticks| < 2^53).
    public static func points(_ ticks: Int64) -> Double { Double(ticks) / Double(ticksPerPoint) }

    public struct Paint: Codable, Hashable {
        public var r: Double, g: Double, b: Double, a: Double
        public init(r: Double, g: Double, b: Double, a: Double) { self.r = r; self.g = g; self.b = b; self.a = a }
        public static let black = Paint(r: 0, g: 0, b: 0, a: 1)
    }

    /// Hit rectangle: top-left anchored, y down, in ticks.
    public struct Rect: Codable, Hashable {
        public var x: Int64, top: Int64, width: Int64, height: Int64
        public init(x: Int64, top: Int64, width: Int64, height: Int64) { self.x = x; self.top = top; self.width = width; self.height = height }
        /// Half-open containment in ticks (`x <= px < x+width`, same for y).
        public func contains(x px: Int64, y py: Int64) -> Bool {
            px >= x && px < x &+ width && py >= top && py < top &+ height
        }
    }

    public struct Caret: Codable, Hashable {
        public var textByte: Int, x: Int64, top: Int64, height: Int64
        enum CodingKeys: String, CodingKey { case textByte = "text_byte", x, top, height }
        public init(textByte: Int, x: Int64, top: Int64, height: Int64) { self.textByte = textByte; self.x = x; self.top = top; self.height = height }
    }

    public struct Cluster: Codable, Equatable {
        public var textStartByte: Int
        public var textEndByte: Int
        public var hitRects: [Rect]
        public var carets: [Caret]
        public var sources: [SourceRange]?
        public var syntheticReason: String?
        enum CodingKeys: String, CodingKey {
            case textStartByte = "text_start_byte", textEndByte = "text_end_byte", hitRects = "hit_rects", carets, sources, syntheticReason = "synthetic_reason"
        }
        public init(textStartByte: Int, textEndByte: Int, hitRects: [Rect], carets: [Caret], sources: [SourceRange]?, syntheticReason: String? = nil) {
            self.textStartByte = textStartByte; self.textEndByte = textEndByte; self.hitRects = hitRects; self.carets = carets
            self.sources = sources; self.syntheticReason = syntheticReason
        }
    }

    public struct Glyph: Codable, Hashable {
        /// Original glyph ID in the run's font (never 0).
        public var gid: Int
        public var originX: Int64, baselineY: Int64, advanceX: Int64, advanceY: Int64
        public var cluster: Int
        enum CodingKeys: String, CodingKey {
            case gid, originX = "origin_x", baselineY = "baseline_y", advanceX = "advance_x", advanceY = "advance_y", cluster
        }
        public init(gid: Int, originX: Int64, baselineY: Int64, advanceX: Int64, advanceY: Int64, cluster: Int) {
            self.gid = gid; self.originX = originX; self.baselineY = baselineY; self.advanceX = advanceX; self.advanceY = advanceY; self.cluster = cluster
        }
    }

    public struct GlyphRun: Codable, Equatable {
        public var fontId: String
        public var fontSize: Int64
        public var text: String
        public var glyphs: [Glyph]
        public var clusters: [Cluster]
        public var paint: Paint
        enum CodingKeys: String, CodingKey { case fontId = "font_id", fontSize = "font_size", text, glyphs, clusters, paint }
        public init(fontId: String, fontSize: Int64, text: String, glyphs: [Glyph], clusters: [Cluster], paint: Paint) {
            self.fontId = fontId; self.fontSize = fontSize; self.text = text; self.glyphs = glyphs; self.clusters = clusters; self.paint = paint
        }
        /// The logical text of `cluster` (its UTF-8 byte range of `text`).
        public func clusterText(_ i: Int) -> String {
            let bytes = Array(text.utf8)
            guard i < clusters.count, clusters[i].textStartByte <= clusters[i].textEndByte, clusters[i].textEndByte <= bytes.count else { return "" }
            return String(decoding: bytes[clusters[i].textStartByte..<clusters[i].textEndByte], as: UTF8.self)
        }
    }

    public struct Rule: Codable, Equatable {
        public var x: Int64, top: Int64, width: Int64, height: Int64
        public var paint: Paint
        public var sources: [SourceRange]?
        public var syntheticReason: String?
        enum CodingKeys: String, CodingKey { case x, top, width, height, paint, sources, syntheticReason = "synthetic_reason" }
        public init(x: Int64, top: Int64, width: Int64, height: Int64, paint: Paint, sources: [SourceRange]?, syntheticReason: String? = nil) {
            self.x = x; self.top = top; self.width = width; self.height = height; self.paint = paint; self.sources = sources; self.syntheticReason = syntheticReason
        }
    }

    /// One `\includegraphics` file as the producer sized it (proposal §3).
    /// Bytes are NOT on the wire: the consumer reads `path` through the rooted
    /// project reader and must refuse bytes whose SHA-256/length differ.
    public struct ImageResource: Codable, Equatable {
        public var imageId: String
        public var sha256: String
        public var byteLength: Int64
        /// `png`, `jpeg` or `pdf`.
        public var format: String
        /// Project-relative, as resolved (extension search applied).
        public var path: String
        /// png/jpeg only.
        public var pixelWidth: Int?
        public var pixelHeight: Int?
        /// pdf only: 1-based page, `[llx, lly, urx, ury]` in points, `/Rotate`.
        public var pdfPage: Int?
        public var pdfBox: [Double]?
        public var pdfRotate: Int?
        enum CodingKeys: String, CodingKey {
            case imageId = "image_id", sha256, byteLength = "byte_length", format, path
            case pixelWidth = "pixel_width", pixelHeight = "pixel_height", pdfPage = "pdf_page", pdfBox = "pdf_box", pdfRotate = "pdf_rotate"
        }
        public init(imageId: String, sha256: String, byteLength: Int64, format: String, path: String,
                    pixelWidth: Int? = nil, pixelHeight: Int? = nil, pdfPage: Int? = nil, pdfBox: [Double]? = nil, pdfRotate: Int? = nil) {
            self.imageId = imageId; self.sha256 = sha256; self.byteLength = byteLength; self.format = format; self.path = path
            self.pixelWidth = pixelWidth; self.pixelHeight = pixelHeight; self.pdfPage = pdfPage; self.pdfBox = pdfBox; self.pdfRotate = pdfRotate
        }
    }

    /// `kind: "image"`: the bounding box on the page (ticks, exact geometry,
    /// clip to it) and the affine `transform` `[a, b, c, d, e, f]` mapping the
    /// image's unit square (u right, v up, origin lower-left) to page points
    /// with y down: `page_x = e + a·u + c·v`, `page_y = f + b·u + d·v`.
    public struct Image: Codable, Equatable {
        public var x: Int64, top: Int64, width: Int64, height: Int64
        public var transform: [Double]
        public var image: ImageResource
        public var sources: [SourceRange]?
        public var syntheticReason: String?
        enum CodingKeys: String, CodingKey { case x, top, width, height, transform, image, sources, syntheticReason = "synthetic_reason" }
        public init(x: Int64, top: Int64, width: Int64, height: Int64, transform: [Double], image: ImageResource, sources: [SourceRange]?, syntheticReason: String? = nil) {
            self.x = x; self.top = top; self.width = width; self.height = height; self.transform = transform; self.image = image
            self.sources = sources; self.syntheticReason = syntheticReason
        }
        /// The unrotated transform for a box at `(x, top, w, h)` points: `[w, 0, 0, -h, x, top + h]`.
        public static func upright(x: Double, top: Double, width: Double, height: Double) -> [Double] { [width, 0, 0, -height, x, top + height] }
    }


    /// One vector path command in page ticks (`path-v0`): `m`/`l` take an
    /// end point, `c` two control points then the end point, `z` closes.
    public enum PathCommand: Codable, Hashable {
        case move(x: Int64, y: Int64)
        case line(x: Int64, y: Int64)
        case cubic(x1: Int64, y1: Int64, x2: Int64, y2: Int64, x: Int64, y: Int64)
        case close

        public init(from decoder: Decoder) throws {
            var c = try decoder.unkeyedContainer()
            let op = try c.decode(String.self)
            func n() throws -> Int64 { try c.decode(Int64.self) }
            switch op {
            case "m": self = .move(x: try n(), y: try n())
            case "l": self = .line(x: try n(), y: try n())
            case "c": self = .cubic(x1: try n(), y1: try n(), x2: try n(), y2: try n(), x: try n(), y: try n())
            case "z": self = .close
            default: throw ValidationError(code: "invalid_display_list", message: "path command '\(op)' is not one of m, l, c, z")
            }
            guard c.isAtEnd else { throw ValidationError(code: "invalid_display_list", message: "path command '\(op)' carries extra operands") }
        }

        public func encode(to encoder: Encoder) throws {
            var c = encoder.unkeyedContainer()
            switch self {
            case .move(let x, let y): try c.encode("m"); try c.encode(x); try c.encode(y)
            case .line(let x, let y): try c.encode("l"); try c.encode(x); try c.encode(y)
            case .cubic(let x1, let y1, let x2, let y2, let x, let y): try c.encode("c"); for v in [x1, y1, x2, y2, x, y] { try c.encode(v) }
            case .close: try c.encode("z")
            }
        }

        /// Every coordinate the command carries (none for `z`).
        public var coordinates: [Int64] {
            switch self {
            case .move(let x, let y), .line(let x, let y): return [x, y]
            case .cubic(let x1, let y1, let x2, let y2, let x, let y): return [x1, y1, x2, y2, x, y]
            case .close: return []
            }
        }
    }

    /// `nonzero` or `evenodd`.
    public enum FillRule: String, Codable, Hashable { case nonzero, evenodd }
    public enum LineCap: String, Codable, Hashable { case butt, round, square }
    public enum LineJoin: String, Codable, Hashable { case miter, round, bevel }

    public struct Dash: Codable, Hashable {
        /// Alternating on/off lengths in ticks (never empty on the wire).
        public var array: [Int64]
        public var phase: Int64
        public init(array: [Int64], phase: Int64) { self.array = array; self.phase = phase }
    }

    public struct Stroke: Codable, Hashable {
        public var width: Int64
        public var cap: LineCap
        public var join: LineJoin
        public var miterLimit: Double
        public var dash: Dash?
        enum CodingKeys: String, CodingKey { case width, cap, join, miterLimit = "miter_limit", dash }
        public init(width: Int64, cap: LineCap = .butt, join: LineJoin = .miter, miterLimit: Double = 10, dash: Dash? = nil) {
            self.width = width; self.cap = cap; self.join = join; self.miterLimit = miterLimit; self.dash = dash
        }
    }

    /// One clip in page space (`kind: "path"`); an item paints only inside every clip.
    public struct ClipPath: Codable, Hashable {
        public var path: [PathCommand]
        public var fillRule: FillRule
        enum CodingKeys: String, CodingKey { case path, fillRule = "fill_rule" }
        public init(path: [PathCommand], fillRule: FillRule = .nonzero) { self.path = path; self.fillRule = fillRule }
    }

    /// A filled (`kind: "path_fill"`) or stroked (`kind: "path_stroke"`)
    /// vector path (TikZ). Coordinates are absolute page ticks.
    public struct Path: Codable, Equatable {
        public enum Op: Hashable {
            case fill(FillRule)
            case stroke(Stroke)
        }
        public var op: Op
        public var path: [PathCommand]
        public var clips: [ClipPath]
        public var paint: Paint
        public var sources: [SourceRange]?
        public var syntheticReason: String?
        public var isFill: Bool { if case .fill = op { return true } else { return false } }
        public var stroke: Stroke? { if case .stroke(let s) = op { return s } else { return nil } }
        public var fillRule: FillRule { if case .fill(let r) = op { return r } else { return .nonzero } }
        public var kind: String { isFill ? "path_fill" : "path_stroke" }

        enum CodingKeys: String, CodingKey { case kind, fillRule = "fill_rule", stroke, path, clips, paint, sources, syntheticReason = "synthetic_reason" }

        public init(op: Op, path: [PathCommand], clips: [ClipPath] = [], paint: Paint, sources: [SourceRange]?, syntheticReason: String? = nil) {
            self.op = op; self.path = path; self.clips = clips; self.paint = paint; self.sources = sources; self.syntheticReason = syntheticReason
        }

        public init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            let kind = try c.decode(String.self, forKey: .kind)
            switch kind {
            case "path_fill": op = .fill(try c.decodeIfPresent(FillRule.self, forKey: .fillRule) ?? .nonzero)
            case "path_stroke": op = .stroke(try c.decode(Stroke.self, forKey: .stroke))
            default: throw ValidationError(code: "unknown_item_kind", message: "display list item kind '\(kind)' is not a path")
            }
            path = try c.decode([PathCommand].self, forKey: .path)
            clips = try c.decodeIfPresent([ClipPath].self, forKey: .clips) ?? []
            paint = try c.decode(Paint.self, forKey: .paint)
            sources = try c.decodeIfPresent([SourceRange].self, forKey: .sources)
            syntheticReason = try c.decodeIfPresent(String.self, forKey: .syntheticReason)
        }

        public func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encode(kind, forKey: .kind)
            switch op {
            case .fill(let rule): try c.encode(rule, forKey: .fillRule)
            case .stroke(let s): try c.encode(s, forKey: .stroke)
            }
            try c.encode(path, forKey: .path)
            if !clips.isEmpty { try c.encode(clips, forKey: .clips) }
            try c.encode(paint, forKey: .paint)
            try c.encodeIfPresent(sources, forKey: .sources)
            try c.encodeIfPresent(syntheticReason, forKey: .syntheticReason)
        }
    }

    /// Paint-ordered page item. Decoding an unknown `kind` throws
    /// `ValidationError.unknownItemKind`: nothing is skipped silently.
    public enum Item: Codable, Equatable {
        case glyphRun(GlyphRun)
        case rule(Rule)
        case image(Image)
        /// `path_fill` and `path_stroke` alike (`Path.op` tells them apart).
        case path(Path)

        private enum KindKey: String, CodingKey { case kind }

        public init(from decoder: Decoder) throws {
            let kind = try decoder.container(keyedBy: KindKey.self).decode(String.self, forKey: .kind)
            switch kind {
            case "glyph_run": self = .glyphRun(try GlyphRun(from: decoder))
            case "rule": self = .rule(try Rule(from: decoder))
            case "image": self = .image(try Image(from: decoder))
            case "path_fill", "path_stroke": self = .path(try Path(from: decoder))
            default: throw ValidationError(code: "unknown_item_kind", message: "display list item kind '\(kind)' is not supported by this consumer")
            }
        }

        public func encode(to encoder: Encoder) throws {
            var kind = encoder.container(keyedBy: KindKey.self)
            switch self {
            case .glyphRun(let r): try kind.encode("glyph_run", forKey: .kind); try r.encode(to: encoder)
            case .rule(let r): try kind.encode("rule", forKey: .kind); try r.encode(to: encoder)
            case .image(let i): try kind.encode("image", forKey: .kind); try i.encode(to: encoder)
            case .path(let p): try p.encode(to: encoder) // writes its own kind
            }
        }
    }

    public struct Page: Codable, Equatable {
        public var number: Int
        public var width: Int64
        public var height: Int64
        public var items: [Item]
        public init(number: Int, width: Int64, height: Int64, items: [Item]) { self.number = number; self.width = width; self.height = height; self.items = items }
        public var widthPt: Double { RenderingV2.points(width) }
        public var heightPt: Double { RenderingV2.points(height) }
    }

    public struct FontResource: Codable, Equatable {
        public var fontId: String
        public var sha256: String
        public var byteLength: Int64
        public var format: String
        public var faceIndex: Int
        public var unitsPerEm: Int
        public var glyphCount: Int
        public var postscriptName: String
        enum CodingKeys: String, CodingKey {
            case fontId = "font_id", sha256, byteLength = "byte_length", format, faceIndex = "face_index", unitsPerEm = "units_per_em", glyphCount = "glyph_count", postscriptName = "postscript_name"
        }
        public init(fontId: String, sha256: String, byteLength: Int64, format: String, faceIndex: Int, unitsPerEm: Int, glyphCount: Int, postscriptName: String) {
            self.fontId = fontId; self.sha256 = sha256; self.byteLength = byteLength; self.format = format; self.faceIndex = faceIndex
            self.unitsPerEm = unitsPerEm; self.glyphCount = glyphCount; self.postscriptName = postscriptName
        }
        public var isPaintable: Bool { RenderingV2.paintableFontFormats.contains(format) }
    }

    public struct DocumentResource: Codable, Equatable {
        public var path: String
        public var revision: Int
        public var sha256: String
        public var byteLength: Int64
        enum CodingKeys: String, CodingKey { case path, revision, sha256, byteLength = "byte_length" }
        public init(path: String, revision: Int, sha256: String, byteLength: Int64) { self.path = path; self.revision = revision; self.sha256 = sha256; self.byteLength = byteLength }
    }

    public struct Diagnostic: Codable, Equatable {
        public enum Severity: String, Codable { case warning, error }
        public var code: String
        public var message: String
        public var severity: Severity
        public var sources: [SourceRange]
        /// Replacement text for `sources[0]` (`display-list-v2-diagnostics`).
        public var suggestion: String?
        public var labels: [Label]?
        public var notes: [String]?
        public var help: Help?

        /// Extra underlined span (`labels[]`); same three fields as runtime-v1.
        public struct Label: Codable, Equatable {
            public var source: SourceRange
            public var text: String
            public var primary: Bool
            public init(source: SourceRange, text: String, primary: Bool) {
                self.source = source; self.text = text; self.primary = primary
            }
        }

        /// `= help:` on the v2 line. `replacement.source` is `#/$defs/source`.
        public struct Help: Codable, Equatable {
            public var message: String
            public var replacement: Replacement?
            enum CodingKeys: String, CodingKey { case message, replacement }
            public init(message: String, replacement: Replacement? = nil) {
                self.message = message; self.replacement = replacement
            }
            public init(from decoder: Decoder) throws {
                let c = try decoder.container(keyedBy: CodingKeys.self)
                message = try c.decode(String.self, forKey: .message)
                replacement = try c.decodeIfPresent(Replacement.self, forKey: .replacement)
            }
            public func encode(to encoder: Encoder) throws {
                var c = encoder.container(keyedBy: CodingKeys.self)
                try c.encode(message, forKey: .message)
                if let replacement { try c.encode(replacement, forKey: .replacement) }
            }
            public struct Replacement: Codable, Equatable {
                public var source: SourceRange
                public var text: String
                public init(source: SourceRange, text: String) { self.source = source; self.text = text }
            }
        }

        enum CodingKeys: String, CodingKey { case code, message, severity, sources, suggestion, labels, notes, help }

        public init(code: String, message: String, severity: Severity, sources: [SourceRange],
                    suggestion: String? = nil, labels: [Label]? = nil, notes: [String]? = nil, help: Help? = nil) {
            self.code = code; self.message = message; self.severity = severity; self.sources = sources
            self.suggestion = Self.emptyToNil(suggestion)
            self.labels = Self.emptyToNil(labels); self.notes = Self.emptyToNil(notes); self.help = help
        }

        public init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            code = try c.decode(String.self, forKey: .code)
            message = try c.decode(String.self, forKey: .message)
            severity = try c.decode(Severity.self, forKey: .severity)
            sources = try c.decode([SourceRange].self, forKey: .sources)
            suggestion = Self.emptyToNil(try c.decodeIfPresent(String.self, forKey: .suggestion))
            labels = Self.emptyToNil(try c.decodeIfPresent([Label].self, forKey: .labels))
            notes = Self.emptyToNil(try c.decodeIfPresent([String].self, forKey: .notes))
            help = try c.decodeIfPresent(Help.self, forKey: .help)
        }

        public func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encode(code, forKey: .code)
            try c.encode(message, forKey: .message)
            try c.encode(severity, forKey: .severity)
            try c.encode(sources, forKey: .sources)
            if let suggestion { try c.encode(suggestion, forKey: .suggestion) }
            if let labels { try c.encode(labels, forKey: .labels) }
            if let notes { try c.encode(notes, forKey: .notes) }
            if let help { try c.encode(help, forKey: .help) }
        }

        /// The runtime-v1 shape the shell's diagnostics list already renders.
        public var asRuntimeV1: RuntimeV1.Diagnostic {
            let v1Help: RuntimeV1.Diagnostic.Help? = help.map { h in
                RuntimeV1.Diagnostic.Help(message: h.message, replacement: h.replacement.map { r in
                    RuntimeV1.Diagnostic.Replacement(startByte: r.source.startByte, endByte: r.source.endByte,
                                                     text: r.text, path: r.source.path)
                })
            }
            return RuntimeV1.Diagnostic(
                severity: severity == .error ? .error : .warning,
                message: message,
                source: sources.first,
                recovery: nil,
                code: code,
                suggestion: suggestion,
                labels: labels?.map { RuntimeV1.Diagnostic.Label(source: $0.source, text: $0.text, primary: $0.primary) },
                notes: notes,
                help: v1Help)
        }

        private static func emptyToNil(_ s: String?) -> String? {
            guard let s, !s.isEmpty else { return nil }
            return s
        }
        private static func emptyToNil<T>(_ a: [T]?) -> [T]? {
            guard let a, !a.isEmpty else { return nil }
            return a
        }
    }

    /// Top-level `navigation` object (`display-list-v2-links` §3). Coordinates
    /// are envelope ticks (`bp_2pow20`, y down). Absent on lists that did not
    /// negotiate the capability; unknown keys are ignored.
    public struct Navigation: Codable, Equatable {
        /// Axis-aligned link rectangle as `[x0, y0, x1, y1]` in page ticks.
        public struct Rect: Hashable, Codable {
            public var x0: Int64, y0: Int64, x1: Int64, y1: Int64
            public init(x0: Int64, y0: Int64, x1: Int64, y1: Int64) { self.x0 = x0; self.y0 = y0; self.x1 = x1; self.y1 = y1 }
            public var minX: Int64 { min(x0, x1) }
            public var maxX: Int64 { max(x0, x1) }
            public var minY: Int64 { min(y0, y1) }
            public var maxY: Int64 { max(y0, y1) }
            /// Half-open containment (`minX <= px < maxX`, same for y), matching `RenderingV2.Rect`.
            public func contains(x px: Int64, y py: Int64) -> Bool {
                px >= minX && px < maxX && py >= minY && py < maxY
            }
            public init(from decoder: Decoder) throws {
                var c = try decoder.unkeyedContainer()
                x0 = try c.decode(Int64.self); y0 = try c.decode(Int64.self)
                x1 = try c.decode(Int64.self); y1 = try c.decode(Int64.self)
                guard c.isAtEnd else { throw DecodingError.dataCorruptedError(in: c, debugDescription: "link rect must be [x0, y0, x1, y1]") }
            }
            public func encode(to encoder: Encoder) throws {
                var c = encoder.unkeyedContainer()
                try c.encode(x0); try c.encode(y0); try c.encode(x1); try c.encode(y1)
            }
        }

        /// Exactly one of an external URI or an internal destination name.
        public enum Target: Hashable, Codable {
            case uri(String)
            case destination(String)
            enum CodingKeys: String, CodingKey { case uri, destination }
            public init(from decoder: Decoder) throws {
                let c = try decoder.container(keyedBy: CodingKeys.self)
                let uri = try c.decodeIfPresent(String.self, forKey: .uri)
                let dest = try c.decodeIfPresent(String.self, forKey: .destination)
                switch (uri, dest) {
                case (let u?, nil): self = .uri(u)
                case (nil, let d?): self = .destination(d)
                default: throw DecodingError.dataCorruptedError(forKey: .uri, in: c, debugDescription: "target must be exactly one of uri or destination")
                }
            }
            public func encode(to encoder: Encoder) throws {
                var c = encoder.container(keyedBy: CodingKeys.self)
                switch self {
                case .uri(let u): try c.encode(u, forKey: .uri)
                case .destination(let d): try c.encode(d, forKey: .destination)
                }
            }
        }

        /// Compiler span of the `\href`/`\url` (proposal `document`/`start`/`end`).
        public struct Source: Codable, Hashable {
            public var document: String
            public var start: Int
            public var end: Int
            public init(document: String, start: Int, end: Int) { self.document = document; self.start = start; self.end = end }
        }

        public struct Link: Equatable, Codable {
            public var page: Int
            /// One entry per line piece; a wrapped link is several `Link`s with
            /// the same target. `rect` or `rects` on the wire.
            public var rects: [Rect]
            /// hyperref colour class (`link`, `url`, `cite`, `file`).
            public var className: String?
            public var border: [String]?
            public var color: [String]?
            public var target: Target
            public var source: Source?
            public init(page: Int, rects: [Rect], className: String? = nil, border: [String]? = nil, color: [String]? = nil, target: Target, source: Source? = nil) {
                self.page = page; self.rects = rects; self.className = className; self.border = border; self.color = color; self.target = target; self.source = source
            }
            enum CodingKeys: String, CodingKey { case page, rect, rects, className = "class", border, color, target, source }
            public init(from decoder: Decoder) throws {
                let c = try decoder.container(keyedBy: CodingKeys.self)
                page = try c.decode(Int.self, forKey: .page)
                var collected: [Rect] = []
                if let one = try c.decodeIfPresent(Rect.self, forKey: .rect) { collected.append(one) }
                if let many = try c.decodeIfPresent([Rect].self, forKey: .rects) { collected.append(contentsOf: many) }
                guard !collected.isEmpty else {
                    throw DecodingError.dataCorruptedError(forKey: .rect, in: c, debugDescription: "link needs rect or rects")
                }
                rects = collected
                className = try c.decodeIfPresent(String.self, forKey: .className)
                border = try c.decodeIfPresent([String].self, forKey: .border)
                color = try c.decodeIfPresent([String].self, forKey: .color)
                target = try c.decode(Target.self, forKey: .target)
                source = try c.decodeIfPresent(Source.self, forKey: .source)
            }
            public func encode(to encoder: Encoder) throws {
                var c = encoder.container(keyedBy: CodingKeys.self)
                try c.encode(page, forKey: .page)
                if rects.count == 1 { try c.encode(rects[0], forKey: .rect) } else { try c.encode(rects, forKey: .rects) }
                try c.encodeIfPresent(className, forKey: .className)
                try c.encodeIfPresent(border, forKey: .border)
                try c.encodeIfPresent(color, forKey: .color)
                try c.encode(target, forKey: .target)
                try c.encodeIfPresent(source, forKey: .source)
            }
        }

        public struct Destination: Codable, Hashable {
            public var page: Int
            public var x: Int64
            public var y: Int64
            public var view: String?
            public init(page: Int, x: Int64, y: Int64, view: String? = nil) { self.page = page; self.x = x; self.y = y; self.view = view }
        }

        public struct OutlineEntry: Codable, Hashable {
            public var title: String
            public var level: Int
            public var destination: String
            public init(title: String, level: Int, destination: String) { self.title = title; self.level = level; self.destination = destination }
        }

        public struct Info: Codable, Hashable {
            public var title: String?
            public var author: String?
            public var subject: String?
            public var keywords: String?
            public var creator: String?
            public init(title: String? = nil, author: String? = nil, subject: String? = nil, keywords: String? = nil, creator: String? = nil) {
                self.title = title; self.author = author; self.subject = subject; self.keywords = keywords; self.creator = creator
            }
        }

        public var links: [Link]
        public var destinations: [String: Destination]
        public var outline: [OutlineEntry]?
        public var outlineOpen: Bool?
        public var pageMode: String?
        public var openAction: String?
        public var info: Info?
        enum CodingKeys: String, CodingKey {
            case links, destinations, outline, outlineOpen = "outline_open", pageMode = "page_mode", openAction = "open_action", info
        }
        public init(links: [Link] = [], destinations: [String: Destination] = [:], outline: [OutlineEntry]? = nil,
                    outlineOpen: Bool? = nil, pageMode: String? = nil, openAction: String? = nil, info: Info? = nil) {
            self.links = links; self.destinations = destinations; self.outline = outline
            self.outlineOpen = outlineOpen; self.pageMode = pageMode; self.openAction = openAction; self.info = info
        }
    }


    public struct DisplayList: Codable, Equatable {
        public var renderFormat: String
        public var coordinateUnit: String
        public var colorSpace: String
        public var textExtraction: String
        public var projectId: String
        public var revision: Int
        public var requiredFeatures: [String]
        public var documents: [DocumentResource]
        public var fonts: [FontResource]
        public var pages: [Page]
        public var diagnostics: [Diagnostic]
        /// Present only when a producer that accepted `linksCapability` emits it.
        /// Decode is tolerant of absence; painting does not depend on it.
        public var navigation: Navigation?
        enum CodingKeys: String, CodingKey {
            case renderFormat = "render_format", coordinateUnit = "coordinate_unit", colorSpace = "color_space", textExtraction = "text_extraction"
            case projectId = "project_id", revision, requiredFeatures = "required_features", documents, fonts, pages, diagnostics, navigation
        }
        public init(renderFormat: String = RenderingV2.renderFormat, coordinateUnit: String = RenderingV2.coordinateUnit,
                    colorSpace: String = RenderingV2.colorSpace, textExtraction: String = RenderingV2.textExtraction,
                    projectId: String, revision: Int, requiredFeatures: [String], documents: [DocumentResource],
                    fonts: [FontResource], pages: [Page], diagnostics: [Diagnostic], navigation: Navigation? = nil) {
            self.renderFormat = renderFormat; self.coordinateUnit = coordinateUnit; self.colorSpace = colorSpace; self.textExtraction = textExtraction
            self.projectId = projectId; self.revision = revision; self.requiredFeatures = requiredFeatures; self.documents = documents
            self.fonts = fonts; self.pages = pages; self.diagnostics = diagnostics; self.navigation = navigation
        }
        public func font(id: String) -> FontResource? { fonts.first { $0.fontId == id } }
    }

    public struct Envelope: Codable, Equatable {
        public var protocolVersion: Int
        public var id: String
        public var type: String
        public var payload: DisplayList
        enum CodingKeys: String, CodingKey { case protocolVersion = "protocol_version", id, type, payload }
        public init(protocolVersion: Int = RenderingV2.protocolVersion, id: String, type: String = RenderingV2.messageType, payload: DisplayList) {
            self.protocolVersion = protocolVersion; self.id = id; self.type = type; self.payload = payload
        }
    }

    /// A diagnostic-bearing refusal. The consumer never renders partially: any
    /// error here means no frame is published for this display list.
    public struct ValidationError: Error, Equatable, CustomStringConvertible {
        public var code: String
        public var message: String
        public var source: SourceRange?
        public init(code: String, message: String, source: SourceRange? = nil) { self.code = code; self.message = message; self.source = source }
        public var description: String { "\(code): \(message)" }
        public var asRuntimeV1: RuntimeV1.Diagnostic {
            RuntimeV1.Diagnostic(severity: .error, message: "[\(code)] \(message)", source: source, recovery: nil)
        }
    }

    // MARK: decoding + validation

    /// Header-only probe so version/type refusals name what was found rather
    /// than failing on an unrelated payload key.
    private struct Header: Decodable {
        var protocolVersion: Int?
        var id: String?
        var type: String?
        enum CodingKeys: String, CodingKey { case protocolVersion = "protocol_version", id, type }
    }

    /// Decodes and validates a `display_list` envelope. Fails closed: an unknown
    /// protocol version, message type, item kind, feature, font reference, or
    /// malformed geometry/cluster is an error, never a partial result.
    public static func decode(_ data: Data) throws -> Envelope {
        let envelope: Envelope
        do {
            // Fast typed reader first (same values for every valid frame); any
            // syntax/shape it does not accept falls back to JSONDecoder, whose
            // error then stands. An unknown item kind is refused by both alike.
            envelope = try RenderingV2Fast.envelope(data)
        } catch {
            envelope = try decodeSlow(data)
        }
        try checkHeader(version: envelope.protocolVersion, type: envelope.type)
        try validate(envelope.payload)
        return envelope
    }

    /// Version/type refusal shared by every decoding path.
    public static func checkHeader(version: Int, type: String) throws {
        guard version == protocolVersion else {
            throw ValidationError(code: "unsupported_protocol_version", message: "protocol version \(version) is not supported; this consumer speaks rendering-v2 (protocol_version 2)")
        }
        guard type == messageType else {
            throw ValidationError(code: "unsupported_message_type", message: "message type '\(type)' is not a display_list")
        }
    }

    /// `JSONDecoder` path: the header is probed first so version/type refusals
    /// name what was found rather than failing on an unrelated payload key.
    private static func decodeSlow(_ data: Data) throws -> Envelope {
        let decoder = JSONDecoder()
        let header: Header
        do { header = try decoder.decode(Header.self, from: data) } catch {
            throw ValidationError(code: "malformed_json", message: "not a JSON object: \(error.localizedDescription)")
        }
        guard let version = header.protocolVersion else {
            throw ValidationError(code: "missing_protocol_version", message: "protocol_version is required")
        }
        guard let type = header.type else { throw ValidationError(code: "missing_type", message: "type is required") }
        try checkHeader(version: version, type: type)
        do { return try decoder.decode(Envelope.self, from: data) } catch let e as ValidationError {
            throw e
        } catch let DecodingError.dataCorrupted(ctx) {
            if let inner = ctx.underlyingError as? ValidationError { throw inner }
            throw ValidationError(code: "malformed_payload", message: ctx.debugDescription)
        } catch {
            throw ValidationError(code: "malformed_payload", message: describe(error))
        }
    }

    private static func describe(_ error: Error) -> String {
        switch error {
        case DecodingError.keyNotFound(let key, let ctx): return "missing key '\(key.stringValue)' at \(path(ctx))"
        case DecodingError.typeMismatch(_, let ctx): return "type mismatch at \(path(ctx)): \(ctx.debugDescription)"
        case DecodingError.valueNotFound(_, let ctx): return "null at \(path(ctx))"
        default: return error.localizedDescription
        }
    }
    private static func path(_ ctx: DecodingError.Context) -> String {
        ctx.codingPath.map { $0.intValue.map { "[\($0)]" } ?? $0.stringValue }.joined(separator: ".").replacingOccurrences(of: ".[", with: "[")
    }

    private static let hex64 = try! NSRegularExpression(pattern: "^[0-9a-f]{64}$")
    private static func isHex64(_ s: String) -> Bool {
        hex64.firstMatch(in: s, range: NSRange(location: 0, length: (s as NSString).length)) != nil
    }

    /// Semantic validation of a decoded payload (see `decode`).
    public static func validate(_ list: DisplayList) throws {
        func fail(_ code: String, _ message: String, _ source: SourceRange? = nil) -> ValidationError { ValidationError(code: code, message: message, source: source) }
        guard list.renderFormat == renderFormat else { throw fail("unsupported_format", "render_format '\(list.renderFormat)' is not \(renderFormat)") }
        guard list.coordinateUnit == coordinateUnit else { throw fail("unsupported_coordinate_unit", "coordinate_unit '\(list.coordinateUnit)' is not \(coordinateUnit)") }
        guard list.colorSpace == colorSpace else { throw fail("unsupported_color_space", "color_space '\(list.colorSpace)' is not \(colorSpace)") }
        guard list.textExtraction == textExtraction else { throw fail("unsupported_text_extraction", "text_extraction '\(list.textExtraction)' is not \(textExtraction)") }
        guard !list.requiredFeatures.isEmpty else { throw fail("invalid_display_list", "required_features must list at least one feature") }
        for f in list.requiredFeatures where !knownFeatures.contains(f) {
            throw fail("unsupported_feature", "required feature '\(f)' is not supported by this consumer")
        }
        guard list.revision >= 0, Int64(list.revision) <= maxExactInteger else { throw fail("invalid_display_list", "revision must be a nonnegative exact integer") }
        guard Bounds.documents.contains(list.documents.count) else { throw fail("invalid_display_list", "documents must declare 1...\(Bounds.documents.upperBound) source documents (found \(list.documents.count))") }
        guard Bounds.fonts.contains(list.fonts.count) else { throw fail("invalid_display_list", "fonts must declare at most \(Bounds.fonts.upperBound) resources (found \(list.fonts.count))") }
        guard Bounds.pages.contains(list.pages.count) else { throw fail("invalid_display_list", "at most \(Bounds.pages.upperBound) pages (found \(list.pages.count))") }
        guard Bounds.diagnostics.contains(list.diagnostics.count) else { throw fail("invalid_display_list", "at most \(Bounds.diagnostics.upperBound) diagnostics (found \(list.diagnostics.count))") }
        var documents: [String: DocumentResource] = [:]
        for d in list.documents {
            guard isProjectPath(d.path) else {
                throw fail("invalid_resource", "document path '\(d.path)' must be project-relative: no empty, '.' or '..' components, no backslash, colon or NUL")
            }
            guard documents.updateValue(d, forKey: d.path) == nil else { throw fail("invalid_resource", "document '\(d.path)' is declared twice") }
            guard isHex64(d.sha256) else { throw fail("invalid_resource", "document '\(d.path)' sha256 is not 64 lowercase hex digits") }
            guard d.revision >= 0, Int64(d.revision) <= maxExactInteger else { throw fail("invalid_resource", "document '\(d.path)' revision must be a nonnegative exact integer") }
            guard Bounds.documentByteLength.contains(d.byteLength) else { throw fail("invalid_resource", "document '\(d.path)' byte_length \(d.byteLength) is outside 0...\(Bounds.documentByteLength.upperBound)") }
        }
        var fontsById: [String: FontResource] = [:]
        for f in list.fonts {
            guard !f.fontId.isEmpty, fontsById.updateValue(f, forKey: f.fontId) == nil else {
                throw fail("invalid_resource", "font resource id '\(f.fontId)' is empty or declared twice")
            }
            guard isHex64(f.sha256) else { throw fail("invalid_resource", "font resource \(f.fontId) sha256 is not 64 lowercase hex digits") }
            guard paintableFontFormats.contains(f.format) || metricsOnlyFontFormats.contains(f.format) else {
                throw fail("unsupported_feature", "font resource \(f.fontId) format '\(f.format)' is not supported (paintable: \(paintableFontFormats.sorted().joined(separator: ", ")))")
            }
            guard f.faceIndex == 0 else { throw fail("unsupported_feature", "font resource \(f.fontId) face_index \(f.faceIndex): only face 0 is supported") }
            guard f.isPaintable ? Bounds.fontByteLength.contains(f.byteLength) : f.byteLength >= 0 else {
                throw fail("invalid_resource", "font resource \(f.fontId) byte_length \(f.byteLength) is outside \(Bounds.fontByteLength)")
            }
            guard (16...16384).contains(f.unitsPerEm) else { throw fail("invalid_resource", "font resource \(f.fontId) units_per_em \(f.unitsPerEm) is out of range") }
            guard (2...65536).contains(f.glyphCount) else { throw fail("invalid_resource", "font resource \(f.fontId) glyph_count \(f.glyphCount) is out of range") }
            guard Bounds.postscriptNameBytes.contains(f.postscriptName.utf8.count) else { throw fail("invalid_resource", "font resource \(f.fontId) postscript_name must be 1...\(Bounds.postscriptNameBytes.upperBound) bytes") }
        }
        // Features the list actually uses must all be declared (rendering-core:
        // "undeclared rendering feature"). `static-truetype` is deliberately not
        // derived from glyph runs: the pipeline declares it only for TrueType
        // resources and paints Latin Modern as `opentype-cff` (documented deviation).
        var usedFeatures: Set<String> = ["rgba-srgb", "cluster-actualtext"]
        var lastPage = 0
        for page in list.pages {
            guard page.number == lastPage + 1 else { throw fail("invalid_display_list", "page numbers must be contiguous from 1 (found \(page.number) after \(lastPage))") }
            lastPage = page.number
            guard isPositiveTick(page.width), isPositiveTick(page.height) else { throw fail("invalid_display_list", "page \(page.number) must have positive exact width and height") }
            guard Bounds.pageItems.contains(page.items.count) else { throw fail("invalid_display_list", "page \(page.number) has \(page.items.count) items (limit \(Bounds.pageItems.upperBound))") }
            for (index, item) in page.items.enumerated() {
                let at = "page \(page.number) item \(index)"
                switch item {
                case .rule(let r):
                    usedFeatures.insert("rule")
                    guard isTick(r.x), isTick(r.top), isPositiveTick(r.width), isPositiveTick(r.height),
                          isTick(r.x &+ r.width), isTick(r.top &+ r.height), isTick(page.height &- r.top &- r.height) else {
                        throw fail("invalid_display_list", "\(at): rule needs positive width/height and exact-range coordinates")
                    }
                    try validatePaint(r.paint, at)
                    try validateProvenance(sources: r.sources, synthetic: r.syntheticReason, documents: documents, at)
                case .image(let i):
                    usedFeatures.insert("image")
                    guard isTick(i.x), isTick(i.top), isPositiveTick(i.width), isPositiveTick(i.height),
                          isTick(i.x &+ i.width), isTick(i.top &+ i.height), isTick(page.height &- i.top &- i.height) else {
                        throw fail("invalid_display_list", "\(at): image needs positive width/height and exact-range coordinates")
                    }
                    guard i.transform.count == 6, i.transform.allSatisfy(\.isFinite) else {
                        throw fail("invalid_display_list", "\(at): image transform must be six finite numbers [a, b, c, d, e, f]")
                    }
                    try validateImageResource(i.image, at)
                    try validateProvenance(sources: i.sources, synthetic: i.syntheticReason, documents: documents, at)
                case .path(let p):
                    usedFeatures.insert(p.isFill ? "path_fill" : "path_stroke")
                    if !p.clips.isEmpty { usedFeatures.insert("clip") }
                    try validatePath(p, at)
                    try validateProvenance(sources: p.sources, synthetic: p.syntheticReason, documents: documents, at)
                case .glyphRun(let run):
                    usedFeatures.insert("glyph_run")
                    guard let font = fontsById[run.fontId] else { throw fail("invalid_resource", "\(at): font resource '\(run.fontId)' is not declared in fonts") }
                    guard isPositiveTick(run.fontSize) else { throw fail("invalid_display_list", "\(at): font_size must be a positive exact tick count") }
                    guard Bounds.runTextBytes.contains(run.text.utf8.count) else { throw fail("invalid_display_list", "\(at): glyph run text must be 1...\(Bounds.runTextBytes.upperBound) bytes") }
                    guard Bounds.glyphs.contains(run.glyphs.count) else { throw fail("invalid_display_list", "\(at): glyph run must carry 1...\(Bounds.glyphs.upperBound) glyphs (found \(run.glyphs.count))") }
                    guard Bounds.clusters.contains(run.clusters.count) else { throw fail("invalid_display_list", "\(at): glyph run must carry 1...\(Bounds.clusters.upperBound) clusters (found \(run.clusters.count))") }
                    try validatePaint(run.paint, at)
                    var referencedClusters = Set<Int>()
                    for (gi, g) in run.glyphs.enumerated() {
                        guard g.gid >= 1, g.gid < font.glyphCount else {
                            throw fail("invalid_display_list", "\(at) glyph \(gi): gid \(g.gid) is outside 1..<\(font.glyphCount) of font \(font.postscriptName)")
                        }
                        guard g.cluster >= 0, g.cluster < run.clusters.count else {
                            throw fail("invalid_display_list", "\(at) glyph \(gi): cluster \(g.cluster) is outside 0..<\(run.clusters.count)")
                        }
                        referencedClusters.insert(g.cluster)
                        guard isTick(g.originX), isTick(g.baselineY), isTick(g.advanceX), isTick(g.advanceY),
                              isTick(g.originX &+ g.advanceX), isTick(g.baselineY &+ g.advanceY), isTick(page.height &- g.baselineY) else {
                            throw fail("invalid_display_list", "\(at) glyph \(gi): origin/advance outside the exact tick range")
                        }
                    }
                    guard referencedClusters.count == run.clusters.count else {
                        // Name the offending cluster(s): index, logical text and source
                        // span(s), so the refusal points at the source that produced them
                        // (measured on flashtex-render 9aaec57a: a missing-glyph scalar glued
                        // to a word, `😀shuffle`, yields a cluster with no glyph).
                        let orphaned = run.clusters.indices.filter { !referencedClusters.contains($0) }
                        let named = orphaned.map { ci -> String in
                            let c = run.clusters[ci]
                            let where_ = c.sources.map { $0.map { "\($0.path) bytes \($0.startByte)..<\($0.endByte)" }.joined(separator: ", ") }
                                ?? c.syntheticReason.map { "generated: \($0)" } ?? "no provenance"
                            return "cluster \(ci) “\(run.clusterText(ci))” (\(where_))"
                        }.joined(separator: "; ")
                        throw ValidationError(code: "invalid_display_list",
                                              message: "\(at): \(orphaned.count) cluster(s) have no glyph (every cluster needs at least one glyph): \(named)",
                                              source: orphaned.first.flatMap { run.clusters[$0].sources?.first })
                    }
                    let textBytes = Array(run.text.utf8)
                    var expectedStart = 0
                    for (ci, c) in run.clusters.enumerated() {
                        let cat = "\(at) cluster \(ci)"
                        guard c.textStartByte == expectedStart, c.textEndByte > c.textStartByte, c.textEndByte <= textBytes.count else {
                            throw fail("invalid_display_list", "\(cat): byte range \(c.textStartByte)..<\(c.textEndByte) does not partition the \(textBytes.count)-byte run text (expected a nonempty range starting at \(expectedStart))")
                        }
                        guard isBoundary(textBytes, c.textStartByte), isBoundary(textBytes, c.textEndByte) else {
                            throw fail("invalid_display_list", "\(cat): byte range \(c.textStartByte)..<\(c.textEndByte) splits a UTF-8 sequence")
                        }
                        expectedStart = c.textEndByte
                        guard Bounds.hitRects.contains(c.hitRects.count) else { throw fail("invalid_display_list", "\(cat): hit_rects must carry 1...\(Bounds.hitRects.upperBound) rectangles (found \(c.hitRects.count))") }
                        guard Bounds.carets.contains(c.carets.count) else { throw fail("invalid_display_list", "\(cat): at most \(Bounds.carets.upperBound) carets (found \(c.carets.count))") }
                        for r in c.hitRects {
                            guard isTick(r.x), isTick(r.top), isTick(r.width), isTick(r.height), r.width >= 0, r.height >= 0,
                                  isTick(r.x &+ r.width), isTick(r.top &+ r.height) else {
                                throw fail("invalid_display_list", "\(cat): hit rect has a negative size or coordinates outside the exact tick range")
                            }
                        }
                        for k in c.carets {
                            guard k.textByte >= c.textStartByte, k.textByte <= c.textEndByte, isBoundary(textBytes, k.textByte) else {
                                throw fail("invalid_display_list", "\(cat): caret text_byte \(k.textByte) is outside the cluster or splits a UTF-8 sequence")
                            }
                            guard isTick(k.x), isTick(k.top), isPositiveTick(k.height), isTick(k.top &+ k.height) else {
                                throw fail("invalid_display_list", "\(cat): caret needs a positive height and exact-range coordinates")
                            }
                        }
                        try validateProvenance(sources: c.sources, synthetic: c.syntheticReason, documents: documents, cat)
                    }
                    guard expectedStart == textBytes.count else {
                        throw fail("invalid_display_list", "\(at): clusters cover \(expectedStart) of \(textBytes.count) text bytes")
                    }
                }
            }
        }
        for d in list.diagnostics {
            guard !d.code.isEmpty, Bounds.diagnosticMessageBytes.contains(d.message.utf8.count) else {
                throw fail("invalid_display_list", "diagnostics must carry a code and a 1...\(Bounds.diagnosticMessageBytes.upperBound)-byte message")
            }
            guard Bounds.diagnosticSources.contains(d.sources.count) else { throw fail("invalid_display_list", "diagnostic '\(d.code)' lists \(d.sources.count) sources (limit \(Bounds.diagnosticSources.upperBound))") }
            for s in d.sources { try validateSource(s, documents: documents, "diagnostic '\(d.code)'") }
            if let suggestion = d.suggestion {
                guard Bounds.diagnosticMessageBytes.contains(suggestion.utf8.count) else {
                    throw fail("invalid_display_list", "diagnostic '\(d.code)' suggestion must be 1...\(Bounds.diagnosticMessageBytes.upperBound) bytes")
                }
            }
            if let labels = d.labels {
                guard Bounds.diagnosticSources.contains(labels.count) else {
                    throw fail("invalid_display_list", "diagnostic '\(d.code)' lists \(labels.count) labels (limit \(Bounds.diagnosticSources.upperBound))")
                }
                for lab in labels { try validateSource(lab.source, documents: documents, "diagnostic '\(d.code)' label") }
            }
            if let notes = d.notes {
                for n in notes {
                    guard Bounds.diagnosticMessageBytes.contains(n.utf8.count) else {
                        throw fail("invalid_display_list", "diagnostic '\(d.code)' note must be 1...\(Bounds.diagnosticMessageBytes.upperBound) bytes")
                    }
                }
            }
            if let help = d.help {
                guard Bounds.diagnosticMessageBytes.contains(help.message.utf8.count) else {
                    throw fail("invalid_display_list", "diagnostic '\(d.code)' help message must be 1...\(Bounds.diagnosticMessageBytes.upperBound) bytes")
                }
                if let r = help.replacement {
                    try validateSource(r.source, documents: documents, "diagnostic '\(d.code)' help replacement")
                }
            }
        }
        let declared = Set(list.requiredFeatures)
        let undeclared = usedFeatures.subtracting(declared).sorted()
        guard undeclared.isEmpty else {
            throw fail("invalid_display_list", "the list uses feature(s) \(undeclared.joined(separator: ", ")) that required_features does not declare (\(list.requiredFeatures.joined(separator: ", ")))")
        }
    }

    /// `|t| <= 2^53 − 1`: representable exactly in JSON and in a Double.
    static func isTick(_ t: Int64) -> Bool { t >= -maxExactInteger && t <= maxExactInteger }
    static func isPositiveTick(_ t: Int64) -> Bool { t > 0 && t <= maxExactInteger }

    /// crates/rendering-core `path`: no backslash, colon or NUL, and no empty,
    /// `.` or `..` component (so no leading `/` either).
    static func isProjectPath(_ p: String) -> Bool {
        !p.isEmpty && !p.contains("\\") && !p.contains(":") && !p.contains("\0")
            && p.split(separator: "/", omittingEmptySubsequences: false).allSatisfy { !($0.isEmpty || $0 == "." || $0 == "..") }
    }

    private static func isBoundary(_ bytes: [UInt8], _ i: Int) -> Bool {
        i == bytes.count || (i >= 0 && i < bytes.count && (bytes[i] & 0xC0) != 0x80)
    }

    /// Proposal §3 resource shape: `image_id` is the SHA-256, a bounded
    /// positive byte length, a known format, a project-relative path, pixel
    /// dimensions for raster formats and a page/box/rotation for PDF.
    static func validateImageResource(_ r: ImageResource, _ at: String) throws {
        func fail(_ m: String) -> ValidationError { ValidationError(code: "invalid_resource", message: "\(at): image resource \(m)") }
        guard isHex64(r.sha256) else { throw fail("sha256 is not 64 lowercase hex digits") }
        guard r.imageId == r.sha256 else { throw fail("image_id '\(r.imageId)' must equal sha256") }
        guard r.byteLength >= 1, r.byteLength <= maxImageByteLength else { throw fail("byte_length \(r.byteLength) is outside 1...\(maxImageByteLength)") }
        guard imageFormats.contains(r.format) else {
            throw ValidationError(code: "unsupported_feature", message: "\(at): image format '\(r.format)' is not supported (\(imageFormats.sorted().joined(separator: ", ")))")
        }
        guard isProjectPath(r.path) else { throw fail("path '\(r.path)' must be project-relative: no empty, '.' or '..' components, no backslash, colon or NUL") }
        if r.format == "pdf" {
            guard let page = r.pdfPage, page >= 1, page <= 100_000 else { throw fail("pdf_page must be a positive page number") }
            guard let box = r.pdfBox, box.count == 4, box.allSatisfy(\.isFinite), box[2] > box[0], box[3] > box[1] else {
                throw fail("pdf_box must be [llx, lly, urx, ury] with positive extent")
            }
            guard [0, 90, 180, 270].contains(r.pdfRotate ?? 0) else { throw fail("pdf_rotate must be 0, 90, 180 or 270") }
        } else {
            guard let w = r.pixelWidth, let h = r.pixelHeight, w >= 1, h >= 1, w <= 1 << 20, h <= 1 << 20 else {
                throw fail("pixel_width/pixel_height must be positive for \(r.format)")
            }
        }
    }


    /// Commands: bounded count, every coordinate an exact tick, the first
    /// command a move (CoreGraphics has no current point before one), and
    /// nothing but a move after `z`. Strokes need a positive exact width, a
    /// finite miter limit ≥ 1 and a dash whose entries are nonnegative exact
    /// ticks with at least one positive (an all-zero dash never advances).
    static func validateCommands(_ cmds: [PathCommand], _ at: String) throws {
        func fail(_ m: String) -> ValidationError { ValidationError(code: "invalid_display_list", message: "\(at): \(m)") }
        guard Bounds.pathCommands.contains(cmds.count) else { throw fail("path must carry 1...\(Bounds.pathCommands.upperBound) commands (found \(cmds.count))") }
        var open = false
        for (i, c) in cmds.enumerated() {
            for v in c.coordinates where !isTick(v) { throw fail("path command \(i) has a coordinate outside the exact tick range") }
            switch c {
            case .move: open = true
            case .line, .cubic: guard open else { throw fail("path command \(i) needs a current point (no preceding move)") }
            case .close: guard open else { throw fail("path command \(i) closes without an open subpath") }; open = false
            }
        }
    }

    static func validatePath(_ p: Path, _ at: String) throws {
        func fail(_ m: String) -> ValidationError { ValidationError(code: "invalid_display_list", message: "\(at): \(m)") }
        try validateCommands(p.path, at)
        guard Bounds.clips.contains(p.clips.count) else { throw fail("at most \(Bounds.clips.upperBound) clips (found \(p.clips.count))") }
        for (ci, clip) in p.clips.enumerated() { try validateCommands(clip.path, "\(at) clip \(ci)") }
        try validatePaint(p.paint, at)
        if let s = p.stroke {
            guard isPositiveTick(s.width) else { throw fail("stroke width must be a positive exact tick count") }
            guard s.miterLimit.isFinite, s.miterLimit >= 1 else { throw fail("stroke miter_limit must be a finite number ≥ 1") }
            if let d = s.dash {
                guard Bounds.dashEntries.contains(d.array.count) else { throw fail("dash array must carry 1...\(Bounds.dashEntries.upperBound) entries (found \(d.array.count))") }
                guard d.array.allSatisfy({ $0 >= 0 && isTick($0) }), d.array.contains(where: { $0 > 0 }) else { throw fail("dash entries must be nonnegative exact ticks with at least one positive") }
                guard d.phase >= 0, isTick(d.phase) else { throw fail("dash phase must be a nonnegative exact tick count") }
            }
        }
    }

    private static func validatePaint(_ p: Paint, _ at: String) throws {
        for v in [p.r, p.g, p.b, p.a] where !(v >= 0 && v <= 1) {
            throw ValidationError(code: "invalid_display_list", message: "\(at): paint components must be within 0...1")
        }
    }

    /// A source range must name a declared document and lie within its
    /// declared byte length (rendering-core `source`).
    private static func validateSource(_ s: SourceRange, documents: [String: DocumentResource], _ at: String) throws {
        guard let doc = documents[s.path] else {
            throw ValidationError(code: "invalid_display_list", message: "\(at): source path '\(s.path)' is not a declared document", source: s)
        }
        guard s.startByte >= 0, s.endByte >= s.startByte, Int64(s.endByte) <= doc.byteLength else {
            throw ValidationError(code: "invalid_display_list", message: "\(at): source range \(s.startByte)..<\(s.endByte) is malformed or outside \(s.path)'s \(doc.byteLength) bytes", source: s)
        }
    }

    private static func validateProvenance(sources: [SourceRange]?, synthetic: String?, documents: [String: DocumentResource], _ at: String) throws {
        switch (sources, synthetic) {
        case (nil, nil): throw ValidationError(code: "invalid_display_list", message: "\(at): needs sources or synthetic_reason")
        case (.some, .some): throw ValidationError(code: "invalid_display_list", message: "\(at): sources and synthetic_reason are mutually exclusive")
        case (nil, .some(let reason)):
            guard Bounds.syntheticReasonBytes.contains(reason.utf8.count) else { throw ValidationError(code: "invalid_display_list", message: "\(at): synthetic_reason must be 1...\(Bounds.syntheticReasonBytes.upperBound) bytes") }
        case (.some(let ranges), nil):
            guard Bounds.sourceRanges.contains(ranges.count) else { throw ValidationError(code: "invalid_display_list", message: "\(at): sources must list 1...\(Bounds.sourceRanges.upperBound) ranges (found \(ranges.count))") }
            for s in ranges { try validateSource(s, documents: documents, at) }
        }
    }
}
