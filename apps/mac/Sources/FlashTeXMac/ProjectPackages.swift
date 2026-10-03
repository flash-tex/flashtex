import AppKit
import ObjectiveC
import Observation
import SwiftUI
import FlashTeXProtocol

/// Package resolution in the shell (docs/user/project-manifest.md
/// `[packages]`, proposal docs/proposals/packages-fonts-manifest.md §2.3/§S3):
/// after every compile, the packages the compiler reports it could not find
/// (`packages x, y are recognised but not implemented`, and the compiler's
/// `no project file found: looked for x.sty` note) are handed to the
/// project-files helper's `resolve_packages`, which answers from the
/// manifest's local libraries and the per-user package cache without
/// touching the network. What is neither there is, under the manifest's
/// `fetch` policy, the subject of ONE consent sheet per project listing what
/// would be fetched from where — Fetch, Not now, Never for this project —
/// and nothing is ever fetched without it (`fetch = "always"` is the
/// remembered answer of an earlier sheet, written when "Remember" was
/// ticked). Resolved files reach the compiler as documents at
/// `packages/<name>/<file>` (ProjectDocuments.implicitClosureDocuments, the
/// direct route) and show under the sidebar's Packages group; a delivery
/// bumps `revision`, which ShellModel.compile compares to recompile.
///
/// Under the new engine (engine v3, which reads TeX Live and the project
/// copy, EngineV3Session.swift) the same state serves it: TeX's own
/// ``LaTeX Error: File `x.sty' not found.`` names the package, the
/// delivered files are written into the session's copy and linked at its top
/// level (EngineV3Mirror.materializePackages), and before a compile the
/// manifest's `[packages] pin` versions and `[packages] path` libraries are
/// resolved from the libraries and the cache alone (`prepareForEngineV3`),
/// so that they come before TeX Live's copies, as TEXINPUTS=.: puts them.
/// Only those, and packages TeX reported missing, reach the new engine
/// (`engineV3Documents`): an unpinned cached copy never shadows TeX Live.
/// Under the new engine nothing is sent to the package source before
/// consent: a missing package is looked up in the libraries and the cache
/// (offline), and the sheet offers it by name; its description is asked
/// for only on Look Up, and its files only on Fetch (or `fetch = "always"`).
///
/// Attached to the model as an associated object (`model.projectPackages`),
/// like `ProjectManifest` and `ProjectFontsState`.
@Observable
@MainActor
final class ProjectPackagesState {
    /// One package the sheet offers to fetch.
    struct Offer: Equatable, Sendable {
        var name: String
        /// The source's stated version (CTAN), nil for a registry.
        var version: String?
        var sourceURL: String
        var files: [String]
        /// False for a name-only offer (the new engine): nothing has been
        /// asked of the source yet, `sourceURL` is the source itself.
        var lookedUp = true

        /// "siunitx 3.3.24 — siunitx.sty, siunitx.cfg"
        var summary: String {
            let what = version.map { "\(name) \($0)" } ?? name
            return files.isEmpty ? what : "\(what) — \(files.joined(separator: ", "))"
        }
        /// "CTAN" or the registry host.
        var sourceLabel: String {
            if sourceURL.hasPrefix("https://mirrors.ctan.org/") { return "CTAN" }
            return URL(string: sourceURL)?.host ?? sourceURL
        }
    }

    /// One resolved package: its files, as compile documents.
    struct Delivered: Equatable, Sendable {
        var name: String
        var version: String
        /// "the local library mylib", "the package cache", "CTAN (fetched now)".
        var source: String
        var files: [ProjectDocuments.ImplicitDocument]
    }

    /// The sheet is up (ContentView attaches it).
    var shown = false
    /// "Remember: fetch without asking for this project" (writes `fetch = "always"`).
    var remember = false
    private(set) var offers: [Offer] = []
    private(set) var delivered: [String: Delivered] = [:]
    /// Packages the helper could not resolve, with the reason (shown in the sheet, not asked again).
    private(set) var unavailable: [String: String] = [:]
    /// "Not now" for this project, this session.
    private(set) var declined: Set<String> = []
    private(set) var root: URL?
    private(set) var status = "no packages resolved yet"
    /// Bumped whenever the delivered set changes; ShellModel.compile recompiles when it differs from the applied result's.
    private(set) var revision = 0
    private(set) var resolving = false
    var applying = false
    var note: String?
    private(set) var errorDetails: String?
    /// Counters for tests.
    private(set) var resolves = 0
    private(set) var recompiles = 0

    /// Test hooks: the helper's `resolve_packages` and `set_packages`. Nil:
    /// `DocumentFilesState.resolvePackages` / `setPackages`.
    @ObservationIgnored var resolver: ((URL, [String], Bool) async -> Result<ProjectFilesV1.ResolvePackages, ProjectManifest.Failure>)?
    @ObservationIgnored var rewriter: ((URL, String, String?, [String: String]?) -> Result<ProjectFilesV1.SetPackages, ProjectManifest.Failure>)?
    /// Test hook: the helper's `offline` `resolve_packages` (names, and
    /// whether `libraries` are listed too). Nil: the helper.
    @ObservationIgnored var offlineResolver: ((URL, [String], Bool) async -> Result<ProjectFilesV1.ResolvePackages, ProjectManifest.Failure>)?
    /// Engine v3: the offline resolution of the pins and libraries is in flight.
    private(set) var engineV3Preparing = false
    /// ... and the session holds its compiles until it ends (only when the
    /// helper was idle as it started: never behind a fetch).
    @ObservationIgnored private var engineV3Holding = false
    var engineV3HoldsCompile: Bool { engineV3Preparing && engineV3Holding }
    /// The manifest's pins and libraries the last offline resolution was for
    /// (nil after a failure: the next try asks again).
    @ObservationIgnored private var engineV3PreparedKey: String?
    /// What that resolution delivered (keys of `delivered`): replaced by the
    /// next successful one, kept when one fails.
    @ObservationIgnored private var engineV3Prepared: Set<String> = []
    /// Packages the new engine's TeX reported missing (this project).
    @ObservationIgnored private var engineV3Missing: Set<String> = []
    /// After a failed offline resolution: the key and when to try again
    /// (doubling from `engineV3RetryBase`, at most a minute).
    @ObservationIgnored private var engineV3FailedKey: String?
    @ObservationIgnored private var engineV3RetryAt: Date?
    @ObservationIgnored private var engineV3RetryWork: DispatchWorkItem?
    /// The first retry's delay (tests shorten it).
    @ObservationIgnored var engineV3RetryBase: TimeInterval = 1
    /// Offline resolutions run, failed in a row (tests).
    private(set) var engineV3Preparations = 0
    private(set) var engineV3Failures = 0
    /// The helper answers one request at a time: an offline request from
    /// here is sent only after the earlier ones are answered (`serialized`),
    /// so it never times out behind a long fetch. Requests waiting or sent.
    @ObservationIgnored private var helperTail: Task<Void, Never>?
    private(set) var helperRequests = 0
    @ObservationIgnored private unowned let model: ShellModel
    @ObservationIgnored private var inFlight: Set<String> = []

    init(model: ShellModel) { self.model = model }

    // MARK: what the compiler could not find

    /// The package names a compile's diagnostics say were not found:
    /// `packages a, b are recognised but not implemented` (the compiler's
    /// `\usepackage` gap diagnostic) and `no project file found: looked for
    /// a.sty` (its resolver's note), de-duplicated in order.
    nonisolated static func unresolvedNames(in diagnostics: [RuntimeV1.Diagnostic]) -> [String] {
        var seen: Set<String> = []
        var out: [String] = []
        func add(_ name: String) {
            let trimmed = name.trimmingCharacters(in: .whitespaces)
            guard isPackageName(trimmed), seen.insert(trimmed).inserted else { return }
            out.append(trimmed)
        }
        for d in diagnostics {
            let texts = [d.message] + (d.notes ?? [])
            for text in texts {
                if let file = texMissingFile(in: text) {
                    // TeX's wording under the new engine: LaTeX Error: File `x.sty' not found.
                    for ext in [".sty", ".cls"] where file.hasSuffix(ext) { add(String(file.dropLast(ext.count))) }
                } else if text.hasPrefix("packages "), text.hasSuffix(" are recognised but not implemented") {
                    let list = text.dropFirst("packages ".count).dropLast(" are recognised but not implemented".count)
                    list.split(separator: ",").forEach { add(String($0)) }
                } else if let range = text.range(of: "no project file found: looked for ") {
                    let rest = text[range.upperBound...]
                    let file = rest.split(whereSeparator: { $0 == " " || $0 == "," || $0 == ";" }).first.map(String.init) ?? ""
                    for ext in [".sty", ".cls"] where file.hasSuffix(ext) { add(String(file.dropLast(ext.count))) }
                }
            }
        }
        return out
    }

    /// The file a LaTeX "File `x' not found" error names (`\usepackage`,
    /// `\documentclass`, `\input` under the new engine), nil otherwise.
    nonisolated static func texMissingFile(in message: String) -> String? {
        guard let open = message.range(of: "LaTeX Error: File `"),
              let close = message.range(of: "' not found", range: open.upperBound ..< message.endIndex) else { return nil }
        let name = String(message[open.upperBound ..< close.lowerBound])
        return name.isEmpty ? nil : name
    }

    nonisolated static func isPackageName(_ s: String) -> Bool {
        !s.isEmpty && !s.hasPrefix(".") && s.count <= 100 && s.allSatisfy { $0.isLetter || $0.isNumber || "-_.+".contains($0) } && s.allSatisfy(\.isASCII)
    }

    /// The packages one diagnostic says were not found (`unresolvedNames`)
    /// that have no `<name>.sty` under `projectRoot` yet: what the Problems
    /// row offers "Create name.sty" and "Fetch name…" for. Empty without a
    /// root (nothing could be written), and once the file exists.
    nonisolated static func missingPackages(for d: RuntimeV1.Diagnostic, projectRoot: URL?) -> [String] {
        guard let root = projectRoot else { return [] }
        return unresolvedNames(in: [d]).filter { name in
            guard case .file(let url) = ProjectDocuments.rootedFile(name + ".sty", under: root) else { return false }
            return !FileManager.default.fileExists(atPath: url.path)
        }
    }

    /// "Fetch name…" on a Problems row: asks the helper about exactly these
    /// packages — declined or not — and shows the consent sheet for what
    /// needs it (cached ones are delivered at once). Says why when the
    /// manifest forbids fetching for this project, or the entry is unsaved.
    func presentFetch(_ names: [String]) {
        guard let projectRoot = model.project.projectRoot else {
            model.navigationNote = "Save the entry document first (⌘S): packages are resolved for a project."
            return
        }
        if root != projectRoot { reset(for: projectRoot) }
        note = nil
        errorDetails = nil
        declined.subtract(names)
        for name in names { unavailable[name] = nil }
        if manifestFetch == "never" || manifestSource == "none" {
            model.navigationNote = "\(ProjectManifest.fileName) says [packages] fetch = \"\(manifestFetch)\", source = \"\(manifestSource)\": nothing is fetched for this project (edit the manifest to change that)."
            return
        }
        let pending = Set(offers.map(\.name))
        let wanted = names.filter { delivered[$0] == nil && !inFlight.contains($0) }
        guard !wanted.isEmpty else {
            let done = names.filter { delivered[$0] != nil }
            model.navigationNote = done.isEmpty ? "\(names.joined(separator: ", ")): already being resolved"
                : "\(done.joined(separator: ", ")) already resolved: " + done.map { "\($0) from \(delivered[$0]!.source)" }.joined(separator: "; ")
            return
        }
        if wanted.allSatisfy(pending.contains) { shown = true; return }
        if model.engineV3Enabled { Task { await resolveForEngineV3(wanted.filter { !pending.contains($0) }) }; return }
        Task { await resolve(wanted.filter { !pending.contains($0) }, consent: manifestFetch == "always") }
    }

    /// Called from `ShellModel.result`'s observer (the previous engine) and
    /// from the new engine's DONE with its rows (`diagnostics`): resolves
    /// what the latest compile could not find and is not yet delivered,
    /// unavailable, declined or in flight. A new project root resets everything.
    func noteCompileResult(diagnostics: [RuntimeV1.Diagnostic]? = nil) {
        guard let projectRoot = model.project.projectRoot else { return }
        if root != projectRoot { reset(for: projectRoot) }
        let pending = Set(offers.map(\.name))
        let names = Self.unresolvedNames(in: diagnostics ?? model.result?.diagnostics ?? []).filter { name in
            delivered[name] == nil && unavailable[name] == nil && !declined.contains(name) && !inFlight.contains(name) && !pending.contains(name)
        }
        guard !names.isEmpty else { return }
        if diagnostics != nil, model.engineV3Enabled { Task { await resolveForEngineV3(names) }; return }
        Task { await resolve(names, consent: manifestFetch == "always") }
    }

    /// The menu command: asks again about everything the last compile could
    /// not find, declined or not, or says why there is nothing to ask.
    func present() {
        guard let projectRoot = model.project.projectRoot else {
            model.navigationNote = "Save the entry document first (⌘S): packages are resolved for a project."
            return
        }
        if root != projectRoot { reset(for: projectRoot) }
        note = nil
        errorDetails = nil
        declined = []
        unavailable = [:]
        if !offers.isEmpty { shown = true; return }
        let last = model.engineV3Enabled ? model.engineV3Diagnostics : model.result?.diagnostics ?? []
        let names = Self.unresolvedNames(in: last).filter { delivered[$0] == nil && !inFlight.contains($0) }
        guard !names.isEmpty else {
            model.navigationNote = delivered.isEmpty ? "Every package the last compile asked for was found." : "Every package the last compile asked for is resolved: \(delivered.keys.sorted().joined(separator: ", "))."
            return
        }
        if manifestFetch == "never" || manifestSource == "none" {
            model.navigationNote = "\(ProjectManifest.fileName) says [packages] fetch = \"\(manifestFetch)\", source = \"\(manifestSource)\": nothing is fetched for this project (edit the manifest to change that)."
        }
        if model.engineV3Enabled { Task { await resolveForEngineV3(names) }; return }
        Task { await resolve(names, consent: manifestFetch == "always") }
    }

    private var manifestFetch: String {
        guard let s = model.manifest.snapshot, model.manifest.snapshotRoot == model.project.projectRoot else { return "ask" }
        return s.manifest.packages.fetch
    }

    private var manifestSource: String {
        guard let s = model.manifest.snapshot, model.manifest.snapshotRoot == model.project.projectRoot else { return "ctan" }
        return s.manifest.packages.source
    }

    private var manifestPins: [String: String] {
        guard let s = model.manifest.snapshot, model.manifest.snapshotRoot == model.project.projectRoot else { return [:] }
        return s.manifest.packages.pin
    }

    private var manifestLibraries: [String: String] {
        guard let s = model.manifest.snapshot, model.manifest.snapshotRoot == model.project.projectRoot else { return [:] }
        return s.manifest.packages.path
    }

    private func reset(for projectRoot: URL) {
        engineV3PreparedKey = nil
        engineV3Prepared = []
        engineV3Missing = []
        engineV3Preparing = false
        engineV3Holding = false
        engineV3FailedKey = nil
        engineV3RetryAt = nil
        engineV3RetryWork?.cancel()
        engineV3RetryWork = nil
        engineV3Failures = 0
        root = projectRoot
        offers = []
        delivered = [:]
        unavailable = [:]
        declined = []
        inFlight = []
        shown = false
        remember = false
        note = nil
        errorDetails = nil
        revision += 1
    }

    // MARK: resolving

    private typealias Reply = Result<ProjectFilesV1.ResolvePackages, ProjectManifest.Failure>

    /// Runs `op`; with `waits` (an offline request), only after every
    /// earlier request from here has been answered, so its short timeout
    /// never runs while the helper is busy with a fetch. Other requests are
    /// sent at once, as before (the previous engine's order is unchanged).
    private func serialized(waits: Bool, _ op: @escaping @MainActor () async -> Reply) async -> Reply {
        let previous = helperTail
        helperRequests += 1
        let task = Task { @MainActor () -> Reply in
            if waits { _ = await previous?.value }
            return await op()
        }
        helperTail = Task { _ = await previous?.value; _ = await task.value }
        let reply = await task.value
        helperRequests -= 1
        return reply
    }

    private func resolveThroughHelper(_ root: URL, _ names: [String], _ consent: Bool, offline: Bool = false, libraries: Bool = false) async -> Reply {
        let entry = model.project.entryPath, files = model.files
        let hook = resolver, offlineHook = offlineResolver
        return await serialized(waits: offline) {
            if offline {
                if let offlineHook { return await offlineHook(root, names, libraries) }
                return await files.resolvePackages(for: root, names: names, consent: false, entry: entry, offline: true, libraries: libraries)
            }
            if let hook { return await hook(root, names, consent) }
            return await files.resolvePackages(for: root, names: names, consent: consent, entry: entry)
        }
    }

    /// Asks the helper about `names`; delivers what is cached (or fetched,
    /// with `consent`), queues what needs consent for the sheet, records
    /// what is unavailable. Delivery recompiles.
    /// `offline` (the new engine): the libraries and the cache only.
    @discardableResult
    func resolve(_ names: [String], consent: Bool, offline: Bool = false) async -> Bool {
        guard let root, !names.isEmpty else { return false }
        inFlight.formUnion(names)
        resolving = true
        defer { resolving = false; inFlight.subtract(names) }
        weak var shell = model
        let reply = await resolveThroughHelper(root, names, consent, offline: offline)
        // The window closed while the helper answered (the new engine's
        // DONE asks too): nothing to deliver to, and `model` is gone.
        guard shell != nil else { return false }
        resolves += 1
        guard root == self.root else { return false } // the project changed meanwhile
        switch reply {
        case .failure(let f):
            errorDetails = f.why
            let message = Self.friendlyResolveError(source: manifestSource)
            note = message
            status = "packages: \(message)"
            FlashTeXLog.write("packages: \(f.why)")
            return false
        case .success(let r):
            errorDetails = nil
            var changed = false
            for p in r.packages {
                switch p.status {
                case .cached, .fetched:
                    let source = p.status == .fetched
                        ? "\(Offer(name: p.name, version: nil, sourceURL: p.sourceUrl ?? "", files: []).sourceLabel) (fetched \(p.version ?? "") just now)"
                        : p.from == "library" ? "the local library (\(ProjectManifest.fileName) [packages] path)" : "the package cache (version \(p.version ?? "?"))"
                    let files = (p.files ?? []).map { ProjectDocuments.ImplicitDocument(path: $0.path, text: $0.text) }
                    let entry = Delivered(name: p.name, version: p.version ?? "", source: source, files: files)
                    if delivered[p.name] != entry { delivered[p.name] = entry; changed = true }
                    unavailable[p.name] = nil
                case .needsConsent:
                    let offer = Offer(name: p.name, version: p.version, sourceURL: p.sourceUrl ?? "", files: p.wouldFetch ?? [])
                    if let i = offers.firstIndex(where: { $0.name == p.name && !$0.lookedUp }) { offers[i] = offer } // Look Up's answer
                    else if !offers.contains(offer) { offers.append(offer) }
                case .notAvailable:
                    unavailable[p.name] = p.reason ?? "not available"
                    offers.removeAll { $0.name == p.name && !$0.lookedUp } // looked up: the source has no such package
                }
            }
            for d in r.diagnostics { FlashTeXLog.write("packages: \(d.key): \(d.message)") }
            let delivered = self.delivered.keys.sorted()
            status = "packages: \(delivered.count) resolved" + (delivered.isEmpty ? "" : " (\(delivered.joined(separator: ", ")))")
                + (offers.isEmpty ? "" : "; \(offers.count) awaiting consent") + (unavailable.isEmpty ? "" : "; \(unavailable.count) unavailable")
            FlashTeXLog.write(status)
            if changed {
                revision += 1
                recompile()
            }
            if !offers.isEmpty, !consent { shown = true }
            return true
        }
    }

    private func recompile() {
        recompiles += 1
        if model.engineV3Enabled { model.engineV3.packagesChanged(model: model); return }
        guard model.workerAttached else { return }
        model.compile()
    }

    // MARK: the new engine: missing packages without asking the source

    /// Packages the new engine's TeX reported missing: the libraries and the
    /// cache first (offline). What is in neither is offered by name in the
    /// sheet (`fetch = "ask"`): nothing about it leaves this Mac until the
    /// user asks (Look Up) or agrees (Fetch). `fetch = "always"` is the
    /// manifest's standing consent: fetched at once. `never` and `source =
    /// "none"`: the offline answer stands.
    func resolveForEngineV3(_ names: [String]) async {
        guard !names.isEmpty else { return }
        engineV3Missing.formUnion(names)
        let fetching = manifestFetch != "never" && manifestSource != "none"
        if fetching, manifestFetch == "always" { await resolve(names, consent: true); return }
        guard await resolve(names, consent: false, offline: true), fetching else { return }
        offerByName(names.filter { delivered[$0] == nil && !declined.contains($0) })
    }

    /// Name-only offers: the sheet shows them, nothing has been sent anywhere.
    private func offerByName(_ names: [String]) {
        guard !names.isEmpty else { return }
        let source = manifestSource == "ctan" ? "https://mirrors.ctan.org/" : manifestSource
        for name in names where !offers.contains(where: { $0.name == name }) {
            unavailable[name] = nil // the offline answer ("not cached") is not the source's
            offers.append(Offer(name: name, version: nil, sourceURL: source, files: [], lookedUp: false))
        }
        shown = true
    }

    /// Look Up (the sheet): asks the source to describe the name-only
    /// offers (version, files, URL); fetches nothing.
    @discardableResult
    func lookUp() async -> Bool {
        let names = offers.filter { !$0.lookedUp }.map(\.name)
        guard !names.isEmpty else { return false }
        note = nil
        errorDetails = nil
        return await resolve(names, consent: false)
    }

    /// What the new engine is given (EngineV3Session): the pinned packages,
    /// the libraries and the packages its TeX reported missing, never an
    /// unpinned cached copy the previous engine asked for (TeX Live's wins).
    func engineV3Documents() -> [ProjectDocuments.ImplicitDocument] {
        let names = engineV3Prepared.union(engineV3Missing).union(manifestPins.keys)
        var seen: Set<String> = []
        var out: [ProjectDocuments.ImplicitDocument] = []
        for name in delivered.keys.sorted() where names.contains(name) {
            for f in delivered[name]!.files where seen.insert(f.path).inserted { out.append(f) }
        }
        return out
    }

    // MARK: the new engine: pins and libraries before TeX Live

    /// Called by the engine-v3 session before it compiles. The new engine
    /// reads TeX Live, which has most pinned packages already, so what the
    /// manifest asks for in their place (each `[packages] pin` version and
    /// every `[packages] path` library) is resolved first, from the
    /// libraries and the cache only (the helper's `offline`: never the
    /// network), once per project and manifest. True while that is in
    /// flight and holds the compile (the end compiles,
    /// `EngineV3Session.packagesChanged`); a resolution that has to wait for
    /// an earlier request (a fetch) holds nothing, the compile uses what was
    /// delivered before. A failure (or timeout) keeps what was delivered and
    /// tries again after 1, 2, 4 … 60 s. A pin that is not cached is offered
    /// by name like a missing package; until it arrives TeX Live's copy is
    /// used and the window says so.
    @discardableResult
    func prepareForEngineV3() -> Bool {
        guard let projectRoot = model.project.projectRoot else { return false }
        if root != projectRoot { reset(for: projectRoot) }
        if engineV3Preparing { return engineV3HoldsCompile }
        let pins = manifestPins, libraries = manifestLibraries
        let key = pins.sorted { $0.key < $1.key }.map { "\($0.key)=\($0.value)" }.joined(separator: ",") + "|"
            + libraries.sorted { $0.key < $1.key }.map { "\($0.key)=\($0.value)" }.joined(separator: ",")
        guard key != engineV3PreparedKey else { return false }
        // After a failure, the same manifest is asked again only once its delay is over.
        if key == engineV3FailedKey, let at = engineV3RetryAt, Date() < at { return false }
        engineV3PreparedKey = key
        if pins.isEmpty, libraries.isEmpty {
            // The manifest no longer names any: what an earlier one brought goes.
            engineV3FailedKey = nil
            guard !engineV3Prepared.isEmpty else { return false }
            for name in engineV3Prepared where !engineV3Missing.contains(name) { delivered[name] = nil }
            engineV3Prepared = []
            revision += 1
            return false
        }
        engineV3Preparing = true
        engineV3Holding = helperRequests == 0
        engineV3Preparations += 1
        let names = pins.keys.sorted()
        // Neither the model nor this state is kept alive by the resolution
        // (a window closed meanwhile ends it).
        weak var shell = model
        Task { [weak self] in
            guard let self else { return }
            let reply = await resolveThroughHelper(projectRoot, names, false, offline: true, libraries: true)
            guard let model = shell else { return }
            guard root == projectRoot, engineV3PreparedKey == key else { return } // the project or manifest changed meanwhile
            engineV3Preparing = false
            engineV3Holding = false
            var uncached: [String] = []
            switch reply {
            case .failure(let f):
                // Keep what the last success delivered; ask again later.
                engineV3Failures += 1
                engineV3PreparedKey = nil
                engineV3FailedKey = key
                let delay = min(engineV3RetryBase * pow(2, Double(engineV3Failures - 1)), 60)
                engineV3RetryAt = Date().addingTimeInterval(delay)
                FlashTeXLog.write("packages: the pins and libraries could not be resolved for the new engine (\(f.why)); again in \(delay) s")
                status = "packages: pins and libraries not resolved (\(f.why)); trying again in \(Int(delay.rounded(.up))) s"
                let work = DispatchWorkItem { [weak self] in
                    guard let self, shell?.engineV3Enabled == true, self.root == projectRoot else { return }
                    self.engineV3RetryWork = nil
                    self.engineV3RetryAt = nil
                    self.prepareForEngineV3() // its end compiles
                }
                engineV3RetryWork?.cancel()
                engineV3RetryWork = work
                DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: work)
                model.engineV3.packagesChanged(model: model) // the held compile, with what was there before
                return
            case .success(let r):
                engineV3Failures = 0
                engineV3FailedKey = nil
                engineV3RetryAt = nil
                for name in engineV3Prepared where !engineV3Missing.contains(name) { delivered[name] = nil }
                engineV3Prepared = []
                let file = ProjectManifest.fileName
                for lib in r.libraries ?? [] {
                    let files = lib.files.map { ProjectDocuments.ImplicitDocument(path: $0.path, text: $0.text) }
                    delivered[lib.name] = Delivered(name: lib.name, version: lib.version, source: "the local library (\(file) [packages] path)", files: files)
                    engineV3Prepared.insert(lib.name)
                }
                for p in r.packages {
                    guard p.status == .cached else { uncached.append(p.name); continue }
                    let files = (p.files ?? []).map { ProjectDocuments.ImplicitDocument(path: $0.path, text: $0.text) }
                    let source = p.from == "library" ? "the local library (\(file) [packages] path)" : "the package cache (version \(p.version ?? "?"), pinned in \(file))"
                    delivered[p.name] = Delivered(name: p.name, version: p.version ?? "", source: source, files: files)
                    engineV3Prepared.insert(p.name)
                }
                for d in r.diagnostics { FlashTeXLog.write("packages: \(d.key): \(d.message)") }
                let done = delivered.keys.sorted()
                status = "packages: \(done.count) resolved" + (done.isEmpty ? "" : " (\(done.joined(separator: ", ")))")
                FlashTeXLog.write(status + " for the new engine")
            }
            revision += 1
            model.engineV3.packagesChanged(model: model) // the held compile
            guard !uncached.isEmpty else { return }
            let pinned = uncached.map { "\($0) \(pins[$0] ?? "")" }.joined(separator: ", ")
            model.navigationNote = "\(ProjectManifest.fileName) pins \(pinned), which the package cache does not have: TeX Live's copy is used until it is fetched."
            let fetching = manifestFetch != "never" && manifestSource != "none"
            let wanted = uncached.filter { delivered[$0] == nil && !declined.contains($0) && !inFlight.contains($0) }
            if fetching, manifestFetch == "always" { await resolve(wanted, consent: true) }
            else if fetching { offerByName(wanted) }
        }
        return engineV3Holding
    }

    // MARK: the sheet's three answers

    /// Fetch: the helper fetches every offered package into the cache (the
    /// consent), the files join the next compile; "Remember" writes
    /// `fetch = "always"` so the sheet does not come back for this project.
    @discardableResult
    func fetch() async -> Bool {
        guard let root, !offers.isEmpty else { return false }
        applying = true
        defer { applying = false }
        let names = offers.map(\.name)
        let before = delivered.count
        note = nil
        errorDetails = nil
        let replySucceeded = await resolve(names, consent: true)
        let failed = names.filter { delivered[$0] == nil }
        if !replySucceeded { return false }
        if !failed.isEmpty {
            note = failed.map { "\($0): \(unavailable[$0] ?? "not fetched")" }.joined(separator: "\n")
            return false
        }
        offers = []
        shown = false
        if remember { await writePolicy(fetch: "always") }
        let fetched = names.joined(separator: ", ")
        model.navigationNote = "Fetched \(fetched) into the package cache (\(delivered.count - before) new); recompiling"
        _ = root
        return true
    }

    /// Not now: nothing fetched, not asked again this session.
    func notNow() {
        declined.formUnion(offers.map(\.name))
        offers = []
        shown = false
        model.navigationNote = "Packages not fetched; File › Fetch Missing Packages… asks again"
    }

    /// Never for this project: writes `fetch = "never"` into the manifest
    /// (created from the template when there is none) through the helper's
    /// rewrite and the rooted save; nothing fetched.
    @discardableResult
    func never() async -> Bool {
        declined.formUnion(offers.map(\.name))
        offers = []
        let ok = await writePolicy(fetch: "never")
        if ok { shown = false; model.navigationNote = "\(ProjectManifest.fileName): [packages] fetch = \"never\" — nothing is fetched for this project" }
        return ok
    }

    private func writePolicy(fetch: String) async -> Bool {
        guard let root else { return false }
        let entry = model.project.entryPath
        let rewritten = rewriter.map { $0(root, entry, fetch, nil) } ?? model.files.setPackages(for: root, entry: entry, fetch: fetch, pin: nil)
        let reply: ProjectFilesV1.SetPackages
        switch rewritten {
        case .success(let r): reply = r
        case .failure(let f):
            note = "cannot write \(ProjectManifest.fileName): \(f.why)"
            return false
        }
        guard reply.changed, let text = reply.text else { model.manifest.refresh(); return true }
        let url = URL(fileURLWithPath: reply.path)
        switch model.files.save(url, text: text, expected: reply.exists ? .any : .newFile, force: false, recordsState: false) {
        case .saved: break
        case .conflict(let c):
            note = "cannot write \(ProjectManifest.fileName): \(c.summary)"
            return false
        case .failed(let why):
            note = "cannot write \(ProjectManifest.fileName): \(why)"
            return false
        }
        model.manifest.refresh()
        return true
    }

    // MARK: what the deliveries feed

    /// The resolved files as compile documents, one per path.
    func documents() -> [ProjectDocuments.ImplicitDocument] {
        var seen: Set<String> = []
        var out: [ProjectDocuments.ImplicitDocument] = []
        for name in delivered.keys.sorted() {
            for f in delivered[name]!.files where seen.insert(f.path).inserted { out.append(f) }
        }
        return out
    }

    /// Sidebar rows (WorkspaceSidebar.swift, the Packages group).
    var rows: [ProjectManifest.Row] {
        var seen: Set<String> = []
        var out: [ProjectManifest.Row] = []
        // Under the new engine, what it is given (`engineV3Documents`).
        let shownNames = model.engineV3Enabled ? Set(engineV3Documents().compactMap { $0.path.split(separator: "/").dropFirst().first.map(String.init) }) : nil
        for name in delivered.keys.sorted() where shownNames?.contains(name) ?? true {
            let d = delivered[name]!
            for f in d.files where seen.insert(f.path).inserted {
                let kind = f.path.hasSuffix(".cls") || f.path.hasSuffix(".clo") ? "class" : "package"
                out.append(ProjectManifest.Row(path: f.path, kind: kind, texinput: nil, origin: nil, source: d.source))
            }
        }
        return out
    }

    nonisolated static func friendlyResolveError(source: String) -> String {
        source == "ctan" ? "Couldn't reach CTAN; check your connection, then try again." : "Couldn't resolve packages; check your connection, then try again."
    }
}

// MARK: - model attachment

extension ShellModel {
    private static var projectPackagesKey = 0
    /// Package resolution state (see `ProjectPackagesState`).
    var projectPackages: ProjectPackagesState {
        if let existing = objc_getAssociatedObject(self, &Self.projectPackagesKey) as? ProjectPackagesState { return existing }
        let state = ProjectPackagesState(model: self)
        objc_setAssociatedObject(self, &Self.projectPackagesKey, state, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return state
    }
}

// MARK: - the consent sheet

/// One sheet per project: what would be fetched, from where, and the three
/// answers. Never shown for a `fetch = "always"` project (the answer is
/// remembered) and never fetches on its own.
struct ProjectPackagesSheet: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        @Bindable var state = model.projectPackages
        let file = model.manifest.snapshot?.path.map { URL(fileURLWithPath: $0).lastPathComponent } ?? ProjectManifest.fileName
        let offers = state.offers
        let offerCount = offers.count
        let sources = Set(offers.map(\.sourceLabel)).sorted().joined(separator: ", ")
        VStack(alignment: .leading, spacing: DS.Space.l) {
            Text("Fetch Missing Packages").font(.title2.bold())
            Text("The document uses \(offerCount) package\(offerCount == 1 ? "" : "s") that \(offerCount == 1 ? "is" : "are") not in this project, a local library or the package cache. FlashTeX can fetch the LaTeX source files below from \(sources.isEmpty ? "the source" : sources) into the per-user cache. Nothing is fetched until you say so, and nothing fetched is ever executed outside the typesetter.")
                .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            VStack(alignment: .leading, spacing: DS.Space.s) {
                ForEach(offers, id: \.name) { offer in
                    HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                        Image(systemName: "shippingbox").foregroundStyle(DS.Colors.textSecondary).accessibilityHidden(true)
                        VStack(alignment: .leading, spacing: DS.Space.xxs) {
                            Text(offer.summary).font(DS.Fonts.base)
                            if offer.lookedUp {
                                Text(offer.sourceURL).font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textSecondary).lineLimit(1).truncationMode(.middle)
                            } else {
                                Text("Not looked up: nothing has been sent to \(offer.sourceLabel) yet").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                            }
                        }
                    }
                    .accessibilityElement(children: .combine)
                    .accessibilityLabel("\(offer.summary), from \(offer.sourceLabel), \(offer.sourceURL)")
                }
            }
            .padding(DS.Space.m)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(DS.Colors.surfaceRaised, in: RoundedRectangle(cornerRadius: DS.Radius.panel))
            .overlay(RoundedRectangle(cornerRadius: DS.Radius.panel).stroke(DS.Colors.componentBorder))
            if !state.unavailable.isEmpty {
                Text(state.unavailable.keys.sorted().map { "\($0): \(state.unavailable[$0] ?? "")" }.joined(separator: "\n"))
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.severityWarning).fixedSize(horizontal: false, vertical: true)
                    .accessibilityLabel("Unavailable: " + state.unavailable.keys.sorted().joined(separator: ", "))
            }
            Toggle("Remember: fetch without asking for this project (writes fetch = \"always\" to \(file))", isOn: $state.remember)
                .font(DS.Fonts.secondary)
                .accessibilityIdentifier("project.packages.remember")
            if let note = state.note {
                VStack(alignment: .leading, spacing: DS.Space.xs) {
                    Text(note).font(.callout).foregroundStyle(DS.Colors.severityWarning).fixedSize(horizontal: false, vertical: true)
                    if let details = state.errorDetails {
                        DisclosureGroup("Show details") {
                            Text(details).font(DS.Fonts.monoSecondary).textSelection(.enabled)
                        }
                        .accessibilityIdentifier("project.packages.error-details")
                    }
                }
            }
            if state.applying {
                ProgressView("Fetching packages…")
                    .controlSize(.small)
                    .accessibilityIdentifier("project.packages.progress")
            }
            HStack {
                Button("Never for This Project") { Task { await state.never() } }
                    .accessibilityHint("Writes fetch = \"never\" to \(file); FlashTeX will not ask again and fetches nothing for this project.")
                    .accessibilityIdentifier("project.packages.never")
                Spacer()
                if state.offers.contains(where: { !$0.lookedUp }) {
                    Button("Look Up") { Task { await state.lookUp() } }
                        .disabled(state.applying || state.resolving)
                        .accessibilityHint("Asks \(sources.isEmpty ? "the source" : sources) for the version and files of the packages listed by name; fetches nothing.")
                        .accessibilityIdentifier("project.packages.lookup")
                }
                Button("Not Now") { state.notNow() }
                    .keyboardShortcut(.cancelAction)
                    .accessibilityHint("Fetches nothing; File › Fetch Missing Packages… asks again.")
                    .accessibilityIdentifier("project.packages.not-now")
                Button(state.applying ? "Fetching…" : "Fetch") { Task { await state.fetch() } }
                    .keyboardShortcut(.defaultAction)
                    .disabled(state.applying || state.offers.isEmpty)
                    .accessibilityIdentifier("project.packages.fetch")
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.sheetWidth)
    }
}
