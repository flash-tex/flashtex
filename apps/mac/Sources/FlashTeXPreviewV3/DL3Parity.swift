import CoreGraphics
import CryptoKit
import Foundation
import FlashTeXDisplayListV3

/// A whole file of frames (spec §6.6) replayed as a client does: resources
/// bound as they arrive, each page prepared when it arrives, forms kept by id.
public struct DL3Document {
    public var pages: [UInt32: DL3PreparedPage] = [:]
    public var forms: [UInt32: DL3PreparedPage] = [:]
    public var fontKeys: Set<String> = []
    public var sources = DL3SourceMap()

    public init(frames bytes: [UInt8], cache: DL3ResourceCache = .shared) throws {
        var bindings = DL3Bindings()
        for (k, body) in try DL3Frames.split(bytes) {
            switch try DL3Event.decode(kind: k, body: body) {
            case .font(let f): bindings.bind(font: f, cache: cache); fontKeys.insert(f.keyHex)
            case .image(let j): bindings.bind(image: j, cache: cache)
            case .page(let p): pages[p.index] = bindings.prepare(p)
            case .form(let p): forms[p.index] = bindings.prepare(p)
            case .sources(let src): sources.apply(src)
            default: break
            }
        }
    }

    public var orderedPages: [DL3PreparedPage] { pages.keys.sorted().compactMap { pages[$0] } }
}

/// Preview-versus-PDF comparison at zero tolerance (DESIGN.md §6.2 gate):
/// each page is rasterised through `DL3Renderer` and, separately, the same
/// page of the engine's PDF is rasterised by Core Graphics into an
/// identically configured bitmap; any pixel whose RGBA differs counts.
public enum DL3Parity {
    public struct PageResult: Codable, Equatable {
        public var page: Int
        public var scale: Double
        public var widthPx: Int, heightPx: Int
        public var differingPixels: Int
        /// Largest per-channel difference over the page (0–255).
        public var maxChannelDelta: Int
        public var incomplete: Bool
        public var problems: [String]
        /// The page draws a Type 3 (bitmap) font.
        public var type3: Bool = false
    }

    public static func rgba(_ image: CGImage) -> [UInt8] {
        // Our bitmaps are 8-bit premultiplied-last sRGB already: take the bytes.
        if image.bitsPerPixel == 32, image.bytesPerRow == image.width * 4,
           image.alphaInfo == .premultipliedLast, let data = image.dataProvider?.data {
            return Array(UnsafeBufferPointer(start: CFDataGetBytePtr(data), count: CFDataGetLength(data)))
        }
        guard let ctx = CGContext(data: nil, width: image.width, height: image.height, bitsPerComponent: 8, bytesPerRow: image.width * 4,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue),
              let base = ctx.data else { return [] }
        ctx.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
        return Array(UnsafeBufferPointer(start: base.assumingMemoryBound(to: UInt8.self), count: image.width * image.height * 4))
    }

    public static func diff(_ a: [UInt8], _ b: [UInt8]) -> (pixels: Int, maxDelta: Int) {
        guard a.count == b.count else { return (max(a.count, b.count) / 4, 255) }
        if a.withUnsafeBytes({ pa in b.withUnsafeBytes { pb in memcmp(pa.baseAddress!, pb.baseAddress!, a.count) == 0 } }) { return (0, 0) }
        var n = 0, m = 0
        a.withUnsafeBufferPointer { pa in b.withUnsafeBufferPointer { pb in
            var i = 0
            while i < pa.count {
                if pa[i] != pb[i] || pa[i + 1] != pb[i + 1] || pa[i + 2] != pb[i + 2] || pa[i + 3] != pb[i + 3] {
                    n += 1
                    for c in 0 ..< 4 { m = max(m, abs(Int(pa[i + c]) - Int(pb[i + c]))) }
                }
                i += 4
            }
        } }
        return (n, m)
    }

    /// `reference`: the page (0-based) of the PDF rendered at a scale into the
    /// same bitmap configuration; default Core Graphics' `drawPDFPage`.
    public static func compare(document: DL3Document, pdf: CGPDFDocument, scales: [Double],
                               reference: ((Int, Double) -> CGImage?)? = nil,
                               images: ((Int, Double, CGImage, CGImage) -> Void)? = nil) -> [PageResult] {
        var out: [PageResult] = []
        for prepared in document.orderedPages {
            let index = Int(prepared.page.index)
            guard let pdfPage = pdf.page(at: index + 1) else { continue }
            for scale in scales {
                guard let a = DL3Renderer.rasterize(prepared, forms: document.forms, scale: scale),
                      let b = reference?(index, scale) ?? DL3Renderer.rasterize(pdfPage: pdfPage, scale: scale) else { continue }
                let d = diff(rgba(a), rgba(b))
                out.append(PageResult(page: index, scale: scale, widthPx: a.width, heightPx: a.height,
                                      differingPixels: d.pixels, maxChannelDelta: d.maxDelta,
                                      incomplete: prepared.needsPDFFallback(forms: document.forms), problems: prepared.problems,
                                      type3: prepared.fonts.values.contains { $0.type3 != nil }))
                images?(index, scale, a, b)
            }
        }
        return out
    }
}
