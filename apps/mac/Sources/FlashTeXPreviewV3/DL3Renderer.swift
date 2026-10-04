import CoreGraphics
import CoreText
import Foundation
import CoreImage
import ImageIO
import IOSurface
import ObjectiveC
import FlashTeXDisplayListV3

// The display-list-v3 preview renderer (DESIGN.md §6.2): a decoded page
// drawn through Core Graphics exactly as a PDF viewer paints the engine's
// PDF, so the bitmap equals Core Graphics' own rendering of that PDF
// (`DL3Parity` measures it, zero tolerance).
//
// Type 1 fonts: the FONT resources are the Type 1 programs pdflatex embeds
// (their subsets have the same charstrings), and Core Graphics still loads
// a Type 1 program from memory (`CGFont(CGDataProvider)`), which is also
// the rasteriser that draws the PDF's embedded subsets. So each program is
// loaded once, as sent, and glyphs are drawn by name (`encoding[code]`)
// with the glyph matrix: no conversion, no second rasteriser. A client on
// another platform does the same with FreeType, which reads Type 1 natively;
// the protocol carries nothing platform-specific.
//
// Everything here is safe off the main thread: prepared pages are immutable
// and every raster draws into a private context.

/// sp per bp.
let K = DL3.spPerBp

/// A font program, loaded once per key: Type 1, TrueType and OpenType
/// through Core Graphics (the rasteriser that draws the PDF's embedded
/// programs), Type 3 bitmap fonts as image masks (protocol §5.1.1).
public final class DL3RenderFont: @unchecked Sendable {
    /// One Type 3 glyph: its 1-bit mask and where it sits in glyph space.
    public struct Type3Glyph { public var mask: CGImage?; public var rect: CGRect }

    public let key: String
    /// Outline fonts; nil for Type 3.
    public let cgFont: CGFont?
    /// code → glyph (0: none / .notdef).
    public let glyphs: [CGGlyph]
    /// Type 3: code → mask.
    public let type3: [Int: Type3Glyph]?
    /// Glyph space → text space beyond the program's own FontMatrix
    /// (pdfTeX's `font_matrix` for SlantFont/ExtendFont), else identity.
    /// Type 3: the font's whole `/FontMatrix` (bitmap pixels → text space).
    public let fontTransform: CGAffineTransform
    /// The program's bounding box at size 1 in glyph space (text units),
    /// or nil when it is empty or not finite: then a tile never culls the
    /// font's glyphs (`DL3Renderer.rasterizeTile`).
    public let inkBox: CGRect?

    init(key: String, cgFont: CGFont?, glyphs: [CGGlyph], type3: [Int: Type3Glyph]? = nil, fontTransform: CGAffineTransform) {
        self.key = key; self.cgFont = cgFont; self.glyphs = glyphs; self.type3 = type3; self.fontTransform = fontTransform
        // Tile culling (DL3Tiles): an outline font's bounding box. A Type 3
        // font has none here, so its glyphs are never culled (and its pages
        // are cut from one whole-page raster, `tilesByTranslation`).
        if let cgFont {
            let box = CTFontGetBoundingBox(CTFontCreateWithGraphicsFont(cgFont, 1, nil, nil)).standardized
            inkBox = box.width.isFinite && box.height.isFinite && box.width > 0 && box.height > 0 ? box : nil
        } else {
            inkBox = nil
        }
    }

    /// Whether `code` draws anything.
    public func draws(_ code: Int) -> Bool {
        if let type3 { return type3[code]?.mask != nil }
        return code < glyphs.count && glyphs[code] != 0
    }

    /// Loads `font`'s program, or says why it cannot be drawn from the display list.
    static func load(_ font: DL3Font) -> Result<DL3RenderFont, DL3Error> {
        let label = "font \(font.pdfName ?? "\(font.id)") (\(font.psName ?? font.info["tex_name"]?.string ?? "?"))"
        if let problem = font.problem { return .failure(DL3Error("\(label): \(problem)")) }
        switch font.format {
        case "type3": return loadType3(font, label: label)
        case "type1", "truetype", "opentype": break
        default: return .failure(DL3Error("\(label): format \(font.format ?? "?") has no program to draw"))
        }
        guard !font.program.isEmpty,
              let provider = CGDataProvider(data: Data(font.program) as CFData),
              let cg = CGFont(provider) else {
            return .failure(DL3Error("\(label): the program does not load"))
        }
        var glyphs = [CGGlyph](repeating: 0, count: 256)
        if font.format == "type1" {
            // A code the sent encoding leaves at .notdef draws the program's
            // built-in glyph for it, as a viewer does for a font dictionary
            // without /Encoding (pdfTeX writes none for a font map entry without
            // an .enc). This also covers FONT frames whose `encoding` missed
            // built-in entries written `dup 1/name put` (no space before the
            // slash; engine displaylist/mod.rs builtin_encoding).
            var names: [String?] = font.encoding ?? [String?](repeating: nil, count: 256)
            var builtIn: [String?]?
            for code in 0 ..< 256 where names[code] == nil || names[code] == ".notdef" || names[code]!.isEmpty {
                if builtIn == nil { builtIn = Type1Encoding.builtIn(font.program) }
                names[code] = builtIn![code]
            }
            var byName: [String: CGGlyph] = [:]
            for (code, name) in names.enumerated() where code < 256 {
                guard let name, !name.isEmpty, name != ".notdef" else { continue }
                if let g = byName[name] { glyphs[code] = g; continue }
                let g = cg.getGlyphWithGlyphName(name: name as CFString)
                byName[name] = g
                glyphs[code] = g
            }
        } else {
            // TrueType/OpenType (writettf/writeotf): the glyph named
            // encoding[code] (post table / CFF charset), `uniXXXX` through the
            // Unicode cmap, `indexN` as glyph index N; a subfont: the glyph
            // the cmap subtable gives subfont[code]; a whole font without an
            // encoding: the PDF TrueType rule (the code through the cmap).
            let ct = CTFontCreateWithGraphicsFont(cg, 1, nil, nil)
            func unicodeGlyph(_ u: Int) -> CGGlyph {
                guard let scalar = Unicode.Scalar(UInt32(max(0, u))) else { return 0 }
                var units = Array(String(Character(scalar)).utf16)
                var gs = [CGGlyph](repeating: 0, count: units.count)
                return CTFontGetGlyphsForCharacters(ct, &units, &gs, units.count) ? gs[0] : 0
            }
            if let sub = font.subfont {
                let cm = font.cmap ?? [3, 1]
                guard cm.count == 2, cm[0] == 0 || (cm[0] == 3 && [1, 10].contains(cm[1])) else {
                    return .failure(DL3Error("\(label): subfont cmap \(cm) is not Unicode"))
                }
                for code in 0 ..< min(256, sub.count) where sub[code] >= 0 { glyphs[code] = unicodeGlyph(sub[code]) }
            } else if let names = font.encoding {
                for (code, name) in names.enumerated() where code < 256 {
                    guard let name, !name.isEmpty, name != ".notdef" else { continue }
                    var g = cg.getGlyphWithGlyphName(name: name as CFString)
                    if g == 0, name.hasPrefix("uni"), name.count == 7, let u = Int(name.dropFirst(3), radix: 16) { g = unicodeGlyph(u) }
                    if g == 0, name.hasPrefix("index"), let n = Int(name.dropFirst(5)), n < cg.numberOfGlyphs { g = CGGlyph(n) }
                    glyphs[code] = g
                }
            } else {
                for code in 0 ..< 256 { glyphs[code] = unicodeGlyph(code) }
            }
        }
        var t = CGAffineTransform.identity
        if let fm = font.fontMatrix {
            // The program's FontMatrix is 1/unitsPerEm (what a size-1 font
            // applies); `font_matrix` replaces it.
            let upem = Double(cg.unitsPerEm)
            t = CGAffineTransform(a: fm[0] * upem, b: fm[1] * upem, c: fm[2] * upem, d: fm[3] * upem, tx: fm[4] * upem, ty: fm[5] * upem)
        }
        return .success(DL3RenderFont(key: font.keyHex, cgFont: cg, glyphs: glyphs, fontTransform: t))
    }

    static func loadType3(_ font: DL3Font, label: String) -> Result<DL3RenderFont, DL3Error> {
        guard let fm = font.fontMatrix else { return .failure(DL3Error("\(label): type3 without font_matrix")) }
        let decoded: [DL3Type3Glyph]
        do { decoded = try DL3Type3.decode(font.program) } catch { return .failure(DL3Error("\(label): \(error)")) }
        var out: [Int: Type3Glyph] = [:]
        for g in decoded {
            let rect = CGRect(x: Double(g.llx), y: Double(g.lly), width: Double(g.width), height: Double(g.height))
            guard g.width > 0, g.height > 0, let provider = CGDataProvider(data: Data(g.rows) as CFData),
                  // PDF /ImageMask true /Decode [1 0]: a 1 bit is ink.
                  let mask = CGImage(maskWidth: Int(g.width), height: Int(g.height), bitsPerComponent: 1, bitsPerPixel: 1,
                                     bytesPerRow: g.bytesPerRow, provider: provider, decode: [1, 0], shouldInterpolate: false) else {
                out[Int(g.code)] = Type3Glyph(mask: nil, rect: rect)
                continue
            }
            out[Int(g.code)] = Type3Glyph(mask: mask, rect: rect)
        }
        let t = CGAffineTransform(a: fm[0], b: fm[1], c: fm[2], d: fm[3], tx: fm[4], ty: fm[5])
        return .success(DL3RenderFont(key: font.keyHex, cgFont: nil, glyphs: [], type3: out, fontTransform: t))
    }
}

/// Built-in `/Encoding` of a Type 1 program (`dup N /name put`), for a FONT
/// that sent no `encoding` (the engine always sends one today).
enum Type1Encoding {
    static let entry = try! NSRegularExpression(pattern: #"dup\s+(\d+)\s*/([^\s/\[\]{}()<>%]+)\s+put"#)

    static func builtIn(_ program: [UInt8]) -> [String?] {
        var names = [String?](repeating: nil, count: 256)
        // The cleartext part (before `eexec`; a PFB's first segment).
        var text = String(decoding: program.prefix(256 << 10), as: UTF8.self)
        if let e = text.range(of: "eexec") { text = String(text[..<e.lowerBound]) }
        guard let enc = text.range(of: "/Encoding") else { return names }
        let tail = String(text[enc.upperBound...])
        if tail.hasPrefix(" StandardEncoding") { return names }
        let ns = tail as NSString
        for m in entry.matches(in: tail, range: NSRange(location: 0, length: ns.length)) {
            if let n = Int(ns.substring(with: m.range(at: 1))), n >= 0, n < 256 { names[n] = ns.substring(with: m.range(at: 2)) }
        }
        return names
    }
}

/// An image resource (spec §5.2), decoded once per key.
public final class DL3RenderImage: @unchecked Sendable {
    /// `.pdf`: an included page drawn as pdfTeX's Form XObject draws it:
    /// `box` is its /BBox, the selected page box in the page's own
    /// coordinates (bp), and `matrix` its /Matrix (identity unless the page
    /// has /Rotate 90, 180 or 270).
    public enum Payload { case raster(CGImage), pdf(CGPDFPage, box: CGRect, matrix: CGAffineTransform) }
    public let key: String
    public let payload: Payload
    /// What holding it costs (decoded samples, or the PDF file's size), for
    /// `DL3ResourceCache`'s bound.
    let cost: Int
    /// The document a `.pdf` payload's page belongs to. A `CGPDFPage` does
    /// not retain its document: once the document is released the page draws
    /// nothing (every included PDF page was blank in the preview).
    let document: CGPDFDocument?
    init(key: String, payload: Payload, document: CGPDFDocument? = nil, cost: Int = 0) {
        self.key = key; self.payload = payload; self.document = document; self.cost = cost
    }

    static func load(_ info: DL3JSON) -> Result<DL3RenderImage, DL3Error> {
        guard let file = info["file"]?.string, let type = info["type"]?.string else { return .failure(DL3Error("image without file/type")) }
        let key = info["key"]?.string ?? file
        let url = URL(fileURLWithPath: file)
        switch type {
        case "png", "jpeg", "jpg":
            guard let src = CGImageSourceCreateWithURL(url as CFURL, nil),
                  let img = CGImageSourceCreateImageAtIndex(src, 0, [kCGImageSourceShouldCache: true] as CFDictionary) else {
                return .failure(DL3Error("\(file): cannot decode"))
            }
            return .success(DL3RenderImage(key: key, payload: .raster(deviceSamples(img)), cost: img.bytesPerRow * img.height))
        case "pdf":
            guard let doc = DL3Renderer.openPDF(url), let page = DL3Renderer.page(of: doc, at: Int(info["page"]?.int ?? 1)) else {
                return .failure(DL3Error("\(file): cannot open page \(info["page"]?.int ?? 1)"))
            }
            let box = pdfBox(info, page: page)
            let rotate = Int(info["rotate"]?.int ?? Int64(page.rotationAngle))
            let size = (try? FileManager.default.attributesOfItem(atPath: file)[.size] as? Int) ?? 0
            return .success(DL3RenderImage(key: key, payload: .pdf(page, box: box, matrix: formMatrix(box: box, rotate: rotate)),
                                           document: doc, cost: size))
        default:
            return .failure(DL3Error("\(file): image type \(type) is not drawn"))
        }
    }

    /// pdfTeX's form /Matrix for a page with /Rotate (pdftoepdf.c
    /// `write_epdf`; flashtex-engine `pdftoepdf.rs`): the page turns about
    /// its box, clockwise as /Rotate says, and the turned box keeps its
    /// lower-left corner. Identity for 0 or a rotation that is not a
    /// multiple of 90 (pdfTeX writes no matrix then).
    static func formMatrix(box: CGRect, rotate: Int) -> CGAffineTransform {
        let (x1, y1, x2, y2) = (box.minX, box.minY, box.maxX, box.maxY)
        switch ((rotate % 360) + 360) % 360 {
        case 90: return CGAffineTransform(a: 0, b: -1, c: 1, d: 0, tx: x1 - y1, ty: y1 + x2)
        case 180: return CGAffineTransform(a: -1, b: 0, c: 0, d: -1, tx: x1 + x2, ty: y1 + y2)
        case 270: return CGAffineTransform(a: 0, b: 1, c: -1, d: 0, tx: x1 + y2, ty: y1 - x1)
        default: return .identity
        }
    }

    /// The included page's box in bp, in the page's own coordinates
    /// (protocol §5.2: `orig_x`, `orig_y`, `width`, `height`; pdfTeX's /BBox). A box no PDF page can have (non-positive, or
    /// beyond the 14,400-unit page limit, PDF 32000-1 Annex C) is not used:
    /// a host that sends pdfTeX's internal scaled points (bp × 65,781.76)
    /// would shrink the page to nothing (the title-page logo of a 592-page
    /// book vanished). The box `page_box` names is then read from the file
    /// itself, with the PDF defaults pdfTeX also applies (crop falls back to
    /// media; bleed, trim and art to crop).
    static func pdfBox(_ info: DL3JSON, page: CGPDFPage) -> CGRect {
        let w = info["width"]?.double ?? 0, h = info["height"]?.double ?? 0
        let given = CGRect(x: info["orig_x"]?.double ?? 0, y: info["orig_y"]?.double ?? 0, width: w, height: h)
        let limit = 14_400.0
        if w > 0, h > 0, w <= limit, h <= limit, abs(given.minX) <= limit, abs(given.minY) <= limit { return given }
        let kind: CGPDFBox
        switch info["page_box"]?.string {
        case "media": kind = .mediaBox
        case "bleed": kind = .bleedBox
        case "trim": kind = .trimBox
        case "art": kind = .artBox
        default: kind = .cropBox
        }
        return page.getBoxRect(kind)
    }
}

/// The decoded samples in the PDF's device colour space, as pdfTeX embeds
/// PNG/JPEG data (`/DeviceGray`, `/DeviceRGB`, `/DeviceCMYK`; no gamma, no
/// profile), not interpolated (no `/Interpolate`).
func deviceSamples(_ img: CGImage) -> CGImage {
    let space: CGColorSpace
    switch img.colorSpace?.model {
    case .monochrome?: space = CGColorSpaceCreateDeviceGray()
    case .rgb?: space = CGColorSpaceCreateDeviceRGB()
    case .cmyk?: space = CGColorSpaceCreateDeviceCMYK()
    default: return img
    }
    guard let provider = img.dataProvider,
          let out = CGImage(width: img.width, height: img.height, bitsPerComponent: img.bitsPerComponent, bitsPerPixel: img.bitsPerPixel,
                            bytesPerRow: img.bytesPerRow, space: space, bitmapInfo: img.bitmapInfo, provider: provider,
                            decode: nil, shouldInterpolate: false, intent: .defaultIntent) else { return img }
    return out
}

/// A page (or form) with its resource ids resolved as they stood when it
/// arrived (spec §5: a later rebinding never changes a page already held).
public struct DL3PreparedPage: @unchecked Sendable {
    public var page: DL3Page { didSet { formIDs = Self.formIDs(page) } }
    /// The forms the page draws (`needsPDFFallback(forms:)` runs over every
    /// page at each DONE: it walked every item, ~107 ms on main for 592
    /// pages of text in a debug build).
    public private(set) var formIDs: [UInt32] = []
    static func formIDs(_ page: DL3Page) -> [UInt32] {
        var ids: [UInt32] = []
        for it in page.items { if case .form(let id, _) = it, !ids.contains(id) { ids.append(id) } }
        return ids
    }
    public var fonts: [UInt16: DL3RenderFont]
    public var images: [UInt32: DL3RenderImage]
    /// Why some item cannot be drawn exactly (a font that did not load,
    /// an image that is missing); empty when every item resolved.
    public var problems: [String]

    public init(page: DL3Page, fonts: [UInt16: DL3RenderFont], images: [UInt32: DL3RenderImage], problems: [String] = []) {
        self.page = page; self.fonts = fonts; self.images = images; self.problems = problems
        formIDs = Self.formIDs(page)
    }

    public var widthPt: Double { page.boxWidth }
    public var heightPt: Double { page.boxHeight }
    /// Needs the exported PDF to be pixel-exact: the engine could not
    /// express something (INCOMPLETE), or a resource did not resolve.
    public var needsPDFFallback: Bool { page.incomplete || !problems.isEmpty }

    /// The same, counting the forms the page draws (a form flagged
    /// INCOMPLETE, e.g. beamer's shaded balls, makes its page inexact).
    public func needsPDFFallback(forms: [UInt32: DL3PreparedPage], depth: Int = 0) -> Bool {
        if needsPDFFallback { return true }
        guard depth < 16 else { return false }
        for id in formIDs {
            if let f = forms[id], f.needsPDFFallback(forms: forms, depth: depth + 1) { return true }
        }
        return false
    }
}

/// Resolves ids to loaded resources, caching fonts and images by key across
/// compiles and connections. Thread-safe.
///
/// Images are kept least-recently-used first out, within `imageLimit`
/// entries and `imageByteLimit` bytes (decoded samples, or the PDF file's
/// size: a `CGPDFDocument` keeps its file mapped). A page that holds an image
/// keeps it alive whatever the cache drops; an id bound again re-reads it.
public final class DL3ResourceCache: @unchecked Sendable {
    public static let shared = DL3ResourceCache()
    private let lock = NSLock()
    private var fonts: [String: Result<DL3RenderFont, DL3Error>] = [:]
    private var images: [String: Result<DL3RenderImage, DL3Error>] = [:]
    /// Image keys, least recently used first.
    private var imageOrder: [String] = []
    private var imageBytes = 0
    public let imageLimit: Int
    public let imageByteLimit: Int
    /// Programs by key, so `have_fonts` can name them (the host then sends an empty program).
    private var programs: Set<String> = []

    public init(imageLimit: Int = 256, imageByteLimit: Int = 512 << 20) {
        self.imageLimit = max(1, imageLimit); self.imageByteLimit = max(0, imageByteLimit)
    }

    /// Images held now, and their cost (tests).
    public var heldImages: (count: Int, bytes: Int) { lock.lock(); defer { lock.unlock() }; return (images.count, imageBytes) }

    public var heldFontKeys: [String] { lock.lock(); defer { lock.unlock() }; return programs.sorted() }

    public func font(_ f: DL3Font) -> Result<DL3RenderFont, DL3Error> {
        let key = f.keyHex
        lock.lock()
        if let hit = fonts[key] { lock.unlock(); return hit }
        lock.unlock()
        if f.program.isEmpty && f.format != "none" && f.problem == nil {
            return .failure(DL3Error("font \(f.pdfName ?? "\(f.id)"): the host sent no program for a key this client does not hold"))
        }
        let r = DL3RenderFont.load(f)
        lock.lock()
        fonts[key] = r
        if case .success = r { programs.insert(key) }
        lock.unlock()
        return r
    }

    public func image(_ info: DL3JSON) -> Result<DL3RenderImage, DL3Error> {
        let key = (info["key"]?.string ?? "") + "|" + (info["file"]?.string ?? "")
        lock.lock()
        if let hit = images[key] {
            if let i = imageOrder.lastIndex(of: key) { imageOrder.remove(at: i) }
            imageOrder.append(key)
            lock.unlock()
            return hit
        }
        lock.unlock()
        let r = DL3RenderImage.load(info)
        lock.lock()
        if images.updateValue(r, forKey: key) == nil {
            imageOrder.append(key)
            imageBytes += Self.cost(r)
        }
        // Drop the least recently used, never the one just loaded.
        while imageOrder.count > 1, imageOrder.count > imageLimit || imageBytes > imageByteLimit {
            let old = imageOrder.removeFirst()
            if let gone = images.removeValue(forKey: old) { imageBytes -= Self.cost(gone) }
        }
        lock.unlock()
        return r
    }

    private static func cost(_ r: Result<DL3RenderImage, DL3Error>) -> Int {
        if case .success(let i) = r { return i.cost }
        return 0
    }
}

/// Per-connection resource ids (spec §5): a FONT/IMAGE (re)binds an id; a
/// page resolves its ids when it arrives.
public struct DL3Bindings: Sendable {
    public var fonts: [UInt16: Result<DL3RenderFont, DL3Error>] = [:]
    public var images: [UInt32: Result<DL3RenderImage, DL3Error>] = [:]
    public init() {}

    public mutating func bind(font: DL3Font, cache: DL3ResourceCache) { fonts[font.id] = cache.font(font) }
    public mutating func bind(image: DL3JSON, cache: DL3ResourceCache) {
        guard let id = image["id"]?.int else { return }
        images[UInt32(truncatingIfNeeded: id)] = cache.image(image)
    }
    public mutating func reset() { fonts = [:]; images = [:] }

    public func prepare(_ page: DL3Page) -> DL3PreparedPage {
        var usedFonts: [UInt16: DL3RenderFont] = [:], usedImages: [UInt32: DL3RenderImage] = [:], problems: [String] = []
        var seenFonts = Set<UInt16>(), seenImages = Set<UInt32>()
        for it in page.items {
            switch it {
            case .glyph(let f, _, _, _, _):
                guard seenFonts.insert(f).inserted else { continue }
                switch fonts[f] {
                case .success(let rf)?: usedFonts[f] = rf
                case .failure(let e)?: problems.append(e.message)
                case nil: problems.append("font id \(f) was never sent")
                }
            case .image(let id, _):
                guard seenImages.insert(id).inserted else { continue }
                switch images[id] {
                case .success(let ri)?: usedImages[id] = ri
                case .failure(let e)?: problems.append(e.message)
                case nil: problems.append("image id \(id) was never sent")
                }
            default: break
            }
        }
        return DL3PreparedPage(page: page, fonts: usedFonts, images: usedImages, problems: problems)
    }
}

/// How a page is painted on screen. `.light` is the PDF exactly (the
/// zero-tolerance parity gate); `.dark` is a reading aid: a dark page, and
/// every colour the page's own items set (text, rules, paths) with its
/// lightness inverted and its hue and saturation kept (black ink becomes
/// near-white, a dark blue link becomes a light blue, a pale box becomes a
/// dark one). Images keep their own pixels.
public enum DL3Appearance: Sendable, Equatable {
    case light, dark

    /// The page ground.
    public var background: CGColor {
        switch self {
        case .light: CGColor(gray: 1, alpha: 1)
        case .dark: CGColor(gray: DL3Appearance.groundLightness, alpha: 1)
        }
    }

    /// The dark ground's lightness: white ink maps onto it, so a page's own
    /// white boxes (beamer's background) become the ground, not black.
    public static let groundLightness = 0.125

    /// Lightness inversion in HSL onto [ground, 1], hue and saturation kept.
    /// `minLightness`: text keeps at least this lightness (a pure blue link,
    /// lightness 0.5, would otherwise stay dark blue on the dark ground).
    public static func darken(r: Double, g: Double, b: Double, minLightness: Double = 0) -> (Double, Double, Double) {
        let mx = max(r, g, b), mn = min(r, g, b)
        let l = (mx + mn) / 2, d = mx - mn
        let inverted = max(minLightness, groundLightness + (1 - groundLightness) * (1 - l))
        guard d > 1e-9 else { return (inverted, inverted, inverted) }
        let sat = l > 0.5 ? d / (2 - mx - mn) : d / (mx + mn)
        var h: Double
        if mx == r { h = (g - b) / d + (g < b ? 6 : 0) } else if mx == g { h = (b - r) / d + 2 } else { h = (r - g) / d + 4 }
        h /= 6
        let l2 = inverted
        let q = l2 < 0.5 ? l2 * (1 + sat) : l2 + sat - l2 * sat, pp = 2 * l2 - q
        func hue(_ t0: Double) -> Double {
            var t = t0
            if t < 0 { t += 1 }
            if t > 1 { t -= 1 }
            if t < 1.0 / 6 { return pp + (q - pp) * 6 * t }
            if t < 1.0 / 2 { return q }
            if t < 2.0 / 3 { return pp + (q - pp) * (2.0 / 3 - t) * 6 }
            return pp
        }
        return (hue(h + 1.0 / 3), hue(h), hue(h - 1.0 / 3))
    }
}

public enum DL3Renderer {
    /// Colour spaces as a PDF renderer maps the PDF's device spaces.
    static let gray = CGColorSpaceCreateDeviceGray()
    static let rgb = CGColorSpaceCreateDeviceRGB()
    static let cmyk = CGColorSpaceCreateDeviceCMYK()

    static func color(_ c: [Double], _ appearance: DL3Appearance = .light, text: Bool = false) -> CGColor {
        if appearance == .dark {
            let (r, g, b): (Double, Double, Double) = {
                switch c.count {
                case 1: return (c[0], c[0], c[0])
                case 3: return (c[0], c[1], c[2])
                default: // CMYK → RGB (naive), then as RGB
                    let k = c[3]
                    return ((1 - c[0]) * (1 - k), (1 - c[1]) * (1 - k), (1 - c[2]) * (1 - k))
                }
            }()
            let d = DL3Appearance.darken(r: r, g: g, b: b, minLightness: text ? 0.72 : 0)
            return CGColor(srgbRed: d.0, green: d.1, blue: d.2, alpha: 1)
        }
        let comps = c.map { CGFloat($0) } + [1]
        let space = c.count == 1 ? gray : c.count == 3 ? rgb : cmyk
        return comps.withUnsafeBufferPointer { CGColor(colorSpace: space, components: $0.baseAddress!)! }
    }

    static func affine(_ m: DL3Matrix) -> CGAffineTransform { CGAffineTransform(a: m.a, b: m.b, c: m.c, d: m.d, tx: m.e, ty: m.f) }

    struct GState {
        var fill: CGColor; var stroke: CGColor; var textRender: UInt8
        /// Dark appearance: the fill colour for text, kept readable on the ground.
        var textFill: CGColor?
    }

    /// Draws `prepared` into `ctx`, whose user space is the page's PDF user
    /// space (bp, y up, the MediaBox's origin at the context's origin).
    /// `forms` are the forms the page may draw (by id).
    public static func draw(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], in ctx: CGContext,
                            appearance: DL3Appearance = .light) {
        draw(prepared, forms: forms, in: ctx, appearance: appearance, cull: [])
    }

    /// `cull`: the context is clipped to these rects (user space, bp):
    /// glyphs whose ink meets none of them are skipped, nothing else changes
    /// (`clippedTiles`). Empty: draw everything.
    static func draw(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], in ctx: CGContext,
                     appearance: DL3Appearance = .light, cull: [CGRect]) {
        ctx.saveGState()
        ctx.translateBy(x: -prepared.page.box[0], y: -prepared.page.box[1])
        drawStream(prepared, forms: forms, in: ctx, depth: 0, appearance: appearance, cull: cull)
        ctx.restoreGState()
    }

    /// `tile`: the context is a tile of the page raster (`rasterizeTile`):
    /// items outside it are skipped and glyph origins and rule edges are
    /// handed over as the whole page rounds them. `cull`: see `draw`.
    static func drawStream(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage], in ctx: CGContext, depth: Int,
                           appearance: DL3Appearance = .light, tile: Tile? = nil, cull: [CGRect] = []) {
        let page = prepared.page
        let H = page.box[3]
        let black = color([0], appearance)
        var gs = GState(fill: black, stroke: black, textRender: 0, textFill: appearance == .dark ? color([0], appearance, text: true) : nil)
        var inText = false // dark: the text fill is set on the context
        var stack: [GState] = []
        var glyphMatrix = CGAffineTransform.identity
        var lastFont: DL3RenderFont?
        var ink = TileInk()
        // The k-th GLYPH's exact origin (ORIGINS, spec §4.2), when the writer sent them.
        let origins = page.origins
        var glyphIndex = 0
        // The k-th RULE's geometry (RULE_GEOMETRY, spec §4.4), when sent.
        let ruleGeometry = page.ruleGeometry
        var ruleIndex = 0
        ctx.setFillColor(black)
        ctx.setStrokeColor(black)
        ctx.setTextDrawingMode(.fill)
        for item in page.items {
            switch item {
            case .glyph(let f, let code, let x, let y, _):
                let gi = glyphIndex
                glyphIndex += 1
                // Stream space (bp, y up): the PDF's own origin, else its sp rounding.
                let ox: Double, oy: Double
                if gi < origins.count { ox = origins[gi].x; oy = origins[gi].y } else { ox = Double(x) / K; oy = snap(H - Double(y) / K) }
                guard gs.textRender != 3, let font = prepared.fonts[f], Int(code) < 256, font.draws(Int(code)) else { continue }
                if let t = gs.textFill, !inText { ctx.setFillColor(t); inText = true }
                if let t3 = font.type3?[Int(code)], let mask = t3.mask {
                    // Type 3 (writet3): the glyph procedure's `w 0 0 h llx lly cm`
                    // image mask, through the FontMatrix and the text matrix.
                    ctx.saveGState()
                    ctx.concatenate(CGAffineTransform(a: glyphMatrix.a, b: glyphMatrix.b, c: glyphMatrix.c, d: glyphMatrix.d,
                                                      tx: ox, ty: oy))
                    ctx.concatenate(font.fontTransform)
                    // Core Graphics smooths a Type 3 glyph's mask as .medium does
                    // (measured on the PK documents: .none/.low/.default differ).
                    ctx.interpolationQuality = .medium
                    ctx.draw(mask, in: t3.rect)
                    ctx.restoreGState()
                    lastFont = nil
                    continue
                }
                guard let cgFont = font.cgFont else { continue }
                let g = font.glyphs[Int(code)]
                var tm = font.fontTransform.concatenating(glyphMatrix)
                tm.tx = ox
                tm.ty = oy
                if !cull.isEmpty, !cull.contains(where: { ink.meets($0, font: font, matrix: tm) }) { continue }
                if let tile {
                    guard ink.meets(tile.visible, font: font, matrix: tm) else { continue }
                    tm.tx = tileCoordinate(tm.tx, scale: tile.scale)
                    tm.ty = tileCoordinate(tm.ty, scale: tile.scale)
                }
                if lastFont !== font { ctx.setFont(cgFont); ctx.setFontSize(1); lastFont = font }
                ctx.textMatrix = tm
                ctx.showGlyphs([g], at: [.zero])
            case .rule(let kind, let x, let y, let w, let h):
                if inText { ctx.setFillColor(gs.fill); inText = false }
                let ri = ruleIndex
                ruleIndex += 1
                if ri < ruleGeometry.count {
                    // As the PDF draws it: the CTM's translation, then `re f`, or `m l S`.
                    let g = ruleGeometry[ri]
                    if let tile, kind == .fill { // (a tile by translation has only filled rules)
                        // The page's edges: `re`'s corners through the CTM's translation.
                        let s = tile.scale
                        func edge(_ t: Double, _ v: Double) -> Double { Double(Float(s * v + s * t)) / s }
                        tileRule(left: edge(g[0], g[2]), right: edge(g[0], g[2] + g[4]),
                                 top: edge(g[1], g[3] + g[5]), bottom: edge(g[1], g[3]), tile: tile, in: ctx)
                        continue
                    }
                    ctx.saveGState()
                    ctx.concatenate(CGAffineTransform(a: 1, b: 0, c: 0, d: 1, tx: g[0], ty: g[1]))
                    ctx.beginPath()
                    if kind == .fill {
                        ctx.addRect(CGRect(x: g[2], y: g[3], width: g[4], height: g[5]))
                        ctx.fillPath()
                    } else {
                        ctx.setLineWidth(g[6]); ctx.setLineCap(.butt)
                        ctx.move(to: CGPoint(x: g[2], y: g[3])); ctx.addLine(to: CGPoint(x: g[4], y: g[5]))
                        ctx.strokePath()
                    }
                    ctx.restoreGState()
                    continue
                }
                let left = snap(Double(x) / K), top = snap(H - Double(y) / K)
                let right = snap(Double(x + w) / K), bottom = snap(H - Double(y + h) / K)
                if let tile, kind == .fill { // (a tile by translation has only filled rules)
                    tileRule(left: left, right: right, top: top, bottom: bottom, tile: tile, in: ctx)
                    continue
                }
                switch kind {
                case .fill:
                    ctx.beginPath()
                    ctx.addRect(CGRect(x: left, y: bottom, width: right - left, height: top - bottom))
                    ctx.fillPath()
                case .strokeH:
                    let mid = (top + bottom) / 2
                    ctx.saveGState()
                    ctx.setLineWidth(top - bottom); ctx.setLineCap(.butt)
                    ctx.beginPath(); ctx.move(to: CGPoint(x: left, y: mid)); ctx.addLine(to: CGPoint(x: right, y: mid))
                    ctx.strokePath()
                    ctx.restoreGState()
                case .strokeV:
                    let mid = (left + right) / 2
                    ctx.saveGState()
                    ctx.setLineWidth(right - left); ctx.setLineCap(.butt)
                    ctx.beginPath(); ctx.move(to: CGPoint(x: mid, y: bottom)); ctx.addLine(to: CGPoint(x: mid, y: top))
                    ctx.strokePath()
                    ctx.restoreGState()
                }
            case .path(let n):
                if inText { ctx.setFillColor(gs.fill); inText = false }
                paint(page.paths[Int(n)], page: page, in: ctx)
            case .clip(let n):
                let p = page.paths[Int(n)]
                var t = affine(page.matrix(p.matrix))
                guard let cg = cgPath(p.segs).copy(using: &t) else { continue }
                ctx.beginPath()
                ctx.addPath(cg)
                ctx.clip(using: p.paint & DL3Path.Paint.clipEvenOdd != 0 ? .evenOdd : .winding)
            case .image(let id, let m):
                guard let img = prepared.images[id] else { continue }
                ctx.saveGState()
                ctx.concatenate(affine(page.matrix(m)))
                // Every tile walks the whole page: an image outside this
                // tile is skipped (a PDF one would cost its document's draw).
                switch img.payload {
                case .raster: if Self.outsideClip(CGRect(x: 0, y: 0, width: 1, height: 1), in: ctx) { ctx.restoreGState(); continue }
                case .pdf(_, let box, let matrix): if Self.outsideClip(box.applying(matrix), in: ctx) { ctx.restoreGState(); continue }
                }
                switch img.payload {
                case .raster(let cgImage):
                    ctx.interpolationQuality = .default // what Core Graphics uses for a PDF image without /Interpolate (measured: .none and .medium/.high differ)
                    // Core Graphics draws a PDF's image XObject whose device
                    // transform is axis-aligned (0/90/180/270°) without edge
                    // antialiasing: an edge pixel the image only partly covers
                    // is all image, not a blend with the page (measured: 0 px
                    // apart that way at 1–4.25 px/pt, ~1,500 edge pixels apart
                    // antialiased at the pane's 2.86 px/pt; lane BEAMER-V3).
                    // A rotated image keeps its antialiased edges, as there.
                    if Self.axisAligned(ctx.ctm) { ctx.setShouldAntialias(false) }
                    ctx.draw(cgImage, in: CGRect(x: 0, y: 0, width: 1, height: 1))
                case .pdf(let pdfPage, let box, let matrix):
                    // pdfTeX includes a PDF page as a Form XObject: its space
                    // is the page's own coordinates (bp), its /BBox the
                    // selected page box there, its /Matrix the rotation (if
                    // any), and its `cm` (this item's matrix) already carries
                    // the scaled −origin. So: the form matrix, the clip to
                    // the box, then the page's content stream as it is
                    // (`drawPDFPage` neither rotates nor clips).
                    ctx.concatenate(matrix)
                    ctx.clip(to: box)
                    DL3Renderer.drawPDFPage(pdfPage, in: ctx)
                }
                ctx.restoreGState()
                lastFont = nil
            case .form(let id, let m):
                guard depth < 16, let form = forms[id] else { continue }
                ctx.saveGState()
                ctx.concatenate(affine(page.matrix(m)))
                let b = form.page.box
                ctx.clip(to: CGRect(x: b[0], y: b[1], width: b[2] - b[0], height: b[3] - b[1]))
                // A form starts from the graphics state where it is drawn (PDF `Do`),
                // but its items set their own colours; mirror the current ones.
                ctx.setFillColor(gs.fill); ctx.setStrokeColor(gs.stroke); inText = false
                drawStream(form, forms: forms, in: ctx, depth: depth + 1, appearance: appearance)
                ctx.restoreGState()
                lastFont = nil
                ctx.setFillColor(gs.fill); ctx.setStrokeColor(gs.stroke); inText = false
            case .save:
                stack.append(gs)
                ctx.saveGState()
            case .restore:
                if let s = stack.popLast() { gs = s }
                ctx.restoreGState()
                lastFont = nil
                ctx.setFillColor(gs.fill); ctx.setStrokeColor(gs.stroke); inText = false
                ctx.setTextDrawingMode(mode(gs.textRender))
            case .fillColor(let c):
                gs.fill = color(c, appearance); ctx.setFillColor(gs.fill); inText = false
                if appearance == .dark { gs.textFill = color(c, appearance, text: true) }
            case .strokeColor(let c):
                gs.stroke = color(c, appearance); ctx.setStrokeColor(gs.stroke)
            case .matrix(let n):
                let m = page.matrix(n)
                glyphMatrix = CGAffineTransform(a: m.a, b: m.b, c: m.c, d: m.d, tx: 0, ty: 0)
            case .span, .unsupported:
                break
            case .textRender(let m):
                gs.textRender = m
                ctx.setTextDrawingMode(mode(m))
            }
        }
    }

    /// Draws a PDF page (a fallback page, an included PDF image) from this
    /// thread's own copy of its document when the document was opened with
    /// `openPDF` (every document the app draws from), else from the page as
    /// given. Core Graphics draws one `CGPDFDocument`'s pages wrongly when
    /// two threads use it at once: shadings (beamer's balls, headlines and
    /// frame titles) came out up to 75 levels off, at random, and the pane
    /// draws fallback pages on a concurrent raster queue, the tile queue and
    /// `concurrentPerform` tile batches at the same time (lane BEAMER-V3). A
    /// lock around the draw was not enough (3 of ~52 test runs still failed:
    /// the page lookup and box reads of the shared document stayed
    /// concurrent). Each page drawn from its own document is exact, so each
    /// thread opens its own document from the same bytes, once, and keeps the
    /// last ones it drew (`PDFThreadDocuments`).
    ///
    /// A CGPDFPage does not keep its document: the session lets go of the
    /// fallback document when a DONE replaces it, while a raster job may
    /// still hold its pages (`page.document` is then nil). A page taken with
    /// `page(of:at:)` carries the bytes too, so such a page is still drawn
    /// from this thread's own document, never from the shared page;
    /// a page whose own document cannot be opened is not drawn. A page not
    /// opened with `openPDF` is drawn as given (its caller draws it from one
    /// thread). Returns whether the page was drawn.
    @discardableResult
    public static func drawPDFPage(_ page: CGPDFPage, in ctx: CGContext) -> Bool {
        guard let bytes = DL3PDFBytes.of(page) ?? page.document.flatMap(DL3PDFBytes.of) else {
            ctx.drawPDFPage(page)
            return true
        }
        guard let own = PDFThreadDocuments.current.document(for: bytes)?.page(at: page.pageNumber) else { return false }
        ctx.drawPDFPage(own)
        return true
    }

    /// Whether `rect` (user space) lies wholly outside what `ctx` can draw
    /// into: its clip, which in a bitmap context is at most the bitmap (a
    /// tile). Compared in device space, the clip grown by a pixel, so an
    /// image that could touch any pixel of the context is drawn.
    static func outsideClip(_ rect: CGRect, in ctx: CGContext) -> Bool {
        let clip = ctx.boundingBoxOfClipPath
        guard !clip.isNull, !clip.isInfinite, !rect.isNull, !rect.isInfinite else { return clip.isNull }
        let device = ctx.convertToDeviceSpace(clip).insetBy(dx: -1, dy: -1)
        return !ctx.convertToDeviceSpace(rect).intersects(device)
    }

    /// Opens a PDF for drawing: the file's bytes are read once (a later
    /// rewrite of the file does not change what is drawn) and kept with the
    /// document, so `drawPDFPage` can open a private copy per thread.
    public static func openPDF(_ url: URL) -> CGPDFDocument? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        return openPDF(data: data)
    }

    public static func openPDF(data: Data) -> CGPDFDocument? {
        guard let provider = CGDataProvider(data: data as CFData), let doc = CGPDFDocument(provider) else { return nil }
        DL3PDFBytes.attach(DL3PDFBytes(data), to: doc)
        return doc
    }

    /// Page `k` (1-based) of `doc`, carrying the document's bytes when it
    /// was opened with `openPDF` (a page outlives its document's last
    /// reference: `drawPDFPage`). Only the pages taken get them: opening
    /// the compile's PDF at a DONE stays O(1) in its pages (it attached
    /// them to every page up front, on the main thread).
    public static func page(of doc: CGPDFDocument, at k: Int) -> CGPDFPage? {
        guard let p = doc.page(at: k) else { return nil }
        if let bytes = DL3PDFBytes.of(doc), DL3PDFBytes.of(p) == nil { DL3PDFBytes.attach(bytes, to: p) }
        return p
    }

    /// Whether `m` maps the unit square's edges onto device rows and columns
    /// (no rotation, or a quarter turn; flips included).
    static func axisAligned(_ m: CGAffineTransform) -> Bool { (m.b == 0 && m.c == 0) || (m.a == 0 && m.d == 0) }

    /// A position rounded to sp is within 7.6e-6 bp of the PDF's. pdfTeX
    /// writes positions with three decimals, so a value that close to the
    /// 0.001 grid is the PDF's exact number: restoring it keeps a glyph that
    /// sits exactly on a rasteriser's subpixel boundary on the PDF's side of
    /// it (measured: whole lines otherwise shift one subpixel step).
    @inline(__always) static func snap(_ v: Double) -> Double {
        let g = (v * 1000).rounded() / 1000
        return abs(g - v) < 8e-6 ? g : v
    }

    static func mode(_ m: UInt8) -> CGTextDrawingMode {
        switch m { case 1: .stroke; case 2: .fillStroke; case 3: .invisible; default: .fill }
    }

    static func cgPath(_ segs: [DL3Seg]) -> CGPath {
        let p = CGMutablePath()
        for s in segs {
            switch s {
            case .move(let x, let y): p.move(to: CGPoint(x: x, y: y))
            case .line(let x, let y): p.addLine(to: CGPoint(x: x, y: y))
            case .curve(let a, let b, let c, let d, let e, let f):
                p.addCurve(to: CGPoint(x: e, y: f), control1: CGPoint(x: a, y: b), control2: CGPoint(x: c, y: d))
            case .close: p.closeSubpath()
            }
        }
        return p
    }

    static func paint(_ p: DL3Path, page: DL3Page, in ctx: CGContext) {
        let fill = p.paint & (DL3Path.Paint.fill | DL3Path.Paint.fillEvenOdd) != 0
        let evenOdd = p.paint & DL3Path.Paint.fillEvenOdd != 0
        let stroke = p.paint & DL3Path.Paint.stroke != 0
        guard fill || stroke else { return }
        ctx.saveGState()
        ctx.concatenate(affine(page.matrix(p.matrix)))
        ctx.beginPath()
        ctx.addPath(cgPath(p.segs))
        if let s = p.stroke {
            ctx.setLineWidth(s.width)
            ctx.setLineCap(s.cap == 1 ? .round : s.cap == 2 ? .square : .butt)
            ctx.setLineJoin(s.join == 1 ? .round : s.join == 2 ? .bevel : .miter)
            ctx.setMiterLimit(s.miter)
            ctx.setLineDash(phase: s.phase, lengths: s.dash.map { CGFloat($0) })
        }
        if fill && stroke { ctx.drawPath(using: evenOdd ? .eoFillStroke : .fillStroke) }
        else if fill { ctx.fillPath(using: evenOdd ? .evenOdd : .winding) }
        else { ctx.strokePath() }
        ctx.restoreGState()
    }

    /// A fresh sRGB bitmap context in PDF space (y up) for a page at
    /// `scale` pixels per point, filled white: the configuration the v2
    /// renderer's zero-tolerance parity was measured in.
    /// Pixel layouts: `.rgba` (premultiplied-last, what the parity tests
    /// compare) and `.screen` (BGRA premultiplied-first, little-endian 32-bit:
    /// Core Animation's native layout, so installing a bitmap as layer
    /// contents needs no conversion at commit). Same colour space and
    /// rasteriser, so the same pixels (checked by `FlashTeXPreviewV3Tests`).
    public enum Layout: Sendable { case rgba, screen
        var bitmapInfo: UInt32 {
            switch self {
            case .rgba: CGImageAlphaInfo.premultipliedLast.rawValue
            case .screen: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue
            }
        }
    }

    /// `background`: the ground, over every pixel (the page's partial last
    /// column and row included), as `rasterizeToSurface` lays it.
    public static func bitmapContext(widthPt: Double, heightPt: Double, scale: Double, layout: Layout = .rgba,
                                     smoothFonts: Bool = false, background: CGColor = CGColor(gray: 1, alpha: 1)) -> CGContext? {
        let w = Int((widthPt * scale).rounded(.up)), h = Int((heightPt * scale).rounded(.up))
        guard w > 0, h > 0,
              let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: layout.bitmapInfo) else { return nil }
        ctx.setFillColor(background)
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        ctx.scaleBy(x: scale, y: scale)
        ctx.setShouldAntialias(true)
        setFontSmoothing(smoothFonts, in: ctx)
        ctx.setAllowsFontSubpixelPositioning(true)
        ctx.setShouldSubpixelPositionFonts(true)
        return ctx
    }

    /// Settings > "Smooth fonts in preview". Off (the default) is the
    /// configuration zero-tolerance parity is measured in, and leaves the
    /// context exactly as before the setting existed. On draws glyphs with
    /// Core Graphics font smoothing, the way Preview.app draws the exported
    /// PDF; ligatures such as fi/ffi then differ from the parity reference.
    public static func setFontSmoothing(_ on: Bool, in ctx: CGContext) {
        if on { ctx.setAllowsFontSmoothing(true) } // without it, `setShouldSmoothFonts(true)` draws nothing different
        ctx.setShouldSmoothFonts(on)
    }

    /// One page's bitmap (off-main safe).
    public static func rasterize(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double, layout: Layout = .rgba,
                                 appearance: DL3Appearance = .light, smoothFonts: Bool = false) -> CGImage? {
        // The ground over every pixel: a dark page's partial last column and
        // row are dark too, as `rasterizeToSurface`'s (and its tiles').
        guard let ctx = bitmapContext(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale, layout: layout,
                                      smoothFonts: smoothFonts, background: appearance.background) else { return nil }
        draw(prepared, forms: forms, in: ctx, appearance: appearance)
        return ctx.makeImage()
    }

    /// The page drawn straight into an IOSurface (BGRA, `.screen` layout,
    /// tagged sRGB): a layer shows it without the copy Core Animation makes
    /// of a CGImage at commit (measured 3.5 ms for a 1.4-megapixel page).
    /// Same context configuration, so the same pixels as `rasterize`.
    public static func rasterizeToSurface(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double,
                                          appearance: DL3Appearance = .light, smoothFonts: Bool = false) -> IOSurface? {
        surface(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale, background: appearance.background,
                smoothFonts: smoothFonts) {
            draw(prepared, forms: forms, in: $0, appearance: appearance)
        }
    }

    /// A PDF page (the fallback for pages the display list cannot express).
    /// Dark: the light rendering with lightness inverted and hue kept
    /// (invert, then rotate hue by half a turn) — images included, as the
    /// PDF's pixels cannot be told apart from its ink.
    public static func rasterizeToSurface(pdfPage: CGPDFPage, scale: Double, appearance: DL3Appearance = .light,
                                          smoothFonts: Bool = false) -> IOSurface? {
        let box = pdfPage.getBoxRect(.mediaBox)
        let s = surface(widthPt: box.width, heightPt: box.height, scale: scale, smoothFonts: smoothFonts) { ctx in
            ctx.translateBy(x: -box.minX, y: -box.minY)
            DL3Renderer.drawPDFPage(pdfPage, in: ctx)
        }
        guard appearance == .dark, let s else { return s }
        let ci = CIImage(ioSurface: s)
            .applyingFilter("CIColorInvert")
            .applyingFilter("CIHueAdjust", parameters: [kCIInputAngleKey: Double.pi])
        let out = surface(widthPt: box.width, heightPt: box.height, scale: scale) { _ in }
        if let out {
            Self.ciContext.render(ci, to: out, bounds: CGRect(x: 0, y: 0, width: s.width, height: s.height),
                                  colorSpace: CGColorSpace(name: CGColorSpace.sRGB))
        }
        return out
    }

    static let ciContext = CIContext(options: [.workingColorSpace: CGColorSpace(name: CGColorSpace.sRGB)!])

    static func surface(widthPt: Double, heightPt: Double, scale: Double, background: CGColor = CGColor(gray: 1, alpha: 1),
                        smoothFonts: Bool = false, _ body: (CGContext) -> Void) -> IOSurface? {
        let w = Int((widthPt * scale).rounded(.up)), h = Int((heightPt * scale).rounded(.up))
        guard w > 0, h > 0,
              let s = IOSurface(properties: [.width: w, .height: h, .bytesPerElement: 4,
                                             .pixelFormat: 0x4247_5241 /* 'BGRA' */]) else { return nil }
        s.lock(options: [], seed: nil)
        defer { s.unlock(options: [], seed: nil) }
        let space = CGColorSpace(name: CGColorSpace.sRGB)!
        guard let ctx = CGContext(data: s.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: s.bytesPerRow,
                                  space: space, bitmapInfo: Layout.screen.bitmapInfo) else { return nil }
        ctx.setFillColor(background)
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        ctx.scaleBy(x: scale, y: scale)
        ctx.setShouldAntialias(true)
        setFontSmoothing(smoothFonts, in: ctx)
        ctx.setAllowsFontSubpixelPositioning(true)
        ctx.setShouldSubpixelPositionFonts(true)
        body(ctx)
        ctx.flush()
        if let plist = space.copyPropertyList() { IOSurfaceSetValue(s, kIOSurfaceColorSpace, plist) }
        return s
    }

    /// A surface's pixels as a CGImage (tests, evidence).
    public static func image(of s: IOSurface) -> CGImage? {
        s.lock(options: .readOnly, seed: nil)
        defer { s.unlock(options: .readOnly, seed: nil) }
        guard let ctx = CGContext(data: s.baseAddress, width: s.width, height: s.height, bitsPerComponent: 8, bytesPerRow: s.bytesPerRow,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: Layout.screen.bitmapInfo) else { return nil }
        return ctx.makeImage()
    }

    /// A page of a PDF rendered the same way (the fallback for INCOMPLETE
    /// pages, and the parity reference).
    public static func rasterize(pdfPage: CGPDFPage, scale: Double, layout: Layout = .rgba, smoothFonts: Bool = false) -> CGImage? {
        let box = pdfPage.getBoxRect(.mediaBox)
        guard let ctx = bitmapContext(widthPt: box.width, heightPt: box.height, scale: scale, layout: layout,
                                      smoothFonts: smoothFonts) else { return nil }
        ctx.translateBy(x: -box.minX, y: -box.minY)
        DL3Renderer.drawPDFPage(pdfPage, in: ctx)
        return ctx.makeImage()
    }
}

/// A PDF's bytes, kept with the `CGPDFDocument` opened from them
/// (`DL3Renderer.openPDF`), so a thread can open its own document of the
/// same PDF. `id` is unique per opened PDF (never reused, unlike an address).
public final class DL3PDFBytes: @unchecked Sendable {
    public let data: Data
    public let id: UInt64
    private static let counter = NSLock()
    nonisolated(unsafe) private static var next: UInt64 = 0

    init(_ data: Data) {
        self.data = data
        Self.counter.lock(); Self.next &+= 1; id = Self.next; Self.counter.unlock()
    }

    /// The PDF is no longer held (its document and pages are gone: a DONE
    /// replaced the fallback PDF, an image left the cache): every thread's
    /// own document of it goes too, rather than staying until 16 others
    /// push it out of an idle worker thread's cache.
    deinit { PDFThreadDocuments.retire(id) }

    nonisolated(unsafe) private static var key: UInt8 = 0
    static func attach(_ bytes: DL3PDFBytes, to doc: AnyObject) {
        withUnsafePointer(to: &key) { objc_setAssociatedObject(doc, $0, bytes, .OBJC_ASSOCIATION_RETAIN) }
    }
    static func of(_ doc: AnyObject) -> DL3PDFBytes? {
        withUnsafePointer(to: &key) { objc_getAssociatedObject(doc, $0) as? DL3PDFBytes }
    }
}

/// One thread's own documents of the PDFs it drew last, opened from the
/// shared bytes: at most `limit` documents and `byteLimit` bytes of PDF
/// (least recently used out; the newest always stays). Each thread has its
/// own (`Thread.threadDictionary`): a document is only ever drawn on its
/// thread. The lock is for `retire`, which drops a PDF no longer held from
/// every thread's cache (GCD keeps its worker threads, and their caches).
///
/// A tile draws every included PDF its rectangle meets, so the limit is
/// above the PDFs one page shows: with 4, a page of 5 or more PDFs (a
/// figure grid) opened each document again on every tile, 54-360 ms each.
final class PDFThreadDocuments {
    static let limit = 16
    static let byteLimit = 128 << 20
    let maxCount: Int, maxBytes: Int
    init(maxCount: Int = limit, maxBytes: Int = byteLimit) { self.maxCount = maxCount; self.maxBytes = maxBytes }
    private let lock = NSLock()
    private var entries: [(id: UInt64, doc: CGPDFDocument, bytes: Int)] = []
    /// Documents opened on this thread (tests).
    private(set) var opened = 0
    /// The documents kept and their PDFs' bytes (tests).
    var kept: (count: Int, bytes: Int) { lock.lock(); defer { lock.unlock() }; return (entries.count, entries.reduce(0) { $0 + $1.bytes }) }
    /// Whether a document of PDF `id` is kept (tests).
    func holds(_ id: UInt64) -> Bool { lock.lock(); defer { lock.unlock() }; return entries.contains { $0.id == id } }

    static var current: PDFThreadDocuments {
        let d = Thread.current.threadDictionary
        if let c = d["flashtex.dl3.pdf-documents"] as? PDFThreadDocuments { return c }
        let c = PDFThreadDocuments()
        d["flashtex.dl3.pdf-documents"] = c
        registryLock.lock()
        registry.removeAll { $0.cache == nil }
        registry.append(Weak(cache: c))
        registryLock.unlock()
        return c
    }

    private struct Weak { weak var cache: PDFThreadDocuments? }
    private static let registryLock = NSLock()
    nonisolated(unsafe) private static var registry: [Weak] = []

    /// Drops PDF `id` from every thread's cache (`DL3PDFBytes.deinit`).
    static func retire(_ id: UInt64) {
        registryLock.lock()
        let caches = registry.compactMap(\.cache)
        registryLock.unlock()
        for c in caches {
            c.lock.lock()
            let gone = c.entries.filter { $0.id == id } // (released after the lock)
            c.entries.removeAll { $0.id == id }
            c.lock.unlock()
            _ = gone
        }
    }

    func document(for bytes: DL3PDFBytes) -> CGPDFDocument? {
        lock.lock()
        if let i = entries.firstIndex(where: { $0.id == bytes.id }) {
            let e = entries.remove(at: i)
            entries.append(e)
            lock.unlock()
            return e.doc
        }
        lock.unlock()
        guard let provider = CGDataProvider(data: bytes.data as CFData), let doc = CGPDFDocument(provider) else { return nil }
        lock.lock()
        opened += 1
        entries.append((bytes.id, doc, bytes.data.count))
        var out: [CGPDFDocument] = []
        while entries.count > 1, entries.count > maxCount || entries.reduce(0, { $0 + $1.bytes }) > maxBytes { out.append(entries.removeFirst().doc) }
        lock.unlock()
        _ = out
        return doc
    }
}
