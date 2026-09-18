import Foundation

/// The project's `.bib` files read directly — no helper needed.
///
/// One BibTeX parser serves two readers: the citation hover
/// (`EditorIntelligence.bibEntry`, EditorHoverResolution.swift) stops at the
/// key it is asked about, and `\cite{` completion (Completion.swift) keeps
/// every key of every bibliography the project declares. Before this the
/// completion saw `.bib` keys only through the helper's project index, so a
/// project without the helper attached offered nothing but `\bibitem`s.
///
/// Which files: `\bibliography{refs,more}` and `\addbibresource{…}` in the
/// open documents, the paths the helper reports as declared bibliographies
/// (`DocumentKinds`), and any `.bib` that is itself open (parsed from the
/// buffer, which is ahead of the disk). Files are read off-main by the
/// completion job and cached per path by modification date and size.
enum BibScanner {
    // MARK: records

    /// One `@type{key, field = value, …}` entry. Field names are lowercased;
    /// values are unbraced/unquoted with whitespace collapsed; of a repeated
    /// field the first wins (exactly what the hover has always shown).
    struct Record: Equatable, Sendable {
        var key: String
        /// `article`, `book`, … lowercased.
        var type: String
        var fields: [String: String]
    }

    /// Walks every entry of `text` in order; `body` returns false to stop.
    /// `@comment`, `@preamble` and `@string` are not entries. Values may be
    /// braced, quoted or a bare word; nested braces are honoured.
    static func forEachRecord(in text: String, _ body: (Record) -> Bool) {
        let ns = text as NSString
        var i = 0
        while i < ns.length {
            guard ns.character(at: i) == 0x40 else { i += 1; continue } // `@`
            var j = i + 1
            while j < ns.length, isLetter(ns.character(at: j)) { j += 1 }
            let type = ns.substring(with: NSRange(location: i + 1, length: j - i - 1)).lowercased()
            while j < ns.length, isSpace(ns.character(at: j)) { j += 1 }
            guard j < ns.length, ns.character(at: j) == 0x7B, !["comment", "preamble", "string"].contains(type),
                  let entryEnd = EditorIntelligence.scanBalanced(in: ns, from: j, open: 0x7B, close: 0x7D) else { i += 1; continue }
            var k = j + 1
            while k < ns.length, isSpace(ns.character(at: k)) { k += 1 }
            let keyStart = k
            while k < entryEnd, ns.character(at: k) != 0x2C { k += 1 } // `,`
            let key = ns.substring(with: NSRange(location: keyStart, length: k - keyStart)).trimmingCharacters(in: .whitespacesAndNewlines)
            let fields = bibFields(in: ns, from: min(k + 1, entryEnd), to: entryEnd)
            if !key.isEmpty, !body(Record(key: key, type: type, fields: fields)) { return }
            i = entryEnd + 1
        }
    }

    /// The entry for `key`, stopping at the first match.
    static func record(forKey key: String, in text: String) -> Record? {
        var found: Record?
        forEachRecord(in: text) { r in
            if r.key == key { found = r; return false }
            return true
        }
        return found
    }

    static func records(in text: String) -> [Record] {
        var out: [Record] = []
        forEachRecord(in: text) { out.append($0); return true }
        return out
    }

    /// `name = value` pairs of one entry body.
    private static func bibFields(in ns: NSString, from start: Int, to end: Int) -> [String: String] {
        var out: [String: String] = [:]
        var i = start
        while i < end {
            while i < end, !isLetter(ns.character(at: i)) { i += 1 }
            let nameStart = i
            while i < end, isLetter(ns.character(at: i)) || ns.character(at: i) == 0x5F { i += 1 }
            guard i > nameStart else { break }
            let name = ns.substring(with: NSRange(location: nameStart, length: i - nameStart)).lowercased()
            while i < end, isSpace(ns.character(at: i)) { i += 1 }
            guard i < end, ns.character(at: i) == 0x3D else { continue } // `=`
            i += 1
            while i < end, isSpace(ns.character(at: i)) { i += 1 }
            guard i < end else { break }
            let c = ns.character(at: i)
            var value = ""
            if c == 0x7B, let close = EditorIntelligence.scanBalanced(in: ns, from: i, open: 0x7B, close: 0x7D), close <= end {
                value = ns.substring(with: NSRange(location: i + 1, length: close - i - 1))
                i = close + 1
            } else if c == 0x22 { // `"`
                var j = i + 1
                while j < end, ns.character(at: j) != 0x22 { j += 1 }
                value = ns.substring(with: NSRange(location: i + 1, length: max(0, j - i - 1)))
                i = min(j + 1, end)
            } else {
                let s = i
                while i < end, ns.character(at: i) != 0x2C, !isSpace(ns.character(at: i)) { i += 1 }
                value = ns.substring(with: NSRange(location: s, length: i - s))
            }
            let collapsed = value.split(whereSeparator: { $0 == "\n" || $0 == "\r" || $0 == "\t" || $0 == " " }).joined(separator: " ")
            if out[name] == nil, !collapsed.isEmpty { out[name] = collapsed }
            while i < end, ns.character(at: i) != 0x2C { i += 1 }
            i += 1
        }
        return out
    }

    private static func isLetter(_ c: unichar) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
    private static func isSpace(_ c: unichar) -> Bool { c == 0x20 || c == 0x09 || c == 0x0A || c == 0x0D }

    // MARK: completion entries

    /// A record as the completion list shows it.
    struct Entry: Equatable, Sendable {
        var key: String
        var type: String
        /// Project-relative path of the file (`refs.bib`, `bib/more.bib`).
        var path: String
        var title: String?
        var author: String?
        var year: String?

        init(key: String, type: String, path: String, title: String? = nil, author: String? = nil, year: String? = nil) {
            self.key = key; self.type = type; self.path = path; self.title = title; self.author = author; self.year = year
        }

        init(record: Record, path: String) {
            self.init(key: record.key, type: record.type, path: path,
                      title: record.fields["title"].map(EditorIntelligence.plainText),
                      author: record.fields["author"].map(EditorIntelligence.plainText).map(EditorIntelligence.shortenAuthors),
                      year: record.fields["year"] ?? record.fields["date"].map { String($0.prefix(4)) })
        }

        /// The row's origin column: `@article · refs.bib`.
        var detail: String { "@\(type) · \(path)" }

        /// The documentation line: the title, else author and year, else nothing.
        var documentation: String? {
            if let title, !title.isEmpty { return title }
            var parts: [String] = []
            if let author, !author.isEmpty { parts.append(author) }
            if let year, !year.isEmpty { parts.append(year) }
            return parts.isEmpty ? nil : parts.joined(separator: " ")
        }
    }

    /// Where the `.bib` files come from, captured on the main thread when the
    /// list is requested (`ShellModel.bibliographySources`) and resolved by
    /// the completion job off-main.
    struct Sources: Equatable, Sendable {
        /// The saved project's directory, which `\bibliography{refs}` names
        /// are rooted under; nil (an unsaved buffer) reads nothing from disk.
        var projectRoot: URL? = nil
        /// Project-relative paths the helper reports as declared
        /// bibliographies (`DocumentKinds.bibliographyPaths`).
        var declaredPaths: [String] = []
        /// The open documents, the active one first. A `.bib` among them is
        /// parsed from its buffer; a LaTeX one is scanned for declarations.
        var documents: [Document] = []

        struct Document: Equatable, Sendable {
            var path: String
            var text: String
            init(path: String, text: String) { self.path = path; self.text = text }
        }

        init(projectRoot: URL? = nil, declaredPaths: [String] = [], documents: [Document] = []) {
            self.projectRoot = projectRoot; self.declaredPaths = declaredPaths; self.documents = documents
        }
    }

    /// Whether a project path names a BibTeX file by extension. Used only to
    /// decide what to parse (and how to colour a buffer no kind was reported
    /// for), never to claim a key's provenance to the reader.
    static func isBibliographyPath(_ path: String) -> Bool {
        path.lowercased().hasSuffix(".bib")
    }

    /// The `.bib` paths `text` declares, in order: `\bibliography{refs,more}`
    /// (names get `.bib` when they lack it) and `\addbibresource[…]{x.bib}`.
    /// `\bibliographystyle{…}` is not a declaration.
    static func declaredBibliographies(in text: String) -> [String] {
        let ns = text as NSString
        var out: [String] = []
        for command in ["\\bibliography", "\\addbibresource"] {
            var search = NSRange(location: 0, length: ns.length)
            while true {
                let r = ns.range(of: command, options: .literal, range: search)
                guard r.location != NSNotFound else { break }
                search = NSRange(location: NSMaxRange(r), length: ns.length - NSMaxRange(r))
                var i = NSMaxRange(r)
                if i < ns.length, isLetter(ns.character(at: i)) { continue } // `\bibliographystyle`
                while i < ns.length, ns.character(at: i) == 0x20 || ns.character(at: i) == 0x09 { i += 1 }
                if i < ns.length, ns.character(at: i) == 0x5B { // `[options]`
                    guard let close = EditorIntelligence.scanBalanced(in: ns, from: i, open: 0x5B, close: 0x5D) else { continue }
                    i = close + 1
                    while i < ns.length, ns.character(at: i) == 0x20 || ns.character(at: i) == 0x09 { i += 1 }
                }
                guard i < ns.length, ns.character(at: i) == 0x7B,
                      let close = EditorIntelligence.scanBalanced(in: ns, from: i, open: 0x7B, close: 0x7D) else { continue }
                let arg = ns.substring(with: NSRange(location: i + 1, length: close - i - 1))
                for raw in arg.split(separator: ",") {
                    let name = raw.trimmingCharacters(in: .whitespacesAndNewlines)
                    guard !name.isEmpty else { continue }
                    out.append(isBibliographyPath(name) ? name : name + ".bib")
                }
            }
        }
        return out
    }

    // MARK: cache

    /// Entries per file, keyed by project path and validated by the file's
    /// modification date and size; entries per open buffer, validated by the
    /// text itself. Safe to use from the completion queue.
    final class Cache: @unchecked Sendable {
        private let lock = NSLock()
        private var files: [String: (modified: Date, size: Int, entries: [Entry])] = [:]
        private var buffers: [String: (text: String, entries: [Entry])] = [:]
        /// Evidence for tests: how many times a file or buffer was parsed.
        private(set) var parses = 0

        init() {}

        /// The entries of the file at `url` (shown as `path`); nil when the
        /// file cannot be read. A hit costs one `attributesOfItem`.
        func entries(at url: URL, path: String) -> [Entry]? {
            guard let attributes = try? FileManager.default.attributesOfItem(atPath: url.path),
                  let modified = attributes[.modificationDate] as? Date else { return nil }
            let size = (attributes[.size] as? NSNumber)?.intValue ?? -1
            lock.lock()
            if let hit = files[path], hit.modified == modified, hit.size == size { lock.unlock(); return hit.entries }
            lock.unlock()
            guard let data = try? Data(contentsOf: url) else { return nil }
            let text = String(decoding: data, as: UTF8.self)
            let entries = Self.parse(text, path: path)
            lock.lock()
            files[path] = (modified, size, entries)
            parses += 1
            lock.unlock()
            return entries
        }

        /// The entries of an open buffer.
        func entries(inBuffer text: String, path: String) -> [Entry] {
            lock.lock()
            if let hit = buffers[path], hit.text == text { lock.unlock(); return hit.entries }
            lock.unlock()
            let entries = Self.parse(text, path: path)
            lock.lock()
            buffers[path] = (text, entries)
            parses += 1
            lock.unlock()
            return entries
        }

        private static func parse(_ text: String, path: String) -> [Entry] {
            records(in: text).map { Entry(record: $0, path: path) }
        }
    }

    /// The process-wide cache the editor's completion uses.
    static let cache = Cache()

    /// Every entry of every bibliography `sources` names, first declaration
    /// first, one row per key (the first file that defines it wins).
    /// `cancelled` is polled between files.
    static func entries(for sources: Sources, cache: Cache = cache, cancelled: () -> Bool = { false }) -> [Entry] {
        var paths: [String] = []
        var seenPaths = Set<String>()
        var fromBuffers: [String: [Entry]] = [:]
        for doc in sources.documents {
            if isBibliographyPath(doc.path) {
                if seenPaths.insert(doc.path).inserted { paths.append(doc.path) }
                fromBuffers[doc.path] = cache.entries(inBuffer: doc.text, path: doc.path)
            } else {
                for path in declaredBibliographies(in: doc.text) where seenPaths.insert(path).inserted { paths.append(path) }
            }
            if cancelled() { return [] }
        }
        for path in sources.declaredPaths where seenPaths.insert(path).inserted { paths.append(path) }
        var out: [Entry] = []
        var seenKeys = Set<String>()
        for path in paths {
            if cancelled() { return [] }
            let entries: [Entry]
            if let buffered = fromBuffers[path] {
                entries = buffered
            } else if let root = sources.projectRoot, case .file(let url) = ProjectDocuments.rootedFile(path, under: root),
                      let read = cache.entries(at: url, path: path) {
                entries = read
            } else {
                continue
            }
            for entry in entries where seenKeys.insert(entry.key).inserted { out.append(entry) }
        }
        return out
    }
}
