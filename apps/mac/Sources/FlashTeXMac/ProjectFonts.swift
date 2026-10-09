import AppKit
import ObjectiveC
import Observation
import SwiftUI
import FlashTeXProtocol

/// File › Project Fonts… (also in the command palette): the project's
/// `[fonts]` table (docs/user/project-manifest.md, proposal
/// docs/proposals/packages-fonts-manifest.md §2.4 — "set font settings
/// globally") as four rows, Text, Math, Sans and Mono, each a searchable
/// picker over the families the engine's own index finds
/// (`flashtex-render --list-fonts`, the same names `\setmainfont{` completes
/// with) with a sample line set in that family, and "Class default" first.
///
/// Apply writes the table into `flashtex.toml` — created from the helper's
/// template when there is none — through the project-files helper's
/// `set_fonts` (`crates/project-manifest` `Manifest::with_fonts`, the one
/// TOML writer: everything else in the file is kept byte for byte) and the
/// ordinary rooted save, then the manifest is re-read and the preview
/// follows (ProjectManifest.refresh → ShellModel.manifestFontsDidChange).
/// A project without a manifest and every row at "Class default" writes
/// nothing. The document's own `\setmainfont` etc. always outrank the table.
///
/// Attached to the model as an associated object (`model.projectFonts`),
/// like `ProjectManifest`.
@Observable
@MainActor
final class ProjectFontsState {
    /// One family per role; nil is "Class default" (the key is not written).
    struct Selection: Equatable, Sendable {
        var text: String?
        var math: String?
        var sans: String?
        var mono: String?

        init(text: String? = nil, math: String? = nil, sans: String? = nil, mono: String? = nil) {
            self.text = text; self.math = math; self.sans = sans; self.mono = mono
        }

        /// The manifest's table as the helper reports it (nil: no manifest read).
        init(manifest fonts: ProjectFilesV1.Manifest.Fonts?) {
            self.init(text: fonts?.text, math: fonts?.math, sans: fonts?.sans, mono: fonts?.mono)
        }

        var isClassDefault: Bool { text == nil && math == nil && sans == nil && mono == nil }

        /// Family names by role for the helper's `set_fonts`; an unset role
        /// is absent, which removes its key.
        var byRole: [String: String] {
            var out: [String: String] = [:]
            if let text { out["text"] = text }
            if let math { out["math"] = math }
            if let sans { out["sans"] = sans }
            if let mono { out["mono"] = mono }
            return out
        }

        subscript(role: Role) -> String? {
            get {
                switch role {
                case .text: text
                case .math: math
                case .sans: sans
                case .mono: mono
                }
            }
            set {
                let value = newValue?.trimmingCharacters(in: .whitespacesAndNewlines)
                let name = (value?.isEmpty ?? true) ? nil : value
                switch role {
                case .text: text = name
                case .math: math = name
                case .sans: sans = name
                case .mono: mono = name
                }
            }
        }
    }

    /// The four slots of the table, in the sheet's row order.
    enum Role: String, CaseIterable, Identifiable, Sendable {
        case text, math, sans, mono
        var id: String { rawValue }
        var title: String {
            switch self {
            case .text: "Text"
            case .math: "Math"
            case .sans: "Sans"
            case .mono: "Mono"
            }
        }
        /// What the row sets, for the hint and VoiceOver.
        var explanation: String {
            switch self {
            case .text: "the body text (\\rmdefault); \\setmainfont in the document still wins"
            case .math: "mathematics (\\setmathfont); only families with an OpenType MATH table are listed"
            case .sans: "\\textsf and \\sffamily (\\sfdefault); \\setsansfont in the document still wins"
            case .mono: "\\texttt and \\ttfamily (\\ttdefault); \\setmonofont in the document still wins"
            }
        }
    }

    enum Outcome: Equatable {
        case applied(path: String)
        /// Nothing to write: the table already says this, or "Class default"
        /// everywhere without a manifest.
        case unchanged(String)
        case refused(String)
    }

    /// The sheet is up (ContentView attaches it).
    var shown = false
    var selection = Selection()
    /// The table the sheet opened with; Apply is enabled when the selection differs.
    private(set) var opened = Selection()
    /// The installed families (engine index order), and those with a MATH table.
    private(set) var families: [String] = []
    private(set) var mathFamilies: [String] = []
    private(set) var listing = false
    var applying = false
    /// The last outcome, shown in the sheet and the status line.
    var note: String?
    /// Completed applies (tests).
    private(set) var applies = 0

    /// The sample every row sets in its family.
    static let sampleText = "The quick brown fox — ∫ x² dx"

    /// Test hooks: the family listing (off-main, once per session) and the
    /// helper's rewrite. Nil: `InstalledFonts` and `DocumentFilesState.setFonts`.
    @ObservationIgnored var lister: (@Sendable (URL?) -> (families: [String], math: [String]))?
    @ObservationIgnored var rewriter: ((URL, String, [String: String]) -> Result<ProjectFilesV1.SetFonts, ProjectManifest.Failure>)?
    @ObservationIgnored private unowned let model: ShellModel

    init(model: ShellModel) { self.model = model }

    var canApply: Bool { selection != opened && !applying }

    /// The families the picker of `role` offers: the MATH-table families
    /// for Math when the engine reported any, else the whole list.
    func families(for role: Role) -> [String] {
        role == .math && !mathFamilies.isEmpty ? mathFamilies : families
    }

    /// The menu / palette command: opens the sheet on the current table.
    func present() {
        guard model.project.projectRoot != nil else {
            model.navigationNote = "Save the entry document first (⌘S): the project fonts are written to flashtex.toml next to it."
            return
        }
        note = nil
        selection = Selection(manifest: model.manifest.snapshot.flatMap { model.manifest.snapshotRoot == model.project.projectRoot ? $0.manifest.fonts : nil })
        opened = selection
        shown = true
        listFamilies()
    }

    /// Lists the installed families once per session, off-main; the rows
    /// show what is known so far (nothing until the listing lands).
    func listFamilies() {
        guard families.isEmpty, !listing else { return }
        listing = true
        let tool = ShellModel.locateRenderPipeline()
        let list: @Sendable (URL?) -> (families: [String], math: [String]) = lister ?? { tool in
            (InstalledFonts.families(renderPipeline: tool), InstalledFonts.mathFamilies(renderPipeline: tool))
        }
        Task.detached(priority: .userInitiated) {
            let listed = list(tool)
            await MainActor.run {
                self.families = listed.families
                self.mathFamilies = listed.math
                self.listing = false
            }
        }
    }

    /// Writes the selection as the manifest's `[fonts]` and closes the sheet
    /// (the sheet stays up, with the reason, on a refusal).
    @discardableResult
    func apply() async -> Outcome {
        guard let root = model.project.projectRoot else {
            return note(.refused("the entry document is not saved, so there is no project root"))
        }
        if selection.isClassDefault, !model.manifest.exists {
            shown = false
            return note(.unchanged("Class default everywhere and no flashtex.toml: nothing to write"))
        }
        applying = true
        defer { applying = false }
        let entry = model.project.entryPath
        let rewritten: Result<ProjectFilesV1.SetFonts, ProjectManifest.Failure> =
            rewriter.map { $0(root, entry, selection.byRole) } ?? model.files.setFonts(for: root, entry: entry, fonts: selection.byRole)
        let reply: ProjectFilesV1.SetFonts
        switch rewritten {
        case .success(let r): reply = r
        case .failure(let f): return note(.refused("cannot write \(ProjectManifest.fileName): \(f.why)"))
        }
        guard reply.changed, let text = reply.text else {
            shown = false
            opened = selection
            return note(.unchanged("\(reply.path) already says this"))
        }
        let url = URL(fileURLWithPath: reply.path)
        switch model.files.save(url, text: text, expected: reply.exists ? .any : .newFile, force: false, recordsState: false) {
        case .saved: break
        case .conflict(let c): return note(.refused("cannot write \(ProjectManifest.fileName): \(c.summary)"))
        case .failed(let why): return note(.refused("cannot write \(ProjectManifest.fileName): \(why)"))
        }
        applies += 1
        opened = selection
        shown = false
        // Re-read the table: a change recompiles (ShellModel.manifestFontsDidChange).
        model.manifest.refresh()
        let outcome = note(.applied(path: reply.path))
        model.navigationNote = selection.isClassDefault
            ? "Project fonts: class default (the [fonts] table of \(url.lastPathComponent) is now empty)"
            : "Project fonts written to \(url.lastPathComponent): " + Role.allCases.compactMap { role in selection[role].map { "\(role.title.lowercased()) \($0)" } }.joined(separator: ", ")
        return outcome
    }

    private func note(_ outcome: Outcome) -> Outcome {
        switch outcome {
        case .applied(let p): note = "written to \(p)"
        case .unchanged(let why), .refused(let why): note = why
        }
        FlashTeXLog.write("project fonts: " + (note ?? ""))
        return outcome
    }
}

// MARK: - samples

/// The sample line of the Fonts sheet and of the `\setmainfont{`
/// completion rows: the family's regular face as AppKit resolves it by
/// family name. A family the engine indexes but the app has no registered
/// face for (a project-local `fonts/` file, `FLASHTEX_FONT_DIRS`) has no
/// sample; the row says so rather than showing another font.
enum FontSamples {
    /// The face for `family` at `size`, nil when AppKit does not know the family.
    static func nsFont(family: String, size: CGFloat) -> NSFont? {
        let manager = NSFontManager.shared
        guard manager.availableFontFamilies.contains(where: { $0.caseInsensitiveCompare(family) == .orderedSame }) else {
            // Also a PostScript or full name (`LMRoman10-Regular`).
            return NSFont(name: family, size: size)
        }
        return manager.font(withFamily: family, traits: [], weight: 5, size: size) ?? NSFont(name: family, size: size)
    }

    /// The "Class default" sample: Latin Modern Roman when the bundle's
    /// copy is registered (PreviewFonts), else the system serif.
    static func classDefaultFont(size: CGFloat) -> NSFont {
        if PreviewFonts.latinModernRegistered, let lm = NSFont(name: "LMRoman10-Regular", size: size) { return lm }
        return NSFont(descriptor: NSFontDescriptor.preferredFontDescriptor(forTextStyle: .body).withDesign(.serif) ?? NSFontDescriptor(), size: size) ?? .systemFont(ofSize: size)
    }
}

// MARK: - model attachment

extension ShellModel {
    private static var projectFontsKey = 0
    /// The Fonts sheet state (see `ProjectFontsState`).
    var projectFonts: ProjectFontsState {
        if let existing = objc_getAssociatedObject(self, &Self.projectFontsKey) as? ProjectFontsState { return existing }
        let state = ProjectFontsState(model: self)
        objc_setAssociatedObject(self, &Self.projectFontsKey, state, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return state
    }
}

// MARK: - the sheet

struct ProjectFontsSheet: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        @Bindable var state = model.projectFonts
        let file = model.manifest.snapshot?.path.map { URL(fileURLWithPath: $0).lastPathComponent } ?? ProjectManifest.fileName
        VStack(alignment: .leading, spacing: DS.Space.l) {
            Text("Project Fonts").font(.title2.bold())
            Text("Written to \(file) as its [fonts] table — the families the document's text, mathematics, sans and mono default to. A \\setmainfont, \\setmathfont, \\setsansfont or \\setmonofont in the document still wins.")
                .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            VStack(alignment: .leading, spacing: DS.Space.m) {
                ForEach(ProjectFontsState.Role.allCases) { role in
                    FontRoleRow(role: role, family: $state.selection[role], families: state.families(for: role), listing: state.listing)
                }
            }
            if state.listing {
                HStack(spacing: DS.Space.s) {
                    ProgressView().controlSize(.small)
                    Text("Listing the installed families (flashtex-render --list-fonts)…").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                }
            } else if state.families.isEmpty {
                Text("No families listed: attach or build flashtex-render (crates/render-pipeline) so the index the render resolves against can be asked. A name can still be typed.")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.severityWarning).fixedSize(horizontal: false, vertical: true)
            }
            if let note = state.note {
                Text(note).font(.callout).foregroundStyle(DS.Colors.severityWarning).fixedSize(horizontal: false, vertical: true)
            }
            HStack {
                Button("Class Default Everywhere") { state.selection = ProjectFontsState.Selection() }
                    .disabled(state.selection.isClassDefault)
                    .accessibilityHint("Clears every row; Apply then empties the [fonts] table (or writes nothing when there is no flashtex.toml).")
                Spacer()
                Button("Cancel") { state.shown = false }.keyboardShortcut(.cancelAction)
                Button("Apply") { Task { await state.apply() } }
                    .keyboardShortcut(.defaultAction)
                    .disabled(!state.canApply)
                    .accessibilityIdentifier("project.fonts.apply")
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.sheetWidth)
        .onAppear { state.listFamilies() }
    }
}

/// One row: the role, its picker, and the sample line in the chosen family.
struct FontRoleRow: View {
    let role: ProjectFontsState.Role
    @Binding var family: String?
    let families: [String]
    let listing: Bool

    private static let sampleSize: CGFloat = 15

    var body: some View {
        HStack(alignment: .center, spacing: DS.Space.m) {
            Text(role.title)
                .font(DS.Fonts.base)
                .frame(width: 44, alignment: .leading)
                .accessibilityHidden(true)
            FontFamilyPicker(role: role, family: $family, families: families, listing: listing)
                .frame(width: 200, alignment: .leading)
            sample
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .help(role.explanation)
    }

    @ViewBuilder private var sample: some View {
        if let family {
            if let font = FontSamples.nsFont(family: family, size: Self.sampleSize) {
                Text(ProjectFontsState.sampleText).font(Font(font)).lineLimit(1).truncationMode(.tail)
                    .accessibilityLabel("Sample in \(family)")
            } else {
                Text("no sample: \(family) is not registered with the app (a project or FLASHTEX_FONT_DIRS font renders in the preview all the same)")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary).lineLimit(2)
            }
        } else {
            Text(ProjectFontsState.sampleText).font(Font(FontSamples.classDefaultFont(size: Self.sampleSize))).lineLimit(1).truncationMode(.tail)
                .foregroundStyle(DS.Colors.textSecondary)
                .accessibilityLabel("Sample in the class default")
        }
    }
}

/// A button naming the choice; its popover is a search field over the
/// families with "Class default" first. Return picks the first match.
struct FontFamilyPicker: View {
    let role: ProjectFontsState.Role
    @Binding var family: String?
    let families: [String]
    let listing: Bool
    @State private var shown = false
    @State private var query = ""

    static let classDefault = "Class default"

    var body: some View {
        Button {
            query = ""
            shown.toggle()
        } label: {
            HStack {
                Text(family ?? Self.classDefault).lineLimit(1).truncationMode(.middle)
                    .foregroundStyle(family == nil ? DS.Colors.textSecondary : DS.Colors.textPrimary)
                Spacer(minLength: DS.Space.s)
                // Not DS.Fonts.secondary: a bare disclosure-chevron glyph,
                // same "icon, not text" rule as railIcon/toolbarIcon.
                Image(systemName: "chevron.up.chevron.down").font(.system(size: 11)).foregroundStyle(DS.Colors.textSecondary)
            }
            .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xs)
            .background(DS.Colors.surfaceRaised, in: RoundedRectangle(cornerRadius: DS.Radius.control))
            .overlay(RoundedRectangle(cornerRadius: DS.Radius.control).stroke(DS.Colors.componentBorder))
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(role.title) font family")
        .accessibilityValue(family ?? Self.classDefault)
        .accessibilityHint(role.explanation)
        .accessibilityIdentifier("project.fonts.\(role.rawValue)")
        .popover(isPresented: $shown, arrowEdge: .bottom) { popover }
    }

    /// "Class default" and every family matching the query (every term a
    /// case-insensitive substring), in the engine's order.
    var matches: [String] {
        let terms = query.lowercased().split(whereSeparator: \.isWhitespace).map(String.init)
        let named = families.filter { f in terms.allSatisfy { f.lowercased().contains($0) } }
        return terms.isEmpty || Self.classDefault.lowercased().contains(query.lowercased()) ? [Self.classDefault] + named : named
    }

    private var popover: some View {
        VStack(alignment: .leading, spacing: DS.Space.s) {
            TextField("Search families", text: $query)
                .textFieldStyle(.roundedBorder)
                .font(DS.Fonts.base)
                .accessibilityLabel("Search \(role.title.lowercased()) font families")
                .onSubmit { if let first = matches.first { choose(first) } }
            ScrollViewReader { proxy in
                List(matches, id: \.self, selection: Binding(get: { family ?? Self.classDefault }, set: { if let name = $0 { choose(name) } })) { name in
                    HStack(spacing: DS.Space.m) {
                        Text(name).font(DS.Fonts.base).lineLimit(1).truncationMode(.middle)
                        Spacer()
                        if let font = name == Self.classDefault ? FontSamples.classDefaultFont(size: 12) : FontSamples.nsFont(family: name, size: 12) {
                            Text("Aa ∫x²").font(Font(font)).foregroundStyle(DS.Colors.textSecondary)
                        }
                    }
                    .tag(name)
                    .id(name)
                    .contentShape(Rectangle())
                    .onTapGesture { choose(name) }
                }
                .listStyle(.inset)
                .frame(minHeight: 220)
                .onAppear { proxy.scrollTo(family ?? Self.classDefault, anchor: .center) }
            }
            if families.isEmpty, !listing {
                TextField("Or type a family name", text: Binding(get: { family ?? "" }, set: { family = $0.isEmpty ? nil : $0 }))
                    .textFieldStyle(.roundedBorder)
                    .font(DS.Fonts.base)
                    .accessibilityLabel("Type the \(role.title.lowercased()) font family")
            }
        }
        .padding(DS.Space.m)
        .frame(width: 320)
    }

    private func choose(_ name: String) {
        family = name == Self.classDefault ? nil : name
        shown = false
    }
}
