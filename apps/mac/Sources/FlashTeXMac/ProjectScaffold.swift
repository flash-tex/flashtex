import AppKit
import ObjectiveC
import Observation
import FlashTeXProtocol

/// Creating a multi-file project from scratch inside the IDE (lane
/// `mac-new-project`): File › New Project… (templates), New File… in the
/// project sidebar, "Create x.tex" for an `\input`/`\include` that does not
/// resolve, and Rename… / Delete… on sidebar rows.
///
/// Everything that decides *what* is written is pure and deterministic
/// (`ProjectTemplate`, `NewFilePath`, `MissingIncludeFix`, `ReferenceRewrite`)
/// so the tests run in temp directories with no panel. The model layer
/// (`ProjectDocuments` extension) writes through the same rooted-path
/// checks the multi-file lane applies (`ProjectIncludes.normalize`,
/// `ProjectDocuments.rootedFile`: no absolute paths, no `..`, no symlink),
/// opens the new document through `openDocument`, and hands every text
/// change to the editor as ONE `pendingEdit` per document — the reviewed,
/// undoable path capture insertions and quick fixes already use. Nothing
/// here writes into an open buffer behind the editor's back.

// MARK: - templates

/// What File › New Project… can create. Each template is `main.tex` plus
/// the members it `\input`s / `\include`s, so the sidebar shows the include
/// tree the moment the entry document opens.
enum ProjectTemplate: String, CaseIterable, Identifiable {
    case blankArticle, articleWithSections, reportWithChapters, homeworkSheet

    var id: String { rawValue }

    var title: String {
        switch self {
        case .blankArticle: "Blank article"
        case .articleWithSections: "Article with sections"
        case .reportWithChapters: "Report with chapters"
        case .homeworkSheet: "Homework sheet"
        }
    }

    var summary: String {
        switch self {
        case .blankArticle: "main.tex only: article class, one section to start typing in."
        case .articleWithSections: "main.tex with \\input{sections/introduction} and \\input{sections/methods}."
        case .reportWithChapters: "main.tex (report class) with \\include{chapters/introduction} and \\include{chapters/background}."
        case .homeworkSheet: "main.tex with the problem-sheet preamble (geometry, amsmath, amssymb, enumitem) and a \\problem macro."
        }
    }

    /// The entry document's path inside the project folder.
    static let entryPath = "main.tex"

    /// Files this template writes, entry first, as rooted relative paths and
    /// UTF-8 text. `name` is the project title as typed (TeX-escaped where
    /// it lands in the source).
    func files(projectName name: String) -> [(path: String, text: String)] {
        let title = Self.escapeTitle(name)
        switch self {
        case .blankArticle:
            return [(Self.entryPath, """
            \\documentclass{article}
            \\usepackage[margin=1in]{geometry}
            \\usepackage{amsmath,amssymb}

            \\title{\(title)}
            \\author{}
            \\date{\\today}

            \\begin{document}
            \\maketitle

            \\section{Introduction}

            \\end{document}

            """)]
        case .articleWithSections:
            return [(Self.entryPath, """
            \\documentclass{article}
            \\usepackage[margin=1in]{geometry}
            \\usepackage{amsmath,amssymb}

            \\title{\(title)}
            \\author{}
            \\date{\\today}

            \\begin{document}
            \\maketitle

            \\input{sections/introduction}
            \\input{sections/methods}

            \\end{document}

            """),
                    ("sections/introduction.tex", "\\section{Introduction}\n\n"),
                    ("sections/methods.tex", "\\section{Methods}\n\n")]
        case .reportWithChapters:
            return [(Self.entryPath, """
            \\documentclass{report}
            \\usepackage[margin=1in]{geometry}
            \\usepackage{amsmath,amssymb}

            \\title{\(title)}
            \\author{}
            \\date{\\today}

            \\begin{document}
            \\maketitle
            \\tableofcontents

            \\include{chapters/introduction}
            \\include{chapters/background}

            \\end{document}

            """),
                    ("chapters/introduction.tex", "\\chapter{Introduction}\n\n"),
                    ("chapters/background.tex", "\\chapter{Background}\n\n")]
        case .homeworkSheet:
            return [(Self.entryPath, """
            \\documentclass[11pt]{article}

            \\usepackage[T1]{fontenc}
            \\usepackage[utf8]{inputenc}
            \\usepackage[margin=1in]{geometry}
            \\usepackage{amsmath,amssymb,amsthm}
            \\usepackage[shortlabels]{enumitem}

            \\setlength{\\parindent}{0pt}
            \\setlength{\\parskip}{0.65em}
            \\setlist[enumerate]{leftmargin=*,itemsep=0.45em,topsep=0.35em}

            \\newcommand{\\N}{\\mathbb{N}}
            \\newcommand{\\Z}{\\mathbb{Z}}
            \\newcommand{\\Q}{\\mathbb{Q}}
            \\newcommand{\\R}{\\mathbb{R}}
            \\newcommand{\\problem}[2]{\\subsection*{Problem #1 \\hfill \\normalfont[#2 points]}}

            \\begin{document}
            \\pagestyle{empty}

            \\begin{center}
                {\\LARGE\\bfseries \(title)}\\\\[7pt]
            \\end{center}

            \\problem{1}{10}

            \\begin{enumerate}[(a)]
                \\item
            \\end{enumerate}

            \\end{document}

            """)]
        }
    }

    /// The characters TeX would otherwise read as markup in a title.
    static func escapeTitle(_ s: String) -> String {
        var out = ""
        for c in s {
            switch c {
            case "\\": out += "\\textbackslash{}"
            case "{", "}", "$", "&", "#", "_", "%": out += "\\" + String(c)
            case "~": out += "\\textasciitilde{}"
            case "^": out += "\\textasciicircum{}"
            default: out.append(c)
            }
        }
        return out
    }
}

/// Writes a template into `<folder>/<name>` (pure file-system work; the
/// model then opens the entry document like ⌘O would).
enum ProjectScaffold {
    struct Created: Equatable {
        var root: URL
        var entry: URL
        var written: [String]
    }

    enum Failure: Error, Equatable {
        case badName(String)
        /// The project folder already holds these files; nothing was written.
        case wouldOverwrite([String])
        case io(String)

        var text: String {
            switch self {
            case .badName(let why): "project name: \(why)"
            case .wouldOverwrite(let paths): "\(paths.joined(separator: ", ")) already exist\(paths.count == 1 ? "s" : "") in that folder"
            case .io(let why): why
            }
        }
    }

    /// One folder segment: no separators, not `.`/`..`, no control characters.
    static func validateName(_ raw: String) -> Result<String, Failure> {
        let name = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if name.isEmpty { return .failure(.badName("empty")) }
        if name == "." || name == ".." { return .failure(.badName("'\(name)' is not a folder name")) }
        if name.contains("/") || name.contains("\\") || name.contains(":") { return .failure(.badName("no path separators")) }
        if name.unicodeScalars.contains(where: { $0.properties.generalCategory == .control }) { return .failure(.badName("control character")) }
        return .success(name)
    }

    /// Files of `template` that already exist under `<folder>/<name>`
    /// (the ones a create would overwrite). Empty when the folder is new.
    static func conflicts(in folder: URL, name: String, template: ProjectTemplate) -> [String] {
        let root = folder.appendingPathComponent(name)
        return template.files(projectName: name).map(\.path).filter {
            FileManager.default.fileExists(atPath: root.appendingPathComponent($0).path)
        }
    }

    /// Creates `<folder>/<name>/…`. Existing files are refused unless
    /// `overwrite` (the sheet asks first); other files in the folder are
    /// left alone.
    static func create(in folder: URL, name rawName: String, template: ProjectTemplate, overwrite: Bool = false) -> Result<Created, Failure> {
        let name: String
        switch validateName(rawName) {
        case .success(let n): name = n
        case .failure(let f): return .failure(f)
        }
        let root = folder.appendingPathComponent(name, isDirectory: true)
        let existing = conflicts(in: folder, name: name, template: template)
        if !existing.isEmpty, !overwrite { return .failure(.wouldOverwrite(existing)) }
        let fm = FileManager.default
        var written: [String] = []
        do {
            for (path, text) in template.files(projectName: name) {
                let url = root.appendingPathComponent(path)
                try fm.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
                try text.write(to: url, atomically: true, encoding: .utf8)
                written.append(path)
            }
        } catch {
            return .failure(.io("could not write \(name): \(error.localizedDescription)"))
        }
        return .success(Created(root: root, entry: root.appendingPathComponent(ProjectTemplate.entryPath), written: written))
    }
}

// MARK: - new file names

/// The rooted `.tex` path a "New File…" name means. Subfolders are allowed
/// (`sections/results` → `sections/results.tex`); anything that leaves the
/// project root, is absolute, or names the entry document is refused.
enum NewFilePath {
    enum Failure: Error, Equatable {
        case invalid(String)
        case isEntry
        case exists(String)

        var text: String {
            switch self {
            case .invalid(let why): why
            case .isEntry: "main.tex is the entry document"
            case .exists(let p): "\(p) already exists"
            }
        }
    }

    /// `raw` normalized to a rooted `.tex` path — or a `.sty`/`.cls` path
    /// when the name says so (a package or class file gets its template,
    /// `PackageTemplate`); any other extension is completed with `.tex`.
    static func resolve(_ raw: String) -> Result<String, Failure> {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let path: String
        do { path = try ProjectIncludes.normalize(trimmed) }
        catch { return .failure(.invalid("\(error)")) }
        let withExt = path.hasSuffix(".tex") || PackageTemplate.kind(of: path) != nil ? path : path + ".tex"
        if withExt == ProjectTemplate.entryPath { return .failure(.isEntry) }
        if withExt.split(separator: "/").last.map({ $0 == ".tex" || $0 == ".sty" || $0 == ".cls" }) ?? true { return .failure(.invalid("empty file name")) }
        return .success(withExt)
    }

    /// The `\input` argument for a rooted `.tex` path (`sections/a.tex` → `sections/a`).
    static func inputArgument(for path: String) -> String {
        path.hasSuffix(".tex") ? String(path.dropLast(4)) : path
    }

    /// The command that references a new file from the document that made
    /// it: `\input{sections/a}` for a `.tex` file, `\usepackage{mystyle}`
    /// for a `.sty` (the compiler resolves it by name next to the entry),
    /// nil for a `.cls` — a class is named by `\documentclass`, which the
    /// document already has.
    static func referenceCommand(for path: String, kind: ProjectIncludes.Kind = .input) -> String? {
        switch PackageTemplate.kind(of: path) {
        case .package: return "\\usepackage{\(PackageTemplate.name(of: path))}"
        case .documentClass: return nil
        case nil: return "\\\(kind.rawValue){\(inputArgument(for: path))}"
        }
    }

    /// The `\input{…}` (or `\usepackage{…}`) line inserted at the caret, on its own line.
    static func referenceText(for path: String, kind: ProjectIncludes.Kind = .input, into text: String, atByte byte: Int) -> String {
        Insertion.insertionText(referenceCommand(for: path, kind: kind) ?? "", into: text, atByte: byte)
    }
}

// MARK: - package and class templates

/// What File › New File… writes into a fresh `.sty` or `.cls` (and the
/// "Create mystyle.sty" quick fix on the compiler's missing-package
/// diagnostic, ProblemsPanel.swift): the ltclass preamble every package
/// and class starts with, with today's date in the `\Provides…` line.
/// A `.tex` gets no template (nil), as before.
enum PackageTemplate {
    enum Kind: Equatable { case package, documentClass }

    /// `.sty` → package, `.cls` → class, anything else nil.
    static func kind(of path: String) -> Kind? {
        switch (path as NSString).pathExtension.lowercased() {
        case "sty": return .package
        case "cls": return .documentClass
        default: return nil
        }
    }

    /// The name `\ProvidesPackage`/`\usepackage` use: the file name without
    /// its extension (`styles/mystyle.sty` → `mystyle`).
    static func name(of path: String) -> String {
        ((path as NSString).lastPathComponent as NSString).deletingPathExtension
    }

    /// `yyyy/mm/dd` as `\ProvidesPackage` wants it.
    static func dateStamp(_ date: Date = Date()) -> String {
        let f = DateFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.timeZone = TimeZone(identifier: "UTC")
        f.dateFormat = "yyyy/MM/dd"
        return f.string(from: date)
    }

    /// The template for `path`, nil when it is not a package or class file.
    static func text(for path: String, date: Date = Date()) -> String? {
        guard let kind = kind(of: path) else { return nil }
        let name = name(of: path)
        let stamp = dateStamp(date)
        switch kind {
        case .package:
            return """
            \\NeedsTeXFormat{LaTeX2e}
            \\ProvidesPackage{\(name)}[\(stamp) v1.0 \(name)]

            % Options: \\DeclareOption{name}{code}, then \\ProcessOptions.
            \\DeclareOption*{\\PackageWarning{\(name)}{Unknown option `\\CurrentOption'}}
            \\ProcessOptions\\relax

            % Packages this one needs.
            % \\RequirePackage{xcolor}

            % Definitions (\\@ is a letter here: no \\makeatletter needed).

            \\endinput

            """
        case .documentClass:
            return """
            \\NeedsTeXFormat{LaTeX2e}
            \\ProvidesClass{\(name)}[\(stamp) v1.0 \(name)]

            % Options not declared here go to the parent class.
            \\DeclareOption*{\\PassOptionsToClass{\\CurrentOption}{article}}
            \\ProcessOptions\\relax
            \\LoadClass{article}

            % Packages this class needs.
            % \\RequirePackage{geometry}

            % Definitions (\\@ is a letter here: no \\makeatletter needed).

            \\endinput

            """
        }
    }
}

// MARK: - missing includes

/// The compiler's diagnostic for an `\input`/`\include` it could not find
/// (`crates/compiler/src/parser.rs`: `included file not found: looked for
/// 'x' and 'x.tex'`) parsed back into the requested name, deterministically:
/// no heuristics, no other message shape matches.
enum MissingIncludeFix {
    static let prefix = "included file not found: looked for '"

    /// The requested name, or nil when `message` is not that diagnostic.
    static func requested(from message: String) -> String? {
        guard message.hasPrefix(prefix) else { return nil }
        let rest = message.dropFirst(prefix.count)
        guard let end = rest.range(of: "' and '") else { return nil }
        let name = String(rest[..<end.lowerBound])
        return name.isEmpty ? nil : name
    }

    struct QuickFix: Equatable {
        var argument: String
        var path: String
        var from: String
    }

    /// The "Create x.tex" quick fix for a diagnostic, when it is the
    /// compiler's missing-include error, the name is rooted, the file does
    /// not exist under `projectRoot`, and the diagnostic names its source.
    static func quickFix(for d: RuntimeV1.Diagnostic, projectRoot: URL?) -> QuickFix? {
        guard let root = projectRoot, let argument = requested(from: d.message), let path = path(for: argument),
              case .file(let url) = ProjectDocuments.rootedFile(path, under: root),
              !FileManager.default.fileExists(atPath: url.path) else { return nil }
        return QuickFix(argument: argument, path: path, from: d.source?.path ?? ProjectTemplate.entryPath)
    }

    /// The rooted `.tex` file "Create …" would write for `argument`
    /// (`x` → `x.tex`, `x.tex` → `x.tex`), nil when it is not rooted.
    static func path(for argument: String) -> String? {
        guard let first = try? ProjectIncludes.candidates(for: argument).first else { return nil }
        return first.hasSuffix(".tex") ? first : first + ".tex"
    }
}

// MARK: - rename: reference rewrite

/// What renaming a member changes in the open buffers: every literal
/// `\input`/`\include` whose rooted candidates contain the old path, in
/// every open document, as one grouped replacement per document (the
/// editor applies each as one undoable edit).
enum ReferenceRewrite {
    struct DocumentEdit: Equatable {
        var path: String
        /// UTF-8 byte range of the covering edit in that document's current text.
        var byteRange: Range<Int>
        var nsRange: NSRange
        var before: String
        var text: String
        var count: Int
    }

    /// Edits for renaming `oldPath` to `newPath` (both rooted `.tex` paths).
    /// `documents` are the open buffers. Deterministic: document order is
    /// the project order, references in source order.
    static func plan(oldPath: String, newPath: String, documents: [RuntimeV1.Document]) -> [DocumentEdit] {
        let newArgument = NewFilePath.inputArgument(for: newPath)
        var out: [DocumentEdit] = []
        for doc in documents {
            let hits = ProjectIncludes.scan(doc.text).filter { ref in
                ref.literal && ((try? ProjectIncludes.candidates(for: ref.argument))?.contains(oldPath) ?? false)
            }
            guard let first = hits.first, let last = hits.last else { continue }
            let bytes = Array(doc.text.utf8)
            let lo = first.argumentStartByte, hi = last.argumentEndByte
            var text = ""
            var cursor = lo
            for ref in hits {
                text += String(decoding: bytes[cursor..<ref.argumentStartByte], as: UTF8.self)
                // `\include{x.tex}` keeps its explicit extension; `\input{x}` stays bare.
                text += ref.argument.hasSuffix(".tex") ? newPath : newArgument
                cursor = ref.argumentEndByte
            }
            text += String(decoding: bytes[cursor..<hi], as: UTF8.self)
            guard let r = doc.text.rangeOfUTF8(start: lo, end: hi) else { continue }
            out.append(DocumentEdit(path: doc.path, byteRange: lo..<hi, nsRange: NSRange(r, in: doc.text),
                                    before: String(doc.text[r]), text: text, count: hits.count))
        }
        return out
    }
}

// MARK: - model operations

extension ProjectDocuments {
    enum CreateOutcome: Equatable {
        case created(path: String)
        case refused(String)
    }

    /// The rooted URL a new member would be written to, or why not.
    struct Refusal: Error, Equatable { var why: String }

    /// Also the destination check of a move (ProjectMove.swift).
    func newFileURL(_ path: String) -> Result<URL, Refusal> {
        guard let root = projectRoot else { return .failure(Refusal(why: "the entry document is not saved, so there is no project root; save it first (⌘S)")) }
        // Every existing ancestor must be a real directory under the root; the
        // leaf may not exist yet, so check the deepest existing prefix.
        var probe = root
        for segment in path.split(separator: "/").dropLast() {
            probe.appendPathComponent(String(segment))
            if let type = (try? FileManager.default.attributesOfItem(atPath: probe.path))?[.type] as? FileAttributeType, type == .typeSymbolicLink {
                return .failure(Refusal(why: "\(segment) is a symbolic link"))
            }
        }
        switch Self.rootedFile(path, under: root) {
        case .file(let url): return .success(url)
        case .refused(let why): return .failure(Refusal(why: why))
        }
    }

    /// New File…: writes `<root>/<path>` (empty, or `text`; a `.sty`/`.cls`
    /// with no text gets its `PackageTemplate`) and opens it as a member
    /// with `role`. Refuses an existing file, a path outside the root, and
    /// the entry document's name.
    @discardableResult
    func createDocument(_ rawPath: String, text given: String = "", role: ProjectDocument.Role = .opened) async -> CreateOutcome {
        let path: String
        switch NewFilePath.resolve(rawPath) {
        case .success(let p): path = p
        case .failure(let f): return noteCreate(.refused("cannot create \(rawPath): \(f.text)"))
        }
        let text = given.isEmpty ? (PackageTemplate.text(for: path) ?? "") : given
        if isOpen(path) { return noteCreate(.refused("cannot create \(path): it is already open")) }
        let url: URL
        switch newFileURL(path) {
        case .success(let u): url = u
        case .failure(let r): return noteCreate(.refused("cannot create \(path): \(r.why)"))
        }
        if FileManager.default.fileExists(atPath: url.path) { return noteCreate(.refused("cannot create \(path): it already exists (open it instead)")) }
        do {
            try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try text.write(to: url, atomically: true, encoding: .utf8)
        } catch {
            return noteCreate(.refused("cannot create \(path): \(error.localizedDescription)"))
        }
        switch await openDocument(path, role: role) {
        case .opened, .alreadyOpen: return noteCreate(.created(path: path))
        case .refused(let why): return noteCreate(.refused("created \(path) but could not open it: \(why)"))
        }
    }

    /// Posts `\input{name}` at the caret of the active document as one
    /// undoable edit (the editor applies it; `ShellModel.editApplied` folds
    /// it into the buffer). Returns why not when nothing was posted.
    @discardableResult
    func insertReferenceAtCaret(to path: String, kind: ProjectIncludes.Kind = .input) -> String? {
        guard let model = self.model else { return "the project was closed" }
        if model.pendingEdit != nil { return "another edit is still pending in the editor" }
        let text = model.activeText
        guard let byte = model.caretByte, let ns = text.nsRange(utf8Bytes: .init(path: model.activePath, startByte: byte, endByte: byte)) else {
            return "the caret is not at a valid position"
        }
        model.pendingEdit = .init(path: model.activePath, nsRange: ns,
                                  text: NewFilePath.referenceText(for: path, kind: kind, into: text, atByte: byte),
                                  token: model.nextEditToken(), revision: model.editorRevision)
        return nil
    }

    /// New File… end to end: create, optionally insert the reference into
    /// the document that is active now, then switch to the new document
    /// once the reference edit has been applied (a switch is refused while
    /// an edit is pending, so the switch waits for the editor).
    func newFile(_ rawPath: String, insertReference: Bool) async -> CreateOutcome {
        guard let model = self.model else { return .refused("cannot create \(rawPath): the project was closed") }
        let referencing = model.activePath
        // A `.sty` is referenced with `\usepackage{name}`; a `.cls` has no
        // reference to insert (the document's `\documentclass` names it).
        let insertReference = insertReference && NewFilePath.referenceCommand(for: (try? NewFilePath.resolve(rawPath).get()) ?? rawPath) != nil
        let outcome = await createDocument(rawPath, role: insertReference ? .included(from: referencing) : .opened)
        guard case .created(let path) = outcome else { return outcome }
        if insertReference, model.activePath == referencing {
            if let why = insertReferenceAtCaret(to: path) {
                noteStatus("created \(path); \\input not inserted: \(why)")
            } else {
                afterPendingEdit { [weak self] in self?.switchDocument(to: path) }
                return outcome
            }
        }
        switchDocument(to: path)
        return outcome
    }

    /// Create missing include: the file `\input{argument}` from `from`
    /// would resolve to, created empty and opened as included from there.
    func createMissingInclude(_ argument: String, from: String) async -> CreateOutcome {
        guard let path = MissingIncludeFix.path(for: argument) else {
            return noteCreate(.refused("cannot create a file for \\input{\(argument)}: not a rooted path"))
        }
        return await createDocument(path, text: "", role: .included(from: from))
    }

    // MARK: rename / delete

    enum RenameOutcome: Equatable {
        case renamed(from: String, to: String, references: Int)
        case refused(String)
    }

    /// Whether `path` can be renamed or deleted now: not the entry, and not
    /// carrying unsaved edits (they would be lost or land in the wrong file).
    func changeRefusal(for path: String) -> String? {
        if path == entryPath { return "\(path) is the entry document" }
        if let why = readOnlyNote(for: path) { return why }
        if isDirty(path) { return "\(path) has unsaved edits; save them first (⌘S)" }
        return nil
    }

    /// Rename…: moves the file, retargets the open member, and posts one
    /// undoable `\input`/`\include` rewrite per open document that
    /// references it (the active document first; the others after the
    /// editor has applied the previous one). Refused when the member is
    /// open with unsaved edits, when the destination exists, or when any
    /// referencing document has unsaved edits of its own (`ReferenceRewrite`
    /// edits its buffer, so its baseline must be its file).
    func renameDocument(_ path: String, to rawNewPath: String) async -> RenameOutcome {
        guard let model = self.model else { return noteRename(.refused("cannot rename \(path): the project was closed")) }
        if let why = changeRefusal(for: path) { return noteRename(.refused("cannot rename \(path): \(why)")) }
        guard isOpen(path) || projectRoot != nil else { return noteRename(.refused("cannot rename \(path): no project root")) }
        let newPath: String
        switch NewFilePath.resolve(rawNewPath) {
        case .success(let p): newPath = p
        case .failure(let f): return noteRename(.refused("cannot rename \(path): \(f.text)"))
        }
        if newPath == path { return noteRename(.refused("cannot rename \(path): same name")) }
        if isOpen(newPath) { return noteRename(.refused("cannot rename \(path): \(newPath) is open")) }
        guard let root = projectRoot else { return noteRename(.refused("cannot rename \(path): no project root")) }
        let from: URL, to: URL
        switch (Self.rootedFile(path, under: root), newFileURL(newPath)) {
        case (.file(let f), .success(let t)): from = f; to = t
        case (.refused(let why), _): return noteRename(.refused("cannot rename \(path): \(why)"))
        case (_, .failure(let r)): return noteRename(.refused("cannot rename \(path): \(r.why)"))
        }
        guard FileManager.default.fileExists(atPath: from.path) else { return noteRename(.refused("cannot rename \(path): no such file under the project root")) }
        if FileManager.default.fileExists(atPath: to.path) { return noteRename(.refused("cannot rename \(path): \(newPath) already exists")) }
        let edits = ReferenceRewrite.plan(oldPath: path, newPath: newPath, documents: model.documents)
        if let dirty = edits.first(where: { $0.path != entryPath && isDirty($0.path) }) ?? edits.first(where: { $0.path == entryPath && isDirty($0.path) }) {
            return noteRename(.refused("cannot rename \(path): \(dirty.path) references it and has unsaved edits; save it first"))
        }
        if model.pendingEdit != nil { return noteRename(.refused("cannot rename \(path): an edit is still pending in the editor")) }
        do {
            try FileManager.default.createDirectory(at: to.deletingLastPathComponent(), withIntermediateDirectories: true)
            try FileManager.default.moveItem(at: from, to: to)
        } catch {
            return noteRename(.refused("cannot rename \(path): \(error.localizedDescription)"))
        }
        // The open member follows its file (same text, same baseline).
        if let i = model.documents.firstIndex(where: { $0.path == path }) {
            model.documents[i].path = newPath
            if model.activePath == path { model.activePath = newPath }
            retarget(path, to: newPath)
        }
        applyReferenceEdits(edits)
        return noteRename(.renamed(from: path, to: newPath, references: edits.reduce(0) { $0 + $1.count }))
    }

    enum DeleteOutcome: Equatable {
        case trashed(path: String)
        case refused(String)
    }

    /// Delete…: detaches the member (refused with unsaved edits) and moves
    /// its file to the Trash (`FileManager.trashItem`, recoverable in Finder).
    func deleteDocument(_ path: String) async -> DeleteOutcome {
        if let why = changeRefusal(for: path) { return noteDelete(.refused("cannot delete \(path): \(why)")) }
        guard let root = projectRoot else { return noteDelete(.refused("cannot delete \(path): no project root")) }
        let url: URL
        switch Self.rootedFile(path, under: root) {
        case .file(let u): url = u
        case .refused(let why): return noteDelete(.refused("cannot delete \(path): \(why)"))
        }
        guard FileManager.default.fileExists(atPath: url.path) else { return noteDelete(.refused("cannot delete \(path): no such file under the project root")) }
        if isOpen(path), case .refused(let why) = await detachDocument(path) { return noteDelete(.refused("cannot delete \(path): \(why)")) }
        do { try FileManager.default.trashItem(at: url, resultingItemURL: nil) }
        catch { return noteDelete(.refused("cannot delete \(path): \(error.localizedDescription)")) }
        return noteDelete(.trashed(path: path))
    }

    // MARK: reviewed edits, one document at a time

    /// Posts `edits` to the editor one document at a time: the first one
    /// after switching to its document, the next after the editor reports
    /// the previous one applied (or refused — then the rest are dropped and
    /// the status says which document still holds the old reference).
    func applyReferenceEdits(_ edits: [ReferenceRewrite.DocumentEdit]) {
        guard let model = self.model else { return }
        // The active document first, so the visible buffer changes at once.
        let ordered = edits.sorted { a, _ in a.path == model.activePath }
        applyNext(ordered)
    }

    private func applyNext(_ edits: [ReferenceRewrite.DocumentEdit]) {
        guard let model = self.model else { return }
        guard let edit = edits.first else { return }
        let rest = Array(edits.dropFirst())
        if model.activePath != edit.path {
            if case .refused(let why) = switchDocument(to: edit.path) {
                noteStatus("reference rewrite in \(edit.path) not applied: \(why)"); return
            }
            // The editor swaps its text on the next view update; post the edit after it.
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { [weak self] in
                MainActor.assumeIsolated { self?.postReferenceEdit(edit, then: rest) }
            }
            return
        }
        postReferenceEdit(edit, then: rest)
    }

    private func postReferenceEdit(_ edit: ReferenceRewrite.DocumentEdit, then rest: [ReferenceRewrite.DocumentEdit]) {
        guard let model = self.model else { return }
        guard let doc = model.documents.first(where: { $0.path == edit.path }),
              let r = doc.text.rangeOfUTF8(start: edit.byteRange.lowerBound, end: edit.byteRange.upperBound),
              String(doc.text[r]).sameBytes(as: edit.before) else {
            noteStatus("reference rewrite in \(edit.path) not applied: the text changed"); return
        }
        model.pendingEdit = .init(path: edit.path, nsRange: NSRange(r, in: doc.text), text: edit.text,
                                  token: model.nextEditToken(), revision: model.editorRevision)
        if !rest.isEmpty { afterPendingEdit { [weak self] in self?.applyNext(rest) } }
    }

    /// Runs `action` once the editor has consumed the current `pendingEdit`
    /// (applied or refused). Observation-based, so no polling.
    func afterPendingEdit(_ action: @escaping @MainActor () -> Void) {
        guard let model = self.model else { return }
        withObservationTracking { _ = model.pendingEdit } onChange: {
            Task { @MainActor [weak model] in
                guard let model else { return }
                if model.pendingEdit == nil { action() } else { self.afterPendingEdit(action) }
            }
        }
    }

    // MARK: status

    private func noteCreate(_ o: CreateOutcome) -> CreateOutcome {
        switch o { case .created(let p): noteStatus("created \(p)"); case .refused(let why): noteStatus(why) }
        FlashTeXLog.write("project: " + status)
        return o
    }

    private func noteRename(_ o: RenameOutcome) -> RenameOutcome {
        switch o {
        case .renamed(let f, let t, let n): noteStatus("renamed \(f) to \(t)" + (n == 0 ? "" : "; \(n) reference\(n == 1 ? "" : "s") rewritten (undo with ⌘Z)"))
        case .refused(let why): noteStatus(why)
        }
        return o
    }

    private func noteDelete(_ o: DeleteOutcome) -> DeleteOutcome {
        switch o { case .trashed(let p): noteStatus("moved \(p) to the Trash"); case .refused(let why): noteStatus(why) }
        return o
    }
}
