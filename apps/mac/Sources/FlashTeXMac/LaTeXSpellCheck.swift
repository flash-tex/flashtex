import AppKit
import Observation

/// LaTeX-aware spell checking for the source editor (issue #67, lane
/// daniel-ux-spellcheck).
///
/// - `LaTeXProse` (pure): which UTF-16 units of a LaTeX source are prose.
///   Command names, math (`$…$`, `$$…$$`, `\(…\)`, `\[…\]`, equation/align/…
///   bodies), comments, `\verb` and verbatim-like environments, the arguments
///   of reference/label/package/file/definition commands, key=value option
///   lists and words carrying accent commands are not.
/// - `LaTeXSpellChecker`: paints `.spellingState` layout-manager temporary
///   attributes (the system's red spelling underline; never the text storage,
///   so undo and the model are unaffected) over misspelled prose words in the
///   visible window plus the last edited paragraph. Checks are debounced; the
///   tokenizer runs on a background queue and `NSSpellChecker.requestChecking`
///   does the dictionary work asynchronously, so a keystroke only pays for a
///   timer reset. The editor's own continuous spell checking stays off (it
///   would flag command names).
/// - Right-click on a flagged word: suggestions, Ignore Spelling, Learn
///   Spelling. A suggestion is applied with `insertText(_:replacementRange:)`,
///   the same undoable user-edit path as typing, so the coordinator's
///   `textDidChange` reports it to the model.
enum LaTeXProse {
    /// Commands whose adjacent `[…]`/`{…}` arguments are all non-prose.
    static let skipArguments: Set<String> = [
        "label", "ref", "eqref", "pageref", "autoref", "cref", "Cref", "vref", "nameref", "labelcref",
        "cite", "citep", "citet", "citealp", "citealt", "citeauthor", "citeyear", "parencite", "textcite",
        "autocite", "footcite", "nocite", "bibitem",
        "usepackage", "RequirePackage", "documentclass", "LoadClass", "usetikzlibrary", "usepgfplotslibrary",
        "input", "include", "includeonly", "includegraphics", "includepdf", "bibliography", "bibliographystyle",
        "addbibresource", "graphicspath", "url", "path",
        "newcommand", "renewcommand", "providecommand", "newenvironment", "renewenvironment",
        "DeclareMathOperator", "DeclareRobustCommand", "newtheorem", "theoremstyle", "newlength", "newcounter",
        "setlength", "addtolength", "setcounter", "addtocounter", "numberwithin", "setlist", "newlist",
        "vspace", "hspace", "pagestyle", "thispagestyle", "pagenumbering", "hypersetup", "geometry",
        "lstset", "tikzset", "pgfplotsset", "captionsetup", "definecolor", "color", "fontsize", "linespread",
        "setstretch", "crefname", "Crefname", "ensuremath", "si", "SI", "num", "qty", "ang", "unit",
        "rule", "raisebox", "resizebox", "scalebox", "makebox", "framebox", "parbox",
        "begin", "end", // handled specially; listed so `\end{x}` is always skipped
    ]
    /// Commands whose optional arguments and first `{…}` argument are non-prose
    /// (the second argument is text: `\href{url}{text}`, `\textcolor{red}{text}`).
    static let skipFirstArgument: Set<String> = ["href", "textcolor", "colorbox", "hyperref"]
    /// Environments whose body is math.
    static let mathEnvironments: Set<String> = [
        "equation", "equation*", "align", "align*", "alignat", "alignat*", "gather", "gather*",
        "multline", "multline*", "flalign", "flalign*", "eqnarray", "eqnarray*", "displaymath", "math",
        "dmath", "dmath*", "subequations",
    ]
    /// Environments whose body is code or verbatim text.
    static let verbatimEnvironments: Set<String> = [
        "verbatim", "verbatim*", "Verbatim", "lstlisting", "minted", "comment", "filecontents",
        "filecontents*", "tikzpicture", "pgfpicture", "tikzcd", "forest",
    ]
    /// Commands whose name is a letter but which put an accent on their argument (`\c{c}`, `\v{s}`).
    static let letterAccents: Set<String> = ["c", "v", "u", "H", "k", "r", "b", "d", "t"]

    private static let backslash = UInt16(UInt8(ascii: "\\")), percent = UInt16(UInt8(ascii: "%")), dollar = UInt16(UInt8(ascii: "$"))
    private static let lbrace = UInt16(UInt8(ascii: "{")), rbrace = UInt16(UInt8(ascii: "}"))
    private static let lbracket = UInt16(UInt8(ascii: "[")), rbracket = UInt16(UInt8(ascii: "]"))
    private static let newline = UInt16(UInt8(ascii: "\n")), star = UInt16(UInt8(ascii: "*")), equals = UInt16(UInt8(ascii: "="))

    static func isLetter(_ c: UInt16) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
    /// Letters of a prose word (non-ASCII units count as letters).
    static func isWordUnit(_ c: UInt16) -> Bool { isLetter(c) || c >= 0x80 }
    static func isHSpace(_ c: UInt16) -> Bool { c == 0x20 || c == 0x09 }
    /// Control symbols that accent the next letter (`\'e`, `\"o`).
    static func isSymbolAccent(_ c: UInt16) -> Bool {
        switch c {
        case UInt16(UInt8(ascii: "'")), UInt16(UInt8(ascii: "\"")), UInt16(UInt8(ascii: "`")), UInt16(UInt8(ascii: "^")),
             UInt16(UInt8(ascii: "~")), UInt16(UInt8(ascii: "=")), UInt16(UInt8(ascii: ".")): true
        default: false
        }
    }

    // MARK: public (pure)

    /// Per-unit prose flags for UTF-16 `units`.
    static func mask(_ units: [UInt16]) -> [Bool] {
        var prose = [Bool](repeating: false, count: units.count)
        units.withUnsafeBufferPointer { u in
            var i = 0
            while i < u.count {
                switch u[i] {
                case percent: i = lineEnd(u, from: i)
                case dollar:
                    if i + 1 < u.count, u[i + 1] == dollar { i = skipPast(u, from: i + 2, closer: [dollar, dollar], blankLineEnds: false) }
                    else { i = skipPast(u, from: i + 1, closer: [dollar], blankLineEnds: true) }
                case backslash: i = controlSequence(u, at: i, prose: &prose)
                case lbrace, rbrace, lbracket, rbracket, UInt16(UInt8(ascii: "~")), UInt16(UInt8(ascii: "&")), UInt16(UInt8(ascii: "#")),
                     UInt16(UInt8(ascii: "^")), UInt16(UInt8(ascii: "_")): i += 1
                default: prose[i] = true; i += 1
                }
            }
        }
        return prose
    }

    /// Maximal prose ranges of `text` (UTF-16).
    static func proseRanges(in text: NSString) -> [NSRange] {
        let flags = mask(units(of: text))
        var result: [NSRange] = []
        var start: Int?
        for (i, flag) in flags.enumerated() {
            if flag { if start == nil { start = i } }
            else if let s = start { result.append(NSRange(location: s, length: i - s)); start = nil }
        }
        if let s = start { result.append(NSRange(location: s, length: flags.count - s)) }
        return result
    }

    /// `text` with every non-prose unit replaced by a space (newlines kept), so
    /// offsets are unchanged and no word spans a prose boundary.
    static func maskedText(_ text: NSString) -> String {
        var u = units(of: text)
        let flags = mask(u)
        for i in u.indices where !flags[i] && u[i] != newline { u[i] = 0x20 }
        return String(utf16CodeUnits: u, count: u.count)
    }

    static func units(of text: NSString) -> [UInt16] {
        var u = [UInt16](repeating: 0, count: text.length)
        if !u.isEmpty { text.getCharacters(&u, range: NSRange(location: 0, length: text.length)) }
        return u
    }

    // MARK: scanning

    private typealias Units = UnsafeBufferPointer<UInt16>

    private static func lineEnd(_ u: Units, from i: Int) -> Int {
        var k = i
        while k < u.count, u[k] != newline { k += 1 }
        return k
    }

    /// Whether a blank line (only spaces/tabs/CR) starts right after the newline at `k`.
    private static func blankLineAfter(_ u: Units, newlineAt k: Int) -> Bool {
        var j = k + 1
        while j < u.count, isHSpace(u[j]) || u[j] == 0x0D { j += 1 }
        return j < u.count && u[j] == newline
    }

    /// Index just past the first unescaped `closer` at or after `from` (the
    /// end of the text when missing; the blank line when `blankLineEnds`).
    private static func skipPast(_ u: Units, from: Int, closer: [UInt16], blankLineEnds: Bool) -> Int {
        var k = from
        while k < u.count {
            if u[k] == closer[0], k + closer.count <= u.count,
               (1..<closer.count).allSatisfy({ u[k + $0] == closer[$0] }) { return k + closer.count }
            if u[k] == backslash { k += 2; continue }
            if blankLineEnds, u[k] == newline, blankLineAfter(u, newlineAt: k) { return k }
            k += 1
        }
        return u.count
    }

    /// Index past the `{…}` or `[…]` group opening at `k` (nesting and escapes
    /// respected; a blank line ends an unterminated group).
    private static func skipGroup(_ u: Units, at k: Int) -> Int {
        let open = u[k], close = open == lbrace ? rbrace : rbracket
        var depth = 0
        var j = k
        while j < u.count {
            let c = u[j]
            if c == backslash { j += 2; continue }
            if c == open { depth += 1 }
            else if c == close { depth -= 1; if depth == 0 { return j + 1 } }
            else if c == newline, blankLineAfter(u, newlineAt: j) { return j }
            j += 1
        }
        return u.count
    }

    /// Skips every `[…]`/`{…}` group directly adjacent to `j`.
    private static func skipAdjacentGroups(_ u: Units, from j: Int) -> Int {
        var j = j
        while j < u.count, u[j] == lbrace || u[j] == lbracket { j = skipGroup(u, at: j) }
        return j
    }

    private static func unmarkWord(before i: Int, _ u: Units, _ prose: inout [Bool]) {
        var k = i - 1
        while k >= 0, isWordUnit(u[k]) { prose[k] = false; k -= 1 }
    }

    /// Skips an accent's argument and the rest of the word it belongs to.
    private static func skipAccentedWord(_ u: Units, from j: Int) -> Int {
        var j = j
        if j < u.count, u[j] == lbrace { j = skipGroup(u, at: j) } else if j < u.count, isWordUnit(u[j]) { j += 1 }
        while j < u.count {
            if isWordUnit(u[j]) { j += 1; continue }
            if u[j] == backslash, j + 1 < u.count, isSymbolAccent(u[j + 1]) { j = skipAccentedWord(u, from: j + 2); continue }
            break
        }
        return j
    }

    private static func controlSequence(_ u: Units, at i: Int, prose: inout [Bool]) -> Int {
        guard i + 1 < u.count else { return u.count }
        let d = u[i + 1]
        if !isLetter(d) {
            switch d {
            case UInt16(UInt8(ascii: "(")): return skipPast(u, from: i + 2, closer: [backslash, UInt16(UInt8(ascii: ")"))], blankLineEnds: false)
            case lbracket: return skipPast(u, from: i + 2, closer: [backslash, rbracket], blankLineEnds: false)
            case backslash: // line break, optional `*` and `[dimension]`
                var j = i + 2
                if j < u.count, u[j] == star { j += 1 }
                if j < u.count, u[j] == lbracket { j = skipGroup(u, at: j) }
                return j
            default:
                guard isSymbolAccent(d) else { return i + 2 } // \% \$ \& \, \  …
                unmarkWord(before: i, u, &prose)
                return skipAccentedWord(u, from: i + 2)
            }
        }
        var j = i + 1
        while j < u.count, isLetter(u[j]) { j += 1 }
        let name = String(utf16CodeUnits: Array(u[(i + 1)..<j]), count: j - i - 1)
        if j < u.count, u[j] == star { j += 1 }
        switch name {
        case "verb":
            guard j < u.count, !isLetter(u[j]), !isHSpace(u[j]), u[j] != newline else { return j }
            let delimiter = u[j]
            var k = j + 1
            while k < u.count, u[k] != delimiter, u[k] != newline { k += 1 }
            return min(k + 1, u.count)
        case "begin":
            guard j < u.count, u[j] == lbrace else { return j }
            let close = skipGroup(u, at: j)
            let nameEnd = close > j + 1 && close <= u.count && u[close - 1] == rbrace ? close - 1 : close
            let env = String(utf16CodeUnits: Array(u[(j + 1)..<nameEnd]), count: nameEnd - j - 1)
            if mathEnvironments.contains(env) || verbatimEnvironments.contains(env) {
                return skipPast(u, from: close, closer: Array("\\end{\(env)}".utf16), blankLineEnds: false)
            }
            return skipAdjacentGroups(u, from: close)
        case "def", "gdef", "edef", "xdef", "let":
            var k = j
            while k < u.count, isHSpace(u[k]) { k += 1 }
            if k < u.count, u[k] == backslash { k += 1; while k < u.count, isLetter(u[k]) { k += 1 } }
            if name == "let" { return k }
            while k < u.count, u[k] != lbrace, u[k] != newline { k += 1 } // parameter text (#1#2)
            return k < u.count && u[k] == lbrace ? skipGroup(u, at: k) : k
        default:
            break
        }
        if letterAccents.contains(name), j < u.count, u[j] == lbrace {
            unmarkWord(before: i, u, &prose)
            return skipAccentedWord(u, from: j)
        }
        if skipArguments.contains(name) {
            var k = j
            while k < u.count, isHSpace(u[k]) { k += 1 }
            return k < u.count && (u[k] == lbrace || u[k] == lbracket) ? skipAdjacentGroups(u, from: k) : j
        }
        if skipFirstArgument.contains(name) {
            var k = j
            while k < u.count, u[k] == lbracket { k = skipGroup(u, at: k) }
            if name != "hyperref", k < u.count, u[k] == lbrace { k = skipGroup(u, at: k) }
            return k
        }
        // Any other command: a `[key=value]` option list is not prose.
        if j < u.count, u[j] == lbracket {
            let end = skipGroup(u, at: j)
            if u[j..<min(end, u.count)].contains(equals) { return end }
        }
        return j
    }
}

// MARK: - checker

@MainActor
final class LaTeXSpellChecker: NSObject {
    static let key = NSAttributedString.Key.spellingState
    static let flag = 1 // NSSpellingStateSpellingFlag
    /// Characters checked on each side of the visible range.
    static let padding = 1_500
    /// Documents up to this length are tokenized from the start (exact
    /// context); beyond it the scan starts at a blank line before the window.
    static let exactContextLimit = 64_000
    static let editDelay: TimeInterval = 0.4
    static let scrollDelay: TimeInterval = 0.12

    private weak var textView: NSTextView?
    private var storageObserver: NSObjectProtocol?
    private var boundsObserver: NSObjectProtocol?
    private let tag = NSSpellChecker.uniqueSpellDocumentTag()
    /// Bumped on every character edit; a result for an older generation is dropped.
    private var generation = 0
    /// Ranges checked at the current generation (merged).
    private var checked: [NSRange] = []
    private var editAnchor: Int?
    private var lastEditEnd: Int?
    private var timer: Timer?
    private(set) var enabled = true
    /// Evidence: check passes started and misspellings painted by the last pass.
    private(set) var checks = 0
    private(set) var lastPainted: [NSRange] = []

    deinit {
        if let storageObserver { NotificationCenter.default.removeObserver(storageObserver) }
        if let boundsObserver { NotificationCenter.default.removeObserver(boundsObserver) }
        timer?.invalidate()
        let tag = tag
        DispatchQueue.main.async { NSSpellChecker.shared.closeSpellDocument(withTag: tag) }
    }

    /// Follows `tv`'s storage and scroll position and the preference.
    func attach(_ tv: NSTextView) {
        textView = tv
        tv.isContinuousSpellCheckingEnabled = false // AppKit's checker would flag command names
        tv.isGrammarCheckingEnabled = false
        if let storage = tv.textStorage {
            storageObserver = NotificationCenter.default.addObserver(
                forName: NSTextStorage.didProcessEditingNotification, object: storage, queue: nil
            ) { [weak self] note in
                MainActor.assumeIsolated {
                    guard let self, let storage = note.object as? NSTextStorage, storage.editedMask.contains(.editedCharacters) else { return }
                    self.generation += 1
                    self.checked = []
                    self.editAnchor = storage.editedRange.location
                    self.lastEditEnd = NSMaxRange(storage.editedRange)
                    self.schedule(after: Self.editDelay)
                }
            }
        }
        if let clip = tv.enclosingScrollView?.contentView {
            clip.postsBoundsChangedNotifications = true
            boundsObserver = NotificationCenter.default.addObserver(
                forName: NSView.boundsDidChangeNotification, object: clip, queue: nil
            ) { [weak self] _ in
                MainActor.assumeIsolated { self?.scrolled() }
            }
        }
        observePreference()
        schedule(after: Self.scrollDelay)
    }

    private func observePreference() {
        enabled = EditorPreferences.shared.spellCheck
        withObservationTracking { _ = EditorPreferences.shared.spellCheck } onChange: { [weak self] in
            Task { @MainActor in
                guard let self else { return }
                self.observePreference()
                if self.enabled { self.recheck() } else { self.clear() }
            }
        }
    }

    /// Drops what was checked (dictionary or preference change) and checks the window again.
    func recheck() {
        checked = []
        schedule(after: 0)
    }

    private func scrolled() {
        guard enabled, let tv = textView else { return }
        // Not inside `processEditing`: the window needs layout (GH#681).
        if let storage = tv.textStorage, !storage.editedMask.isEmpty {
            DispatchQueue.main.async { [weak self] in self?.scrolled() }
            return
        }
        let window = Self.window(for: tv)
        if !checked.contains(where: { NSIntersectionRange($0, window) == window }) { schedule(after: Self.scrollDelay) }
    }

    private func schedule(after delay: TimeInterval) {
        guard enabled else { return }
        timer?.invalidate()
        let t = Timer(timeInterval: delay, repeats: false) { [weak self] _ in MainActor.assumeIsolated { self?.check() } }
        RunLoop.main.add(t, forMode: .common)
        timer = t
    }

    func clear() {
        timer?.invalidate()
        guard let tv = textView, let lm = tv.layoutManager, let length = tv.textStorage?.length, length > 0 else { return }
        lm.removeTemporaryAttribute(Self.key, forCharacterRange: NSRange(location: 0, length: length))
        checked = []
        lastPainted = []
    }

    /// Characters in the visible rect plus padding, snapped to whole lines;
    /// the whole text when the view is not in a window.
    static func window(for tv: NSTextView) -> NSRange {
        let ns = (tv.textStorage?.string ?? tv.string) as NSString
        guard let lm = tv.layoutManager, let container = tv.textContainer, tv.window != nil,
              !tv.visibleRect.isEmpty else { return NSRange(location: 0, length: ns.length) }
        let glyphs = lm.glyphRange(forBoundingRect: tv.visibleRect, in: container)
        let visible = lm.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil)
        let start = max(0, visible.location - padding)
        let end = min(ns.length, NSMaxRange(visible) + padding)
        return ns.lineRange(for: NSRange(location: start, length: max(0, end - start)))
    }

    /// Where tokenizing must start so `target` is scanned with its LaTeX context.
    static func scanStart(in ns: NSString, for target: NSRange) -> Int {
        guard target.location > exactContextLimit else { return 0 }
        let floor = target.location - exactContextLimit / 2
        let blank = ns.range(of: "\n\n", options: .backwards, range: NSRange(location: floor, length: target.location - floor))
        return blank.location == NSNotFound ? ns.lineRange(for: NSRange(location: floor, length: 0)).location : blank.location
    }

    private func check() {
        guard enabled, let tv = textView, let storage = tv.textStorage else { return }
        if tv.hasMarkedText() { schedule(after: Self.editDelay); return } // wait for the composition to commit
        let ns = storage.string as NSString
        var targets = [Self.window(for: tv)]
        if let anchor = editAnchor, anchor <= ns.length {
            let paragraph = ns.paragraphRange(for: NSRange(location: anchor, length: 0))
            if NSIntersectionRange(paragraph, targets[0]) != paragraph { targets.append(paragraph) }
        }
        editAnchor = nil
        let gen = generation
        let caret = tv.selectedRange()
        let skipAt = lastEditEnd.flatMap { caret.length == 0 && caret.location == $0 ? $0 : nil }
        for target in targets where target.length > 0 {
            let start = Self.scanStart(in: ns, for: target)
            let scan = NSRange(location: start, length: NSMaxRange(target) - start)
            let source = ns.substring(with: scan) // copied on the main thread; tokenized off it
            checks += 1
            let tag = tag
            DispatchQueue.global(qos: .userInitiated).async { [weak self] in
                let masked = LaTeXProse.maskedText(source as NSString)
                DispatchQueue.main.async {
                    MainActor.assumeIsolated {
                        guard let self, self.generation == gen, self.enabled else { return }
                        let relative = NSRange(location: target.location - start, length: target.length)
                        NSSpellChecker.shared.requestChecking(
                            of: masked, range: relative, types: NSTextCheckingResult.CheckingType.spelling.rawValue,
                            options: nil, inSpellDocumentWithTag: tag
                        ) { [weak self] _, results, _, _ in
                            let ranges = results.filter { $0.resultType == .spelling }
                                .map { NSRange(location: $0.range.location + start, length: $0.range.length) }
                            DispatchQueue.main.async {
                                MainActor.assumeIsolated { self?.paint(ranges, over: target, generation: gen, skipWordEndingAt: skipAt) }
                            }
                        }
                    }
                }
            }
        }
    }

    private func paint(_ misspelled: [NSRange], over target: NSRange, generation gen: Int, skipWordEndingAt skip: Int?) {
        guard gen == generation, enabled, let tv = textView, let lm = tv.layoutManager,
              let length = tv.textStorage?.length, NSMaxRange(target) <= length else { return }
        lm.removeTemporaryAttribute(Self.key, forCharacterRange: target)
        var painted: [NSRange] = []
        for r in misspelled where r.length > 0 && NSMaxRange(r) <= length && NSMaxRange(r) != skip {
            lm.addTemporaryAttribute(Self.key, value: Self.flag, forCharacterRange: r)
            painted.append(r)
        }
        lastPainted = painted
        checked = SourceEditorView.MarkPainter.merged(checked + [target])
    }

    // MARK: context menu

    /// Flagged word range at `index`, if any.
    func misspelledRange(at index: Int) -> NSRange? {
        guard enabled, let tv = textView, let lm = tv.layoutManager, let length = tv.textStorage?.length,
              index >= 0, index < length else { return nil }
        var effective = NSRange(location: 0, length: 0)
        guard lm.temporaryAttribute(Self.key, atCharacterIndex: index, effectiveRange: &effective) != nil else { return nil }
        return effective
    }

    /// Puts suggestions, Ignore and Learn at the top of `menu` for a flagged word at `index`.
    func decorate(_ menu: NSMenu, at index: Int) -> NSMenu {
        guard let tv = textView, let range = misspelledRange(at: index) else { return menu }
        let word = ((tv.textStorage?.string ?? "") as NSString).substring(with: range)
        // AppKit's own spelling items act on its checker's state; drop them to avoid a second, LaTeX-unaware list.
        for item in menu.items where item.action.map({ NSStringFromSelector($0).lowercased().contains("spelling") }) == true {
            menu.removeItem(item)
        }
        let guesses = NSSpellChecker.shared.guesses(forWordRange: NSRange(location: 0, length: (word as NSString).length),
                                                    in: word, language: nil, inSpellDocumentWithTag: tag) ?? []
        var items: [NSMenuItem] = []
        if guesses.isEmpty {
            let none = NSMenuItem(title: "No Guesses Found", action: nil, keyEquivalent: "")
            none.isEnabled = false
            items.append(none)
        }
        for guess in guesses.prefix(8) {
            let item = NSMenuItem(title: guess, action: #selector(replaceWord(_:)), keyEquivalent: "")
            item.target = self
            item.representedObject = Correction(range: range, word: word, replacement: guess)
            items.append(item)
        }
        items.append(.separator())
        for (title, action) in [("Ignore Spelling", #selector(ignoreWord(_:))), ("Learn Spelling", #selector(learnWord(_:)))] {
            let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
            item.target = self
            item.representedObject = Correction(range: range, word: word, replacement: word)
            items.append(item)
        }
        items.append(.separator())
        for (offset, item) in items.enumerated() { menu.insertItem(item, at: offset) }
        return menu
    }

    final class Correction: NSObject {
        let range: NSRange, word: String, replacement: String
        init(range: NSRange, word: String, replacement: String) { self.range = range; self.word = word; self.replacement = replacement }
    }

    /// Replaces the word through the text view's user-edit path (one undo step,
    /// reported to the model by the editor's `textDidChange`).
    @objc func replaceWord(_ sender: NSMenuItem) {
        guard let c = sender.representedObject as? Correction, let tv = textView, let storage = tv.textStorage,
              NSMaxRange(c.range) <= storage.length, (storage.string as NSString).substring(with: c.range) == c.word,
              !tv.hasMarkedText() else { return }
        tv.breakUndoCoalescing()
        tv.insertText(c.replacement, replacementRange: c.range)
        tv.breakUndoCoalescing()
    }

    @objc func ignoreWord(_ sender: NSMenuItem) {
        guard let c = sender.representedObject as? Correction else { return }
        NSSpellChecker.shared.ignoreWord(c.word, inSpellDocumentWithTag: tag)
        recheck()
    }

    @objc func learnWord(_ sender: NSMenuItem) {
        guard let c = sender.representedObject as? Correction else { return }
        NSSpellChecker.shared.learnWord(c.word)
        recheck()
    }
}

extension SourceEditorView.Coordinator {
    /// Right-click on a flagged word shows spelling suggestions (LaTeXSpellChecker).
    @objc(textView:menu:forEvent:atIndex:)
    func textView(_ view: NSTextView, menu: NSMenu, for event: NSEvent, at charIndex: Int) -> NSMenu? {
        spelling.decorate(menu, at: charIndex)
    }
}
