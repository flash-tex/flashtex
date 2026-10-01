import CoreGraphics
import CoreText
import Foundation
import CoreImage
import ImageIO
import IOSurface
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

/// A Type 1 (or other CG-loadable) font program, loaded once per key.
public final class DL3RenderFont: @unchecked Sendable {
    public let key: String
    public let cgFont: CGFont
    public let ctFont: CTFont
    /// code → glyph (0: none / .notdef).
    public let glyphs: [CGGlyph]
    /// Glyph space → text space beyond the program's own FontMatrix
    /// (pdfTeX's `font_matrix` for SlantFont/ExtendFont), else identity.
    public let fontTransform: CGAffineTransform

    init(key: String, cgFont: CGFont, glyphs: [CGGlyph], fontTransform: CGAffineTransform) {
        self.key = key; self.cgFont = cgFont; self.glyphs = glyphs; self.fontTransform = fontTransform
        ctFont = CTFontCreateWithGraphicsFont(cgFont, 1, nil, nil)
    }

    /// Loads `font`'s program. Returns nil (with the reason) when it cannot
    /// be drawn: not embedded (`format: none`), or not loadable.
    static func load(_ font: DL3Font) -> Result<DL3RenderFont, DL3Error> {
        guard font.format == "type1" || font.format == "opentype" || font.format == "truetype" else {
            return .failure(DL3Error("font \(font.pdfName ?? "\(font.id)"): format \(font.format ?? "?") has no program to draw"))
        }
        guard !font.program.isEmpty,
              let provider = CGDataProvider(data: Data(font.program) as CFData),
              let cg = CGFont(provider) else {
            return .failure(DL3Error("font \(font.pdfName ?? "\(font.id)") (\(font.psName ?? "?")): the program does not load"))
        }
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
        var glyphs = [CGGlyph](repeating: 0, count: 256)
        var byName: [String: CGGlyph] = [:]
        for (code, name) in names.enumerated() where code < 256 {
            guard let name, !name.isEmpty, name != ".notdef" else { continue }
            if let g = byName[name] { glyphs[code] = g; continue }
            let g = cg.getGlyphWithGlyphName(name: name as CFString)
            byName[name] = g
            glyphs[code] = g
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
    public enum Payload { case raster(CGImage), pdf(CGPDFPage, box: CGRect) }
    public let key: String
    public let payload: Payload
    init(key: String, payload: Payload) { self.key = key; self.payload = payload }

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
            return .success(DL3RenderImage(key: key, payload: .raster(deviceSamples(img))))
        case "pdf":
            guard let doc = CGPDFDocument(url as CFURL), let page = doc.page(at: Int(info["page"]?.int ?? 1)) else {
                return .failure(DL3Error("\(file): cannot open page \(info["page"]?.int ?? 1)"))
            }
            let w = info["width"]?.double ?? 0, h = info["height"]?.double ?? 0
            let box = CGRect(x: info["orig_x"]?.double ?? 0, y: info["orig_y"]?.double ?? 0, width: w, height: h)
            return .success(DL3RenderImage(key: key, payload: .pdf(page, box: box)))
        default:
            return .failure(DL3Error("\(file): image type \(type) is not drawn"))
        }
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
    public var page: DL3Page
    public var fonts: [UInt16: DL3RenderFont]
    public var images: [UInt32: DL3RenderImage]
    /// Why some item cannot be drawn exactly (a font that did not load,
    /// an image that is missing); empty when every item resolved.
    public var problems: [String]

    public init(page: DL3Page, fonts: [UInt16: DL3RenderFont], images: [UInt32: DL3RenderImage], problems: [String] = []) {
        self.page = page; self.fonts = fonts; self.images = images; self.problems = problems
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
        for it in page.items {
            if case .form(let id, _) = it, let f = forms[id], f.needsPDFFallback(forms: forms, depth: depth + 1) { return true }
        }
        return false
    }
}

/// Resolves ids to loaded resources, caching fonts and images by key across
/// compiles and connections. Thread-safe.
public final class DL3ResourceCache: @unchecked Sendable {
    public static let shared = DL3ResourceCache()
    private let lock = NSLock()
    private var fonts: [String: Result<DL3RenderFont, DL3Error>] = [:]
    private var images: [String: Result<DL3RenderImage, DL3Error>] = [:]
    /// Programs by key, so `have_fonts` can name them (the host then sends an empty program).
    private var programs: Set<String> = []

    public init() {}

    public var heldFontKeys: [String] { lock.lock(); defer { lock.unlock() }; return programs.sorted() }

    public func font(_ f: DL3Font) -> Result<DL3RenderFont, DL3Error> {
        let key = f.keyHex
        lock.lock()
        if let hit = fonts[key] { lock.unlock(); return hit }
        lock.unlock()
        if f.program.isEmpty && f.format != "none" {
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
        if let hit = images[key] { lock.unlock(); return hit }
        lock.unlock()
        let r = DL3RenderImage.load(info)
        lock.lock(); images[key] = r; lock.unlock()
        return r
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
        ctx.saveGState()
        ctx.translateBy(x: -prepared.page.box[0], y: -prepared.page.box[1])
        drawStream(prepared, forms: forms, in: ctx, depth: 0, appearance: appearance)
        ctx.restoreGState()
    }

    static func drawStream(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage], in ctx: CGContext, depth: Int,
                           appearance: DL3Appearance = .light) {
        let page = prepared.page
        let H = page.box[3]
        let black = color([0], appearance)
        var gs = GState(fill: black, stroke: black, textRender: 0, textFill: appearance == .dark ? color([0], appearance, text: true) : nil)
        var inText = false // dark: the text fill is set on the context
        var stack: [GState] = []
        var glyphMatrix = CGAffineTransform.identity
        var lastFont: DL3RenderFont?
        ctx.setFillColor(black)
        ctx.setStrokeColor(black)
        ctx.setTextDrawingMode(.fill)
        for item in page.items {
            switch item {
            case .glyph(let f, let code, let x, let y, _):
                guard gs.textRender != 3, let font = prepared.fonts[f], Int(code) < 256 else { continue }
                let g = font.glyphs[Int(code)]
                guard g != 0 else { continue }
                if let t = gs.textFill, !inText { ctx.setFillColor(t); inText = true }
                if lastFont !== font { ctx.setFont(font.cgFont); ctx.setFontSize(1); lastFont = font }
                var tm = font.fontTransform.concatenating(glyphMatrix)
                tm.tx = Double(x) / K
                tm.ty = snap(H - Double(y) / K)
                ctx.textMatrix = tm
                ctx.showGlyphs([g], at: [.zero])
            case .rule(let kind, let x, let y, let w, let h):
                if inText { ctx.setFillColor(gs.fill); inText = false }
                let left = snap(Double(x) / K), top = snap(H - Double(y) / K)
                let right = snap(Double(x + w) / K), bottom = snap(H - Double(y + h) / K)
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
                switch img.payload {
                case .raster(let cgImage):
                    ctx.interpolationQuality = .default // what Core Graphics uses for a PDF image without /Interpolate (measured: .none and .medium/.high differ)
                    ctx.draw(cgImage, in: CGRect(x: 0, y: 0, width: 1, height: 1))
                case .pdf(let pdfPage, let box):
                    ctx.clip(to: CGRect(x: 0, y: 0, width: 1, height: 1))
                    if box.width > 0, box.height > 0 {
                        ctx.scaleBy(x: 1 / box.width, y: 1 / box.height)
                        ctx.translateBy(x: -box.minX, y: -box.minY)
                    }
                    ctx.drawPDFPage(pdfPage)
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

    public static func bitmapContext(widthPt: Double, heightPt: Double, scale: Double, layout: Layout = .rgba) -> CGContext? {
        let w = Int((widthPt * scale).rounded(.up)), h = Int((heightPt * scale).rounded(.up))
        guard w > 0, h > 0,
              let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: layout.bitmapInfo) else { return nil }
        ctx.setFillColor(CGColor(gray: 1, alpha: 1))
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        ctx.scaleBy(x: scale, y: scale)
        ctx.setShouldAntialias(true)
        ctx.setShouldSmoothFonts(false)
        ctx.setAllowsFontSubpixelPositioning(true)
        ctx.setShouldSubpixelPositionFonts(true)
        return ctx
    }

    /// One page's bitmap (off-main safe).
    public static func rasterize(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double, layout: Layout = .rgba,
                                 appearance: DL3Appearance = .light) -> CGImage? {
        guard let ctx = bitmapContext(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale, layout: layout) else { return nil }
        if appearance == .dark {
            ctx.saveGState(); ctx.setFillColor(appearance.background)
            ctx.fill(CGRect(x: 0, y: 0, width: prepared.widthPt, height: prepared.heightPt)); ctx.restoreGState()
        }
        draw(prepared, forms: forms, in: ctx, appearance: appearance)
        return ctx.makeImage()
    }

    /// The page drawn straight into an IOSurface (BGRA, `.screen` layout,
    /// tagged sRGB): a layer shows it without the copy Core Animation makes
    /// of a CGImage at commit (measured 3.5 ms for a 1.4-megapixel page).
    /// Same context configuration, so the same pixels as `rasterize`.
    public static func rasterizeToSurface(_ prepared: DL3PreparedPage, forms: [UInt32: DL3PreparedPage] = [:], scale: Double,
                                          appearance: DL3Appearance = .light) -> IOSurface? {
        surface(widthPt: prepared.widthPt, heightPt: prepared.heightPt, scale: scale, background: appearance.background) {
            draw(prepared, forms: forms, in: $0, appearance: appearance)
        }
    }

    /// A PDF page (the fallback for pages the display list cannot express).
    /// Dark: the light rendering with lightness inverted and hue kept
    /// (invert, then rotate hue by half a turn) — images included, as the
    /// PDF's pixels cannot be told apart from its ink.
    public static func rasterizeToSurface(pdfPage: CGPDFPage, scale: Double, appearance: DL3Appearance = .light) -> IOSurface? {
        let box = pdfPage.getBoxRect(.mediaBox)
        let s = surface(widthPt: box.width, heightPt: box.height, scale: scale) { ctx in
            ctx.translateBy(x: -box.minX, y: -box.minY)
            ctx.drawPDFPage(pdfPage)
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
                        _ body: (CGContext) -> Void) -> IOSurface? {
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
        ctx.setShouldSmoothFonts(false)
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
    public static func rasterize(pdfPage: CGPDFPage, scale: Double, layout: Layout = .rgba) -> CGImage? {
        let box = pdfPage.getBoxRect(.mediaBox)
        guard let ctx = bitmapContext(widthPt: box.width, heightPt: box.height, scale: scale, layout: layout) else { return nil }
        ctx.translateBy(x: -box.minX, y: -box.minY)
        ctx.drawPDFPage(pdfPage)
        return ctx.makeImage()
    }
}
