import AppKit
import SwiftUI
import FlashTeXProtocol

/// Sheet state for the editor navigation commands (EditorNavigation.swift):
/// Rename Symbol…, Wrap Selection in Environment…, Go to Symbol….
struct EditorNavigationState: Equatable {
    var renameShown = false
    var renameSymbol: EditorNavigation.Symbol?
    var renameNewName = ""
    var renamePlan: EditorNavigation.RenamePlan?
    var renameStatus = ""
    var wrapShown = false
    var symbolPickerShown = false
    var changeShown = false
    var changeName = ""
    /// VoiceOver announcements posted for a refused Change Environment (tests).
    var changeAnnouncements: [String] = []
    /// Evidence for tests: rename plans applied (label, count).
    var renamesApplied: [String] = []
}

/// One entry of the Go to Symbol picker: an outline item of one document.
struct SymbolPickerEntry: Identifiable, Equatable {
    var path: String
    var item: DocumentOutline.Item
    var score: Int
    var id: String { path + "#" + item.id }
}

extension ShellModel {
    // MARK: environments

    /// Selects `ns` in the active document (the same `Selection` token the
    /// preview click uses, so the editor scrolls to it and keeps the keyboard).
    func selectInEditor(_ ns: NSRange, path: String? = nil) {
        if let path, path != activePath, case .refused(let why) = project.switchDocument(to: path) { navigationNote = why; return }
        selection = .init(path: activePath, nsRange: ns, token: (selection?.token ?? 0) + 1)
        caretUTF16 = ns.location
        caretLengthUTF16 = ns.length
    }

    /// ⌘⇧A: select the innermost environment around the caret; when the
    /// selection already is a whole environment, its parent.
    func selectEnvironment() {
        let text = activeText as NSString
        let sel = NSRange(location: min(caretUTF16, text.length), length: min(caretLengthUTF16, text.length - min(caretUTF16, text.length)))
        guard let pair = EditorNavigation.selectEnvironment(around: sel, in: text) else {
            navigationNote = sel.length > 0 && EditorNavigation.enclosingEnvironment(at: sel.location, in: text) != nil
                ? "The selection is the outermost environment." : "Caret is not inside an environment."
            return
        }
        let whole = pair.whole(limit: text.length)
        selectInEditor(whole)
        let lines = SourceEditorView.lineColumn(text: activeText, utf16: whole.location)?.line ?? 0
        let end = SourceEditorView.lineColumn(text: activeText, utf16: max(whole.location, NSMaxRange(whole) - 1))?.line ?? lines
        navigationNote = "Selected \\begin{\(pair.name)}…\\end{\(pair.name)}" + (pair.end == nil ? " (unclosed)" : "") + " (lines \(lines)–\(end))."
    }

    /// ⌘⇧W: wrap the selection in `env` — one undoable edit through the
    /// pending-edit path, then the caret at the body start.
    func wrapSelection(inEnvironment env: String) {
        let name = env.trimmingCharacters(in: .whitespaces)
        guard !name.isEmpty, name.allSatisfy({ $0.isLetter || $0 == "*" }) else { navigationNote = "“\(env)” is not an environment name."; return }
        let text = activeText as NSString
        let sel = NSRange(location: min(caretUTF16, text.length), length: min(caretLengthUTF16, text.length - min(caretUTF16, text.length)))
        let wrap = EditorNavigation.wrap(selection: sel, in: text, environment: name, indentUnit: EditorPreferences.shared.indentString)
        pendingEdit = .init(path: activePath, nsRange: wrap.range, text: wrap.replacement, token: nextEditToken(), revision: editorRevision)
        // Applied after the edit lands (the view applies the pending edit first, then the newest selection).
        selection = .init(path: activePath, nsRange: wrap.selection, token: (selection?.token ?? 0) + 1)
        editorNavigation.wrapShown = false
        navigationNote = "Wrapped in \\begin{\(name)}…\\end{\(name)} (undo with ⌘Z)."
    }

    // MARK: rename symbol

    /// ⌥⇧R: the symbol under the caret becomes the rename sheet's subject.
    func presentRenameSymbol() {
        let text = activeText as NSString
        guard let symbol = EditorNavigation.symbol(at: min(caretUTF16, text.length), in: text) else {
            navigationNote = "Caret is not on a \\label/\\ref key or a command."; return
        }
        if case .command(let name) = symbol, definition(ofCommand: name) == nil {
            navigationNote = "\\\(name) has no \\newcommand/\\def definition in the project; only user-defined commands are renamed."; return
        }
        editorNavigation.renameSymbol = symbol
        editorNavigation.renameNewName = { if case .command(let n) = symbol { return n } else if case .label(let k) = symbol { return k }; return "" }()
        editorNavigation.renamePlan = nil
        editorNavigation.renameStatus = ""
        editorNavigation.renameShown = true
    }

    /// Open documents in project order, the active one first.
    private var renameDocuments: [(path: String, text: String)] {
        let active = documents.filter { $0.path == activePath }.map { ($0.path, $0.text) }
        return active + documents.filter { $0.path != activePath }.map { ($0.path, $0.text) }
    }

    /// Plans the rename over the open documents (nothing is changed).
    func planRenameSymbol() {
        guard let symbol = editorNavigation.renameSymbol else { return }
        let newName = editorNavigation.renameNewName.trimmingCharacters(in: .whitespaces)
        if let why = EditorNavigation.nameProblem(newName, for: symbol) { editorNavigation.renameStatus = "Refused: \(why)."; editorNavigation.renamePlan = nil; return }
        let normalized = newName.hasPrefix("\\") ? String(newName.dropFirst()) : newName
        let unchanged: Bool = { if case .command(let n) = symbol { return n == normalized }; if case .label(let k) = symbol { return k == normalized }; return false }()
        if unchanged { editorNavigation.renameStatus = "The new name is the current name."; editorNavigation.renamePlan = nil; return }
        let target: EditorNavigation.Symbol = { if case .label = symbol { return .label(normalized) } else { return .command(normalized) } }()
        let collision = renameDocuments.contains { !EditorNavigation.occurrences(of: target, in: $0.text as NSString).isEmpty }
        if collision { editorNavigation.renameStatus = "Refused: \(target.displayName) already exists in the project."; editorNavigation.renamePlan = nil; return }
        guard let plan = EditorNavigation.renamePlan(symbol, to: normalized, in: renameDocuments) else {
            editorNavigation.renameStatus = "No occurrence of \(symbol.displayName) in the open documents."; editorNavigation.renamePlan = nil; return
        }
        editorNavigation.renamePlan = plan
        let route = controllerAttached && controllerState.ready ? "one guarded apply_group per file through the helper" : "one undoable edit per open document"
        editorNavigation.renameStatus = "Proposal: \(plan.summary) — \(route). Nothing is changed until you click Apply."
    }

    /// Apply after review: with the durable helper ready, the shared
    /// reviewed-application core (fresh snapshot, per-file guarded
    /// `apply_group`, ledger undo); otherwise one grouped undoable edit in
    /// the active buffer and a direct replacement in the other open buffers.
    func applyRenameSymbol() async {
        guard let plan = editorNavigation.renamePlan else { return }
        let label = "Rename \(plan.symbol.displayName) to \(plan.newName)"
        if controllerAttached, controllerState.ready {
            let engine = ProjectSearchClient(model: self)
            guard let snapReply = await engine.controllerSnapshot() else { editorNavigation.renameStatus = ProjectSearch.noHelperMessage; return }
            let snap: ProjectSearchClient.Snapshot
            switch snapReply {
            case .failure(let e): editorNavigation.renameStatus = "Snapshot refused: \(e.message)"; return
            case .success(let s): snap = s
            }
            var edits: [ProjectSearch.ReplacementEdit] = []
            for doc in plan.documents {
                guard let revision = snap.versions[doc.path], let text = documents.first(where: { $0.path == doc.path })?.text else {
                    editorNavigation.renameStatus = "\(doc.path) is not part of the helper's project; nothing applied."; return
                }
                for r in doc.ranges {
                    guard let start = CaretSync.byteOffset(ofCaretUTF16: r.location, in: text), let end = CaretSync.byteOffset(ofCaretUTF16: NSMaxRange(r), in: text) else {
                        editorNavigation.renameStatus = "A span in \(doc.path) is not valid; plan again."; return
                    }
                    edits.append(.init(path: doc.path, revision: revision, start: start, end: end,
                                       expectedText: (text as NSString).substring(with: r), replacement: doc.replacement))
                }
            }
            switch await engine.applyReviewedEdits(edits, sourceVersions: snap.versions, membershipGeneration: snap.generation, label: label, commandPrefix: "symbol-rename") {
            case .failure(let e): editorNavigation.renameStatus = e.message; return
            case .success(let outcomes):
                let applied = outcomes.filter { if case .applied = $0.state { return true } else { return false } }.count
                editorNavigation.renameStatus = "\(label): \(applied) of \(outcomes.count) file\(outcomes.count == 1 ? "" : "s") applied"
                    + (applied < outcomes.count ? " — " + outcomes.compactMap { if case .refused(let why) = $0.state { return "\($0.path): \(why)" } else { return nil } }.joined(separator: "; ") : "") + "."
                if applied > 0 { await engine.settleAfterApply() }
            }
        } else {
            var applied = 0
            for doc in plan.documents {
                guard let i = documents.firstIndex(where: { $0.path == doc.path }) else { continue }
                let text = documents[i].text as NSString
                if doc.path == activePath {
                    guard let g = doc.grouped(in: text), pendingEdit == nil else { continue }
                    pendingEdit = .init(path: activePath, nsRange: g.range, text: g.text, token: nextEditToken(), revision: editorRevision)
                    applied += 1
                } else if let new = doc.applied(to: text) {
                    documents[i].text = new // an open non-active buffer (no helper: nothing durable to reconcile; the next compile sends it)
                    applied += 1
                }
            }
            editorNavigation.renameStatus = "\(label): \(applied) of \(plan.documents.count) buffer\(plan.documents.count == 1 ? "" : "s") applied (⌘Z undoes the active document's edit)."
        }
        editorNavigation.renamesApplied.append("\(label): \(plan.count)")
        editorNavigation.renamePlan = nil
        editorNavigation.renameShown = false
        navigationNote = editorNavigation.renameStatus
    }

    // MARK: go to definition

    /// The first `\newcommand`/`\def`/… of `\name` in the open documents (active first).
    func definition(ofCommand name: String, environment: Bool = false) -> (path: String, definition: EditorNavigation.Definition)? {
        for doc in renameDocuments {
            if let d = EditorNavigation.definition(of: name, in: doc.text as NSString, environment: environment) { return (doc.path, d) }
        }
        return nil
    }

    /// ⌃⌘J / ⌘-click on `\foo`: select its definition (switching documents
    /// when it lives in another open one); labels, citations, files and
    /// environments keep their existing routes (`goToMatching`).
    func goToDefinition() {
        let text = activeText as NSString
        let caret = min(caretUTF16, text.length)
        switch EditorIntelligence.definitionTarget(in: text, at: caret) {
        case .command(let name)?: goToDefinition(ofCommand: name)
        case .environment(let name)?:
            if let hit = definition(ofCommand: name, environment: true) { reveal(hit) } else { goToMatching() }
        case .label?, .citation?: goToMatching()
        case .file(let path, _)?: Task { await project.openDocument(path, role: .opened) }
        case nil: navigationNote = "Caret is not on a command, reference or file."
        }
    }

    func goToDefinition(ofCommand name: String) {
        guard let hit = definition(ofCommand: name) else {
            navigationNote = "\\\(name) has no \\newcommand/\\def/\\DeclareMathOperator definition in the open documents" + (EditorIntelligence.CommandDocs.documentation(for: name) != nil ? " (a standard command)." : ".")
            return
        }
        reveal(hit)
    }

    private func reveal(_ hit: (path: String, definition: EditorNavigation.Definition)) {
        selectInEditor(hit.definition.range, path: hit.path)
        navigationNote = "Definition: \(hit.definition.summary) at line \(hit.definition.line)" + (hit.path == activePath ? "." : " in \(hit.path).")
    }

    // MARK: go to symbol

    /// Outline items of every open document ranked by `query` (all of them,
    /// in document order, for the empty query).
    func symbolPickerEntries(query: String) -> [SymbolPickerEntry] {
        var out: [SymbolPickerEntry] = []
        for doc in renameDocuments {
            for item in DocumentOutline.scan(doc.text) {
                guard let score = EditorNavigation.fuzzyScore(query, in: item.title) ?? (query.isEmpty ? 0 : EditorNavigation.fuzzyScore(query, in: item.command)) else { continue }
                out.append(SymbolPickerEntry(path: doc.path, item: item, score: score))
            }
        }
        return query.isEmpty ? out : out.enumerated().sorted { ($1.element.score, $0.offset) < ($0.element.score, $1.offset) }.map(\.element)
    }

    func goToSymbol(_ entry: SymbolPickerEntry) {
        editorNavigation.symbolPickerShown = false
        reveal(outlineItem: entry.item, in: entry.path)
    }

    // MARK: hover peek

    /// The user's own definition of `\name`, for the hover (EditorIntelligence quick info).
    func definitionSummary(forCommand name: String) -> String? {
        guard let hit = definition(ofCommand: name) else { return nil }
        return hit.definition.summary + " (line \(hit.definition.line)" + (hit.path == activePath ? ")" : " in \(hit.path))")
    }
}

// MARK: - sheets

/// The three sheets, attached to the main window with one modifier (ContentView.swift).
struct EditorNavigationSheets: ViewModifier {
    @Environment(ShellModel.self) var model

    func body(content: Content) -> some View {
        @Bindable var model = model
        content
            .sheet(isPresented: $model.editorNavigation.renameShown) { RenameSymbolSheet().environment(model) }
            .sheet(isPresented: $model.editorNavigation.wrapShown) { WrapEnvironmentSheet().environment(model) }
            .sheet(isPresented: $model.editorNavigation.changeShown) { ChangeEnvironmentSheet().environment(model) }
            .sheet(isPresented: $model.editorNavigation.symbolPickerShown) { SymbolPickerSheet().environment(model) }
    }
}

/// Rename Symbol: the symbol, the new name, Plan (review per file) and Apply.
struct RenameSymbolSheet: View {
    @Environment(ShellModel.self) var model
    @FocusState private var focused: Bool
    static let identifier = "rename.symbol"

    var body: some View {
        @Bindable var model = model
        let state = model.editorNavigation
        VStack(alignment: .leading, spacing: DS.Space.m) {
            Text("Rename \(state.renameSymbol?.displayName ?? "symbol")").font(.headline)
            HStack {
                Text(state.renameSymbol.map { if case .command = $0 { return "\\" } else { return "" } } ?? "").font(.body.monospaced()).foregroundStyle(.secondary)
                TextField("New name", text: $model.editorNavigation.renameNewName)
                    .textFieldStyle(.roundedBorder).font(.body.monospaced())
                    .focused($focused)
                    .accessibilityLabel("New name")
                    .onSubmit { if state.renamePlan == nil { model.planRenameSymbol() } else { Task { await model.applyRenameSymbol() } } }
                    .onKeyPress(.escape) { model.editorNavigation.renameShown = false; return .handled }
            }
            if let plan = state.renamePlan {
                List {
                    ForEach(plan.documents, id: \.path) { doc in
                        HStack {
                            Text(doc.path).font(.body.monospaced())
                            Spacer()
                            Text("\(doc.ranges.count) occurrence\(doc.ranges.count == 1 ? "" : "s")").foregroundStyle(.secondary)
                        }
                    }
                }
                .frame(minHeight: DS.Layout.diagnosticsListMinHeight, maxHeight: DS.Layout.sheetListMaxHeight)
                .accessibilityLabel("Rename plan: \(plan.summary)")
            }
            Text(state.renameStatus).font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier(Self.identifier + ".status")
            HStack {
                Spacer()
                Button("Cancel") { model.editorNavigation.renameShown = false }.keyboardShortcut(.cancelAction)
                Button("Plan Rename") { model.planRenameSymbol() }
                Button("Apply") { Task { await model.applyRenameSymbol() } }
                    .keyboardShortcut(.defaultAction)
                    .disabled(state.renamePlan == nil)
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.sheetWidth)
        .onAppear { focused = true }
        .accessibilityIdentifier(Self.identifier)
    }
}

/// Wrap Selection in Environment: an environment name with completion-like
/// suggestions (the common ones first, then every environment the document uses).
struct WrapEnvironmentSheet: View {
    @Environment(ShellModel.self) var model
    @State private var name = ""
    @FocusState private var focused: Bool
    static let identifier = "wrap.environment"
    static let common = ["itemize", "enumerate", "equation", "align", "figure", "table", "center", "theorem", "proof", "verbatim", "minipage", "tabular"]

    var suggestions: [String] {
        let used = Set(DocumentOutline.scan(model.activeText).filter { $0.kind == .environment }.map(\.title))
        let all = Self.common + used.subtracting(Self.common).sorted()
        return name.isEmpty ? all : all.filter { EditorNavigation.fuzzyScore(name, in: $0) != nil }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.m) {
            Text("Wrap selection in environment").font(.headline)
            TextField("Environment name", text: $name)
                .textFieldStyle(.roundedBorder).font(.body.monospaced())
                .focused($focused)
                .accessibilityLabel("Environment name")
                .onSubmit { model.wrapSelection(inEnvironment: name.isEmpty ? (suggestions.first ?? "") : name) }
                .onKeyPress(.escape) { model.editorNavigation.wrapShown = false; return .handled }
            ScrollView {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 100))], alignment: .leading, spacing: DS.Space.xs) {
                    ForEach(suggestions, id: \.self) { env in
                        Button(env) { model.wrapSelection(inEnvironment: env) }
                            .buttonStyle(.bordered).controlSize(.small).font(.body.monospaced())
                    }
                }
            }
            .frame(maxHeight: DS.Layout.searchPreviewMaxHeight)
            HStack {
                Spacer()
                Button("Cancel") { model.editorNavigation.wrapShown = false }.keyboardShortcut(.cancelAction)
                Button("Wrap") { model.wrapSelection(inEnvironment: name) }.keyboardShortcut(.defaultAction).disabled(name.isEmpty)
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.settingsWidth)
        .onAppear { focused = true }
        .accessibilityIdentifier(Self.identifier)
    }
}

/// Go to Symbol: fuzzy picker over every heading, environment and label of the open documents.
struct SymbolPickerSheet: View {
    @Environment(ShellModel.self) var model
    @Environment(\.dismiss) private var dismiss
    @State private var query = ""
    @State private var selected: String?
    @FocusState private var focused: Bool
    static let identifier = "symbol.picker"

    private var entries: [SymbolPickerEntry] { Array(model.symbolPickerEntries(query: query).prefix(200)) }

    var body: some View {
        let rows = entries
        VStack(spacing: 0) {
            HStack(spacing: DS.Space.m) {
                Image(systemName: "number").foregroundStyle(.secondary)
                TextField("Go to heading, environment or label…", text: $query)
                    .textFieldStyle(.plain).font(.title3)
                    .focused($focused)
                    .accessibilityLabel("Symbol search")
                    .onKeyPress(.downArrow) { move(1, in: rows); return .handled }
                    .onKeyPress(.upArrow) { move(-1, in: rows); return .handled }
                    .onKeyPress(.return) { if let e = rows.first(where: { $0.id == selected }) ?? rows.first { model.goToSymbol(e) }; return .handled }
                    .onKeyPress(.escape) { model.editorNavigation.symbolPickerShown = false; return .handled }
                Text("\(rows.count)").font(.caption).foregroundStyle(.tertiary).monospacedDigit()
            }
            .padding(.horizontal, DS.Space.l).padding(.vertical, DS.Space.m)
            Divider()
            if rows.isEmpty {
                ContentUnavailableView.search(text: query).frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(rows, selection: $selected) { e in
                    HStack(spacing: DS.Space.m) {
                        Image(systemName: e.item.kind == .section ? "number" : e.item.kind == .environment ? "curlybraces" : "tag").foregroundStyle(.secondary)
                        Text(e.item.title.isEmpty ? "(untitled)" : e.item.title).lineLimit(1)
                        Text(e.item.kind == .section ? e.item.command : e.item.kind.rawValue).font(.caption2).foregroundStyle(.secondary)
                        Spacer()
                        Text("\(e.path):\(e.item.line)").font(.caption.monospaced()).foregroundStyle(.tertiary)
                    }
                    .tag(e.id)
                    .contentShape(Rectangle())
                    .onTapGesture { model.goToSymbol(e) }
                    .accessibilityLabel("\(e.item.command) \(e.item.title), line \(e.item.line) in \(e.path)")
                }
                .listStyle(.plain)
                .accessibilityIdentifier(Self.identifier)
            }
        }
        .frame(width: DS.Layout.pickerWindowSize.width, height: DS.Layout.pickerWindowSize.height)
        .onAppear { focused = true; selected = rows.first?.id }
        .onChange(of: query) { _, _ in selected = entries.first?.id }
    }

    private func move(_ delta: Int, in rows: [SymbolPickerEntry]) {
        guard !rows.isEmpty else { return }
        let i = rows.firstIndex { $0.id == selected } ?? 0
        selected = rows[(i + delta + rows.count) % rows.count].id
    }
}
