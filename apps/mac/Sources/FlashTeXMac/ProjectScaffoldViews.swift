import AppKit
import ObjectiveC
import Observation
import SwiftUI
import UniformTypeIdentifiers

/// The sheets behind File › New Project… / New File… and the sidebar's
/// Rename… / Delete… (ProjectScaffold.swift holds the logic). Folder
/// choice goes through an injectable provider so tests never show a panel.
@MainActor
@Observable
final class ProjectScaffoldState {
    enum Sheet: Identifiable, Equatable {
        case newProject
        case newFile
        case rename(path: String)
        case delete(path: String)
        var id: String {
            switch self {
            case .newProject: "new-project"
            case .newFile: "new-file"
            case .rename(let p): "rename:" + p
            case .delete(let p): "delete:" + p
            }
        }
    }

    var sheet: Sheet?
    /// Folder the New Project sheet writes into (remembered for the session).
    var folder: URL?
    var projectName = "Untitled"
    var template: ProjectTemplate = .articleWithSections
    var newFileName = ""
    var insertReference = true
    var renameTo = ""
    /// Last refusal / outcome, shown in the sheet.
    var note: String?

    /// Chooses the project folder (an NSOpenPanel in the app; tests inject a URL).
    @ObservationIgnored var chooseFolder: () -> URL? = {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.message = "Choose the folder the new project folder is created in"
        panel.prompt = "Choose"
        return panel.runModal() == .OK ? panel.url : nil
    }

    /// Asks before overwriting existing template files (an NSAlert in the app).
    @ObservationIgnored var confirmOverwrite: ([String]) -> Bool = { paths in
        let alert = NSAlert()
        alert.messageText = "Replace \(paths.joined(separator: ", "))?"
        alert.informativeText = "The folder already contains these template files. Replacing them overwrites their content; other files in the folder are left alone."
        alert.addButton(withTitle: "Replace")
        alert.addButton(withTitle: "Cancel")
        return alert.runModal() == .alertFirstButtonReturn
    }

    @ObservationIgnored unowned let model: ShellModel

    init(model: ShellModel) { self.model = model }

    func presentNewProject() { note = nil; sheet = .newProject }

    func presentNewFile() {
        guard model.project.projectRoot != nil else {
            model.navigationNote = "Save the entry document first (⌘S): a new file needs a project folder to live in."
            return
        }
        note = nil
        newFileName = ""
        insertReference = model.activePath == model.project.entryPath
        sheet = .newFile
    }

    func presentRename(_ path: String) {
        if let why = model.project.changeRefusal(for: path) { model.navigationNote = "Cannot rename \(path): \(why)"; return }
        note = nil; renameTo = path; sheet = .rename(path: path)
    }

    func presentDelete(_ path: String) {
        if let why = model.project.changeRefusal(for: path) { model.navigationNote = "Cannot delete \(path): \(why)"; return }
        note = nil; sheet = .delete(path: path)
    }

    // MARK: actions

    enum NewProjectOutcome: Equatable { case opened(URL), refused(String), cancelled }

    /// Creates the project in `folder`/`projectName` and opens its main.tex
    /// as the entry document (through `openTex`, so a dirty buffer is
    /// respected: `dirty` is what the sheet's Save/Discard decision chose).
    @discardableResult
    func createProject(dirty: ShellModel.DirtyDisposition = .none) -> NewProjectOutcome {
        guard let folder else { note = "Choose a folder first."; return .refused("no folder") }
        var overwrite = false
        let conflicts = ProjectScaffold.conflicts(in: folder, name: projectName.trimmingCharacters(in: .whitespacesAndNewlines), template: template)
        if !conflicts.isEmpty {
            guard confirmOverwrite(conflicts) else { note = "Not created: \(conflicts.joined(separator: ", ")) kept."; return .cancelled }
            overwrite = true
        }
        switch ProjectScaffold.create(in: folder, name: projectName, template: template, overwrite: overwrite) {
        case .failure(let f):
            note = f.text
            return .refused(f.text)
        case .success(let created):
            switch model.openTex(at: created.entry, dirty: dirty) {
            case .opened:
                sheet = nil
                model.captureNote = "Created \(created.written.count) file\(created.written.count == 1 ? "" : "s") in \(created.root.lastPathComponent): " + created.written.joined(separator: ", ")
                // The include tree is in the sidebar already (discovery); open
                // it so every member compiles and gets a tab.
                Task { await model.project.openDiscoveredIncludes() }
                return .opened(created.entry)
            case .blockedByUnsavedEdits:
                note = "The files were written, but the current buffer has unsaved edits. Save or discard it, then open \(created.entry.path) with ⌘O."
                return .refused("unsaved edits")
            case .saveFailed, .readFailed:
                note = model.captureNote ?? "could not open the new main.tex"
                return .refused(note!)
            }
        }
    }

    func createFile() async {
        let outcome = await model.project.newFile(newFileName, insertReference: insertReference)
        switch outcome {
        case .created: sheet = nil; model.navigationNote = model.project.status
        case .refused(let why): note = why
        }
    }

    func rename(_ path: String) async {
        switch await model.project.renameDocument(path, to: renameTo) {
        case .renamed: sheet = nil; model.navigationNote = model.project.status
        case .refused(let why): note = why
        }
    }

    func delete(_ path: String) async {
        switch await model.project.deleteDocument(path) {
        case .trashed: sheet = nil; model.navigationNote = model.project.status
        case .refused(let why): note = why
        }
    }
}

extension ShellModel {
    private static var scaffoldKey = 0
    /// New Project / New File / Rename / Delete sheet state (ProjectScaffoldViews.swift).
    var scaffold: ProjectScaffoldState {
        if let existing = objc_getAssociatedObject(self, &Self.scaffoldKey) as? ProjectScaffoldState { return existing }
        let state = ProjectScaffoldState(model: self)
        objc_setAssociatedObject(self, &Self.scaffoldKey, state, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return state
    }
}

// MARK: - sheets

/// Attaches the four sheets to a view (the sidebar list).
struct ProjectScaffoldSheets: ViewModifier {
    @Environment(ShellModel.self) var model

    func body(content: Content) -> some View {
        @Bindable var state = model.scaffold
        content.sheet(item: $state.sheet) { sheet in
            switch sheet {
            case .newProject: NewProjectSheet()
            case .newFile: NewFileSheet()
            case .rename(let path): RenameSheet(path: path)
            case .delete(let path): DeleteSheet(path: path)
            }
        }
    }
}

struct NewProjectSheet: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        @Bindable var state = model.scaffold
        VStack(alignment: .leading, spacing: DS.Space.l) {
            Text("New Project").font(.title2.bold())
            Grid(alignment: .leadingFirstTextBaseline, horizontalSpacing: 10, verticalSpacing: 10) {
                GridRow {
                    Text("Folder")
                    HStack {
                        Text(state.folder?.path ?? "No folder chosen").lineLimit(1).truncationMode(.middle)
                            .foregroundStyle(state.folder == nil ? .secondary : .primary)
                        Spacer()
                        Button("Choose…") { if let url = state.chooseFolder() { state.folder = url } }
                            .accessibilityIdentifier("project.new.folder")
                    }
                }
                GridRow {
                    Text("Name")
                    TextField("Project name", text: $state.projectName)
                        .textFieldStyle(.roundedBorder)
                        .accessibilityIdentifier("project.new.name")
                }
                GridRow {
                    Text("Template").gridColumnAlignment(.leading)
                    Picker("Template", selection: $state.template) {
                        ForEach(ProjectTemplate.allCases) { Text($0.title).tag($0) }
                    }
                    .pickerStyle(.radioGroup)
                    .labelsHidden()
                    .accessibilityIdentifier("project.new.template")
                }
            }
            Text(state.template.summary).font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            if let folder = state.folder {
                Text("Creates \(folder.appendingPathComponent(state.projectName.trimmingCharacters(in: .whitespacesAndNewlines)).path)/\(state.template.files(projectName: state.projectName).map(\.path).joined(separator: ", "))")
                    .font(.caption2).foregroundStyle(.tertiary).lineLimit(2).truncationMode(.middle)
            }
            if let note = state.note { Text(note).font(.callout).foregroundStyle(DS.Colors.severityWarning).fixedSize(horizontal: false, vertical: true) }
            HStack {
                Spacer()
                Button("Cancel") { state.sheet = nil }.keyboardShortcut(.cancelAction)
                Button("Create") { create() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(state.folder == nil || state.projectName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .accessibilityIdentifier("project.new.create")
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.sheetWidth)
    }

    private func create() {
        let state = model.scaffold
        guard model.isDirty || model.project.anyDirty else { state.createProject(); return }
        let alert = NSAlert()
        alert.messageText = "Save changes to \(model.documentURL?.lastPathComponent ?? "the unsaved buffer") before creating the project?"
        alert.informativeText = "Discarded text stays recoverable this session via Edit > Restore Discarded Buffer."
        alert.addButton(withTitle: "Save")
        alert.addButton(withTitle: "Discard")
        alert.addButton(withTitle: "Cancel")
        switch alert.runModal() {
        case .alertFirstButtonReturn:
            if model.documentURL == nil, !model.saveTexAs() { return }
            state.createProject(dirty: .saveFirst)
        case .alertSecondButtonReturn: state.createProject(dirty: .discard)
        default: break
        }
    }
}

struct NewFileSheet: View {
    @Environment(ShellModel.self) var model
    @FocusState private var nameFocused: Bool

    var body: some View {
        @Bindable var state = model.scaffold
        let resolved = NewFilePath.resolve(state.newFileName)
        VStack(alignment: .leading, spacing: DS.Space.l) {
            Text("New File").font(.title2.bold())
            Text("In \(model.project.projectRoot?.path ?? "the project folder")").font(.caption).foregroundStyle(.secondary).lineLimit(1).truncationMode(.middle)
            TextField("Name (e.g. sections/results)", text: $state.newFileName)
                .textFieldStyle(.roundedBorder)
                .focused($nameFocused)
                .accessibilityIdentifier("project.newfile.name")
                .onSubmit { if case .success = resolved { Task { await state.createFile() } } }
            switch resolved {
            case .success(let path): Text("Creates \(path)").font(.caption).foregroundStyle(.secondary)
            case .failure(let f): Text(state.newFileName.isEmpty ? "A .tex name, subfolders allowed, inside the project folder." : f.text).font(.caption).foregroundStyle(state.newFileName.isEmpty ? Color.secondary : DS.Colors.severityWarning)
            }
            Toggle("Insert \\input{…} at the caret in \(model.activePath)", isOn: $state.insertReference)
                .accessibilityIdentifier("project.newfile.insert")
            if let note = state.note { Text(note).font(.callout).foregroundStyle(DS.Colors.severityWarning).fixedSize(horizontal: false, vertical: true) }
            HStack {
                Spacer()
                Button("Cancel") { state.sheet = nil }.keyboardShortcut(.cancelAction)
                Button("Create") { Task { await state.createFile() } }
                    .keyboardShortcut(.defaultAction)
                    .disabled({ if case .success = resolved { false } else { true } }())
                    .accessibilityIdentifier("project.newfile.create")
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.sheetNarrowWidth)
        .onAppear { nameFocused = true }
    }
}

struct RenameSheet: View {
    @Environment(ShellModel.self) var model
    let path: String

    var body: some View {
        @Bindable var state = model.scaffold
        let resolved = NewFilePath.resolve(state.renameTo)
        let references = ReferenceRewrite.plan(oldPath: path, newPath: (try? resolved.get()) ?? path, documents: model.documents)
        VStack(alignment: .leading, spacing: DS.Space.l) {
            Text("Rename \(path)").font(.title2.bold())
            TextField("New name", text: $state.renameTo)
                .textFieldStyle(.roundedBorder)
                .accessibilityIdentifier("project.rename.name")
            switch resolved {
            case .success(let p) where p == path: Text("Same name.").font(.caption).foregroundStyle(.secondary)
            case .success(let p):
                let n = references.reduce(0) { $0 + $1.count }
                Text("Moves the file to \(p)" + (n == 0 ? "; no open document references it." : "; rewrites \(n) \\input/\\include reference\(n == 1 ? "" : "s") in \(references.map(\.path).joined(separator: ", ")) (one undoable edit each)."))
                    .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            case .failure(let f): Text(f.text).font(.caption).foregroundStyle(DS.Colors.severityWarning)
            }
            if let note = state.note { Text(note).font(.callout).foregroundStyle(DS.Colors.severityWarning).fixedSize(horizontal: false, vertical: true) }
            HStack {
                Spacer()
                Button("Cancel") { state.sheet = nil }.keyboardShortcut(.cancelAction)
                Button("Rename") { Task { await state.rename(path) } }
                    .keyboardShortcut(.defaultAction)
                    .disabled({ if case .success(let p) = resolved, p != path { false } else { true } }())
                    .accessibilityIdentifier("project.rename.apply")
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.settingsWidth)
    }
}

struct DeleteSheet: View {
    @Environment(ShellModel.self) var model
    let path: String

    var body: some View {
        @Bindable var state = model.scaffold
        VStack(alignment: .leading, spacing: DS.Space.l) {
            Text("Move \(path) to the Trash?").font(.title2.bold())
            Text("The file leaves the project and goes to the Trash (recoverable in Finder). References to it in other documents are left as they are and show as missing in the sidebar.")
                .font(.callout).fixedSize(horizontal: false, vertical: true)
            if let note = state.note { Text(note).font(.callout).foregroundStyle(DS.Colors.severityWarning).fixedSize(horizontal: false, vertical: true) }
            HStack {
                Spacer()
                Button("Cancel") { state.sheet = nil }.keyboardShortcut(.cancelAction)
                Button("Move to Trash") { Task { await state.delete(path) } }
                    .keyboardShortcut(.defaultAction)
                    .accessibilityIdentifier("project.delete.apply")
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.sheetNarrowWidth)
    }
}
