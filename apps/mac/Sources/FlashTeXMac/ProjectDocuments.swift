import AppKit
import ObjectiveC
import Observation
import FlashTeXProtocol

/// Multi-file LaTeX projects in the shell (lane `mac-multifile`).
///
/// `ShellModel.documents` is the project membership the compiler sees: the
/// entry document first, then every document opened into the session, in a
/// stable open order. This file adds the model layer around it:
///
/// - `ProjectIncludes`: a bounded lexical scan of `\input{…}` / `\include{…}`
///   in a document (a port of `crates/project-files/src/scan.rs`, restricted
///   to the two source-including commands) with runtime-v1 UTF-8 byte spans,
///   and rooted-path normalization mirroring `ProjectPath::normalize`. This is
///   not macro expansion; `\input{\jobname}` is reported as non-literal.
/// - `ProjectDocuments`: opens a discovered (or any rooted) document through
///   the durable helper when the preview controller is attached — with the
///   exact `snapshot` versions and membership generation, so a stale snapshot
///   is refused by the helper and reported, never retried blindly — or reads
///   it directly under the rooted project directory otherwise (no absolute
///   paths, no `..` escape, no symlink component); keeps `ShellModel.documents`
///   in sync with per-document durable revisions; detaches explicitly; and
///   switches the editor between documents while preserving each document's
///   caret/selection. A switch never loses text: every keystroke is already in
///   `documents[i].text`, and a switch is refused while a capture insertion is
///   still pending in the editor (it would land in the wrong document).
///
/// Attached to the model as an associated object (`model.project`), the same
/// way `DocumentFilesState` is, so ShellModel.swift stays untouched.

// MARK: - lexical include discovery

enum ProjectIncludes {
    enum Kind: String, Equatable { case input, include }

    /// One `\input`/`\include` found in a source text. Spans are zero-based,
    /// end-exclusive UTF-8 byte ranges (runtime-v1 `source` convention).
    struct Reference: Equatable {
        var kind: Kind
        /// The referenced name as written (whitespace-trimmed).
        var argument: String
        /// From the backslash through the closing brace (or the bare name).
        var startByte: Int
        var endByte: Int
        /// Span of `argument` itself.
        var argumentStartByte: Int
        var argumentEndByte: Int
        /// False when the argument contains `\` or `#` and would need expansion.
        var literal: Bool

        var sourceRange: RuntimeV1.SourceRange {
            .init(path: "", startByte: startByte, endByte: endByte)
        }
    }

    /// Upper bound on references reported per document (discovery is bounded;
    /// the scan stops once the limit is reached).
    static let maxReferences = 64
    /// Direct reads are bounded like the helper's ledger documents.
    static let maxDocumentBytes = 8 * 1024 * 1024

    private static let verbatimEnvironments: Set<String> = ["verbatim", "verbatim*", "comment", "lstlisting", "minted", "Verbatim"]

    /// Scans `text` for `\input`/`\include` in source order, skipping `%`
    /// comments, `\verb` and verbatim-like environments. A command name is the
    /// maximal run of ASCII letters (`\inputfoo` never matches `\input`).
    static func scan(_ text: String, limit: Int = maxReferences) -> [Reference] {
        var scanner = Scanner(bytes: Array(text.utf8), limit: max(0, limit))
        scanner.run()
        return scanner.out
    }

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

        /// Length of the UTF-8 scalar starting at `i` (1 for a continuation byte).
        private func charLen(at i: Int) -> Int {
            let b = bytes[i]
            if b < 0x80 { return 1 }
            if b & 0xE0 == 0xC0 { return 2 }
            if b & 0xF0 == 0xE0 { return 3 }
            if b & 0xF8 == 0xF0 { return 4 }
            return 1
        }

        private func slice(_ start: Int, _ end: Int) -> String {
            String(decoding: bytes[start..<end], as: UTF8.self)
        }

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
                // Control symbol (`\%`, `\\`, `\{`): skip the symbol character.
                if pos < bytes.count { pos += charLen(at: pos) }
                return
            }
            let name = slice(nameStart, pos)
            switch name {
            case "verb": skipVerb()
            case "begin": maybeSkipVerbatimEnvironment()
            case "input": reference(.input, start: start)
            case "include": reference(.include, start: start)
            default: break
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

        /// If the next byte is `{`, consumes through the matching `}` and
        /// returns the inner span. Braces nest; a backslash escapes the next char.
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
            pos = bytes.count // unbalanced: consume to the end
            return nil
        }

        private mutating func skipOptionalArgument() {
            guard pos < bytes.count, bytes[pos] == UInt8(ascii: "[") else { return }
            var i = pos
            while i < bytes.count, bytes[i] != UInt8(ascii: "]"), bytes[i] != UInt8(ascii: "\n") { i += 1 }
            if i < bytes.count, bytes[i] == UInt8(ascii: "]") { pos = i + 1 }
        }

        private mutating func reference(_ kind: Kind, start: Int) {
            let afterName = pos
            skipWhitespace()
            skipOptionalArgument()
            skipWhitespace()
            if let inner = bracedSpan() {
                push(kind, argStart: inner.start, argEnd: inner.end, start: start, end: pos)
                return
            }
            // Bare `\input name` (TeX primitive form); `\include` requires braces.
            if kind == .input {
                let nameStart = pos
                while pos < bytes.count {
                    let b = bytes[pos]
                    if isSpace(b) || b == UInt8(ascii: "\\") || b == UInt8(ascii: "%") || b == UInt8(ascii: "{") || b == UInt8(ascii: "}") { break }
                    pos += charLen(at: pos)
                }
                if pos > nameStart { push(kind, argStart: nameStart, argEnd: pos, start: start, end: pos); return }
            }
            pos = afterName
        }

        private mutating func push(_ kind: Kind, argStart: Int, argEnd: Int, start: Int, end: Int) {
            var a = argStart, b = argEnd
            while a < b, isSpace(bytes[a]) { a += 1 }
            while b > a, isSpace(bytes[b - 1]) { b -= 1 }
            guard a < b else { return }
            let argument = slice(a, b)
            out.append(Reference(kind: kind, argument: argument, startByte: start, endByte: end,
                                 argumentStartByte: a, argumentEndByte: b,
                                 literal: !argument.contains("\\") && !argument.contains("#")))
        }
    }

    // MARK: rooted paths (mirror of crates/project-files ProjectPath::normalize)

    enum PathError: Error, Equatable, CustomStringConvertible {
        case empty, absolute, escapesRoot, forbiddenCharacter(Character)
        var description: String {
            switch self {
            case .empty: "path is empty"
            case .absolute: "path must be project-relative, not absolute"
            case .escapesRoot: "path escapes the project root via '..'"
            case .forbiddenCharacter(let c): "path contains forbidden character \(String(reflecting: c))"
            }
        }
    }

    /// Normalizes a project-relative path: forward slashes, no `.`/empty
    /// segments, `..` pops (an error when nothing is left), no backslash,
    /// colon, NUL or control characters, no leading `/` or `~`.
    static func normalize(_ raw: String) throws -> String {
        if let c = raw.first(where: { $0 == "\\" || $0 == ":" || $0 == "\0" || $0.unicodeScalars.contains { $0.properties.generalCategory == .control } }) {
            throw PathError.forbiddenCharacter(c)
        }
        if raw.hasPrefix("/") || raw.hasPrefix("~") { throw PathError.absolute }
        var segments: [Substring] = []
        for seg in raw.split(separator: "/", omittingEmptySubsequences: false) {
            switch seg {
            case "", ".": continue
            case "..": if segments.popLast() == nil { throw PathError.escapesRoot }
            default: segments.append(seg)
            }
        }
        if segments.isEmpty { throw PathError.empty }
        return segments.joined(separator: "/")
    }

    /// Rooted candidates for an `\input`/`\include` argument, in TeX's order:
    /// `name.tex` first, then the literal name (a name ending in `.tex` is
    /// tried as written). References resolve against the project root, not
    /// the referencing file's directory (TeX working-directory rule).
    static func candidates(for argument: String) throws -> [String] {
        let base = try normalize(argument)
        let name = base.split(separator: "/").last.map(String.init) ?? base
        if name.hasSuffix(".tex"), name.count > 4 { return [base] }
        return [base + ".tex", base]
    }
}

// MARK: - project membership

/// One member of the project as the shell sees it.
struct ProjectDocument: Equatable, Identifiable {
    enum Role: Equatable {
        case entry
        /// Opened because `from` references it (`\input`/`\include`).
        case included(from: String)
        /// Opened explicitly by path.
        case opened
    }
    enum Origin: Equatable {
        /// Text came with the fixture/buffer (no file behind it that this lane read).
        case buffer
        /// Imported through the preview controller's durable ledger.
        case helper
        /// Read directly under the rooted project directory.
        case disk
    }
    var path: String
    var role: Role
    var origin: Origin
    /// Durable revision the helper reported for this path (nil off the helper route).
    var durableRevision: Int?
    /// Whether the buffer differs from the text it was opened with (entry: its saved text).
    var isDirty: Bool
    var id: String { path }
}

@MainActor
@Observable
final class ProjectDocuments {
    /// One `\input`/`\include` in the entry document and what it resolves to.
    struct Discovered: Equatable {
        enum State: Equatable {
            /// Already a project member under `resolvedPath`.
            case open
            /// Resolvable and not open yet (direct mode: exists on disk).
            case available
            /// Cannot be resolved: not literal, not rooted, or missing.
            case unresolvable(String)
        }
        var reference: ProjectIncludes.Reference
        /// The rooted candidate paths tried, in order.
        var candidates: [String]
        var resolvedPath: String?
        var state: State
    }

    enum OpenOutcome: Equatable {
        case opened(path: String)
        case alreadyOpen(path: String)
        case refused(String)
    }

    /// What a detach means today: the helper offers no persistent exclusion
    /// (STDIO.md: "Project restart restores all retained documents; persistent
    /// exclusions are not implemented"), and the direct route has no project
    /// file either — a detach lasts for this session only.
    static let detachScopeNote = "this session only: the helper restores retained documents on restart and includes are rediscovered from the entry"

    enum DetachOutcome: Equatable {
        case detached(path: String)
        case refused(String)
    }

    enum SaveOutcome: Equatable {
        case saved(path: String, sha256: String)
        case conflict(DocumentConflict)
        case failed(String)
    }

    enum SwitchOutcome: Equatable {
        case switched(to: String, restoredCaret: NSRange)
        case unchanged
        case refused(String)
    }

    /// Bounded wait for one helper reply (`snapshot`, `open_document`, …).
    var helperTimeout: TimeInterval = 10
    /// Human-readable state of the last membership operation.
    private(set) var status = "no project operation yet"
    /// Last `snapshot`/`open_document`/`detach_document` versions and generation
    /// seen from the helper (nil until the first helper operation).
    private(set) var membershipGeneration: Int?
    private(set) var sourceVersions: [String: Int] = [:]
    /// Text of a detached document that still had unsaved edits (recoverable this session).
    private(set) var detachedBuffers: [String: String] = [:]

    @ObservationIgnored unowned let model: ShellModel // read by ProjectScaffold.swift (New File / Rename / Delete)
    @ObservationIgnored private var roles: [String: ProjectDocument.Role] = [:]
    @ObservationIgnored private var origins: [String: ProjectDocument.Origin] = [:]
    /// Text each non-entry document had when this lane opened it (dirty baseline).
    @ObservationIgnored private var baselines: [String: String] = [:]
    /// SHA-256 of the disk file each non-entry document was opened from (nil:
    /// no file seen), the mandatory expectation of a rooted save.
    @ObservationIgnored private var diskBaselines: [String: String?] = [:]
    /// Explicit conflict of each member's last save, by member path (nil once
    /// resolved by a later save of that member): conflicts on two members never
    /// overwrite each other (#789).
    private(set) var saveConflicts: [String: DocumentConflict] = [:]

    func saveConflict(for path: String) -> DocumentConflict? { saveConflicts[path] }

    /// A replaced project inherits no member conflicts (`replaceProject`).
    func clearSaveConflicts() { if !saveConflicts.isEmpty { saveConflicts = [:] } }
    /// Caret/selection (UTF-16) last seen in each document.
    @ObservationIgnored private(set) var carets: [String: NSRange] = [:]
    @ObservationIgnored private var armed = false
    @ObservationIgnored private var controllerArmed = false
    @ObservationIgnored private var syncing = false
    /// Number of `syncWithHelper` passes that sent at least one request (tests).
    @ObservationIgnored private(set) var helperSyncs = 0

    init(model: ShellModel) {
        self.model = model
        armActivePathTracking()
        armControllerTracking()
        // Demo/automation hook (like FLASHTEX_SEED_FILE): open the entry
        // document's includes at launch and optionally start in one of them.
        let env = ProcessInfo.processInfo.environment
        if env["FLASHTEX_OPEN_INCLUDES"] == "1" {
            Task { @MainActor [weak self] in
                guard let self else { return }
                _ = await self.openDiscoveredIncludes()
                if let path = env["FLASHTEX_ACTIVE_PATH"] { self.switchDocument(to: path) }
            }
        }
    }

    /// Drops metadata for paths no longer in `ShellModel.documents`
    /// (`replaceProject`, fixtures) so a later project cannot inherit them.
    private func prune() {
        let open = Set(model.documents.map(\.path))
        for key in roles.keys where !open.contains(key) { roles.removeValue(forKey: key) }
        for key in origins.keys where !open.contains(key) { origins.removeValue(forKey: key) }
        for key in baselines.keys where !open.contains(key) { baselines.removeValue(forKey: key) }
        for key in carets.keys where !open.contains(key) { carets.removeValue(forKey: key) }
        for key in diskBaselines.keys where !open.contains(key) { diskBaselines.removeValue(forKey: key) }
        for key in saveConflicts.keys where !open.contains(key) { saveConflicts.removeValue(forKey: key) }
    }

    /// Status line from another file's operation (ProjectScaffold.swift).
    func noteStatus(_ line: String) { status = line; FlashTeXLog.write("project: " + line) }

    /// Moves a member's metadata to its new path after a rename (ProjectScaffold.swift).
    func retarget(_ path: String, to newPath: String) {
        roles[newPath] = roles.removeValue(forKey: path)
        origins[newPath] = origins.removeValue(forKey: path)
        baselines[newPath] = baselines.removeValue(forKey: path)
        if let disk = diskBaselines.removeValue(forKey: path) { diskBaselines[newPath] = disk }
        carets[newPath] = carets.removeValue(forKey: path)
        saveConflicts[newPath] = saveConflicts.removeValue(forKey: path)
    }

    // MARK: membership view

    /// The entry document: first in `ShellModel.documents` (the order this
    /// lane maintains; `replaceProject`/fixtures also put the entry first).
    var entryPath: String { model.documents.first?.path ?? model.activePath }

    /// Project members in `ShellModel.documents` order (entry first), with
    /// this lane's metadata and the helper's durable revision per path.
    var listing: [ProjectDocument] {
        model.documents.enumerated().map { i, doc in
            ProjectDocument(path: doc.path,
                            role: i == 0 ? .entry : roles[doc.path] ?? .opened,
                            origin: i == 0 ? (model.documentURL == nil ? .buffer : .disk) : origins[doc.path] ?? .buffer,
                            durableRevision: model.controllerState.durable[doc.path]?.revision,
                            isDirty: isDirty(doc.path))
        }
    }

    func isOpen(_ path: String) -> Bool { model.documents.contains { $0.path == path } }

    /// Whether `path`'s buffer differs from the text this lane opened it with.
    /// The entry document's dirtiness is `ShellModel.isDirty` (saved text).
    func isDirty(_ path: String) -> Bool {
        guard let doc = model.documents.first(where: { $0.path == path }) else { return false }
        if path == entryPath {
            // `ShellModel.isDirty` compares the saved text with the *active*
            // buffer; compare it with the entry buffer whichever document is active.
            if let saved = model.savedText { return !saved.sameBytes(as: doc.text) }
            return model.documentURL == nil && model.editorRevision > 1 && !doc.text.isEmpty
        }
        guard let baseline = baselines[path] else { return false }
        return !baseline.sameBytes(as: doc.text)
    }

    /// Any member with unsaved edits (quit/open flows).
    var anyDirty: Bool { listing.contains { $0.isDirty } }

    /// The rooted project directory: the entry document's directory (nil for
    /// an unsaved buffer — nothing can be resolved against it).
    var projectRoot: URL? { model.documentURL?.deletingLastPathComponent().standardizedFileURL }

    // MARK: discovery

    /// `\input`/`\include` targets of the entry document (or `path`), each
    /// with its rooted resolution: open, available, or why not. Bounded to
    /// `ProjectIncludes.maxReferences`.
    func discoverIncludes(in path: String? = nil) -> [Discovered] {
        prune()
        let from = path ?? entryPath
        guard let text = model.documents.first(where: { $0.path == from })?.text else { return [] }
        return resolve(ProjectIncludes.scan(text), route: model.controllerAttached)
    }

    // MARK: transitive discovery

    /// Nesting bound for the include closure (entry = depth 0).
    nonisolated static let maxIncludeDepth = 8
    /// Total documents the closure may name (the helper's project bound).
    nonisolated static let maxClosureDocuments = 256

    /// One reference in the include closure of the entry document.
    struct ClosureNode: Equatable {
        var reference: ProjectIncludes.Reference
        /// Document the reference was found in.
        var from: String
        /// 1 for the entry's own includes, up to `maxIncludeDepth`.
        var depth: Int
        var candidates: [String]
        var resolvedPath: String?
        var state: Discovered.State
        /// A diamond: the path was already reached through another reference
        /// (not scanned again, not a cycle).
        var duplicate = false
    }

    struct Closure: Equatable {
        /// Nodes in stable depth-first source order.
        var nodes: [ClosureNode]
        /// Resolved paths in first-reached order (entry excluded).
        var paths: [String]
        var truncated = false
        var unresolvable: [ClosureNode] { nodes.filter { if case .unresolvable = $0.state { true } else { false } } }
    }

    /// Text a document contributes to discovery: the open buffer, else the
    /// rooted disk file (both routes; on the helper route the ledger may
    /// differ from disk until the document is opened, which then reads the
    /// ledger). Nil when the text is not available.
    private func discoveryText(for path: String) -> String? {
        if let doc = model.documents.first(where: { $0.path == path }) { return doc.text }
        guard let root = projectRoot, case .file(let url) = Self.rootedFile(path, under: root),
              let attrs = try? FileManager.default.attributesOfItem(atPath: url.path),
              (attrs[.size] as? Int ?? 0) <= ProjectIncludes.maxDocumentBytes,
              let data = try? Data(contentsOf: url) else { return nil }
        return String(data: data, encoding: .utf8)
    }

    /// The transitive `\input`/`\include` closure of the entry document
    /// (chapter → section → …), depth-first in source order, bounded by
    /// `maxIncludeDepth` and `maxClosureDocuments`. A reference back to a
    /// document on the current include chain is refused as a cycle; a
    /// document reached twice through different parents is listed once as
    /// `duplicate`. Lexical only: no macro expansion.
    func discoverClosure(maxDepth: Int = ProjectDocuments.maxIncludeDepth) -> Closure {
        prune()
        var closure = Closure(nodes: [], paths: [])
        var reached: Set<String> = [entryPath]
        var chain: [String] = [entryPath]
        func visit(_ path: String, depth: Int) {
            guard depth <= maxDepth, let text = discoveryText(for: path) else { return }
            for d in resolve(ProjectIncludes.scan(text), route: model.controllerAttached) {
                var node = ClosureNode(reference: d.reference, from: path, depth: depth, candidates: d.candidates,
                                       resolvedPath: d.resolvedPath, state: d.state)
                if let resolved = d.resolvedPath ?? d.candidates.first(where: reached.contains) {
                    if chain.contains(resolved) {
                        node.resolvedPath = resolved
                        node.state = .unresolvable("\\\(d.reference.kind.rawValue){\(d.reference.argument)} closes an include cycle: "
                                                   + (chain + [resolved]).joined(separator: " → "))
                        closure.nodes.append(node)
                        continue
                    }
                    if reached.contains(resolved) {
                        node.resolvedPath = resolved
                        node.duplicate = true
                        node.state = isOpen(resolved) ? .open : .available
                        closure.nodes.append(node)
                        continue
                    }
                }
                if node.resolvedPath != nil, depth + 1 > maxDepth {
                    node.state = .unresolvable("nested deeper than \(maxDepth) levels; not discovered")
                }
                closure.nodes.append(node)
                guard let resolved = node.resolvedPath else { continue }
                if case .unresolvable = node.state { continue }
                guard closure.paths.count < Self.maxClosureDocuments else { closure.truncated = true; continue }
                reached.insert(resolved)
                closure.paths.append(resolved)
                chain.append(resolved)
                visit(resolved, depth: depth + 1)
                chain.removeLast()
            }
        }
        visit(entryPath, depth: 0)
        return closure
    }

    // MARK: implicit include-closure documents

    /// One document in the include closure that is not an open project
    /// member: a rooted path and the text last read from disk for it. Role
    /// `.implicit` — never a project member (`ShellModel.documents` never
    /// gets it), never durable (no `open_document`/`edit` for it), never
    /// written. Exists only for the compile request and this lane's file
    /// watchers; opening it the ordinary way (a click on its sidebar row,
    /// `openDocument`) promotes it to a real member and it stops being
    /// reported here, and detaching it reverts it to implicit on the next
    /// `discoverClosure()` — there is no separate state to update either way.
    struct ImplicitDocument: Equatable { var path: String; var text: String }

    @ObservationIgnored private var implicitWatchers: [String: DocumentWatcher] = [:]
    @ObservationIgnored private var implicitDebounce: DispatchWorkItem?

    /// The compile request's documents beyond the open project members:
    /// every `discoverClosure()` path that is not already open, read fresh
    /// through the same rooted reader `openDirectly` uses (no symlink on any
    /// path component, `ProjectIncludes.maxDocumentBytes` bound, valid
    /// UTF-8). A path that cannot be read this way right now (raced away
    /// since discovery, grew past the bound, not UTF-8) is left out rather
    /// than fabricated: the compiler then reports its ordinary "referenced
    /// but missing" diagnostic for that include, exactly as for an unopened
    /// include today.
    ///
    /// Also rearms this lane's per-path `DocumentWatcher`s to exactly the
    /// current implicit set, so an on-disk edit to one of them schedules a
    /// recompile coalesced behind the existing auto-compile debounce
    /// (`ShellModel.debounceInterval`) — never the entry's own watcher, never
    /// the durable ledger.
    ///
    /// Direct/worker route only (`ShellModel.compile()`). The durable helper
    /// (preview-controller) compiles from its own ledger membership and its
    /// `compile`/`edit` requests carry no field for a read-only, non-durable
    /// document (`crates/preview-controller/STDIO.md`), so a project attached
    /// to it still needs an explicit open for an unopened include; extending
    /// that route needs a helper-side protocol change, out of this lane's scope.
    func implicitClosureDocuments() -> [ImplicitDocument] {
        guard let root = projectRoot else { rearmImplicitWatchers([:]); return [] }
        let closure = discoverClosure()
        let open = Set(model.documents.map(\.path))
        var out: [ImplicitDocument] = []
        var urls: [String: URL] = [:]
        for path in closure.paths where !open.contains(path) {
            guard case .file(let url) = Self.rootedFile(path, under: root) else { continue }
            urls[path] = url
            guard let attrs = try? FileManager.default.attributesOfItem(atPath: url.path),
                  (attrs[.size] as? Int ?? 0) <= ProjectIncludes.maxDocumentBytes,
                  let data = try? Data(contentsOf: url),
                  let text = String(data: data, encoding: .utf8) else { continue }
            out.append(ImplicitDocument(path: path, text: text))
        }
        rearmImplicitWatchers(urls)
        return out
    }

    /// Keeps exactly one `DocumentWatcher` per path the last
    /// `implicitClosureDocuments()` pass considered (including one whose read
    /// failed — a symlink or size violation a later edit might fix, or a file
    /// about to reappear): stops watchers for paths no longer implicit (now
    /// open, or no longer in the closure), starts one for each new path.
    private func rearmImplicitWatchers(_ current: [String: URL]) {
        for path in implicitWatchers.keys where current[path] == nil {
            implicitWatchers.removeValue(forKey: path)?.stop()
        }
        for (path, url) in current where implicitWatchers[path]?.url != url {
            let watcher = implicitWatchers[path] ?? DocumentWatcher()
            watcher.onChange = { [weak self] in self?.scheduleImplicitRecompile() }
            watcher.watch(url)
            implicitWatchers[path] = watcher
        }
    }

    /// An implicit document changed on disk: recompile, coalesced exactly
    /// like a keystroke burst (`ShellModel.debounceInterval`), so several
    /// implicit files touched together (a `git checkout`, a build script)
    /// become one compile.
    private func scheduleImplicitRecompile() {
        let model = self.model // strong across the debounce delay (see flushToHelper)
        guard model.autoCompile, model.workerAttached else { return }
        implicitDebounce?.cancel()
        if ShellModel.debounceInterval == 0 { model.compile(); return }
        let item = DispatchWorkItem { [weak model] in model?.compile() }
        implicitDebounce = item
        DispatchQueue.main.asyncAfter(deadline: .now() + ShellModel.debounceInterval, execute: item)
    }

    /// Resolves scanned references like `discoverIncludes` does.
    private func resolve(_ refs: [ProjectIncludes.Reference], route helper: Bool) -> [Discovered] {
        refs.map { ref in
            guard ref.literal else {
                return Discovered(reference: ref, candidates: [], resolvedPath: nil, state: .unresolvable("argument needs macro expansion"))
            }
            let candidates: [String]
            do { candidates = try ProjectIncludes.candidates(for: ref.argument) }
            catch { return Discovered(reference: ref, candidates: [], resolvedPath: nil, state: .unresolvable("\(error)")) }
            if let open = candidates.first(where: isOpen) {
                return Discovered(reference: ref, candidates: candidates, resolvedPath: open, state: .open)
            }
            if helper, let known = candidates.first(where: { sourceVersions[$0] != nil }) {
                return Discovered(reference: ref, candidates: candidates, resolvedPath: known, state: .available)
            }
            guard let root = projectRoot else {
                return Discovered(reference: ref, candidates: candidates, resolvedPath: nil, state: .unresolvable("no project root (the entry document is not saved)"))
            }
            for candidate in candidates {
                switch Self.rootedFile(candidate, under: root) {
                case .file(let url) where FileManager.default.fileExists(atPath: url.path):
                    return Discovered(reference: ref, candidates: candidates, resolvedPath: candidate, state: .available)
                case .refused(let why):
                    return Discovered(reference: ref, candidates: candidates, resolvedPath: nil, state: .unresolvable(why))
                default: continue
                }
            }
            if helper {
                // The helper may still open it (retained ledger); let it decide.
                return Discovered(reference: ref, candidates: candidates, resolvedPath: candidates[0], state: .available)
            }
            return Discovered(reference: ref, candidates: candidates, resolvedPath: nil, state: .unresolvable("no such file under the project root"))
        }
    }

    /// Result of the last `openDiscoveredIncludes` ("Open All Includes").
    struct OpenReport: Equatable {
        var opened: [String] = []
        var alreadyOpen: [String] = []
        var refused: [String] = []
        /// `"\input{x} in main.tex: why"` for each unresolvable reference.
        var unresolvable: [String] = []
        var truncated = false
        var summary: String {
            var parts: [String] = []
            parts.append("opened \(opened.count)" + (opened.isEmpty ? "" : " (" + opened.joined(separator: ", ") + ")"))
            if !alreadyOpen.isEmpty { parts.append("\(alreadyOpen.count) already open") }
            if !refused.isEmpty { parts.append("refused: " + refused.joined(separator: "; ")) }
            if !unresolvable.isEmpty { parts.append("unresolvable: " + unresolvable.joined(separator: "; ")) }
            if truncated { parts.append("closure truncated at \(ProjectDocuments.maxClosureDocuments) documents") }
            return parts.joined(separator: "; ")
        }
    }
    private(set) var lastOpenReport: OpenReport?

    /// Opens the entry document's include closure in stable depth-first
    /// source order (chapter.tex before the sections it includes), through
    /// the helper or directly, and reports what could not be resolved
    /// (`lastOpenReport`, `status`). Rediscovers after each open so a
    /// document read from the helper's ledger contributes its own includes.
    @discardableResult
    func openDiscoveredIncludes() async -> [OpenOutcome] {
        let model = self.model // strong across the helper round trips (see flushToHelper)
        defer { withExtendedLifetime(model) {} }
        var outcomes: [OpenOutcome] = []
        var report = OpenReport()
        var attempted: Set<String> = []
        var progress = true
        while progress {
            progress = false
            let closure = discoverClosure()
            report.truncated = closure.truncated
            for node in closure.nodes where node.state == .available && !node.duplicate {
                guard let path = node.resolvedPath, !attempted.contains(path) else { continue }
                attempted.insert(path)
                let outcome = await openDocument(path, role: .included(from: node.from))
                outcomes.append(outcome)
                switch outcome {
                case .opened(let p): report.opened.append(p); progress = true
                case .alreadyOpen(let p): report.alreadyOpen.append(p)
                case .refused(let why): report.refused.append(why)
                }
            }
        }
        let closure = discoverClosure()
        report.unresolvable = closure.unresolvable.map {
            "\\\($0.reference.kind.rawValue){\($0.reference.argument)} in \($0.from): " + { if case .unresolvable(let why) = $0.state { why } else { "" } }($0)
        }
        report.truncated = report.truncated || closure.truncated
        lastOpenReport = report
        status = "open all includes: " + report.summary
        FlashTeXLog.write("project: " + status)
        return outcomes
    }

    // MARK: open

    /// Opens the document an `\input`/`\include` argument refers to, trying
    /// the rooted candidates in TeX order (`name.tex`, then `name`): an open
    /// member wins, else the first candidate that exists (direct mode) or the
    /// first one the helper accepts.
    func openInclude(_ argument: String, role: ProjectDocument.Role? = nil) async -> OpenOutcome {
        let candidates: [String]
        do { candidates = try ProjectIncludes.candidates(for: argument) }
        catch { return note(.refused("\(argument): \(error)")) }
        if let open = candidates.first(where: isOpen) { return .alreadyOpen(path: open) }
        let role = role ?? .included(from: entryPath)
        var last: OpenOutcome = .refused("\(argument): no candidate")
        for candidate in candidates {
            if !model.controllerAttached, let root = projectRoot, case .file(let url) = Self.rootedFile(candidate, under: root),
               !FileManager.default.fileExists(atPath: url.path) {
                last = .refused("cannot open \(candidate): no such file under the project root")
                continue
            }
            last = await openDocument(candidate, role: role)
            if case .refused = last { continue }
            return last
        }
        return note(last)
    }

    /// Opens a rooted document into the project: through the helper when the
    /// controller is attached (exact snapshot, stale refused), else from disk
    /// under the project root. Appends to `ShellModel.documents` (entry stays
    /// first; open order is stable) and records the dirty baseline.
    func openDocument(_ rawPath: String, role: ProjectDocument.Role = .opened) async -> OpenOutcome {
        prune()
        let path: String
        do { path = try ProjectIncludes.normalize(rawPath) }
        catch { return note(.refused("\(rawPath): \(error)")) }
        if isOpen(path) { return .alreadyOpen(path: path) }
        if model.controllerAttached { return await openThroughHelper(path, role: role) }
        return openDirectly(path, role: role)
    }

    private func openDirectly(_ path: String, role: ProjectDocument.Role) -> OpenOutcome {
        guard let root = projectRoot else {
            return note(.refused("cannot open \(path): the entry document is not saved, so there is no project root"))
        }
        let url: URL
        switch Self.rootedFile(path, under: root) {
        case .file(let u): url = u
        case .refused(let why): return note(.refused("cannot open \(path): \(why)"))
        }
        guard let attrs = try? FileManager.default.attributesOfItem(atPath: url.path) else {
            return note(.refused("cannot open \(path): no such file under \(root.path)"))
        }
        if let size = attrs[.size] as? Int, size > ProjectIncludes.maxDocumentBytes {
            return note(.refused("cannot open \(path): \(size) bytes exceeds the \(ProjectIncludes.maxDocumentBytes)-byte document limit"))
        }
        guard let data = try? Data(contentsOf: url), let text = String(data: data, encoding: .utf8) else {
            return note(.refused("cannot open \(path): not readable as UTF-8"))
        }
        insert(path: path, text: text, role: role, origin: .disk, diskSHA256: SourceDigest.sha256Hex(text))
        return note(.opened(path: path))
    }

    private func openThroughHelper(_ path: String, role: ProjectDocument.Role) async -> OpenOutcome {
        guard let snapshot = await refreshSnapshot() else {
            return note(.refused("cannot open \(path): \(status)"))
        }
        let document: [String: Any]
        if snapshot.versions[path] != nil {
            // Already a member on the helper side (discovered at startup or
            // retained in its ledger): read the durable document, no open.
            switch await helperRequest("document", ["path": path]) {
            case .success(let payload):
                guard let doc = payload["document"] as? [String: Any] else { return note(.refused("helper document reply for \(path) has no document")) }
                document = doc
            case .failure(let e): return note(.refused("helper could not read \(path): \(e.message)"))
            }
        } else {
            // Exact snapshot: the helper refuses anything stale ("project
            // membership snapshot is stale"); we report it and let the caller
            // re-run with a fresh snapshot rather than retrying blindly.
            let payload: [String: Any] = ["path": path, "source_versions": snapshot.versions, "membership_generation": snapshot.generation]
            switch await helperRequest("open_document", payload) {
            case .success(let reply):
                guard let doc = reply["document"] as? [String: Any] else { return note(.refused("helper open_document reply for \(path) has no document")) }
                document = doc
                adoptMembership(reply)
                if let e = reply["preview_error"] as? String { model.log("controller preview_error after opening \(path): \(e)") }
            case .failure(let e):
                if e.message.contains("stale") {
                    membershipGeneration = nil; sourceVersions = [:]
                    return note(.refused("helper refused open of \(path): \(e.message) (snapshot g\(snapshot.generation) was superseded; run the open again)"))
                }
                return note(.refused("helper refused open of \(path): \(e.message)"))
            }
        }
        guard let text = document["text"] as? String, let revision = document["revision"] as? Int,
              let sha = document["source_sha256"] as? String, document["path"] as? String == path else {
            return note(.refused("helper document for \(path) is missing fields"))
        }
        if isOpen(path) { return .alreadyOpen(path: path) } // raced with another open
        // The ledger text is authoritative; the disk hash (if any) is what a
        // later rooted save must find, so an external edit is a conflict.
        let disk = await diskSHA256(of: path)
        if isOpen(path) { return .alreadyOpen(path: path) }
        insert(path: path, text: text, role: role, origin: .helper, diskSHA256: disk)
        recordDurable(path: path, revision: revision, sha256: sha, text: text)
        return note(.opened(path: path), extra: " (durable r\(revision), membership g\(membershipGeneration ?? snapshot.generation))")
    }

    private func insert(path: String, text: String, role: ProjectDocument.Role, origin: ProjectDocument.Origin, diskSHA256: String?) {
        model.documents.append(.init(path: path, text: text))
        roles[path] = role
        origins[path] = origin
        baselines[path] = text
        diskBaselines[path] = diskSHA256
        saveConflicts[path] = nil // a fresh disk baseline supersedes it
        detachedBuffers.removeValue(forKey: path)
        model.log("project: opened \(path) (\(text.utf8.count) bytes, \(origin)) — \(model.documents.count) documents")
        // Unsaved text kept for this member by an earlier session or detach is
        // offered (DirtySnapshots.swift), never applied over the opened text.
        if let root = projectRoot, let offered = model.offerDirtySnapshot(for: root.appendingPathComponent(path), currentText: text) {
            status = "opened \(path); unsaved text from before is available (\(offered.reason))"
        }
    }

    // MARK: detach

    /// Removes a non-entry document from the project. Unsaved edits are kept
    /// in `detachedBuffers` (never lost silently) but the detach is refused
    /// unless `discardingEdits` says so. On the helper route the membership
    /// change goes through `detach_document` with the exact snapshot first;
    /// a refusal leaves the shell's membership unchanged.
    func detachDocument(_ path: String, discardingEdits: Bool = false) async -> DetachOutcome {
        prune()
        guard path != entryPath else { return note(.refused("cannot detach the entry document \(path)")) }
        guard let doc = model.documents.first(where: { $0.path == path }) else { return note(.refused("\(path) is not open")) }
        if isDirty(path), !discardingEdits {
            return note(.refused("\(path) has unsaved edits; save or discard them before detaching"))
        }
        if model.activePath == path, case .refused(let why) = switchDocument(to: entryPath) {
            return note(.refused("cannot detach \(path): \(why)"))
        }
        if model.controllerAttached, model.controllerState.durable[path] != nil {
            guard let snapshot = await refreshSnapshot() else { return note(.refused("cannot detach \(path): \(status)")) }
            let payload: [String: Any] = ["path": path, "source_versions": snapshot.versions, "membership_generation": snapshot.generation]
            switch await helperRequest("detach_document", payload) {
            case .success(let reply):
                adoptMembership(reply)
                model.controllerState.durable.removeValue(forKey: path)
                model.controllerState.textByDurable.removeValue(forKey: path)
                model.controllerState.editorRevisionByDurable.removeValue(forKey: path)
            case .failure(let e):
                if e.message.contains("stale") { membershipGeneration = nil; sourceVersions = [:] }
                return note(.refused("helper refused detach of \(path): \(e.message)"))
            }
        }
        if isDirty(path) {
            detachedBuffers[path] = doc.text
            // Session memory plus a durable snapshot (the helper's ledger drops
            // a detached document's text with the membership).
            if let root = projectRoot { model.preserveDirtyText(doc.text, at: root.appendingPathComponent(path), reason: "detached from the project with unsaved edits") }
        }
        model.documents.removeAll { $0.path == path }
        roles.removeValue(forKey: path); origins.removeValue(forKey: path); baselines.removeValue(forKey: path)
        carets.removeValue(forKey: path); diskBaselines.removeValue(forKey: path)
        if model.anchor?.path == path { model.anchor = nil }
        model.log("project: detached \(path) — \(model.documents.count) documents")
        return note(.detached(path: path), extra: " (" + Self.detachScopeNote + ")")
    }

    // MARK: editor switch

    /// Makes `path` the editor document. The outgoing document's caret and
    /// selection are remembered; the incoming one's are restored (clamped to
    /// its current text, on a scalar boundary), via `ShellModel.selection` so
    /// the editor applies them once its text is swapped. Refused while a
    /// capture insertion is still pending in the editor: it carries no
    /// document check and would be applied to the wrong buffer.
    @discardableResult
    func switchDocument(to path: String) -> SwitchOutcome {
        prune()
        guard path != model.activePath else { return .unchanged }
        guard isOpen(path) else { return .refused("\(path) is not open") }
        if let pending = model.pendingEdit {
            return .refused("a capture insertion into \(pending.path) is still pending; try again in a moment")
        }
        let outgoing = model.activePath
        rememberCaret(for: outgoing)
        model.activePath = path
        // The controller's one-slot edit queue follows `activePath`: a keystroke
        // still queued for the outgoing document would otherwise resubmit the
        // new one and never reach the ledger. Flush it explicitly.
        if model.controllerAttached { Task { await flushToHelper(outgoing) } }
        let text = model.activeText
        let restored = Self.clamp(carets[path] ?? NSRange(location: 0, length: 0), to: text)
        model.caretUTF16 = restored.location
        model.caretLengthUTF16 = restored.length
        model.selection = .init(path: path, nsRange: restored, token: (model.selection?.token ?? 0) + 1)
        // A document whose buffer equals its durable text is current at this
        // editor revision: previews compiled from that durable revision must
        // not read as stale merely because the mapping was recorded earlier.
        if let durable = model.controllerState.durable[path],
           model.controllerState.textByDurable[path]?[durable.revision]?.sameBytes(as: text) == true {
            model.controllerState.editorRevisionByDurable[path, default: [:]][durable.revision] = model.editorRevision
        }
        status = "editing \(path)"
        return .switched(to: path, restoredCaret: restored)
    }

    /// Records the caret/selection currently shown for `path`.
    func rememberCaret(for path: String) {
        guard isOpen(path) else { return }
        carets[path] = NSRange(location: model.caretUTF16, length: model.caretLengthUTF16)
    }

    /// `activePath` is also switched directly by navigation (a span in another
    /// document, ⌘⇧D, diagnostics stepping). Observation fires before the
    /// change, while the caret still belongs to the outgoing document: record
    /// it so a later user switch back restores it.
    private func armActivePathTracking() {
        guard !armed else { return }
        armed = true
        withObservationTracking { [weak self] in
            guard let self else { return }
            _ = self.model.activePath
        } onChange: { [weak self] in
            MainActor.assumeIsolated {
                guard let self else { return }
                self.armed = false
                self.rememberCaret(for: self.model.activePath)
                self.armActivePathTracking()
            }
        }
    }

    /// Clamps a remembered UTF-16 range to `text` and moves its ends off a
    /// surrogate split; the caret alone when the range no longer fits.
    static func clamp(_ range: NSRange, to text: String) -> NSRange {
        let ns = text as NSString
        func boundary(_ i: Int) -> Int {
            var i = min(max(0, i), ns.length)
            if i > 0, i < ns.length, UTF16.isTrailSurrogate(ns.character(at: i)), UTF16.isLeadSurrogate(ns.character(at: i - 1)) { i -= 1 }
            return i
        }
        let start = boundary(range.location)
        let end = boundary(range.location + range.length)
        return end > start ? NSRange(location: start, length: end - start) : NSRange(location: start, length: 0)
    }

    // MARK: save (non-entry documents)

    /// Saves a non-entry member to its rooted file: through the helper's
    /// `export` (durable source, mandatory disk expectation, refused on a
    /// changed/appeared file) when attached, else through the file layer's
    /// compare-and-replace. The entry document keeps `ShellModel.saveTex`.
    /// Nothing is written on a conflict; the buffer and baseline are kept.
    func saveDocument(_ path: String, timeout: TimeInterval = 10) async -> SaveOutcome {
        prune()
        guard path != entryPath else { return .failed("\(path) is the entry document; use Save (ShellModel.saveTex)") }
        guard let doc = model.documents.first(where: { $0.path == path }) else { return .failed("\(path) is not open") }
        guard let root = projectRoot else { return .failed("no project root") }
        let url = root.appendingPathComponent(path)
        let text = doc.text
        let expectedDisk = diskBaselines[path] ?? nil
        if model.controllerAttached {
            // A reply resuming after File > Open replaced the project belongs to
            // the old project: never record it on the new one's same-named member.
            let generation = model.projectGeneration
            func replaced() -> Bool { model.projectGeneration != generation || projectRoot != root }
            let replacedFailure = SaveOutcome.failed("\(path) was closed while saving; nothing recorded for the open project")
            let flushed = await flushToHelper(path, timeout: timeout)
            if replaced() { return noteSave(replacedFailure) }
            guard flushed, let durable = model.controllerState.durable[path],
                  model.controllerState.textByDurable[path]?[durable.revision]?.sameBytes(as: text) == true else {
                return noteSave(.failed("\(path) did not become durable within \(Int(timeout)) s"))
            }
            let reply = await helperRequest("export", ["path": path, "expected_revision": durable.revision,
                                                       "expected_sha256": durable.sha256, "expected_disk_sha256": expectedDisk ?? NSNull()])
            if replaced() { return noteSave(replacedFailure) }
            let verdict = await Self.exportVerdict(reply, path: path, text: text) { [weak self] in await self?.diskSHA256(of: path) }
            if replaced() { return noteSave(replacedFailure) }
            switch verdict {
            case .saved(let sha, let afterError):
                baselines[path] = text
                diskBaselines[path] = sha
                saveConflicts[path] = nil
                model.snapshotSaved(url: url, text: text)
                let how = afterError.map { " (the helper reported \"\($0)\" after the rename; the file holds exactly the exported text)" } ?? ""
                return noteSave(.saved(path: path, sha256: sha), extra: " through the preview controller (durable r\(durable.revision))" + how)
            case .conflict(let kind, let theirs):
                let conflict = DocumentConflict(url: url, kind: kind, ours: expectedDisk, theirs: theirs, size: nil, mtimeUnixMs: nil, viaHelper: true)
                saveConflicts[path] = conflict
                model.preserveDirtyText(text, at: url, reason: "save refused: file changed on disk")
                return noteSave(.conflict(conflict))
            case .failed(let why):
                model.preserveDirtyText(text, at: url, reason: "save failed: \(why)")
                return noteSave(.failed(why))
            }
        }
        return saveDocumentNow(path)
    }

    /// What an `export` reply means for the member's baseline (STDIO.md).
    enum ExportVerdict: Equatable {
        /// The file holds exactly `text` (receipt hash verified). `afterError`
        /// is the helper's error when the write landed but the reply was an
        /// error ("an error can follow rename"): disk was inspected, not assumed.
        case saved(sha256: String, afterError: String?)
        case conflict(ProjectFilesV1.ConflictKind, theirs: String?)
        case failed(String)
    }

    /// The exact guard on the export route, on top of the helper's own
    /// (`expected_revision`/`expected_sha256`/`expected_disk_sha256`, rooted
    /// lock, rename): a receipt is a save only when it names this path and
    /// hashes to exactly the text sent — a different hash means the helper
    /// exported some other durable text and the buffer stays dirty. A refusal
    /// naming a project-files conflict is a conflict with the disk hash read
    /// through `file_status`. Any other error is checked against disk first:
    /// the helper documents errors after a successful rename (directory
    /// durability), so a file that now hashes to the text sent *is* saved and
    /// the baseline follows it; otherwise nothing changes and the error stands.
    static func exportVerdict(_ reply: Result<[String: Any], ControllerError>, path: String, text: String,
                              diskSHA256: () async -> String?) async -> ExportVerdict {
        let sent = SourceDigest.sha256Hex(text)
        switch reply {
        case .success(let payload):
            if let p = payload["path"] as? String, p != path {
                return .failed("export receipt names \(p), not \(path); buffer kept unsaved")
            }
            guard let sha = payload["sha256"] as? String else {
                return .failed("export receipt for \(path) carries no sha256; buffer kept unsaved")
            }
            guard sha == sent else {
                return .failed("export receipt hash for \(path) (\(sha.prefix(12))) is not the text sent (\(sent.prefix(12))); buffer kept unsaved")
            }
            if let bytes = payload["bytes"] as? Int, bytes != text.utf8.count {
                return .failed("export receipt for \(path) reports \(bytes) bytes, sent \(text.utf8.count); buffer kept unsaved")
            }
            return .saved(sha256: sha, afterError: nil)
        case .failure(let e):
            if let kind = ShellModel.conflictKind(inExportRefusal: e.message) {
                return .conflict(kind, theirs: await diskSHA256())
            }
            if await diskSHA256() == sent { return .saved(sha256: sent, afterError: e.message) }
            return .failed(e.message)
        }
    }

    /// Synchronous save of a non-entry member through the file layer's
    /// rooted compare-and-replace (bounded helper wait inside), for flows
    /// that cannot suspend (quit). Bypasses the preview controller's ledger
    /// export exactly as `ShellModel.saveTex` does for the entry document.
    @discardableResult
    func saveDocumentNow(_ path: String) -> SaveOutcome {
        prune()
        guard path != entryPath else { return .failed("\(path) is the entry document; use Save (ShellModel.saveTex)") }
        guard let doc = model.documents.first(where: { $0.path == path }) else { return .failed("\(path) is not open") }
        guard let root = projectRoot else { return .failed("no project root") }
        let url = root.appendingPathComponent(path)
        let text = doc.text
        let expectedDisk = diskBaselines[path] ?? nil
        let expected: ProjectFilesV1.Expected = expectedDisk.map { .hash($0) } ?? .newFile
        switch model.files.save(url, text: text, expected: expected, force: false, recordsState: false) {
        case .saved(let sha):
            baselines[path] = text
            diskBaselines[path] = sha
            saveConflicts[path] = nil
            model.snapshotSaved(url: url, text: text)
            return noteSave(.saved(path: path, sha256: sha))
        case .conflict(let c):
            saveConflicts[path] = c
            model.preserveDirtyText(text, at: url, reason: "save refused: file changed on disk")
            return noteSave(.conflict(c))
        case .failed(let why):
            return noteSave(.failed(why))
        }
    }

    private func noteSave(_ outcome: SaveOutcome, extra: String = "") -> SaveOutcome {
        status = switch outcome {
        case .saved(let p, let sha): "saved \(p)\(extra) (sha256 \(sha.prefix(12)))"
        case .conflict(let c): c.summary
        case .failed(let why): "save failed: \(why)"
        }
        FlashTeXLog.write("project: " + status)
        return outcome
    }

    /// Makes `path`'s buffer durable on the helper: waits (bounded) for any
    /// in-flight edit of that path, then submits one `edit` if the buffer
    /// still differs from the durable text. True when durable.
    @discardableResult
    func flushToHelper(_ path: String, timeout: TimeInterval = 10) async -> Bool {
        // The model owns this object (unowned back-reference); a strong local
        // keeps it alive across the waits below, like `DocumentKinds.refresh`:
        // a `switchDocument` task polling here must never touch a freed model.
        let model = self.model
        guard model.controllerAttached, model.controllerState.ready else { return false }
        let deadline = Date().addingTimeInterval(timeout)
        guard await awaitInFlight(of: path, deadline: deadline) else { return false }
        guard let durable = model.controllerState.durable[path], let text = model.documents.first(where: { $0.path == path })?.text else { return false }
        if model.controllerState.textByDurable[path]?[durable.revision]?.sameBytes(as: text) == true { return true }
        if model.activePath == path, model.controllerState.inFlight == nil {
            // The controller's own queue handles the active document.
            model.controllerSubmitEdit()
            guard await awaitInFlight(of: path, deadline: deadline) else { return false }
            return model.controllerState.textByDurable[path]?[model.controllerState.durable[path]?.revision ?? -1]?.sameBytes(as: text) == true
        }
        let reply = await helperRequest("edit", ["path": path, "expected_revision": durable.revision, "expected_sha256": durable.sha256, "text": text])
        switch reply {
        case .success(let receipt):
            guard let d = receipt["document"] as? [String: Any], let r = d["revision"] as? Int, let h = d["source_sha256"] as? String, let t = d["text"] as? String else { return false }
            recordDurable(path: path, revision: r, sha256: h, text: t)
            if let e = receipt["preview_error"] as? String { model.log("controller preview_error after flushing \(path): \(e)") }
            model.log("project: flushed \(path) to the helper as durable r\(r)")
            return t.sameBytes(as: text)
        case .failure(let e):
            status = "helper refused the \(path) buffer: \(e.message)"
            model.log("project: " + status)
            if e.message.hasPrefix("document_conflict") { _ = try? model.controller?.document(path: path) }
            return false
        }
    }

    /// Waits (until `deadline`) for the controller's in-flight edit of `path`
    /// to be released. The controller releases it when the preview for its
    /// durable revision arrives, judged by the *active* document's version;
    /// after a switch away from `path` that check can never pass although the
    /// preview did arrive (its compiled text is the durable text). Release it
    /// here in that case so the queue moves on (parent diff: judge the
    /// release by `inFlight.path`, which makes this branch unreachable).
    private func awaitInFlight(of path: String, deadline: Date) async -> Bool {
        let model = self.model // strong across the polling waits (see flushToHelper)
        while let inFlight = model.controllerState.inFlight, inFlight.path == path {
            if let want = inFlight.durableRevision, model.activePath != path,
               let durableText = model.controllerState.textByDurable[path]?[want],
               model.compiledDocuments[path]?.sameBytes(as: durableText) == true {
                model.log("project: releasing the in-flight edit of \(path) (durable r\(want) previewed while \(model.activePath) is active)")
                model.controllerState.inFlight = nil
                model.inFlightRevision = nil
                if model.controllerState.queued { model.controllerState.queued = false; model.controllerSubmitEdit() }
                break
            }
            if Date() > deadline { return false }
            try? await Task.sleep(nanoseconds: 10_000_000)
        }
        return true
    }

    /// Disk hash of `path` as the helper sees it (`file_status`), nil when
    /// the file is missing or the helper cannot answer.
    private func diskSHA256(of path: String) async -> String? {
        guard case .success(let payload) = await helperRequest("file_status", ["path": path]),
              let disk = payload["disk"] as? [String: Any] else { return nil }
        return (disk["sha256"] as? String) ?? (disk["disk_sha256"] as? String)
    }

    // MARK: helper attach / sync

    /// `controllerStatus` changes when the helper becomes ready (and on every
    /// durable receipt); each change re-checks that every open member has a
    /// durable document on the helper, cheaply when nothing is missing.
    private func armControllerTracking() {
        guard !controllerArmed else { return }
        controllerArmed = true
        withObservationTracking { [weak self] in
            guard let self else { return }
            _ = self.model.controllerStatus
        } onChange: { [weak self] in
            // Fires before the new value is stored: read it on the next turn.
            DispatchQueue.main.async { [weak self] in
                MainActor.assumeIsolated {
                    guard let self else { return }
                    self.controllerArmed = false
                    self.armControllerTracking()
                    guard self.model.controllerAttached, self.model.controllerState.ready, self.needsHelperSync else { return }
                    Task { await self.syncWithHelper() }
                }
            }
        }
    }

    /// Open members (beyond the entry, which the controller reads itself)
    /// the helper has no durable document for yet.
    private var needsHelperSync: Bool {
        model.documents.dropFirst().contains { model.controllerState.durable[$0.path] == nil }
    }

    /// Gives every open member a durable document on the helper: documents
    /// opened directly before the controller was attached (or across a helper
    /// restart) are read from the helper when it already lists them, else
    /// imported with `open_document` under the exact snapshot; a buffer that
    /// differs from the durable text is then submitted as one edit so the
    /// preview compiles what the editor shows. Never touches the entry
    /// document (ShellModel+Controller owns it).
    func syncWithHelper() async {
        let model = self.model // strong across the helper round trips (see flushToHelper)
        guard !syncing, model.controllerAttached, model.controllerState.ready, needsHelperSync else { return }
        syncing = true
        defer { syncing = false }
        helperSyncs += 1
        for doc in model.documents.dropFirst() where model.controllerState.durable[doc.path] == nil {
            guard let snapshot = await refreshSnapshot() else { return }
            let reply: Result<[String: Any], ControllerError>
            if snapshot.versions[doc.path] != nil {
                reply = await helperRequest("document", ["path": doc.path])
            } else {
                reply = await helperRequest("open_document", ["path": doc.path, "source_versions": snapshot.versions,
                                                              "membership_generation": snapshot.generation])
            }
            guard case .success(let payload) = reply else {
                if case .failure(let e) = reply { status = "helper could not attach \(doc.path): \(e.message)"; model.log("project: " + status) }
                continue
            }
            adoptMembership(payload)
            guard let durable = payload["document"] as? [String: Any], let text = durable["text"] as? String,
                  let revision = durable["revision"] as? Int, let sha = durable["source_sha256"] as? String else {
                status = "helper document for \(doc.path) is missing fields"
                continue
            }
            recordDurable(path: doc.path, revision: revision, sha256: sha, text: text)
            origins[doc.path] = .helper
            guard let current = model.documents.first(where: { $0.path == doc.path })?.text, !current.sameBytes(as: text) else {
                status = "\(doc.path) durable r\(revision) on the helper"
                continue
            }
            // The editor buffer is authoritative for what the user sees.
            let edited = await helperRequest("edit", ["path": doc.path, "expected_revision": revision, "expected_sha256": sha, "text": current])
            switch edited {
            case .success(let receipt):
                if let d = receipt["document"] as? [String: Any], let r = d["revision"] as? Int, let h = d["source_sha256"] as? String, let t = d["text"] as? String {
                    recordDurable(path: doc.path, revision: r, sha256: h, text: t)
                    status = "\(doc.path) buffer submitted as durable r\(r)"
                }
                if let e = receipt["preview_error"] as? String { model.log("controller preview_error after syncing \(doc.path): \(e)") }
            case .failure(let e):
                status = "helper refused the \(doc.path) buffer: \(e.message)"
                model.log("project: " + status)
            }
        }
    }

    // MARK: helper transport

    struct Snapshot: Equatable { var versions: [String: Int]; var generation: Int }

    /// `snapshot` from the helper: the exact source versions and membership
    /// generation `open_document`/`detach_document` must echo.
    func refreshSnapshot() async -> Snapshot? {
        switch await helperRequest("snapshot", [:]) {
        case .success(let payload):
            return adoptSnapshot(payload)
        case .failure(let e):
            status = "helper snapshot failed: \(e.message)"
            return nil
        }
    }

    /// Adopts a `snapshot` reply as the current versions/generation (also the
    /// ready-time learn in ShellModel+DisplayCandidates.swift, which sends the
    /// request itself so it precedes the first compile); nil, with `status`
    /// set, when the reply lacks the fields.
    @discardableResult
    func adoptSnapshot(_ payload: [String: Any]) -> Snapshot? {
        guard let versions = payload["source_versions"] as? [String: Int], let generation = payload["membership_generation"] as? Int else {
            status = "helper snapshot is missing source_versions/membership_generation"
            return nil
        }
        sourceVersions = versions
        membershipGeneration = generation
        return Snapshot(versions: versions, generation: generation)
    }

    /// Current active-source metadata from the helper (`project_status`), or nil.
    func projectStatus() async -> [String: Any]? {
        guard case .success(let payload) = await helperRequest("project_status", [:]) else { return nil }
        adoptMembership(payload)
        return payload
    }

    /// The helper is gone (detach, exit, reattach): its generation and
    /// versions mean nothing to the next session, which learns them afresh
    /// (`snapshot` on `ready`; ShellModel+DisplayCandidates.swift). Until then
    /// the display-candidate gate refuses every candidate as `membership_unknown`.
    func forgetMembership() {
        membershipGeneration = nil
        sourceVersions = [:]
    }

    private func adoptMembership(_ payload: [String: Any]) {
        if let versions = payload["source_versions"] as? [String: Int] { sourceVersions = versions }
        if let generation = payload["membership_generation"] as? Int { membershipGeneration = generation }
    }

    private func recordDurable(path: String, revision: Int, sha256: String, text: String) {
        model.controllerState.durable[path] = (revision, sha256)
        model.controllerState.textByDurable[path, default: [:]][revision] = text
        model.controllerState.editorRevisionByDurable[path, default: [:]][revision] = model.editorRevision
    }

    /// One helper round trip through the controller's reply routing
    /// (`controllerState.awaiting`), bounded by `helperTimeout`; a late reply
    /// is dropped by the routing table once the waiter is gone.
    func helperRequest(_ type: String, _ payload: [String: Any]) async -> Result<[String: Any], ControllerError> {
        let model = self.model // strong until the reply or the timeout (see flushToHelper)
        guard let controller = model.controller, controller.isRunning, model.controllerState.ready else {
            return .failure(.init(message: "preview controller not ready"))
        }
        let id: String
        do { id = try controller.send(type, payload) } catch { return .failure(.init(message: "\(type) failed to send: \(error.localizedDescription)")) }
        let box = ReplyBox()
        return await withCheckedContinuation { cont in
            model.controllerState.awaiting[id] = { result in
                guard box.settle() else { return }
                cont.resume(returning: result)
            }
            let timeout = helperTimeout
            DispatchQueue.main.asyncAfter(deadline: .now() + timeout) { [weak model] in
                MainActor.assumeIsolated {
                    guard box.settle() else { return }
                    model?.controllerState.awaiting.removeValue(forKey: id)
                    cont.resume(returning: .failure(.init(message: "no \(type) reply from the preview controller within \(Int(timeout)) s")))
                }
            }
        }
    }

    private final class ReplyBox {
        private var settled = false
        /// True for the first caller only.
        func settle() -> Bool { if settled { return false }; settled = true; return true }
    }

    // MARK: rooted files (direct mode)

    enum Rooted: Equatable { case file(URL), refused(String) }

    /// `<root>/<path>` when every component stays under `root` with no
    /// symlink on the way (the same refusal the rooted helper applies).
    nonisolated static func rootedFile(_ path: String, under root: URL) -> Rooted {
        let rootResolved = root.resolvingSymlinksInPath().standardizedFileURL
        var url = rootResolved
        let fm = FileManager.default
        for segment in path.split(separator: "/") {
            url.appendPathComponent(String(segment))
            if let type = (try? fm.attributesOfItem(atPath: url.path))?[.type] as? FileAttributeType, type == .typeSymbolicLink {
                return .refused("\(url.lastPathComponent) is a symbolic link")
            }
        }
        let resolved = url.resolvingSymlinksInPath().standardizedFileURL
        guard resolved.path == rootResolved.path || resolved.path.hasPrefix(rootResolved.path + "/") else {
            return .refused("\(path) resolves outside the project root")
        }
        return .file(url)
    }

    // MARK: status

    @discardableResult
    private func note<T>(_ outcome: T, extra: String = "") -> T {
        let line: String = switch outcome {
        case let o as OpenOutcome:
            switch o {
            case .opened(let p): "opened \(p)\(extra)"
            case .alreadyOpen(let p): "\(p) is already open"
            case .refused(let why): why
            }
        case let o as DetachOutcome:
            switch o {
            case .detached(let p): "detached \(p)\(extra)"
            case .refused(let why): why
            }
        default: "\(outcome)"
        }
        status = line
        FlashTeXLog.write("project: " + line)
        return outcome
    }
}

// MARK: - model attachment

extension ShellModel {
    private static var projectKey = 0
    /// Multi-file project state (see `ProjectDocuments`).
    var project: ProjectDocuments {
        if let existing = objc_getAssociatedObject(self, &Self.projectKey) as? ProjectDocuments { return existing }
        let state = ProjectDocuments(model: self)
        objc_setAssociatedObject(self, &Self.projectKey, state, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return state
    }
}
