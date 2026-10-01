import AppKit
import CryptoKit
import Foundation
import ImageIO
import UniformTypeIdentifiers
import FlashTeXPreviewV3

/// Instant reopen (owner decision 8A): the last rendered pages of a project,
/// kept on disk and shown the moment the project opens — before the engine
/// host has even started — marked stale until the recompile replaces them.
///
/// Per project (keyed by the project's path): `manifest.json` (the SHA-256 of
/// every editor document, the modification time and size of every other
/// input file in the project folder, the main file, every page's size, the
/// scale) and PNGs of the pages that were near the viewport plus the first
/// ones. The snapshot is used only when every editor document's text hashes
/// to what it was AND every other input (`.tex`, `.bib`, `.sty`/`.cls`,
/// images, ... — `inputExtensions`) has the same time and size, none added
/// or removed: a chapter, bibliography or figure changed outside the app
/// invalidates it. When the folder cannot be listed in full (`maxEntries`)
/// nothing is known, so nothing is saved or shown. Shown pages are stale
/// until the compile sends each one. All snapshots together stay under
/// `budgetBytes` (least recently written go first).
struct EngineV3Snapshot: Codable {
    static let version = 2
    static let budgetBytes = 256 << 20
    static let maxPages = 12

    struct Page: Codable { var width: Double; var height: Double; var image: String? }
    var version = Self.version
    var main: String
    var documents: [String: String] // path → sha256 hex
    /// Every other input file: path → "mtime:size" (`inputs(root:)`).
    var inputs: [String: String]
    var pages: [Page]
    var pixelsPerPoint: Double
    var dark: Bool
    var savedAt: Date

    static var root: URL { EngineV3.cacheDirectory.appendingPathComponent("snapshots", isDirectory: true) }

    static func directory(projectKey: String) -> URL { root.appendingPathComponent(projectKey, isDirectory: true) }

    /// A project's key: the project directory (or, for an untitled buffer, nothing: no snapshot).
    @MainActor static func key(for model: ShellModel) -> String? {
        guard let root = model.project.projectRoot else { return nil }
        return SHA256.hash(data: Data((root.standardizedFileURL.path + "\u{0}" + model.project.entryPath).utf8))
            .prefix(12).map { String(format: "%02x", $0) }.joined()
    }

    static func hashes(_ docs: [(path: String, text: String)]) -> [String: String] {
        var out: [String: String] = [:]
        for d in docs { out[d.path] = SHA256.hash(data: Data(d.text.utf8)).map { String(format: "%02x", $0) }.joined() }
        return out
    }

    // MARK: inputs

    /// Files a compile may read, besides the editor's documents.
    static let inputExtensions: Set<String> = [
        "tex", "ltx", "sty", "cls", "clo", "cfg", "def", "fd", "dtx", "ins",
        "bib", "bst", "bbl", "bbx", "cbx", "lbx", "dbx", "aux", "toc", "lof", "lot", "ind", "idx", "gls", "nls",
        "png", "jpg", "jpeg", "pdf", "eps", "ps", "mps", "svg", "gif", "tif", "tiff", "bmp", "jbig2", "jb2",
        "pgf", "tikz", "csv", "dat", "tsv", "txt", "lua", "map", "enc", "tfm", "vf", "pfb", "otf", "ttf",
    ]
    /// Directory entries listed at most (the project copy's bound, EngineV3Mirror.sync).
    static let maxEntries = 20_000

    static func isInput(_ rel: String) -> Bool { inputExtensions.contains((rel as NSString).pathExtension.lowercased()) }

    /// "mtime:size" of a file (following a link), nil when it cannot be read.
    static func fingerprint(_ path: String) -> String? {
        var st = stat()
        guard stat(path, &st) == 0 else { return nil }
        return "\(st.st_mtimespec.tv_sec).\(st.st_mtimespec.tv_nsec):\(st.st_size)"
    }

    /// Every input file under `root` (relative path → fingerprint), as
    /// EngineV3Mirror.sync walks it (hidden files and packages skipped).
    /// Nil when unsure: the folder cannot be listed, or has more than
    /// `maxEntries` entries.
    static func inputs(root: URL) -> [String: String]? {
        let fm = FileManager.default
        guard let e = fm.enumerator(at: root, includingPropertiesForKeys: [.isDirectoryKey], options: [.skipsHiddenFiles, .skipsPackageDescendants]) else { return nil }
        let prefix = root.standardizedFileURL.path + "/"
        var out: [String: String] = [:]
        var n = 0
        for case let url as URL in e {
            n += 1
            if n > maxEntries { return nil }
            let path = url.standardizedFileURL.path
            guard path.hasPrefix(prefix) else { continue }
            let rel = String(path.dropFirst(prefix.count))
            guard isInput(rel), (try? url.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory != true else { continue }
            out[rel] = fingerprint(path) ?? "unreadable"
        }
        return out
    }

    /// `inputs` without the editor's documents (their text is hashed instead).
    static func others(_ inputs: [String: String], documents: some Sequence<String>) -> [String: String] {
        var out = inputs
        for d in documents { out[d] = nil }
        return out
    }

    /// The snapshot for these documents, if one exists and still matches
    /// them and every other input file under `root`.
    static func load(projectKey: String, root: URL, documents: [(path: String, text: String)]) -> (EngineV3Snapshot, URL)? {
        let dir = directory(projectKey: projectKey)
        guard let data = try? Data(contentsOf: dir.appendingPathComponent("manifest.json")),
              let s = try? JSONDecoder().decode(EngineV3Snapshot.self, from: data), s.version == version,
              s.documents == hashes(documents),
              let now = inputs(root: root), others(now, documents: s.documents.keys) == s.inputs else { return nil }
        return (s, dir)
    }

    /// Writes a snapshot (off the main thread): rasterises `pages` (index →
    /// prepared page) at `pixelsPerPoint`, then the manifest, then trims
    /// the store to the budget.
    static func save(projectKey: String, main: String, documents: [String: String], inputs: [String: String], sizes: [CGSize],
                     pages: [Int: DL3PreparedPage], forms: [UInt32: DL3PreparedPage], pixelsPerPoint: Double, dark: Bool) {
        let dir = directory(projectKey: projectKey)
        let fm = FileManager.default
        try? fm.createDirectory(at: dir, withIntermediateDirectories: true)
        var entries = sizes.map { Page(width: Double($0.width), height: Double($0.height), image: nil) }
        for (i, p) in pages where i < entries.count {
            guard let img = DL3Renderer.rasterize(p, forms: forms, scale: pixelsPerPoint, appearance: dark ? .dark : .light) else { continue }
            let name = "page-\(i).png"
            let url = dir.appendingPathComponent(name)
            guard let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil) else { continue }
            CGImageDestinationAddImage(dest, img, nil)
            if CGImageDestinationFinalize(dest) { entries[i].image = name }
        }
        // Stale page images of an older snapshot.
        let keep = Set(entries.compactMap(\.image) + ["manifest.json"])
        for name in (try? fm.contentsOfDirectory(atPath: dir.path)) ?? [] where !keep.contains(name) {
            try? fm.removeItem(at: dir.appendingPathComponent(name))
        }
        let s = EngineV3Snapshot(main: main, documents: documents, inputs: inputs, pages: entries, pixelsPerPoint: pixelsPerPoint, dark: dark, savedAt: Date())
        if let data = try? JSONEncoder().encode(s) { try? data.write(to: dir.appendingPathComponent("manifest.json"), options: .atomic) }
        trim()
    }

    /// Keeps all snapshots under the budget, least recently written first out.
    static func trim() {
        let fm = FileManager.default
        guard let names = try? fm.contentsOfDirectory(atPath: root.path) else { return }
        var dirs: [(url: URL, date: Date, bytes: Int)] = []
        for n in names {
            let d = root.appendingPathComponent(n)
            let files = (try? fm.contentsOfDirectory(at: d, includingPropertiesForKeys: [.fileSizeKey, .contentModificationDateKey])) ?? []
            let bytes = files.reduce(0) { $0 + ((try? $1.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? 0) }
            let date = (try? d.appendingPathComponent("manifest.json").resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate) ?? .distantPast
            dirs.append((d, date, bytes))
        }
        var total = dirs.reduce(0) { $0 + $1.bytes }
        for d in dirs.sorted(by: { $0.date < $1.date }) where total > budgetBytes {
            try? fm.removeItem(at: d.url)
            total -= d.bytes
        }
    }

    /// A page's stored bitmap.
    static func image(_ url: URL) -> CGImage? {
        guard let src = CGImageSourceCreateWithURL(url as CFURL, nil) else { return nil }
        return CGImageSourceCreateImageAtIndex(src, 0, [kCGImageSourceShouldCacheImmediately: true] as CFDictionary)
    }
}
