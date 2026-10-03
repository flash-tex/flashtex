import Foundation

/// Folder access for the iPad (IPAD-FOLDER-ACCESS). A `.tex` file opened
/// through the document picker grants a security scope for that file only,
/// so its folder — where pasted images go — is not writable. The user grants
/// the folder once through a folder picker; its security-scoped bookmark is
/// kept here (UserDefaults, keyed by the folder's path) and entered for later
/// writes in that folder or any folder below it.
///
/// On iOS a bookmark made while a picked URL's scope is entered carries that
/// scope (there is no `.withSecurityScope` option there); the URL it resolves
/// to must be entered again (`startAccessingSecurityScopedResource`) before use.
public struct PadFolderBookmarks {
    /// Makes and resolves bookmark data; the system one by default, a fake in tests.
    public struct Codec: Sendable {
        public var make: @Sendable (URL) throws -> Data
        public var resolve: @Sendable (Data) throws -> (url: URL, stale: Bool)

        public init(make: @escaping @Sendable (URL) throws -> Data,
                    resolve: @escaping @Sendable (Data) throws -> (url: URL, stale: Bool)) {
            self.make = make
            self.resolve = resolve
        }

        public static let system = Codec(
            make: { url in
                // A picked URL's scope must be entered while its bookmark is made.
                let scoped = url.startAccessingSecurityScopedResource()
                defer { if scoped { url.stopAccessingSecurityScopedResource() } }
                return try url.bookmarkData(options: [], includingResourceValuesForKeys: nil, relativeTo: nil)
            },
            resolve: { data in
                var stale = false
                let url = try URL(resolvingBookmarkData: data, options: [.withoutUI], relativeTo: nil, bookmarkDataIsStale: &stale)
                return (url, stale)
            })
    }

    /// A stored grant that covers a folder.
    public struct Grant: Equatable, Sendable {
        /// The granted folder, resolved from its bookmark: the URL whose
        /// security scope is entered around a write.
        public var scope: URL
        /// The folder asked about, under `scope` (the same path unless the
        /// granted folder moved).
        public var folder: URL
        /// The bookmark was stale and has been made again.
        public var refreshed: Bool
    }

    public static let defaultsKey = "flashtexpad.folderBookmarks"

    let defaults: UserDefaults
    let key: String
    let codec: Codec

    public init(defaults: UserDefaults = .standard, key: String = PadFolderBookmarks.defaultsKey, codec: Codec = .system) {
        self.defaults = defaults
        self.key = key
        self.codec = codec
    }

    /// The key a folder is stored under: its standardized path, symlinks
    /// resolved (`/private/var` and `/var` agree), no trailing slash. A
    /// folder that does not exist yet is resolved through its nearest
    /// existing ancestor, so it keys like the same folder once created.
    public static func path(of folder: URL, fileManager: FileManager = .default) -> String {
        var probe = folder.standardizedFileURL
        var rest: [String] = []
        while !fileManager.fileExists(atPath: probe.path), probe.pathComponents.count > 1 {
            rest.insert(probe.lastPathComponent, at: 0)
            probe = probe.deletingLastPathComponent()
        }
        var p = probe.resolvingSymlinksInPath().path
        if p.count > 1, p.hasSuffix("/") { p.removeLast() }
        for component in rest { p += (p == "/" ? "" : "/") + component }
        return p
    }

    /// Whether `folder` is `ancestor` or lies below it (by path).
    public static func path(_ folder: String, isInside ancestor: String) -> Bool {
        folder == ancestor || folder.hasPrefix(ancestor == "/" ? "/" : ancestor + "/")
    }

    private var stored: [String: Data] {
        get { (defaults.dictionary(forKey: key) as? [String: Data]) ?? [:] }
        nonmutating set { defaults.set(newValue, forKey: key) }
    }

    /// The granted folders' paths, sorted.
    public var grantedPaths: [String] { stored.keys.sorted() }

    /// Stores a bookmark for `folder` (a URL from the folder picker),
    /// replacing any earlier one for the same path. Grants below it are
    /// pruned (it covers them). A folder a stored grant above it still
    /// covers is not stored again; a grant above it whose folder is gone
    /// for good is dropped first.
    public func grant(_ folder: URL) throws {
        let data = try codec.make(folder)
        let key = Self.path(of: folder)
        var all = stored
        for ancestor in all.keys where ancestor != key && Self.path(key, isInside: ancestor) {
            do {
                let (url, _) = try codec.resolve(all[ancestor]!)
                if Self.path(of: url) == ancestor {
                    stored = Self.pruned(all, under: ancestor)
                    return
                }
            } catch where Self.isPermanent(error) {
                all[ancestor] = nil
            } catch {}
        }
        all[key] = data
        stored = Self.pruned(all, under: key)
    }

    /// `all` without the grants strictly below `key` (it covers them).
    static func pruned(_ all: [String: Data], under key: String) -> [String: Data] {
        all.filter { $0.key == key || !path($0.key, isInside: key) }
    }

    /// Whether a bookmark failed for good — its folder no longer exists or
    /// the data is corrupt — rather than for now (a file provider offline,
    /// a volume unplugged), which keeps the bookmark for a later try.
    public static func isPermanent(_ error: Error) -> Bool {
        let ns = error as NSError
        if ns.domain == NSCocoaErrorDomain,
           [NSFileNoSuchFileError, NSFileReadNoSuchFileError, NSFileReadCorruptFileError].contains(ns.code) { return true }
        if ns.domain == NSPOSIXErrorDomain, ns.code == Int(ENOENT) { return true }
        if let underlying = ns.userInfo[NSUnderlyingErrorKey] as? Error { return isPermanent(underlying) }
        return false
    }

    public func revoke(_ folder: URL) {
        stored[Self.path(of: folder)] = nil
    }

    /// The deepest stored grant covering `folder` (the folder itself or an
    /// ancestor), resolved. A stale bookmark is made again from the URL it
    /// resolved to and re-keyed if the folder moved. One whose folder is
    /// gone for good (`isPermanent`) is dropped; one failing for now is
    /// kept. Either way the next covering grant is tried.
    public func grant(covering folder: URL) -> Grant? {
        let target = Self.path(of: folder)
        let candidates = stored.filter { Self.path(target, isInside: $0.key) }.sorted { $0.key.count > $1.key.count }
        for (path, data) in candidates {
            let url: URL, stale: Bool
            do { (url, stale) = try codec.resolve(data) } catch {
                if Self.isPermanent(error) { stored[path] = nil }
                continue
            }
            let suffix = String(target.dropFirst(path.count)).trimmingCharacters(in: CharacterSet(charactersIn: "/"))
            let resolvedFolder = suffix.isEmpty ? url : url.appendingPathComponent(suffix, isDirectory: true)
            if stale {
                // Made again from the resolved URL (its scope entered by the codec).
                if let fresh = try? codec.make(url) {
                    var all = stored
                    all[path] = nil
                    let moved = Self.path(of: url)
                    all[moved] = fresh
                    stored = Self.pruned(all, under: moved)
                }
            }
            return Grant(scope: url, folder: resolvedFolder, refreshed: stale)
        }
        return nil
    }
}

/// A project folder opened whole ("Open folder…").
public enum PadProjectFolder {
    /// The `.tex` file to open in `folder` (top level only): `main.tex`;
    /// else the first, by name, that has `\documentclass` in its first 64 KB;
    /// else the first `.tex` by name. Nil when there is none.
    public static func mainDocument(in folder: URL, fileManager: FileManager = .default) -> URL? {
        let entries = (try? fileManager.contentsOfDirectory(at: folder, includingPropertiesForKeys: [.isRegularFileKey],
                                                              options: [.skipsHiddenFiles])) ?? []
        let tex = entries
            .filter { $0.pathExtension.lowercased() == "tex" && (try? $0.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true }
            .sorted { $0.lastPathComponent.localizedStandardCompare($1.lastPathComponent) == .orderedAscending }
        if let main = tex.first(where: { $0.lastPathComponent.lowercased() == "main.tex" }) { return main }
        if let root = tex.first(where: { hasDocumentClass($0) }) { return root }
        return tex.first
    }

    static func hasDocumentClass(_ url: URL) -> Bool {
        guard let handle = try? FileHandle(forReadingFrom: url) else { return false }
        defer { try? handle.close() }
        guard let head = try? handle.read(upToCount: 64 * 1024) else { return false }
        return String(decoding: head, as: UTF8.self).contains("\\documentclass")
    }
}
