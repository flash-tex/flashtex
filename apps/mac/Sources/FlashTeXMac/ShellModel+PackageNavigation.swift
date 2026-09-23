import Foundation
import SwiftUI

/// The project's package and class files as editor intelligence sees them
/// (lane pkg-editor): what completion scans for "declared in mystyle.sty"
/// rows, what Go to Definition and the hover peek search after the open
/// documents, and how a file that is not a project member is opened —
/// a rooted `.sty` next to the entry as an ordinary member, a virtual
/// `texinputs/<i>/…` or `packages/<name>/…` path read-only with a banner
/// saying where it comes from (`ProjectDocuments.openVirtual`).
///
/// The compile result's `metadata.packages` (`RuntimeV1.PackageRecord`,
/// `docs/contracts/runtime-v1.md`) is what a package defined as the engine
/// loaded it, with the definition's byte span in the file: Go to Definition
/// and the hover peek read it first (`packageDefinition`), completion rows
/// come from it (`Completion.packageDeclarations(in:records:)`). The package
/// inputs' text, as the compiler's own `\usepackage` resolver reads it
/// (ProjectManifest.packageInputs, ProjectPackagesState.documents), remains
/// the fallback: no result yet, an older producer, or a name the engine did
/// not record is found by the same lexical scan the open documents use
/// (EditorNavigation).
extension ShellModel {
    /// One package input beyond the open members: its text and, when it
    /// has no file under the project root, where it really comes from.
    struct PackageInput: Equatable {
        var path: String
        var text: String
        /// Nil for a file under the project root (openable as a member).
        var virtualSource: String?
    }

    /// The package inputs that are not open members, in the compile
    /// request's order: the manifest's (a `texinputs` mount says where its
    /// real file is), then the resolved packages (each says which library
    /// or cache delivered it).
    var packageInputs: [PackageInput] {
        let open = Set(documents.map(\.path))
        var seen = open
        var out: [PackageInput] = []
        let rowsByPath = Dictionary(manifest.rows.map { ($0.path, $0) }, uniquingKeysWith: { a, _ in a })
        for input in manifest.packageInputs() where seen.insert(input.path).inserted {
            let row = rowsByPath[input.path]
            let source = row?.origin.map { "\($0) (texinputs[\(row?.texinput ?? 0)] of \(ProjectManifest.fileName))" }
            out.append(PackageInput(path: input.path, text: input.text, virtualSource: source))
        }
        let delivered = Dictionary(projectPackages.rows.map { ($0.path, $0.source) }, uniquingKeysWith: { a, _ in a })
        for doc in projectPackages.documents() where seen.insert(doc.path).inserted {
            out.append(PackageInput(path: doc.path, text: doc.text, virtualSource: delivered[doc.path] ?? "a resolved package"))
        }
        return out
    }

    /// What completion scans for package declarations: the open `.sty`/`.cls`
    /// members other than the active buffer (its own declarations are
    /// "declared in this document"), then `packageInputs`. Read once per
    /// list request; the scan runs off-main.
    func packageDocumentsForEditor() -> [Completion.SourceDocument] {
        var out = documents.filter { $0.path != activePath && ProjectManifest.isPackagePath($0.path) }
            .map { Completion.SourceDocument(path: $0.path, text: $0.text) }
        out += packageInputs.map { Completion.SourceDocument(path: $0.path, text: $0.text) }
        return out
    }

    /// A definition found in a package input that is not open.
    struct PackageDefinition: Equatable {
        var input: PackageInput
        var definition: EditorNavigation.Definition
    }

    /// The definition of `\name` (or of environment `name`) in the package
    /// inputs, after `definition(ofCommand:)` found none in the open
    /// documents: the engine's own (`enginePackageDefinition`), else the
    /// first `\newcommand`/`\def`/… (`\newenvironment`/`\newtheorem`) the
    /// lexical scan finds in the inputs' text.
    func packageDefinition(ofCommand name: String, environment: Bool = false) -> PackageDefinition? {
        if let hit = enginePackageDefinition(ofCommand: name, environment: environment) { return hit }
        for input in packageInputs {
            if let d = EditorNavigation.definition(of: name, in: input.text as NSString, environment: environment) {
                return PackageDefinition(input: input, definition: d)
            }
        }
        return nil
    }

    /// `result.metadata.packages` lookup: the first definition of `name` of
    /// the asked kind (`Completion.declaration(of:)`) in a file that is a
    /// package input, its span -- UTF-8 bytes into the text the compiler
    /// read -- converted to UTF-16 against the input's text. Nil when the
    /// result carries no record for the name, or when the input's text has
    /// changed since it was compiled (`compiledDocuments`): the engine's
    /// offsets then no longer index the text, and the lexical scan takes
    /// over. `via` is the engine's definer; the summary has no body (the
    /// selection shows the whole statement).
    private func enginePackageDefinition(ofCommand name: String, environment: Bool) -> PackageDefinition? {
        guard let records = result?.metadata?.packages, !records.isEmpty else { return nil }
        let inputs = packageInputs
        for record in records {
            guard let input = inputs.first(where: { $0.path == record.path }) else { continue }
            if let compiled = compiledDocuments[record.path], compiled != input.text { continue }
            for d in record.definitions where d.name == name {
                guard let declaration = Completion.declaration(of: d), (declaration.kind == .environment) == environment else { continue }
                guard let range = input.text.nsRange(utf8Bytes: d.span.sourceRange),
                      let line = EditorDiagnostics.lineNumber(ofByte: d.span.start, in: input.text) else { continue }
                return PackageDefinition(input: input, definition: EditorNavigation.Definition(name: name, via: d.definer, range: range, body: nil, line: line))
            }
        }
        return nil
    }

    /// Opens a package input as a member — rooted: through the ordinary
    /// open (a `.sty` next to the entry becomes an editable member);
    /// virtual: read-only with the banner — and switches to it. False with
    /// the refusal noted.
    @discardableResult
    func openPackageInput(_ input: PackageInput) async -> Bool {
        if let source = input.virtualSource {
            switch project.openVirtual(input.path, text: input.text, source: source) {
            case .opened, .alreadyOpen:
                if case .refused(let why) = project.switchDocument(to: input.path) { navigationNote = why; return false }
                return true
            case .refused(let why):
                navigationNote = why
                return false
            }
        }
        return await openAndSwitch(input.path, role: .opened) { [weak self] in self?.navigationNote = $0 }
    }

    /// Opens the package input at `path` (a diagnostic's path, a definition's)
    /// when it is one; false — with nothing noted — for any other path.
    @discardableResult
    func openPackageInput(at path: String) async -> Bool {
        guard let input = packageInputs.first(where: { $0.path == path }) else { return false }
        return await openPackageInput(input)
    }

    /// "Create name.sty" on the Problems row of the compiler's missing-package
    /// diagnostic: writes `<root>/name.sty` with the package template
    /// (`PackageTemplate`, through the rooted create — an existing file is
    /// refused, never overwritten), opens it as a member and switches to
    /// it; the next compile resolves `\usepackage{name}` to it.
    @discardableResult
    func createPackageFile(named name: String) async -> ProjectDocuments.CreateOutcome {
        guard ProjectPackagesState.isPackageName(name) else {
            let outcome = ProjectDocuments.CreateOutcome.refused("\(name) is not a package name")
            navigationNote = "\(name) is not a package name"
            return outcome
        }
        let outcome = await project.createDocument(name + ".sty", role: .opened)
        switch outcome {
        case .created(let path):
            project.switchDocument(to: path)
            navigationNote = "Created \(path) from the package template; the next compile loads it for \\usepackage{\(name)}"
        case .refused(let why):
            navigationNote = why
        }
        return outcome
    }

    /// ⌘-click / ⌃⌘J on a macro a package defines: opens the file (read-only
    /// when virtual) and selects the definition, like `reveal` does for an
    /// open document.
    func goToPackageDefinition(_ hit: PackageDefinition) {
        Task { @MainActor [weak self] in
            guard let self, await self.openPackageInput(hit.input) else { return }
            self.selectInEditor(hit.definition.range, path: hit.input.path)
            self.navigationNote = "Definition: \(hit.definition.summary) at line \(hit.definition.line) in \(hit.input.path)"
                + (hit.input.virtualSource.map { " (read-only, from \($0))." } ?? ".")
        }
    }
}

/// The strip above a read-only buffer: where the shown file really lives.
struct ReadOnlyBanner: View {
    let note: String

    var body: some View {
        HStack(spacing: DS.Space.s) {
            Image(systemName: "lock.fill").foregroundStyle(DS.Colors.textSecondary).accessibilityHidden(true)
            Text(note).font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary).lineLimit(1).truncationMode(.middle)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xs)
        .background(DS.Colors.surfaceRaised)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Read-only: \(note)")
        .accessibilityIdentifier("editor.readonly")
    }
}
