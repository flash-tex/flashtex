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
//      the `FlashTeX.EngineV3.enabled` default): "New engine" or "Previous
//      engine"; absent means the built-in default.
//   5. The engine this document was last typeset with (a record written
//      when it opens), so a change of the built-in default never switches a
//      document someone has already seen typeset (ruling 4, #1236 §5.3).
//   6. The built-in default, `EngineChoice.defaultForNewDocuments`.
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
        case environment, window, user, appSetting, record, builtInDefault

        var explanation: String {
            switch self {
            case .environment: "set by FLASHTEX_ENGINE_V3 in the environment"
            case .window: "set for this window"
            case .user: "your choice for this document"
            case .appSetting: "Settings > Compile"
            case .record: "the engine this document was last typeset with"
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
        case projectFonts([String])
        /// `[packages] pin`: the new engine uses TeX Live's packages.
        case pinnedPackages([String])
        /// `[packages] path`: local libraries, which the new engine does not read yet.
        case packageLibraries([String])

        /// The banner's and the status item's short reason.
        var short: String {
            switch self {
            case .noTeXLive: "no TeX Live is installed"
            case .projectFonts: "this project sets fonts in flashtex.toml"
            case .pinnedPackages: "this project pins package versions"
            case .packageLibraries: "this project uses local package libraries"
            }
        }

        /// The full reason (tooltips, the menu, the announcement).
        var detail: String {
            switch self {
            case .noTeXLive:
                "No TeX Live installation was found. The new engine prepares its pdfLaTeX format from your TeX Live; install MacTeX or TeX Live to use it."
            case .projectFonts(let roles):
                "This project's flashtex.toml sets [fonts] (\(roles.joined(separator: ", "))). The new engine is pdfLaTeX-compatible and typesets with TeX's fonts, so it would ignore them."
            case .pinnedPackages(let names):
                "This project's flashtex.toml pins package versions ([packages] pin: \(names.joined(separator: ", "))). The new engine uses your TeX Live's packages, so it would not honour the pins."
            case .packageLibraries(let names):
                "This project's flashtex.toml uses local package libraries ([packages] path: \(names.joined(separator: ", "))), which the new engine does not read yet."
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
                        appSetting: Engine?, builtInDefault: Engine, blocker: () -> Blocker?) -> EngineChoice {
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
        resolve(environment: ProcessInfo.processInfo.environment["FLASHTEX_ENGINE_V3"], window: nil, entry: nil,
                appSetting: EngineV3.underTest ? nil : EngineChoiceStore.appSetting, builtInDefault: builtInDefault,
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

    /// The manifest's fallback facts (pure): `[fonts]` roles, pins, libraries.
    static func blocker(manifest m: ProjectFilesV1.Manifest.Body) -> Blocker? {
        let f = m.fonts
        let roles = [("text", f.text), ("math", f.math), ("mono", f.mono), ("sans", f.sans)]
            .compactMap { role, name in (name ?? "").trimmingCharacters(in: .whitespaces).isEmpty ? nil : role }
        if !roles.isEmpty { return .projectFonts(roles) }
        if !m.packages.pin.isEmpty { return .pinnedPackages(m.packages.pin.keys.sorted()) }
        if !m.packages.path.isEmpty { return .packageLibraries(m.packages.path.keys.sorted()) }
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
/// own suite under XCTest), keyed by the entry document's resolved path:
/// `{engine: new|previous, source: user|record, set_at, app_version}`.
/// The app setting is the existing `FlashTeX.EngineV3.enabled` key.
@MainActor
enum EngineChoiceStore {
    static let documentsKey = "FlashTeX.EngineV3.documents"

    struct Entry: Equatable, Sendable {
        enum Source: String, Sendable { case user, record }
        var engine: EngineChoice.Engine
        var source: Source
        var setAt = Date()
        var appVersion = EngineChoiceStore.appVersion

        static func == (a: Entry, b: Entry) -> Bool { a.engine == b.engine && a.source == b.source }
    }

    nonisolated static var appVersion: String { Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "dev" }

    static func key(_ url: URL) -> String { url.standardizedFileURL.resolvingSymlinksInPath().path }

    static func entry(for url: URL) -> Entry? {
        guard let d = (EngineV3.defaults.dictionary(forKey: documentsKey) ?? [:])[key(url)] as? [String: Any],
              let engine = (d["engine"] as? String).flatMap(EngineChoice.Engine.init(rawValue:)),
              let source = (d["source"] as? String).flatMap(Entry.Source.init(rawValue:)) else { return nil }
        return Entry(engine: engine, source: source, setAt: d["set_at"] as? Date ?? .distantPast,
                     appVersion: d["app_version"] as? String ?? "")
    }

    static func set(_ entry: Entry?, for url: URL) {
        var all = EngineV3.defaults.dictionary(forKey: documentsKey) ?? [:]
        if let entry {
            all[key(url)] = ["engine": entry.engine.rawValue, "source": entry.source.rawValue,
                             "set_at": entry.setAt, "app_version": entry.appVersion] as [String: Any]
        } else {
            all.removeValue(forKey: key(url))
        }
        EngineV3.defaults.set(all, forKey: documentsKey)
    }

    /// Settings > Compile > "Engine for other documents": nil is the built-in default.
    static var appSetting: EngineChoice.Engine? {
        get {
            guard EngineV3.defaults.object(forKey: EngineV3.enabledKey) != nil else { return nil }
            return EngineV3.defaults.bool(forKey: EngineV3.enabledKey) ? .new : .previous
        }
        set {
            if let newValue { EngineV3.defaults.set(newValue == .new, forKey: EngineV3.enabledKey) }
            else { EngineV3.defaults.removeObject(forKey: EngineV3.enabledKey) }
        }
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
                                     builtInDefault: EngineChoice.builtInDefault, blocker: { self.engineBlocker() })
        if let url, !c.isForced, entry?.source != .user, entry?.engine != c.effective {
            EngineChoiceStore.set(.init(engine: c.effective, source: .record), for: url)
        }
        applyEngineChoice(c)
    }

    /// The user picks an engine for the open document (the status bar's
    /// engine menu, View > Engine for This Document). Saved per document; an
    /// unsaved buffer keeps it until another document opens. The new engine
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
    /// manifest's `[fonts]`, `[packages] pin` and `[packages] path`.
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
        guard engineChoiceDocument == documentURL else { return } // the open path resolves after its own read
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

/// Settings > Compile: the engine for documents without their own choice.
struct EngineChoiceSettingsSection: View {
    @State private var setting: EngineChoice.Engine? = EngineChoiceStore.appSetting

    var body: some View {
        Section("Engine") {
            Picker("Engine for other documents", selection: Binding(get: { setting }, set: { setting = $0; EngineChoiceStore.appSetting = $0 })) {
                Text("Default (\(EngineChoice.defaultForNewDocuments.title))").tag(EngineChoice.Engine?.none)
                ForEach(EngineChoice.Engine.allCases, id: \.self) { Text($0.menuTitle).tag(EngineChoice.Engine?.some($0)) }
            }
            Text("Applies to documents you have not chosen an engine for, when they open. Choose for the open document from the engine item in the status bar. When the new engine cannot typeset a project (no TeX Live, fonts or pinned packages in flashtex.toml), the previous engine does, and the window says why.")
                .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
        }
    }
}
