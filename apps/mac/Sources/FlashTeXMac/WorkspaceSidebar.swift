import SwiftUI
import FlashTeXProtocol

/// The tool column (design-principles §4): tool windows stack sharing this
/// column — the Project tree, and under it the Outline of the active buffer
/// (sections, environments, labels — DocumentOutline.swift), each toggled
/// from the rail (ContentView.swift). The Outline ships collapsed. Problems
/// moved out of this column entirely: its homes are the bottom panel and the
/// status-bar badge. Every row drives an existing model operation: switching
/// goes through `ProjectDocuments.switchDocument`, includes open through
/// `openDocument`, outline rows select through `reveal(outlineItem:)`.
/// Nothing here is a new source of truth.
struct WorkspaceSidebar: View {
    @Environment(ShellModel.self) var model
    let projectVisible: Bool
    let outlineVisible: Bool
    /// Outline of the active buffer, rescanned ~150 ms after edits settle so
    /// a keystroke never pays for a scan on its own frame (TypingBench).
    @State private var outline: [DocumentOutline.Item] = []
    @State private var outlineFor: (path: String, revision: Int) = ("", -1)
    /// The header's filter (D1's in-header "Show" menu): which item kinds the
    /// outline lists. Document order is never regrouped — for prose the order
    /// is the meaning (design-principles §7).
    @State private var shownKinds: Set<DocumentOutline.Kind> = Set(DocumentOutline.Kind.allCases)

    static let identifier = "workspace.sidebar"

    var body: some View {
        VStack(spacing: 0) {
            if projectVisible {
                List {
                    ProjectSection()
                }
                .listStyle(.sidebar)
                .scrollContentBackground(.hidden)
                .environment(\.defaultMinListRowHeight, DS.Row.tree)
            }
            if projectVisible && outlineVisible { Divider() }
            if outlineVisible {
                List {
                    OutlineSection(outline: outline, shownKinds: $shownKinds, stale: outlineFor.revision != model.chrome.editorRevision) // throttled (ShellChrome): not per keystroke
                }
                .listStyle(.sidebar)
                .scrollContentBackground(.hidden)
                .environment(\.defaultMinListRowHeight, DS.Row.outline)
            }
        }
        .background(DS.Colors.surfaceSecondary)
        .accessibilityIdentifier(Self.identifier)
        .modifier(ProjectScaffoldSheets()) // New Project / New File / Rename / Delete (ProjectScaffoldViews.swift)
        .task(id: "\(model.activePath)@\(model.chrome.editorRevision)/\(outlineVisible)") {
            guard outlineVisible else { return } // no scans for a hidden panel
            // Rescan after a short quiet period; the previous scan is cancelled.
            let revision = model.chrome.editorRevision, path = model.activePath
            if outlineFor.revision >= 0 { try? await Task.sleep(for: .milliseconds(150)) }
            guard !Task.isCancelled else { return }
            outline = model.outline
            outlineFor = (path, revision)
        }
    }
}

// MARK: - Project

/// Open members (entry first) plus every `\input`/`\include` the entry
/// references that is not open yet (bounded discovery, ProjectDocuments.swift).
private struct ProjectSection: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        // Throttled, change-only copies (ShellChrome.swift): `project.listing`
        // and `discoverClosure()` read `documents`, so this List (an AppKit
        // outline view) re-evaluated on every keystroke.
        let listing = model.chrome.listing
        let kinds = model.documentKinds
        let closure = model.chrome.closure
        Section {
            ForEach(listing) { doc in
                SidebarRow(selected: doc.path == model.activePath) {
                    model.switchOrNote(doc.path)
                } label: {
                    Label {
                        HStack(spacing: DS.Space.xs) {
                            Text(doc.path).lineLimit(1).truncationMode(.middle)
                            if doc.isDirty { Circle().fill(DS.Colors.statusModified).frame(width: DS.Size.modifiedDot, height: DS.Size.modifiedDot).accessibilityLabel("edited") }
                            Spacer(minLength: 0)
                            if let r = doc.durableRevision { Text("r\(r)").font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textTertiary) }
                        }
                    } icon: {
                        let style = FileTypeStyle.of(path: doc.path, entry: doc.role == .entry,
                                                     bibliography: kinds.kind(of: doc.path) == .bibliography)
                        Image(systemName: style.systemImage)
                            .foregroundStyle(style.color)
                    }
                }
                .help(Self.tooltip(for: doc, kind: kinds.kind(of: doc.path)))
                .accessibilityLabel(Self.spoken(for: doc, kind: kinds.kind(of: doc.path), active: doc.path == model.activePath))
                .contextMenu { // ProjectScaffoldViews.swift
                    Button("New File…") { model.scaffold.presentNewFile() }
                    if doc.role != .entry {
                        Divider()
                        Button("Rename…") { model.scaffold.presentRename(doc.path) }
                        Button("Delete…") { model.scaffold.presentDelete(doc.path) }
                    }
                }
            }
            let closed = closure.nodes.filter { $0.state == .available }
            ForEach(Array(closed.enumerated()), id: \.offset) { _, n in
                let name = n.resolvedPath ?? n.reference.argument
                SidebarRow(selected: false) {
                    Task { await model.project.openDocument(name, role: .included(from: n.from)) }
                } label: {
                    Label {
                        Text(name).lineLimit(1).truncationMode(.middle).foregroundStyle(.secondary)
                    } icon: { Image(systemName: "doc.badge.plus").foregroundStyle(.tertiary) }
                }
                .help("\\\(n.reference.kind.rawValue){\(n.reference.argument)} from \(n.from) — click to open")
                .accessibilityLabel("\(name), not open, included from \(n.from); activate to open")
            }
            // A literal, rooted reference with no file behind it: one click creates it (ProjectScaffold.swift).
            let missing = closure.nodes.filter { if case .unresolvable(let why) = $0.state { return why.hasPrefix("no such file") } else { return false } }
            ForEach(Array(missing.enumerated()), id: \.offset) { _, n in
                let name = MissingIncludeFix.path(for: n.reference.argument) ?? n.reference.argument
                SidebarRow(selected: false) {
                    Task { await model.project.createMissingInclude(n.reference.argument, from: n.from); model.navigationNote = model.project.status }
                } label: {
                    Label {
                        HStack(spacing: DS.Space.xs) {
                            Text(name).lineLimit(1).truncationMode(.middle).foregroundStyle(DS.Colors.textSecondary)
                            Text("missing — create").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.severityWarning)
                        }
                    } icon: { Image(systemName: "doc.badge.plus").foregroundStyle(DS.Colors.severityWarning) }
                }
                .help("\\\(n.reference.kind.rawValue){\(n.reference.argument)} from \(n.from) has no file — click to create \(name)")
                .accessibilityLabel("\(name), missing, included from \(n.from); activate to create it")
            }
        } header: {
            HStack {
                Label("Project", systemImage: "folder")
                Spacer()
                Text("\(listing.count)").font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textTertiary)
                Button { model.scaffold.presentNewFile() } label: { Image(systemName: "plus") } // ProjectScaffoldViews.swift
                    .buttonStyle(.plain).foregroundStyle(DS.Colors.textSecondary)
                    .disabled(model.project.projectRoot == nil)
                    .help("New File… (⌘N): a rooted .tex file in this project, opened in a tab")
                    .accessibilityLabel("New file")
                    .accessibilityIdentifier("project.newfile")
            }
        }
    }

    static func tooltip(for doc: ProjectDocument, kind: DocumentKind?) -> String {
        var parts = [doc.path]
        switch doc.role {
        case .entry: parts.append("entry document")
        case .included(let from): parts.append("included from \(from)")
        case .opened: parts.append("opened by path")
        }
        if kind == .bibliography { parts.append("bibliography source (declared)") }
        switch doc.origin {
        case .buffer: parts.append("buffer")
        case .helper: parts.append("via the helper's durable ledger")
        case .disk: parts.append("read from disk")
        }
        if let r = doc.durableRevision { parts.append("durable revision \(r)") }
        if doc.isDirty { parts.append("edited since last save") }
        return parts.joined(separator: " · ")
    }

    static func spoken(for doc: ProjectDocument, kind: DocumentKind?, active: Bool) -> String {
        var s = doc.path
        if doc.role == .entry { s += ", entry" }
        if kind == .bibliography { s += ", bibliography" }
        if doc.isDirty { s += ", edited" }
        if active { s += ", active" }
        return s
    }
}

// MARK: - Outline

private struct OutlineSection: View {
    @Environment(ShellModel.self) var model
    let outline: [DocumentOutline.Item]
    @Binding var shownKinds: Set<DocumentOutline.Kind>
    let stale: Bool
    /// Follow-caret: the id of the section (else environment) the caret is in
    /// (`DocumentOutline.current`), refreshed by the leaf `CaretFollower` so a
    /// caret move re-evaluates this section only when the current item changes.
    @State private var currentID: String?

    var body: some View {
        // One flat list in document order (never grouped by type — for prose
        // the order is the meaning, §7), indented by nesting: sections by
        // their sectioning level, environments and labels one step under the
        // section they follow. Item kinds are told apart by their typed icon.
        let rows = Self.rows(outline: outline, shownKinds: shownKinds)
        Section {
            CaretFollower(outline: outline, currentID: $currentID)
            if outline.isEmpty {
                Text(stale ? "Scanning…" : "No sections, environments or labels in \(model.activePath)")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
            }
            ForEach(rows, id: \.item.id) { row in
                SidebarRow(selected: row.item.id == currentID) {
                    model.reveal(outlineItem: row.item)
                } label: {
                    HStack(spacing: DS.Space.xs) {
                        Image(systemName: OutlineItemStyle.icon(row.item))
                            .foregroundStyle(OutlineItemStyle.color(row.item))
                            .font(DS.Fonts.secondary)
                            .frame(width: DS.Size.fileIcon)
                        Text(row.item.displayTitle.isEmpty ? "(untitled)" : row.item.displayTitle).lineLimit(1)
                        Spacer(minLength: 0)
                        Text("\(row.item.line)").font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textTertiary)
                    }
                    .padding(.leading, CGFloat(row.indent) * DS.Space.m)
                }
                .help(Self.tooltip(row.item))
                .accessibilityLabel(Self.spoken(row.item))
            }
        } header: {
            HStack {
                Label("Outline", systemImage: "list.bullet.indent")
                Spacer()
                if stale { ProgressView().controlSize(.mini) }
                // D1's in-header filter: which kinds are shown; order is
                // always the document's own.
                Menu {
                    ForEach(DocumentOutline.Kind.allCases, id: \.self) { kind in
                        Toggle(kind.title, isOn: Binding(
                            get: { shownKinds.contains(kind) },
                            set: { if $0 { shownKinds.insert(kind) } else { shownKinds.remove(kind) } }))
                    }
                } label: {
                    Image(systemName: shownKinds.count == DocumentOutline.Kind.allCases.count ? "line.3.horizontal.decrease.circle" : "line.3.horizontal.decrease.circle.fill")
                }
                .menuStyle(.borderlessButton).fixedSize()
                .help("Show or hide sections, environments and labels; the list always keeps document order")
                .accessibilityLabel("Filter outline")
            }
        }
    }

    struct Row: Equatable { var item: DocumentOutline.Item; var indent: Int }

    /// Rows in document order with their indent: a section sits at its level
    /// relative to the document's top sectioning level; an environment or
    /// label sits one step under the section it follows (plus its own
    /// nesting), so the tree reads like the document.
    static func rows(outline: [DocumentOutline.Item], shownKinds: Set<DocumentOutline.Kind>) -> [Row] {
        let topLevel = outline.filter { $0.kind == .section }.map(\.level).min() ?? 0
        var sectionDepth = 0
        var out: [Row] = []
        for item in outline {
            switch item.kind {
            case .section:
                sectionDepth = min(max(0, item.level - topLevel), 4)
                if shownKinds.contains(.section) { out.append(Row(item: item, indent: sectionDepth)) }
            case .environment, .label:
                if shownKinds.contains(item.kind) {
                    out.append(Row(item: item, indent: min(sectionDepth + 1 + max(0, item.level), 5)))
                }
            }
        }
        return out
    }

    static func tooltip(_ item: DocumentOutline.Item) -> String {
        switch item.kind {
        case .section: return "\\\(item.command){\(item.title)} — line \(item.line)"
        case .environment: return "\\begin{\(item.title)}" + (item.caption.map { " “\($0)”" } ?? "") + " — line \(item.line)"
        case .label: return "\\label{\(item.title)} — line \(item.line)"
        }
    }

    static func spoken(_ item: DocumentOutline.Item) -> String {
        switch item.kind {
        case .section: return "\(item.command) \(item.title), line \(item.line)"
        case .environment: return "environment \(item.title)" + (item.caption.map { ", \($0)" } ?? "") + ", line \(item.line)"
        case .label: return "label \(item.title), line \(item.line)"
        }
    }
}

/// Typed-icon vocabulary for outline items, shared by the Outline tool
/// window and the palette's Sections/Labels scopes: the glyph tells the
/// kind, the colour its *type identity* (floats green/blue, math purple,
/// labels orange, structure neutral) — never decoration.
enum OutlineItemStyle {
    static func icon(_ item: DocumentOutline.Item) -> String {
        switch item.kind {
        case .section: return item.command == "part" ? "book.closed" : item.level <= 1 ? "number" : "number.square"
        case .environment:
            if DocumentOutline.floatEnvironments.contains(item.title) { return item.title.hasPrefix("table") ? "tablecells" : "photo" }
            if DocumentOutline.theoremEnvironments.contains(item.title) { return "text.book.closed" }
            if mathEnvironments.contains(item.title) { return "function" }
            return "curlybraces"
        case .label: return "tag"
        }
    }

    static func color(_ item: DocumentOutline.Item) -> Color {
        switch item.kind {
        case .section: return DS.Colors.textSecondary
        case .environment:
            if item.title.hasPrefix("table") { return DS.Colors.typeTable }
            if DocumentOutline.floatEnvironments.contains(item.title) { return DS.Colors.typeFloat }
            if mathEnvironments.contains(item.title) || DocumentOutline.theoremEnvironments.contains(item.title) { return DS.Colors.typeMath }
            return DS.Colors.textSecondary
        case .label: return DS.Colors.typeLabel
        }
    }

    private static let mathEnvironments: Set<String> = ["equation", "equation*", "align", "align*", "gather", "gather*", "multline", "multline*"]
}

/// Leaf view that tracks the caret (a debounced `.task(id:)` on
/// `model.caretUTF16`) and publishes the current outline item's id, so the
/// outline rows re-evaluate only when the current item actually changes.
private struct CaretFollower: View {
    @Environment(ShellModel.self) var model
    let outline: [DocumentOutline.Item]
    @Binding var currentID: String?

    var body: some View {
        EmptyView()
            .task(id: "\(model.caretUTF16)/\(outline.count)/\(outline.first?.utf16.location ?? -1)") {
                try? await Task.sleep(for: .milliseconds(80))
                guard !Task.isCancelled else { return }
                let id = DocumentOutline.current(at: model.caretUTF16, in: outline)?.id
                if id != currentID { currentID = id }
            }
    }
}

// MARK: - Row

/// A sidebar row that is a button (keyboard + VoiceOver activation) and
/// draws the selected state like a `List` selection.
struct SidebarRow<Label: View>: View {
    let selected: Bool
    let action: () -> Void
    @ViewBuilder let label: () -> Label
    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            label()
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .padding(.horizontal, DS.Space.s)
        .frame(height: DS.Row.tree)
        // Full-row highlight, hover and selected (design-principles §6).
        .background(selected ? DS.Colors.accentSelection.opacity(DS.State.selectionTintOpacity)
                             : hovering ? DS.Colors.textPrimary.opacity(DS.State.hoverOpacity) : Color.clear,
                    in: RoundedRectangle(cornerRadius: DS.Radius.tab))
        .onHover { hovering = $0 }
        .listRowInsets(EdgeInsets(top: 0, leading: DS.Space.xs, bottom: 0, trailing: DS.Space.xs))
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

extension ShellModel {
    /// Sidebar/tab switching: a refused switch (pending capture insertion,
    /// unknown member) lands in the footer note instead of silently failing.
    func switchOrNote(_ path: String) {
        if case .refused(let why) = project.switchDocument(to: path) { navigationNote = why }
    }
}
