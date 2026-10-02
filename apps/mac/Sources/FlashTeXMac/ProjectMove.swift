import Foundation
import FlashTeXProtocol

/// Moving a project file to another folder — by drag-and-drop in the project
/// tree (SidebarTree.swift → WorkspaceSidebar.swift) or through the "Move
/// to…" sheet (ProjectScaffoldViews.swift) — with every literal file
/// reference that pointed at it rewritten to the new rooted path.
///
/// Rename… (ProjectScaffold.swift) is the template: the same rooted-path
/// checks, the same refusals (entry document, unsaved edits, an existing
/// destination, a referencing document with unsaved edits), the same
/// `FileManager.moveItem` route (there is no helper request for renames or
/// moves; the durable ledger follows through `retarget`), and the same
/// reviewed edit path — one `pendingEdit` per open document, undoable with
/// ⌘Z there. What is new: the reference scan covers the six file-taking
/// commands (`\input`, `\include`, `\includegraphics`, `\bibliography`,
/// `\addbibresource`, `\lstinputlisting`), so figures and bibliographies
/// move too, and documents on disk that are in the include closure but not
/// open are rewritten on disk (reported in the note; not undoable).

// MARK: - file references (pure)

/// A bounded lexical scan of the commands that take a project file, with
/// the UTF-8 byte span of each argument. Shares `ProjectIncludes.scan`'s
/// rules — `%` comments, `\verb` and verbatim environments are skipped, a
/// command name is the maximal run of ASCII letters — and adds an optional
/// `[…]` argument that may span lines (`\includegraphics[width=…,\n…]`)
/// and `\bibliography{a,b}`'s comma-separated list (one reference per item).
enum FileReferences {
    static let commands: Set<String> = ["input", "include", "includegraphics", "bibliography", "addbibresource", "lstinputlisting"]

    struct Reference: Equatable {
        var command: String
        /// The argument as written, whitespace-trimmed.
        var argument: String
        /// Span of `argument` itself (zero-based, end-exclusive UTF-8 bytes).
        var argumentStartByte: Int
        var argumentEndByte: Int
        /// False when the argument contains `\` or `#` and would need expansion.
        var literal: Bool
    }

    static let maxReferences = 256
    /// An optional argument longer than this is not one (an unclosed `[`).
    static let maxOptionalArgumentBytes = 1024

    static func scan(_ text: String, limit: Int = maxReferences) -> [Reference] {
        var scanner = Scanner(bytes: Array(text.utf8), limit: max(0, limit))
        scanner.run()
        return scanner.out
    }

    /// What `argument` becomes when the file at rooted `oldPath` now lives at
    /// rooted `newPath`, or nil when it does not resolve to that file.
    /// Resolution is TeX's: against the project root, `./` allowed, the
    /// extension optional (`\input{sections/a}` → `sections/a.tex`,
    /// `\includegraphics{fig/plot}` → `fig/plot.pdf`). A spelling without the
    /// extension stays without it; a `./` prefix is kept.
    static func rewrite(argument: String, oldPath: String, newPath: String) -> String? {
        guard let normalized = try? ProjectIncludes.normalize(argument) else { return nil }
        let prefix = argument.hasPrefix("./") ? "./" : ""
        if normalized == oldPath { return prefix + newPath }
        let ext = (oldPath as NSString).pathExtension
        if !ext.isEmpty, normalized + "." + ext == oldPath {
            let newExt = (newPath as NSString).pathExtension
            return prefix + (newExt == ext ? String(newPath.dropLast(ext.count + 1)) : newPath)
        }
        if normalized.hasPrefix(oldPath + "/") { return prefix + newPath + normalized.dropFirst(oldPath.count) }
        return nil
    }

    private static let verbatimEnvironments: Set<String> = ["verbatim", "verbatim*", "comment", "lstlisting", "minted", "Verbatim"]

    private struct Scanner {
        let bytes: [UInt8]
        let limit: Int
        var pos = 0
        var out: [Reference] = []

        init(bytes: [UInt8], limit: Int) { self.bytes = bytes; self.limit = limit }

        mutating func run() {
            while pos < bytes.count, out.count < limit {
                switch bytes[pos] {
                case UInt8(ascii: "%"): skipLine()
                case UInt8(ascii: "\\"): command()
                default: pos += 1
                }
            }
        }

        private func isAlpha(_ b: UInt8) -> Bool { (b >= 0x41 && b <= 0x5A) || (b >= 0x61 && b <= 0x7A) }
        private func isSpace(_ b: UInt8) -> Bool { b == 0x20 || b == 0x09 || b == 0x0A || b == 0x0D || b == 0x0C }

        private func charLen(at i: Int) -> Int {
            let b = bytes[i]
            if b < 0x80 { return 1 }
            if b & 0xE0 == 0xC0 { return 2 }
            if b & 0xF0 == 0xE0 { return 3 }
            if b & 0xF8 == 0xF0 { return 4 }
            return 1
        }

        private func slice(_ start: Int, _ end: Int) -> String { String(decoding: bytes[start..<end], as: UTF8.self) }

        private mutating func skipLine() {
            while pos < bytes.count, bytes[pos] != UInt8(ascii: "\n") { pos += 1 }
        }

        private mutating func skipWhitespace() {
            while pos < bytes.count, isSpace(bytes[pos]) { pos += 1 }
        }

        private mutating func command() {
            let start = pos
            pos += 1
            let nameStart = pos
            while pos < bytes.count, isAlpha(bytes[pos]) { pos += 1 }
            if pos == nameStart {
                if pos < bytes.count { pos += charLen(at: pos) } // control symbol
                return
            }
            let name = slice(nameStart, pos)
            switch name {
            case "verb": skipVerb()
            case "begin": maybeSkipVerbatimEnvironment()
            default: if FileReferences.commands.contains(name) { reference(name, start: start) }
            }
        }

        private mutating func skipVerb() {
            if pos < bytes.count, bytes[pos] == UInt8(ascii: "*") { pos += 1 }
            guard pos < bytes.count else { return }
            let delimLen = charLen(at: pos)
            let delim = Array(bytes[pos..<min(pos + delimLen, bytes.count)])
            pos += delimLen
            if let off = find(delim, from: pos) { pos = off + delimLen } else { pos = bytes.count }
        }

        private mutating func maybeSkipVerbatimEnvironment() {
            let save = pos
            skipWhitespace()
            guard let inner = bracedSpan() else { pos = save; return }
            let env = slice(inner.start, inner.end).trimmingCharacters(in: .whitespacesAndNewlines)
            guard verbatimEnvironments.contains(env) else { pos = save; return }
            let marker = Array("\\end{\(env)}".utf8)
            if let off = find(marker, from: pos) { pos = off + marker.count } else { pos = bytes.count }
        }

        private func find(_ needle: [UInt8], from: Int) -> Int? {
            guard !needle.isEmpty, needle.count <= bytes.count - from else { return nil }
            var i = from
            while i + needle.count <= bytes.count {
                if bytes[i] == needle[0], Array(bytes[i..<i + needle.count]) == needle { return i }
                i += 1
            }
            return nil
        }

        private mutating func bracedSpan() -> (start: Int, end: Int)? {
            guard pos < bytes.count, bytes[pos] == UInt8(ascii: "{") else { return nil }
            let innerStart = pos + 1
            var depth = 0
            var i = pos
            while i < bytes.count {
                switch bytes[i] {
                case UInt8(ascii: "\\"):
                    i += 1
                    if i < bytes.count { i += charLen(at: i) }
                    continue
                case UInt8(ascii: "{"): depth += 1
                case UInt8(ascii: "}"):
                    depth -= 1
                    if depth == 0 { pos = i + 1; return (innerStart, i) }
                default: break
                }
                i += 1
            }
            pos = bytes.count
            return nil
        }

        /// `[…]` may span lines (graphics keys); an unclosed or overlong one is
        /// left alone so the scan never eats the rest of the document.
        private mutating func skipOptionalArgument() {
            guard pos < bytes.count, bytes[pos] == UInt8(ascii: "[") else { return }
            var i = pos
            let end = min(bytes.count, pos + FileReferences.maxOptionalArgumentBytes)
            while i < end, bytes[i] != UInt8(ascii: "]") { i += 1 }
            if i < end, bytes[i] == UInt8(ascii: "]") { pos = i + 1 }
        }

        private mutating func reference(_ command: String, start: Int) {
            let afterName = pos
            if pos < bytes.count, bytes[pos] == UInt8(ascii: "*") { pos += 1 }
            skipWhitespace()
            skipOptionalArgument()
            skipWhitespace()
            if let inner = bracedSpan() {
                if command == "bibliography" {
                    var itemStart = inner.start
                    for i in inner.start...inner.end where i == inner.end || bytes[i] == UInt8(ascii: ",") {
                        push(command, argStart: itemStart, argEnd: i)
                        itemStart = i + 1
                    }
                } else {
                    push(command, argStart: inner.start, argEnd: inner.end)
                }
                return
            }
            if command == "input" { // bare `\input name` (TeX primitive form)
                let nameStart = pos
                while pos < bytes.count {
                    let b = bytes[pos]
                    if isSpace(b) || b == UInt8(ascii: "\\") || b == UInt8(ascii: "%") || b == UInt8(ascii: "{") || b == UInt8(ascii: "}") { break }
                    pos += charLen(at: pos)
                }
                if pos > nameStart { push(command, argStart: nameStart, argEnd: pos); return }
            }
            pos = afterName
        }

        private mutating func push(_ command: String, argStart: Int, argEnd: Int) {
            var a = argStart, b = argEnd
            while a < b, isSpace(bytes[a]) { a += 1 }
            while b > a, isSpace(bytes[b - 1]) { b -= 1 }
            guard a < b, out.count < limit else { return }
            let argument = slice(a, b)
            out.append(Reference(command: command, argument: argument, argumentStartByte: a, argumentEndByte: b,
                                 literal: !argument.contains("\\") && !argument.contains("#")))
        }
    }
}

// MARK: - move: reference rewrite plan

extension ReferenceRewrite {
    /// Edits for moving the file at rooted `oldPath` to rooted `newPath`:
    /// one grouped replacement per document, covering every literal
    /// reference (all six commands) that resolves to the moved file.
    static func planMove(oldPath: String, newPath: String, documents: [RuntimeV1.Document]) -> [DocumentEdit] {
        var out: [DocumentEdit] = []
        for doc in documents {
            let hits: [(start: Int, end: Int, text: String)] = FileReferences.scan(doc.text).compactMap { ref in
                guard ref.literal, let text = FileReferences.rewrite(argument: ref.argument, oldPath: oldPath, newPath: newPath) else { return nil }
                return (ref.argumentStartByte, ref.argumentEndByte, text)
            }
            guard let first = hits.first, let last = hits.last else { continue }
            let bytes = Array(doc.text.utf8)
            let lo = first.start, hi = last.end
            var text = ""
            var cursor = lo
            for hit in hits {
                text += String(decoding: bytes[cursor..<hit.start], as: UTF8.self)
                text += hit.text
                cursor = hit.end
            }
            text += String(decoding: bytes[cursor..<hi], as: UTF8.self)
            guard let r = doc.text.rangeOfUTF8(start: lo, end: hi) else { continue }
            out.append(DocumentEdit(path: doc.path, byteRange: lo..<hi, nsRange: NSRange(r, in: doc.text),
                                    before: String(doc.text[r]), text: text, count: hits.count))
        }
        return out
    }
}

// MARK: - move target (pure)

/// Where a move lands and why it may not: the destination is `folder` (a
/// rooted folder path; empty = the project root) plus the file's own name.
enum MoveTarget {
    enum Failure: Error, Equatable {
        case invalid(String)
        case ontoSelf
        case intoDescendant
        case sameFolder
        case isEntry

        var text: String {
            switch self {
            case .invalid(let why): why
            case .ontoSelf: "cannot move a file into itself"
            case .intoDescendant: "cannot move a file into a folder under it"
            case .sameFolder: "it is already in that folder"
            case .isEntry: "\(ProjectTemplate.entryPath) is the entry document's name"
            }
        }
    }

    /// The rooted path `path` has after moving into `rawFolder`.
    static func resolve(path: String, intoFolder rawFolder: String) -> Result<String, Failure> {
        let trimmed = rawFolder.trimmingCharacters(in: .whitespacesAndNewlines)
        let folder: String
        if trimmed.isEmpty || trimmed == "." || trimmed == "./" {
            folder = ""
        } else {
            do { folder = try ProjectIncludes.normalize(trimmed) } catch { return .failure(.invalid("\(error)")) }
        }
        if folder == path { return .failure(.ontoSelf) }
        if folder.hasPrefix(path + "/") { return .failure(.intoDescendant) }
        if folder == Self.folder(of: path) { return .failure(.sameFolder) }
        let name = path.split(separator: "/").last.map(String.init) ?? path
        let newPath = folder.isEmpty ? name : folder + "/" + name
        if newPath == ProjectTemplate.entryPath { return .failure(.isEntry) }
        return .success(newPath)
    }

    /// The rooted folder `path` lives in ("" at the root).
    static func folder(of path: String) -> String {
        path.split(separator: "/").dropLast().joined(separator: "/")
    }
}

/// How the project tree's rows map to drag sources and drop targets. The
/// tree is flat (WorkspaceSidebar.swift): there are no folder rows, so a
/// file row stands in for the folder it lives in — dropping on
/// `chapters/b.tex` moves into `chapters/`; dropping on the tree's empty
/// space moves to the project root.
enum ProjectTreeMove {
    /// The file a row stands for: open members by path, closed includes by
    /// their resolved path; nil for missing-file, caption and outline rows.
    static func path(forRowID id: String) -> String? {
        if id.hasPrefix("missing:") || id.hasPrefix("outline:") { return nil }
        if id.hasPrefix("closed:") { return String(id.dropFirst("closed:".count)) }
        return id
    }

    /// The folder a drop on row `id` lands in (nil id: the background, i.e.
    /// the root); nil when the row is not a target.
    static func dropFolder(rowID: String?) -> String? {
        guard let rowID else { return "" }
        guard let path = path(forRowID: rowID) else { return nil }
        return MoveTarget.folder(of: path)
    }
}

// MARK: - model operation

extension ProjectDocuments {
    enum MoveOutcome: Equatable {
        /// `open`: documents whose buffers got a reviewed (undoable) edit;
        /// `onDisk`: closed closure documents rewritten in place.
        case moved(from: String, to: String, references: Int, open: [String], onDisk: [String])
        case refused(String)
    }

    /// Move (drag-and-drop / Move to…): relocates the file into `folder`
    /// (rooted; empty = root), retargets the open member if it is one, posts
    /// one undoable reference rewrite per open document that references it,
    /// and rewrites closed documents of the include closure on disk. Refused
    /// on the rename rules (entry, unsaved edits, existing destination, a
    /// referencing buffer with unsaved edits, an edit still pending), onto
    /// itself, into a folder under it, or into the folder it is already in.
    func moveDocument(_ path: String, intoFolder folder: String) async -> MoveOutcome {
        guard let model = self.model else { return noteMove(.refused("cannot move \(path): the project was closed")) }
        if let why = changeRefusal(for: path) { return noteMove(.refused("cannot move \(path): \(why)")) }
        let newPath: String
        switch MoveTarget.resolve(path: path, intoFolder: folder) {
        case .success(let p): newPath = p
        case .failure(let f): return noteMove(.refused("cannot move \(path): \(f.text)"))
        }
        if isOpen(newPath) { return noteMove(.refused("cannot move \(path): \(newPath) is open")) }
        guard let root = projectRoot else { return noteMove(.refused("cannot move \(path): no project root")) }
        let from: URL, to: URL
        switch (Self.rootedFile(path, under: root), newFileURL(newPath)) {
        case (.file(let f), .success(let t)): from = f; to = t
        case (.refused(let why), _): return noteMove(.refused("cannot move \(path): \(why)"))
        case (_, .failure(let r)): return noteMove(.refused("cannot move \(path): \(r.why)"))
        }
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: from.path, isDirectory: &isDirectory) else {
            return noteMove(.refused("cannot move \(path): no such file under the project root"))
        }
        if isDirectory.boolValue { return noteMove(.refused("cannot move \(path): it is a folder")) }
        if FileManager.default.fileExists(atPath: to.path) { return noteMove(.refused("cannot move \(path): \(newPath) already exists")) }
        let edits = ReferenceRewrite.planMove(oldPath: path, newPath: newPath, documents: model.documents)
        if let dirty = edits.first(where: { $0.path != entryPath && isDirty($0.path) }) ?? edits.first(where: { $0.path == entryPath && isDirty($0.path) }) {
            return noteMove(.refused("cannot move \(path): \(dirty.path) references it and has unsaved edits; save it first"))
        }
        if model.pendingEdit != nil { return noteMove(.refused("cannot move \(path): an edit is still pending in the editor")) }
        // Closed closure documents are read before the move (the moved file
        // may be one of them); their rewrite happens after it.
        let diskEdits = ReferenceRewrite.planMove(oldPath: path, newPath: newPath, documents: closedClosureDocuments(excluding: path))
        do {
            try FileManager.default.createDirectory(at: to.deletingLastPathComponent(), withIntermediateDirectories: true)
            try FileManager.default.moveItem(at: from, to: to)
        } catch {
            return noteMove(.refused("cannot move \(path): \(error.localizedDescription)"))
        }
        if let i = model.documents.firstIndex(where: { $0.path == path }) {
            model.documents[i].path = newPath
            if model.activePath == path { model.activePath = newPath }
            retarget(path, to: newPath)
        }
        var onDisk: [String] = []
        var failed: [String] = []
        for edit in diskEdits {
            guard case .file(let url) = Self.rootedFile(edit.path, under: root),
                  let data = try? Data(contentsOf: url), let text = String(data: data, encoding: .utf8),
                  let r = text.rangeOfUTF8(start: edit.byteRange.lowerBound, end: edit.byteRange.upperBound),
                  String(text[r]).sameBytes(as: edit.before) else { failed.append(edit.path); continue }
            let rewritten = (text as NSString).replacingCharacters(in: NSRange(r, in: text), with: edit.text)
            do { try rewritten.write(to: url, atomically: true, encoding: .utf8); onDisk.append(edit.path) }
            catch { failed.append(edit.path) }
        }
        applyReferenceEdits(edits)
        let outcome = MoveOutcome.moved(from: path, to: newPath,
                                        references: edits.reduce(0) { $0 + $1.count } + diskEdits.filter { onDisk.contains($0.path) }.reduce(0) { $0 + $1.count },
                                        open: edits.map(\.path), onDisk: onDisk)
        return noteMove(outcome, failed: failed)
    }

    /// Documents of the include closure that are on disk but not open
    /// (the sidebar's dimmed rows), read directly under the project root.
    private func closedClosureDocuments(excluding path: String) -> [RuntimeV1.Document] {
        guard let root = projectRoot else { return [] }
        var seen: Set<String> = []
        var out: [RuntimeV1.Document] = []
        for node in discoverClosure().nodes {
            guard node.state == .available, let p = node.resolvedPath, p != path, !isOpen(p), seen.insert(p).inserted,
                  case .file(let url) = Self.rootedFile(p, under: root),
                  let attrs = try? FileManager.default.attributesOfItem(atPath: url.path),
                  (attrs[.size] as? Int ?? 0) <= ProjectIncludes.maxDocumentBytes,
                  let data = try? Data(contentsOf: url), let text = String(data: data, encoding: .utf8) else { continue }
            out.append(.init(path: p, text: text))
        }
        return out
    }

    private func noteMove(_ o: MoveOutcome, failed: [String] = []) -> MoveOutcome {
        switch o {
        case .moved(let f, let t, let n, let open, let onDisk):
            var line = "moved \(f) → \(t)"
            if n == 0 {
                line += "; no references to update"
            } else {
                line += "; \(n) reference\(n == 1 ? "" : "s") updated"
                if !open.isEmpty { line += " in \(open.joined(separator: ", ")) (⌘Z reverts the edit in each document; the file itself stays moved)" }
                if !onDisk.isEmpty { line += (open.isEmpty ? " " : "; ") + "on disk in \(onDisk.joined(separator: ", "))" }
            }
            if !failed.isEmpty { line += "; not updated on disk: \(failed.joined(separator: ", "))" }
            noteStatus(line)
        case .refused(let why): noteStatus(why)
        }
        return o
    }
}
