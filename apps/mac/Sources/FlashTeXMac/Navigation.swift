import AppKit
import SwiftUI
import FlashTeXProtocol
import FlashTeXAccessibility

/// Source-aware navigation. The text functions are pure and return UTF-8 byte
/// ranges into the current buffers (no compile result involved, so they are
/// never stale). Result-backed navigation (preview click, diagnostics, caret
/// reveal) goes through `ShellModel.navigateExactly`, which maps runtime-v1
/// UTF-8 byte spans onto the editor's UTF-16 selection exactly: a span whose
/// bytes were edited since the compile is refused with an explanation, a span
/// is never split inside a scalar or a composed character sequence, and a span
/// in another open document switches the active document.
enum Navigation {
    struct ByteRange: Equatable {
        let start: Int
        let end: Int
    }

    /// One `\name{arg}` occurrence: `range` spans the backslash through `}`,
    /// `argRange` the argument text.
    struct CommandUse: Equatable {
        let name: String
        let arg: String
        let range: ByteRange
        let argRange: ByteRange
    }

    static let referenceCommands: Set<String> = ["ref", "eqref", "pageref", "autoref"]
    private static let interestingCommands: Set<String> = referenceCommands.union(["begin", "end", "label"])

    /// Every `\begin{…}`, `\end{…}`, `\label{…}`, and reference command with a
    /// complete braced argument, in document order. Commands inside a `%`
    /// comment (to the end of the line) are skipped; `\%` and `\\` are escapes.
    static func commandUses(in text: String) -> [CommandUse] {
        var out: [CommandUse] = []
        Completion.withBytes(text) { b in
            guard let p = b.baseAddress else { return }
            let n = b.count
            let table = Completion.wordByteClass
            let backslash = Completion.backslash, percent = UInt8(ascii: "%"), newline = UInt8(ascii: "\n")
            let open = UInt8(ascii: "{"), close = UInt8(ascii: "}")
            var i = 0
            while i < n {
                let c = p[i]
                if c == percent {
                    while i < n, p[i] != newline { i += 1 }
                    continue
                }
                guard c == backslash else { i += 1; continue }
                var j = i + 1
                while j < n, table[Int(p[j])] == 1 { j += 1 }
                guard j > i + 1 else { i = min(n, i + 2); continue } // `\%`, `\\`, `\{`, trailing `\`
                let nameBytes = UnsafeBufferPointer(start: p + i + 1, count: j - i - 1)
                let name = String(decoding: nameBytes, as: UTF8.self)
                guard interestingCommands.contains(name), j < n, p[j] == open else { i = j; continue }
                var k = j + 1
                while k < n, p[k] != close, p[k] != open, p[k] != backslash, p[k] != newline, p[k] != percent { k += 1 }
                guard k < n, p[k] == close else { i = j; continue }
                out.append(CommandUse(name: name,
                                      arg: String(decoding: UnsafeBufferPointer(start: p + j + 1, count: k - j - 1), as: UTF8.self),
                                      range: ByteRange(start: i, end: k + 1), argRange: ByteRange(start: j + 1, end: k)))
                i = k + 1
            }
        }
        return out
    }

    enum Target: Equatable {
        case found(ByteRange, note: String)
        case notFound(String)
    }

    /// Counterpart of the command under the caret (`caretByte`, UTF-8) within
    /// one document: `\ref{X}` → its `\label{X}`; `\label{X}` → the next
    /// reference to it (wrapping); `\begin{X}` ↔ `\end{X}` honouring nesting
    /// of the same name. See `matchingRange(in:activePath:caretByte:)` for
    /// labels and references that live in other open documents.
    static func matchingRange(in text: String, caretByte: Int) -> Target {
        switch matchingRange(in: [.init(path: "", text: text)], activePath: "", caretByte: caretByte) {
        case .found(_, let range, let note): return .found(range, note: note)
        case .notFound(let why): return .notFound(why)
        }
    }

    enum DocumentTarget: Equatable {
        case found(path: String, ByteRange, note: String)
        case notFound(String)
    }

    /// Multi-document counterpart lookup. `\begin`/`\end` match within the
    /// active document. `\ref{X}` finds `\label{X}` in the active document
    /// first, then the other open documents in project order. `\label{X}`
    /// cycles through every reference to it: those after the caret in the
    /// active document, then the following documents, wrapping around.
    static func matchingRange(in documents: [RuntimeV1.Document], activePath: String, caretByte: Int) -> DocumentTarget {
        guard let active = documents.firstIndex(where: { $0.path == activePath }) else {
            return .notFound("No open document named \(activePath).")
        }
        let uses = commandUses(in: documents[active].text)
        // A caret at the byte where one command ends and the next begins
        // (`\end{a}\begin{b}`) belongs to the one that starts there.
        let here = uses.firstIndex(where: { $0.range.start == caretByte })
            ?? uses.firstIndex(where: { $0.range.start <= caretByte && caretByte <= $0.range.end })
        guard let here else {
            return .notFound("Caret is not inside \\begin, \\end, \\label, or a \\ref-style command.")
        }
        let use = uses[here]
        let elsewhere = documents.count > 1 ? " or any open document" : ""
        switch use.name {
        case "begin":
            var depth = 0
            for other in uses[(here + 1)...] where other.arg == use.arg {
                if other.name == "begin" { depth += 1 }
                else if other.name == "end" {
                    if depth == 0 { return .found(path: activePath, other.range, note: "Matched \\begin{\(use.arg)} → \\end{\(use.arg)} at byte \(other.range.start).") }
                    depth -= 1
                }
            }
            return .notFound("\\begin{\(use.arg)} at byte \(use.range.start) has no matching \\end{\(use.arg)}.")
        case "end":
            var depth = 0
            for other in uses[..<here].reversed() where other.arg == use.arg {
                if other.name == "end" { depth += 1 }
                else if other.name == "begin" {
                    if depth == 0 { return .found(path: activePath, other.range, note: "Matched \\end{\(use.arg)} → \\begin{\(use.arg)} at byte \(other.range.start).") }
                    depth -= 1
                }
            }
            return .notFound("\\end{\(use.arg)} at byte \(use.range.start) has no matching \\begin{\(use.arg)}.")
        case "label":
            // All references in project order, starting with the active document.
            var refs: [(path: String, use: CommandUse)] = []
            for offset in 0..<documents.count {
                let doc = documents[(active + offset) % documents.count]
                let docUses = offset == 0 ? uses : commandUses(in: doc.text)
                for r in docUses where referenceCommands.contains(r.name) && r.arg == use.arg {
                    refs.append((doc.path, r))
                }
            }
            guard !refs.isEmpty else { return .notFound("No reference to label \(use.arg) in this document\(elsewhere).") }
            let after = refs.firstIndex { $0.path != activePath || $0.use.range.start > use.range.start }
            let next = refs[after ?? 0]
            let index = (after ?? 0) + 1
            let place = next.path == activePath ? "" : " in \(next.path)"
            return .found(path: next.path, next.use.range,
                          note: "Reference \(index) of \(refs.count) to label \(use.arg) at byte \(next.use.range.start)\(place).")
        default:
            for offset in 0..<documents.count {
                let doc = documents[(active + offset) % documents.count]
                let docUses = offset == 0 ? uses : commandUses(in: doc.text)
                if let label = docUses.first(where: { $0.name == "label" && $0.arg == use.arg }) {
                    let place = doc.path == activePath ? "" : " in \(doc.path)"
                    return .found(path: doc.path, label.range, note: "Definition of \(use.arg): \\label at byte \(label.range.start)\(place).")
                }
            }
            return .notFound("No \\label{\(use.arg)} in this document\(elsewhere).")
        }
    }

    // MARK: - byte span → editor selection

    enum RangeMapping: Equatable {
        /// `widenedFrom` is set when the span started or ended inside a
        /// composed character sequence (`e` + U+0301, a ZWJ emoji, a ligature
        /// glyph is one scalar and never splits) and was extended to cover it.
        case selected(NSRange, widenedFrom: NSRange?)
        case refused(String)
    }

    /// Exact UTF-16 selection for UTF-8 bytes `start..<end` of `text`.
    /// Refused when out of range, reversed, or inside a multi-byte scalar;
    /// widened outward to whole composed character sequences (the units the
    /// text view selects and moves the caret by), so the selection is never a
    /// split cluster. An empty span at a cluster-interior position is moved
    /// to the start of that cluster and stays empty.
    static func editorRange(start: Int, end: Int, in text: String, path: String = "") -> RangeMapping {
        let byteCount = text.utf8.count
        let label = path.isEmpty ? "" : " in \(path)"
        guard start >= 0, end >= start, end <= byteCount else {
            return .refused("Bytes \(start)..<\(end) are not a valid range\(label) (buffer is \(byteCount) bytes).")
        }
        guard let r = text.rangeOfUTF8(start: start, end: end) else {
            return .refused("Bytes \(start)..<\(end)\(label) start or end inside a multi-byte character; refusing to split it.")
        }
        let ns = NSRange(r, in: text)
        let nsText = text as NSString
        let whole: NSRange
        if ns.length == 0 {
            whole = ns.location < nsText.length
                ? NSRange(location: nsText.rangeOfComposedCharacterSequence(at: ns.location).location, length: 0)
                : ns
        } else {
            whole = nsText.rangeOfComposedCharacterSequences(for: ns)
        }
        return .selected(whole, widenedFrom: whole == ns ? nil : ns)
    }

    // MARK: - byte span across an edit

    enum Rebased: Equatable {
        /// The span in `current`; `note` names the shift when there was one.
        case mapped(start: Int, end: Int, note: String?)
        /// Why the span cannot be mapped (which bytes overlap the edit).
        case refused(String)
    }

    /// Maps bytes `start..<end` of `baseline` onto `current` across the single
    /// changed region between them (byte-for-byte comparison), refusing a span
    /// that overlaps the edit and verifying that the mapped bytes still spell
    /// the baseline bytes so a wrong span can never be selected silently.
    static func rebaseExactly(start: Int, end: Int, from baseline: String, to current: String, path: String) -> Rebased {
        if baseline.sameBytes(as: current) { return .mapped(start: start, end: end, note: nil) }
        let region = SourceMapping.changedRegion(from: baseline, to: current)
        switch SourceMapping.rebase(start: start, end: end, across: region) {
        case .overlapsEdit:
            return .refused("bytes \(start)..<\(end) of \(path) overlap the edit at \(region.startByte)..<\(region.oldEndByte), now \(region.startByte)..<\(region.newEndByte)")
        case .rebased(let s, let e):
            guard let was = baseline.rangeOfUTF8(start: start, end: end),
                  let now = current.rangeOfUTF8(start: s, end: e),
                  String(baseline[was]).sameBytes(as: String(current[now])) else {
                return .refused("bytes \(start)..<\(end) of \(path) no longer spell the same text after rebasing to \(s)..<\(e)")
            }
            return .mapped(start: s, end: e, note: s != start ? "rebased from \(start)..<\(end) across edits" : nil)
        case .unchanged:
            return .mapped(start: start, end: end, note: nil)
        }
    }

    // MARK: - helper (project index) probe

    /// Commands whose braced argument names a label or citation the project
    /// index resolves; a caret on the command name probes the argument.
    static let indexedArgumentCommands: Set<String> = [
        "ref", "eqref", "pageref", "autoref", "cref", "Cref", "nameref",
        "cite", "citep", "citet", "citeauthor", "citeyear", "parencite", "textcite", "autocite", "nocite",
    ]

    /// Byte offset to send to the helper's lexical `navigate` for the caret,
    /// or nil when the caret is on `\begin`/`\end` (environments are matched
    /// in the buffer). The index keys symbols by their *name* bytes (the
    /// argument of `\ref{…}`, the letters of `\mycmd`), so a caret on the
    /// backslash or on `ref` is moved onto the name it refers to; anywhere
    /// else the caret byte itself is sent and the helper decides.
    static func helperProbeOffset(in text: String, caretByte: Int) -> Int? {
        Completion.withBytes(text) { b -> Int? in
            guard let p = b.baseAddress, caretByte >= 0, caretByte <= b.count else { return caretByte }
            let n = b.count
            let table = Completion.wordByteClass
            func isLetter(_ i: Int) -> Bool { i >= 0 && i < n && table[Int(p[i])] == 1 }
            // A command token `\name` containing the caret (caret on `\`, a letter, or just after the name).
            var s = caretByte
            if s < n, p[s] == Completion.backslash { s += 1 } else { while isLetter(s - 1) { s -= 1 } }
            if s >= 1, p[s - 1] == Completion.backslash, isLetter(s) {
                var e = s
                while isLetter(e) { e += 1 }
                if caretByte >= s - 1, caretByte <= e, e > s {
                    let name = String(decoding: UnsafeBufferPointer(start: p + s, count: e - s), as: UTF8.self)
                    if name == "begin" || name == "end" { return nil }
                    if indexedArgumentCommands.contains(name) {
                        var k = e
                        if k < n, p[k] == UInt8(ascii: "*") { k += 1 }
                        while k < n, p[k] == UInt8(ascii: "[") {
                            while k < n, p[k] != UInt8(ascii: "]"), p[k] != UInt8(ascii: "\n") { k += 1 }
                            if k < n, p[k] == UInt8(ascii: "]") { k += 1 }
                        }
                        if k < n, p[k] == UInt8(ascii: "{") {
                            k += 1
                            while k < n, p[k] == UInt8(ascii: " ") { k += 1 }
                            return k
                        }
                    }
                    return s
                }
            }
            // Inside a brace group right after `\name`: the helper resolves the
            // item under the caret; a caret on the closing brace moves to the
            // argument start so `\ref{x|}` still resolves.
            var open = caretByte - 1
            while open >= 0, p[open] != UInt8(ascii: "{"), p[open] != UInt8(ascii: "}"), p[open] != UInt8(ascii: "\n"), p[open] != Completion.backslash { open -= 1 }
            if open >= 0, p[open] == UInt8(ascii: "{") {
                var t = open - 1
                if t >= 0, p[t] == UInt8(ascii: "]") { while t >= 0, p[t] != UInt8(ascii: "[") { t -= 1 }; t -= 1 }
                if t >= 0, p[t] == UInt8(ascii: "*") { t -= 1 }
                var ns = t
                while isLetter(ns) { ns -= 1 }
                if ns >= 0, p[ns] == Completion.backslash, t > ns {
                    let name = String(decoding: UnsafeBufferPointer(start: p + ns + 1, count: t - ns), as: UTF8.self)
                    if name == "begin" || name == "end" { return nil }
                    if caretByte >= n || p[caretByte] == UInt8(ascii: "}") || p[caretByte] == UInt8(ascii: " ") {
                        var k = open + 1
                        while k < n, p[k] == UInt8(ascii: " ") { k += 1 }
                        return k
                    }
                }
            }
            return caretByte
        }
    }
}

// MARK: - model actions

extension ShellModel {
    /// Exact preview → source navigation. Same contract as `navigate(to:expectedText:)`
    /// with these guarantees:
    /// - staleness is decided byte-for-byte (`sameBytes`), so a normalization-only
    ///   edit (é → e + U+0301) is an edit, not a match;
    /// - a span overlapping the edited region is refused, and the note says
    ///   which bytes were edited;
    /// - a rebased span is verified to spell the same bytes it did at compile
    ///   time (item text is informational: generated text such as a section
    ///   number legitimately differs from its source);
    /// - the selection covers whole composed character sequences;
    /// - the active document switches when the span lives in another open one.
    ///
    /// `compiledText` overrides the recorded baseline for `source.path` (a v2
    /// display list that attests the buffer's exact digest passes the buffer).
    func navigateExactly(to source: RuntimeV1.SourceRange?, expectedText: String? = nil, compiledText: String? = nil) {
        if let why = historicalRefusal(of: "navigation") { navigationNote = why; return }
        guard let source else {
            navigationNote = "This item has no source mapping."
            return
        }
        guard let doc = documents.first(where: { $0.path == source.path }) else {
            let open = documents.map(\.path).joined(separator: ", ")
            navigationNote = "No open document named \(source.path) (open: \(open))."
            return
        }
        var start = source.startByte, end = source.endByte
        var notes: [String] = []
        let revision = result?.revision ?? 0
        if let compiled = compiledText ?? self.compiledText(for: source.path) {
            switch Navigation.rebaseExactly(start: start, end: end, from: compiled, to: doc.text, path: source.path) {
            case .refused(let why):
                navigationNote = "Source for this item was edited since revision \(revision) (\(why)); recompile to navigate."
                return
            case .mapped(let s, let e, let note):
                if let note { notes.append(note) }
                start = s; end = e
            }
        } else if previewIsStale {
            navigationNote = "Buffer edited since revision \(revision) and no compiled text is recorded; recompile to navigate."
            return
        }
        let ns: NSRange
        switch Navigation.editorRange(start: start, end: end, in: doc.text, path: source.path) {
        case .refused(let why):
            navigationNote = why
            return
        case .selected(let range, let widenedFrom):
            ns = range
            if let widenedFrom {
                notes.append("widened from UTF-16 \(widenedFrom.location)..<\(NSMaxRange(widenedFrom)) to whole characters")
            }
        }
        if let expectedText, !(doc.text as NSString).substring(with: ns).sameBytes(as: expectedText) {
            notes.append("“\(expectedText)” is generated from this source")
        }
        if activePath != source.path {
            activePath = source.path
            notes.append("switched to \(source.path)")
        }
        selection = .init(path: source.path, nsRange: ns, token: (selection?.token ?? 0) + 1)
        caretUTF16 = ns.location
        caretLengthUTF16 = ns.length
        navigationNote = "Selected \(source.path) bytes \(start)..<\(end) → UTF-16 \(ns.location)..<\(NSMaxRange(ns))"
            + (notes.isEmpty ? "" : " (" + notes.joined(separator: "; ") + ")")
    }

    /// The exact text `path` was compiled from for the applied result: the
    /// recorded request text or, on the helper route, the durable text at the
    /// version the applied preview names — an include the helper compiled
    /// before this window read it (Open All Includes / FLASHTEX_OPEN_INCLUDES=1
    /// after the first preview) has no request text recorded, but its durable
    /// text at that version is the same bytes once read.
    func compiledText(for path: String) -> String? {
        if let text = compiledDocuments[path] { return text }
        guard controllerAttached, let revision = displayCandidates.applied?.sourceVersions[path] else { return nil }
        return controllerState.textByDurable[path]?[revision]
    }

    /// ⌘⇧D: select the counterpart of the `\begin`/`\end`/`\label`/`\ref`
    /// under the caret; a label or reference in another open document
    /// switches to it.
    func goToMatching() {
        guard let caretByte = CaretSync.byteOffset(ofCaretUTF16: caretUTF16, in: activeText) else {
            navigationNote = "Caret position \(caretUTF16) is not valid in \(activePath)."
            return
        }
        // With the durable helper attached, labels, citations and user commands
        // resolve project-wide through its lexical index (STDIO.md `navigate`);
        // environments stay in the buffer. Without a helper: in-buffer matching.
        if controllerAttached, controllerState.ready, let probe = Navigation.helperProbeOffset(in: activeText, caretByte: caretByte) {
            navigationNote = "Looking up the project index…"
            Task { await goToMatchingViaHelper(path: activePath, byteOffset: probe, text: activeText) }
            return
        }
        goToMatchingInBuffer(caretByte: caretByte)
    }

    /// In-buffer matching across the open documents (the no-helper route).
    func goToMatchingInBuffer(caretByte: Int) {
        switch Navigation.matchingRange(in: documents, activePath: activePath, caretByte: caretByte) {
        case .notFound(let why):
            navigationNote = why
        case .found(let path, let range, let note):
            guard let text = documents.first(where: { $0.path == path })?.text,
                  case .selected(let ns, _) = Navigation.editorRange(start: range.start, end: range.end, in: text, path: path) else {
                navigationNote = "Bytes \(range.start)..<\(range.end) are not a valid range in \(path)."
                return
            }
            activePath = path
            selection = .init(path: path, nsRange: ns, token: (selection?.token ?? 0) + 1)
            caretUTF16 = ns.location
            caretLengthUTF16 = ns.length
            navigationNote = note
        }
    }

    // MARK: helper (project index) navigation

    /// One source location the helper reported: `{path, revision, start_byte, end_byte}`.
    struct IndexLocation: Equatable {
        var path: String
        var revision: Int
        var start: Int
        var end: Int

        init?(_ json: Any?) {
            guard let o = json as? [String: Any], let path = o["path"] as? String, let revision = o["revision"] as? Int,
                  let start = o["start_byte"] as? Int, let end = o["end_byte"] as? Int else { return nil }
            self.path = path; self.revision = revision; self.start = start; self.end = end
        }
    }

    enum HelperReply: Equatable {
        /// `navigate` found a symbol: its name, origin span and lexical definitions.
        case found(name: String, origin: IndexLocation, definitions: [IndexLocation], truncated: Bool)
        /// The caret is on no indexed symbol.
        case nothing
        /// The version map no longer matches the helper's snapshot.
        case staleVersions(String)
        case refused(String)
    }

    /// One request → one reply through `controllerState.awaiting` (the pattern
    /// `controllerSave`/`controllerFileStatus` use). Nil when nothing is attached.
    func controllerRequest(_ type: String, _ payload: PreviewControllerClient.JSONObject) async -> Result<[String: Any], ControllerError>? {
        guard let controller, controller.isRunning, controllerState.ready else { return nil }
        let id: String
        do { id = try controller.send(type, payload) } catch { return .failure(.init(message: "\(type) failed to send: \(error.localizedDescription)")) }
        return await withCheckedContinuation { cont in
            controllerState.awaiting[id] = { cont.resume(returning: $0) }
        }
    }

    /// The helper's complete current version map (`snapshot`).
    func controllerSourceVersions() async -> Result<[String: Int], ControllerError>? {
        guard let reply = await controllerRequest("snapshot", [:]) else { return nil }
        switch reply {
        case .failure(let e): return .failure(e)
        case .success(let payload):
            guard let versions = payload["source_versions"] as? [String: Int] else {
                return .failure(.init(message: "snapshot reply has no source_versions"))
            }
            return .success(versions)
        }
    }

    /// Lexical `navigate {source_versions, path, byte_offset}`.
    func controllerNavigate(sourceVersions: [String: Int], path: String, byteOffset: Int) async -> HelperReply? {
        guard let reply = await controllerRequest("navigate", ["source_versions": sourceVersions, "path": path, "byte_offset": byteOffset]) else { return nil }
        switch reply {
        case .failure(let e):
            return e.message.contains("source versions changed") ? .staleVersions(e.message) : .refused(e.message)
        case .success(let payload):
            guard let nav = payload["navigation"] as? [String: Any] else { return .nothing }
            guard let name = nav["name"] as? String, let origin = IndexLocation(nav["origin"]) else {
                return .refused("navigate reply is missing name or origin")
            }
            let definitions = (nav["definitions"] as? [Any] ?? []).compactMap(IndexLocation.init)
            return .found(name: name, origin: origin, definitions: definitions, truncated: nav["definitions_truncated"] as? Bool ?? false)
        }
    }

    /// Occurrences of one exact name (`complete` with the name as prefix), so
    /// a caret on a definition can cycle through its references project-wide.
    func controllerOccurrences(sourceVersions: [String: Int], category: String, name: String) async -> Result<[IndexLocation], ControllerError>? {
        guard let reply = await controllerRequest("complete", ["source_versions": sourceVersions, "category": category, "prefix": name, "limit": 100]) else { return nil }
        switch reply {
        case .failure(let e): return .failure(e)
        case .success(let payload):
            let items = payload["completions"] as? [[String: Any]] ?? []
            guard let item = items.first(where: { $0["name"] as? String == name }) else { return .success([]) }
            return .success((item["occurrences"] as? [Any] ?? []).compactMap(IndexLocation.init))
        }
    }

    /// ⌘⇧D through the helper: `snapshot` → `navigate` at `byteOffset` of the
    /// durable `path`; a definition (or, from a definition, the next reference
    /// in project order) is selected exactly in its document, opening that
    /// document in this window when it is not open yet. Every step re-checks
    /// versions: the buffer must equal the durable text the helper indexed,
    /// and a stale-version error is refused, never retried against a guess.
    func goToMatchingViaHelper(path: String, byteOffset: Int, text: String) async {
        guard let versionsReply = await controllerSourceVersions() else { navigationNote = "Preview controller detached; nothing looked up."; return }
        let versions: [String: Int]
        switch versionsReply {
        case .failure(let e): navigationNote = "Project index unavailable: \(e.message)"; return
        case .success(let v): versions = v
        }
        guard let revision = versions[path] else {
            navigationNote = "\(path) is not part of the helper's project (indexed: \(versions.keys.sorted().joined(separator: ", ")))."
            return
        }
        guard let durable = controllerState.textByDurable[path]?[revision] else {
            navigationNote = "Durable revision \(revision) of \(path) is not known to this session yet; try again."
            return
        }
        guard durable.sameBytes(as: text) else {
            navigationNote = "\(path) has edits the helper has not indexed yet (durable r\(revision)); try again in a moment."
            return
        }
        guard let reply = await controllerNavigate(sourceVersions: versions, path: path, byteOffset: byteOffset) else {
            navigationNote = "Preview controller detached; nothing looked up."; return
        }
        switch reply {
        case .staleVersions(let why):
            navigationNote = "Project changed while looking up the index (\(why)); try again."
        case .refused(let why):
            navigationNote = "Project index refused the lookup: \(why)"
        case .nothing:
            navigationNote = "Caret byte \(byteOffset) of \(path) is on no label, citation or command the project index knows (\\begin/\\end are matched in the buffer)."
        case .found(let name, let origin, let definitions, let truncated):
            let isDefinition = definitions.contains { $0.path == origin.path && $0.start == origin.start && $0.end == origin.end }
            if isDefinition {
                // From a definition: cycle its references, project order (helper path order), wrapping.
                let category = Self.categoryGuess(originText: durable, origin: origin)
                guard let occ = await controllerOccurrences(sourceVersions: versions, category: category, name: name) else {
                    navigationNote = "Preview controller detached; nothing looked up."; return
                }
                switch occ {
                case .failure(let e): navigationNote = "Project index refused the reference lookup: \(e.message)"
                case .success(let all):
                    let refs = all.filter { !definitions.contains($0) }
                        .sorted { a, b in a.path != b.path ? a.path < b.path : a.start < b.start }
                    guard !refs.isEmpty else { navigationNote = "No reference to \(name) in the project (lexical index)."; return }
                    let after = refs.firstIndex { $0.path > origin.path || ($0.path == origin.path && $0.start > origin.start) } ?? 0
                    let target = refs[after]
                    await selectIndexLocation(target, versions: versions, label: "Reference \(after + 1) of \(refs.count) to \(name)")
                }
            } else {
                guard let target = definitions.first else {
                    navigationNote = "No definition of \(name) in the project (lexical index)."; return
                }
                let count = definitions.count > 1 ? " (definition 1 of \(definitions.count)\(truncated ? "+" : ""))" : ""
                await selectIndexLocation(target, versions: versions, label: "Definition of \(name)\(count)")
            }
        }
    }

    /// Whether `origin` (a definition) is a label, citation or command, from
    /// the command that introduces it in the durable text.
    static func categoryGuess(originText: String, origin: IndexLocation) -> String {
        let bytes = Array(originText.utf8)
        var i = origin.start - 1
        while i >= 0, bytes[i] != UInt8(ascii: "\\") { i -= 1 }
        guard i >= 0 else { return "command" }
        var e = i + 1
        while e < bytes.count, Completion.wordByteClass[Int(bytes[e])] == 1 { e += 1 }
        let name = String(decoding: bytes[(i + 1)..<e], as: UTF8.self)
        if name == "label" { return "label" }
        if name == "bibitem" { return "citation" }
        return "command"
    }

    /// Selects a helper-reported location exactly: the document's durable text
    /// at the reported revision is the baseline; an open buffer that moved on
    /// is rebased across the edit (refused on overlap); a document not open in
    /// this window is fetched (`document`) and opened.
    func selectIndexLocation(_ loc: IndexLocation, versions: [String: Int], label: String) async {
        guard versions[loc.path] == loc.revision else {
            navigationNote = "\(label): \(loc.path) r\(loc.revision) is not the current durable revision (r\(versions[loc.path].map(String.init) ?? "?")); try again."
            return
        }
        if controllerState.textByDurable[loc.path]?[loc.revision] == nil {
            // Not read yet: ask for it; the parent's handler records the durable text.
            _ = try? controller?.document(path: loc.path)
            let deadline = Date().addingTimeInterval(2)
            while controllerState.textByDurable[loc.path]?[loc.revision] == nil, Date() < deadline, controllerAttached {
                try? await Task.sleep(nanoseconds: 5_000_000)
            }
        }
        guard let baseline = controllerState.textByDurable[loc.path]?[loc.revision] else {
            navigationNote = "\(label): durable revision \(loc.revision) of \(loc.path) could not be read from the helper."
            return
        }
        var notes: [String] = []
        if documents.firstIndex(where: { $0.path == loc.path }) == nil {
            documents.append(.init(path: loc.path, text: baseline))
            notes.append("opened \(loc.path) at durable r\(loc.revision)")
        }
        guard let doc = documents.first(where: { $0.path == loc.path }) else { return }
        let start: Int, end: Int
        switch Navigation.rebaseExactly(start: loc.start, end: loc.end, from: baseline, to: doc.text, path: loc.path) {
        case .refused(let why):
            navigationNote = "\(label): edited since the index was built (\(why)); try again when the edit is durable."
            return
        case .mapped(let s, let e, let note):
            start = s; end = e
            if let note { notes.append(note) }
        }
        let ns: NSRange
        switch Navigation.editorRange(start: start, end: end, in: doc.text, path: loc.path) {
        case .refused(let why): navigationNote = "\(label): \(why)"; return
        case .selected(let range, let widenedFrom):
            ns = range
            if let widenedFrom { notes.append("widened from UTF-16 \(widenedFrom.location)..<\(NSMaxRange(widenedFrom)) to whole characters") }
        }
        if activePath != loc.path { activePath = loc.path; notes.append("switched to \(loc.path)") }
        selection = .init(path: loc.path, nsRange: ns, token: (selection?.token ?? 0) + 1)
        caretUTF16 = ns.location
        caretLengthUTF16 = ns.length
        navigationNote = "\(label): \(loc.path) bytes \(start)..<\(end) (project index, durable r\(loc.revision))"
            + (notes.isEmpty ? "" : " (" + notes.joined(separator: "; ") + ")")
    }

    /// ⌘⇧] / ⌘⇧[: step through the underlined diagnostics (`editorMarkReport`,
    /// exact mark identities, stale spans withheld) of the active document in
    /// document order; past its last mark the step continues into the next
    /// open document with marks (project order, wrapping), switching to it.
    /// Diagnostics under edited text are skipped and counted in the note.
    func goToDiagnostic(forward: Bool) {
        if let why = historicalRefusal(of: "diagnostic navigation") { navigationNote = why; return }
        guard let result else {
            navigationNote = "No compile result loaded; nothing to navigate to."
            return
        }
        let report = editorMarkReport
        let here = EditorDiagnostics.step(report.marks, fromUTF16: caretUTF16, forward: forward,
                                          currentID: currentDiagnosticID, in: activeText)
        var chosen: (path: String, step: EditorDiagnosticNavigation.Step, report: EditorDiagnostics.Report)?
        if let here, !here.wrapped || documents.count == 1 {
            chosen = (activePath, here, report)
        } else if let active = documents.firstIndex(where: { $0.path == activePath }), documents.count > 1 {
            // Leaving the active document at either end: the first (or last)
            // mark of the next open document that has one.
            for offset in 1..<documents.count {
                let doc = documents[(active + offset) % documents.count]
                let other = diagnosticReport(for: doc.path, currentText: doc.text) // retention rule applied (ShellModel+DiagnosticRetention.swift)
                if let step = EditorDiagnostics.step(other.marks, fromUTF16: forward ? -1 : Int.max, forward: forward, in: doc.text) {
                    chosen = (doc.path, step, other)
                    break
                }
            }
            if chosen == nil, let here { chosen = (activePath, here, report) }
        }
        guard let chosen else {
            let total = result.diagnostics.count
            let open = documents.count > 1 ? "an open document (\(documents.map(\.path).joined(separator: ", ")))" : activePath
            navigationNote = total == 0 ? "Revision \(result.revision) has no diagnostics."
                : report.staleNote.map { "No diagnostic can be selected: " + $0 + "." }
                ?? "None of the \(total) diagnostic\(total == 1 ? "" : "s") has a source in \(open)."
            return
        }
        let switched = chosen.path != activePath
        activePath = chosen.path
        currentDiagnosticID = chosen.step.item.id
        selection = .init(path: chosen.path, nsRange: chosen.step.item.nsRange, token: (selection?.token ?? 0) + 1)
        caretUTF16 = chosen.step.item.nsRange.location
        caretLengthUTF16 = chosen.step.item.nsRange.length
        navigationNote = chosen.step.announcement
            + (switched ? " (in \(chosen.path))" : "")
            + (chosen.report.staleNote.map { "; " + $0 } ?? "")
    }

    /// ⌘⇧J: select the full source span of the preview item under the caret
    /// (so the preview's caret highlight and page scroll follow) and say where
    /// it landed. It is also the manual override for automatic following
    /// (`CaretFollow.swift`): asking for the caret scrolls to it at once —
    /// without waiting for the debounce, and without regard to the "Preview
    /// follows the caret" preference, on or off — and resumes following (once
    /// the preference is on) if a manual preview scroll had stopped it.
    func revealCaretInPreview() {
        caretFollow.note(.explicit)
        guard let result else {
            navigationNote = "No compile result loaded; the caret maps to no preview item."
            return
        }
        guard let byte = CaretSync.byteOffset(ofCaretUTF16: caretUTF16, in: activeText) else {
            navigationNote = "Caret position \(caretUTF16) is not valid in \(activePath)."
            return
        }
        let hits = caretIndex()?.itemsContaining(byte: byte) ?? []
        guard let hit = hits.first,
              let page = result.pages.first(where: { $0.number == hit.page }),
              hit.index < page.items.count,
              case .text(let item) = page.items[hit.index], let source = item.source else {
            // display-list-v2-window §4.1: with pages elided, "no item" is not
            // the same claim as "nothing here" — the item may be on a page the
            // producer was never asked to build. Say which it is.
            if let why = v2WindowSourceActionRefusal {
                navigationNote = why
                return
            }
            navigationNote = "Caret byte \(byte) is inside no preview item" + (previewIsStale ? " (preview is from an older revision)." : ".")
            return
        }
        let before = selection
        navigateExactly(to: source, expectedText: item.text)
        if let sel = selection, sel != before {
            let pages = Set(hits.map(\.page)).sorted()
            let more = hits.count > 1
                ? " (+\(hits.count - 1) more" + (pages.count > 1 ? ", pages \(pages.map(String.init).joined(separator: ", "))" : "") + ")."
                : "."
            navigationNote = "Caret is in page \(hit.page) item \(hit.index) “\(item.text)”" + more
        }
    }
}

// MARK: - menu

/// `Navigate` menu, added from `FlashTeXMacApp` with one line.
struct NavigationCommands: Commands {
    var model: ShellModel
    /// The diagnostics panel's selection/occurrence cursor (DiagnosticsPanel.swift), while one is in the scene.
    @FocusedValue(\.diagnosticsPanel) private var diagnosticsPanel

    var body: some Commands {
        CommandMenu("Navigate") {
            Button("Go to Matching \\begin/\\end or \\label/\\ref") { model.goToMatching() }
                .keyboardShortcut("d", modifiers: [.command, .shift])
            // Editor navigation lane (ShellModel+EditorNavigation.swift / EditorNavigation.swift).
            Button("Go to Definition") { model.goToDefinition() }
                .keyboardShortcut("j", modifiers: [.command, .control])
            Button("Go to Symbol…") { model.editorNavigation.symbolPickerShown = true }
                .keyboardShortcut("t", modifiers: [.command, .shift])
            Divider()
            Button("Select Environment") { model.selectEnvironment() }
                .keyboardShortcut("a", modifiers: [.command, .shift])
            Button("Wrap Selection in Environment…") { model.editorNavigation.wrapShown = true }
                .keyboardShortcut("w", modifiers: [.command, .shift])
            Button("Rename Symbol…") { model.presentRenameSymbol() }
                .keyboardShortcut("r", modifiers: [.option, .shift])
            Divider()
            Button("Next Diagnostic") { model.goToDiagnostic(forward: true) }
                .keyboardShortcut("]", modifiers: [.command, .shift])
                .disabled(!model.toolbarHasResult) // change-only mirror: a per-reply `result` read here re-evaluates the App scene (FlashTeXMacApp.commands)
            Button("Previous Diagnostic") { model.goToDiagnostic(forward: false) }
                .keyboardShortcut("[", modifiers: [.command, .shift])
                .disabled(!model.toolbarHasResult) // change-only mirror: a per-reply `result` read here re-evaluates the App scene (FlashTeXMacApp.commands)
            Button("Next Occurrence") { if let p = diagnosticsPanel { model.stepOccurrence(forward: true, panel: p) } }
                .keyboardShortcut("]", modifiers: [.command, .option])
                .disabled(diagnosticsPanel == nil)
            Button("Previous Occurrence") { if let p = diagnosticsPanel { model.stepOccurrence(forward: false, panel: p) } }
                .keyboardShortcut("[", modifiers: [.command, .option])
                .disabled(diagnosticsPanel == nil)
            Divider()
            Button("Reveal Caret in Preview") { model.revealCaretInPreview() }
                .keyboardShortcut("j", modifiers: [.command, .shift])
                .disabled(!model.toolbarHasResult) // change-only mirror: a per-reply `result` read here re-evaluates the App scene (FlashTeXMacApp.commands)
        }
    }
}
