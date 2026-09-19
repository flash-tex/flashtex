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
    /// Counters for tests.
    private(set) var resolves = 0
    private(set) var recompiles = 0

    /// Test hooks: the helper's `resolve_packages` and `set_packages`. Nil:
    /// `DocumentFilesState.resolvePackages` / `setPackages`.
    @ObservationIgnored var resolver: ((URL, [String], Bool) async -> Result<ProjectFilesV1.ResolvePackages, ProjectManifest.Failure>)?
    @ObservationIgnored var rewriter: ((URL, String, String?, [String: String]?) -> Result<ProjectFilesV1.SetPackages, ProjectManifest.Failure>)?
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
                if text.hasPrefix("packages "), text.hasSuffix(" are recognised but not implemented") {
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

    nonisolated static func isPackageName(_ s: String) -> Bool {
        !s.isEmpty && !s.hasPrefix(".") && s.count <= 100 && s.allSatisfy { $0.isLetter || $0.isNumber || "-_.+".contains($0) } && s.allSatisfy(\.isASCII)
    }

    /// Called from `ShellModel.result`'s observer: resolves what the latest
    /// compile could not find and is not yet delivered, unavailable,
    /// declined or in flight. A new project root resets everything.
    func noteCompileResult() {
        guard let projectRoot = model.project.projectRoot else { return }
        if root != projectRoot { reset(for: projectRoot) }
        let pending = Set(offers.map(\.name))
        let names = Self.unresolvedNames(in: model.result?.diagnostics ?? []).filter { name in
            delivered[name] == nil && unavailable[name] == nil && !declined.contains(name) && !inFlight.contains(name) && !pending.contains(name)
        }
        guard !names.isEmpty else { return }
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
        declined = []
        unavailable = [:]
        if !offers.isEmpty { shown = true; return }
        let names = Self.unresolvedNames(in: model.result?.diagnostics ?? []).filter { delivered[$0] == nil && !inFlight.contains($0) }
        guard !names.isEmpty else {
            model.navigationNote = delivered.isEmpty ? "Every package the last compile asked for was found." : "Every package the last compile asked for is resolved: \(delivered.keys.sorted().joined(separator: ", "))."
            return
        }
        if manifestFetch == "never" || manifestSource == "none" {
            model.navigationNote = "\(ProjectManifest.fileName) says [packages] fetch = \"\(manifestFetch)\", source = \"\(manifestSource)\": nothing is fetched for this project (edit the manifest to change that)."
        }
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

    private func reset(for projectRoot: URL) {
        root = projectRoot
        offers = []
        delivered = [:]
        unavailable = [:]
        declined = []
        inFlight = []
        shown = false
        remember = false
        note = nil
        revision += 1
    }

    // MARK: resolving

    private func resolveThroughHelper(_ root: URL, _ names: [String], _ consent: Bool) async -> Result<ProjectFilesV1.ResolvePackages, ProjectManifest.Failure> {
        if let resolver { return await resolver(root, names, consent) }
        return await model.files.resolvePackages(for: root, names: names, consent: consent, entry: model.project.entryPath)
    }

    /// Asks the helper about `names`; delivers what is cached (or fetched,
    /// with `consent`), queues what needs consent for the sheet, records
    /// what is unavailable. Delivery recompiles.
    @discardableResult
    func resolve(_ names: [String], consent: Bool) async -> Bool {
        guard let root, !names.isEmpty else { return false }
        inFlight.formUnion(names)
        resolving = true
        defer { resolving = false; inFlight.subtract(names) }
        let reply = await resolveThroughHelper(root, names, consent)
        resolves += 1
        guard root == self.root else { return false } // the project changed meanwhile
        switch reply {
        case .failure(let f):
            status = "packages: \(f.why)"
            note = f.why
            FlashTeXLog.write(status)
            return false
        case .success(let r):
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
                    if !offers.contains(offer) { offers.append(offer) }
                case .notAvailable:
                    unavailable[p.name] = p.reason ?? "not available"
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
            return changed
        }
    }

    private func recompile() {
        recompiles += 1
        guard model.workerAttached else { return }
        model.compile()
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
        offers = []
        _ = await resolve(names, consent: true)
        let failed = names.filter { delivered[$0] == nil }
        if !failed.isEmpty {
            note = failed.map { "\($0): \(unavailable[$0] ?? "not fetched")" }.joined(separator: "\n")
            return false
        }
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
        for name in delivered.keys.sorted() {
            let d = delivered[name]!
            for f in d.files where seen.insert(f.path).inserted {
                let kind = f.path.hasSuffix(".cls") || f.path.hasSuffix(".clo") ? "class" : "package"
                out.append(ProjectManifest.Row(path: f.path, kind: kind, texinput: nil, origin: nil, source: d.source))
            }
        }
        return out
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
        let sources = Set(state.offers.map(\.sourceLabel)).sorted().joined(separator: ", ")
        VStack(alignment: .leading, spacing: DS.Space.l) {
            Text("Fetch Missing Packages").font(.title2.bold())
            Text("The document uses \(state.offers.count) package\(state.offers.count == 1 ? "" : "s") that \(state.offers.count == 1 ? "is" : "are") not in this project, a local library or the package cache. FlashTeX can fetch the LaTeX source files below from \(sources.isEmpty ? "the source" : sources) into the per-user cache. Nothing is fetched until you say so, and nothing fetched is ever executed outside the typesetter.")
                .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            VStack(alignment: .leading, spacing: DS.Space.s) {
                ForEach(state.offers, id: \.name) { offer in
                    HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                        Image(systemName: "shippingbox").foregroundStyle(DS.Colors.textSecondary).accessibilityHidden(true)
                        VStack(alignment: .leading, spacing: DS.Space.xxs) {
                            Text(offer.summary).font(DS.Fonts.base)
                            Text(offer.sourceURL).font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textSecondary).lineLimit(1).truncationMode(.middle)
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
                Text(note).font(.callout).foregroundStyle(DS.Colors.severityWarning).fixedSize(horizontal: false, vertical: true)
            }
            HStack {
                Button("Never for This Project") { Task { await state.never() } }
                    .accessibilityHint("Writes fetch = \"never\" to \(file); FlashTeX will not ask again and fetches nothing for this project.")
                    .accessibilityIdentifier("project.packages.never")
                Spacer()
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
