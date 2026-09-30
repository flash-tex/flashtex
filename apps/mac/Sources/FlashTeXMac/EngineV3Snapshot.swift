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
/// every editor document, the main file, every page's size, the scale) and
/// PNGs of the pages that were near the viewport plus the first ones. The
/// snapshot is used only when every document's text hashes to what it was:
/// an edit outside the app invalidates it. All snapshots together stay under
/// `budgetBytes` (least recently written go first).
struct EngineV3Snapshot: Codable {
    static let version = 1
    static let budgetBytes = 256 << 20
    static let maxPages = 12

    struct Page: Codable { var width: Double; var height: Double; var image: String? }
    var version = Self.version
    var main: String
    var documents: [String: String] // path → sha256 hex
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

    /// The snapshot for these documents, if one exists and still matches them.
    static func load(projectKey: String, documents: [(path: String, text: String)]) -> (EngineV3Snapshot, URL)? {
        let dir = directory(projectKey: projectKey)
        guard let data = try? Data(contentsOf: dir.appendingPathComponent("manifest.json")),
              let s = try? JSONDecoder().decode(EngineV3Snapshot.self, from: data), s.version == version,
              s.documents == hashes(documents) else { return nil }
        return (s, dir)
    }

    /// Writes a snapshot (off the main thread): rasterises `pages` (index →
    /// prepared page) at `pixelsPerPoint`, then the manifest, then trims
    /// the store to the budget.
    static func save(projectKey: String, main: String, documents: [String: String], sizes: [CGSize],
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
        let s = EngineV3Snapshot(main: main, documents: documents, pages: entries, pixelsPerPoint: pixelsPerPoint, dark: dark, savedAt: Date())
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
