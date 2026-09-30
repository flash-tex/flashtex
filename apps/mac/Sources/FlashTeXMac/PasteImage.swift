import AppKit
import Observation
import SwiftUI
import UniformTypeIdentifiers
import FlashTeXEditorCore

/// Paste an image into the LaTeX editor: it is saved into the project and a
/// figure is inserted at the caret (owner decision 2026-09-30,
/// PASTE-IMAGE-FIGURE). This file is the AppKit half: reading the pasteboard,
/// writing the file, and the Settings section. What text goes where is the
/// shared, unit-tested `PasteImageFigure` (FlashTeXEditorCore).
///
/// Behaviour:
/// - **When.** Paste takes the image path only for a single copied image
///   *file* (Finder's Copy, whose file-name string is not meant for the
///   buffer), or for image data (PDF, PNG, JPEG, TIFF) with **nothing
///   text-like** on the pasteboard: plain text, RTF, RTFD (flat or not),
///   HTML, or anything conforming to `public.text` (`textLikeTypes`). A web
///   URL next to the image (a browser's Copy Image) keeps the image path only
///   when the URL is the sole text-like flavour *and* names an image file
///   (`…/plot.png`); any other URL is what the author meant to paste, so the
///   ordinary paste inserts it. Everything else — and everything while the
///   setting is off — pastes exactly as before.
/// - **Where.** `figures/` (the setting; a `\graphicspath` of the root
///   document that does not list it wins — read from the open buffer, or from
///   disk when the root is not open), created if missing; a folder that
///   resolves (through a symlink) outside the project is refused. Named
///   `pasted-YYYYMMDD-HHMMSS.png` (a copied file keeps its own, sanitized,
///   name), never overwriting: `-2`, `-3`, … TIFF (and GIF/HEIC/BMP/WebP
///   files) become PNG; PDF stays vector PDF; PNG and JPEG are written as is.
///   An image file already inside the project is referenced where it is.
///   Images over `maximumBytes` (50 MB) and iCloud files not downloaded yet
///   are refused with a status message (the download is started).
/// - **Off the main thread.** Reading the root from disk, decoding/encoding
///   and writing run on a background queue; the snippet is inserted back on
///   the main thread. If the buffer changed meanwhile the insertion uses the
///   *current* selection; if the editor switched documents nothing is
///   inserted (the status line says where the file went).
/// - **Undo.** The text edit — the snippet, plus `\usepackage{graphicx}`
///   when the root document is the one being edited — is ONE undo step
///   ("Undo Paste Image"). Undo leaves the saved image on disk: deleting a
///   file the author may have referenced elsewhere, or be about to redo, is
///   never the editor's call. The status line names the file.
/// - **graphicx elsewhere.** When the root is another *open* document,
///   graphicx is written into that buffer directly (autosaved). That write is
///   NOT undoable from the editor: ⌘Z in the active document does not remove
///   it, and the status line says so. With the durable helper attached, whose
///   revisions a direct write would bypass, the status line asks for the line
///   instead; likewise when the root is not open at all.
/// - **Preamble.** A caret before `\begin{document}` gets the ordinary paste
///   and a status message: no figure or `\includegraphics` belongs there,
///   and no file is written.
/// - **Unsaved document.** No project directory: nothing is written and the
///   status line says why (non-blocking). A copied file's name still pastes
///   as text, as before.
/// - The engine-v3 preview links the new file into its project copy before
///   the edit's compile (`EngineV3Session.projectFilesChanged`).
@MainActor
enum PasteImage {
    /// What the pasteboard offers as an image.
    enum Source: Equatable, Sendable {
        case data(Data, Kind)
        case file(URL)
    }

    enum Kind: Equatable, Sendable {
        case pdf, png, jpeg, tiff
    }

    /// Image file extensions a pasted file URL may have. pdflatex (and the
    /// FlashTeX engine) read PDF, PNG and JPEG; the rest are converted to PNG.
    nonisolated static let includableExtensions: Set<String> = ["pdf", "png", "jpg", "jpeg"]
    nonisolated static let convertibleExtensions: Set<String> = ["tif", "tiff", "gif", "heic", "heif", "bmp", "webp"]
    /// Largest image the paste writes (bytes).
    nonisolated static let maximumBytes = 50 * 1024 * 1024

    static let jpegType = NSPasteboard.PasteboardType("public.jpeg")
    static let flatRTFDType = NSPasteboard.PasteboardType("com.apple.flat-rtfd")
    /// Flavours that mean the pasteboard carries text for the buffer.
    static let textLikeTypes: [NSPasteboard.PasteboardType] = [.string, .rtf, .rtfd, flatRTFDType, .html]
    private static let imageDataTypes: [(NSPasteboard.PasteboardType, Kind)] = [(.pdf, .pdf), (.png, .png), (jpegType, .jpeg), (.tiff, .tiff)]

    /// The image `pasteboard` holds for an image paste, or nil for an
    /// ordinary paste (text on it, a non-image URL, several files, a
    /// non-image file, nothing).
    static func read(_ pasteboard: NSPasteboard) -> Source? {
        if let url = singleImageFile(on: pasteboard) { return .file(url) }
        guard offersImageData(pasteboard) else { return nil }
        let types = pasteboard.types ?? []
        // Vector first: a PDF flavour next to a TIFF rendering of it is the original.
        for (type, kind) in imageDataTypes where types.contains(type) {
            if let data = pasteboard.data(forType: type), !data.isEmpty { return .data(data, kind) }
        }
        return nil
    }

    /// Whether Paste would be an image paste (the setting is on and the
    /// pasteboard holds one). Reads types and URLs only, never image data:
    /// menu validation calls it.
    static func wouldHandle(_ pasteboard: NSPasteboard, preferences: PasteImagePreferences? = nil) -> Bool {
        guard (preferences ?? .shared).enabled else { return false }
        return singleImageFile(on: pasteboard) != nil || offersImageData(pasteboard)
    }

    /// Image data with nothing text-like beside it, except a web URL that
    /// itself names an image file.
    private static func offersImageData(_ pasteboard: NSPasteboard) -> Bool {
        let types = pasteboard.types ?? []
        guard imageDataTypes.contains(where: { types.contains($0.0) }), !hasFiles(pasteboard), !isTextLike(types) else { return false }
        guard types.contains(.URL) else { return true }
        guard let raw = pasteboard.string(forType: .URL), let url = URL(string: raw) else { return false }
        let ext = url.pathExtension.lowercased()
        return includableExtensions.contains(ext) || convertibleExtensions.contains(ext)
    }

    static func isTextLike(_ types: [NSPasteboard.PasteboardType]) -> Bool {
        types.contains { type in
            textLikeTypes.contains(type) || (UTType(type.rawValue)?.conforms(to: .text) ?? false)
        }
    }

    private static func fileURLs(on pasteboard: NSPasteboard) -> [URL] {
        (pasteboard.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]) as? [URL]) ?? []
    }

    private static func hasFiles(_ pasteboard: NSPasteboard) -> Bool { !fileURLs(on: pasteboard).isEmpty }

    private static func singleImageFile(on pasteboard: NSPasteboard) -> URL? {
        let urls = fileURLs(on: pasteboard)
        guard urls.count == 1, let url = urls.first else { return nil }
        let ext = url.pathExtension.lowercased()
        return includableExtensions.contains(ext) || convertibleExtensions.contains(ext) ? url : nil
    }

    /// Why `source` is refused before any work (too large; an iCloud file
    /// not downloaded yet, whose download is then started), or nil.
    nonisolated static func refusal(for source: Source) -> String? {
        switch source {
        case .data(let data, _):
            return data.count > maximumBytes ? "The pasted image is larger than \(maximumBytes / 1_048_576) MB; it was not saved." : nil
        case .file(let url):
            let values = try? url.resourceValues(forKeys: [.fileSizeKey, .isUbiquitousItemKey, .ubiquitousItemDownloadingStatusKey])
            if values?.isUbiquitousItem == true, let status = values?.ubiquitousItemDownloadingStatus, status != .current {
                try? FileManager.default.startDownloadingUbiquitousItem(at: url)
                return "\(url.lastPathComponent) is in iCloud and not downloaded yet; paste it again once it has downloaded."
            }
            if let size = values?.fileSize, size > maximumBytes {
                return "\(url.lastPathComponent) is larger than \(maximumBytes / 1_048_576) MB; it was not copied."
            }
            return nil
        }
    }

    // MARK: saving (any thread)

    enum SaveError: LocalizedError, Equatable {
        case unreadableImage
        case unreadableFile(String)
        case tooLarge
        case outsideProject(String)
        case write(String)

        var errorDescription: String? {
            switch self {
            case .unreadableImage: "the image could not be converted to PNG"
            case .unreadableFile(let why): "the file could not be read (\(why))"
            case .tooLarge: "the image is larger than 50 MB"
            case .outsideProject(let folder): "the folder \(folder) resolves outside the project"
            case .write(let why): why
            }
        }
    }

    /// Saves `source` under `projectRoot/folder` and returns the path
    /// `\includegraphics` takes (project-relative, `/`-separated). An image
    /// file already inside the project (with a TeX-safe path) is not copied.
    /// Never overwrites; refuses a folder that resolves outside the project.
    nonisolated static func save(_ source: Source, projectRoot: URL, folder: String, date: Date = Date(),
                                 fileManager: FileManager = .default) throws -> String {
        let root = projectRoot.standardizedFileURL
        let bytes: Data
        let base: String
        let ext: String
        switch source {
        case .data(let data, let kind):
            guard data.count <= maximumBytes else { throw SaveError.tooLarge }
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
            guard data.count <= maximumBytes else { throw SaveError.tooLarge }
            base = PasteImageFigure.sanitizedBaseName(url.deletingPathExtension().lastPathComponent)
            if includableExtensions.contains(fileExt) {
                (bytes, ext) = (data, fileExt)
            } else {
                guard let png = pngData(from: data) else { throw SaveError.unreadableImage }
                (bytes, ext) = (png, "png")
            }
        }
        let dir = folder.isEmpty ? root : root.appendingPathComponent(folder, isDirectory: true)
        guard resolvesInside(dir, root: root, fileManager: fileManager) else { throw SaveError.outsideProject(folder) }
        do {
            try fileManager.createDirectory(at: dir, withIntermediateDirectories: true)
        } catch {
            throw SaveError.write(error.localizedDescription)
        }
        guard resolvesInside(dir, root: root, fileManager: fileManager) else { throw SaveError.outsideProject(folder) }
        do {
            let name = PasteImageFigure.uniqueFileName(base: base, fileExtension: ext) {
                fileManager.fileExists(atPath: dir.appendingPathComponent($0).path)
            }
            try bytes.write(to: dir.appendingPathComponent(name), options: [.withoutOverwriting])
            return PasteImageFigure.graphicsPath(folder: folder, fileName: name)
        } catch {
            throw SaveError.write(error.localizedDescription)
        }
    }

    /// Whether `dir` — through its nearest existing ancestor, symlinks
    /// resolved — lies inside `root`.
    nonisolated static func resolvesInside(_ dir: URL, root: URL, fileManager: FileManager = .default) -> Bool {
        var probe = dir.standardizedFileURL
        while !fileManager.fileExists(atPath: probe.path), probe.pathComponents.count > 1 { probe = probe.deletingLastPathComponent() }
        let resolved = probe.resolvingSymlinksInPath().path
        let base = root.standardizedFileURL.resolvingSymlinksInPath().path
        return resolved == base || resolved.hasPrefix(base + "/")
    }

    /// `url`'s path relative to `root` when it lies inside it and TeX can
    /// read the path as written; nil otherwise.
    nonisolated static func projectRelativePath(of url: URL, root: URL) -> String? {
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
    nonisolated static func pngData(from data: Data) -> Data? {
        let rep = NSBitmapImageRep(data: data) ?? NSImage(data: data)?.tiffRepresentation.flatMap(NSBitmapImageRep.init(data:))
        return rep?.representation(using: .png, properties: [:])
    }

    /// The background half of a paste: the root's text (the open buffer, or
    /// read from disk), the folder it implies, and the saved file.
    struct Job: Sendable {
        var source: Source
        var projectRoot: URL
        var configuredFolder: String
        /// The root document's open buffer; nil reads `rootURL` from disk.
        var rootText: String?
        var rootURL: URL?
        var date: Date
    }

    struct Outcome: Sendable {
        var saved: Result<String, SaveError>
        /// The root document's text as read from disk (nil when it was open or unreadable).
        var rootDiskText: String?
    }

    nonisolated static func perform(_ job: Job) -> Outcome {
        var disk: String?
        if job.rootText == nil, let url = job.rootURL { disk = try? String(contentsOf: url, encoding: .utf8) }
        let folder = PasteImageFigure.imageFolder(configured: job.configuredFolder, rootText: job.rootText ?? disk ?? "")
        do {
            return Outcome(saved: .success(try save(job.source, projectRoot: job.projectRoot, folder: folder, date: job.date)), rootDiskText: disk)
        } catch let error as SaveError {
            return Outcome(saved: .failure(error), rootDiskText: disk)
        } catch {
            return Outcome(saved: .failure(.write(error.localizedDescription)), rootDiskText: disk)
        }
    }

    // MARK: host

    /// What the editor needs from the project for a paste.
    struct Host {
        /// The project directory (the root document's folder); nil for an unsaved document.
        var projectRoot: URL?
        /// The document the editor shows.
        var activePath: String
        /// The root (entry) document's project-relative path.
        var rootPath: String
        /// Whether the editor shows the root document (graphicx then joins the paste's undo step).
        var editingRoot: Bool
        /// The root document's open buffer; nil when it is not open (then it is read from disk).
        var rootText: String?
        /// Adds graphicx to the root document when it is open but not the
        /// edited one; returns a status fragment (nil: nothing to say).
        var ensureGraphicxInRoot: () -> String?
        /// Shows a non-blocking status message.
        var note: (String) -> Void
        /// A file was added to the project (the preview links it in).
        var filesChanged: () -> Void = {}
    }

    static let unsavedNote = "Save the document to paste images: they are saved into the project folder, next to it."
    static let preambleNote = "Images cannot go in the preamble: move the caret below \\begin{document} and paste again."
}

// MARK: - model

extension ShellModel {
    /// The project facts an image paste needs (`SourceEditorView.imagePasteHost`).
    func imagePasteHost() -> PasteImage.Host {
        let entry = project.entryPath
        return PasteImage.Host(
            projectRoot: project.projectRoot,
            activePath: activePath,
            rootPath: entry,
            editingRoot: activePath == entry,
            rootText: documents.first(where: { $0.path == entry })?.text,
            ensureGraphicxInRoot: { [weak self] in self?.ensureGraphicxInEntryBuffer() },
            note: { [weak self] in self?.navigationNote = $0 },
            filesChanged: { [weak self] in
                guard let self, self.engineV3Enabled else { return }
                self.engineV3.projectFilesChanged(model: self)
            }
        )
    }

    /// Adds `\usepackage{graphicx}` to the entry document's buffer while
    /// another document is active. Without the durable helper this is a
    /// direct buffer write (as a multi-file rename applies to open non-active
    /// buffers) that autosaves — and is not on any undo stack: ⌘Z in the
    /// active editor does not remove it, which the returned note says. With
    /// the helper attached (`helperAttached`, default `controllerAttached`)
    /// the write would bypass its revisions, so the author is asked instead.
    func ensureGraphicxInEntryBuffer(helperAttached: Bool? = nil) -> String? {
        let entry = project.entryPath
        guard entry != activePath, let i = documents.firstIndex(where: { $0.path == entry }) else { return nil }
        let text = documents[i].text
        guard let insertion = PasteImageFigure.graphicxInsertion(in: text) else {
            return PasteImageFigure.loadsGraphicx(in: text) ? nil : "Load graphicx in the root document's preamble."
        }
        if helperAttached ?? controllerAttached { return "Add \\usepackage{graphicx} to \(entry)." }
        documents[i].text = (text as NSString).replacingCharacters(in: NSRange(location: insertion.location, length: 0), with: insertion.text)
        scheduleAutosave() // `updateActiveText` does this for the active buffer only
        return "Added \\usepackage{graphicx} to \(entry) (⌘Z here does not remove it)."
    }
}

// MARK: - editor

extension SourceEditorView.Coordinator {
    /// Paste with an image on `pasteboard`. Returns false to let the ordinary
    /// paste run (no image, the setting off, a copied file in an unsaved
    /// document, a caret in the preamble); true when the paste is taken. The
    /// file is saved off the main thread and the figure inserted afterwards
    /// as one undo step; `completion` reports whether text was inserted.
    @discardableResult
    func pasteImage(from pasteboard: NSPasteboard, in tv: NSTextView, date: Date = Date(),
                    preferences: PasteImagePreferences? = nil,
                    completion: @escaping @MainActor (Bool) -> Void = { _ in }) -> Bool {
        let prefs = preferences ?? .shared
        guard prefs.enabled, tv.isEditable, !tv.hasMarkedText(),
              let host = parent.imagePasteHost(), let source = PasteImage.read(pasteboard) else { return false }
        var isFile = false
        if case .file = source { isFile = true }
        guard let root = host.projectRoot else {
            host.note(PasteImage.unsavedNote)
            // A copied file's name still pastes as text, as before; bare image data has nothing else to paste.
            return !isFile
        }
        let text = SourceEditorView.nativeText(of: tv)
        let selection = tv.selectedRange()
        let context = PasteImageFigure.context(in: text, caret: selection.location, selectionEnd: NSMaxRange(selection),
                                               mathMode: mathMode(at: selection.location, in: tv))
        if context.inPreamble {
            host.note(PasteImage.preambleNote)
            return false
        }
        if let why = PasteImage.refusal(for: source) {
            host.note(why)
            NSSound.beep()
            return true
        }
        let options = prefs.options(indentUnit: EditorPreferences.shared.indentString)
        let job = PasteImage.Job(source: source, projectRoot: root, configuredFolder: options.folder,
                                 rootText: host.editingRoot ? text : host.rootText,
                                 rootURL: root.appendingPathComponent(host.rootPath), date: date)
        let finish: @MainActor (PasteImage.Outcome) -> Void = { [weak self, weak tv] outcome in
            guard let self, let tv else { completion(false); return }
            completion(self.finishImagePaste(outcome, in: tv, host: host, snapshot: text, selection: selection, options: options))
        }
        DispatchQueue.global(qos: .userInitiated).async {
            let outcome = PasteImage.perform(job)
            DispatchQueue.main.async { MainActor.assumeIsolated { finish(outcome) } }
        }
        return true
    }

    /// The main-thread half: revalidate, insert (one undo group), report.
    private func finishImagePaste(_ outcome: PasteImage.Outcome, in tv: NSTextView, host: PasteImage.Host,
                                  snapshot: String, selection: NSRange, options: PasteImageFigure.Options) -> Bool {
        let path: String
        switch outcome.saved {
        case .failure(let error):
            host.note("Could not save the pasted image: \(error.localizedDescription)")
            NSSound.beep()
            return false
        case .success(let p): path = p
        }
        host.filesChanged() // before the edit, so its compile finds the file
        let now = parent.imagePasteHost()
        guard now?.activePath == host.activePath, tv.isEditable, !tv.hasMarkedText() else {
            host.note("Pasted image saved as \(path); not inserted because the editor changed documents.")
            return false
        }
        let text = SourceEditorView.nativeText(of: tv)
        // Unchanged buffer: the selection the paste was made at. Otherwise its
        // offsets mean nothing any more, and the current selection is used.
        let target = text == snapshot ? selection : tv.selectedRange()
        let label = PasteImageFigure.sanitizedBaseName(((path as NSString).lastPathComponent as NSString).deletingPathExtension)
        guard let plan = PasteImageFigure.plan(text: text, selection: target, path: path, label: label, options: options,
                                               mathMode: mathMode(at: target.location, in: tv), ensureGraphicx: host.editingRoot) else {
            host.note("Pasted image saved as \(path); not inserted: the caret is in the preamble.")
            return false
        }
        (tv as? CompletingTextView)?.endSnippet()
        applyLineEdits(plan.edits, to: tv, actionName: "Paste Image", selection: plan.selection)
        tv.scrollRangeToVisible(plan.selection)
        var message = "Pasted image saved as \(path)."
        if plan.addsGraphicx {
            message += " Added \\usepackage{graphicx}."
        } else if host.editingRoot {
            if !PasteImageFigure.loadsGraphicx(in: text) { message += " Load graphicx in the preamble." }
        } else if host.rootText != nil {
            if let extra = host.ensureGraphicxInRoot() { message += " " + extra }
        } else if let disk = outcome.rootDiskText, !PasteImageFigure.loadsGraphicx(in: disk) {
            message += " Add \\usepackage{graphicx} to \(host.rootPath)."
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
