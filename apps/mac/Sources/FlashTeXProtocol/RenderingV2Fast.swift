import Foundation

/// Typed byte-level reader for the rendering-v2 `display_list` envelope
/// (`RenderingV2.decode`'s fast path). `JSONDecoder` + `Codable` read a
/// 1.9 MB two-page envelope (4.7k glyphs) in ~75 ms — most of the per-
/// keystroke cost of the live v2 route; this reader builds the same
/// `RenderingV2` values directly from the bytes in a fraction of that.
///
/// Semantics follow `JSONDecoder` where it matters: RFC 8259 syntax only,
/// unknown keys ignored, duplicate keys keep the last value, integers must be
/// integer literals within `Int`, doubles are read with the correctly rounded
/// `Double(_:)`, `null` for an optional field is `nil`. Anything this reader
/// does not accept throws `RenderingV2Fast.Error` and the caller falls back to
/// `JSONDecoder`, whose error then stands — the fast path never changes which
/// inputs are rejected, only how fast valid ones are read. Validation
/// (`RenderingV2.validate`) runs on the result exactly as on the slow path.
public struct RenderingV2Fast {
    public struct Error: Swift.Error, CustomStringConvertible {
        public var offset: Int
        public var message: String
        public var description: String { "fast rendering-v2 reader: \(message) at byte \(offset)" }
    }

    private let b: UnsafeBufferPointer<UInt8>
    private var i = 0
    /// Page-reuse hook and the byte ranges recorded while reading (`envelope(_:reuse:)`).
    private var pageReuse: ((Int, UnsafeRawBufferPointer) -> RenderingV2.Page?)?
    private var pageRanges: [Range<Int>] = []
    private var fontsRange: Range<Int>?
    private var reusedPages: [Int] = []

    private init(_ bytes: UnsafeBufferPointer<UInt8>) { b = bytes }

    /// Reads one `display_list` envelope. Throws `Error` for anything outside
    /// the accepted syntax/shape, including an unknown item kind; the caller
    /// then falls back to the `Codable` path, whose diagnostics stand.
    public static func envelope(_ data: Data) throws -> RenderingV2.Envelope {
        try data.withUnsafeBytes { raw -> RenderingV2.Envelope in
            var p = RenderingV2Fast(raw.bindMemory(to: UInt8.self))
            p.ws()
            let env = try p.envelope()
            p.ws()
            guard p.i == p.b.count else { throw p.err("trailing characters") }
            return env
        }
    }

    /// `envelope(_:)` plus the byte range of every `payload.pages[i]` object
    /// and of the `payload.fonts` array in the input, with page-level reuse:
    /// before a page object is decoded, `reuse` sees its exact bytes (index in
    /// `pages`, raw object bytes) and may return an already decoded `Page` for
    /// them — identical bytes decode to identical values, so the returned page
    /// is what decoding would have produced. Pages taken from `reuse` are
    /// listed in `reusedPages`; every other page is decoded exactly as by
    /// `envelope(_:)`. Validation (`RenderingV2.validate`) is the caller's, as
    /// on the plain path.
    public struct Decoded {
        public var envelope: RenderingV2.Envelope
        public var pageRanges: [Range<Int>]
        public var fontsRange: Range<Int>?
        public var reusedPages: [Int]
    }

    public static func envelope(_ data: Data, reuse: (Int, UnsafeRawBufferPointer) -> RenderingV2.Page?) throws -> Decoded {
        try withoutActuallyEscaping(reuse) { reuse in
            try data.withUnsafeBytes { raw -> Decoded in
                var p = RenderingV2Fast(raw.bindMemory(to: UInt8.self))
                p.pageReuse = reuse
                p.ws()
                let env = try p.envelope()
                p.ws()
                guard p.i == p.b.count else { throw p.err("trailing characters") }
                return Decoded(envelope: env, pageRanges: p.pageRanges, fontsRange: p.fontsRange, reusedPages: p.reusedPages)
            }
        }
    }

    /// Decodes one page object from `range` of `data` (a range reported by
    /// `envelope(_:reuse:)`), exactly as the envelope reader would.
    public static func page(_ data: Data, range: Range<Int>) throws -> RenderingV2.Page {
        try data.withUnsafeBytes { raw -> RenderingV2.Page in
            let all = raw.bindMemory(to: UInt8.self)
            guard range.lowerBound >= 0, range.upperBound <= all.count, let base = all.baseAddress else {
                throw Error(offset: range.lowerBound, message: "page range outside the data")
            }
            var p = RenderingV2Fast(UnsafeBufferPointer(start: base + range.lowerBound, count: range.count))
            let page = try p.page()
            p.ws()
            guard p.i == p.b.count else { throw p.err("trailing characters after the page object") }
            return page
        }
    }

    /// Top-level `protocol_version`, `id` and `type` of an envelope line with the
    /// payload skipped by a string/nesting-aware byte scan (no number or string
    /// decoding): ~1 ms for a 2 MB line, against ~12 ms for a full parse. Nil
    /// when the line is not a JSON object with those three fields.
    public struct Header: Equatable {
        public var protocolVersion: Int
        public var id: String
        public var type: String
    }

    public static func header(_ data: Data) -> Header? {
        data.withUnsafeBytes { raw -> Header? in
            var p = RenderingV2Fast(raw.bindMemory(to: UInt8.self))
            var version: Int?, id: String?, type: String?
            do {
                try p.object { key, p in
                    switch key {
                    case "protocol_version": version = try p.int()
                    case "id": id = try p.string()
                    case "type": type = try p.string()
                    default: try p.skipRaw()
                    }
                }
            } catch { return nil }
            guard let version, let id, let type else { return nil }
            return Header(protocolVersion: version, id: id, type: type)
        }
    }

    /// Skips one value by bracket depth and string boundaries only.
    private mutating func skipRaw() throws {
        ws()
        guard i < b.count else { throw err("unexpected end") }
        var depth = 0
        repeat {
            let c = b[i]
            switch c {
            case 0x22: // string: skip to the closing quote, honoring escapes
                i += 1
                while i < b.count, b[i] != 0x22 { i += b[i] == 0x5C ? 2 : 1 }
                guard i < b.count else { throw err("unterminated string") }
                i += 1
            case 0x7B, 0x5B: depth += 1; i += 1
            case 0x7D, 0x5D: depth -= 1; i += 1
            case 0x2C where depth == 0: return
            default: i += 1
            }
        } while depth > 0 && i < b.count
        guard depth == 0 else { throw err("unbalanced value") }
        // A scalar value ends at ',' or '}' (handled by the caller); scan up to it.
        while i < b.count, b[i] != 0x2C, b[i] != 0x7D { i += 1 }
    }

    // MARK: envelope

    private mutating func envelope() throws -> RenderingV2.Envelope {
        var version: Int?, id: String?, type: String?, payload: RenderingV2.DisplayList?
        try object { key, p in
            switch key {
            case "protocol_version": version = try p.int()
            case "id": id = try p.string()
            case "type": type = try p.string()
            case "payload": payload = try p.displayList()
            default: try p.skip(depth: 1)
            }
        }
        guard let version else { throw err("missing protocol_version") }
        guard let id else { throw err("missing id") }
        guard let type else { throw err("missing type") }
        guard let payload else { throw err("missing payload") }
        return RenderingV2.Envelope(protocolVersion: version, id: id, type: type, payload: payload)
    }

    private mutating func displayList() throws -> RenderingV2.DisplayList {
        var renderFormat: String?, unit: String?, colorSpace: String?, extraction: String?
        var projectId: String?, revision: Int?, features: [String]?
        var documents: [RenderingV2.DocumentResource]?, fonts: [RenderingV2.FontResource]?
        var pages: [RenderingV2.Page]?, diagnostics: [RenderingV2.Diagnostic]?
        var navigation: RenderingV2.Navigation?
        try object { key, p in
            switch key {
            case "render_format": renderFormat = try p.string()
            case "coordinate_unit": unit = try p.string()
            case "color_space": colorSpace = try p.string()
            case "text_extraction": extraction = try p.string()
            case "project_id": projectId = try p.string()
            case "revision": revision = try p.int()
            case "required_features": features = try p.array { try $0.string() }
            case "documents": documents = try p.array { try $0.document() }
            case "fonts":
                p.ws()
                let start = p.i
                fonts = try p.array { try $0.font() }
                p.fontsRange = start..<p.i
            case "pages": pages = try p.pages()
            case "diagnostics": diagnostics = try p.array { try $0.diagnostic() }
            case "navigation":
                p.ws()
                if try p.literalNull() { navigation = nil } else { navigation = try p.navigation() }
            default: try p.skip(depth: 2)
            }
        }
        guard let renderFormat, let unit, let colorSpace, let extraction, let projectId, let revision,
              let features, let documents, let fonts, let pages, let diagnostics else { throw err("missing display_list field") }
        return RenderingV2.DisplayList(renderFormat: renderFormat, coordinateUnit: unit, colorSpace: colorSpace, textExtraction: extraction,
                                       projectId: projectId, revision: revision, requiredFeatures: features, documents: documents,
                                       fonts: fonts, pages: pages, diagnostics: diagnostics, navigation: navigation)
    }

    private mutating func document() throws -> RenderingV2.DocumentResource {
        var path: String?, revision: Int?, sha: String?, length: Int64?
        try object { key, p in
            switch key {
            case "path": path = try p.string()
            case "revision": revision = try p.int()
            case "sha256": sha = try p.string()
            case "byte_length": length = try p.int64()
            default: try p.skip(depth: 3)
            }
        }
        guard let path, let revision, let sha, let length else { throw err("missing document field") }
        return RenderingV2.DocumentResource(path: path, revision: revision, sha256: sha, byteLength: length)
    }

    private mutating func font() throws -> RenderingV2.FontResource {
        var id: String?, sha: String?, length: Int64?, format: String?, face: Int?, upem: Int?, count: Int?, name: String?
        try object { key, p in
            switch key {
            case "font_id": id = try p.string()
            case "sha256": sha = try p.string()
            case "byte_length": length = try p.int64()
            case "format": format = try p.string()
            case "face_index": face = try p.int()
            case "units_per_em": upem = try p.int()
            case "glyph_count": count = try p.int()
            case "postscript_name": name = try p.string()
            default: try p.skip(depth: 3)
            }
        }
        guard let id, let sha, let length, let format, let face, let upem, let count, let name else { throw err("missing font field") }
        return RenderingV2.FontResource(fontId: id, sha256: sha, byteLength: length, format: format, faceIndex: face,
                                        unitsPerEm: upem, glyphCount: count, postscriptName: name)
    }

    private mutating func diagnostic() throws -> RenderingV2.Diagnostic {
        var code: String?, message: String?, severity: RenderingV2.Diagnostic.Severity?, sources: [RenderingV2.SourceRange]?
        var suggestion: String?, labels: [RenderingV2.Diagnostic.Label]?, notes: [String]?, help: RenderingV2.Diagnostic.Help?
        try object { key, p in
            switch key {
            case "code": code = try p.string()
            case "message": message = try p.string()
            case "severity":
                let s = try p.string()
                guard let v = RenderingV2.Diagnostic.Severity(rawValue: s) else { throw p.err("unknown severity \(s)") }
                severity = v
            case "sources": sources = try p.array { try $0.sourceRange() }
            case "suggestion": suggestion = try p.optionalString()
            case "labels": labels = try p.optionalArray { try $0.diagnosticLabel() }
            case "notes": notes = try p.optionalArray { try $0.string() }
            case "help": help = try p.optionalDiagnosticHelp()
            default: try p.skip(depth: 3)
            }
        }
        guard let code, let message, let severity, let sources else { throw err("missing diagnostic field") }
        return RenderingV2.Diagnostic(code: code, message: message, severity: severity, sources: sources,
                                      suggestion: suggestion, labels: labels, notes: notes, help: help)
    }

    private mutating func diagnosticLabel() throws -> RenderingV2.Diagnostic.Label {
        var source: RenderingV2.SourceRange?, text: String?, primary: Bool?
        try object { key, p in
            switch key {
            case "source": source = try p.sourceRange()
            case "text": text = try p.string()
            case "primary": primary = try p.bool()
            default: try p.skip(depth: 3)
            }
        }
        guard let source, let text, let primary else { throw err("missing label field") }
        return .init(source: source, text: text, primary: primary)
    }

    private mutating func optionalDiagnosticHelp() throws -> RenderingV2.Diagnostic.Help? {
        ws()
        if try literalNull() { return nil }
        return try diagnosticHelp()
    }

    private mutating func diagnosticHelp() throws -> RenderingV2.Diagnostic.Help {
        var message: String?, replacement: RenderingV2.Diagnostic.Help.Replacement?
        try object { key, p in
            switch key {
            case "message": message = try p.string()
            case "replacement":
                p.ws()
                if try p.literalNull() { replacement = nil }
                else { replacement = try p.diagnosticHelpReplacement() }
            default: try p.skip(depth: 3)
            }
        }
        guard let message else { throw err("missing help message") }
        return .init(message: message, replacement: replacement)
    }

    private mutating func diagnosticHelpReplacement() throws -> RenderingV2.Diagnostic.Help.Replacement {
        var source: RenderingV2.SourceRange?, text: String?
        try object { key, p in
            switch key {
            case "source": source = try p.sourceRange()
            case "text": text = try p.string()
            default: try p.skip(depth: 3)
            }
        }
        guard let source, let text else { throw err("missing help replacement field") }
        return .init(source: source, text: text)
    }

    private mutating func page() throws -> RenderingV2.Page {
        var number: Int?, width: Int64?, height: Int64?, items: [RenderingV2.Item]?
        try object { key, p in
            switch key {
            case "number": number = try p.int()
            case "width": width = try p.int64()
            case "height": height = try p.int64()
            case "items": items = try p.array { try $0.item() }
            default: try p.skip(depth: 3)
            }
        }
        guard let number, let width, let height, let items else { throw err("missing page field") }
        return RenderingV2.Page(number: number, width: width, height: height, items: items)
    }

    /// `pages` array: each element's byte range is recorded; with a reuse hook
    /// installed, the element's exact bytes are offered before decoding.
    private mutating func pages() throws -> [RenderingV2.Page] {
        ws(); try expect(0x5B) // [
        var out: [RenderingV2.Page] = []
        ws()
        if i < b.count, b[i] == 0x5D { i += 1; return out }
        while true {
            ws()
            let start = i
            let end = try skipObject()
            pageRanges.append(start..<end)
            var reused: RenderingV2.Page?
            if let pageReuse, let base = b.baseAddress {
                reused = pageReuse(out.count, UnsafeRawBufferPointer(start: base + start, count: end - start))
            }
            if let reused {
                reusedPages.append(out.count)
                out.append(reused)
            } else {
                i = start
                out.append(try page())
                guard i == end else { throw err("page object range mismatch") }
            }
            ws()
            guard i < b.count else { throw err("unterminated array") }
            if b[i] == 0x2C { i += 1; continue }
            if b[i] == 0x5D { i += 1; return out }
            throw err("expected ',' or ']'")
        }
    }

    /// Skips one object by bracket depth and string boundaries; returns the
    /// offset just past its closing brace (`i` is left there too).
    private mutating func skipObject() throws -> Int {
        guard i < b.count, b[i] == 0x7B else { throw err("expected '{'") }
        var depth = 0
        repeat {
            let c = b[i]
            switch c {
            case 0x22:
                i += 1
                while i < b.count, b[i] != 0x22 { i += b[i] == 0x5C ? 2 : 1 }
                guard i < b.count else { throw err("unterminated string") }
                i += 1
            case 0x7B, 0x5B: depth += 1; i += 1
            case 0x7D, 0x5D: depth -= 1; i += 1
            default: i += 1
            }
        } while depth > 0 && i < b.count
        guard depth == 0 else { throw err("unbalanced object") }
        return i
    }

    private mutating func item() throws -> RenderingV2.Item {
        // All fields of every kind are collected in one pass (keys may come in any order).
        var kind: String?
        var fontId: String?, fontSize: Int64?, text: String?, glyphs: [RenderingV2.Glyph]?, clusters: [RenderingV2.Cluster]?
        var paint: RenderingV2.Paint?
        var x: Int64?, top: Int64?, width: Int64?, height: Int64?
        var sources: [RenderingV2.SourceRange]?, synthetic: String?
        var transform: [Double]?, image: RenderingV2.ImageResource?
        var fillRule: String?, stroke: RenderingV2.Stroke?, commands: [RenderingV2.PathCommand]?, clips: [RenderingV2.ClipPath]?
        let start = i
        try object { key, p in
            switch key {
            case "kind": kind = try p.string()
            case "transform": transform = try p.array { try $0.double() }
            case "image": image = try p.imageResource()
            case "fill_rule": fillRule = try p.string()
            case "stroke": stroke = try p.stroke()
            case "path": commands = try p.commands()
            case "clips": clips = try p.array { try $0.clip() }
            case "font_id": fontId = try p.string()
            case "font_size": fontSize = try p.int64()
            case "text": text = try p.string()
            case "glyphs": glyphs = try p.array { try $0.glyph() }
            case "clusters": clusters = try p.array { try $0.cluster() }
            case "paint": paint = try p.paint()
            case "x": x = try p.int64()
            case "top": top = try p.int64()
            case "width": width = try p.int64()
            case "height": height = try p.int64()
            case "sources": sources = try p.optionalArray { try $0.sourceRange() }
            case "synthetic_reason": synthetic = try p.optionalString()
            default: try p.skip(depth: 4)
            }
        }
        guard let kind else { throw Error(offset: start, message: "item without kind") }
        switch kind {
        case "glyph_run":
            guard let fontId, let fontSize, let text, let glyphs, let clusters, let paint else { throw Error(offset: start, message: "missing glyph_run field") }
            return .glyphRun(RenderingV2.GlyphRun(fontId: fontId, fontSize: fontSize, text: text, glyphs: glyphs, clusters: clusters, paint: paint))
        case "rule":
            guard let x, let top, let width, let height, let paint else { throw Error(offset: start, message: "missing rule field") }
            return .rule(RenderingV2.Rule(x: x, top: top, width: width, height: height, paint: paint, sources: sources, syntheticReason: synthetic))
        case "image":
            guard let x, let top, let width, let height, let transform, let image else { throw Error(offset: start, message: "missing image field") }
            return .image(RenderingV2.Image(x: x, top: top, width: width, height: height, transform: transform, image: image, sources: sources, syntheticReason: synthetic))
        case "path_fill", "path_stroke":
            guard let commands, let paint else { throw Error(offset: start, message: "missing \(kind) field") }
            let op: RenderingV2.Path.Op
            if kind == "path_fill" {
                guard let rule = RenderingV2.FillRule(rawValue: fillRule ?? "nonzero") else { throw Error(offset: start, message: "unknown fill_rule") }
                op = .fill(rule)
            } else {
                guard let stroke else { throw Error(offset: start, message: "path_stroke without stroke") }
                op = .stroke(stroke)
            }
            return .path(RenderingV2.Path(op: op, path: commands, clips: clips ?? [], paint: paint, sources: sources, syntheticReason: synthetic))
        default:
            // Fall back so the slow path reports it after the header checks
            // (a runtime-v1 compile_result must be refused for its version, not its items).
            throw Error(offset: start, message: "item kind '\(kind)' is not supported by the fast reader")
        }
    }

    /// `image` resource object (display-list-v2-images §3); optional fields
    /// absent or `null` are nil, as `JSONDecoder` reads them.
    private mutating func imageResource() throws -> RenderingV2.ImageResource {
        var id: String?, sha: String?, length: Int64?, format: String?, path: String?
        var pw: Int?, ph: Int?, page: Int?, box: [Double]?, rotate: Int?
        try object { key, p in
            switch key {
            case "image_id": id = try p.string()
            case "sha256": sha = try p.string()
            case "byte_length": length = try p.int64()
            case "format": format = try p.string()
            case "path": path = try p.string()
            case "pixel_width": pw = try p.optionalInt()
            case "pixel_height": ph = try p.optionalInt()
            case "pdf_page": page = try p.optionalInt()
            case "pdf_box": box = try p.optionalArray { try $0.double() }
            case "pdf_rotate": rotate = try p.optionalInt()
            default: try p.skip(depth: 5)
            }
        }
        guard let id, let sha, let length, let format, let path else { throw err("missing image resource field") }
        return RenderingV2.ImageResource(imageId: id, sha256: sha, byteLength: length, format: format, path: path,
                                         pixelWidth: pw, pixelHeight: ph, pdfPage: page, pdfBox: box, pdfRotate: rotate)
    }

    /// `[["m",x,y],["l",x,y],["c",x1,y1,x2,y2,x,y],["z"]]` (path-v0).
    private mutating func commands() throws -> [RenderingV2.PathCommand] {
        try array { p in
            p.ws(); try p.expect(0x5B) // [
            let op = try p.string()
            var n: [Int64] = []
            while true {
                p.ws()
                guard p.i < p.b.count else { throw p.err("unterminated path command") }
                if p.b[p.i] == 0x5D { p.i += 1; break }
                try p.expect(0x2C)
                n.append(try p.int64())
            }
            switch (op, n.count) {
            case ("m", 2): return .move(x: n[0], y: n[1])
            case ("l", 2): return .line(x: n[0], y: n[1])
            case ("c", 6): return .cubic(x1: n[0], y1: n[1], x2: n[2], y2: n[3], x: n[4], y: n[5])
            case ("z", 0): return .close
            default: throw p.err("path command '\(op)' with \(n.count) operands")
            }
        }
    }

    private mutating func clip() throws -> RenderingV2.ClipPath {
        var commands: [RenderingV2.PathCommand]?, rule: String?, kind: String?
        try object { key, p in
            switch key {
            case "kind": kind = try p.string()
            case "path": commands = try p.commands()
            case "fill_rule": rule = try p.string()
            default: try p.skip(depth: 5)
            }
        }
        guard kind == nil || kind == "path" else { throw err("clip kind '\(kind ?? "")' is not path") }
        guard let commands, let fillRule = RenderingV2.FillRule(rawValue: rule ?? "nonzero") else { throw err("malformed clip") }
        return RenderingV2.ClipPath(path: commands, fillRule: fillRule)
    }

    private mutating func stroke() throws -> RenderingV2.Stroke {
        var width: Int64?, cap: String?, join: String?, miter: Double?
        var dashArray: [Int64]?, dashPhase: Int64?, dashPresent = false
        try object { key, p in
            switch key {
            case "width": width = try p.int64()
            case "cap": cap = try p.string()
            case "join": join = try p.string()
            case "miter_limit": miter = try p.double()
            case "dash":
                p.ws()
                if try p.literalNull() { break }
                dashPresent = true
                try p.object { k, q in
                    switch k {
                    case "array": dashArray = try q.array { try $0.int64() }
                    case "phase": dashPhase = try q.int64()
                    default: try q.skip(depth: 5)
                    }
                }
            default: try p.skip(depth: 5)
            }
        }
        guard let width, let cap = RenderingV2.LineCap(rawValue: cap ?? ""), let join = RenderingV2.LineJoin(rawValue: join ?? ""), let miter else {
            throw err("malformed stroke")
        }
        var dash: RenderingV2.Dash?
        if dashPresent {
            guard let dashArray, let dashPhase else { throw err("malformed dash") }
            dash = RenderingV2.Dash(array: dashArray, phase: dashPhase)
        }
        return RenderingV2.Stroke(width: width, cap: cap, join: join, miterLimit: miter, dash: dash)
    }

    private mutating func optionalInt() throws -> Int? {
        ws()
        if try literalNull() { return nil }
        return try int()
    }

    private mutating func bool() throws -> Bool {
        ws()
        if i + 4 <= b.count, b[i] == 0x74, b[i + 1] == 0x72, b[i + 2] == 0x75, b[i + 3] == 0x65 { i += 4; return true }
        if i + 5 <= b.count, b[i] == 0x66, b[i + 1] == 0x61, b[i + 2] == 0x6C, b[i + 3] == 0x73, b[i + 4] == 0x65 { i += 5; return false }
        throw err("expected boolean")
    }

    /// `display-list-v2-links` §3; every field optional except that a present
    /// object is kept (empty links/destinations are legal).
    private mutating func navigation() throws -> RenderingV2.Navigation {
        var links: [RenderingV2.Navigation.Link]?
        var destinations: [String: RenderingV2.Navigation.Destination]?
        var outline: [RenderingV2.Navigation.OutlineEntry]?
        var outlineOpen: Bool?, pageMode: String?, openAction: String?
        var info: RenderingV2.Navigation.Info?
        try object { key, p in
            switch key {
            case "links": links = try p.array { try $0.navLink() }
            case "destinations":
                var map: [String: RenderingV2.Navigation.Destination] = [:]
                try p.object { name, q in map[name] = try q.navDestination() }
                destinations = map
            case "outline": outline = try p.array { try $0.navOutlineEntry() }
            case "outline_open": outlineOpen = try p.bool()
            case "page_mode": pageMode = try p.string()
            case "open_action": openAction = try p.string()
            case "info": info = try p.navInfo()
            default: try p.skip(depth: 3)
            }
        }
        return RenderingV2.Navigation(links: links ?? [], destinations: destinations ?? [:], outline: outline,
                                      outlineOpen: outlineOpen, pageMode: pageMode, openAction: openAction, info: info)
    }

    private mutating func navRect() throws -> RenderingV2.Navigation.Rect {
        let n = try array { try $0.int64() }
        guard n.count == 4 else { throw err("link rect must be [x0, y0, x1, y1]") }
        return RenderingV2.Navigation.Rect(x0: n[0], y0: n[1], x1: n[2], y1: n[3])
    }

    private mutating func navLink() throws -> RenderingV2.Navigation.Link {
        var page: Int?, rects: [RenderingV2.Navigation.Rect] = []
        var className: String?, border: [String]?, color: [String]?
        var target: RenderingV2.Navigation.Target?, source: RenderingV2.Navigation.Source?
        try object { key, p in
            switch key {
            case "page": page = try p.int()
            case "rect": rects.append(try p.navRect())
            case "rects": rects.append(contentsOf: try p.array { try $0.navRect() })
            case "class": className = try p.string()
            case "border": border = try p.array { try $0.string() }
            case "color": color = try p.array { try $0.string() }
            case "target": target = try p.navTarget()
            case "source": source = try p.navSource()
            default: try p.skip(depth: 4)
            }
        }
        guard let page, let target, !rects.isEmpty else { throw err("missing link field") }
        return RenderingV2.Navigation.Link(page: page, rects: rects, className: className, border: border, color: color, target: target, source: source)
    }

    private mutating func navTarget() throws -> RenderingV2.Navigation.Target {
        var uri: String?, dest: String?
        try object { key, p in
            switch key {
            case "uri": uri = try p.string()
            case "destination": dest = try p.string()
            default: try p.skip(depth: 5)
            }
        }
        switch (uri, dest) {
        case (let u?, nil): return .uri(u)
        case (nil, let d?): return .destination(d)
        default: throw err("target must be exactly one of uri or destination")
        }
    }

    private mutating func navSource() throws -> RenderingV2.Navigation.Source {
        var document: String?, start: Int?, end: Int?
        try object { key, p in
            switch key {
            case "document": document = try p.string()
            case "start": start = try p.int()
            case "end": end = try p.int()
            default: try p.skip(depth: 5)
            }
        }
        guard let document, let start, let end else { throw err("missing link source field") }
        return RenderingV2.Navigation.Source(document: document, start: start, end: end)
    }

    private mutating func navDestination() throws -> RenderingV2.Navigation.Destination {
        var page: Int?, x: Int64?, y: Int64?, view: String?
        try object { key, p in
            switch key {
            case "page": page = try p.int()
            case "x": x = try p.int64()
            case "y": y = try p.int64()
            case "view": view = try p.string()
            default: try p.skip(depth: 5)
            }
        }
        guard let page, let x, let y else { throw err("missing destination field") }
        return RenderingV2.Navigation.Destination(page: page, x: x, y: y, view: view)
    }

    private mutating func navOutlineEntry() throws -> RenderingV2.Navigation.OutlineEntry {
        var title: String?, level: Int?, destination: String?
        try object { key, p in
            switch key {
            case "title": title = try p.string()
            case "level": level = try p.int()
            case "destination": destination = try p.string()
            default: try p.skip(depth: 4)
            }
        }
        guard let title, let level, let destination else { throw err("missing outline entry field") }
        return RenderingV2.Navigation.OutlineEntry(title: title, level: level, destination: destination)
    }

    private mutating func navInfo() throws -> RenderingV2.Navigation.Info {
        var title: String?, author: String?, subject: String?, keywords: String?, creator: String?
        try object { key, p in
            switch key {
            case "title": title = try p.string()
            case "author": author = try p.string()
            case "subject": subject = try p.string()
            case "keywords": keywords = try p.string()
            case "creator": creator = try p.string()
            default: try p.skip(depth: 4)
            }
        }
        return RenderingV2.Navigation.Info(title: title, author: author, subject: subject, keywords: keywords, creator: creator)
    }

    private mutating func glyph() throws -> RenderingV2.Glyph {
        var gid: Int?, ox: Int64?, by: Int64?, ax: Int64?, ay: Int64?, cluster: Int?
        try object { key, p in
            switch key {
            case "gid": gid = try p.int()
            case "origin_x": ox = try p.int64()
            case "baseline_y": by = try p.int64()
            case "advance_x": ax = try p.int64()
            case "advance_y": ay = try p.int64()
            case "cluster": cluster = try p.int()
            default: try p.skip(depth: 5)
            }
        }
        guard let gid, let ox, let by, let ax, let ay, let cluster else { throw err("missing glyph field") }
        return RenderingV2.Glyph(gid: gid, originX: ox, baselineY: by, advanceX: ax, advanceY: ay, cluster: cluster)
    }

    private mutating func cluster() throws -> RenderingV2.Cluster {
        var start: Int?, end: Int?, rects: [RenderingV2.Rect]?, carets: [RenderingV2.Caret]?
        var sources: [RenderingV2.SourceRange]?, synthetic: String?
        try object { key, p in
            switch key {
            case "text_start_byte": start = try p.int()
            case "text_end_byte": end = try p.int()
            case "hit_rects": rects = try p.array { try $0.rect() }
            case "carets": carets = try p.array { try $0.caret() }
            case "sources": sources = try p.optionalArray { try $0.sourceRange() }
            case "synthetic_reason": synthetic = try p.optionalString()
            default: try p.skip(depth: 5)
            }
        }
        guard let start, let end, let rects, let carets else { throw err("missing cluster field") }
        return RenderingV2.Cluster(textStartByte: start, textEndByte: end, hitRects: rects, carets: carets, sources: sources, syntheticReason: synthetic)
    }

    private mutating func rect() throws -> RenderingV2.Rect {
        var x: Int64?, top: Int64?, width: Int64?, height: Int64?
        try object { key, p in
            switch key {
            case "x": x = try p.int64()
            case "top": top = try p.int64()
            case "width": width = try p.int64()
            case "height": height = try p.int64()
            default: try p.skip(depth: 6)
            }
        }
        guard let x, let top, let width, let height else { throw err("missing rect field") }
        return RenderingV2.Rect(x: x, top: top, width: width, height: height)
    }

    private mutating func caret() throws -> RenderingV2.Caret {
        var byte: Int?, x: Int64?, top: Int64?, height: Int64?
        try object { key, p in
            switch key {
            case "text_byte": byte = try p.int()
            case "x": x = try p.int64()
            case "top": top = try p.int64()
            case "height": height = try p.int64()
            default: try p.skip(depth: 6)
            }
        }
        guard let byte, let x, let top, let height else { throw err("missing caret field") }
        return RenderingV2.Caret(textByte: byte, x: x, top: top, height: height)
    }

    private mutating func sourceRange() throws -> RenderingV2.SourceRange {
        var path: String?, start: Int?, end: Int?
        try object { key, p in
            switch key {
            case "path": path = try p.string()
            case "start_byte": start = try p.int()
            case "end_byte": end = try p.int()
            default: try p.skip(depth: 6)
            }
        }
        guard let path, let start, let end else { throw err("missing source range field") }
        return RenderingV2.SourceRange(path: path, startByte: start, endByte: end)
    }

    private mutating func paint() throws -> RenderingV2.Paint {
        var r: Double?, g: Double?, bl: Double?, a: Double?
        try object { key, p in
            switch key {
            case "r": r = try p.double()
            case "g": g = try p.double()
            case "b": bl = try p.double()
            case "a": a = try p.double()
            default: try p.skip(depth: 5)
            }
        }
        guard let r, let g, let bl, let a else { throw err("missing paint component") }
        return RenderingV2.Paint(r: r, g: g, b: bl, a: a)
    }

    // MARK: primitives

    private func err(_ message: String) -> Error { Error(offset: i, message: message) }

    private mutating func ws() {
        while i < b.count, b[i] == 0x20 || b[i] == 0x0A || b[i] == 0x0D || b[i] == 0x09 { i += 1 }
    }

    private mutating func expect(_ byte: UInt8) throws {
        guard i < b.count, b[i] == byte else { throw err("expected '\(Character(UnicodeScalar(byte)))'") }
        i += 1
    }

    /// `{ "key": value, ... }` — `body` reads each value (and must consume it).
    private mutating func object(_ body: (String, inout RenderingV2Fast) throws -> Void) throws {
        ws(); try expect(0x7B) // {
        ws()
        if i < b.count, b[i] == 0x7D { i += 1; return }
        while true {
            ws()
            let key = try string()
            ws(); try expect(0x3A) // :
            ws()
            try body(key, &self)
            ws()
            guard i < b.count else { throw err("unterminated object") }
            if b[i] == 0x2C { i += 1; continue }
            if b[i] == 0x7D { i += 1; return }
            throw err("expected ',' or '}'")
        }
    }

    private mutating func array<T>(_ element: (inout RenderingV2Fast) throws -> T) throws -> [T] {
        ws(); try expect(0x5B) // [
        var out: [T] = []
        ws()
        if i < b.count, b[i] == 0x5D { i += 1; return out }
        while true {
            ws()
            out.append(try element(&self))
            ws()
            guard i < b.count else { throw err("unterminated array") }
            if b[i] == 0x2C { i += 1; continue }
            if b[i] == 0x5D { i += 1; return out }
            throw err("expected ',' or ']'")
        }
    }

    private mutating func optionalArray<T>(_ element: (inout RenderingV2Fast) throws -> T) throws -> [T]? {
        ws()
        if try literalNull() { return nil }
        return try array(element)
    }

    private mutating func optionalString() throws -> String? {
        ws()
        if try literalNull() { return nil }
        return try string()
    }

    private mutating func literalNull() throws -> Bool {
        guard i + 4 <= b.count, b[i] == 0x6E else { return false }
        guard b[i + 1] == 0x75, b[i + 2] == 0x6C, b[i + 3] == 0x6C else { throw err("invalid literal") }
        i += 4
        return true
    }

    private mutating func string() throws -> String {
        ws(); try expect(0x22) // "
        let start = i
        // Fast path: no escapes → one UTF-8 validation over the slice.
        while i < b.count {
            let c = b[i]
            if c == 0x22 {
                guard let s = String(validatingUTF8Slice: b, start, i) else { throw err("invalid UTF-8 in string") }
                i += 1
                return s
            }
            if c == 0x5C { break }
            if c < 0x20 { throw err("control character in string") }
            i += 1
        }
        // Escapes present: decode into a scalar buffer.
        var out: [UInt8] = Array(b[start..<i])
        while i < b.count {
            let c = b[i]
            if c == 0x22 {
                i += 1
                guard let s = String(bytes: out, encoding: .utf8) else { throw err("invalid UTF-8 in string") }
                return s
            }
            if c < 0x20 { throw err("control character in string") }
            if c != 0x5C { out.append(c); i += 1; continue }
            i += 1
            guard i < b.count else { throw err("unterminated escape") }
            let e = b[i]; i += 1
            switch e {
            case 0x22: out.append(0x22)
            case 0x5C: out.append(0x5C)
            case 0x2F: out.append(0x2F)
            case 0x62: out.append(0x08)
            case 0x66: out.append(0x0C)
            case 0x6E: out.append(0x0A)
            case 0x72: out.append(0x0D)
            case 0x74: out.append(0x09)
            case 0x75:
                var scalar = try hex4()
                if (0xD800...0xDBFF).contains(scalar) {
                    guard i + 1 < b.count, b[i] == 0x5C, b[i + 1] == 0x75 else { throw err("unpaired surrogate") }
                    i += 2
                    let low = try hex4()
                    guard (0xDC00...0xDFFF).contains(low) else { throw err("invalid low surrogate") }
                    scalar = 0x10000 + ((scalar - 0xD800) << 10) + (low - 0xDC00)
                } else if (0xDC00...0xDFFF).contains(scalar) {
                    throw err("unpaired surrogate")
                }
                guard let u = Unicode.Scalar(scalar) else { throw err("invalid scalar") }
                out.append(contentsOf: Array(String(Character(u)).utf8))
            default: throw err("invalid escape")
            }
        }
        throw err("unterminated string")
    }

    private mutating func hex4() throws -> UInt32 {
        guard i + 4 <= b.count else { throw err("short \\u escape") }
        var v: UInt32 = 0
        for _ in 0..<4 {
            let c = b[i]; i += 1
            let d: UInt32
            switch c {
            case 0x30...0x39: d = UInt32(c - 0x30)
            case 0x41...0x46: d = UInt32(c - 0x41 + 10)
            case 0x61...0x66: d = UInt32(c - 0x61 + 10)
            default: throw err("invalid hex digit")
            }
            v = v << 4 | d
        }
        return v
    }

    /// Integer literal (`-?digits`, no fraction/exponent) within `Int64`.
    private mutating func int64() throws -> Int64 {
        ws()
        let start = i
        var negative = false
        if i < b.count, b[i] == 0x2D { negative = true; i += 1 }
        guard i < b.count, (0x30...0x39).contains(b[i]) else { throw err("expected integer") }
        if b[i] == 0x30, i + 1 < b.count, (0x30...0x39).contains(b[i + 1]) { throw err("leading zero") }
        var v: Int64 = 0
        while i < b.count, (0x30...0x39).contains(b[i]) {
            let d = Int64(b[i] - 0x30)
            let (m, o1) = v.multipliedReportingOverflow(by: 10)
            let (s, o2) = negative ? m.subtractingReportingOverflow(d) : m.addingReportingOverflow(d)
            guard !o1, !o2 else { throw Error(offset: start, message: "integer out of range") }
            v = s
            i += 1
        }
        if i < b.count, b[i] == 0x2E || b[i] == 0x65 || b[i] == 0x45 { throw Error(offset: start, message: "integer expected, found fraction/exponent") }
        return v
    }

    private mutating func int() throws -> Int {
        let v = try int64()
        guard let x = Int(exactly: v) else { throw err("integer out of range") }
        return x
    }

    private mutating func bool() throws -> Bool {
        ws()
        if i < b.count, b[i] == 0x74 { try literal("true"); return true }
        if i < b.count, b[i] == 0x66 { try literal("false"); return false }
        throw err("expected boolean")
    }

    /// JSON number → Double via the standard library's correctly rounded parser.
    private mutating func double() throws -> Double {
        ws()
        let start = i
        if i < b.count, b[i] == 0x2D { i += 1 }
        guard i < b.count, (0x30...0x39).contains(b[i]) else { throw err("expected number") }
        if b[i] == 0x30, i + 1 < b.count, (0x30...0x39).contains(b[i + 1]) { throw err("leading zero") }
        while i < b.count, (0x30...0x39).contains(b[i]) { i += 1 }
        if i < b.count, b[i] == 0x2E {
            i += 1
            guard i < b.count, (0x30...0x39).contains(b[i]) else { throw err("digit expected after '.'") }
            while i < b.count, (0x30...0x39).contains(b[i]) { i += 1 }
        }
        if i < b.count, b[i] == 0x65 || b[i] == 0x45 {
            i += 1
            if i < b.count, b[i] == 0x2B || b[i] == 0x2D { i += 1 }
            guard i < b.count, (0x30...0x39).contains(b[i]) else { throw err("digit expected in exponent") }
            while i < b.count, (0x30...0x39).contains(b[i]) { i += 1 }
        }
        guard let text = String(validatingUTF8Slice: b, start, i), let v = Double(text) else { throw Error(offset: start, message: "invalid number") }
        return v
    }

    /// Skips any value (used for unknown keys). Bounded nesting.
    private mutating func skip(depth: Int) throws {
        guard depth < 64 else { throw err("nesting too deep") }
        ws()
        guard i < b.count else { throw err("unexpected end") }
        switch b[i] {
        case 0x7B: try object { _, p in try p.skip(depth: depth + 1) }
        case 0x5B: _ = try array { p in try p.skip(depth: depth + 1) }
        case 0x22: _ = try string()
        case 0x74: try literal("true")
        case 0x66: try literal("false")
        case 0x6E: try literal("null")
        default: _ = try double()
        }
    }

    private mutating func literal(_ word: StaticString) throws {
        let n = word.utf8CodeUnitCount
        guard i + n <= b.count else { throw err("invalid literal") }
        var ok = true
        word.withUTF8Buffer { w in for k in 0..<n where b[i + k] != w[k] { ok = false } }
        guard ok else { throw err("invalid literal") }
        i += n
    }
}

// MARK: display-list-v2-delta (isolated; crates/render-pipeline/docs/proposals/display-list-v2-delta.md r5)

extension RenderingV2Fast {
    /// The full `display_list` envelope plus, for every page, the exact byte
    /// length of its JSON object as it sits on the wire (the writer's bytes;
    /// `page_bytes` of the proposal). Same acceptance as `envelope(_:)`.
    public static func envelopeWithPageBytes(_ data: Data) throws -> (envelope: RenderingV2.Envelope, pageBytes: [Int]) {
        try data.withUnsafeBytes { raw -> (RenderingV2.Envelope, [Int]) in
            var p = RenderingV2Fast(raw.bindMemory(to: UInt8.self))
            p.ws()
            var version: Int?, id: String?, type: String?, payload: RenderingV2.DisplayList?
            var pageBytes: [Int] = []
            try p.object { key, p in
                switch key {
                case "protocol_version": version = try p.int()
                case "id": id = try p.string()
                case "type": type = try p.string()
                case "payload": payload = try p.displayListRecordingPageBytes(&pageBytes)
                default: try p.skip(depth: 1)
                }
            }
            p.ws()
            guard p.i == p.b.count else { throw p.err("trailing characters") }
            guard let version, let id, let type, let payload else { throw p.err("missing envelope field") }
            return (RenderingV2.Envelope(protocolVersion: version, id: id, type: type, payload: payload), pageBytes)
        }
    }

    private mutating func displayListRecordingPageBytes(_ pageBytes: inout [Int]) throws -> RenderingV2.DisplayList {
        var renderFormat: String?, unit: String?, colorSpace: String?, extraction: String?
        var projectId: String?, revision: Int?, features: [String]?
        var documents: [RenderingV2.DocumentResource]?, fonts: [RenderingV2.FontResource]?
        var pages: [RenderingV2.Page]?, diagnostics: [RenderingV2.Diagnostic]?
        var navigation: RenderingV2.Navigation?
        var lengths: [Int] = []
        try object { key, p in
            switch key {
            case "render_format": renderFormat = try p.string()
            case "coordinate_unit": unit = try p.string()
            case "color_space": colorSpace = try p.string()
            case "text_extraction": extraction = try p.string()
            case "project_id": projectId = try p.string()
            case "revision": revision = try p.int()
            case "required_features": features = try p.array { try $0.string() }
            case "documents": documents = try p.array { try $0.document() }
            case "fonts": fonts = try p.array { try $0.font() }
            case "pages":
                pages = try p.array { q in
                    q.ws()
                    let start = q.i
                    let page = try q.page()
                    lengths.append(q.i - start)
                    return page
                }
            case "diagnostics": diagnostics = try p.array { try $0.diagnostic() }
            case "navigation":
                p.ws()
                if try p.literalNull() { navigation = nil } else { navigation = try p.navigation() }
            default: try p.skip(depth: 2)
            }
        }
        guard let renderFormat, let unit, let colorSpace, let extraction, let projectId, let revision,
              let features, let documents, let fonts, let pages, let diagnostics else { throw err("missing display_list field") }
        pageBytes = lengths
        return RenderingV2.DisplayList(renderFormat: renderFormat, coordinateUnit: unit, colorSpace: colorSpace, textExtraction: extraction,
                                       projectId: projectId, revision: revision, requiredFeatures: features, documents: documents,
                                       fonts: fonts, pages: pages, diagnostics: diagnostics, navigation: navigation)
    }

    /// Decoded `display_list_delta` envelope. Raw byte lengths of the header
    /// arrays and of the `id`/`project_id`/`revision` values are recorded so the
    /// consumer can compute the exact full-line size without a JSON writer.
    public struct DeltaEnvelope: Equatable {
        public struct Base: Equatable {
            public var requestId: String, projectId: String, revision: Int, pageCount: Int, listDigest: String
        }
        public struct Relocation: Equatable {
            public var path: String, editStart: Int, editEnd: Int, delta: Int
        }
        public struct ChangedPage: Equatable {
            public var page: RenderingV2.Page
            /// Byte length of the page object on the wire.
            public var wireBytes: Int
        }
        /// Raw wire slice lengths reused verbatim by the exact size formula.
        public struct RawParts: Equatable {
            public var id: Int, projectId: Int, revision: Int, requiredFeatures: Int, documents: Int, fonts: Int, diagnostics: Int
        }
        public var protocolVersion: Int
        public var id: String
        public var type: String
        public var renderFormat: String, coordinateUnit: String, colorSpace: String, textExtraction: String
        public var projectId: String
        public var revision: Int
        public var requiredFeatures: [String]
        public var digestScheme: String
        public var base: Base
        public var documents: [RenderingV2.DocumentResource]
        public var fonts: [RenderingV2.FontResource]
        public var diagnostics: [RenderingV2.Diagnostic]
        public var relocations: [Relocation]
        public var pageCount: Int
        public var pageDigests: [String]
        public var pageBytes: [Int]
        public var changedPages: [ChangedPage]
        public var removedPages: [Int]
        public var listDigest: String
        public var raw: RawParts
    }

    /// Reads one `display_list_delta` envelope. Bounded before indexing: the
    /// page count is capped, and `page_digests`/`page_bytes` must each have
    /// exactly `page_count` entries (checked here, before any use).
    public static func delta(_ data: Data, maxPages: Int) throws -> DeltaEnvelope {
        try data.withUnsafeBytes { raw -> DeltaEnvelope in
            var p = RenderingV2Fast(raw.bindMemory(to: UInt8.self))
            p.ws()
            var version: Int?, id: String?, type: String?
            var out: DeltaEnvelope?
            var rawId = 0
            try p.object { key, p in
                switch key {
                case "protocol_version": version = try p.int()
                case "id":
                    let s = p.i
                    id = try p.string()
                    rawId = p.i - s
                case "type": type = try p.string()
                case "payload": out = try p.deltaPayload(maxPages: maxPages)
                default: try p.skip(depth: 1)
                }
            }
            p.ws()
            guard p.i == p.b.count else { throw p.err("trailing characters") }
            guard let version, let id, let type, var env = out else { throw p.err("missing envelope field") }
            env.protocolVersion = version
            env.id = id
            env.type = type
            env.raw.id = rawId
            return env
        }
    }

    private mutating func deltaPayload(maxPages: Int) throws -> DeltaEnvelope {
        var renderFormat: String?, unit: String?, colorSpace: String?, extraction: String?
        var projectId: String?, revision: Int?, features: [String]?, scheme: String?
        var base: DeltaEnvelope.Base?
        var documents: [RenderingV2.DocumentResource]?, fonts: [RenderingV2.FontResource]?, diagnostics: [RenderingV2.Diagnostic]?
        var relocations: [DeltaEnvelope.Relocation]?
        var pageCount: Int?, pageDigests: [String]?, pageBytes: [Int]?
        var changed: [DeltaEnvelope.ChangedPage]?, removed: [Int]?, listDigest: String?
        var raw = DeltaEnvelope.RawParts(id: 0, projectId: 0, revision: 0, requiredFeatures: 0, documents: 0, fonts: 0, diagnostics: 0)
        try object { key, p in
            switch key {
            case "render_format": renderFormat = try p.string()
            case "coordinate_unit": unit = try p.string()
            case "color_space": colorSpace = try p.string()
            case "text_extraction": extraction = try p.string()
            case "project_id":
                let s = p.i
                projectId = try p.string()
                raw.projectId = p.i - s
            case "revision":
                let s = p.i
                revision = try p.int()
                raw.revision = p.i - s
            case "required_features":
                let s = p.i
                features = try p.array { try $0.string() }
                raw.requiredFeatures = p.i - s
            case "digest_scheme": scheme = try p.string()
            case "base":
                var rid: String?, pid: String?, rev: Int?, count: Int?, digest: String?
                try p.object { k, q in
                    switch k {
                    case "request_id": rid = try q.string()
                    case "project_id": pid = try q.string()
                    case "revision": rev = try q.int()
                    case "page_count": count = try q.int()
                    case "list_digest": digest = try q.string()
                    default: try q.skip(depth: 3)
                    }
                }
                guard let rid, let pid, let rev, let count, let digest else { throw p.err("missing base field") }
                base = DeltaEnvelope.Base(requestId: rid, projectId: pid, revision: rev, pageCount: count, listDigest: digest)
            case "documents":
                let s = p.i
                documents = try p.array { try $0.document() }
                raw.documents = p.i - s
            case "fonts":
                let s = p.i
                fonts = try p.array { try $0.font() }
                raw.fonts = p.i - s
            case "diagnostics":
                let s = p.i
                diagnostics = try p.array { try $0.diagnostic() }
                raw.diagnostics = p.i - s
            case "relocations":
                relocations = try p.array { q in
                    var path: String?, a: Int?, b: Int?, d: Int?
                    try q.object { k, r in
                        switch k {
                        case "path": path = try r.string()
                        case "edit_start": a = try r.int()
                        case "edit_end": b = try r.int()
                        case "delta": d = try r.int()
                        default: try r.skip(depth: 3)
                        }
                    }
                    guard let path, let a, let b, let d, a >= 0, b >= a else { throw q.err("invalid relocation") }
                    return DeltaEnvelope.Relocation(path: path, editStart: a, editEnd: b, delta: d)
                }
            case "page_count":
                let n = try p.int()
                guard n >= 0, n <= maxPages else { throw p.err("page_count outside 0...\(maxPages)") }
                pageCount = n
            case "page_digests": pageDigests = try p.array { try $0.string() }
            case "page_bytes":
                pageBytes = try p.array { q in
                    let v = try q.int()
                    guard v >= 0, Int64(v) <= RenderingV2.maxExactInteger else { throw q.err("page_bytes entry out of range") }
                    return v
                }
            case "changed_pages":
                changed = try p.array { q in
                    q.ws()
                    let start = q.i
                    let page = try q.page()
                    return DeltaEnvelope.ChangedPage(page: page, wireBytes: q.i - start)
                }
            case "removed_pages": removed = try p.array { try $0.int() }
            case "list_digest": listDigest = try p.string()
            default: try p.skip(depth: 2)
            }
        }
        guard let renderFormat, let unit, let colorSpace, let extraction, let projectId, let revision, let features, let scheme,
              let base, let documents, let fonts, let diagnostics, let relocations, let pageCount, let pageDigests, let pageBytes,
              let changed, let removed, let listDigest else { throw err("missing display_list_delta field") }
        guard pageDigests.count == pageCount, pageBytes.count == pageCount else { throw err("page_digests/page_bytes must have page_count entries") }
        guard changed.count <= pageCount else { throw err("more changed pages than page_count") }
        return DeltaEnvelope(protocolVersion: 0, id: "", type: "", renderFormat: renderFormat, coordinateUnit: unit, colorSpace: colorSpace,
                             textExtraction: extraction, projectId: projectId, revision: revision, requiredFeatures: features, digestScheme: scheme,
                             base: base, documents: documents, fonts: fonts, diagnostics: diagnostics, relocations: relocations,
                             pageCount: pageCount, pageDigests: pageDigests, pageBytes: pageBytes, changedPages: changed, removedPages: removed,
                             listDigest: listDigest, raw: raw)
    }
}

private extension String {
    /// Validated UTF-8 from `bytes[start..<end]`, nil when invalid.
    init?(validatingUTF8Slice bytes: UnsafeBufferPointer<UInt8>, _ start: Int, _ end: Int) {
        guard let base = bytes.baseAddress else { self = ""; return }
        let slice = UnsafeBufferPointer(start: base + start, count: end - start)
        // Validate strictly (no U+FFFD substitution) before decoding.
        var it = slice.makeIterator()
        var decoder = UTF8()
        loop: while true {
            switch decoder.decode(&it) {
            case .scalarValue: continue
            case .emptyInput: break loop
            case .error: return nil
            }
        }
        self.init(decoding: slice, as: UTF8.self)
    }
}
