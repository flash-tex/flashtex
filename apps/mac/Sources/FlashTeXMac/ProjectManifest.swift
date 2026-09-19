import AppKit
import ObjectiveC
import Observation
import FlashTeXProtocol

/// The optional project manifest, `flashtex.toml` (docs/user/project-manifest.md),
/// as the project-files helper reports it (`manifest` operation,
/// `DocumentFilesClient.swift`). There is exactly one parser for the
/// manifest — `crates/project-manifest`, linked into the CLI and the helper —
/// so this file never reads TOML: it asks the helper bound to the project
/// root, which walks up from the root to find the governing manifest and
/// answers with the resolved manifest, its warnings, the classified
/// `texinputs` and the *package inputs* (the `.sty`/`.cls`/`.def`/`.clo`
/// files next to the entry and every document-kind file under each
/// `texinputs` directory, with their text). Without the helper there is no
/// manifest: a folder still opens through its only `.tex` file, and the
/// status says why nothing more was read.
///
/// What the snapshot feeds:
/// - opening a folder (`ShellModel.openTex(at:)` on a directory): the entry
///   is `[project] entry`, else the folder's only `.tex` file;
/// - the compile request (`ProjectDocuments.implicitClosureDocuments`): the
///   package inputs that are not open members go out as documents, so the
///   compiler's `\usepackage` resolver finds `name.sty` by path;
/// - the sidebar (`WorkspaceSidebar.swift`): one dimmed row per package input
///   that is not open, with the class/style icon;
/// - File › Create flashtex.toml…: the helper's commented template, written
///   next to the entry through the ordinary rooted save and opened as a member.
///
/// Refreshed when a project opens and when the manifest file changes on disk
/// (one `DocumentWatcher`, like the entry's); never per keystroke or per
/// compile. Attached to the model as an associated object (`model.manifest`),
/// the way `ProjectDocuments` and `DocumentKinds` are.
@Observable
@MainActor
final class ProjectManifest {
    static let fileName = "flashtex.toml"

    /// Why a manifest read or a folder open was refused.
    struct Failure: Error, Equatable, Sendable {
        var why: String
        init(_ why: String) { self.why = why }
    }

    /// Whether a project path is the manifest (coloured as TOML, never LaTeX).
    nonisolated static func isManifestPath(_ path: String) -> Bool {
        path == fileName || path.hasSuffix("/" + fileName) || path.lowercased().hasSuffix(".toml")
    }

    /// One package input as the sidebar shows it.
    struct Row: Equatable, Sendable {
        var path: String
        var kind: String
        /// `[project] texinputs` index it came from; nil for a file next to the entry.
        var texinput: Int?
        /// The real file when `path` is a virtual `texinputs/<i>/…` mount.
        var origin: String?

        var tooltip: String {
            let what = kind == "class" ? "document class" : kind == "package" ? "package" : kind
            let from = texinput.map { "texinputs[\($0)] of flashtex.toml" } ?? "next to the entry document"
            if let origin { return "\(what) from \(from): \(origin) — compiled from there, outside the project root" }
            return "\(what) from \(from) — click to open"
        }

        var spoken: String {
            "\(path), \(kind == "class" ? "document class" : kind), not open, " + (origin == nil ? "activate to open" : "outside the project root")
        }
    }

    /// The last successful helper read for the current project root, nil
    /// before one (or when there is no root, or no helper).
    private(set) var snapshot: ProjectFilesV1.Manifest?
    /// The root `snapshot` was read for.
    private(set) var snapshotRoot: URL?
    private(set) var status = "no manifest read yet"
    /// Completed refreshes (tests).
    private(set) var refreshes = 0

    /// Test hook: replaces the helper read (a fake snapshot, or a failure
    /// standing in for "no helper"). Nil: `DocumentFilesState.manifest(for:entry:)`.
    @ObservationIgnored var reader: ((URL, String) -> Result<ProjectFilesV1.Manifest, Failure>)?
    @ObservationIgnored private unowned let model: ShellModel
    @ObservationIgnored private let watcher = DocumentWatcher()

    init(model: ShellModel) {
        self.model = model
        watcher.onChange = { [weak self] in self?.refresh() }
    }

    /// Whether the current project has a manifest on disk.
    var exists: Bool { snapshot?.exists == true && snapshotRoot == model.project.projectRoot }

    /// `<root>/flashtex.toml`: where Create flashtex.toml… writes, whatever
    /// the helper found above the root (a manifest above governs; creating
    /// one next to the entry makes the entry's directory its own project).
    var manifestURL: URL? { model.project.projectRoot?.appendingPathComponent(Self.fileName) }

    // MARK: reading

    private func read(root: URL, entry: String) -> Result<ProjectFilesV1.Manifest, Failure> {
        if let reader { return reader(root, entry) }
        return model.files.manifest(for: root, entry: entry)
    }

    /// Re-reads the manifest for the current project root. No root (an
    /// unsaved buffer): nothing to read, the snapshot is dropped.
    func refresh() {
        guard let root = model.project.projectRoot else {
            snapshot = nil
            snapshotRoot = nil
            status = "no project root (the entry document is not saved)"
            watcher.stop()
            return
        }
        switch read(root: root, entry: model.project.entryPath) {
        case .success(let m):
            snapshot = m
            snapshotRoot = root
            refreshes += 1
            let inputs = m.files.count
            status = m.exists
                ? "\(m.path ?? Self.fileName): \(inputs) package input\(inputs == 1 ? "" : "s")"
                    + (m.warnings.isEmpty ? "" : "; \(m.warnings.count) warning\(m.warnings.count == 1 ? "" : "s"): " + m.warnings.map(\.key).joined(separator: ", "))
                    + (m.diagnostics.isEmpty ? "" : "; " + m.diagnostics.map(\.message).joined(separator: "; "))
                : "no flashtex.toml (defaults); \(inputs) package input\(inputs == 1 ? "" : "s") next to the entry"
            FlashTeXLog.write("manifest: " + status)
            // Watch the governing file (or where one would appear) so an
            // edit re-reads it; the watcher is a no-op on a missing file.
            if ProcessInfo.processInfo.environment["FLASHTEX_NO_FILE_WATCH"] != "1" {
                let url = m.path.map { URL(fileURLWithPath: $0) } ?? root.appendingPathComponent(Self.fileName)
                if watcher.url != url || !watcher.isWatching { _ = watcher.watch(url) }
            }
        case .failure(let f):
            snapshot = nil
            snapshotRoot = root
            status = f.why
            FlashTeXLog.write("manifest: " + f.why)
        }
    }

    // MARK: what the snapshot feeds

    /// The package inputs the helper reported for the current root, as
    /// compile documents (`ProjectDocuments.implicitClosureDocuments` drops
    /// the ones that are open members: the buffer is authoritative).
    func packageInputs() -> [ProjectDocuments.ImplicitDocument] {
        guard let snapshot, snapshotRoot == model.project.projectRoot else { return [] }
        return snapshot.files.map { ProjectDocuments.ImplicitDocument(path: $0.path, text: $0.text) }
    }

    /// Sidebar rows for the package inputs (open members are filtered by the sidebar).
    var rows: [Row] {
        guard let snapshot, snapshotRoot == model.project.projectRoot else { return [] }
        return snapshot.files.map { Row(path: $0.path, kind: $0.kind, texinput: $0.texinput, origin: $0.origin) }
    }

    // MARK: opening a folder

    /// The document a folder opens as: the manifest's `[project] entry`
    /// (relative to the manifest's directory) when it names an existing
    /// file, else the folder's only `.tex` file. Two candidates or none is
    /// a refusal naming them — like the CLI, the app never guesses.
    nonisolated static func entry(in folder: URL, manifest: ProjectFilesV1.Manifest?) -> Result<URL, Failure> {
        if let manifest, manifest.exists, let name = manifest.manifest.project.entry {
            let dir = manifest.manifestDir.map { URL(fileURLWithPath: $0) } ?? folder
            let url = dir.appendingPathComponent(name).standardizedFileURL
            var isDirectory: ObjCBool = false
            guard FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory), !isDirectory.boolValue else {
                return .failure(Failure("\(manifest.path ?? fileName) names entry \"\(name)\", which is not a file under \(dir.path)"))
            }
            return .success(url)
        }
        let names = ((try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? [])
            .filter { $0.lowercased().hasSuffix(".tex") }
            .sorted()
        switch names.count {
        case 1: return .success(folder.appendingPathComponent(names[0]))
        case 0: return .failure(Failure("no .tex file in the folder and no flashtex.toml naming an entry"))
        default: return .failure(Failure("\(names.count) .tex files (\(names.joined(separator: ", "))); open one of them, or create a flashtex.toml with [project] entry"))
        }
    }

    /// `entry(in:manifest:)` with the helper's read for `folder`. Without
    /// the helper the folder's only `.tex` file still opens, and the
    /// refusal (if any) says the manifest could not be read.
    func resolveEntry(in folder: URL) -> Result<URL, Failure> {
        switch read(root: folder, entry: "main.tex") {
        case .success(let m): return Self.entry(in: folder, manifest: m)
        case .failure(let f):
            return Self.entry(in: folder, manifest: nil).mapError { Failure("\($0.why) (\(f.why))") }
        }
    }

    // MARK: creating the manifest

    enum CreateOutcome: Equatable {
        case created(path: String)
        case refused(String)
    }

    /// Writes the helper's commented template (naming this project's entry)
    /// as `<root>/flashtex.toml` through the rooted save with `expected:
    /// .newFile` — an existing file is a conflict, never overwritten — then
    /// re-reads the manifest and opens the file as a project member.
    @discardableResult
    func createManifest() async -> CreateOutcome {
        guard let root = model.project.projectRoot, let url = manifestURL else {
            return note(.refused("cannot create \(Self.fileName): the entry document is not saved, so there is no project root"))
        }
        if FileManager.default.fileExists(atPath: url.path) {
            return note(.refused("\(Self.fileName) already exists next to \(model.project.entryPath); open it instead"))
        }
        let template: String
        switch read(root: root, entry: model.project.entryPath) {
        case .success(let m): template = m.template
        case .failure(let f): return note(.refused("cannot create \(Self.fileName): \(f.why)"))
        }
        switch model.files.save(url, text: template, expected: .newFile, force: false, recordsState: false) {
        case .saved: break
        case .conflict(let c): return note(.refused("cannot create \(Self.fileName): \(c.summary)"))
        case .failed(let why): return note(.refused("cannot create \(Self.fileName): \(why)"))
        }
        refresh()
        switch await model.project.openDocument(Self.fileName, role: .opened) {
        case .opened, .alreadyOpen:
            _ = model.project.switchDocument(to: Self.fileName)
            return note(.created(path: Self.fileName))
        case .refused(let why):
            return note(.refused("created \(Self.fileName) but could not open it: \(why)"))
        }
    }

    /// The File menu / sidebar command: creates and reports in the status line.
    func createManifestInteractive() async {
        switch await createManifest() {
        case .created(let path): model.navigationNote = "Created \(path) — every key is optional; see docs/user/project-manifest.md"
        case .refused(let why): model.navigationNote = why
        }
    }

    private func note(_ outcome: CreateOutcome) -> CreateOutcome {
        switch outcome {
        case .created(let p): status = "created \(p)"
        case .refused(let why): status = why
        }
        FlashTeXLog.write("manifest: " + status)
        return outcome
    }
}

// MARK: - model attachment

extension ShellModel {
    private static var manifestKey = 0
    /// The project manifest (see `ProjectManifest`).
    var manifest: ProjectManifest {
        if let existing = objc_getAssociatedObject(self, &Self.manifestKey) as? ProjectManifest { return existing }
        let state = ProjectManifest(model: self)
        objc_setAssociatedObject(self, &Self.manifestKey, state, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return state
    }
}
