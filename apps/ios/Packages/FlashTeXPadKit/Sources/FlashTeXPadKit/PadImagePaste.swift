import FlashTeXEditorCore
import Foundation

/// Paste an image into the iPad editor (IMAGE-DROP-IPAD): the file half,
/// Foundation only. What text goes where is the shared `PasteImageFigure`
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
        case write(String)

        public var errorDescription: String? {
            switch self {
            case .tooLarge: "the image is larger than 50 MB"
            case .outsideProject(let folder): "the folder \(folder) resolves outside the project"
            case .write(let why): why
            }
        }
    }

    /// Saves `data` (already PNG, JPEG or PDF bytes) as
    /// `projectFolder/folder/pasted-YYYYMMDD-HHMMSS.ext` — never
    /// overwriting (`-2`, `-3`, …; a name taken meanwhile is retried) — and
    /// returns the `\includegraphics` path. `folder` comes from
    /// `PasteImageFigure.imageFolder` (the document's `\graphicspath` wins).
    public static func save(_ data: Data, fileExtension ext: String, projectFolder: URL, folder: String,
                            date: Date = Date(), fileManager: FileManager = .default) throws -> String {
        guard data.count <= maximumBytes else { throw SaveError.tooLarge }
        let root = projectFolder.standardizedFileURL
        let dir = folder.isEmpty ? root : root.appendingPathComponent(folder, isDirectory: true)
        do {
            try fileManager.createDirectory(at: dir, withIntermediateDirectories: true)
        } catch {
            throw SaveError.write(error.localizedDescription)
        }
        let resolved = dir.resolvingSymlinksInPath().path, base = root.resolvingSymlinksInPath().path
        guard resolved == base || resolved.hasPrefix(base + "/") else { throw SaveError.outsideProject(folder) }
        var attempt = 0
        while true {
            let name = PasteImageFigure.uniqueFileName(base: PasteImageFigure.baseName(for: date), fileExtension: ext) {
                fileManager.fileExists(atPath: dir.appendingPathComponent($0).path)
            }
            do {
                try data.write(to: dir.appendingPathComponent(name), options: [.withoutOverwriting])
                return PasteImageFigure.graphicsPath(folder: folder, fileName: name)
            } catch {
                attempt += 1
                if attempt < 5, fileManager.fileExists(atPath: dir.appendingPathComponent(name).path) { continue }
                throw SaveError.write(error.localizedDescription)
            }
        }
    }
}
