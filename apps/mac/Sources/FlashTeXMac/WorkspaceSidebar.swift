import SwiftUI
import AppKit
import FlashTeXProtocol

/// The tool column (design-principles §4): tool windows stack sharing this
/// column — the Project tree, and under it the Outline of the active buffer
/// (sections, environments, labels — DocumentOutline.swift), each toggled
/// from the rail (ContentView.swift). The Outline ships collapsed. Problems
/// moved out of this column entirely: its homes are the bottom panel and the
/// status-bar badge.
///
/// Both trees are `NSOutlineView`s (SidebarTree.swift), per the brief's
/// architecture rule; every row still drives an existing model operation:
/// switching goes through `ProjectDocuments.switchDocument`, includes open
/// through `openDocument`, outline rows select through
/// `reveal(outlineItem:)`. Nothing here is a new source of truth.
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
    /// Follow-caret: id of the outline item the caret is in.
    @State private var currentOutlineID: String?

    static let identifier = "workspace.sidebar"

    var body: some View {
        VStack(spacing: 0) {
            if projectVisible {
                ProjectSection()
            }
            if projectVisible && outlineVisible { Divider().overlay(DS.Colors.separator) }
            if outlineVisible {
                OutlineSection(outline: outline,
                                  shownKinds: $shownKinds,
                                  stale: outlineFor.revision != model.chrome.editorRevision,
                                  currentID: currentOutlineID)
            }
        }
        .background(DS.Colors.surfacePrimary) // the flat opaque tool window (Islands), never a sidebar material
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
        .task(id: "\(model.caretUTF16)/\(outline.count)/\(outline.first?.utf16.location ?? -1)") {
            guard outlineVisible else { return }
            try? await Task.sleep(for: .milliseconds(80))
            guard !Task.isCancelled else { return }
            let id = DocumentOutline.current(at: model.caretUTF16, in: outline)?.id
            if id != currentOutlineID { currentOutlineID = id }
        }
    }
}

// MARK: - tool window header

/// The IntelliJ tool-window header: the window's name leading, quiet
/// controls trailing, on the panel's own surface.
struct ToolWindowHeader<Trailing: View>: View {
    let title: String
    @ViewBuilder var trailing: () -> Trailing

    var body: some View {
        HStack(spacing: DS.Space.s) {
            Text(title)
                .font(DS.Fonts.base.weight(.semibold))
                .foregroundStyle(DS.Colors.textPrimary)
            Spacer(minLength: 0)
            trailing()
        }
        .padding(.horizontal, DS.Space.l)
        .frame(height: DS.Row.toolWindowHeader)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("\(title) header")
    }
}

// MARK: - Project

/// Open members (entry first) plus every `\input`/`\include` the entry
/// references that is not open yet (bounded discovery, ProjectDocuments.swift),
/// and creatable missing includes.
private struct ProjectSection: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        // Throttled, change-only copies (ShellChrome.swift): `project.listing`
        // and `discoverClosure()` read `documents`, so this tree re-evaluated
        // on every keystroke otherwise.
        let listing = model.chrome.listing
        let kinds = model.documentKinds
        let closure = model.chrome.closure
        VStack(spacing: 0) {
            ToolWindowHeader(title: "Project") {
                Text("\(listing.count)").font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textTertiary)
                Button { model.scaffold.presentNewFile() } label: { // ProjectScaffoldViews.swift
                    Image(systemName: "plus")
                        .font(DS.Fonts.base)
                        .foregroundStyle(DS.Colors.textSecondary)
                        .frame(width: DS.Size.inlineIconButton, height: DS.Size.inlineIconButton)
                        .contentShape(Rectangle())
                }
                .buttonStyle(PressableStyle(cornerRadius: DS.Radius.control))
                .disabled(model.project.projectRoot == nil)
                .help("New File… (⌘N): a rooted .tex file in this project, opened in a tab")
                .accessibilityLabel("New file")
                .accessibilityIdentifier("project.newfile")
            }
            SidebarTree(rows: Self.rows(listing: listing, kinds: kinds, closure: closure, activePath: model.activePath),
                        selectedID: model.activePath,
                        onSelect: { id in select(id, listing: listing, closure: closure) },
                        menuItems: { id in menu(for: id, listing: listing) },
                        accessibilityLabel: "Project tree")
        }
    }

    /// Row ids: open documents use their path; discovered/missing includes a
    /// prefixed reference so they never collide with an open path.
    static func rows(listing: [ProjectDocument], kinds: DocumentKinds,
                     closure: ProjectDocuments.Closure, activePath: String) -> [SidebarTree.Row] {
        var rows: [SidebarTree.Row] = listing.map { doc in
            let style = FileTypeStyle.of(path: doc.path, entry: doc.role == .entry,
                                         bibliography: kinds.kind(of: doc.path) == .bibliography)
            return SidebarTree.Row(
                id: doc.path,
                icon: style.systemImage,
                iconColor: style.nsColor,
                title: doc.path,
                trailing: doc.durableRevision.map { "r\($0)" },
                modified: doc.isDirty,
                tooltip: tooltip(for: doc, kind: kinds.kind(of: doc.path)),
                accessibilityLabel: spoken(for: doc, kind: kinds.kind(of: doc.path), active: doc.path == activePath))
        }
        for n in closure.nodes where n.state == .available {
            let name = n.resolvedPath ?? n.reference.argument
            rows.append(SidebarTree.Row(
                id: "closed:\(name)",
                icon: "doc.badge.plus",
                iconColor: DS.Palette.textTertiary,
                title: name,
                dimmed: true,
                tooltip: "\\\(n.reference.kind.rawValue){\(n.reference.argument)} from \(n.from) — click to open",
                accessibilityLabel: "\(name), not open, included from \(n.from); activate to open"))
        }
        for n in closure.nodes {
            guard case .unresolvable(let why) = n.state, why.hasPrefix("no such file") else { continue }
            let name = MissingIncludeFix.path(for: n.reference.argument) ?? n.reference.argument
            rows.append(SidebarTree.Row(
                id: "missing:\(n.reference.argument):\(n.from)",
                icon: "doc.badge.plus",
                iconColor: DS.Palette.severityWarning,
                title: "\(name) — missing, create",
                dimmed: true,
                tooltip: "\\\(n.reference.kind.rawValue){\(n.reference.argument)} from \(n.from) has no file — click to create \(name)",
                accessibilityLabel: "\(name), missing, included from \(n.from); activate to create it"))
        }
        return rows
    }

    private func select(_ id: String, listing: [ProjectDocument], closure: ProjectDocuments.Closure) {
        if id.hasPrefix("closed:") {
            let name = String(id.dropFirst("closed:".count))
            let from = closure.nodes.first { ($0.resolvedPath ?? $0.reference.argument) == name && $0.state == .available }?.from ?? model.chrome.entryPath
            Task { await model.project.openDocument(name, role: .included(from: from)) }
        } else if id.hasPrefix("missing:") {
            let parts = id.dropFirst("missing:".count).split(separator: ":", maxSplits: 1).map(String.init)
            guard parts.count == 2 else { return }
            Task { _ = await model.project.createMissingInclude(parts[0], from: parts[1]); model.navigationNote = model.project.status }
        } else {
            model.switchOrNote(id)
        }
    }

    private func menu(for id: String, listing: [ProjectDocument]) -> [SidebarTree.MenuItem] {
        guard let doc = listing.first(where: { $0.path == id }) else {
            return [.init(title: "New File…", action: { model.scaffold.presentNewFile() })]
        }
        var items: [SidebarTree.MenuItem] = [.init(title: "New File…", action: { model.scaffold.presentNewFile() })]
        if doc.role != .entry {
            let path = doc.path
            items.append(.divider)
            items.append(.init(title: "Rename…", action: { model.scaffold.presentRename(path) }))
            items.append(.init(title: "Delete…", action: { model.scaffold.presentDelete(path) }))
        }
        return items
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
    let currentID: String?

    var body: some View {
        let rows = Self.rows(outline: outline, shownKinds: shownKinds, stale: stale, activePath: model.activePath)
        VStack(spacing: 0) {
            ToolWindowHeader(title: "Outline") {
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
                        .font(DS.Fonts.base)
                        .foregroundStyle(DS.Colors.textSecondary)
                }
                .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
                .help("Show or hide sections, environments and labels; the list always keeps document order")
                .accessibilityLabel("Filter outline")
            }
            SidebarTree(rows: rows,
                        selectedID: currentID,
                        onSelect: { id in
                            if let item = outline.first(where: { $0.id == id }) { model.reveal(outlineItem: item) }
                        },
                        accessibilityLabel: "Outline")
        }
    }

    /// One flat list in document order (never grouped by type — for prose
    /// the order is the meaning, §7), indented by nesting: sections by their
    /// sectioning level, environments and labels one step under the section
    /// they follow. Item kinds are told apart by their typed icon.
    static func rows(outline: [DocumentOutline.Item], shownKinds: Set<DocumentOutline.Kind>,
                     stale: Bool, activePath: String) -> [SidebarTree.Row] {
        if outline.isEmpty {
            return [SidebarTree.Row(
                id: "outline:empty", icon: stale ? "clock" : "list.bullet.indent",
                iconColor: DS.Palette.textTertiary,
                title: stale ? "Scanning…" : "No sections in \(activePath)",
                dimmed: true, selectable: false)]
        }
        let topLevel = outline.filter { $0.kind == .section }.map(\.level).min() ?? 0
        var sectionDepth = 0
        var out: [SidebarTree.Row] = []
        for item in outline {
            let indent: Int
            switch item.kind {
            case .section:
                sectionDepth = min(max(0, item.level - topLevel), 4)
                guard shownKinds.contains(.section) else { continue }
                indent = sectionDepth
            case .environment, .label:
                guard shownKinds.contains(item.kind) else { continue }
                indent = min(sectionDepth + 1 + max(0, item.level), 5)
            }
            out.append(SidebarTree.Row(
                id: item.id,
                icon: OutlineItemStyle.icon(item),
                iconColor: OutlineItemStyle.nsColor(item),
                title: item.displayTitle.isEmpty ? "(untitled)" : item.displayTitle,
                trailing: "\(item.line)",
                indent: indent,
                tooltip: tooltip(item),
                accessibilityLabel: spoken(item)))
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

    static func color(_ item: DocumentOutline.Item) -> Color { Color(nsColor: nsColor(item)) }

    static func nsColor(_ item: DocumentOutline.Item) -> NSColor {
        switch item.kind {
        case .section: return DS.Palette.textSecondary
        case .environment:
            if item.title.hasPrefix("table") { return DS.Palette.typeBlue }
            if DocumentOutline.floatEnvironments.contains(item.title) { return DS.Palette.typeGreen }
            if mathEnvironments.contains(item.title) || DocumentOutline.theoremEnvironments.contains(item.title) { return DS.Palette.typePurple }
            return DS.Palette.textSecondary
        case .label: return DS.Palette.typeOrange
        }
    }

    private static let mathEnvironments: Set<String> = ["equation", "equation*", "align", "align*", "gather", "gather*", "multline", "multline*"]
}

extension ShellModel {
    /// Sidebar/tab switching: a refused switch (pending capture insertion,
    /// unknown member) lands in the footer note instead of silently failing.
    func switchOrNote(_ path: String) {
        if case .refused(let why) = project.switchDocument(to: path) { navigationNote = why }
    }
}
