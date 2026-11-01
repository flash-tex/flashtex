import AppKit
import SwiftUI

// One engine (the old-engine retirement plan's stage S6, #1236; DESIGN.md §12
// P5): every document is typeset by the pdfLaTeX-compatible engine
// (`flashtex-host`, EngineV3Host.swift), or by its Unicode mode
// (`flashtex-host-unicode`, EngineV3Mode.swift). The previous engine and its
// fallback rules are gone. What is left here:
//   - the facts about the TeX distribution the engine prepares its format
//     from (`EngineChoice.texLiveAvailable`, the host's `prepared` report);
//   - when there is none, a plain, non-blocking message with a Download
//     button that retries the TeX bundle's download (`TeXMissingBanner`;
//     owner ruling RQ20, DESIGN.md §13 2026-10-21: no other engine);
//   - documents recorded as typeset by the previous engine before S6: they
//     open on the new engine with a one-time, dismissable banner that says
//     so and why (`PreviousEngineNoticeBanner`; ruling 4, plan §5.3: never
//     silently). The record then says `new`.
//
// Records are app-local (one UserDefaults dictionary keyed by the entry
// document's path), never written into the project.

/// The TeX distribution the engine needs, and what the host reports about it.
enum EngineChoice {
    /// Whether the new engine has a TeX distribution to prepare its format
    /// from: a configured bundle (`FLASHTEX_RESOLVER=bundle`,
    /// `FLASHTEX_BUNDLE`, `FLASHTEX_BUNDLE_DIGEST` or a
    /// `flashtex-bundle.lock`, EngineV3Bundle.swift; DESIGN.md §4.4), else an
    /// installed TeX Live (`texLiveInstalled`). The host's own report is
    /// checked again when it starts. Whether a bundle may be downloaded yet
    /// is asked separately (EngineV3Bundle.gate).
    static func texLiveAvailable(environment env: [String: String] = ProcessInfo.processInfo.environment) -> Bool {
        if env["FLASHTEX_RESOLVER"] == "bundle" || !(env["FLASHTEX_BUNDLE"] ?? "").isEmpty { return true }
        if texLiveInstalled(environment: env) { return true }
        return EngineV3Bundle.configured(environment: env) != nil
    }

    /// A `kpsewhich` in the directories `crates/flashtex-engine/src/resolver.rs`
    /// `texlive_candidates()` searches, in its order (`FLASHTEX_TEXLIVE_BIN`
    /// alone when set).
    static func texLiveInstalled(environment env: [String: String] = ProcessInfo.processInfo.environment) -> Bool {
        let fm = FileManager.default
        return texLiveCandidates(environment: env).contains { dir in
            var isDir: ObjCBool = false
            return fm.fileExists(atPath: (dir as NSString).appendingPathComponent("kpsewhich"), isDirectory: &isDir) && !isDir.boolValue
        }
    }

    /// `texlive_candidates()` of the engine's resolver, as paths.
    static func texLiveCandidates(environment env: [String: String]) -> [String] {
        if let d = env["FLASHTEX_TEXLIVE_BIN"] { return [d] }
        let fm = FileManager.default
        var c = (env["PATH"] ?? "").split(separator: ":").map(String.init).filter { $0.hasPrefix("/") }
        var files = ["/etc/paths"]
        files += ((try? fm.contentsOfDirectory(atPath: "/etc/paths.d")) ?? []).sorted().map { "/etc/paths.d/" + $0 }
        for f in files {
            for line in ((try? String(contentsOfFile: f, encoding: .utf8)) ?? "").split(separator: "\n") {
                let l = line.trimmingCharacters(in: .whitespaces)
                if l.hasPrefix("/") { c.append(l) }
            }
        }
        c.append("/Library/TeX/texbin")
        var roots = ["/usr/local/texlive"]
        if let h = env["HOME"] { roots.append((h as NSString).appendingPathComponent("texlive")) }
        roots.append("/opt/texlive")
        for root in roots {
            let years = ((try? fm.contentsOfDirectory(atPath: root)) ?? [])
                .filter { $0.count == 4 && $0.allSatisfy(\.isASCII) && $0.allSatisfy(\.isNumber) }
                .sorted().reversed()
            for y in years {
                let bin = "\(root)/\(y)/bin"
                for arch in ((try? fm.contentsOfDirectory(atPath: bin)) ?? []).sorted() { c.append("\(bin)/\(arch)") }
            }
        }
        c += ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"]
        return c
    }

    /// Whether the host's start-up summary (`prepared`) says it found no
    /// TeX Live and could not prepare a format from anything else.
    static func hostLacksTeXLive(_ prepared: DL3JSONView) -> Bool {
        prepared.texlive == nil && prepared.formatFailed
    }

    /// The manifest's `[fonts]` roles that name a family (`text`, `math`,
    /// `mono`, `sans`, in that order). Unicode mode applies them; Classic
    /// mode, like pdfLaTeX, does not (EngineV3Mode.swift).
    static func fontRoles(manifest m: ProjectFilesV1.Manifest.Body) -> [String] {
        let f = m.fonts
        return [("text", f.text), ("math", f.math), ("mono", f.mono), ("sans", f.sans)]
            .compactMap { role, name in (name ?? "").trimmingCharacters(in: .whitespaces).isEmpty ? nil : role }
    }
}

/// The parts of the host's `prepared` summary the rule reads (kept apart
/// from `DL3JSON` so it is testable with plain values).
struct DL3JSONView: Equatable, Sendable {
    var texlive: String?
    var formatFailed: Bool
}

/// Why the engine has nothing to compile with (owner ruling RQ20): it is
/// said, never replaced by another engine.
enum TeXMissing: Equatable, Sendable {
    /// No TeX Live was found and no TeX bundle is configured, or the host
    /// reported that it found neither.
    case noTeX
    /// No TeX Live, and the user chose not to download the TeX bundle
    /// (EngineV3BundleSheet).
    case declined

    /// The message's first line.
    static let headline = "Compiling needs the TeX bundle or a TeX Live installation."

    /// The rest of the message.
    var detail: String {
        switch self {
        case .noTeX:
            "No TeX Live installation was found. Install MacTeX or TeX Live, or download the TeX files FlashTeX can use instead."
        case .declined:
            "No TeX Live is installed, and the TeX files were not downloaded. Download them to compile, or install MacTeX or TeX Live."
        }
    }
}

// MARK: - storage

/// Per-document records: one UserDefaults dictionary,
/// `FlashTeX.EngineV3.documents`, in `EngineV3.defaults` (a test process's
/// own suite under XCTest), keyed by the project root plus the entry
/// (`<root>::<entry>`, #1236 §5.2):
/// `{engine: new|previous, source: user|record, origin?, set_at, used_at, app_version, dir_id?, entry}`.
/// `previous` is a record written before the previous engine retired (S6):
/// such a document gets the one-time notice once, then its record is `new`.
/// `dir_id` (`<volume UUID>:<inode>` of the root folder, only on a volume
/// with persistent file IDs) finds an entry again after the project folder
/// was moved or renamed on that volume. At most `maxEntries`: automatic
/// records go before deliberate choices, the least recently used first.
@MainActor
enum EngineChoiceStore {
    static let documentsKey = "FlashTeX.EngineV3.documents"
    static let maxEntries = 500
    /// The stored `engine` of the retired previous engine.
    static let retiredEngine = "previous"
    /// The stored `engine` of the one engine.
    static let currentEngine = "new"

    /// Defaults the app no longer reads (the retirement plan's §5.5): the
    /// old global switch and its migration flag, the app setting for
    /// documents without a choice, and the v1 preview's debug status. The
    /// first launch of this build removes them (`removeRetiredDefaults`).
    static let retiredDefaultsKeys = [
        "FlashTeX.EngineV3.enabled",
        "FlashTeX.EngineV3.legacyMigrated.v1",
        "FlashTeX.EngineV3.defaultEngine",
        "FlashTeX.Preview.v1.debugStatus",
    ]
    static let retiredDefaultsRemovedKey = "FlashTeX.EngineV3.retiredDefaultsRemoved.v1"

    struct Entry: Equatable, Sendable {
        enum Source: String, Sendable { case user, record }
        /// `new`, or `previous` for a record written before S6.
        var engine: String
        var source: Source
        var setAt = Date()
        var appVersion = EngineChoiceStore.appVersion

        var isRetiredEngine: Bool { engine == EngineChoiceStore.retiredEngine }

        static func == (a: Entry, b: Entry) -> Bool { a.engine == b.engine && a.source == b.source }
    }

    nonisolated static var appVersion: String { Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "dev" }

    /// The entry document's project root (its folder, resolved) and its name there.
    static func parts(_ url: URL) -> (root: URL, entry: String) {
        let u = url.standardizedFileURL.resolvingSymlinksInPath()
        return (u.deletingLastPathComponent(), u.lastPathComponent)
    }

    static func key(_ url: URL) -> String { let p = parts(url); return p.root.path + "::" + p.entry }

    // MARK: folder identity

    /// What identifies a folder across a move: its volume and inode. An
    /// inode means something only on a volume with persistent file IDs
    /// (not FAT, exFAT, SMB or NFS, where inodes are reused), and only on
    /// the same volume (by UUID: a device number is reused at each mount).
    struct FolderIdentity: Equatable, Sendable {
        var volumeUUID: String?
        var persistentIDs: Bool
        var inode: UInt64

        /// The stored `dir_id`, nil when this folder cannot be followed.
        var id: String? {
            guard persistentIDs, let volumeUUID, !volumeUUID.isEmpty else { return nil }
            return "\(volumeUUID):\(inode)"
        }
    }

    /// The identity of an existing folder, nil when there is none at that
    /// path. Tests replace it (fake volumes); the app reads the file system.
    static var identityProvider: (String) -> FolderIdentity? = liveIdentity

    static func liveIdentity(_ path: String) -> FolderIdentity? {
        var st = stat()
        guard stat(path, &st) == 0 else { return nil }
        let v = try? URL(fileURLWithPath: path).resourceValues(forKeys: [.volumeUUIDStringKey, .volumeSupportsPersistentIDsKey])
        return FolderIdentity(volumeUUID: v?.volumeUUIDString, persistentIDs: v?.volumeSupportsPersistentIDs ?? false, inode: UInt64(st.st_ino))
    }

    /// The volume UUID of the nearest existing folder at or above `path`.
    static func mountedVolumeUUID(above path: String) -> String? {
        var p = path
        while true {
            if let i = identityProvider(p) { return i.volumeUUID }
            let parent = (p as NSString).deletingLastPathComponent
            if parent == p || parent.isEmpty { return nil }
            p = parent
        }
    }

    /// Whether the entry stored for `oldRoot` with `dirID` is this folder,
    /// moved or renamed: the same volume (by UUID) with persistent IDs and
    /// the same inode, the old root gone, and the old root's volume mounted
    /// (its nearest existing ancestor is on that volume), so "gone" is not
    /// an unmounted disk or a restore onto another one.
    static func isMove(from oldRoot: String, dirID: String, to current: FolderIdentity) -> Bool {
        guard let id = current.id, id == dirID, identityProvider(oldRoot) == nil else { return false }
        return mountedVolumeUUID(above: oldRoot) == current.volumeUUID
    }

    // MARK: entries

    private static var all: [String: Any] {
        get { EngineV3.defaults.dictionary(forKey: documentsKey) ?? [:] }
        set { EngineV3.defaults.set(newValue, forKey: documentsKey) }
    }

    private static func decode(_ v: Any?) -> Entry? {
        guard let d = v as? [String: Any], let engine = d["engine"] as? String,
              engine == currentEngine || engine == retiredEngine,
              let source = (d["source"] as? String).flatMap(Entry.Source.init(rawValue:)) else { return nil }
        return Entry(engine: engine, source: source, setAt: d["set_at"] as? Date ?? .distantPast,
                     appVersion: d["app_version"] as? String ?? "")
    }

    /// The entry for a document: by its key; else under #1421's key (the
    /// full path); else, when its project folder was moved or renamed on
    /// the same volume (`isMove`), by the folder's identity and the entry's
    /// name. A found entry moves to the current key. Anything less certain
    /// is a new folder: no entry, and the old one is left alone.
    static func entry(for url: URL) -> Entry? {
        let k = key(url)
        var dict = all
        if let e = decode(dict[k]) { return e }
        let (root, name) = parts(url)
        let identity = identityProvider(root.path)
        var found: String?
        let pathKey = root.appendingPathComponent(name).path
        if decode(dict[pathKey]) != nil {
            found = pathKey
        } else if let identity, identity.id != nil {
            found = dict.keys.sorted().first { key in
                guard let d = dict[key] as? [String: Any], let dirID = d["dir_id"] as? String, d["entry"] as? String == name,
                      let oldRoot = key.components(separatedBy: "::").first, oldRoot != root.path else { return false }
                return isMove(from: oldRoot, dirID: dirID, to: identity)
            }
        }
        guard let found, var d = dict[found] as? [String: Any] else { return nil }
        d["dir_id"] = identity?.id
        d["entry"] = name
        dict[found] = nil
        dict[k] = d
        all = dict
        return decode(d)
    }

    static func set(_ entry: Entry?, for url: URL) {
        var dict = all
        let k = key(url)
        if let entry {
            let (root, name) = parts(url)
            var d: [String: Any] = ["engine": entry.engine, "source": entry.source.rawValue,
                                    "set_at": entry.setAt, "used_at": Date(), "app_version": entry.appVersion, "entry": name]
            if let id = identityProvider(root.path)?.id { d["dir_id"] = id }
            dict[k] = d
        } else {
            dict.removeValue(forKey: k)
        }
        all = prune(dict)
    }

    /// Notes that a document's entry was used (the least recently used go first).
    static func touch(_ url: URL) {
        var dict = all
        guard var d = dict[key(url)] as? [String: Any] else { return }
        d["used_at"] = Date()
        dict[key(url)] = d
        all = dict
    }

    /// At most `maxEntries` entries. Evicted first: automatic records
    /// before deliberate choices, then the least recently used, then by key
    /// (a total order, so equal dates prune the same way every time).
    static func prune(_ dict: [String: Any]) -> [String: Any] {
        guard dict.count > maxEntries else { return dict }
        func rank(_ key: String) -> (Int, Date, String) {
            let d = dict[key] as? [String: Any]
            let user = d?["source"] as? String == Entry.Source.user.rawValue ? 1 : 0
            return (user, (d?["used_at"] as? Date) ?? (d?["set_at"] as? Date) ?? .distantPast, key)
        }
        let order = dict.keys.sorted { a, b in
            let x = rank(a), y = rank(b)
            if x.0 != y.0 { return x.0 < y.0 }
            if x.1 != y.1 { return x.1 < y.1 }
            return x.2 < y.2
        }
        var out = dict
        for key in order.prefix(dict.count - maxEntries) { out[key] = nil }
        return out
    }

    /// Once per defaults domain: removes `retiredDefaultsKeys`, which this
    /// build never reads (the retirement plan's §5.5).
    static func removeRetiredDefaults(in d: UserDefaults) {
        guard !d.bool(forKey: retiredDefaultsRemovedKey) else { return }
        for k in retiredDefaultsKeys { d.removeObject(forKey: k) }
        d.set(true, forKey: retiredDefaultsRemovedKey)
    }
}

// MARK: - the window

extension ShellModel {
    /// The document that just opened (File > Open, a folder, a reload), after
    /// its manifest was read: a record of the previous engine (a document last
    /// typeset before S6) shows the one-time notice and becomes `new`; any
    /// other document gets a record of the engine, once. Logs the `engine:`
    /// line the attribution tools read.
    func resolveEngineForOpenedDocument() {
        engineChoiceDocument = documentURL
        previousEngineNotice = false
        if let url = documentURL {
            let entry = EngineChoiceStore.entry(for: url)
            if entry?.isRetiredEngine == true {
                previousEngineNotice = true
                EngineChoiceStore.set(.init(engine: EngineChoiceStore.currentEngine, source: .record), for: url)
                announceEngine(PreviousEngineNoticeBanner.headline)
            } else if entry == nil {
                EngineChoiceStore.set(.init(engine: EngineChoiceStore.currentEngine, source: .record), for: url)
            } else {
                EngineChoiceStore.touch(url)
            }
        }
        engineLog("engine: new (\(documentMode.mode.rawValue))" + (documentURL.map { " " + $0.lastPathComponent } ?? ""))
    }

    /// The open document was saved under a new name (Save As) or for the
    /// first time: the new file's manifest is read and it gets its record.
    /// The caller set `engineChoicePending` before `documentURL` changed
    /// (no open of the new path before its manifest is read); cleared here.
    func engineDocumentSaved(from old: URL?) {
        defer { engineChoicePending = false }
        guard let url = documentURL, old?.standardizedFileURL != url.standardizedFileURL else { return }
        manifest.refresh() // engineChoicePending: manifestDidRefresh waits for the resolution below
        let wasNoticed = previousEngineNotice
        resolveEngineForOpenedDocument()
        if wasNoticed { previousEngineNotice = true } // the notice a Save As found up stays up until dismissed
    }

    /// The manifest was read again (an outside edit, the Fonts sheet): the
    /// mode may now differ (`[fonts]` routes a document to Unicode mode,
    /// UnicodeFonts.swift), and the engine's `texinputs` links follow.
    func manifestDidRefresh() {
        // The open path resolves after its own read.
        guard !engineChoicePending, engineChoiceDocument == documentURL else { return }
        if engineV3Enabled { engineV3.manifestChanged(model: self) }
    }

    /// The host reported no TeX Live (and no format): the window says so
    /// (RQ20), with no other engine.
    func engineV3HostLacksTeXLive() {
        texMissing = EngineV3Bundle.declinedSomeBundle ? .declined : .noTeX
        texMissingDismissed = false
    }

    /// The user chose not to download the TeX files (EngineV3BundleSheet):
    /// the window says that compiling needs them, with a Download button.
    func engineV3BundleDeclined() {
        texMissing = .declined
        texMissingDismissed = false
    }

    /// The message's Download button: the TeX bundle's download is allowed
    /// and the engine starts again (it downloads as it starts).
    func retryTeXDownload() {
        texMissing = nil
        texMissingDismissed = false
        engineV3.answerBundleConsent(true)
    }

    /// Whether the message can offer a download: a TeX bundle is configured.
    var texDownloadAvailable: Bool { EngineV3Bundle.configured() != nil }
}

// MARK: - views

/// Over the preview when there is no TeX to compile with (RQ20): a plain,
/// non-blocking message. Download retries the TeX bundle's download.
struct TeXMissingBanner: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        if let missing = model.texMissing, !model.texMissingDismissed {
            HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(DS.Colors.severityWarning)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: DS.Space.xs) {
                    Text(TeXMissing.headline).font(DS.Fonts.secondary.weight(.semibold))
                    Text(missing.detail).font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: 0)
                if model.texDownloadAvailable {
                    Button("Download") { model.retryTeXDownload() }
                        .ideDefault()
                        .accessibilityHint("Downloads the TeX files the engine needs, then compiles.")
                        .accessibilityIdentifier("tex.missing.download")
                }
                Button("Dismiss") { model.texMissingDismissed = true }
                    .ideSecondary()
            }
            .padding(DS.Space.m)
            .background(DS.Colors.surfaceSecondary)
            .accessibilityElement(children: .contain)
            .accessibilityLabel(TeXMissing.headline + " " + missing.detail)
            .accessibilityIdentifier("tex.missing")
        }
    }
}

/// Over the preview, once, for a document last typeset by the previous
/// engine (ruling 4, the retirement plan's §5.3): the new engine typesets it
/// now, and why. Dismissed for good: its record now says `new`.
struct PreviousEngineNoticeBanner: View {
    @Environment(ShellModel.self) var model

    static let headline = "This document is now typeset with the new engine."
    static let detail = "It was last typeset with FlashTeX's previous engine, which has been retired. The new engine is compatible with pdfLaTeX, so the pages may differ slightly from before."

    var body: some View {
        if model.previousEngineNotice {
            HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                Image(systemName: "info.circle.fill")
                    .foregroundStyle(DS.Colors.textSecondary)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: DS.Space.xs) {
                    Text(Self.headline).font(DS.Fonts.secondary.weight(.semibold))
                    Text(Self.detail).font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: 0)
                Button("Dismiss") { model.previousEngineNotice = false }
                    .ideSecondary()
                    .accessibilityIdentifier("engine.previous.dismiss")
            }
            .padding(DS.Space.m)
            .background(DS.Colors.surfaceSecondary)
            .accessibilityElement(children: .contain)
            .accessibilityLabel(Self.headline + " " + Self.detail)
            .accessibilityIdentifier("engine.previous")
        }
    }
}

extension ProjectManifest {
    /// The `[project] texinputs` files the new engine finds by name: each
    /// is linked at the project copy's top level (EngineV3Mirror.linkTexInputs),
    /// as `TEXINPUTS=.:dir1:dir2:` would find it, in manifest order; a file
    /// next to the entry of the same name wins. A file under the root links
    /// to its place in the copy (which is the editor's text when it is
    /// open); one outside the root (`texinputs/<i>/…`) to the real file.
    var texInputLinks: [EngineV3Mirror.TexInputLink] {
        guard let snapshot = currentSnapshot else { return [] }
        var seen = Set<String>()
        var out: [EngineV3Mirror.TexInputLink] = []
        for f in snapshot.files where f.texinput != nil {
            let name = (f.path as NSString).lastPathComponent
            guard !name.isEmpty, seen.insert(name).inserted else { continue }
            out.append(.init(name: name, inCopy: f.origin == nil ? f.path : nil, external: f.origin))
        }
        return out
    }
}
