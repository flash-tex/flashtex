import CoreGraphics
import CryptoKit
import Foundation
import ImageIO
import FlashTeXProtocol

/// Content-addressed image resolution for `display-list-v2-images`
/// (protocol/proposals/display-list-v2-image.md §5). An image item names a
/// project-relative `path` plus the SHA-256 and byte length of the bytes the
/// producer sized; the bytes themselves are never on the wire. This store
/// reads them under the open project's directory with the SAME rooted rule
/// the project-files helper applies (`ProjectDocuments.rootedFile`: every
/// component stays under the root, no symlink on the way — the helper's
/// `read` is UTF-8 text only, so binary assets take this rooted local read),
/// verifies length and SHA-256 BEFORE decoding, and caches decoded images by
/// `image_id`. Anything else is a typed refusal: the item paints nothing,
/// the frame is kept, and the pane shows one non-modal notice per path.
final class V2ImageStore {
    /// A refusal names the item's path; `notice` is the one-line banner text.
    struct Refusal: Error, Equatable, CustomStringConvertible {
        var code: String
        var path: String
        var message: String
        var description: String { "\(code): \(message)" }
        /// "stale image: <path>" for bytes that no longer match; otherwise
        /// "image unavailable: <path>: <why>".
        var notice: String { code == "image_stale" ? "stale image: \(path)" : "image unavailable: \(path): \(message)" }
    }

    /// What the painter draws: a decoded raster, or one page of a PDF (the
    /// document is retained alongside the page).
    enum Payload {
        case raster(CGImage)
        case pdf(CGPDFDocument, CGPDFPage)
    }

    struct Resolved {
        var resource: RenderingV2.ImageResource
        var payload: Payload
    }

    static let shared = V2ImageStore()

    private let lock = NSLock()
    private var rootURL: URL?
    private var cache: [String: Resolved] = [:]
    private(set) var reads = 0

    init(root: URL? = nil) { rootURL = root }

    /// The rooted directory image paths resolve under; nil refuses every
    /// image (no project open). Changing it drops nothing: entries are keyed
    /// by content hash, which no directory can change.
    var root: URL? {
        get { lock.lock(); defer { lock.unlock() }; return rootURL }
        set { lock.lock(); rootURL = newValue; lock.unlock() }
    }

    /// Identity of the current root for page-cache keys (`V2PageCache.Key`).
    var rootKey: String { root?.standardizedFileURL.path ?? "" }

    var cachedCount: Int { lock.lock(); defer { lock.unlock() }; return cache.count }

    func clear() { lock.lock(); cache = [:]; lock.unlock() }

    /// Resolves one item's resource to decoded bytes or throws a `Refusal`.
    func resolve(_ r: RenderingV2.ImageResource) throws -> Resolved {
        lock.lock(); defer { lock.unlock() }
        let key = "\(r.imageId)#\(r.format)#\(r.pdfPage ?? 0)"
        if let cached = cache[key] { return cached }
        guard let root = rootURL else { throw Refusal(code: "image_unavailable", path: r.path, message: "no project root") }
        let url: URL
        switch ProjectDocuments.rootedFile(r.path, under: root) {
        case .file(let u): url = u
        case .refused(let why): throw Refusal(code: "image_refused", path: r.path, message: why)
        }
        let data: Data
        do { data = try Data(contentsOf: url, options: .mappedIfSafe) } catch {
            throw Refusal(code: "image_unavailable", path: r.path, message: "not readable (\(error.localizedDescription))")
        }
        reads += 1
        guard Int64(data.count) == r.byteLength else {
            throw Refusal(code: "image_stale", path: r.path, message: "\(data.count) bytes on disk, the display list sized \(r.byteLength)")
        }
        guard V2FontStore.hex(SHA256.hash(data: data)) == r.sha256 else {
            throw Refusal(code: "image_stale", path: r.path, message: "bytes on disk do not hash to \(r.sha256.prefix(12))…")
        }
        let payload: Payload
        switch r.format {
        case "png", "jpeg":
            guard let source = CGImageSourceCreateWithData(data as CFData, nil),
                  let image = CGImageSourceCreateImageAtIndex(source, 0, [kCGImageSourceShouldCache: true] as CFDictionary) else {
                throw Refusal(code: "image_unavailable", path: r.path, message: "\(r.format) could not be decoded")
            }
            guard image.width == r.pixelWidth, image.height == r.pixelHeight else {
                throw Refusal(code: "image_stale", path: r.path, message: "decoded \(image.width)×\(image.height) px, the display list sized \(r.pixelWidth ?? 0)×\(r.pixelHeight ?? 0)")
            }
            payload = .raster(image)
        case "pdf":
            guard let provider = CGDataProvider(data: data as CFData), let document = CGPDFDocument(provider) else {
                throw Refusal(code: "image_unavailable", path: r.path, message: "pdf could not be opened")
            }
            guard let page = document.page(at: r.pdfPage ?? 1) else {
                throw Refusal(code: "image_stale", path: r.path, message: "page \(r.pdfPage ?? 1) of \(document.numberOfPages) not present")
            }
            payload = .pdf(document, page)
        default:
            throw Refusal(code: "image_unavailable", path: r.path, message: "format '\(r.format)' is not paintable")
        }
        let resolved = Resolved(resource: r, payload: payload)
        cache[key] = resolved
        return resolved
    }
}

/// One image item converted for CoreGraphics: the clip rectangle and the
/// unit-square → PDF-space (y up, points) matrix, plus, for PDF pages, the
/// page-space → unit-square matrix built from the producer's `pdf_box` and
/// `pdf_rotate` (proposal §3: the unit square IS that box after rotation).
struct V2PreparedImage {
    var clip: CGRect
    var unitToPage: CGAffineTransform
    var pageToUnit: CGAffineTransform?
    var payload: V2ImageStore.Payload

    init(item: RenderingV2.Image, resolved: V2ImageStore.Resolved, pageHeight: Double) {
        let q = V2PreparedPage.serialized
        let t = item.transform
        // Proposal §5.4: y-down page points → PDF y-up space is [a, -b, c, -d, e, H − f].
        unitToPage = CGAffineTransform(a: q(t[0]), b: q(-t[1]), c: q(t[2]), d: q(-t[3]), tx: q(t[4]), ty: q(pageHeight - t[5]))
        let rect = GlyphRunRenderer.pdfRect(x: item.x, top: item.top, width: item.width, height: item.height, pageHeight: pageHeight)
        clip = CGRect(x: q(rect.origin.x), y: q(rect.origin.y), width: q(rect.width), height: q(rect.height))
        payload = resolved.payload
        if case .pdf = resolved.payload, let box = resolved.resource.pdfBox, box.count == 4 {
            pageToUnit = V2PreparedImage.pageToUnit(box: box, rotate: resolved.resource.pdfRotate ?? 0)
        } else {
            pageToUnit = nil
        }
    }

    /// Maps PDF page user space to the unit square so that `pdf_box` rotated
    /// clockwise by `rotate` (as a viewer applies `/Rotate`) fills it with
    /// u to the right and v up.
    static func pageToUnit(box: [Double], rotate: Int) -> CGAffineTransform {
        let llx = box[0], lly = box[1], bw = box[2] - box[0], bh = box[3] - box[1]
        switch rotate {
        case 90: return CGAffineTransform(a: 0, b: -1 / bw, c: 1 / bh, d: 0, tx: -lly / bh, ty: 1 + llx / bw)
        case 180: return CGAffineTransform(a: -1 / bw, b: 0, c: 0, d: -1 / bh, tx: 1 + llx / bw, ty: 1 + lly / bh)
        case 270: return CGAffineTransform(a: 0, b: 1 / bw, c: -1 / bh, d: 0, tx: 1 + lly / bh, ty: -llx / bw)
        default: return CGAffineTransform(a: 1 / bw, b: 0, c: 0, d: 1 / bh, tx: -llx / bw, ty: -lly / bh)
        }
    }

    /// Paints into `ctx` (PDF space): clipped to the item's box, the raster
    /// fills the unit square, a PDF page is drawn through both matrices.
    func draw(in ctx: CGContext) {
        ctx.saveGState()
        ctx.clip(to: clip)
        ctx.concatenate(unitToPage)
        switch payload {
        case .raster(let image):
            ctx.interpolationQuality = .high
            ctx.draw(image, in: CGRect(x: 0, y: 0, width: 1, height: 1))
        case .pdf(_, let page):
            ctx.clip(to: CGRect(x: 0, y: 0, width: 1, height: 1))
            if let pageToUnit { ctx.concatenate(pageToUnit) }
            ctx.drawPDFPage(page)
        }
        ctx.restoreGState()
    }
}
