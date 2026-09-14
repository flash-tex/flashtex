import Foundation
import Observation
import FlashTeXProtocol

/// A texcount-style word/statistics scan of one buffer: words in body text,
/// in section/chapter headers, and in `\caption{…}` arguments, plus the
/// number of inline and display math blocks. Pure and synchronous — callers
/// that scan on every keystroke (`WordCountModel` below) do so off the main
/// thread and debounced; this type itself has no threading opinion.
///
/// Scope, matched to GH68: comments, verbatim-like environments (`verbatim`,
/// `lstlisting`, `minted`, `alltt`, `comment`, `\verb`), math (`$…$`, `$$…$$`,
/// `\(…\)`, `\[…\]`, and the standard math environments), the preamble
/// (everything before `\begin{document}`), and the arguments of reference/
/// citation/include-style commands (`\label`, `\ref`, `\cite`, `\input`,
/// `\includegraphics`, …) are all excluded from every word count. A command
/// not on that skip list is treated as typographic (`\textbf{…}`, `\emph{…}`,
/// list/float environments, …): its name contributes no words, but text in
/// its argument counts normally in whatever bucket was already active. This
/// mirrors texcount's own default for unlisted commands; it does not expand
/// user macros, so a custom macro's literal argument (e.g. a homework
/// template's `\problem{1}{4}`) is counted as if it were prose, same as
/// texcount without a supplied command list.
enum DocumentStatistics {
    /// Word/block totals. `totalWords` is what the status bar shows.
    struct Counts: Equatable, Sendable {
        var bodyWords = 0
        var headerWords = 0
        var captionWords = 0
        var inlineMath = 0
        var displayMath = 0
        var totalWords: Int { bodyWords + headerWords + captionWords }

        static func + (a: Counts, b: Counts) -> Counts {
            Counts(bodyWords: a.bodyWords + b.bodyWords, headerWords: a.headerWords + b.headerWords,
                   captionWords: a.captionWords + b.captionWords, inlineMath: a.inlineMath + b.inlineMath,
                   displayMath: a.displayMath + b.displayMath)
        }
        static func += (a: inout Counts, b: Counts) { a = a + b }
    }

    /// One heading's span: everything from its title to the next heading (of
    /// any level) in the same document. `documentPath` is set only when the
    /// aggregate covers more than one open document (see `analyze(documents:)`).
    struct SectionBreakdown: Equatable, Sendable {
        var documentPath: String?
        /// `part`/`chapter` 0 … `paragraph`/`subparagraph` 4/5 (`DocumentOutline.sectionLevels`).
        var level: Int
        var title: String
        var counts: Counts
    }

    /// Result of scanning one document.
    struct Result: Equatable, Sendable {
        var counts: Counts
        var sections: [SectionBreakdown]
        /// True when the document exceeded `maxScannedBytes` and was not scanned (counts are all zero).
        var truncated: Bool = false
    }

    /// Result of scanning the open document set (`analyze(documents:)`).
    struct AggregateResult: Equatable, Sendable {
        var total: Counts
        var sections: [SectionBreakdown]
        var documentCount: Int
        var truncated: Bool = false
    }

    /// Documents longer than this are not scanned (mirrors `DocumentOutline.maxScannedUTF16`'s
    /// guard against a pathological rescan); the bench in docs/evidence covers up to 500 KB.
    static let maxScannedBytes = 4 * 1024 * 1024

    /// Scans one document's full text.
    static func analyze(_ text: String) -> Result {
        guard text.utf8.count <= maxScannedBytes else { return Result(counts: Counts(), sections: [], truncated: true) }
        let scanner = Scanner(text: text)
        scanner.run()
        return Result(counts: scanner.total, sections: scanner.sections)
    }

    /// Scans every document in the open set and sums the totals. Section
    /// titles are tagged with their owning path only when there is more than
    /// one document, so a single-file project's popover stays unlabeled.
    static func analyze(documents: [(path: String, text: String)]) -> AggregateResult {
        var total = Counts()
        var sections: [SectionBreakdown] = []
        var truncated = false
        let multi = documents.count > 1
        for doc in documents {
            let r = analyze(doc.text)
            truncated = truncated || r.truncated
            total += r.counts
            if multi {
                sections.append(contentsOf: r.sections.map { s in
                    var s = s; s.documentPath = doc.path; return s
                })
            } else {
                sections.append(contentsOf: r.sections)
            }
        }
        return AggregateResult(total: total, sections: sections, documentCount: documents.count, truncated: truncated)
    }

    /// Word count of one UTF-16 range of `text` (the editor selection), using
    /// the same scan — so a selection that happens to contain, say, a whole
    /// math block is excluded the same way the full-document count excludes it.
    static func wordCount(inSelection text: String, utf16Range: NSRange) -> Int {
        guard utf16Range.length > 0 else { return 0 }
        let ns = text as NSString
        guard utf16Range.location >= 0, utf16Range.location + utf16Range.length <= ns.length else { return 0 }
        return analyze(ns.substring(with: utf16Range)).counts.totalWords
    }

    // MARK: - Scanner

    private enum Bucket { case body, header, caption }

    /// Commands whose argument(s) are never prose: skip the command name and
    /// every immediately-following `[…]`/`{…}` group (in the order LaTeX
    /// reads them), greedily, so a two-argument `\newcommand{\foo}[1]{…}`
    /// macro *definition* is skipped in full rather than counted as text.
    private static let skipCommands: Set<String> = [
        "label", "ref", "eqref", "pageref", "cref", "Cref", "vref", "autoref", "nameref",
        "cite", "citep", "citet", "citeauthor", "citeyear", "citealt", "citealp",
        "footcite", "textcite", "parencite", "Citep", "Citet",
        "url", "path", "includegraphics", "input", "include", "subfile", "subimport", "import",
        "bibliography", "addbibresource", "bibliographystyle", "printbibliography", "bibitem",
        "usepackage", "RequirePackage", "documentclass",
        "newcommand", "renewcommand", "providecommand", "DeclareRobustCommand",
        "newenvironment", "renewenvironment", "DeclareMathOperator", "newcolumntype", "newtheorem",
        "setlength", "addtolength", "newlength", "setcounter", "newcounter",
        "pagestyle", "thispagestyle", "includepdf", "graphicspath",
        "hypersetup", "definecolor", "geometry", "usetikzlibrary",
        "hspace", "vspace", "hphantom", "vphantom", "phantom",
    ]
    private static let verbatimEnvironments: Set<String> = [
        "verbatim", "verbatim*", "Verbatim", "BVerbatim", "lstlisting", "minted", "alltt", "comment",
    ]
    private static let mathEnvironments: Set<String> = [
        "equation", "equation*", "align", "align*", "alignat", "alignat*", "flalign", "flalign*",
        "gather", "gather*", "multline", "multline*", "eqnarray", "eqnarray*", "displaymath", "math",
    ]
    /// Escaped-special-character commands (`\%`, `\&`, …): resolved to the
    /// literal glyph for section-title display only; never counted as a word
    /// (see the type doc's note on chunk-boundary word splitting).
    private static let symbolGlyph: [UInt8: Character] = [
        0x25: "%", 0x24: "$", 0x26: "&", 0x23: "#", 0x5F: "_", 0x7B: "{", 0x7D: "}", 0x7E: "~", 0x5E: "^",
    ]

    /// One pass over `text`'s UTF-8 bytes. ASCII structural characters
    /// (`\ % $ { }`) are matched a byte at a time; everything else (including
    /// multibyte UTF-8 continuation/lead bytes) is treated as ordinary word
    /// content, so accented/CJK text is counted without decoding scalars.
    private final class Scanner {
        private struct Scope { var restoreBucket: Bucket; var endsTitleCapture: Bool; var level: Int? = nil }

        private let bytes: [UInt8]
        private var idx = 0
        private var currentBucket: Bucket = .body
        private var scopeStack: [Scope] = []
        private var titleBuffer = ""
        private(set) var total = Counts()
        private(set) var sections: [SectionBreakdown] = []
        private var currentSectionIndex: Int?

        init(text: String) { bytes = Array(text.utf8) }

        func run() {
            let beginDocument: [UInt8] = Array("\\begin{document}".utf8)
            idx = findLiteral(beginDocument, from: 0).map { $0 + beginDocument.count } ?? 0
            while idx < bytes.count {
                let b = bytes[idx]
                switch b {
                case 0x5C: handleBackslash()
                case 0x25: skipComment()
                case 0x24: handleDollar()
                case 0x7B: // '{'
                    scopeStack.append(Scope(restoreBucket: currentBucket, endsTitleCapture: false))
                    idx += 1
                case 0x7D: // '}'
                    idx += 1
                    if let top = scopeStack.popLast() {
                        if top.endsTitleCapture { finalizeTitle(level: top.level ?? 1) }
                        currentBucket = top.restoreBucket
                    }
                default:
                    let start = idx
                    while idx < bytes.count, !isControl(bytes[idx]) { idx += 1 }
                    addWords(wordsIn(start..<idx), bucket: currentBucket)
                    if currentBucket == .header { titleBuffer += String(decoding: bytes[start..<idx], as: UTF8.self) }
                }
            }
        }

        // MARK: byte classification

        private func isControl(_ b: UInt8) -> Bool { b == 0x5C || b == 0x25 || b == 0x24 || b == 0x7B || b == 0x7D }
        private func isLetter(_ b: UInt8) -> Bool { (b >= 0x41 && b <= 0x5A) || (b >= 0x61 && b <= 0x7A) }
        private func isWhitespace(_ b: UInt8) -> Bool { b == 0x20 || b == 0x09 || b == 0x0A || b == 0x0D }
        private func isWordByte(_ b: UInt8) -> Bool {
            (b >= 0x30 && b <= 0x39) || isLetter(b) || b == 0x27 || b == 0x2D || b == 0x5F || b >= 0x80
        }

        private func wordsIn(_ range: Range<Int>) -> Int {
            var count = 0, inWord = false
            for k in range {
                let w = isWordByte(bytes[k])
                if w, !inWord { count += 1 }
                inWord = w
            }
            return count
        }

        // MARK: counting

        private func addWords(_ n: Int, bucket: Bucket) {
            guard n > 0 else { return }
            switch bucket {
            case .body: total.bodyWords += n; if let i = currentSectionIndex { sections[i].counts.bodyWords += n }
            case .header: total.headerWords += n; if let i = currentSectionIndex { sections[i].counts.headerWords += n }
            case .caption: total.captionWords += n; if let i = currentSectionIndex { sections[i].counts.captionWords += n }
            }
        }

        private func addMath(inline: Int = 0, display: Int = 0) {
            total.inlineMath += inline; total.displayMath += display
            if let i = currentSectionIndex { sections[i].counts.inlineMath += inline; sections[i].counts.displayMath += display }
        }

        private func finalizeTitle(level: Int) {
            let cleaned = titleBuffer.components(separatedBy: .whitespacesAndNewlines).filter { !$0.isEmpty }.joined(separator: " ")
            sections.append(SectionBreakdown(documentPath: nil, level: level, title: cleaned.isEmpty ? "(untitled)" : cleaned, counts: Counts()))
            currentSectionIndex = sections.count - 1
            titleBuffer = ""
        }

        // MARK: raw scanning helpers

        private func skipWhitespace() { while idx < bytes.count, isWhitespace(bytes[idx]) { idx += 1 } }

        private func findLiteral(_ pattern: [UInt8], from start: Int) -> Int? {
            guard !pattern.isEmpty, start <= bytes.count - pattern.count else { return nil }
            var i = start
            while i <= bytes.count - pattern.count {
                if bytes[i] == pattern[0] {
                    var match = true
                    var k = 1
                    while k < pattern.count { if bytes[i + k] != pattern[k] { match = false; break }; k += 1 }
                    if match { return i }
                }
                i += 1
            }
            return nil
        }

        @discardableResult
        private func advancePastLiteral(_ pattern: [UInt8]) -> Bool {
            if let found = findLiteral(pattern, from: idx) { idx = found + pattern.count; return true }
            idx = bytes.count
            return false
        }

        private func skipBalanced(open: UInt8, close: UInt8) {
            guard idx < bytes.count, bytes[idx] == open else { return }
            var depth = 1
            idx += 1
            while idx < bytes.count, depth > 0 {
                if bytes[idx] == open { depth += 1 } else if bytes[idx] == close { depth -= 1 }
                idx += 1
            }
        }

        private func skipArgumentsGreedy() {
            while true {
                skipWhitespace()
                guard idx < bytes.count else { return }
                if bytes[idx] == 0x5B { skipBalanced(open: 0x5B, close: 0x5D) } // [...]
                else if bytes[idx] == 0x7B { skipBalanced(open: 0x7B, close: 0x7D) } // {...}
                else { return }
            }
        }

        /// Reads the raw `{name}` right after (whitespace-tolerant) the cursor,
        /// consuming it, without treating its content as words or commands.
        /// Used for `\begin{name}`/`\end{name}`.
        private func readBraceGroupRaw() -> String {
            skipWhitespace()
            guard idx < bytes.count, bytes[idx] == 0x7B else { return "" }
            idx += 1
            let start = idx
            var depth = 1
            while idx < bytes.count, depth > 0 {
                if bytes[idx] == 0x7B { depth += 1 } else if bytes[idx] == 0x7D { depth -= 1 }
                if depth > 0 { idx += 1 }
            }
            let name = String(decoding: bytes[start..<idx], as: UTF8.self)
            if idx < bytes.count { idx += 1 } // closing '}'
            return name
        }

        private func skipComment() {
            while idx < bytes.count, bytes[idx] != 0x0A { idx += 1 }
            if idx < bytes.count { idx += 1 } // the newline itself is ordinary whitespace
        }

        private func handleDollar() {
            if idx + 1 < bytes.count, bytes[idx + 1] == 0x24 {
                idx += 2
                advancePastLiteral([0x24, 0x24])
                addMath(display: 1)
            } else {
                idx += 1
                advancePastLiteral([0x24])
                addMath(inline: 1)
            }
        }

        private func handleBackslash() {
            idx += 1
            guard idx < bytes.count else { return }
            let c = bytes[idx]
            if c == 0x28 { idx += 1; advancePastLiteral([0x5C, 0x29]); addMath(inline: 1); return } // \( ... \)
            if c == 0x5B { idx += 1; advancePastLiteral([0x5C, 0x5D]); addMath(display: 1); return } // \[ ... \]
            if !isLetter(c) {
                idx += 1 // one escaped symbol byte, e.g. \% \& \_ \\ \  \-
                if currentBucket == .header, let glyph = DocumentStatistics.symbolGlyph[c] { titleBuffer.append(glyph) }
                return
            }
            dispatch(command: readCommandName())
        }

        private func readCommandName() -> String {
            let start = idx
            while idx < bytes.count, isLetter(bytes[idx]) { idx += 1 }
            let name = String(decoding: bytes[start..<idx], as: UTF8.self)
            if idx < bytes.count, bytes[idx] == 0x2A { idx += 1 } // trailing '*', not part of the name
            return name
        }

        private func dispatch(command name: String) {
            if name == "begin" || name == "end" {
                let env = readBraceGroupRaw()
                if name == "end", env == "document" { idx = bytes.count; return }
                guard name == "begin" else { return } // a lone \end{env} names nothing further to skip
                if DocumentStatistics.verbatimEnvironments.contains(env) {
                    skipArgumentsGreedy() // e.g. \begin{lstlisting}[language=Python]
                    advancePastLiteral(Array("\\end{\(env)}".utf8))
                } else if DocumentStatistics.mathEnvironments.contains(env) {
                    advancePastLiteral(Array("\\end{\(env)}".utf8))
                    addMath(display: 1)
                }
                return
            }
            if name == "verb" { handleVerb(); return }
            if name == "href" { handleHref(); return }
            if name == "caption" { handleArgumentBucket(.caption, level: nil); return }
            if let level = DocumentOutline.sectionLevels[name] { handleArgumentBucket(.header, level: level); return }
            if DocumentStatistics.skipCommands.contains(name) { skipArgumentsGreedy() }
            // else: transparent — the command name contributes no words; any
            // following {…} is picked up by the generic brace handling above.
        }

        /// `\section{…}` / `\caption{…}`-style commands: an optional `[short]`
        /// form is skipped, then the next `{…}` is captured into `bucket`
        /// (and, for a heading, into a new `SectionBreakdown`) until its
        /// matching close brace.
        private func handleArgumentBucket(_ bucket: Bucket, level: Int?) {
            skipWhitespace()
            if idx < bytes.count, bytes[idx] == 0x5B { skipBalanced(open: 0x5B, close: 0x5D); skipWhitespace() }
            guard idx < bytes.count, bytes[idx] == 0x7B else { return }
            idx += 1
            scopeStack.append(Scope(restoreBucket: currentBucket, endsTitleCapture: level != nil, level: level))
            currentBucket = bucket
            if level != nil { titleBuffer = "" }
        }

        /// `\href[…]{url}{text}`: the URL is never text; the display text
        /// falls through to the generic brace handling right after.
        private func handleHref() {
            skipWhitespace()
            if idx < bytes.count, bytes[idx] == 0x5B { skipBalanced(open: 0x5B, close: 0x5D); skipWhitespace() }
            if idx < bytes.count, bytes[idx] == 0x7B { skipBalanced(open: 0x7B, close: 0x7D) }
        }

        private func handleVerb() {
            guard idx < bytes.count else { return }
            let delimiter = bytes[idx]
            idx += 1
            while idx < bytes.count, bytes[idx] != delimiter { idx += 1 }
            if idx < bytes.count { idx += 1 }
        }
    }
}

// MARK: - ShellModel integration (debounced, off-main-thread)

/// Owns the last computed `DocumentStatistics` for the open document set and
/// recomputes it in the background, coalescing bursts of edits the same way
/// `ShellModel`'s own compile debounce does (`ShellModel.swift`,
/// `ProposalPreview.swift`): a keystroke calls `scheduleUpdate`, which cancels
/// any pending timer and starts a new one; only the timer that survives
/// `debounceInterval` (default 300 ms) actually scans, and it scans on
/// `DispatchQueue.global`, never the main thread the editor is typing on.
@MainActor
@Observable
final class WordCountModel {
    private(set) var total: DocumentStatistics.Counts?
    private(set) var sections: [DocumentStatistics.SectionBreakdown] = []
    /// Wall time of the last background scan, in milliseconds (perf evidence / manual QA only).
    private(set) var lastScanMs: Double?

    @ObservationIgnored private var debounce: DispatchWorkItem?
    @ObservationIgnored private var generation = 0

    /// Scheduling seam: production debounces via
    /// `DispatchQueue.main.asyncAfter` (the default below). Tests inject
    /// `{ _, item in item.perform() }` for deterministic, immediate
    /// execution with no real wall-clock delay. Takes the
    /// `DispatchWorkItem` itself — not a bare closure — so
    /// `debounce?.cancel()` still keeps a superseded scan from running.
    @ObservationIgnored var scheduleDebounce: (TimeInterval, DispatchWorkItem) -> Void = { delay, item in
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: item)
    }

    /// Background-execution seam: production scans on a real background queue
    /// (the default below). `DocumentStatistics.analyze` on real documents is
    /// slow enough that running it on the main thread in production would be
    /// a typing-latency regression, so the default must stay off-main. Tests
    /// inject `{ work in work() }` to run the scan inline, which — paired
    /// with a synchronous `scheduleDebounce` above and an `await`-based drain
    /// of the `Task { @MainActor }` publish in `recompute` — makes the whole
    /// schedule→scan→publish pipeline deterministic with no wall-clock wait.
    @ObservationIgnored var recomputeExecutor: (@Sendable @escaping () -> Void) -> Void = { work in
        DispatchQueue.global(qos: .userInitiated).async(execute: work)
    }

    static let debounceInterval: TimeInterval = {
        if let s = ProcessInfo.processInfo.environment["FLASHTEX_WORDCOUNT_DEBOUNCE_MS"], let ms = Double(s) { return max(0, ms) / 1000 }
        return 0.3
    }()

    /// Schedules a rescan of `documents` after `debounceInterval`, replacing any not-yet-run scan.
    func scheduleUpdate(documents: [RuntimeV1.Document]) {
        debounce?.cancel()
        generation += 1
        let gen = generation
        let snapshot = documents.map { (path: $0.path, text: $0.text) }
        let item = DispatchWorkItem { [weak self] in self?.recompute(snapshot, generation: gen) }
        debounce = item
        if Self.debounceInterval == 0 {
            item.perform()
        } else {
            scheduleDebounce(Self.debounceInterval, item)
        }
    }

    /// Fires on the main actor after every `recompute` cycle reaches its
    /// publish decision — whether it actually published or was superseded.
    /// Production leaves this `nil` (no cost, no behavior change). Tests use
    /// it to wait on the real publication itself instead of assuming
    /// unstructured `Task { @MainActor }` jobs run in enqueue order, which
    /// is not a documented Swift concurrency guarantee.
    @ObservationIgnored var onRecomputeSettled: (() -> Void)?

    private func recompute(_ documents: [(path: String, text: String)], generation gen: Int) {
        recomputeExecutor { [weak self] in
            let start = DispatchTime.now()
            let result = DocumentStatistics.analyze(documents: documents)
            let ms = Double(DispatchTime.now().uptimeNanoseconds - start.uptimeNanoseconds) / 1_000_000
            Task { @MainActor [weak self] in
                defer { self?.onRecomputeSettled?() }
                guard let self, gen == self.generation else { return } // superseded by a later edit
                self.total = result.total
                self.sections = result.sections
                self.lastScanMs = ms
            }
        }
    }
}
