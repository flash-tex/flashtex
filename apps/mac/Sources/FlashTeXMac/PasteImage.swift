import AppKit
import Observation
import SwiftUI
import FlashTeXEditorCore

/// Paste an image into the LaTeX editor: it is saved into the project and a
/// figure is inserted at the caret (owner decision 2026-09-30,
/// PASTE-IMAGE-FIGURE). This file is the AppKit half: reading the pasteboard,
/// writing the file, and the Settings section. What text goes where is the
/// shared, unit-tested `PasteImageFigure` (FlashTeXEditorCore).
///
/// Behaviour:
/// - Paste takes the image path only when the pasteboard holds a single image
///   *file* (Finder's Copy, whose file-name string is not meant for the
///   buffer), or image data (PDF, PNG, JPEG, TIFF) and **no text**. Anything
///   with text on it pastes exactly as before, as does everything while the
///   setting is off.
/// - The file goes to `figures/` (the setting; a `\graphicspath` of the root
///   document that does not list it wins), created if missing, named
///   `pasted-YYYYMMDD-HHMMSS.png` (a copied file keeps its own, sanitized,
///   name), never overwriting: `-2`, `-3`, … TIFF (and GIF/HEIC/BMP/WebP
///   files) become PNG; PDF stays vector PDF; PNG and JPEG are written as is.
///   An image file already inside the project is referenced where it is.
/// - The text edit — the snippet, plus `\usepackage{graphicx}` when the root
///   document is the one being edited — is ONE undo step ("Undo Paste
///   Image"). Undo leaves the saved image on disk: deleting a file the
///   author may have referenced elsewhere, or be about to redo, is never
///   the editor's call. The status line names the file so it can be removed
///   by hand.
/// - When the root document is another open file, graphicx is added to that
///   buffer directly (a separate edit, undone from its own tab), or — with
///   the durable helper attached, whose revisions a direct write would
///   bypass — the status line asks for it instead.
/// - An unsaved document has no project directory: nothing is written and
///   the status line says why (non-blocking).
@MainActor
enum PasteImage {
    /// What the pasteboard offers as an image.
    enum Source: Equatable {
        case data(Data, Kind)
        case file(URL)
    }

    enum Kind: Equatable {
        case pdf, png, jpeg, tiff
    }

    /// Image file extensions a pasted file URL may have. pdflatex (and the
    /// FlashTeX engine) read PDF, PNG and JPEG; the rest are converted to PNG.
    static let includableExtensions: Set<String> = ["pdf", "png", "jpg", "jpeg"]
    static let convertibleExtensions: Set<String> = ["tif", "tiff", "gif", "heic", "heif", "bmp", "webp"]

    static let jpegType = NSPasteboard.PasteboardType("public.jpeg")

    /// The image `pasteboard` holds for an image paste, or nil for an
    /// ordinary paste (text on it, several files, a non-image file, nothing).
    static func read(_ pasteboard: NSPasteboard) -> Source? {
        if let url = singleImageFile(on: pasteboard) { return .file(url) }
        guard !hasFiles(pasteboard), !hasText(pasteboard) else { return nil }
        let types = pasteboard.types ?? []
        // Vector first: a PDF flavour next to a TIFF rendering of it is the original.
        for (type, kind) in [(NSPasteboard.PasteboardType.pdf, Kind.pdf), (.png, .png), (jpegType, .jpeg), (.tiff, .tiff)]
        where types.contains(type) {
            if let data = pasteboard.data(forType: type), !data.isEmpty { return .data(data, kind) }
        }
        return nil
    }

    /// Whether Paste would be an image paste (the setting is on and the
    /// pasteboard holds one). Reads types and URLs only, never image data:
    /// menu validation calls it.
    static func wouldHandle(_ pasteboard: NSPasteboard) -> Bool {
        guard PasteImagePreferences.shared.enabled else { return false }
        if singleImageFile(on: pasteboard) != nil { return true }
        guard !hasFiles(pasteboard), !hasText(pasteboard) else { return false }
        let types = pasteboard.types ?? []
        return [.pdf, .png, jpegType, .tiff].contains(where: types.contains)
    }

    private static func fileURLs(on pasteboard: NSPasteboard) -> [URL] {
        (pasteboard.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]) as? [URL]) ?? []
    }

    private static func hasFiles(_ pasteboard: NSPasteboard) -> Bool { !fileURLs(on: pasteboard).isEmpty }

    private static func hasText(_ pasteboard: NSPasteboard) -> Bool {
        (pasteboard.types ?? []).contains(.string)
    }

    private static func singleImageFile(on pasteboard: NSPasteboard) -> URL? {
        let urls = fileURLs(on: pasteboard)
        guard urls.count == 1, let url = urls.first else { return nil }
        let ext = url.pathExtension.lowercased()
        return includableExtensions.contains(ext) || convertibleExtensions.contains(ext) ? url : nil
    }

    // MARK: saving

    enum SaveError: LocalizedError, Equatable {
        case unreadableImage
        case unreadableFile(String)
        case write(String)

        var errorDescription: String? {
            switch self {
            case .unreadableImage: "the pasteboard's image could not be converted to PNG"
            case .unreadableFile(let why): "the file could not be read (\(why))"
            case .write(let why): why
            }
        }
    }

    /// Saves `source` under `projectRoot/folder` and returns the path
    /// `\includegraphics` takes (project-relative, `/`-separated). An image
    /// file already inside the project (with a TeX-safe path) is not copied.
    static func save(_ source: Source, projectRoot: URL, folder: String, date: Date = Date(),
                     fileManager: FileManager = .default) throws -> String {
        let root = projectRoot.standardizedFileURL
        let bytes: Data
        let base: String
        let ext: String
        switch source {
        case .data(let data, let kind):
            base = PasteImageFigure.baseName(for: date)
            switch kind {
            case .pdf: (bytes, ext) = (data, "pdf")
            case .png: (bytes, ext) = (data, "png")
            case .jpeg: (bytes, ext) = (data, "jpg")
            case .tiff:
                guard let png = pngData(from: data) else { throw SaveError.unreadableImage }
                (bytes, ext) = (png, "png")
            }
        case .file(let url):
            let fileExt = url.pathExtension.lowercased()
            if includableExtensions.contains(fileExt), let inside = projectRelativePath(of: url, root: root) { return inside }
            let data: Data
            do { data = try Data(contentsOf: url) } catch { throw SaveError.unreadableFile(error.localizedDescription) }
            base = PasteImageFigure.sanitizedBaseName(url.deletingPathExtension().lastPathComponent)
            if includableExtensions.contains(fileExt) {
                (bytes, ext) = (data, fileExt)
            } else {
                guard let png = pngData(from: data) else { throw SaveError.unreadableImage }
                (bytes, ext) = (png, "png")
            }
        }
        let dir = folder.isEmpty ? root : root.appendingPathComponent(folder, isDirectory: true)
        do {
            try fileManager.createDirectory(at: dir, withIntermediateDirectories: true)
            let name = PasteImageFigure.uniqueFileName(base: base, fileExtension: ext) {
                fileManager.fileExists(atPath: dir.appendingPathComponent($0).path)
            }
            try bytes.write(to: dir.appendingPathComponent(name), options: [.withoutOverwriting])
            return PasteImageFigure.graphicsPath(folder: folder, fileName: name)
        } catch {
            throw SaveError.write(error.localizedDescription)
        }
    }

    /// `url`'s path relative to `root` when it lies inside it and TeX can
    /// read the path as written; nil otherwise.
    static func projectRelativePath(of url: URL, root: URL) -> String? {
        let file = url.standardizedFileURL.resolvingSymlinksInPath().path
        let base = root.resolvingSymlinksInPath().path
        guard file.hasPrefix(base + "/") else { return nil }
        let relative = String(file.dropFirst(base.count + 1))
        let stem = (relative as NSString).deletingPathExtension
        // The directory part must be a safe folder and the stem must not carry
        // a second dot (graphicx would take it for the extension).
        guard let dir = PasteImageFigure.normalizedFolder(stem), dir == stem,
              !(stem as NSString).lastPathComponent.contains(".") else { return nil }
        return relative
    }

    /// PNG bytes of any image AppKit reads (TIFF, GIF, HEIC, BMP, WebP).
    static func pngData(from data: Data) -> Data? {
        let rep = NSBitmapImageRep(data: data) ?? NSImage(data: data)?.tiffRepresentation.flatMap(NSBitmapImageRep.init(data:))
        return rep?.representation(using: .png, properties: [:])
    }

    // MARK: host

    /// What the editor needs from the project for a paste.
    struct Host {
        /// The project directory (the root document's folder); nil for an unsaved document.
        var projectRoot: URL?
        /// Whether the editor shows the root document (graphicx then joins the paste's undo step).
        var editingRoot: Bool
        /// The root document's buffer (for `\graphicspath` when another file is being edited).
        var rootText: String
        /// Adds graphicx to the root document when it is not the edited one;
        /// returns a status fragment (nil: nothing to say).
        var ensureGraphicxInRoot: () -> String?
        /// Shows a non-blocking status message.
        var note: (String) -> Void
    }

    static let unsavedNote = "Save the document to paste images: they are saved into the project folder, next to it."
}

// MARK: - model

extension ShellModel {
    /// The project facts an image paste needs (`SourceEditorView.imagePasteHost`).
    func imagePasteHost() -> PasteImage.Host {
        let entry = project.entryPath
        return PasteImage.Host(
            projectRoot: project.projectRoot,
            editingRoot: activePath == entry,
            rootText: entryText,
            ensureGraphicxInRoot: { [weak self] in self?.ensureGraphicxInEntryBuffer() },
            note: { [weak self] in self?.navigationNote = $0 }
        )
    }

    /// Adds `\usepackage{graphicx}` to the entry document's buffer while
    /// another document is active. Without the durable helper this is a
    /// direct buffer write (as a multi-file rename applies to open non-active
    /// buffers) that autosaves; with it attached the write would bypass the
    /// helper's revisions, so the author is asked to add the line instead.
    func ensureGraphicxInEntryBuffer() -> String? {
        let entry = project.entryPath
        guard entry != activePath, let i = documents.firstIndex(where: { $0.path == entry }) else { return nil }
        let text = documents[i].text
        guard let insertion = PasteImageFigure.graphicxInsertion(in: text) else {
            return PasteImageFigure.loadsGraphicx(in: text) ? nil : "Load graphicx in the root document's preamble."
        }
        if controllerAttached { return "Add \\usepackage{graphicx} to \(entry)." }
        documents[i].text = (text as NSString).replacingCharacters(in: NSRange(location: insertion.location, length: 0), with: insertion.text)
        scheduleAutosave() // `updateActiveText` does this for the active buffer only
        return "Added \\usepackage{graphicx} to \(entry)."
    }
}

// MARK: - editor

extension SourceEditorView.Coordinator {
    /// Paste with an image on `pasteboard`: save it, insert the figure (one
    /// undo step), report where it went. False lets the ordinary paste run.
    func pasteImage(from pasteboard: NSPasteboard, in tv: NSTextView, date: Date = Date(),
                    preferences: PasteImagePreferences? = nil) -> Bool {
        let prefs = preferences ?? .shared
        guard prefs.enabled, tv.isEditable, !tv.hasMarkedText(),
              let host = parent.imagePasteHost(), let source = PasteImage.read(pasteboard) else { return false }
        guard let root = host.projectRoot else {
            host.note(PasteImage.unsavedNote)
            // A copied file's name still pastes as text, as before; bare image data has nothing else to paste.
            if case .file = source { return false }
            return true
        }
        let text = SourceEditorView.nativeText(of: tv)
        let options = prefs.options(indentUnit: EditorPreferences.shared.indentString)
        let folder = PasteImageFigure.imageFolder(configured: options.folder, rootText: host.editingRoot ? text : host.rootText)
        let path: String
        do {
            path = try PasteImage.save(source, projectRoot: root, folder: folder, date: date)
        } catch {
            host.note("Could not save the pasted image: \(error.localizedDescription)")
            NSSound.beep()
            return true
        }
        let label = PasteImageFigure.sanitizedBaseName(((path as NSString).lastPathComponent as NSString).deletingPathExtension)
        let selection = tv.selectedRange()
        let plan = PasteImageFigure.plan(text: text, selection: selection, path: path, label: label, options: options,
                                         mathMode: mathMode(at: selection.location, in: tv), ensureGraphicx: host.editingRoot)
        (tv as? CompletingTextView)?.endSnippet()
        applyLineEdits(plan.edits, to: tv, actionName: "Paste Image", selection: plan.selection)
        tv.scrollRangeToVisible(plan.selection)
        var message = "Pasted image saved as \(path)."
        if plan.addsGraphicx {
            message += " Added \\usepackage{graphicx}."
        } else if !host.editingRoot {
            if let extra = host.ensureGraphicxInRoot() { message += " " + extra }
        } else if !PasteImageFigure.loadsGraphicx(in: text) {
            message += " Load graphicx in the preamble."
        }
        host.note(message)
        announceNow(text: currentText(of: tv), range: tv.selectedRange(), prefix: "Pasted image \(path). ")
        return true
    }
}

// MARK: - preferences

/// Settings for pasting images (Settings > Images). Default on. Kept apart
/// from `EditorPreferences` so the feature's settings stay self-contained;
/// stored under `FlashTeX.PasteImage.v1.*`.
@Observable @MainActor
final class PasteImagePreferences {
    static let shared = PasteImagePreferences(defaults: .standard)

    enum Key: String, CaseIterable {
        case enabled, folder, width, wrapInFigure
        var storageKey: String { "FlashTeX.PasteImage.v1." + rawValue }
    }

    @ObservationIgnored private let defaults: UserDefaults

    var enabled: Bool { didSet { defaults.set(enabled, forKey: Key.enabled.storageKey) } }
    /// Project-relative folder; an invalid one (absolute, `..`, spaces) falls back to `figures`.
    var folder: String { didSet { defaults.set(folder, forKey: Key.folder.storageKey) } }
    /// `\includegraphics` width (`0.8\linewidth`); empty for none, `key=value` for a full option list.
    var width: String { didSet { defaults.set(width, forKey: Key.width.storageKey) } }
    var wrapInFigure: Bool { didSet { defaults.set(wrapInFigure, forKey: Key.wrapInFigure.storageKey) } }

    init(defaults: UserDefaults) {
        self.defaults = defaults
        enabled = defaults.object(forKey: Key.enabled.storageKey) as? Bool ?? true
        folder = defaults.string(forKey: Key.folder.storageKey) ?? PasteImageFigure.Options.defaultFolder
        width = defaults.string(forKey: Key.width.storageKey) ?? PasteImageFigure.Options.defaultWidth
        wrapInFigure = defaults.object(forKey: Key.wrapInFigure.storageKey) as? Bool ?? true
    }

    func options(indentUnit: String) -> PasteImageFigure.Options {
        PasteImageFigure.Options(folder: folder, width: width, wrapInFigure: wrapInFigure, indentUnit: indentUnit)
    }

    /// Whether `folder` is usable as typed (else `figures` is used).
    var folderIsValid: Bool { PasteImageFigure.normalizedFolder(folder) != nil }

    func resetToDefaults() {
        enabled = true
        folder = PasteImageFigure.Options.defaultFolder
        width = PasteImageFigure.Options.defaultWidth
        wrapInFigure = true
    }
}

/// Settings > Images: the paste-image-to-figure switches.
struct PasteImageSettingsSection: View {
    @Bindable var preferences: PasteImagePreferences = .shared

    var body: some View {
        Section("Pasted images") {
            Toggle("Paste images as figures", isOn: $preferences.enabled)
                .accessibilityHint("When on, pasting an image into a saved document saves it in the project and inserts a figure. Text pastes are never affected.")
            Group {
                TextField("Image folder", text: $preferences.folder, prompt: Text(PasteImageFigure.Options.defaultFolder))
                    .accessibilityHint("Folder inside the project where pasted images are saved. A \\graphicspath in the root document that does not list it takes precedence.")
                if !preferences.folderIsValid {
                    Text("Not a project-relative folder TeX can read; “\(PasteImageFigure.Options.defaultFolder)” is used.")
                        .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                }
                TextField("Width", text: $preferences.width, prompt: Text(PasteImageFigure.Options.defaultWidth))
                    .accessibilityHint("The includegraphics width. Leave empty for none, or type a full option list such as height=4cm.")
                Toggle("Wrap in a figure environment", isOn: $preferences.wrapInFigure)
                    .accessibilityHint("Adds a centred figure with a caption and label. Inside a figure, a table or math, only the includegraphics line is inserted.")
            }
            .disabled(!preferences.enabled)
            Text("Undo removes the inserted text but keeps the saved image file.")
                .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
            Button("Restore Defaults") { preferences.resetToDefaults() }
        }
    }
}
