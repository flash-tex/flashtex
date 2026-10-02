import FlashTeXEditorCore
import Foundation
import ImageIO

/// Paste an image into the iPad editor (IMAGE-DROP-IPAD): the file half
/// (Foundation and ImageIO only). What text goes where is the shared `PasteImageFigure`
/// (the Mac's logic, FlashTeXEditorCore); the editor (`EditorController`)
/// reads the pasteboard and applies the plan as one undo step.
///
/// The iPad has no project model beyond the opened `.tex` file: its folder
/// is the project folder (`PadModel.projectFolder`). The bundled demo and
/// fixtures have none, and nothing is written for them.
public enum PadImagePaste {
    public static let noProjectNote = "Open a .tex file from Files to paste images: they are saved into its folder's figures/."
    public static let preambleNote = "Images cannot go in the preamble: move the caret below \\begin{document} and paste again."
    /// Largest image the paste writes (bytes), as on the Mac.
    public static let maximumBytes = 50 * 1024 * 1024

    public enum SaveError: Error, Equatable, LocalizedError {
        case tooLarge
        case outsideProject(String)
        /// The app may not write in the project folder (on a real iPad: the
        /// file was opened on its own, so its folder's security scope was
        /// never granted). Carries the project folder's name.
        case noAccess(String)
        case unreadableImage
        case write(String)

        public var errorDescription: String? {
            switch self {
            case .tooLarge: "the image is larger than 50 MB"
            case .outsideProject(let folder): "the folder \(folder) resolves outside the project"
            case .noAccess(let folder): "FlashTeXPad has no permission to write in “\(folder)”"
            case .unreadableImage: "the image could not be converted to PNG"
            case .write(let why): why
            }
        }
    }

    /// Shown while a HEIC, TIFF, GIF, … paste is converted to PNG off the main thread.
    public static let convertingNote = "Converting the pasted image to PNG…"

    /// Why bytes of this size are refused before any work (conversion
    /// included), as the Mac's `PasteImage.refusal`; nil when they fit.
    public static func refusal(byteCount: Int) -> String? {
        byteCount > maximumBytes ? "The pasted image is larger than \(maximumBytes / 1_048_576) MB; it was not saved." : nil
    }

    /// Saves `data` (already PNG, JPEG or PDF bytes) as
    /// `projectFolder/folder/pasted-YYYYMMDD-HHMMSS.ext` — never
    /// overwriting (`-2`, `-3`, …; a name taken meanwhile is retried) — and
    /// returns the `\includegraphics` path. `folder` comes from
    /// `PasteImageFigure.imageFolder` (the document's `\graphicspath` wins).
    ///
    /// The folder is checked to lie inside the project before anything is
    /// created and again after (a symlink made meanwhile), as the Mac's
    /// `PasteImage.save` does. The folder and the file are written under
    /// `NSFileCoordinator`, so a file-provider folder (iCloud Drive, a
    /// third-party provider) sees a coordinated write. A permission failure
    /// is `.noAccess`, which the editor turns into an "Allow access" offer.
    public static func save(_ data: Data, fileExtension ext: String, projectFolder: URL, folder: String,
                            date: Date = Date(), fileManager: FileManager = .default) throws -> String {
        guard data.count <= maximumBytes else { throw SaveError.tooLarge }
        let root = projectFolder.standardizedFileURL
        let dir = folder.isEmpty ? root : root.appendingPathComponent(folder, isDirectory: true)
        guard resolvesInside(dir, root: root, fileManager: fileManager) else { throw SaveError.outsideProject(folder) }
        if !fileManager.fileExists(atPath: dir.path) {
            do {
                try coordinatedWrite(at: dir) { try fileManager.createDirectory(at: $0, withIntermediateDirectories: true) }
            } catch {
                throw failure(error, root: root)
            }
        }
        guard resolvesInside(dir, root: root, fileManager: fileManager) else { throw SaveError.outsideProject(folder) }
        var attempt = 0
        while true {
            let name = PasteImageFigure.uniqueFileName(base: PasteImageFigure.baseName(for: date), fileExtension: ext) {
                fileManager.fileExists(atPath: dir.appendingPathComponent($0).path)
            }
            do {
                try coordinatedWrite(at: dir.appendingPathComponent(name)) { try data.write(to: $0, options: [.withoutOverwriting]) }
                return PasteImageFigure.graphicsPath(folder: folder, fileName: name)
            } catch {
                attempt += 1
                if attempt < 5, fileManager.fileExists(atPath: dir.appendingPathComponent(name).path) { continue }
                throw failure(error, root: root)
            }
        }
    }

    /// Whether `dir` — through its nearest existing ancestor, symlinks
    /// resolved — lies inside `root` (the Mac's `PasteImage.resolvesInside`).
    public static func resolvesInside(_ dir: URL, root: URL, fileManager: FileManager = .default) -> Bool {
        var probe = dir.standardizedFileURL
        while !fileManager.fileExists(atPath: probe.path), probe.pathComponents.count > 1 { probe = probe.deletingLastPathComponent() }
        let resolved = probe.resolvingSymlinksInPath().path
        let base = root.standardizedFileURL.resolvingSymlinksInPath().path
        return resolved == base || resolved.hasPrefix(base + "/")
    }

    /// Whether `error` (or the error under it) is a sandbox or file-mode
    /// permission refusal, which folder access can fix.
    public static func isPermissionDenied(_ error: Error) -> Bool {
        let ns = error as NSError
        if ns.domain == NSCocoaErrorDomain, ns.code == NSFileWriteNoPermissionError || ns.code == NSFileReadNoPermissionError { return true }
        if ns.domain == NSPOSIXErrorDomain, ns.code == Int(EPERM) || ns.code == Int(EACCES) { return true }
        if let underlying = ns.userInfo[NSUnderlyingErrorKey] as? Error { return isPermissionDenied(underlying) }
        return false
    }

    private static func failure(_ error: Error, root: URL) -> SaveError {
        if let e = error as? SaveError { return e }
        return isPermissionDenied(error) ? .noAccess(root.lastPathComponent) : .write(error.localizedDescription)
    }

    /// Runs `body` under a coordinated write of `url` (the URL the
    /// coordinator hands back is the one written).
    static func coordinatedWrite(at url: URL, _ body: (URL) throws -> Void) throws {
        var coordinationError: NSError?
        var bodyError: Error?
        NSFileCoordinator(filePresenter: nil).coordinate(writingItemAt: url, options: [], error: &coordinationError) { target in
            do { try body(target) } catch { bodyError = error }
        }
        if let error = bodyError ?? coordinationError { throw error }
    }

    // MARK: converting (any thread)

    /// PNG bytes of any image ImageIO reads (HEIC, TIFF, GIF's first frame,
    /// BMP, WebP), with its EXIF orientation applied — `\includegraphics`
    /// ignores orientation metadata, so a portrait photo stays upright.
    /// Nil when the bytes are not an image.
    public static func pngData(from data: Data) -> Data? {
        guard let source = CGImageSourceCreateWithData(data as CFData, nil), CGImageSourceGetCount(source) > 0,
              let props = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let width = props[kCGImagePropertyPixelWidth] as? Int, let height = props[kCGImagePropertyPixelHeight] as? Int,
              width > 0, height > 0 else { return nil }
        let options: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceThumbnailMaxPixelSize: max(width, height),
            kCGImageSourceShouldCacheImmediately: true,
        ]
        guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else { return nil }
        let out = NSMutableData()
        guard let dest = CGImageDestinationCreateWithData(out as CFMutableData, "public.png" as CFString, 1, nil) else { return nil }
        CGImageDestinationAddImage(dest, image, nil)
        guard CGImageDestinationFinalize(dest) else { return nil }
        return out as Data
    }
}
