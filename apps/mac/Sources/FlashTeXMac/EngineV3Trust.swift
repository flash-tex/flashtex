import Darwin
import Foundation

/// Project trust (owner decision 9A). A project that came from elsewhere
/// (its main file or its folder carries the `com.apple.quarantine` extended
/// attribute: downloaded, unpacked from a downloaded archive, received) and
/// that the user has not trusted compiles with shell escape OFF (`\write18`
/// does nothing) and the preview shows "Trust this project?". Once trusted,
/// it compiles with restricted `\write18`, as pdflatex does by default
/// (texmf.cnf's `shell_escape = p`). A project made on this Mac (no
/// quarantine) is trusted.
///
/// **Fails closed.** A record names exactly what the user trusted, bound to
/// what it was when they trusted it:
/// - The quarantined item itself. A downloaded `.tex` whose folder is not
///   quarantined (`~/Downloads/paper.tex`) records that file, never its
///   folder: another download next to it asks again. A quarantined folder
///   (an unpacked archive) records the folder; a quarantined main file in it
///   from a different download (another quarantine event) asks again.
/// - Every other quarantined file in a project folder (any of them can be
///   `\input`) that did not come with the folder's own download, such as a
///   `.sty` downloaded later into a trusted folder: each is recorded when
///   the user trusts the prompt that counted it, and a later one asks
///   again. The walk is the project copy's (EngineV3Mirror.sync: 20,000
///   entries, hidden files skipped); files past that bound are not checked.
/// - A lone file in a shared folder (`isShared`: Downloads, Desktop, ...)
///   is not a project: the folder is never walked for trust. Trust covers
///   the file and the files its own download brought (the same quarantine
///   event); a quarantined file from another download counts only if the
///   document reads it (`referencedFiles`, found lexically at each
///   decision). Every other download there never asks. A name built by a
///   macro is not seen: the worst case is restricted shell escape.
/// - The decision runs off the main thread (EngineV3Session.startWalk);
///   until it is made, compiles send shell escape off.
/// - Its canonical path, its inode and volume, and its quarantine event (the
///   xattr's UUID, else its time and agent). Moving or renaming it, or a new
///   download unpacked to the same path (a new inode, a new event), asks again.
/// - The button records the identities the prompt was computed from, never
///   a fresh look: if an item changed in between, the project is still not
///   trusted and the prompt is shown again for what is there now.
/// - A project with no quarantine attribute anywhere (a git clone, `curl`,
///   `unzip` in a shell) is trusted, by design.
///
/// The records live in the app's defaults; an instance with its own cache
/// (`FLASHTEX_V3_CACHE`: tests, benches) keeps them in a file beside that
/// cache instead, and a test process without one in a temporary file, so
/// neither reads nor writes the owner's records.
enum EngineV3Trust {
    static let recordKey = "FlashTeX.EngineV3.trustRecords.v2"
    static let quarantineAttribute = "com.apple.quarantine"

    /// The shell-escape mode a COMPILE for this project sends.
    static func shellEscape(trusted: Bool) -> String { trusted ? "restricted" : "off" }

    // MARK: identity

    /// One trusted item, as it was when trusted.
    struct Identity: Codable, Equatable {
        var path: String
        var inode: UInt64
        var volume: String
        /// The quarantine event (nil: the item is not quarantined).
        var quarantine: String?
    }

    static func canonical(_ url: URL) -> URL { url.standardizedFileURL.resolvingSymlinksInPath() }

    /// The raw `com.apple.quarantine` value, nil when there is none.
    static func quarantineValue(_ url: URL) -> String? {
        let path = url.path
        let n = getxattr(path, quarantineAttribute, nil, 0, 0, 0)
        guard n >= 0 else { return nil }
        var buf = [UInt8](repeating: 0, count: n)
        let m = n == 0 ? 0 : getxattr(path, quarantineAttribute, &buf, n, 0, 0)
        guard m >= 0 else { return "" }
        return String(decoding: buf.prefix(m), as: UTF8.self)
    }

    static func isQuarantined(_ url: URL) -> Bool { quarantineValue(url) != nil }

    /// The download a quarantine value records: `flags;time;agent;UUID` → the
    /// UUID; without one, time and agent (never the flags, which Gatekeeper
    /// updates after the first open).
    static func quarantineEvent(_ value: String) -> String {
        let f = value.split(separator: ";", omittingEmptySubsequences: false).map(String.init)
        if f.count >= 4, !f[3].isEmpty { return f[3] }
        return f.dropFirst().joined(separator: ";")
    }

    /// The item's identity now; nil when it cannot be read (fails closed).
    static func identity(_ url: URL) -> Identity? {
        let c = canonical(url)
        var st = stat()
        guard stat(c.path, &st) == 0 else { return nil }
        let volume = (try? c.resourceValues(forKeys: [.volumeUUIDStringKey]))?.volumeUUIDString ?? "dev:\(st.st_dev)"
        return Identity(path: c.path, inode: UInt64(st.st_ino), volume: volume, quarantine: quarantineValue(c).map(quarantineEvent))
    }

    /// Folders that are never recorded as a whole, whatever their attributes.
    static let sharedFolders: Set<String> = {
        let fm = FileManager.default
        var out: Set<String> = [canonical(fm.homeDirectoryForCurrentUser).path, "/", canonical(fm.temporaryDirectory).path]
        for d in [FileManager.SearchPathDirectory.downloadsDirectory, .desktopDirectory, .documentDirectory] {
            for u in fm.urls(for: d, in: .userDomainMask) { out.insert(canonical(u).path) }
        }
        return out
    }()
    /// Tests: more folders treated as shared (a stand-in for ~/Downloads).
    nonisolated(unsafe) static var testSharedFolders: Set<String> = []

    /// A shared folder (Downloads, Desktop, ...): never a project, never
    /// walked for trust; only the main file's own download counts.
    static func isShared(_ root: URL) -> Bool {
        let p = canonical(root).path
        return sharedFolders.contains(p) || testSharedFolders.contains(p)
    }

    // MARK: store

    enum Store {
        case defaults(UserDefaults)
        case file(URL)

        /// This instance's store: beside `FLASHTEX_V3_CACHE` when set, a
        /// temporary file in a test process, else the app's defaults.
        static var current: Store {
            let env = ProcessInfo.processInfo.environment
            if let cache = env["FLASHTEX_V3_CACHE"], !cache.isEmpty {
                let dir = URL(fileURLWithPath: (cache as NSString).expandingTildeInPath, isDirectory: true).standardizedFileURL
                return .file(dir.deletingLastPathComponent().appendingPathComponent(dir.lastPathComponent + "-trust.json"))
            }
            if NSClassFromString("XCTestCase") != nil {
                return .file(FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-engine-v3-trust-\(getpid()).json"))
            }
            return .defaults(.standard)
        }

        func load() -> [Identity] {
            let data: Data?
            switch self {
            case .defaults(let d): data = d.data(forKey: EngineV3Trust.recordKey)
            case .file(let url): data = try? Data(contentsOf: url)
            }
            return data.flatMap { try? JSONDecoder().decode([Identity].self, from: $0) } ?? []
        }

        func save(_ records: [Identity]) {
            guard let data = try? JSONEncoder().encode(records) else { return }
            switch self {
            case .defaults(let d): d.set(data, forKey: EngineV3Trust.recordKey)
            case .file(let url):
                try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
                try? data.write(to: url, options: .atomic)
            }
        }
    }

    // MARK: decisions

    /// What must be recorded for this project to be trusted, as it is now:
    /// - the quarantined folder, unless it is a shared folder such as Downloads;
    /// - the main file when it is quarantined by another download than the
    ///   folder's, or the folder does not count;
    /// - every other quarantined file in the folder (`others`: the project
    ///   walk's, EngineV3Mirror.sync) that did not come with the folder's
    ///   download. TeX can `\input` any of them, whatever its name.
    /// Nil when an item that counts cannot be read (fails closed); empty
    /// when nothing is quarantined.
    static func subjects(root: URL, main: URL?, others: [URL] = []) -> [Identity]? {
        let rootQuarantined = isQuarantined(root), mainQuarantined = main.map(isQuarantined) ?? false
        guard rootQuarantined || mainQuarantined || !others.isEmpty else { return [] }
        guard let r = identity(root) else { return nil }
        let folderCounts = rootQuarantined && !isShared(root)
        // A quarantined shared folder says nothing about which download this
        // is: the main file stands for the project.
        let mainCounts = mainQuarantined || (rootQuarantined && !folderCounts)
        var out: [Identity] = folderCounts ? [r] : []
        // A file that came with the trusted download: the folder's, or, when
        // the folder does not count, the main file's.
        var mainEvent: String?
        func covered(_ m: Identity) -> Bool {
            guard let q = m.quarantine else { return false }
            return folderCounts ? q == r.quarantine : q == mainEvent
        }
        var mainPath: String?
        if mainCounts {
            guard let main, let m = identity(main) else { return nil }
            mainPath = m.path
            if !covered(m) { out.append(m) }
            if !folderCounts { mainEvent = m.quarantine }
        } else if let main {
            mainPath = canonical(main).path
        }
        var seen = Set(out.map(\.path))
        let trustedEvent = folderCounts ? r.quarantine : mainEvent
        for f in others {
            // One getxattr first: a file of the trusted download (the common
            // case, an unpacked archive) needs no identity.
            guard let value = quarantineValue(f) else { continue }
            if let trustedEvent, quarantineEvent(value) == trustedEvent { continue }
            guard let m = identity(f) else { return nil }
            if m.path == mainPath || m.path == r.path || covered(m) || seen.contains(m.path) { continue }
            seen.insert(m.path)
            out.append(m)
        }
        return out
    }

    // MARK: what to check

    /// The trust decision for one project, computed off the main thread.
    struct Decision {
        var trusted: Bool
        /// The other quarantined files that were checked.
        var others: [URL]
        /// What trusting it records: the subjects not yet recorded as they
        /// are now (empty when trusted, or when it cannot be trusted as is).
        var need: [Identity]
    }

    /// Decides a project's trust. In a project folder, every quarantined
    /// file of its walk (`walkQuarantined`) counts; in a shared folder
    /// (`isShared`: a lone file in Downloads), only the files the document
    /// itself reads (`referencedFiles`), so other downloads there never ask.
    /// Reads the disk: call off the main thread.
    static func decide(root: URL, main: String, texts: [String: String], walkQuarantined: [URL], store: Store = .current) -> Decision {
        let others = isShared(root) ? referencedFiles(root: root, main: main, texts: texts).filter(isQuarantined) : walkQuarantined
        guard let need = subjects(root: root, main: root.appendingPathComponent(main), others: others) else {
            return Decision(trusted: false, others: others, need: [])
        }
        let records = store.load()
        let missing = need.filter { !records.contains($0) }
        return Decision(trusted: missing.isEmpty, others: others, need: missing)
    }

    /// Files under `root` the document reads as TeX, found lexically from
    /// `main`: `\input`/`\include` (ProjectIncludes, transitively, bounded),
    /// local `\usepackage`/`\RequirePackage`/`\documentclass`/`\LoadClass`
    /// (`.sty`/`.cls`), and the main file's own auxiliary files (`.aux`,
    /// `.bbl`, `.toc`, ...). `texts`: the editor's documents (path → text);
    /// other files are read from disk. A name built by a macro is not seen.
    static func referencedFiles(root: URL, main: String, texts: [String: String]) -> [URL] {
        let fm = FileManager.default
        func exists(_ rel: String) -> Bool {
            var dir: ObjCBool = false
            return fm.fileExists(atPath: root.appendingPathComponent(rel).path, isDirectory: &dir) && !dir.boolValue
        }
        var found: [String] = []
        var seen: Set<String> = [main]
        var queue = [main]
        let stem = (main as NSString).deletingPathExtension
        for ext in ["aux", "bbl", "toc", "lof", "lot", "ind", "gls", "nls", "out", "nav", "snm", "vrb"] where exists(stem + "." + ext) {
            found.append(stem + "." + ext)
        }
        let packages = try? NSRegularExpression(pattern: #"\\(usepackage|RequirePackage|documentclass|LoadClass)\s*(?:\[[^\]]*\])?\s*\{([^}]*)\}"#)
        while let rel = queue.first, found.count < 256 {
            queue.removeFirst()
            let text: String
            if let t = texts[rel] {
                text = t
            } else {
                let url = root.appendingPathComponent(rel)
                guard let size = (try? fm.attributesOfItem(atPath: url.path))?[.size] as? Int, size <= ProjectIncludes.maxDocumentBytes,
                      let data = try? Data(contentsOf: url) else { continue }
                text = String(decoding: data, as: UTF8.self)
            }
            var refs: [String] = []
            for r in ProjectIncludes.scan(text) where r.literal {
                if let c = (try? ProjectIncludes.candidates(for: r.argument))?.first(where: exists) { refs.append(c) }
            }
            if let packages {
                let ns = text as NSString
                for m in packages.matches(in: text, range: NSRange(location: 0, length: ns.length)) {
                    let ext = ns.substring(with: m.range(at: 1)).hasSuffix("Class") || ns.substring(with: m.range(at: 1)) == "documentclass" ? "cls" : "sty"
                    for name in ns.substring(with: m.range(at: 2)).split(separator: ",") {
                        let n = name.trimmingCharacters(in: .whitespacesAndNewlines)
                        if !n.isEmpty, !n.contains("/"), exists(n + "." + ext) { refs.append(n + "." + ext) }
                    }
                }
            }
            for r in refs where !seen.contains(r) {
                seen.insert(r)
                found.append(r)
                queue.append(r)
            }
        }
        return found.map { root.appendingPathComponent($0) }
    }

    /// Whether every subject is recorded as it is now.
    static func covered(_ need: [Identity], store: Store = .current) -> Bool {
        if need.isEmpty { return true }
        let records = store.load()
        return need.allSatisfy(records.contains)
    }

    /// Whether a project (its folder, main file and other quarantined
    /// files) may run restricted `\write18`. An untitled buffer (no folder)
    /// has nothing to run: trusted.
    static func isTrusted(root: URL?, main: URL?, others: [URL] = [], store: Store = .current) -> Bool {
        guard let root else { return true }
        guard let need = subjects(root: root, main: main, others: others) else { return false }
        return covered(need, store: store)
    }

    /// Records exactly these items (what the prompt was shown for).
    static func record(_ identities: [Identity], store: Store = .current) {
        guard !identities.isEmpty else { return }
        var list = store.load()
        // One record per path: a newer item there replaces the older one.
        list.removeAll { old in identities.contains { $0.path == old.path } }
        list.append(contentsOf: identities)
        store.save(list)
    }

    /// Records the project's subjects as they are now (tests).
    static func record(root: URL, main: URL?, others: [URL] = [], store: Store = .current) {
        guard let new = subjects(root: root, main: main, others: others) else { return }
        record(new, store: store)
    }
}
