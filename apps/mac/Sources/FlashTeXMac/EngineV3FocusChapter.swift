import AppKit
import Foundation
import Observation
import SwiftUI
import FlashTeXDisplayListV3

// Chapter focus (lane FOCUS-CHAPTER, owner idea 2026-10-04): in a large
// `\include` project, preview only the chapter being written.
//
// It is LaTeX's own mechanism, not an approximation: the COMPILE carries
// `includeonly` and the host gives pdflatex the first line
// `\AtBeginDocument{\includeonly{chapters/03}}\input main.tex` (host/server.rs
// `Job`: `\includeonly` at `\begin{document}`, which keeps the chapter out of
// S₀'s key), so the pages are exactly what pdflatex makes of that document. The other
// chapters are not typeset; their page numbers, counters and labels come
// from their `.aux` files, as LaTeX does it. The user's sources are never
// touched (the host compiles the project copy, and only its command line
// differs).
//
// The focused job runs in an output folder of its own (`out-focus` beside
// the copy's `out`), started from the whole document's auxiliary files each
// time the focus changes: a focused run rewrites `main.aux`
// (`\@abspage@last` is its own page count), and sharing the folder would
// make the way back to the whole document a cold compile plus an extra
// `.aux` pass instead of a resume from its stored S₀.
//
// Changing the focus is a change of job for the host (another first line):
// the resident document is replaced, and each job keeps its own stored S₀
// (host/resident.rs `s0_path`). Export and Print are the whole document
// unless the user explicitly asks for the focused one.

/// The chapter focus of one EngineV3Session.
@MainActor
@Observable
final class EngineV3Focus {
    /// One focused chapter: the name as the document's `\include` writes it
    /// (what `\includeonly` takes) and the file it resolves to.
    struct Chapter: Equatable {
        var name: String
        var path: String
    }

    /// The focused chapters; empty: the whole document.
    private(set) var chapters: [Chapter] = []
    /// What the focus was set for: a focus never outlives its project,
    /// main file or project generation (another project opened).
    @ObservationIgnored private var scope: Scope?
    struct Scope: Equatable { var root: URL?; var main: String; var generation: Int }
    /// The chapter set the focus output folder was last started for: a new
    /// set starts it again from the whole document's files.
    @ObservationIgnored private(set) var seededFor: [String]?
    /// The next export is the focused document (the user chose it).
    @ObservationIgnored var exportFocused = false
    /// Focus folders started (tests).
    @ObservationIgnored private(set) var seeds = 0

    var isActive: Bool { !chapters.isEmpty }
    var names: [String] { chapters.map(\.name) }

    func set(_ chapters: [Chapter], scope: Scope) {
        self.chapters = chapters
        self.scope = scope
    }

    func clear() {
        if !chapters.isEmpty { chapters = [] }
        scope = nil
        seededFor = nil
        exportFocused = false
    }

    /// The focus output folder of a project copy.
    nonisolated static func output(base: URL) -> URL { base.appendingPathComponent("out-focus", isDirectory: true) }

    /// The focused job for a COMPILE (`EngineV3Session.request`), or nil for
    /// the whole document: no focus, a focus set for another project, main
    /// file or generation (dropped), a host without the capability, or an
    /// export the user did not ask to focus. Starts the focus output folder
    /// when the chapter set is new, on the send queue before this COMPILE.
    func job(model: ShellModel, main: String, project: EngineV3Mirror, offered: Bool, exporting: Bool) -> (names: [String], output: URL)? {
        guard isActive else { return nil }
        guard scope == Scope(root: model.project.projectRoot, main: main, generation: model.projectGeneration) else {
            clear()
            return nil
        }
        guard offered, !exporting || exportFocused else { return nil }
        let output = Self.output(base: project.base)
        if seededFor != names {
            seededFor = names
            seeds += 1
            let from = project.output
            EngineV3Session.sendQueue.async { Self.seed(from: from, to: output) }
        }
        return (names, output)
    }

    // MARK: which file is a chapter

    /// The `\include` name of `path` in the entry's include closure, or nil
    /// when nothing `\include`s it (an `\input` file, the main file, an
    /// argument that needs expansion, or a name `\includeonly` cannot carry).
    nonisolated static func includeName(for path: String, in closure: ProjectDocuments.Closure) -> String? {
        for n in closure.nodes where n.reference.kind == .include && n.reference.literal && n.resolvedPath == path {
            if case .unresolvable = n.state { continue }
            let name = n.reference.argument
            if validName(name) { return name }
        }
        return nil
    }

    /// A name the host takes for `includeonly` (host/server.rs
    /// `includeonly_list`): relative, inside the root, nothing a first line
    /// would read as markup.
    nonisolated static func validName(_ name: String) -> Bool {
        guard !name.isEmpty, name.trimmingCharacters(in: .whitespacesAndNewlines) == name,
              !name.hasPrefix("/") else { return false }
        if name.unicodeScalars.contains(where: { ",{}\\%#\"".unicodeScalars.contains($0) || CharacterSet.controlCharacters.contains($0) }) { return false }
        return !name.split(separator: "/", omittingEmptySubsequences: false).contains("..")
    }

    // MARK: the focus output folder

    /// Output files a focused run does not read back.
    nonisolated static let notCopied: Set<String> = ["pdf", "log", "synctex", "gz", "fls"]
    /// A file larger than this is not an auxiliary file; not copied.
    nonisolated static let maxCopiedBytes = 64 << 20

    /// Starts `to` from the whole document's output `from`, as a pdflatex
    /// user's focused run starts from their last full run's files: emptied,
    /// then every `.aux`, `.toc`, `.bbl`, … copied (subfolders kept:
    /// `chapters/03.aux`). Symbolic links are not followed.
    nonisolated static func seed(from: URL, to: URL) {
        let fm = FileManager.default
        try? fm.removeItem(at: to)
        try? fm.createDirectory(at: to, withIntermediateDirectories: true)
        let keys: [URLResourceKey] = [.isRegularFileKey, .isDirectoryKey, .isSymbolicLinkKey, .fileSizeKey]
        guard let e = fm.enumerator(at: from, includingPropertiesForKeys: keys, options: []) else { return }
        let prefix = from.standardizedFileURL.path + "/"
        for case let u as URL in e {
            guard let v = try? u.resourceValues(forKeys: Set(keys)) else { continue }
            if v.isSymbolicLink == true { e.skipDescendants(); continue }
            let path = u.standardizedFileURL.path
            guard path.hasPrefix(prefix) else { continue }
            let dst = to.appendingPathComponent(String(path.dropFirst(prefix.count)))
            // Every folder (`\include{chapters/03}` writes `chapters/03.aux`).
            if v.isDirectory == true { try? fm.createDirectory(at: dst, withIntermediateDirectories: true); continue }
            guard v.isRegularFile == true, (v.fileSize ?? 0) <= maxCopiedBytes,
                  !notCopied.contains(u.pathExtension.lowercased()) else { continue }
            try? fm.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
            try? fm.copyItem(at: u, to: dst)
        }
    }
}

// MARK: - session and model

extension EngineV3Session {
    /// Focus the preview on `chapters` (empty: the whole document) and compile.
    func setFocus(_ chapters: [EngineV3Focus.Chapter], model: ShellModel) {
        if chapters.isEmpty {
            guard focus.isActive else { return }
            focus.clear()
        } else {
            guard chapters != focus.chapters else { return }
            focus.set(chapters, scope: .init(root: model.project.projectRoot, main: Self.mainFile(model: model), generation: model.projectGeneration))
        }
        compileNow(model: model, reason: "focus")
    }
}

extension ShellModel {
    /// The chapter `path` is, when the preview can focus on it: the engine-v3
    /// preview with a host that honours `includeonly`, and `path` is
    /// `\include`d (the throttled closure, ShellChrome).
    func focusChapter(for path: String) -> EngineV3Focus.Chapter? {
        guard engineV3Enabled, engineV3.phase == .ready, engineV3.hostOffersIncludeOnly,
              let name = EngineV3Focus.includeName(for: path, in: chrome.closure) else { return nil }
        return .init(name: name, path: path)
    }

    /// "Focus Preview on This Chapter" for `path` (the active file by default).
    func focusPreview(on path: String? = nil) {
        let path = path ?? activePath
        guard let chapter = focusChapter(for: path) else {
            captureNote = "\(path) is not a chapter the document \\include's, so the preview cannot focus on it."
            return
        }
        engineV3.setFocus([chapter], model: self)
        captureNote = "Previewing \(chapter.path) only (\\includeonly{\(chapter.name)}); other chapters' numbers come from their .aux files."
    }

    /// "Show All": the whole document again.
    func showWholeDocument() {
        guard engineV3.focus.isActive else { return }
        engineV3.setFocus([], model: self)
        captureNote = "Previewing the whole document."
    }

    /// Export the focused chapters' PDF (`\includeonly`): only when the user
    /// asks; Export PDF… and Print are the whole document.
    func exportFocusedPDF() {
        guard engineV3.focus.isActive else { exportPDF(); return }
        if let why = engineV3.exportRefusal() { reportExportFailure(why); return }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.pdf]
        let job = (engineV3.mainFile as NSString).lastPathComponent.replacingOccurrences(of: ".tex", with: "")
        let chapter = ((engineV3.focus.chapters.first?.path ?? "chapter") as NSString).deletingPathExtension
            .replacingOccurrences(of: "/", with: "-")
        panel.nameFieldStringValue = "\(job.isEmpty ? "document" : job)-\(chapter).pdf"
        panel.message = "Export the focused chapters only: the PDF pdflatex writes with \\includeonly"
        guard panel.runModal() == .OK, let out = panel.url else { return }
        engineV3.focus.exportFocused = true
        exportPDFEngineV3(to: .recordingCurrentDisk(out)) { [weak self] _ in self?.engineV3.focus.exportFocused = false }
    }
}

// MARK: - views

/// Over the preview while it is focused: what it shows, and the way back.
struct FocusChapterBanner: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        let focus = model.engineV3.focus
        if model.engineV3Enabled, focus.isActive {
            let shown = focus.chapters.map(\.path).joined(separator: ", ")
            HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                Image(systemName: "scope")
                    .foregroundStyle(DS.Colors.accentSelection)
                    .accessibilityHidden(true)
                Text("Previewing \(shown) only")
                    .font(DS.Fonts.secondary.weight(.semibold))
                    .lineLimit(1).truncationMode(.middle)
                    .help("The preview compiles \\includeonly{\(focus.names.joined(separator: ","))}: the other chapters are not typeset, and their page numbers and references come from their .aux files. Export and Print are the whole document.")
                Spacer(minLength: 0)
                Button("Export Chapter…") { model.exportFocusedPDF() }
                    .ideSecondary()
                    .help("Export only the focused chapters, as pdflatex writes them with \\includeonly")
                    .accessibilityIdentifier("focus.export")
                Button("Show All") { model.showWholeDocument() }
                    .ideSecondary()
                    .accessibilityIdentifier("focus.show-all")
            }
            .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.s)
            .background(DS.Colors.surfaceSecondary)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Previewing \(shown) only")
            .accessibilityIdentifier("focus.banner")
        }
    }
}

/// The title bar's focus toggle: on, the preview shows the active chapter
/// only; off, the whole document. Disabled when the active file is not an
/// `\include`d chapter and nothing is focused.
struct FocusChapterTitleBarToggle: View {
    @Environment(ShellModel.self) var model
    @State private var hovering = false

    var body: some View {
        let focused = model.engineV3.focus.isActive
        let chapter = focused ? nil : model.focusChapter(for: model.activePath)
        Toggle(isOn: Binding(get: { focused }, set: { on in on ? model.focusPreview() : model.showWholeDocument() })) {
            IconButtonLabel(icon: "scope", on: focused, hovering: hovering)
        }
        .toggleStyle(.button).buttonStyle(PressableStyle())
        .disabled(!focused && chapter == nil)
        .onHover { hovering = $0 }
        .help(focused ? "Show the whole document in the preview"
              : chapter.map { "Focus the preview on \($0.path) (\\includeonly{\($0.name)})" }
              ?? "Focus the preview on a chapter: available in a file the document \\include's")
        .accessibilityLabel(focused ? "Show whole document" : "Focus preview on this chapter")
        .accessibilityIdentifier("toolbar.focus-chapter")
    }
}
