import Foundation
import ImageIO
import NearbyClient
import UniformTypeIdentifiers

/// A picked photo or camera shot made into the PNG a capture carries.
///
/// The Mac accepts at most `NearbyWire.maxImageBytes` (8 MiB) per image,
/// and PNG ignores EXIF orientation. A camera photo (12 MP on an iPad) is
/// 15–40 MB as a full-resolution PNG, so encoding the picked image as it is
/// failed `CaptureQueue.validate` for every camera shot and most library
/// photos, and a portrait shot would have arrived on its side. Here the image
/// is decoded with its orientation applied, its longest side bounded by
/// `maxPixels` (the canvas render's bound), and stepped down until the PNG
/// fits. Pure ImageIO: runs on any thread (the app calls it off the main
/// actor) and is tested on macOS.
public enum CaptureImage {
    /// Longest side of a capture, in pixels (the canvas render's
    /// `CanvasLayout.maxCapturePixels`).
    public static let maxPixels = 4096
    /// Below this longest side a photo is no longer worth converting.
    public static let minPixels = 512

    public struct Prepared: Equatable, Sendable {
        public let png: Data
        public let width: Int
        public let height: Int
    }

    public enum Failure: Error, Equatable, Sendable, CustomStringConvertible {
        case unreadable
        case tooManyPixels
        case tooLarge
        public var description: String {
            switch self {
            case .unreadable: return "the picked image could not be read"
            case .tooManyPixels: return "the picked image has more than \(PadImagePaste.maximumPixels / 1_000_000) megapixels"
            case .tooLarge: return "the picked image does not fit the Mac's \(NearbyWire.maxImageBytes / 1_048_576) MiB limit even when scaled down"
            }
        }
    }

    /// PNG for `data` (any image ImageIO reads: JPEG, HEIC, PNG, …), upright,
    /// longest side ≤ `maxPixels`, at most `maxBytes`. A PNG that already
    /// satisfies all three is returned byte for byte.
    public static func sendablePNG(from data: Data, maxPixels: Int = maxPixels,
                                   maxBytes: Int = NearbyWire.maxImageBytes) -> Result<Prepared, Failure> {
        guard let source = CGImageSourceCreateWithData(data as CFData, nil), CGImageSourceGetCount(source) > 0,
              let props = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let width = props[kCGImagePropertyPixelWidth] as? Int, let height = props[kCGImagePropertyPixelHeight] as? Int,
              width > 0, height > 0 else { return .failure(.unreadable) }
        guard !PadImagePaste.tooManyPixels(width: width, height: height) else { return .failure(.tooManyPixels) }
        let orientation = (props[kCGImagePropertyOrientation] as? NSNumber)?.intValue ?? 1
        let isPNG = (CGImageSourceGetType(source) as String?) == UTType.png.identifier
        if isPNG, orientation == 1, max(width, height) <= maxPixels, data.count <= maxBytes {
            return .success(Prepared(png: data, width: width, height: height))
        }
        var side = min(maxPixels, max(width, height))
        while side >= minPixels {
            let options: [CFString: Any] = [
                kCGImageSourceCreateThumbnailFromImageAlways: true,
                kCGImageSourceCreateThumbnailWithTransform: true,
                kCGImageSourceThumbnailMaxPixelSize: side,
                kCGImageSourceShouldCacheImmediately: true,
            ]
            guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary),
                  let png = encodePNG(image) else { return .failure(.unreadable) }
            if png.count <= maxBytes { return .success(Prepared(png: png, width: image.width, height: image.height)) }
            // PNG size grows with the pixel count: aim just under the limit,
            // and always shrink by at least a tenth so the loop ends.
            let ratio = (Double(maxBytes) / Double(png.count)).squareRoot() * 0.95
            side = Int(Double(side) * min(0.9, max(0.5, ratio)))
        }
        return .failure(.tooLarge)
    }

    static func encodePNG(_ image: CGImage) -> Data? {
        let out = NSMutableData()
        guard let dest = CGImageDestinationCreateWithData(out as CFMutableData, UTType.png.identifier as CFString, 1, nil) else { return nil }
        CGImageDestinationAddImage(dest, image, nil)
        guard CGImageDestinationFinalize(dest) else { return nil }
        return out as Data
    }
}
