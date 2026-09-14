import SwiftUI
import FlashTeXProtocol

/// Document tabs above the editor (mac-ui-redesign): one tab per open
/// project member in `ShellModel.documents` order (entry first), the active
/// one highlighted, a dot for unsaved edits, the helper's durable revision,
/// and a close control on non-entry members that detaches them for this
/// session (`ProjectDocuments.detachDocument`). Switching goes through
/// `ProjectDocuments.switchDocument` so each document keeps its caret and a
/// pending insertion is never applied to the wrong buffer. The Project menu
/// (open includes, save/detach, bibliography kinds) and the kind/byte
/// indicators sit at the trailing end, where the old header put them.
struct DocumentTabBar: View {
    @Environment(ShellModel.self) var model

    static let identifier = "document.tabs"

    var body: some View {
        HStack(spacing: 0) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: DS.Space.xxs) {
                    ForEach(model.chrome.listing) { doc in // throttled, change-only copy (ShellChrome.swift): `project.listing` reads `documents` per keystroke
                        DocumentTab(doc: doc, active: doc.path == model.activePath, kind: model.documentKinds.kind(of: doc.path))
                    }
                }
                .padding(.horizontal, DS.Space.s)
            }
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Open documents")
            .accessibilityIdentifier(Self.identifier)
            Spacer(minLength: DS.Space.m)
            if model.narrowLayout {
                // The collapsed preview's way back (design-principles §4):
                // the window is too narrow for both columns.
                Toggle(isOn: Binding(get: { model.narrowPreviewShown }, set: { model.narrowPreviewShown = $0 })) {
                    Image(systemName: "doc.richtext")
                }
                .toggleStyle(.button).buttonStyle(.accessoryBar).controlSize(.small)
                .help("Show the preview (the window is too narrow for editor and preview side by side)")
                .accessibilityLabel("Show preview")
            }
            ProjectMenu()
            DocumentKindIndicator() // DocumentKinds.swift: helper-reported bibliography kind, read-only
            if model.documentURL == nil {
                // No file identity yet: the one state the modified dot cannot carry.
                Text("unsaved buffer").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                    .padding(.trailing, DS.Space.m)
            } else {
                Spacer().frame(width: DS.Space.m)
            }
        }
        .frame(height: DS.Row.tab)
        .background(DS.Colors.surfaceSecondary) // the recessed strip the active tab is raised against
    }
}

private struct DocumentTab: View {
    @Environment(ShellModel.self) var model
    let doc: ProjectDocument
    let active: Bool
    let kind: DocumentKind?
    @State private var hovering = false

    var body: some View {
        let style = FileTypeStyle.of(path: doc.path, entry: doc.role == .entry, bibliography: kind == .bibliography)
        HStack(spacing: DS.Space.xs) {
            // Colour-coded file identity, same vocabulary as the tree (§6).
            Image(systemName: style.systemImage)
                .font(DS.Fonts.secondary).foregroundStyle(style.color)
            Text(doc.path).font(DS.Fonts.base).lineLimit(1)
                .foregroundStyle(active ? DS.Colors.textPrimary : DS.Colors.textSecondary)
            if doc.isDirty {
                Circle().fill(DS.Colors.statusModified).frame(width: DS.Size.modifiedDot, height: DS.Size.modifiedDot).accessibilityHidden(true)
            }
            if doc.role != .entry {
                Button {
                    Task {
                        switch await model.project.detachDocument(doc.path) {
                        case .refused(let why): model.captureNote = why
                        case .detached(let path): model.captureNote = "Detached \(path) — " + ProjectDocuments.detachScopeNote
                        }
                    }
                } label: {
                    Image(systemName: "xmark").font(DS.Fonts.header)
                        .foregroundStyle(DS.Colors.textSecondary)
                        .frame(width: DS.Size.inlineIconButton, height: DS.Size.inlineIconButton)
                        .background(hovering ? DS.Colors.textPrimary.opacity(DS.State.pressedOpacity) : .clear, in: RoundedRectangle(cornerRadius: DS.Radius.control))
                }
                .buttonStyle(.plain)
                // Close affordance on hover and on the active tab only (§5).
                .opacity(hovering || active ? 1 : 0)
                .help("Detach \(doc.path) for this session (" + ProjectDocuments.detachScopeNote + ")")
                .accessibilityLabel("Detach \(doc.path)")
            }
        }
        .padding(.horizontal, DS.Space.m)
        .frame(height: DS.Row.tab - DS.Space.xs)
        // Islands treatment (§5): the active tab is a filled, rounded card
        // raised out of the recessed strip; inactive tabs carry no chrome.
        .background(active ? AnyShapeStyle(DS.Colors.surfaceRaised)
                           : hovering ? AnyShapeStyle(DS.Colors.textPrimary.opacity(DS.State.hoverOpacity)) : AnyShapeStyle(.clear),
                    in: RoundedRectangle(cornerRadius: DS.Radius.tab))
        .overlay {
            if active {
                RoundedRectangle(cornerRadius: DS.Radius.tab)
                    .strokeBorder(DS.Colors.separator, lineWidth: DS.Size.hairline)
            }
        }
        .shadow(color: active ? DS.Colors.textPrimary.opacity(DS.State.hoverOpacity) : .clear,
                radius: DS.Space.xxs, y: DS.Size.hairline)
        .contentShape(Rectangle())
        .onTapGesture { model.switchOrNote(doc.path) }
        .onHover { hovering = $0 }
        .contextMenu {
            Button("Show \(doc.path)") { model.switchOrNote(doc.path) }.disabled(active)
            if doc.role != .entry {
                Button("Save \(doc.path)") { Task { await model.project.saveDocument(doc.path) } }
                    .disabled(model.documentURL == nil)
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(doc.path)\(doc.role == .entry ? ", entry" : "")\(doc.isDirty ? ", edited" : "")")
        .accessibilityAddTraits(active ? [.isSelected, .isButton] : .isButton)
        .accessibilityAction { model.switchOrNote(doc.path) }
        .help(Self.tooltip(doc))
    }

    static func tooltip(_ doc: ProjectDocument) -> String {
        var s = doc.path
        switch doc.role {
        case .entry: s += " — entry document"
        case .included(let from): s += " — included from \(from)"
        case .opened: s += " — opened by path"
        }
        if let r = doc.durableRevision { s += " · durable r\(r)" }
        if doc.isDirty { s += " · edited" }
        return s
    }
}

/// Project membership: open the entry document's `\input`/`\include`
/// targets, save or detach the active non-entry document. Discovery runs
/// when the menu opens (bounded lexical scan, ProjectDocuments.swift).
struct ProjectMenu: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        Menu {
            // The transitive closure (chapter → section → …), depth-first in
            // source order, indented by depth; cycles and missing files are
            // listed with their reason. Bounded: 8 levels, 256 documents.
            let closure = model.chrome.closure // throttled copy (ShellChrome): `discoverClosure()` reads the entry text per keystroke
            if closure.nodes.isEmpty {
                Text("No \\input or \\include in \(model.chrome.entryPath)")
            }
            ForEach(Array(closure.nodes.enumerated()), id: \.offset) { _, n in
                let indent = String(repeating: "    ", count: max(0, n.depth))
                let name = n.resolvedPath ?? n.reference.argument
                switch n.state {
                case .available:
                    Button(indent + "Open \(name)") { Task { await model.project.openDocument(name, role: .included(from: n.from)) } }
                case .open:
                    Button(indent + "Show \(name)") { model.project.switchDocument(to: name) }
                case .unresolvable(let why):
                    Text(indent + "\\\(n.reference.kind.rawValue){\(n.reference.argument)}: \(why)")
                }
            }
            if closure.truncated { Text("closure truncated at \(ProjectDocuments.maxClosureDocuments) documents") }
            if closure.nodes.contains(where: { $0.state == .available }) {
                Button("Open All Includes") { Task { await model.project.openDiscoveredIncludes() } }
                    .help("Opens the whole include closure in this order; unresolvable references are reported in the footer note")
            }
            if let report = model.project.lastOpenReport, !report.unresolvable.isEmpty {
                Divider()
                Text("Open All: \(report.unresolvable.count) unresolvable")
                ForEach(Array(report.unresolvable.enumerated()), id: \.offset) { _, line in Text(line) }
            }
            if model.activePath != model.chrome.entryPath {
                Divider()
                Button("Save \(model.activePath)") { Task { await model.project.saveDocument(model.activePath) } }
                    .disabled(model.documentURL == nil)
                Button("Detach \(model.activePath) (this session)") {
                    Task {
                        switch await model.project.detachDocument(model.activePath) {
                        case .refused(let why): model.captureNote = why
                        case .detached(let path): model.captureNote = "Detached \(path) — " + ProjectDocuments.detachScopeNote
                        }
                    }
                }
                .help("Session only: " + ProjectDocuments.detachScopeNote)
            }
            DocumentKindsMenuSection() // DocumentKinds.swift: declare/undeclare bibliography sources
        } label: {
            Label("Project", systemImage: "doc.on.doc")
        }
        .menuStyle(.borderlessButton).fixedSize()
        .help(model.project.status)
    }
}
