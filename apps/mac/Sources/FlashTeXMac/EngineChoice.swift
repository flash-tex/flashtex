import AppKit
import SwiftUI

// Which engine typesets a document (lane P5-ENGINE-CHOICE; DESIGN.md §10,
// §12 P5 "a per-document switch the scoreboard flips", decision 3; the
// retirement plan's §5, #1236). The app has two engines while the old one is
// still a fallback: the new pdfLaTeX-compatible engine (`flashtex-host`,
// EngineV3Host.swift) and the previous engine (the worker / preview
// controller). One engine at a time (#1340): `ShellModel.engineV3Enabled`
// is the window's effective engine, and this file decides it.
//
// The engine of the open document, highest first:
//   1. `FLASHTEX_ENGINE_V3=1|0`: forces every document (CI, tests,
//      developers); no fallback rule applies.
//   2. The window's own override: `engineV3Enabled` set directly (benches,
//      tests); no fallback rule applies.
//   3. The user's choice for this document (the status bar's engine menu,
//      View > Engine for This Document).
//   4. The app setting (Settings > Compile > "Engine for other documents",
//      `FlashTeX.EngineV3.defaultEngine`): "New engine" or "Previous
//      engine"; absent means the built-in default.
//   5. The engine this document was last typeset with (a record written
//      when it opens, and not while a fallback rule blocks the new engine),
//      so a change of the built-in default never switches a document
//      someone has already seen typeset (ruling 4, #1236 §5.3).
//   6. The old global switch, `FlashTeX.EngineV3.enabled` = true (the View
//      toggle before this lane), read only for a document with no entry
//      yet, which it turns into that document's own choice (#1236 §5.2).
//      A stored false is no choice: it is removed once (`migrateLegacyFlag`).
//   7. The built-in default, `EngineChoice.defaultForNewDocuments`.
// Then the fallback rules: when the result is the new engine but it cannot
// typeset this project as the previous one would, the previous engine is
// used and the window says why (a banner over the preview, the status bar's
// engine item, a VoiceOver announcement). Never silently.
//
// Choices and records are app-local (one UserDefaults dictionary keyed by
// the entry document's path), never written into the project, so nothing
// reaches collaborators through git.

struct EngineChoice: Equatable, Sendable {
    enum Engine: String, Sendable, CaseIterable {
        /// The pdfLaTeX-compatible engine (`flashtex-host`).
        case new
        /// The old engine (worker, preview controller), until it retires.
        case previous

        var title: String { self == .new ? "New engine" : "Previous engine" }
        var menuTitle: String { self == .new ? "New Engine (pdfLaTeX-compatible)" : "Previous Engine" }
    }

    enum Source: String, Sendable {
        case environment, window, user, appSetting, record, legacySwitch, builtInDefault

        var explanation: String {
            switch self {
            case .environment: "set by FLASHTEX_ENGINE_V3 in the environment"
            case .window: "set for this window"
            case .user: "your choice for this document"
            case .appSetting: "Settings > Compile"
            case .record: "the engine this document was last typeset with"
            case .legacySwitch: "the earlier View > Engine v3 Preview switch, now this document's choice"
            case .builtInDefault: "the default for documents without a choice"
            }
        }
    }

    /// Why the new engine cannot typeset this project (yet).
    enum Blocker: Equatable, Sendable {
        /// No TeX Live: the new engine builds its format from the user's
        /// TeX Live (D12) and, without one, has nothing to load. The host's
        /// own report (`prepared` with no `texlive`) counts too.
        case noTeXLive
        /// `[fonts]` roles in flashtex.toml: pdfLaTeX typesets with TeX
        /// fonts, so it would ignore them.
        /// (`[packages] pin` and `path` no longer block it: the new engine
        /// takes the pinned versions and the libraries ahead of TeX Live,
        /// ProjectPackagesState.prepareForEngineV3.)
        case projectFonts([String])

        /// The banner's and the status item's short reason.
        var short: String {
            switch self {
            case .noTeXLive: "no TeX Live is installed"
            case .projectFonts: "this project sets fonts in flashtex.toml"
            }
        }

        /// The full reason (tooltips, the menu, the announcement).
        var detail: String {
            switch self {
            case .noTeXLive:
                "No TeX Live installation was found. The new engine prepares its pdfLaTeX format from your TeX Live; install MacTeX or TeX Live to use it."
            case .projectFonts(let roles):
                "This project's flashtex.toml sets [fonts] (\(roles.joined(separator: ", "))). The new engine is pdfLaTeX-compatible and typesets with TeX's fonts, so it would ignore them."
            }
        }
    }

    var preferred: Engine
    var source: Source
    var blocker: Blocker?

    /// The engine that typesets: the preferred one unless a fallback rule applies.
    var effective: Engine { blocker == nil ? preferred : .previous }
    /// Forced choices (environment, window override) bypass the fallback rules.
    var isForced: Bool { source == .environment || source == .window }

    /// The status item's text.
    var title: String { effective.title }

    /// What the status item's tooltip and the menu say about the choice.
    var explanation: String {
        if let blocker { return "Previous engine: \(blocker.detail)" }
        return "\(effective.title) (\(source.explanation))."
    }

    // MARK: the built-in default

    /// The default for documents with no choice, no app setting and no
    /// record. **The P5 switch-over (S5) is this one line**: `.new` makes
    /// every document nobody has typeset yet open with the new engine;
    /// documents already typeset keep their engine (`Source.record`) and the
    /// fallback rules still apply. Not to be flipped outside the P5 gate.
    static let defaultForNewDocuments: Engine = .previous

    /// Tests only: stands in for `defaultForNewDocuments`.
    @MainActor static var builtInDefaultForTests: Engine?

    /// The built-in default in effect. Under XCTest it stays `.previous`
    /// whatever the shipping default is, so the old engine's tests do not
    /// switch engines when the default flips; a test that wants the new
    /// engine turns it on itself.
    @MainActor static var builtInDefault: Engine {
        if let builtInDefaultForTests { return builtInDefaultForTests }
        return EngineV3.underTest ? .previous : defaultForNewDocuments
    }

    // MARK: resolving

    /// The choice from its inputs (pure). `blocker` is asked only when the
    /// preferred engine is the new one and the choice is not forced.
    static func resolve(environment: String?, window: Engine?, entry: EngineChoiceStore.Entry?,
                        appSetting: Engine?, legacyAllNew: Bool = false, builtInDefault: Engine,
                        blocker: () -> Blocker?) -> EngineChoice {
        var c: EngineChoice
        switch environment {
        case "1": c = EngineChoice(preferred: .new, source: .environment)
        case "0": c = EngineChoice(preferred: .previous, source: .environment)
        default:
            if let window {
                c = EngineChoice(preferred: window, source: .window)
            } else if let entry, entry.source == .user {
                c = EngineChoice(preferred: entry.engine, source: .user)
            } else if let appSetting {
                c = EngineChoice(preferred: appSetting, source: .appSetting)
            } else if let entry {
                c = EngineChoice(preferred: entry.engine, source: .record)
            } else if legacyAllNew {
                c = EngineChoice(preferred: .new, source: .legacySwitch)
            } else {
                c = EngineChoice(preferred: builtInDefault, source: .builtInDefault)
            }
        }
        if c.preferred == .new, !c.isForced { c.blocker = blocker() }
        return c
    }

    /// A window's choice before any document opens (no record, no manifest):
    /// what `engineV3Enabled` starts as. Under XCTest the app setting is not
    /// read (a test that wants v3 turns it on itself).
    @MainActor static var atLaunch: EngineChoice {
        if !EngineV3.underTest { EngineChoiceStore.migrateLegacyFlag(in: EngineV3.defaults) }
        return resolve(environment: ProcessInfo.processInfo.environment["FLASHTEX_ENGINE_V3"], window: nil, entry: nil,
                       appSetting: EngineV3.underTest ? nil : EngineChoiceStore.appSetting,
                       legacyAllNew: !EngineV3.underTest && EngineChoiceStore.legacyAllNew, builtInDefault: builtInDefault,
                       blocker: { texLiveAvailable() ? nil : .noTeXLive })
    }

    // MARK: fallback facts

    /// Whether the new engine has a TeX distribution to prepare its format
    /// from: a configured bundle (`FLASHTEX_RESOLVER=bundle`,
    /// `FLASHTEX_BUNDLE`; DESIGN.md §4.4), else a `kpsewhich` in the
    /// directories `crates/flashtex-engine/src/resolver.rs`
    /// `texlive_candidates()` searches, in its order (`FLASHTEX_TEXLIVE_BIN`
    /// alone when set). The host's own report is checked again when it starts.
    static func texLiveAvailable(environment env: [String: String] = ProcessInfo.processInfo.environment) -> Bool {
        if env["FLASHTEX_RESOLVER"] == "bundle" || !(env["FLASHTEX_BUNDLE"] ?? "").isEmpty { return true }
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

    /// The manifest's fallback facts (pure): `[fonts]` roles. Pins and
    /// libraries are the new engine's too (ProjectPackages.swift).
    static func blocker(manifest m: ProjectFilesV1.Manifest.Body) -> Blocker? {
        let f = m.fonts
        let roles = [("text", f.text), ("math", f.math), ("mono", f.mono), ("sans", f.sans)]
            .compactMap { role, name in (name ?? "").trimmingCharacters(in: .whitespaces).isEmpty ? nil : role }
        if !roles.isEmpty { return .projectFonts(roles) }
        return nil
    }
}

/// The parts of the host's `prepared` summary the fallback reads (kept
/// apart from `DL3JSON` so the rule is testable with plain values).
struct DL3JSONView: Equatable, Sendable {
    var texlive: String?
    var formatFailed: Bool
}

// MARK: - storage

/// Per-document choices and records: one UserDefaults dictionary,
/// `FlashTeX.EngineV3.documents`, in `EngineV3.defaults` (a test process's
/// own suite under XCTest), keyed by the project root plus the entry
/// (`<root>::<entry>`, #1236 §5.2):
/// `{engine: new|previous, source: user|record, origin?, set_at, used_at, app_version, dir_id?, entry}`.
/// `dir_id` (`<volume UUID>:<inode>` of the root folder, only on a volume
/// with persistent file IDs) finds an entry again after the project folder
/// was moved or renamed on that volume. At most `maxEntries`: automatic
/// records go before deliberate choices, the least recently used first.
/// The app setting is `FlashTeX.EngineV3.defaultEngine`; the old global
/// switch `FlashTeX.EngineV3.enabled` is read only as a fallback.
@MainActor
enum EngineChoiceStore {
    static let documentsKey = "FlashTeX.EngineV3.documents"
    static let appSettingKey = "FlashTeX.EngineV3.defaultEngine"
    static let legacyMigratedKey = "FlashTeX.EngineV3.legacyMigrated.v1"
    static let maxEntries = 500
    /// `origin` of a choice the old global switch made (Settings lists them).
    static let legacyOrigin = "legacy-switch"

    struct Entry: Equatable, Sendable {
        enum Source: String, Sendable { case user, record }
        var engine: EngineChoice.Engine
        var source: Source
        var setAt = Date()
        var appVersion = EngineChoiceStore.appVersion

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
        guard let d = v as? [String: Any],
              let engine = (d["engine"] as? String).flatMap(EngineChoice.Engine.init(rawValue:)),
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

    static func set(_ entry: Entry?, for url: URL, origin: String? = nil) {
        var dict = all
        let k = key(url)
        if let entry {
            let (root, name) = parts(url)
            var d: [String: Any] = ["engine": entry.engine.rawValue, "source": entry.source.rawValue,
                                    "set_at": entry.setAt, "used_at": Date(), "app_version": entry.appVersion, "entry": name]
            if let id = identityProvider(root.path)?.id { d["dir_id"] = id }
            if let origin { d["origin"] = origin }
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

    // MARK: Settings

    /// Documents with their own choice, and how many of them the old global switch made.
    static var choiceCounts: (user: Int, fromLegacySwitch: Int) {
        let users = all.values.compactMap { $0 as? [String: Any] }.filter { $0["source"] as? String == Entry.Source.user.rawValue }
        return (users.count, users.filter { $0["origin"] as? String == legacyOrigin }.count)
    }

    /// Settings > Compile > Reset Per-Document Choices: every document's own
    /// choice goes (records stay), and so does the old global switch, which
    /// would otherwise make them again.
    static func resetChoices() {
        all = all.filter { ($0.value as? [String: Any])?["source"] as? String != Entry.Source.user.rawValue }
        EngineV3.defaults.removeObject(forKey: EngineV3.enabledKey)
    }

    /// Settings > Compile > "Engine for other documents": nil is the built-in default.
    static var appSetting: EngineChoice.Engine? {
        get { EngineV3.defaults.string(forKey: appSettingKey).flatMap(EngineChoice.Engine.init(rawValue:)) }
        set {
            if let newValue { EngineV3.defaults.set(newValue.rawValue, forKey: appSettingKey) }
            else { EngineV3.defaults.removeObject(forKey: appSettingKey) }
        }
    }

    /// The old global switch was on (`FlashTeX.EngineV3.enabled` = true).
    static var legacyAllNew: Bool { EngineV3.defaults.object(forKey: EngineV3.enabledKey) as? Bool == true }

    /// Once per defaults domain: the old View toggle wrote
    /// `FlashTeX.EngineV3.enabled` on every change, so a stored false only
    /// says someone once turned v3 off. That is no choice and is removed,
    /// so that user gets the default (and its later flip). A stored true is
    /// kept and read only for documents with no entry yet, each of which
    /// it turns into that document's own choice when it opens.
    static func migrateLegacyFlag(in d: UserDefaults) {
        guard !d.bool(forKey: legacyMigratedKey) else { return }
        if d.object(forKey: EngineV3.enabledKey) as? Bool == false { d.removeObject(forKey: EngineV3.enabledKey) }
        d.set(true, forKey: legacyMigratedKey)
    }
}

// MARK: - the window's engine

extension ShellModel {
    /// Resolves the engine for the document that just opened (File > Open,
    /// a folder, a reload) and applies it, after its manifest was read.
    /// Records the engine it is typeset with, unless the choice is forced.
    /// A change of the default or of the app setting applies here, at the
    /// next open; an open window keeps its engine (#1236 §5.4).
    func resolveEngineForOpenedDocument() {
        engineChoiceDocument = documentURL
        engineFallbackDismissed = false
        let url = documentURL
        let entry = url.map(EngineChoiceStore.entry(for:)) ?? nil
        let c = EngineChoice.resolve(environment: ProcessInfo.processInfo.environment["FLASHTEX_ENGINE_V3"],
                                     window: engineWindowOverride, entry: entry, appSetting: EngineChoiceStore.appSetting,
                                     legacyAllNew: EngineChoiceStore.legacyAllNew,
                                     builtInDefault: EngineChoice.builtInDefault, blocker: { self.engineBlocker() })
        if let url { noteEngineTypeset(c, url: url, entry: entry) }
        applyEngineChoice(c)
    }

    /// Keeps what a document is typeset with: the old global switch becomes
    /// its own choice; otherwise a record of the engine, but never while a
    /// fallback rule blocks the new engine (a temporary block must not pin
    /// the document to the previous engine past the default's flip) and
    /// never for a forced choice.
    func noteEngineTypeset(_ c: EngineChoice, url: URL, entry: EngineChoiceStore.Entry?) {
        if c.source == .legacySwitch {
            EngineChoiceStore.set(.init(engine: .new, source: .user), for: url, origin: EngineChoiceStore.legacyOrigin)
        } else if !c.isForced, c.blocker == nil, entry?.source != .user, entry?.engine != c.preferred {
            EngineChoiceStore.set(.init(engine: c.preferred, source: .record), for: url)
        } else if entry != nil {
            EngineChoiceStore.touch(url)
        }
    }

    /// The open document was saved under a new name (Save As) or for the
    /// first time: a choice made for the unsaved buffer (or the old file)
    /// is kept for this file, and the fallback rules are checked again for
    /// its project (its manifest is read again).
    /// The caller set `engineChoicePending` before `documentURL` changed
    /// (no v3 open of the new path before its engine is known); cleared here.
    ///
    /// Order: the new project's manifest first, so the rules are its own;
    /// then a file with no entry yet takes the window's choice (a `user`
    /// choice as is; otherwise a record, never while blocked), while a file
    /// that has one keeps it; then the engine is resolved for the file.
    func engineDocumentSaved(from old: URL?) {
        defer { engineChoicePending = false }
        guard let url = documentURL, old?.standardizedFileURL != url.standardizedFileURL else { return }
        manifest.refresh() // engineChoicePending: manifestDidRefresh waits for the resolution below
        if EngineChoiceStore.entry(for: url) == nil {
            var carried = engineChoice
            if carried.source == .user {
                EngineChoiceStore.set(.init(engine: carried.preferred, source: .user), for: url)
            } else {
                if carried.preferred == .new, !carried.isForced { carried.blocker = engineBlocker() }
                noteEngineTypeset(carried, url: url, entry: nil)
            }
        }
        let (wasFallback, wasDismissed) = (engineChoice.blocker, engineFallbackDismissed)
        resolveEngineForOpenedDocument()
        // The same fallback, dismissed before the save: it stays dismissed.
        if wasDismissed, engineChoice.blocker != nil, engineChoice.blocker == wasFallback { engineFallbackDismissed = true }
    }

    /// The user picks an engine for the open document (the status bar's
    /// engine menu, View > Engine for This Document). Saved per document; an
    /// unsaved buffer keeps it, and its first save stores it for the file. The new engine
    /// still falls back when a rule says it cannot typeset the project.
    func chooseEngine(_ engine: EngineChoice.Engine) {
        engineWindowOverride = nil
        engineHostLacksTeXLive = false // asked again: the host's report is checked again when it starts
        engineFallbackDismissed = false
        if let url = documentURL { EngineChoiceStore.set(.init(engine: engine, source: .user), for: url) }
        var c = EngineChoice(preferred: engine, source: .user)
        if let forced = ProcessInfo.processInfo.environment["FLASHTEX_ENGINE_V3"], forced == "1" || forced == "0" {
            c = EngineChoice(preferred: forced == "1" ? .new : .previous, source: .environment)
        }
        if c.preferred == .new, !c.isForced { c.blocker = engineBlocker() }
        applyEngineChoice(c)
        navigationNote = c.blocker.map { "The new engine cannot typeset this project: \($0.detail)" }
            ?? "This document is typeset with the \(c.effective.title.lowercased())."
    }

    /// Forgets the user's choice for the open document: the app setting,
    /// the record and the default decide again.
    func clearEngineChoice() {
        if let url = documentURL { EngineChoiceStore.set(nil, for: url) }
        engineWindowOverride = nil
        resolveEngineForOpenedDocument()
    }

    /// Whether the new engine would be blocked for the open project right
    /// now, and why: no TeX Live (probed, or reported by the host), then the
    /// manifest's `[fonts]`.
    func engineBlocker() -> EngineChoice.Blocker? {
        if engineHostLacksTeXLive || !EngineChoice.texLiveAvailable() { return .noTeXLive }
        if let snapshot = manifest.currentSnapshot { return EngineChoice.blocker(manifest: snapshot.manifest) }
        return nil
    }

    /// The manifest was read again (an outside edit, the Fonts sheet): a
    /// fallback rule may now apply, or no longer apply, to the open
    /// document; the new engine's `texinputs` links follow. The preferred
    /// engine is not re-resolved (that happens when a document opens).
    func manifestDidRefresh() {
        // The open path resolves after its own read.
        guard !engineChoicePending, engineChoiceDocument == documentURL else { return }
        var c = engineChoice
        if c.preferred == .new, !c.isForced { c.blocker = engineBlocker() }
        if c != engineChoice {
            engineFallbackDismissed = false
            applyEngineChoice(c)
        }
        if engineV3Enabled { engineV3.manifestChanged(model: self) }
    }

    /// The host reported no TeX Live (and no format): the new engine falls
    /// back for this window, with the reason, unless it was forced.
    func engineV3HostLacksTeXLive() {
        guard !engineChoice.isForced, engineChoice.preferred == .new else { return }
        engineHostLacksTeXLive = true
        var c = engineChoice
        c.blocker = .noTeXLive
        engineFallbackDismissed = false
        applyEngineChoice(c)
    }

    /// Shows the choice and switches the window's engine to it (#1340's
    /// path: the other engine's work is dropped, this one compiles the
    /// current buffers). A new fallback is announced.
    func applyEngineChoice(_ c: EngineChoice) {
        let announce = c.blocker != nil && c.blocker != engineChoice.blocker
        engineChoice = c
        applyingEngineChoice = true
        engineV3Enabled = c.effective == .new
        applyingEngineChoice = false
        if announce, let b = c.blocker {
            let message = EngineFallbackBanner.headline(b)
            FlashTeXLog.write("engine choice: \(message) \(b.detail)")
            engineAnnouncements.append(message)
            NSAccessibility.post(element: NSApplication.shared, notification: .announcementRequested,
                                 userInfo: [.announcement: message, .priority: NSAccessibilityPriorityLevel.high.rawValue])
        }
    }

    /// `engineV3Enabled` was set directly (a bench, a test): that is the
    /// window's override, kept for every document it opens.
    func engineV3EnabledSetDirectly() {
        guard !applyingEngineChoice else { return }
        let engine: EngineChoice.Engine = engineV3Enabled ? .new : .previous
        engineWindowOverride = engine
        engineChoice = EngineChoice(preferred: engine, source: .window)
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

// MARK: - views

/// The status bar's engine item: which engine typesets this document, a
/// warning mark when a fallback rule applies, and the switch.
struct EngineChoiceStatusItem: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        let c = model.engineChoice
        Menu {
            EngineChoiceMenuItems(model: model)
        } label: {
            Label(c.title, systemImage: c.blocker == nil ? "bolt.horizontal.circle.fill" : "exclamationmark.triangle.fill")
                .font(DS.Fonts.secondary)
                .foregroundStyle(c.blocker == nil ? AnyShapeStyle(.secondary) : AnyShapeStyle(DS.Colors.severityWarning))
        }
        .font(DS.Fonts.secondary)
        .controlSize(.small)
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .fixedSize()
        .help(c.explanation)
        .accessibilityLabel(Self.spokenLabel(c))
        .accessibilityValue(Self.spokenValue(c))
        .accessibilityHint("Choose which engine typesets this document.")
        .accessibilityIdentifier("engine.choice")
    }
}

extension EngineChoiceStatusItem {
    /// What VoiceOver calls the item, and its value.
    static func spokenLabel(_ c: EngineChoice) -> String { "Engine: \(c.title)" }
    static func spokenValue(_ c: EngineChoice) -> String { c.blocker.map { "fallback, \($0.short)" } ?? c.source.explanation }
}

/// The engine menu's items (the status bar item and View > Engine for This Document).
struct EngineChoiceMenuItems: View {
    let model: ShellModel

    var body: some View {
        let c = model.engineChoice
        ForEach(EngineChoice.Engine.allCases, id: \.self) { engine in
            Button {
                model.chooseEngine(engine)
            } label: {
                if c.effective == engine { Label(engine.menuTitle, systemImage: "checkmark") } else { Text(engine.menuTitle) }
            }
            .disabled(engine == .new && c.blocker != nil && c.preferred == .new)
        }
        Divider()
        Text(c.explanation)
        if c.source == .user {
            Button("Use the Default for This Document") { model.clearEngineChoice() }
        }
    }
}

/// Over the preview while a fallback rule applies: the previous engine
/// typesets this project, and why. Dismissed until the next open or change.
struct EngineFallbackBanner: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        if let blocker = model.engineChoice.blocker, !model.engineFallbackDismissed {
            HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(DS.Colors.severityWarning)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: DS.Space.xs) {
                    Text(Self.headline(blocker)).font(DS.Fonts.secondary.weight(.semibold))
                    Text(blocker.detail).font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: 0)
                Button("Dismiss") { model.engineFallbackDismissed = true }
                    .ideSecondary()
            }
            .padding(DS.Space.m)
            .background(DS.Colors.surfaceSecondary)
            .accessibilityElement(children: .combine)
            .accessibilityLabel(Self.headline(blocker) + " " + blocker.detail)
            .accessibilityIdentifier("engine.fallback")
        }
    }
}

extension EngineFallbackBanner {
    /// The banner's first line (also what is announced when the fallback starts).
    static func headline(_ b: EngineChoice.Blocker) -> String { "Typeset with the previous engine: \(b.short)." }
}

/// Settings > Compile: the engine for documents without their own choice,
/// and the documents that carry their own (with a reset).
struct EngineChoiceSettingsSection: View {
    @State private var setting: EngineChoice.Engine? = EngineChoiceStore.appSetting
    @State private var counts = EngineChoiceStore.choiceCounts
    @State private var legacyOn = EngineChoiceStore.legacyAllNew

    /// The line under the reset button (nil: nothing to reset).
    static func choicesNote(user: Int, fromLegacySwitch: Int, legacyOn: Bool) -> String? {
        guard user > 0 || legacyOn else { return nil }
        var parts = [user == 1 ? "1 document has its own engine choice" : "\(user) documents have their own engine choice"]
        if fromLegacySwitch > 0 {
            parts.append("\(fromLegacySwitch) of them set to the new engine by the earlier Engine v3 Preview switch")
        }
        var text = parts.joined(separator: ", ") + "."
        if legacyOn { text += " That switch is still on: documents opened for the first time get the new engine as their own choice." }
        return text
    }

    var body: some View {
        Section("Engine") {
            Picker("Engine for other documents", selection: Binding(get: { setting }, set: { setting = $0; EngineChoiceStore.appSetting = $0 })) {
                Text("Default (\(EngineChoice.defaultForNewDocuments.title))").tag(EngineChoice.Engine?.none)
                ForEach(EngineChoice.Engine.allCases, id: \.self) { Text($0.menuTitle).tag(EngineChoice.Engine?.some($0)) }
            }
            if let note = Self.choicesNote(user: counts.user, fromLegacySwitch: counts.fromLegacySwitch, legacyOn: legacyOn) {
                Text(note).font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                Button("Reset Per-Document Choices") {
                    EngineChoiceStore.resetChoices()
                    counts = EngineChoiceStore.choiceCounts
                    legacyOn = EngineChoiceStore.legacyAllNew
                }
                .help("Forget every document's own engine choice (and the earlier switch); documents then follow the setting above when they open.")
            }
            Text("Applies to documents you have not chosen an engine for, when they open. Choose for the open document from the engine item in the status bar. When the new engine cannot typeset a project (no TeX Live, or fonts in flashtex.toml), the previous engine does, and the window says why.")
                .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
        }
    }
}
