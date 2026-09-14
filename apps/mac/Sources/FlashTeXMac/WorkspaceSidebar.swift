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
    @State private var expanded: Set<DocumentOutline.Kind> = [.section, .environment, .label]

    static let identifier = "workspace.sidebar"

    var body: some View {
        VStack(spacing: 0) {
            if projectVisible {
                List {
                    ProjectSection()
                }
                .listStyle(.sidebar)
                .scrollContentBackground(.hidden)
            }
            if projectVisible && outlineVisible { Divider() }
            if outlineVisible {
                List {
                    OutlineSection(outline: outline, expanded: $expanded, stale: outlineFor.revision != model.chrome.editorRevision) // throttled (ShellChrome): not per keystroke
                }
                .listStyle(.sidebar)
                .scrollContentBackground(.hidden)
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
                        Image(systemName: Self.icon(for: doc, kind: kinds.kind(of: doc.path)))
                            .foregroundStyle(doc.path == model.activePath ? DS.Colors.accentSelection : DS.Colors.textSecondary)
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

    static func icon(for doc: ProjectDocument, kind: DocumentKind?) -> String {
        if kind == .bibliography { return "books.vertical" }
        if doc.role == .entry { return "doc.text.fill" }
        return "doc.text"
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
    @Binding var expanded: Set<DocumentOutline.Kind>
    let stale: Bool
    /// Follow-caret: the id of the section (else environment) the caret is in
    /// (`DocumentOutline.current`), refreshed by the leaf `CaretFollower` so a
    /// caret move re-evaluates this section only when the current item changes.
    @State private var currentID: String?

    var body: some View {
        let counts = DocumentOutline.counts(outline)
        // Structure depth relative to the document's top level: an article's
        // \section rows sit at depth 0, a report's \chapter rows do.
        let topLevel = outline.filter { $0.kind == .section }.map(\.level).min() ?? 0
        Section {
            CaretFollower(outline: outline, currentID: $currentID)
            if outline.isEmpty {
                Text(stale ? "Scanning…" : "No sections, environments or labels in \(model.activePath)")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
            }
            ForEach(DocumentOutline.Kind.allCases, id: \.self) { kind in
                let items = DocumentOutline.items(kind, in: outline)
                if !items.isEmpty {
                    DisclosureGroup(isExpanded: Binding(get: { expanded.contains(kind) },
                                                        set: { if $0 { expanded.insert(kind) } else { expanded.remove(kind) } })) {
                        ForEach(items) { item in
                            SidebarRow(selected: item.id == currentID) {
                                model.reveal(outlineItem: item)
                            } label: {
                                HStack(spacing: DS.Space.xs) {
                                    Image(systemName: Self.icon(item)).foregroundStyle(DS.Colors.textSecondary).font(DS.Fonts.secondary)
                                    Text(item.displayTitle.isEmpty ? "(untitled)" : item.displayTitle).lineLimit(1)
                                    Spacer(minLength: 0)
                                    Text("\(item.line)").font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textTertiary)
                                }
                                .padding(.leading, CGFloat(min(max(0, item.kind == .section ? item.level - topLevel : item.level), 4)) * DS.Space.m)
                            }
                            .help(Self.tooltip(item))
                            .accessibilityLabel(Self.spoken(item))
                        }
                    } label: {
                        HStack {
                            Text(kind.title)
                            Spacer()
                            Text("\(counts[kind] ?? 0)").font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textTertiary)
                        }
                    }
                }
            }
        } header: {
            HStack {
                Label("Outline", systemImage: "list.bullet.indent")
                Spacer()
                if stale { ProgressView().controlSize(.mini) }
            }
        }
    }

    static func icon(_ item: DocumentOutline.Item) -> String {
        switch item.kind {
        case .section: return item.command == "part" ? "book.closed" : item.level <= 1 ? "number" : "number.square"
        case .environment:
            if DocumentOutline.floatEnvironments.contains(item.title) { return item.title.hasPrefix("table") ? "tablecells" : "photo" }
            return item.caption != nil ? "text.book.closed" : "curlybraces"
        case .label: return "tag"
        }
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

    var body: some View {
        Button(action: action) {
            label()
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .padding(.horizontal, DS.Space.s).padding(.vertical, DS.Space.xxs)
        .background(selected ? DS.Colors.accentSelection.opacity(DS.State.selectionTintOpacity) : Color.clear, in: RoundedRectangle(cornerRadius: DS.Radius.tab))
        .listRowInsets(EdgeInsets(top: DS.Size.hairline, leading: DS.Space.xs, bottom: DS.Size.hairline, trailing: DS.Space.xs))
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
