import CoreGraphics
import Foundation
import ImageIO

/// The Captures list's thumbnails: each capture's PNG is decoded once, to a
/// small image (`maxPixels` on the longest side), and kept by capture id.
/// The list used to build `UIImage(data: png)` in every row's body — a new
/// full-size image (up to 4096 px, 8 MiB) on every re-render, and the list
/// re-renders on every 2 s outcome poll — so each refresh re-decoded every
/// capture's PNG on the main thread to draw a 72 × 54 pt box. Thread-safe:
/// the app makes thumbnails off the main actor and reads the cache on it.
public final class CaptureThumbnails: @unchecked Sendable { // `cache` is thread-safe; `_decodes` is under `lock`
    /// Longest side in pixels (72 × 54 pt at 2×, with room for 3×).
    public static let maxPixels = 216

    private let cache = NSCache<NSString, CGImage>()
    private let lock = NSLock()
    private var _decodes = 0

    public init(countLimit: Int = 100) { cache.countLimit = countLimit }

    /// How many PNGs were decoded so far (tests).
    public var decodes: Int { lock.withLock { _decodes } }

    /// The thumbnail already made for `id`, without decoding anything.
    public func cached(_ id: String) -> CGImage? { cache.object(forKey: id as NSString) }

    /// The thumbnail for `id`, decoding `png` only the first time. Nil when
    /// `png` is not an image.
    public func thumbnail(id: String, png: Data) -> CGImage? {
        if let hit = cached(id) { return hit }
        lock.withLock { _decodes += 1 }
        guard let source = CGImageSourceCreateWithData(png as CFData, nil) else { return nil }
        let options: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceThumbnailMaxPixelSize: Self.maxPixels,
            kCGImageSourceShouldCacheImmediately: true,
        ]
        guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else { return nil }
        cache.setObject(image, forKey: id as NSString)
        return image
    }
}
